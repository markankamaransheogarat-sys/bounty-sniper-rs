use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use tracing::warn;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DedupCache {
    pub notified_issues: HashSet<String>,
}

impl DedupCache {
    pub fn load_or_create(path: &str) -> Self {
        if Path::new(path).exists() {
            match fs::read_to_string(path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(cache) => return cache,
                    Err(e) => warn!("Failed to parse dedup cache: {}, creating new", e),
                },
                Err(e) => warn!("Failed to read dedup cache file: {}, creating new", e),
            }
        }
        Self::default()
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn is_notified(&self, issue_key: &str) -> bool {
        self.notified_issues.contains(issue_key)
    }

    pub fn mark_notified(&mut self, issue_key: &str) {
        self.notified_issues.insert(issue_key.to_string());
    }
}
