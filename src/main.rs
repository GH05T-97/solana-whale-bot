// In your main.rs or lib.rs
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up logging
    teloxide::enable_logging!();

    // Read environment variables
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
        .expect("TELEGRAM_BOT_TOKEN must be set");
    let chat_id = std::env::var("TELEGRAM_CHAT_ID")
        .expect("TELEGRAM_CHAT_ID must be set")
        .parse::<i64>()
        .expect("Invalid TELEGRAM_CHAT_ID");

    // Create and start the bot
    let whale_bot = WhaleBot::new(&bot_token, chat_id).await?;

    // Implement a robust main loop with restart capability
    loop {
        match whale_bot.start().await {
            Ok(_) => break,
            Err(e) => {
                eprintln!("Bot encountered an error: {}. Restarting...", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}