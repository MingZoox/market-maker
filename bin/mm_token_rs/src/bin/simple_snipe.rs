use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use ethers::{providers::Middleware, types::U256};
use mm_token_rs::{
    constants::Env,
    core::{GasPrice, SnipeService},
};
use mm_token_utils::log::setup_logger;
use provider_utils::http_providers::HttpProviders;
use tokio::{sync::RwLock, task::JoinSet};

#[allow(clippy::never_loop)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    setup_logger(None)?;

    let mut set = JoinSet::new();
    let env = Env::new();
    let exit = env.exit.clone();
    let http_provider =
        Arc::new(HttpProviders::get_healthy_provider(&env.listen_network, false).await?);
    let fetched_gas_price = http_provider.get_gas_price().await?;
    let gas_price: Arc<RwLock<U256>> = Arc::new(RwLock::new(fetched_gas_price));

    let provider_index: Arc<RwLock<usize>> = Arc::new(RwLock::new(
        HttpProviders::init_provider_index(&env.listen_network, false).await?,
    ));
    set.spawn(GasPrice::fetch_periodically(
        exit.clone(),
        env.listen_network,
        provider_index.clone(),
        gas_price.clone(),
        Duration::from_secs(3),
    ));
    set.spawn(HttpProviders::fetch_periodically(
        env.listen_network,
        false,
        Some(exit.clone()),
        provider_index.clone(),
    ));
    set.spawn(async {
        let mut snipe_service = SnipeService::new(env, gas_price, provider_index, http_provider);
        snipe_service.init().await?;
        snipe_service.start_event_mode().await?;
        Ok(())
    });

    while let Some(res) = set.join_next().await {
        log::error!("program exited, res {:?}", res);
        // gracefully shutdown
        exit.store(true, Ordering::Relaxed);
    }

    Ok(())
}
