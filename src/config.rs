use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "bounty-sniper",
    about = "High-Frequency GitHub Bounty Scanner & Verification Radar in Rust with Telegram Alerts"
)]
pub struct Config {
    #[arg(long, env = "GITHUB_TOKEN")]
    pub github_token: Option<String>,

    #[arg(
        long,
        env = "TELEGRAM_BOT_TOKEN",
        default_value = "7729508480:AAER27k6f9XScuakIKKRuZ_rU431hfPZe0k"
    )]
    pub telegram_bot_token: String,

    #[arg(long, env = "TELEGRAM_CHAT_ID")]
    pub telegram_chat_id: Option<i64>,

    #[arg(long, default_value = "5")]
    pub min_stars: u64,

    #[arg(long, default_value = "30")]
    pub poll_interval_secs: u64,

    #[arg(long, default_value = "sniper_cache.json")]
    pub cache_file: String,

    #[arg(long, help = "Run once and exit (ideal for GitHub Actions cron)")]
    pub once: bool,
}

impl Config {
    pub fn load() -> Self {
        dotenvy::dotenv().ok();
        Self::parse()
    }
}
