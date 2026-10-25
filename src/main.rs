mod cache;
mod config;
mod filters;
mod github;
mod models;
mod scanner;
mod telegram;

use anyhow::Result;
use config::Config;
use scanner::Scanner;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::load();
    let mut scanner = Scanner::new(config);

    scanner.run().await?;

    Ok(())
}
