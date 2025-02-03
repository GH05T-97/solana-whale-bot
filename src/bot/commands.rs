use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug)]
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
    MonitorTokenVolume(String),  // Accept input as a single string
}

impl Command {
    pub fn parse_monitor_token_volume(&self) -> Option<(String, f64, f64, u64)> {
        if let Command::MonitorTokenVolume(input) = self {
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() == 4 {
                if let (Ok(min), Ok(max), Ok(duration)) = (
                    parts[1].parse::<f64>(),
                    parts[2].parse::<f64>(),
                    parts[3].parse::<u64>(),
                ) {
                    return Some((parts[0].to_string(), min, max, duration));
                }
            }
        }
        None
    }
}
