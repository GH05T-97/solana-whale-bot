use teloxide::utils::command::BotCommands;
use std::collections::HashSet;

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    #[command(description = "Start monitoring trades")]
    Start,
    #[command(description = "Stop monitoring trades")]
    Stop,
    #[command(description = "Monitor specific token - Usage: /monitorToken <token_symbol>")]
    MonitorToken(String),
    #[command(description = "Set volume threshold for token - Usage: /monitorTokenVolume <token_symbol> <min_volume> <max_volume> <timeframe_minutes>")]
    MonitorTokenVolume(String, f64, f64, u64),
}