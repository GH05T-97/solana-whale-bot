use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    #[command(description = "Start monitoring trading volume")]
    Start,
    #[command(description = "Stop monitoring")]
    Stop,
    #[command(description = "Set minimum volume in USD")]
    SetMinVolume { amount: f64 },
    #[command(description = "Set maximum volume in USD")]
    SetMaxVolume { amount: f64 },
    #[command(description = "Show current hot trading pairs")]
    HotPairs,
    #[command(description = "Show current settings")]
    Settings,
    #[command(description = "Show help message")]
    Help,
}