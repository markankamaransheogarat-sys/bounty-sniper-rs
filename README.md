# 🎯 BountySniper-RS

**High-Frequency GitHub Bounty Scanner & Verification Radar in Rust with Instant Telegram Alerts**

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![GitHub Actions](https://img.shields.io/badge/Actions-Cloud%20Scanner-success.svg)](.github/workflows/sniper.yml)

---

## ⚡ Overview

**BountySniper-RS** is an ultra-fast, zero-overhead scanner built in Rust designed to spot genuine, open, and high-value GitHub bounties within seconds of publication.

Unlike naive scrapers and spam bots that pollute issues with copy-pasted cryptocurrency addresses, **BountySniper-RS** employs strict heuristic validation to filter out 99% of honeypots, scam mirrors, gated collaborator tasks, and overly crowded competitions.

---

## 🛡️ Anti-Spam & Verification Engine

1. **Known Bot Mirror Blacklist:**  
   Automatically eliminates clone networks and AI bot mirrors (`bounty-bot`, `bounty-plaza`, `GLASS`, `SecureBananaLabs`, `doomsday-network`, etc.).
2. **Competition Guardrail:**  
   Inspects open pull requests linked to the issue (`matching_prs`). Issues with >2 competing PRs are flagged as `Contested` or filtered out.
3. **Maintainer Gating Detection:**  
   Parses issue threads for internal lockouts (e.g. `locked to @user`, `must be a core team member / collaborator`).
4. **Reward Extraction:**  
   Uses regex matching for USD, USDC, USDT, DOT, ETH, and DAI rewards, as well as official maintainer labels (`Price: XX USD`, `Bug bounty 💰`, `💎 Bounty`).
5. **Deduplication State Cache:**  
   Maintains a persistent JSON cache (`sniper_cache.json`) to guarantee zero duplicate alerts.

---

## 🚀 Quickstart

### 1. Requirements
- Rust 1.80+ (or use the precompiled binary)
- GitHub Personal Access Token (for high API rate limits)
- Telegram Bot Token

### 2. Run Locally / Continuous Daemon
```bash
# Clone the repository
git clone https://github.com/markankamaransheogarat-sys/bounty-sniper-rs.git
cd bounty-sniper-rs

# Run the daemon
cargo run --release -- \
  --github-token "$GITHUB_TOKEN" \
  --telegram-bot-token "7729508480:AAER27k6f9XScuakIKKRuZ_rU431hfPZe0k" \
  --poll-interval-secs 30
```

### 3. Run in Single-Shot Mode
```bash
cargo run --release -- --once
```

---

## 🤖 Telegram Bot Configuration

1. Open Telegram and search for [@amadeuus_bot](https://t.me/amadeuus_bot).
2. Tap `/start` or send any message.
3. The scanner will automatically detect your `chat_id` on the first scan cycle and begin dispatching alerts!

---

## ☁️ Running 24/7 in GitHub Actions

The repository includes a ready-to-use GitHub Actions workflow [`.github/workflows/sniper.yml`](.github/workflows/sniper.yml) running on a 10-minute cron schedule.

To configure GitHub Cloud execution:
1. Go to repository **Settings -> Secrets and variables -> Actions**.
2. Add repository secrets:
   - `TELEGRAM_BOT_TOKEN`: Your Telegram Bot Token (`7729508480:AAER27k6f9XScuakIKKRuZ_rU431hfPZe0k`)
   - `TELEGRAM_CHAT_ID`: Your Telegram numeric Chat ID (optional, auto-detected if omitted)

---

## 📄 License
MIT License. Authored by [markankamaransheogarat-sys](https://github.com/markankamaransheogarat-sys).
