use std::sync::Arc;

use mm_token_rs::{constants::Env, core::LaunchingProcessService};
use mm_token_utils::log::setup_logger;
use provider_utils::http_providers::HttpProviders;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    setup_logger(None)?;
    let env = Env::new();
    let http_provider =
        Arc::new(HttpProviders::get_healthy_provider(&env.listen_network, false).await?);

    let launching_service = LaunchingProcessService::new(env, http_provider);
    launching_service.active_trading_and_buy().await?;

    Ok(())
}
