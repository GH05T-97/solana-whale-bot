use teloxide::prelude::*;
use crate::bot::commands::Command;

pub struct WhaleBot {
    bot: Bot,
    chat_id: i64,
    min_amount: f64,  
    is_tracking: bool,
}

impl WhaleBot {
    pub async fn new(token: &str, chat_id: i64) -> Result<Self, Box<dyn std::error::Error>> {
        let bot = Bot::new(token);

        Ok(Self {
            bot,
            chat_id,
            min_amount: 1000.0, // Default 1000 SOL
            is_tracking: false,
        })
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting whale tracking bot...");
        self.setup_handlers().await?;
        Ok(())
    }

    async fn setup_handlers(&self) -> Result<(), Box<dyn std::error::Error>> {
        let bot = self.bot.clone();

        teloxide::commands_repl(bot, answer, Command::ty()).await;
        Ok(())
    }
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Start => {
            bot.send_message(
                msg.chat.id,
                "🐋 Whale tracking started! You'll receive alerts for large transactions."
            ).await?;
        }
        Command::Stop => {
            bot.send_message(
                msg.chat.id,
                "Whale tracking stopped."
            ).await?;
        }
        Command::SetMinimum { amount } => {
            bot.send_message(
                msg.chat.id,
                format!("Minimum transaction amount set to {} SOL", amount)
            ).await?;
        }
        Command::Settings => {
            bot.send_message(
                msg.chat.id,
                "Current Settings:\nMinimum Amount: 1000 SOL\nTracking: Active"
            ).await?;
        }
        Command::Help => {
            let help_text = "🐋 Whale Tracker Bot\n\n\
                Commands:\n\
                /start - Start tracking whales\n\
                /stop - Stop tracking whales\n\
                /setminimum <amount> - Set minimum transaction amount in SOL\n\
                /settings - Show current settings\n\
                /help - Show this message";
            bot.send_message(msg.chat.id, help_text).await?;
        }
    }
    Ok(())
}