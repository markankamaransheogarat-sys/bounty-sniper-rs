use crate::models::{Issue, Repository};
use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tracing::debug;

pub struct GitHubClient {
    client: Client,
    _token: Option<String>,
}

impl GitHubClient {
    pub fn new(token: Option<String>) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("bounty-sniper-rs/0.1.0"),
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );

        if let Some(t) = &token {
            let mut val = HeaderValue::from_str(&format!("Bearer {}", t)).unwrap();
            val.set_sensitive(true);
            headers.insert(AUTHORIZATION, val);
        }

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(20))
            .build()
            .expect("Failed to build HTTP client");

        Self { client, _token: token }
    }

    pub async fn search_issues(&self, query: &str) -> Result<Vec<Issue>> {
        let url = format!(
            "https://api.github.com/search/issues?q={}&sort=updated&order=desc&per_page=30",
            urlencoding::encode(query)
        );

        debug!("Searching GitHub issues: {}", query);
        let resp = self.client.get(&url).send().await?;

        if resp.status().as_u16() == 403 || resp.status().as_u16() == 429 {
            tracing::warn!("GitHub rate limit reached, pacing requests...");
            tokio::time::sleep(Duration::from_secs(3)).await;
            return Ok(Vec::new());
        }

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("GitHub search API error: {}", err_text);
        }

        let val: Value = resp.json().await?;
        let items = val.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default();

        let mut issues = Vec::new();
        for item in items {
            if let Ok(issue) = serde_json::from_value::<Issue>(item) {
                issues.push(issue);
            }
        }

        Ok(issues)
    }

    pub async fn get_repository(&self, repo_url: &str) -> Result<Repository> {
        let resp = self.client.get(repo_url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("Failed to fetch repository at {}", repo_url);
        }
        let repo: Repository = resp.json().await?;
        Ok(repo)
    }

    pub async fn get_issue_comments(&self, comments_url: &str) -> Result<Vec<String>> {
        let resp = self.client.get(comments_url).send().await?;
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let val: Value = resp.json().await?;
        let mut bodies = Vec::new();
        if let Some(arr) = val.as_array() {
            for comment in arr {
                if let Some(body) = comment.get("body").and_then(|b| b.as_str()) {
                    bodies.push(body.to_string());
                }
            }
        }

        Ok(bodies)
    }

    pub async fn count_matching_open_prs(&self, repo_full_name: &str, issue_number: u64) -> Result<usize> {
        let q = format!("repo:{} is:pr is:open \"#{}\"", repo_full_name, issue_number);
        let url = format!(
            "https://api.github.com/search/issues?q={}&per_page=1",
            urlencoding::encode(&q)
        );

        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Ok(0);
        }

        let val: Value = resp.json().await?;
        let total = val.get("total_count").and_then(|t| t.as_u64()).unwrap_or(0);
        Ok(total as usize)
    }
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut encoded = String::new();
        for b in s.bytes() {
            match b {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(b as char);
                }
                _ => {
                    encoded.push_str(&format!("%{:02X}", b));
                }
            }
        }
        encoded
    }
}
