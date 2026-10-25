use crate::models::{BountyCandidate, BountyStatus};
use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use tracing::{info, warn};

pub struct TelegramNotifier {
    client: Client,
    bot_token: String,
    pub chat_id: Option<i64>,
}

impl TelegramNotifier {
    pub fn new(bot_token: String, chat_id: Option<i64>) -> Self {
        Self {
            client: Client::new(),
            bot_token,
            chat_id,
        }
    }

    pub async fn resolve_chat_id(&mut self) -> Result<Option<i64>> {
        if self.chat_id.is_some() {
            return Ok(self.chat_id);
        }

        let url = format!(
            "https://api.telegram.org/bot{}/getUpdates?limit=5",
            self.bot_token
        );
        let resp = self.client.get(&url).send().await?;
        if resp.status().is_success() {
            let val: serde_json::Value = resp.json().await?;
            if let Some(arr) = val.get("result").and_then(|r| r.as_array()) {
                for item in arr.iter().rev() {
                    if let Some(chat_id) = item
                        .get("message")
                        .and_then(|m| m.get("chat"))
                        .and_then(|c| c.get("id"))
                        .and_then(|id| id.as_i64())
                    {
                        info!("Auto-discovered Telegram chat_id: {}", chat_id);
                        self.chat_id = Some(chat_id);
                        return Ok(Some(chat_id));
                    }
                }
            }
        }

        Ok(None)
    }

    pub async fn send_alert(&self, candidate: &BountyCandidate) -> Result<()> {
        let chat_id = match self.chat_id {
            Some(id) => id,
            None => {
                warn!("Cannot send Telegram alert: chat_id not set and not yet discovered. Send a message to the bot first!");
                return Ok(());
            }
        };

        let status_emoji = match candidate.status {
            BountyStatus::Pursue => "🔥 <b>[PRIORITY PURSUE]</b>",
            BountyStatus::Hold => "⏳ <b>[ON HOLD / CLAIMED]</b>",
            BountyStatus::Contested => "⚔️ <b>[CONTESTED]</b>",
            BountyStatus::Gated => "🔒 <b>[GATED / CORE ONLY]</b>",
            BountyStatus::Spam => "🚫 <b>[SPAM / FILTERED]</b>",
        };

        let reward_str = candidate
            .reward
            .as_deref()
            .unwrap_or("Verified Bounty");

        let reasons_list = candidate
            .reasons
            .iter()
            .map(|r| format!("• {}", html_escape(r)))
            .collect::<Vec<_>>()
            .join("\n");

        let message = format!(
            "{status_emoji}\n\n\
            📦 <b>Repository:</b> <code>{repo}</code> (⭐ {stars})\n\
            💰 <b>Reward:</b> <b>{reward}</b>\n\
            ⚡ <b>Competition:</b> {prs} open PR(s)\n\
            📌 <b>Issue #{number}:</b> <a href=\"{url}\">{title}</a>\n\n\
            🛡️ <b>Sniper Assessment:</b>\n{reasons}\n\n\
            👉 <a href=\"{url}\"><b>Open on GitHub & Claim</b></a>",
            status_emoji = status_emoji,
            repo = html_escape(&candidate.repo_full_name),
            stars = candidate.stars,
            reward = html_escape(reward_str),
            prs = candidate.competition_prs,
            number = candidate.issue_number,
            url = candidate.issue_url,
            title = html_escape(&candidate.title),
            reasons = reasons_list
        );

        self.send_html_message(chat_id, &message).await
    }

    pub async fn send_html_message(&self, chat_id: i64, html: &str) -> Result<()> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        let payload = json!({
            "chat_id": chat_id,
            "text": html,
            "parse_mode": "HTML",
            "disable_web_page_preview": false
        });

        let resp = self.client.post(&url).json(&payload).send().await?;
        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            warn!("Failed to send Telegram message: {}", err_text);
        } else {
            info!("Telegram alert successfully delivered to chat {}", chat_id);
        }

        Ok(())
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
