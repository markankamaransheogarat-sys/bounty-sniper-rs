use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub name: String,
    pub full_name: String,
    #[serde(default)]
    pub stargazers_count: u64,
    #[serde(default)]
    pub forks_count: u64,
    #[serde(default)]
    pub open_issues_count: u64,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub fork: bool,
    #[serde(default)]
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub login: String,
    pub html_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub html_url: String,
    pub comments: u64,
    pub created_at: String,
    pub updated_at: String,
    pub user: Option<User>,
    pub repository_url: String,
    #[serde(default)]
    pub labels: Vec<Label>,
    #[serde(default)]
    pub assignees: Vec<User>,
    #[serde(default)]
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BountyStatus {
    Pursue,     // Clean, verified, 0 or low competition
    Hold,       // Active claim signal within recent hours
    Contested,  // Many PRs (>2) competing
    Gated,      // Restricted to core team/collaborators
    Spam,       // Blacklisted bot/scam mirror
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BountyCandidate {
    pub issue_url: String,
    pub repo_full_name: String,
    pub issue_number: u64,
    pub title: String,
    pub stars: u64,
    pub reward: Option<String>,
    pub status: BountyStatus,
    pub competition_prs: usize,
    pub reasons: Vec<String>,
}
