use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use cached::TimedCache;
use ethers::{
    providers::Middleware,
    types::{H256, U256},
};
use mm_token_rs::{
    constants::Env,
    core::{AutoBuyService, GasPrice},
};
use mm_token_utils::{env::get_env, log::setup_logger};
use provider_utils::http_providers::HttpProviders;
use tokio::{
    sync::{Mutex, RwLock},
    task::JoinSet,
};

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

    let tx_hashes_cache: Arc<Mutex<TimedCache<H256, bool>>> =
        Arc::new(Mutex::new(TimedCache::with_lifespan(120)));

    let auto_buy_event_listen_enabled: bool = get_env("AUTO_BUY_EVENT_LISTEN_ENABLED", None)
        .parse()
        .unwrap();
    if auto_buy_event_listen_enabled {
        let env = env.clone();
        let gas_price = gas_price.clone();
        let provider_index = provider_index.clone();
        let http_provider = http_provider.clone();
        let tx_hashes_cache_clone = tx_hashes_cache.clone();
        set.spawn(async {
            let mut auto_buy_service =
                AutoBuyService::new(env, gas_price, provider_index, http_provider);
            auto_buy_service.init().await?;
            auto_buy_service
                .start_event_mode(tx_hashes_cache_clone)
                .await?;
            Ok(())
        });
    }

    // NOTE: base/blast not support stream mempool
    let auto_buy_mempool_listen_enabled: bool = get_env("AUTO_BUY_MEMPOOL_LISTEN_ENABLED", None)
        .parse()
        .unwrap();
    if auto_buy_mempool_listen_enabled {
        let env = env.clone();
        let gas_price = gas_price.clone();
        let provider_index = provider_index.clone();
        let http_provider = http_provider.clone();
        let tx_hashes_cache_clone = tx_hashes_cache.clone();

        set.spawn(async {
            let mut auto_buy_service =
                AutoBuyService::new(env, gas_price, provider_index, http_provider);
            auto_buy_service.init().await?;
            auto_buy_service
                .start_mempool_mode(tx_hashes_cache_clone)
                .await?;
            Ok(())
        });
    }

    while let Some(res) = set.join_next().await {
        log::error!("program exited, res {:?}", res);
        // gracefully shutdown
        exit.store(true, Ordering::Relaxed);
    }

    Ok(())
}
