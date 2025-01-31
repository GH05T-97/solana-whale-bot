use teloxide::{
    prelude::*,
    dispatching::{HandlerExt, UpdateFilterExt},
};
use crate::bot::commands::Command;
use crate::bot::trading::VolumeTracker;
use std::time::Duration;
use std::sync::{Arc, Mutex};

pub struct WhaleBot {
    bot: Bot,
    chat_id: i64,
    volume_tracker: VolumeTracker,
    is_tracking: bool,
}

impl WhaleBot {
    pub async fn new(token: &str, chat_id: i64) -> Result<Self, Box<dyn std::error::Error>> {
        let bot = Bot::new(token);
        let volume_tracker = VolumeTracker::new(
            "https://api.mainnet-beta.solana.com",
            5000.0, // $5k minimum
            10000.0, // $10k maximum
        );

        Ok(Self {
            bot,
            chat_id,
            volume_tracker,
            is_tracking: false,
        })
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting trading volume monitor...");
        self.setup_handlers().await?;
        Ok(())
    }

    async fn setup_handlers(&self) -> Result<(), Box<dyn std::error::Error>> {
        let bot = self.bot.clone();
        let chat_id = self.chat_id;
        let volume_tracker = Arc::new(Mutex::new(self.volume_tracker.clone()));
        let is_tracking = Arc::new(Mutex::new(self.is_tracking));

        let handler = Update::filter_message()
            .filter_command::<Command>()
            .endpoint(move |bot: Bot, msg: Message, cmd: Command| {
                let volume_tracker = volume_tracker.clone();
                async move {
                    match cmd {
                        Command::Start => {
                            *is_tracking.lock().unwrap() = true;

                            // Start the monitoring in a separate task
                            let monitor_bot = bot.clone();
                            let monitor_tracker = Arc::clone(&volume_tracker);
                            let monitor_is_tracking = Arc::clone(&is_tracking);

                            tokio::spawn(async move {
                                while *monitor_is_tracking.lock().unwrap() {
                                    if let Ok(hot_pairs) = monitor_tracker.lock().unwrap().track_trades().await {
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

                                                if let Err(e) = monitor_bot.send_message(chat_id, message).await {
                                                    println!("Error sending message: {}", e);
                                                }
                                            }
                                        }
                                    }
                                    tokio::time::sleep(Duration::from_secs(30)).await;
                                }
                            });

                            bot.send_message(
                                msg.chat.id,
                                "🔍 Started monitoring trading volume patterns!"
                            ).await?;
                        },
                        Command::Stop => {
                            *is_tracking.lock().unwrap() = false;
                            bot.send_message(
                                msg.chat.id,
                                "⏹️ Monitoring stopped. Use /start to resume monitoring."
                            ).await?;
                        },
                        Command::SetMinVolume { amount } => {
                            if amount > 0.0 && amount < 10000.0 {
                                bot.send_message(
                                    msg.chat.id,
                                    format!("✅ Minimum volume threshold set to ${:.2}", amount)
                                ).await?;
                            } else {
                                bot.send_message(
                                    msg.chat.id,
                                    "❌ Invalid amount. Please set a value between $0 and $10,000"
                                ).await?;
                            }
                        },
                        Command::SetMaxVolume { amount } => {
                            if amount > 5000.0 && amount <= 10000.0 {
                                bot.send_message(
                                    msg.chat.id,
                                    format!("✅ Maximum volume threshold set to ${:.2}", amount)
                                ).await?;
                            } else {
                                bot.send_message(
                                    msg.chat.id,
                                    "❌ Invalid amount. Please set a value between $5,000 and $10,000"
                                ).await?;
                            }
                        },
                        Command::HotPairs => {
                            let hot_pairs = volume_tracker.get_hot_pairs();

                            if hot_pairs.is_empty() {
                                bot.send_message(
                                    msg.chat.id,
                                    "No hot trading pairs found at the moment."
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

                                bot.send_message(msg.chat.id, message).await?;
                            }
                        },
                        Command::Settings => {
                            let settings_message = format!(
                                "⚙️ Current Settings:\n\
                                Minimum Volume: ${:.2}\n\
                                Maximum Volume: ${:.2}\n\
                                Alert Mode: {}\n\
                                Monitoring Interval: 30 seconds\n\
                                \n\
                                Use these commands to adjust:\n\
                                /setminvolume - Set minimum volume\n\
                                /setmaxvolume - Set maximum volume",
                                volume_tracker.min_volume,
                                volume_tracker.max_volume,
                                if true { "Active" } else { "Inactive" }
                            );

                            bot.send_message(msg.chat.id, settings_message).await?;
                        },
                        Command::Help => {
                            let help_text = "🤖 Trading Volume Monitor\n\n\
                                Available Commands:\n\
                                /start - Start monitoring trading volume\n\
                                /stop - Stop monitoring\n\
                                /setminvolume - Set minimum volume threshold\n\
                                /setmaxvolume - Set maximum volume threshold\n\
                                /hotpairs - Show current hot trading pairs\n\
                                /settings - Show current settings\n\
                                /help - Show this message\n\n\
                                The bot monitors trading activity and alerts you when it detects\n\
                                concentrated trading volume between $5,000 and $10,000.";

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

    async fn monitor_volume(&mut self) {
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

                        if let Err(e) = self.bot.send_message(self.chat_id, message).await {
                            println!("Error sending message: {}", e);
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    }
}