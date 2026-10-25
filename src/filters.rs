use crate::models::{BountyStatus, Issue, Repository};
use regex::Regex;
use std::sync::OnceLock;

static REWARD_RE: OnceLock<Regex> = OnceLock::new();
static CLAIM_RE: OnceLock<Regex> = OnceLock::new();
static GATED_RE: OnceLock<Regex> = OnceLock::new();

fn reward_regex() -> &'static Regex {
    REWARD_RE.get_or_init(|| {
        Regex::new(r"(?i)(?:\$\s?[\d,]+(?:\.\d+)?(?:\s?(?:USD|USDC))?|[\d,]+(?:\.\d+)?\s?(?:USDC|USD|EUR|BTC|ETH|SOL|DOT|DAI)\b|Price:\s*[\d,]+\s*(?:USD|USDC|DAI))").unwrap()
    })
}

fn claim_regex() -> &'static Regex {
    CLAIM_RE.get_or_init(|| {
        Regex::new(r"(?im)^\s*/(claim|try|attempt|reserve)\b.*$").unwrap()
    })
}

fn gated_regex() -> &'static Regex {
    GATED_RE.get_or_init(|| {
        Regex::new(r"(?i)(must be a core team member|collaborator|locked to @|only team members)").unwrap()
    })
}

const BLACKLISTED_OWNERS_AND_REPOS: &[&str] = &[
    "alfrexo95/bounty-bot",
    "zhangjiayang6835-cyber/bounty-plaza",
    "blake-guthrie/glass",
    "doomsday-network",
    "securebananalabs",
    "amillimatrix-eng",
    "zoverions",
    "soulschoolacademy",
    "scottonchain",
    "auscaster/frantic-board",
    "maybaum-prod",
    "dsh-bounty",
    "saricizmeli",
    "19260296862/my",
    "qwer80371832/loadertest",
    "freecad-documentation-project",
    "limitlessprojects",
];

const BOT_SPAMMERS: &[&str] = &[
    "segunmicheal27",
    "ramgox",
    "alfrexo95",
    "thanos web3",
    "palmpay",
];

const VERIFIED_PROGRAMS: &[&str] = &[
    "paraspell/xcm-tools",
    "opensandbox-group/opensandbox",
    "tabularisdb/tabularis",
    "ubiquity/",
];

pub struct EvaluationResult {
    pub status: BountyStatus,
    pub reward: Option<String>,
    pub reasons: Vec<String>,
}

