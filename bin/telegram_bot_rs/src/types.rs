use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "snake_case",
    description = "These commands are supported:"
)]
pub enum BotCommand {
    #[command(description = "guild for using commands")]
    Help,
    // call APIs command
    #[command(description = "display network status.")]
    GetNetworkStatus,
    // #[command(description = "display deployment checklist.")]
    // GetDeploymentChecklist,
    #[command(description = "display deployer information.")]
    GetDeployer,
    #[command(description = "display buyer wallets information.")]
    GetBuyers,
    #[command(description = "display auto buyer wallets information.")]
    GetAutoBuyers,
    #[command(description = "display seller wallets information.")]
    GetSellers,
    #[command(description = "display market_makers information.")]
    GetMarketMakers,
    #[command(description = "launch process.")]
    LaunchProcess,
    // launch process command
    // #[command(description = "launch buy bot")]
    // LaunchBuyBot,
    // #[command(description = "launch sell bot")]
    // LaunchSellBot,
}
