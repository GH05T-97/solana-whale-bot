use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    #[command(description = "Start monitoring trades")]
    Start,
    #[command(description = "Stop monitoring trades")]
    Stop,
    #[command(description = "Show current hot trading pairs")]
    HotPairs,
    #[command(description = "Monitor specific token")]
    MonitorToken(String),
    #[command(description = "Set volume threshold for token")]
    MonitorTokenVolume(String, f64, f64, u64),
}

impl Command {
    pub fn parse_with_bot_commands(text: &str) -> Option<Self> {
        match BotCommands::parse(text, "WhaleTrackBot") {
            Ok(cmd) => Some(cmd),
            Err(_) => None
        }
    }
}