pub fn evaluate_candidate(
    issue: &Issue,
    repo: &Repository,
    matching_pr_count: usize,
    comments_body: &[String],
    min_stars: u64,
) -> EvaluationResult {
    let mut reasons = Vec::new();
    let repo_lower = repo.full_name.to_lowercase();

    // 1. Check Blacklist
    for blacklisted in BLACKLISTED_OWNERS_AND_REPOS {
        if repo_lower.contains(blacklisted) {
            reasons.push(format!("Repository is on the known spam/bot blacklist: {}", blacklisted));
            return EvaluationResult {
                status: BountyStatus::Spam,
                reward: None,
                reasons,
            };
        }
    }

    if let Some(user) = &issue.user {
        let user_lower = user.login.to_lowercase();
        for bot in BOT_SPAMMERS {
            if user_lower.contains(bot) {
                reasons.push(format!("Author matches known bounty spammer: {}", bot));
                return EvaluationResult {
                    status: BountyStatus::Spam,
                    reward: None,
                    reasons,
                };
            }
        }
    }

    // 1b. Exclude Hackathons / Contests / Speculative proposals / Ladders
    let title_lower = issue.title.to_lowercase();
    let is_speculative = [
        "hackathon",
        "challenge",
        "contest",
        "grant application",
        "competition",
        "bounty program",
        "airdrop",
        "wildcard",
        "proposal",
        "rfc",
        "ladder",
        "pool",
        "leaderboard",
        "moonshot",
    ]
    .iter()
    .any(|kw| title_lower.contains(kw) || issue.labels.iter().any(|l| l.name.to_lowercase().contains(kw)));

    if is_speculative {
        reasons.push("Filtered out: Speculative proposal / hackathon / ladder contest (not a direct bug bounty)".to_string());
        return EvaluationResult {
            status: BountyStatus::Spam,
            reward: None,
            reasons,
        };
    }

    // 1c. Exclude Stale / Dormant Issues (>60 days old)
    if let Ok(created_at) = chrono::DateTime::parse_from_rfc3339(&issue.created_at) {
        let age_days = (chrono::Utc::now() - created_at.with_timezone(&chrono::Utc)).num_days();
        if age_days > 60 {
            reasons.push(format!("Stale issue: opened {} days ago", age_days));
            return EvaluationResult {
                status: BountyStatus::Spam,
                reward: None,
                reasons,
            };
        }
    }

    // 1d. Exclude comments poisoned by bot spammers
    for comment in comments_body {
        let c_low = comment.to_lowercase();
        for bot in BOT_SPAMMERS {
            if c_low.contains(bot) {
                reasons.push(format!("Comments poisoned by known bot spammer ({})", bot));
                return EvaluationResult {
                    status: BountyStatus::Spam,
                    reward: None,
                    reasons,
                };
            }
        }
    }

    // 2. Extract Reward
    let mut extracted_reward = None;
    let combined_text = format!(
        "{}\n{}\n{}",
        issue.title,
        issue.body.as_deref().unwrap_or(""),
        issue.labels.iter().map(|l| l.name.as_str()).collect::<Vec<_>>().join(" ")
    );

    for mat in reward_regex().find_iter(&combined_text) {
        let text = mat.as_str().trim();
        extracted_reward = Some(text.to_string());
        break;
    }

    // Check specific label formats like "Bug bounty 💰"
    let has_bounty_label = issue.labels.iter().any(|l| {
        let n = l.name.to_lowercase();
        n.contains("bounty") || n.contains("price:") || n.contains("reward")
    });

    let is_verified_program = VERIFIED_PROGRAMS.iter().any(|p| repo_lower.contains(p));

    if extracted_reward.is_none() && has_bounty_label {
        if is_verified_program {
            extracted_reward = Some("Verified Bug Bounty Program".to_string());
        } else {
            reasons.push("Vague bounty label without explicit cash figure ($/USD/crypto) or escrow platform".to_string());
            return EvaluationResult {
                status: BountyStatus::Spam,
                reward: None,
                reasons,
            };
        }
    }

    if extracted_reward.is_none() {
        reasons.push("No explicit reward pattern or bounty label detected in issue".to_string());
        return EvaluationResult {
            status: BountyStatus::Spam,
            reward: None,
            reasons,
        };
    }

    // 3. Check Gating / Lockout / Assignees
    if !issue.assignees.is_empty() {
        let assignee_names: Vec<_> = issue.assignees.iter().map(|u| u.login.as_str()).collect();
        reasons.push(format!("Issue is already assigned to: {}", assignee_names.join(", ")));
        return EvaluationResult {
            status: BountyStatus::Hold,
            reward: extracted_reward,
            reasons,
        };
    }

    for comment in comments_body {
        if gated_regex().is_match(comment) {
            reasons.push("Issue is restricted to core team members / collaborators".to_string());
            return EvaluationResult {
                status: BountyStatus::Gated,
                reward: extracted_reward,
                reasons,
            };
        }
    }

    // 4. Check Claim / Reservation Signals in Comments
    let mut recent_claim_count = 0;
    for comment in comments_body {
        if claim_regex().is_match(comment) {
            recent_claim_count += 1;
        }
    }

    if recent_claim_count > 0 {
        reasons.push(format!("Active reservation in comments ({} developer(s) attempted/claimed)", recent_claim_count));
        return EvaluationResult {
            status: BountyStatus::Hold,
            reward: extracted_reward,
            reasons,
        };
    }

    // 5. Check PR competition (any open PR means someone is already working/submitted)
    if matching_pr_count > 0 {
        reasons.push(format!("Active competing PR(s): {} open PR(s) already submitted", matching_pr_count));
        return EvaluationResult {
            status: BountyStatus::Contested,
            reward: extracted_reward,
            reasons,
        };
    }

    // 6. Stars / Credibility
    if repo.stargazers_count < min_stars && !has_bounty_label {
        reasons.push(format!("Repository stars ({} < {})", repo.stargazers_count, min_stars));
        return EvaluationResult {
            status: BountyStatus::Hold,
            reward: extracted_reward,
            reasons,
        };
    }

    reasons.push("Clean ticket! 0 competing PRs, unassigned and ready to claim".to_string());
    EvaluationResult {
        status: BountyStatus::Pursue,
        reward: extracted_reward,
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Label, User};

    fn make_test_repo(full_name: &str, stars: u64) -> Repository {
        Repository {
            name: full_name.split('/').last().unwrap().to_string(),
            full_name: full_name.to_string(),
            stargazers_count: stars,
            forks_count: 5,
            open_issues_count: 10,
            html_url: format!("https://github.com/{}", full_name),
            fork: false,
            archived: false,
        }
    }

    fn make_test_issue(title: &str, body: &str, labels: &[&str]) -> Issue {
        Issue {
            id: 1,
            number: 100,
            title: title.to_string(),
            body: Some(body.to_string()),
            html_url: "https://github.com/test/repo/issues/100".to_string(),
            comments: 0,
            created_at: "2026-10-25T12:00:00Z".to_string(),
            updated_at: "2026-10-25T12:00:00Z".to_string(),
            user: Some(User {
                login: "legit-dev".to_string(),
                html_url: None,
            }),
            repository_url: "https://api.github.com/repos/test/repo".to_string(),
            labels: labels
                .iter()
                .map(|l| Label {
                    name: l.to_string(),
                    color: None,
                })
                .collect(),
            assignees: vec![],
            state: "open".to_string(),
        }
    }

    #[test]
    fn test_blacklists_spam_repos() {
        let repo = make_test_repo("alfrexo95/bounty-bot", 0);
        let issue = make_test_issue("Bounty $100", "some text", &["bounty"]);
        let eval = evaluate_candidate(&issue, &repo, 0, &[], 5);
        assert_eq!(eval.status, BountyStatus::Spam);
    }

    #[test]
    fn test_blacklists_spammer_authors() {
        let repo = make_test_repo("some-cool/project", 150);
        let mut issue = make_test_issue("Issue $50", "body", &["bounty"]);
        issue.user = Some(User {
            login: "segunmicheal27".to_string(),
            html_url: None,
        });
        let eval = evaluate_candidate(&issue, &repo, 0, &[], 5);
        assert_eq!(eval.status, BountyStatus::Spam);
    }

    #[test]
    fn test_detects_clean_bounty() {
        let repo = make_test_repo("paraspell/xcm-tools", 450);
        let issue = make_test_issue(
            "[Bug bounty] Dedot extractDestParaId returns non-number",
            "Details here",
            &["Bug bounty 💰"],
        );
        let eval = evaluate_candidate(&issue, &repo, 0, &[], 5);
        assert_eq!(eval.status, BountyStatus::Pursue);
        assert!(eval.reward.is_some());
    }

    #[test]
    fn test_detects_contested_bounty() {
        let repo = make_test_repo("popular/lib", 2000);
        let issue = make_test_issue("Fix typo $50", "body", &["bounty"]);
        let eval = evaluate_candidate(&issue, &repo, 5, &[], 5);
        assert_eq!(eval.status, BountyStatus::Contested);
    }

    #[test]
    fn test_detects_gated_bounty() {
        let repo = make_test_repo("ubiquity/ai.ubq.fi", 500);
        let issue = make_test_issue("Task", "body", &["Price: 75 USD"]);
        let comments = vec!["You must be a core team member to start this task".to_string()];
        let eval = evaluate_candidate(&issue, &repo, 0, &comments, 5);
        assert_eq!(eval.status, BountyStatus::Gated);
    }

    #[test]
    fn test_rejects_hackathons_and_challenges() {
        let repo = make_test_repo("XHToken/Spark-X2.6", 800);
        let issue = make_test_issue("HER Hack-Astron #7 | Challenge", "Win $100", &["bounty", "reward"]);
        let eval = evaluate_candidate(&issue, &repo, 0, &[], 5);
        assert_eq!(eval.status, BountyStatus::Spam);
    }

    #[test]
    fn test_rejects_vague_unfunded_bounties() {
        let repo = make_test_repo("Reqrefusion/FreeCAD-Documentation-Project", 20);
        let issue = make_test_issue("Maintaining Macros", "No explicit price", &["Bounty", "Reward"]);
        let eval = evaluate_candidate(&issue, &repo, 0, &[], 5);
        assert_eq!(eval.status, BountyStatus::Spam);
    }

    #[test]
    fn test_rejects_speculative_wildcards_and_ladders() {
        let repo = make_test_repo("limitlessprojects/Go-Hybrid", 14);
        let issue = make_test_issue(
            "[wildcard] Make FFI claims proof-carrying",
            "Milestone 1 ($250)",
            &["💎 Bounty", "wildcard"],
        );
        let eval = evaluate_candidate(&issue, &repo, 0, &[], 5);
        assert_eq!(eval.status, BountyStatus::Spam);
    }

    #[test]
    fn test_detects_competing_pr_as_contested() {
        let repo = make_test_repo("legit/repo", 500);
        let issue = make_test_issue("Fix bug $100", "Details", &["bounty"]);
        let eval = evaluate_candidate(&issue, &repo, 1, &[], 5);
        assert_eq!(eval.status, BountyStatus::Contested);
    }

    #[test]
    fn test_detects_assigned_issues_as_hold() {
        let repo = make_test_repo("legit/repo", 500);
        let mut issue = make_test_issue("Fix bug $100", "Details", &["bounty"]);
        issue.assignees = vec![User {
            login: "someone".to_string(),
            html_url: None,
        }];
        let eval = evaluate_candidate(&issue, &repo, 0, &[], 5);
        assert_eq!(eval.status, BountyStatus::Hold);
    }

    #[test]
    fn test_detects_comment_claims_as_hold() {
        let repo = make_test_repo("legit/repo", 500);
        let issue = make_test_issue("Fix bug $100", "Details", &["bounty"]);
        let comments = vec!["/attempt I want to work on this".to_string()];
        let eval = evaluate_candidate(&issue, &repo, 0, &comments, 5);
        assert_eq!(eval.status, BountyStatus::Hold);
    }
}
