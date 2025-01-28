use teloxide::{
    prelude::*,
    dispatching::{HandlerExt, UpdateFilterExt},
};
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
            min_amount: 1000.0,
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
        let min_amount = self.min_amount;
        let is_tracking = self.is_tracking;
        let chat_id = self.chat_id;

        let handler = Update::filter_message()
            .filter_command::<Command>()
            .endpoint(move |bot: Bot, msg: Message, cmd: Command| {
                let min_amount = min_amount;
                let is_tracking = is_tracking;
                let chat_id = chat_id;
                async move {
                    match cmd {
                        Command::Start => {
                            bot.send_message(
                                msg.chat.id,
                                format!("🐋 Whale tracking started! Monitoring transactions above {} SOL", min_amount)
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
                            let status = if is_tracking { "Active" } else { "Inactive" };
                            bot.send_message(
                                msg.chat.id,
                                format!("Current Settings:\nMinimum Amount: {} SOL\nTracking: {}", min_amount, status)
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
                    Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
                }
            });

        Dispatcher::builder(bot, handler)
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;

        Ok(())
    }
}