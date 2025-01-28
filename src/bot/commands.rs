use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    #[command(description = "Start tracking whales")]
    Start,
    #[command(description = "Stop tracking whales")]
    Stop,
    #[command(description = "Set minimum transaction amount (in SOL)")]
    SetMinimum { amount: f64 },
    #[command(description = "Show current tracking settings")]
    Settings,
    #[command(description = "Show help message")]
    Help,
}