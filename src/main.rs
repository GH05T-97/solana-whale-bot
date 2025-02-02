use dotenv::dotenv;
use solana_whale_trader::bot::WhaleBot;
use std::env;
use env_logger::{Builder, Env};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logger before anything else
    Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    dotenv().ok();

    let token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN must be set");
    let chat_id = env::var("TELEGRAM_CHAT_ID")
        .expect("TELEGRAM_CHAT_ID must be set")
        .parse::<i64>()
        .expect("TELEGRAM_CHAT_ID must be a valid integer");

    let bot = WhaleBot::new(&token, chat_id).await?;
    bot.start().await?;

    Ok(())
}