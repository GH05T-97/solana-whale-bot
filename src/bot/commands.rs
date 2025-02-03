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
    #[command(description = "Monitor specific token - Usage: /monitorToken <token_symbol>")]
    MonitorToken(String),
    #[command(description = "Set volume threshold for token - Usage: /monitorTokenVolume <token_symbol> <min_volume> <max_volume> <timeframe_minutes>")]
    MonitorTokenVolume(String, f64, f64, u64),
}

impl Command {
    pub fn parse(s: &str) -> Option<Command> {
        let mut parts = s.split_whitespace();
        match parts.next() {
            Some("/start") => Some(Command::Start),
            Some("/stop") => Some(Command::Stop),
            Some("/hotpairs") => Some(Command::HotPairs),
            Some("/monitortoken") => {
                parts.next().map(|token| Command::MonitorToken(token.to_string()))
            },
            Some("/monitortokenvolume") => {
                let token = parts.next()?;
                let min_volume = parts.next()?.parse().ok()?;
                let max_volume = parts.next()?.parse().ok()?;
                let timeframe = parts.next()?.parse().ok()?;
                Some(Command::MonitorTokenVolume(token.to_string(), min_volume, max_volume, timeframe))
            },
            _ => None
        }
    }
}