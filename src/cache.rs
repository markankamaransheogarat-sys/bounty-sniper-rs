use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use tracing::warn;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DedupCache {
    pub notified_issues: BTreeSet<String>,
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
        let tmp_path = format!("{path}.tmp");
        fs::write(&tmp_path, json)?;
        fs::rename(&tmp_path, path)?;
        Ok(())
    }

    pub fn is_notified(&self, issue_key: &str) -> bool {
        self.notified_issues.contains(issue_key)
    }

    pub fn mark_notified(&mut self, issue_key: &str) {
        self.notified_issues.insert(issue_key.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_saves_deterministically_and_dedups() {
        let dir =
            std::env::temp_dir().join(format!("bounty_sniper_cache_test_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("cache.json");
        let path_str = file.to_str().unwrap();

        let mut cache = DedupCache::default();
        cache.mark_notified("https://github.com/b/repo/issues/2");
        cache.mark_notified("https://github.com/a/repo/issues/1");
        cache.save(path_str).unwrap();

        let raw = fs::read_to_string(path_str).unwrap();
        let idx_a = raw.find("https://github.com/a/repo/issues/1").unwrap();
        let idx_b = raw.find("https://github.com/b/repo/issues/2").unwrap();
        assert!(
            idx_a < idx_b,
            "BTreeSet should serialize keys in sorted order"
        );

        let reloaded = DedupCache::load_or_create(path_str);
        assert!(reloaded.is_notified("https://github.com/a/repo/issues/1"));
        assert!(!reloaded.is_notified("https://github.com/c/repo/issues/3"));

        let _ = fs::remove_dir_all(&dir);
    }
}
