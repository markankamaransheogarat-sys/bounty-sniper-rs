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
];

const BOT_SPAMMERS: &[&str] = &[
    "segunmicheal27",
    "ramgox",
    "alfrexo95",
    "thanos web3",
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

    if extracted_reward.is_none() && has_bounty_label {
        extracted_reward = Some("Official Bug Bounty Label".to_string());
    }

    if extracted_reward.is_none() {
        reasons.push("No explicit reward pattern or bounty label detected in issue".to_string());
        return EvaluationResult {
            status: BountyStatus::Spam,
            reward: None,
            reasons,
        };
    }

    // 3. Check Gating / Lockout
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

    // 4. Check Claim / Reservation Signals
    let mut recent_claim_count = 0;
    for comment in comments_body {
        if claim_regex().is_match(comment) {
            recent_claim_count += 1;
        }
    }

    // 5. Check PR competition
    if matching_pr_count > 2 {
        reasons.push(format!("Overly crowded: {} open PRs already competing", matching_pr_count));
        return EvaluationResult {
            status: BountyStatus::Contested,
            reward: extracted_reward,
            reasons,
        };
    }

    if recent_claim_count > 0 && matching_pr_count > 0 {
        reasons.push(format!("Active reservation with {} competing PR(s)", matching_pr_count));
        return EvaluationResult {
            status: BountyStatus::Hold,
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

    reasons.push(format!("Clean ticket! {} competing PRs, reward: {}", matching_pr_count, extracted_reward.as_deref().unwrap_or("Yes")));
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
}
