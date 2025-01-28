use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[BotCommands(rename_rule = "lowercase")]
pub enum Command {
    #[BotCommands(description = "Start tracking whales")]
    Start,
    #[BotCommands(description = "Stop tracking whales")]
    Stop,
    #[BotCommands(description = "Set minimum transaction amount (in SOL)")]
    SetMinimum { amount: f64 },
    #[BotCommands(description = "Show current tracking settings")]
    Settings,
    #[BotCommands(description = "Show help message")]
    Help,
}