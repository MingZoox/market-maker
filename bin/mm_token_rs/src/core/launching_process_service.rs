use anyhow::anyhow;
use cached::TimedCache;
use ethers::{
    providers::{Http, Middleware, Provider},
    types::{Bytes, H256, U256},
};
use futures::{future::join_all, FutureExt};
use mm_token_utils::env::get_env;
use provider_utils::http_providers::HttpProviders;
use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};
use tokio::{
    sync::{Mutex, RwLock},
    task::{self, JoinSet},
};

use crate::{constants::Env, routers::RouterService};

use super::{BuyService, GasPrice, MarketMakerService, SellService, WalletService};

#[derive(Debug, Clone)]
pub struct LaunchingProcessService {
    env: Env,
    http_provider: Arc<Provider<Http>>,
}

impl LaunchingProcessService {
    pub fn new(env: Env, http_provider: Arc<Provider<Http>>) -> Self {
        Self { env, http_provider }
    }

    pub async fn active_trading_and_buy(&self) -> anyhow::Result<()> {
        let mut futures = Vec::new();

        let fetched_gas_price = self.http_provider.get_gas_price().await?;
        let gas_price: Arc<RwLock<U256>> = Arc::new(RwLock::new(fetched_gas_price));

        let router_service = RouterService::new(
            self.env.clone(),
            gas_price.clone(),
            self.http_provider.clone(),
        );

        let wallet_service = WalletService::new(self.env.clone(), self.http_provider.clone());
        let provider_index: Arc<RwLock<usize>> = Arc::new(RwLock::new(
            HttpProviders::init_provider_index(&self.env.listen_network, false).await?,
        ));

        let buy_service = BuyService::new(
            self.env.clone(),
            gas_price,
            provider_index.clone(),
            self.http_provider.clone(),
        );
        let sign_txs = buy_service.get_signed_buy_txs().await?;

        let signed_active_trading_tx = router_service.get_active_trading_tx().await?;
        let http_provider = self.http_provider.clone();
        let active_trading_future = task::spawn(async move {
            match http_provider
                .send_raw_transaction(signed_active_trading_tx)
                .await
            {
                Ok(response) => log::info!(
                    "Active trading transaction sent successfully: {:?}",
                    response
                ),
                Err(e) => log::error!("Failed to send active trading transaction: {:?}", e),
            }
        });
        futures.push(active_trading_future.boxed());

        for sign_tx in sign_txs {
            let http_clone = self.http_provider.clone();
            let wallet_service_clone = wallet_service.clone();

            // Spawn async task for each future
            let (sign_tx, wallet_index, buy_nonce) = sign_tx.clone();
            let buy_and_migrate_future = task::spawn(async move {
                match Self::buy_and_migrate_task(
                    wallet_service_clone,
                    sign_tx,
                    http_clone,
                    wallet_index,
                    buy_nonce,
                    fetched_gas_price,
                )
                .await
                {
                    Ok(response) => log::info!(
                        "Buy and migrate task completed successfully: {:?}",
                        response
                    ),
                    Err(e) => log::error!("Failed to complete buy and migrate task: {:?}", e),
                }
            });

            futures.push(buy_and_migrate_future.boxed());
        }

        join_all(futures).await;
        Ok(())
    }

    async fn buy_and_migrate_task(
        wallet_service: WalletService,
        sign_tx: Bytes,
        http_provider: Arc<Provider<Http>>,
        wallet_index: usize,
        buy_nonce: U256,
        fetched_gas_price: U256,
    ) -> anyhow::Result<()> {
        let pending_tx = http_provider.send_raw_transaction(sign_tx).await;

        match pending_tx {
            Ok(_pending_tx) => {
                wallet_service
                    .migrate_token_to_seller_by_index(
                        wallet_index as u32,
                        buy_nonce,
                        fetched_gas_price,
                    )
                    .await?;
                Ok(())
            }
            Err(err) => {
                log::info!(
                    "Pending tx error wallet_index {:?} with err: {:#?}",
                    wallet_index,
                    err
                );
                Ok(())
            }
        }
    }

    pub async fn migrate_tokens(&self) -> anyhow::Result<()> {
        let wallet_service = WalletService::new(self.env.clone(), self.http_provider.clone());
        wallet_service.migrate_token_buyer_to_seller().await?;
        Ok(())
    }

    pub async fn start_auto_sell(&self) -> anyhow::Result<()> {
        let mut set = JoinSet::new();
        let exit = self.env.exit.clone();
        let fetched_gas_price = self.http_provider.get_gas_price().await?;
        let gas_price: Arc<RwLock<U256>> = Arc::new(RwLock::new(fetched_gas_price));
        let provider_index: Arc<RwLock<usize>> = Arc::new(RwLock::new(
            HttpProviders::init_provider_index(&self.env.listen_network, false).await?,
        ));
        set.spawn(GasPrice::fetch_periodically(
            exit.clone(),
            self.env.listen_network,
            provider_index.clone(),
            gas_price.clone(),
            Duration::from_secs(3),
        ));

        let tx_hashes_cache: Arc<Mutex<TimedCache<H256, bool>>> =
            Arc::new(Mutex::new(TimedCache::with_lifespan(120)));

        let auto_sell_event_listen_enabled: bool = get_env("AUTO_SELL_EVENT_LISTEN_ENABLED", None)
            .parse()
            .unwrap();
        if auto_sell_event_listen_enabled {
            let env_clone = self.env.clone();
            let provider_clone = self.http_provider.clone();
            let gas_price_clone = gas_price.clone();
            let provider_index_clone = provider_index.clone();
            let tx_hashes_cache_clone = tx_hashes_cache.clone();

            set.spawn(async move {
                let mut sell_service = SellService::new(
                    env_clone,
                    gas_price_clone,
                    provider_index_clone,
                    provider_clone,
                );
                sell_service.init().await?;
                sell_service.start_event_mode(tx_hashes_cache_clone).await?;
                Ok(())
            });
        }

        // NOTE: base/blast not support stream mempool
        let auto_sell_mempool_listen_enabled: bool =
            get_env("AUTO_SELL_MEMPOOL_LISTEN_ENABLED", None)
                .parse()
                .unwrap();
        if auto_sell_mempool_listen_enabled {
            let env = self.env.clone();
            let gas_price_clone = gas_price.clone();
            let provider_index_clone = provider_index.clone();
            let http_provider_clone = self.http_provider.clone();
            let tx_hashes_cache_clone = tx_hashes_cache.clone();

            set.spawn(async move {
                let mut sell_service = SellService::new(
                    env,
                    gas_price_clone,
                    provider_index_clone,
                    http_provider_clone,
                );
                sell_service.init().await?;
                sell_service
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

        Err(anyhow!("AutoSell process is stopped !!"))
    }

    pub async fn start_market_making(&self) -> anyhow::Result<()> {
        let fetched_gas_price = self.http_provider.get_gas_price().await?;
        let gas_price: Arc<RwLock<U256>> = Arc::new(RwLock::new(fetched_gas_price));
        let market_maker_service =
            MarketMakerService::new(self.env.clone(), gas_price, self.http_provider.clone());

        market_maker_service.market_make().await?;
        Ok(())
    }
}
