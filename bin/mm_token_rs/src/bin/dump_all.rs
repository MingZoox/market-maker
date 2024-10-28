use std::{env, sync::Arc};

use ethers::{providers::Middleware, types::U256};
use mm_token_rs::{constants::Env, core::WalletService};
use mm_token_utils::log::setup_logger;
use provider_utils::http_providers::HttpProviders;
use tokio::sync::RwLock;

const DEFAULT_INTERVAL: u32 = 600;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    setup_logger(None)?;

    let args: Vec<String> = env::args().collect();
    let (dump_interval_min, dump_interval_max) = if args.len() != 3 {
        log::warn!(
            "Function {} need 2 params: <dump-interval-min> <dump-interval-max>, set config to default 600s",
            args[0]
        );

        (DEFAULT_INTERVAL, DEFAULT_INTERVAL)
    } else {
        // Extract the arguments (skipping the program name)
        let dump_interval_min: u32 = args[1].parse().unwrap();
        let dump_interval_max: u32 = args[2].parse().unwrap();

        (dump_interval_min, dump_interval_max)
    };

    let env = Env::new();
    let http_provider =
        Arc::new(HttpProviders::get_healthy_provider(&env.listen_network, false).await?);
    let fetched_gas_price = http_provider.get_gas_price().await?;
    let gas_price: Arc<RwLock<U256>> = Arc::new(RwLock::new(fetched_gas_price));
    let wallet_service = WalletService::new(env, http_provider);
    wallet_service
        .dump_all(gas_price, dump_interval_min, dump_interval_max)
        .await?;

    Ok(())
}
