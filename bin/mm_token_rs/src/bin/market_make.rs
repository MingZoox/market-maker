use std::sync::Arc;

use ethers::{providers::Middleware, types::U256};
use mm_token_rs::{constants::Env, core::MarketMakerService};
use mm_token_utils::log::setup_logger;
use provider_utils::http_providers::HttpProviders;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    setup_logger(None)?;

    let env = Env::new();
    let http_provider =
        Arc::new(HttpProviders::get_healthy_provider(&env.listen_network, false).await?);

    let fetched_gas_price = http_provider.get_gas_price().await?;
    let gas_price: Arc<RwLock<U256>> = Arc::new(RwLock::new(fetched_gas_price));
    let market_maker_service =
        MarketMakerService::new(env.clone(), gas_price, http_provider.clone());

    market_maker_service.market_make().await?;
    Ok(())
}
