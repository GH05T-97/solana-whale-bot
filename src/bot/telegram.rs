#![allow(unused_variables)]
use teloxide::{
    prelude::*,
    dispatching::{HandlerExt, UpdateFilterExt},
    types::ChatId
};
use crate::bot::commands::Command;
use crate::bot::trading::VolumeTracker;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;
use log::{info, warn, error};
pub struct WhaleBot {
    bot: Bot,
    chat_id: i64,
    volume_tracker: Arc<TokioMutex<VolumeTracker>>,  // Changed to TokioMutex
    is_tracking: Arc<TokioMutex<bool>>,  // Changed to TokioMutex
}

impl WhaleBot {
    pub async fn new(token: &str, chat_id: i64) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        info!("Initializing WhaleBot with chat_id: {}", chat_id);
         let bot = Bot::new(token);

        // Implement a retry mechanism for bot initialization
        let mut retry_count = 0;
        let max_retries = 5;

        loop {
            match bot.get_me().await {
                Ok(_) => break,
                Err(e) => {
                    retry_count += 1;
                    if retry_count > max_retries {
                        return Err(format!("Failed to initialize bot after {} retries: {}", max_retries, e).into());
                    }

                    eprintln!("Bot initialization error: {}. Retrying...", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }

        let volume_tracker = VolumeTracker::new(
            "https://api.mainnet-beta.solana.com",
            5000.0,
            10000.0
        );

        Ok(Self {
            bot,
            chat_id,
            volume_tracker: Arc::new(TokioMutex::new(volume_tracker)),
            is_tracking: Arc::new(TokioMutex::new(false)),
        })
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut retry_interval = Duration::from_secs(5);

        loop {
            match self.setup_handlers().await {
                Ok(_) => break,
                Err(e) => {
                    eprintln!("Bot setup error: {}. Retrying in {:?}...", e, retry_interval);
                    tokio::time::sleep(retry_interval).await;

                    // Exponential backoff
                    retry_interval = std::cmp::min(
                        retry_interval * 2,
                        Duration::from_secs(60)
                    );
                }
            }
        }

        Ok(())
    }

    async fn setup_handlers(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Setting up WhaleBot command handlers");
        let bot = self.bot.clone();
        let _chat_id = ChatId(self.chat_id);
        let volume_tracker = Arc::clone(&self.volume_tracker);
        let is_tracking = Arc::clone(&self.is_tracking);

        let bot = self.bot.clone();

        // Clear previous webhook if any
        bot.delete_webhook().send().await?;


        let handler = Update::filter_message()
            .filter_command::<Command>()
            .endpoint(move |bot: Bot, msg: Message, cmd: Command| {
                let volume_tracker = Arc::clone(&volume_tracker);
                let is_tracking = Arc::clone(&is_tracking);
                async move {
                    info!("Received command: {:?} from chat_id: {}", cmd, msg.chat.id);
                    match cmd {
                        Command::Start => {
                            info!("Starting monitoring for chat_id: {}", msg.chat.id);
                            *is_tracking.lock().await = true;

                            let monitor_bot = bot.clone();
                            let monitor_tracker = Arc::clone(&volume_tracker);
                            let monitor_is_tracking = Arc::clone(&is_tracking);
                            let chat_id = msg.chat.id;

                            tokio::spawn(async move {
                                info!("Spawned monitoring task for chat_id: {}", chat_id);
                                while *monitor_is_tracking.lock().await {
                                    info!("Starting trade tracking cycle");
                                    let hot_pairs = {
                                        let mut tracker = monitor_tracker.lock().await;
                                        match tracker.track_trades().await {
                                            Ok(pairs) => {
                                                info!("Successfully tracked trades, found {} hot pairs", pairs.len());
                                                pairs
                                            }
                                            Err(e) => {
                                                error!("Error tracking trades: {}", e);
                                                Vec::new()
                                            }
                                        }
                                    };

                                    for volume in hot_pairs {
                                        if volume.trade_count >= 3 {
                                            info!("Hot trading activity detected for token: {}", volume.token_name);
                                            let message = format!(
                                                "🔥 Hot Trading Activity Detected!\n\
                                                Token: {}\n\
                                                Average Trade: ${:.2}\n\
                                                Spot Trades: {}\n\
                                                AMM Swaps: {}\n\
                                                Total Trades: {}\n\
                                                Total Volume: ${:.2}",
                                                volume.token_name,
                                                volume.average_trade_size,
                                                volume.trade_count,
                                                volume.swap_count,
                                                volume.trade_count + volume.swap_count,
                                                volume.total_volume
                                            );

                                            if let Err(e) = monitor_bot.send_message(ChatId(chat_id.0), message).await {
                                                error!("Error sending message: {}", e);
                                            }
                                        }
                                    }
                                    info!("Sleeping for 30 seconds before next cycle");
                                    tokio::time::sleep(Duration::from_secs(30)).await;
                                }
                                info!("Monitoring task ended for chat_id: {}", chat_id);
                            });

                            info!("Sending start confirmation message");
                            bot.send_message(
                                ChatId(msg.chat.id.0),
                                "🔍 Started monitoring trading volume patterns!"
                            ).await?;
                        },
                        Command::Stop => {
                            info!("Stopping monitoring for chat_id: {}", msg.chat.id);
                            *is_tracking.lock().await = false;
                            bot.send_message(
                                ChatId(msg.chat.id.0),
                                "⏹️ Monitoring stopped. Use /start to resume monitoring."
                            ).await?;
                        },
                        Command::MonitorToken(token_symbol) => {
                            info!("Adding token {} to monitoring list", token_symbol);
                            let mut tracker = volume_tracker.lock().await;
                            match tracker.add_monitored_token(&token_symbol).await {
                                Ok(symbol) => {
                                    bot.send_message(
                                        ChatId(msg.chat.id.0),
                                        format!("🎯 Now monitoring {} token", symbol)
                                    ).await?;
                                }
                                Err(e) => {
                                    bot.send_message(
                                        ChatId(msg.chat.id.0),
                                        format!("❌ Error: {}", e)
                                    ).await?;
                                }
                            }
                        },
                        Command::MonitorTokenVolume(input) => {
                            let parts: Vec<&str> = input.split_whitespace().collect();

                            if parts.len() != 4 {
                                bot.send_message(
                                    ChatId(msg.chat.id.0),
                                    "❌ Invalid format! Use: /monitortokenvolume <token> <min> <max> <timeframe>",
                                )
                                .await?;
                                return Ok(());
                            }

                            let token_symbol = parts[0].to_string();
                            let min: f64 = match parts[1].parse() {
                                Ok(val) => val,
                                Err(_) => {
                                    bot.send_message(ChatId(msg.chat.id.0), "❌ Invalid min volume format!").await?;
                                    return Ok(());
                                }
                            };

                            let max: f64 = match parts[2].parse() {
                                Ok(val) => val,
                                Err(_) => {
                                    bot.send_message(ChatId(msg.chat.id.0), "❌ Invalid max volume format!").await?;
                                    return Ok(());
                                }
                            };

                            let timeframe: u64 = match parts[3].parse() {
                                Ok(val) => val,
                                Err(_) => {
                                    bot.send_message(ChatId(msg.chat.id.0), "❌ Invalid timeframe format!").await?;
                                    return Ok(());
                                }
                            };

                            info!(
                                "Updating volume thresholds for {}: min=${}, max=${}, timeframe={}min",
                                token_symbol, min, max, timeframe
                            );

                            let mut tracker = volume_tracker.lock().await;

                            match tracker.get_token_info(&token_symbol).await {
                                Ok(token_info) => {
                                    if !tracker.monitored_tokens.contains(&token_info.address) {
                                        bot.send_message(
                                            ChatId(msg.chat.id.0),
                                            format!(
                                                "❌ Please first add {} to monitoring using /monitorToken",
                                                token_symbol
                                            ),
                                        )
                                        .await?;
                                        return Ok(());
                                    }

                                    tracker.set_token_volume_threshold(token_info.address, min, max, timeframe);
                                    bot.send_message(
                                        ChatId(msg.chat.id.0),
                                        format!(
                                            "📊 Updated monitoring thresholds for {}:\nMin Volume: ${}\nMax Volume: ${}\nTimeframe: {} minutes",
                                            token_symbol, min, max, timeframe
                                        ),
                                    )
                                    .await?;
                                }
                                Err(_) => {
                                    bot.send_message(
                                        ChatId(msg.chat.id.0),
                                        format!("❌ Error: Token {} not found", token_symbol),
                                    )
                                    .await?;
                                }
                            }
                        },
                        _ => {
                            warn!("Unhandled command received: {:?}", cmd);
                        }
                    }
                    Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
                }
            });

            info!("Building dispatcher");
            let mut dispatcher = Dispatcher::builder(bot, handler)
                .enable_ctrlc_handler()
                .build();

                info!("Building dispatcher");
                let dispatcher = Dispatcher::builder(bot, handler)
                    .enable_ctrlc_handler()
                    .build();

                // Use long polling with a timeout
                tokio::select! {
                    dispatch_result = dispatcher.dispatch() => {
                        if let Err(e) = dispatch_result {
                            error!("Dispatcher error: {}", e);
                            return Err(e.into());
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_secs(3600)) => {
                        error!("Dispatcher timeout, restarting...");
                        return Err("Dispatcher timeout".into());
                    }
                }

                Ok(())
    }
}