use crate::cache::DedupCache;
use crate::config::Config;
use crate::filters::evaluate_candidate;
use crate::github::GitHubClient;
use crate::models::{BountyCandidate, BountyStatus};
use crate::telegram::TelegramNotifier;
use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

pub struct Scanner {
    config: Config,
    github: GitHubClient,
    telegram: TelegramNotifier,
    cache: DedupCache,
}

impl Scanner {
    pub fn new(config: Config) -> Self {
        let github = GitHubClient::new(config.github_token.clone());
        let telegram = TelegramNotifier::new(
            config.telegram_bot_token.clone(),
            config.telegram_chat_id,
        );
        let cache = DedupCache::load_or_create(&config.cache_file);

        Self {
            config,
            github,
            telegram,
            cache,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting BountySniper-RS daemon...");

        // Try to auto-resolve chat_id if not configured
        if self.telegram.chat_id.is_none() {
            if let Ok(Some(id)) = self.telegram.resolve_chat_id().await {
                info!("Target chat ID set to: {}", id);
            } else {
                warn!("Telegram chat_id is unknown. Please send any message (e.g. /start) to @amadeuus_bot so it can send you alerts!");
            }
        }

        loop {
            // Check if chat_id can be resolved now if still missing
            if self.telegram.chat_id.is_none() {
                let _ = self.telegram.resolve_chat_id().await;
            }

            info!("Scanning GitHub for fresh bounties...");
            if let Err(e) = self.scan_cycle().await {
                error!("Error during scan cycle: {:?}", e);
            }

            if self.config.once {
                info!("Single-run mode completed. Exiting.");
                break;
            }

            sleep(Duration::from_secs(self.config.poll_interval_secs)).await;
        }

        Ok(())
    }

    async fn scan_cycle(&mut self) -> Result<()> {
        let queries = [
            "is:issue is:open label:bounty sort:updated-desc",
            "is:issue is:open label:\"Bug bounty 💰\" sort:updated-desc",
            "is:issue is:open label:\"💎 Bounty\" sort:updated-desc",
            "is:issue is:open \"algora.io\" sort:updated-desc",
            "is:issue is:open \"Price:\" label sort:updated-desc",
        ];

        let mut discovered = 0;
        let mut alerted = 0;

        for query in &queries {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            match self.github.search_issues(query).await {
                Ok(issues) => {
                    for issue in issues {
                        let issue_key = issue.html_url.clone();
                        if self.cache.is_notified(&issue_key) {
                            continue;
                        }

                        discovered += 1;

                        // Fetch repo info
                        let repo = match self.github.get_repository(&issue.repository_url).await {
                            Ok(r) => r,
                            Err(e) => {
                                warn!("Failed to get repo for {}: {:?}", issue.repository_url, e);
                                continue;
                            }
                        };

                        // Fetch comments to check for reservations / gating
                        let comments = self
                            .github
                            .get_issue_comments(&format!(
                                "{}/comments",
                                issue.repository_url
                            ))
                            .await
                            .unwrap_or_default();

                        // Count matching PRs
                        let matching_prs = self
                            .github
                            .count_matching_open_prs(&repo.full_name, issue.number)
                            .await
                            .unwrap_or(0);

                        // Evaluate
                        let eval = evaluate_candidate(
                            &issue,
                            &repo,
                            matching_prs,
                            &comments,
                            self.config.min_stars,
                        );

                        // If Pursue or Hold, alert!
                        if eval.status == BountyStatus::Pursue || eval.status == BountyStatus::Hold {
                            let candidate = BountyCandidate {
                                issue_url: issue.html_url.clone(),
                                repo_full_name: repo.full_name.clone(),
                                issue_number: issue.number,
                                title: issue.title.clone(),
                                stars: repo.stargazers_count,
                                reward: eval.reward,
                                status: eval.status,
                                competition_prs: matching_prs,
                                reasons: eval.reasons,
                            };

                            info!(
                                "Bounty Qualified: {} #{} (Status: {:?})",
                                candidate.repo_full_name, candidate.issue_number, candidate.status
                            );

                            if let Err(e) = self.telegram.send_alert(&candidate).await {
                                warn!("Failed to send Telegram alert: {:?}", e);
                            } else {
                                alerted += 1;
                            }
                        }

                        // Mark as seen in cache regardless of whether it was pursue or spam so we don't re-parse every loop
                        self.cache.mark_notified(&issue_key);
                    }
                }
                Err(e) => {
                    warn!("Search failed for query '{}': {:?}", query, e);
                }
            }
        }

        // Save cache
        let _ = self.cache.save(&self.config.cache_file);

        info!(
            "Scan cycle complete. Discovered {} new issues, dispatched {} Telegram alerts.",
            discovered, alerted
        );

        Ok(())
    }
}
