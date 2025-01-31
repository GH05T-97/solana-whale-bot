use teloxide::{
    prelude::*,
    dispatching::{HandlerExt, UpdateFilterExt},
    types::ChatId
};
use crate::bot::commands::Command;
use crate::bot::trading::VolumeTracker;
use std::time::Duration;
use std::sync::{Arc, Mutex};

pub struct WhaleBot {
    bot: Bot,
    chat_id: i64,
    volume_tracker: Arc<Mutex<VolumeTracker>>, // Wrap in Arc<Mutex> for thread safety
    is_tracking: Arc<Mutex<bool>>,
}

impl WhaleBot {
    pub async fn new(token: &str, chat_id: i64) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let bot = Bot::new(token);
        let volume_tracker = VolumeTracker::new(
            "https://api.mainnet-beta.solana.com",
            5000.0, // $5k minimum
            10000.0, // $10k maximum
        );

        Ok(Self {
            bot,
            chat_id,
            volume_tracker: Arc::new(Mutex::new(volume_tracker)), // Initialize as Arc<Mutex>
            is_tracking: Arc::new(Mutex::new(false)), // Initialize as Arc<Mutex>
        })
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting trading volume monitor...");
        self.setup_handlers().await?;
        Ok(())
    }

    async fn setup_handlers(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let bot = self.bot.clone();
        let chat_id = ChatId(self.chat_id);
        let volume_tracker = Arc::clone(&self.volume_tracker);
        let is_tracking = Arc::clone(&self.is_tracking);

        let handler = Update::filter_message()
            .filter_command::<Command>()
            .endpoint(move |bot: Bot, msg: Message, cmd: Command| {
                let volume_tracker = Arc::clone(&volume_tracker);
                let is_tracking = Arc::clone(&is_tracking);
                async move {
                    match cmd {
                        Command::Start => {
                            *is_tracking.lock().unwrap() = true;

                            // Start the monitoring in a separate task
                            let monitor_bot = bot.clone();
                            let monitor_tracker = Arc::clone(&volume_tracker);
                            let monitor_is_tracking = Arc::clone(&is_tracking);
                            let chat_id = msg.chat.id;

                            tokio::spawn(async move {
                                while *monitor_is_tracking.lock().unwrap() {
                                    let hot_pairs = {
                                        let mut tracker = monitor_tracker.lock().unwrap();
                                        tracker.track_trades().await.unwrap_or_else(|_| Vec::new())
                                    };

                                    for volume in hot_pairs {
                                        if volume.trade_count >= 3 {
                                            let message = format!(
                                                "🔥 Hot Trading Activity Detected!\n\
                                                Token: {}\n\
                                                Average Trade: ${:.2}\n\
                                                Number of Trades: {}\n\
                                                Total Volume: ${:.2}",
                                                volume.token_name,
                                                volume.average_trade_size,
                                                volume.trade_count,
                                                volume.total_volume
                                            );

                                            if let Err(e) = monitor_bot.send_message(ChatId(chat_id.0), message).await {
                                                println!("Error sending message: {}", e);
                                            }
                                        }
                                    }
                                    tokio::time::sleep(Duration::from_secs(30)).await;
                                }
                            });

                            bot.send_message(
                                ChatId(msg.chat.id.0),
                                "🔍 Started monitoring trading volume patterns!"
                            ).await?;
                        },
                        Command::Stop => {
                            *is_tracking.lock().unwrap() = false;
                            bot.send_message(
                                ChatId(msg.chat.id.0),
                                "⏹️ Monitoring stopped. Use /start to resume monitoring."
                            ).await?;
                        },
                        Command::HotPairs => {
                            let hot_pairs = {
                                let tracker = volume_tracker.lock().unwrap();
                                tracker.get_hot_pairs().clone() // Clone the data to avoid holding the lock
                            };

                            if hot_pairs.is_empty() {
                                bot.send_message(
                                    ChatId(msg.chat.id.0),
                                    "📊 No active trading pairs in the specified range found yet."
                                ).await?;
                            } else {
                                let mut message = String::from("🔥 Current Hot Trading Pairs:\n\n");

                                for pair in hot_pairs {
                                    message.push_str(&format!(
                                        "Token: {}\n\
                                        Average Trade: ${:.2}\n\
                                        Number of Trades: {}\n\
                                        Total Volume: ${:.2}\n\n",
                                        pair.token_name,
                                        pair.average_trade_size,
                                        pair.trade_count,
                                        pair.total_volume
                                    ));
                                }

                                bot.send_message(ChatId(msg.chat.id.0), message).await?;
                            }
                        },
                        _ => {} // Handle other commands
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


    async fn monitor_volume(&mut self) {
        let bot = self.bot.clone();
        let chat_id = self.chat_id;
        while self.is_tracking {
            if let Ok(hot_pairs) = self.volume_tracker.track_trades().await {
                for volume in hot_pairs {
                    if volume.trade_count >= 3 {
                        let message = format!(
                            "🔥 Hot Trading Activity Detected!\n\
                            Token: {}\n\
                            Average Trade: ${:.2}\n\
                            Number of Trades: {}\n\
                            Total Volume: ${:.2}",
                            volume.token_name,
                            volume.average_trade_size,
                            volume.trade_count,
                            volume.total_volume
                        );

                        if let Err(e) = bot.send_message(ChatId(chat_id), message).await {  // Convert here
                            println!("Error sending message: {}", e);
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    }
}