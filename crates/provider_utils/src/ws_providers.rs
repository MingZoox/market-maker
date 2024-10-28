use cached::{Cached, TimedCache};
use ethers::{
    providers::{Middleware, Provider, Ws, WsClientError},
    types::{Filter, Log, H256},
};
use std::sync::Arc;
use tokio::sync::{
    broadcast::{self, Sender},
    Mutex,
};
use tokio_stream::StreamExt;

use crate::{enums::ENetwork, networks::NETWORKS};

pub struct WsProviders;

impl WsProviders {
    pub async fn subscribe_logs_stream(
        network: &ENetwork,
        filter: Filter,
        is_external: bool,
    ) -> anyhow::Result<broadcast::Receiver<Log>> {
        let (event_sender, event_receiver): (Sender<Log>, _) = broadcast::channel(128);
        let ws_providers = Self::get_ws_providers(network, is_external).await?;
        let tx_hashes_cache: Arc<Mutex<TimedCache<H256, bool>>> =
            Arc::new(Mutex::new(TimedCache::with_lifespan(180)));

        for ws_provider in ws_providers {
            let event_sender_clone = event_sender.clone();
            let filter_clone = filter.clone();
            let tx_hashes_cache_clone = tx_hashes_cache.clone();

            tokio::spawn(async move {
                let mut stream = ws_provider
                    .subscribe_logs(&filter_clone)
                    .await
                    .unwrap()
                    .fuse();

                while let Some(event) = stream.next().await {
                    let tx_hash = event.transaction_hash.unwrap();
                    let mut tx_hashes_cache = tx_hashes_cache_clone.lock().await;
                    if tx_hashes_cache.cache_get(&tx_hash).is_none() {
                        event_sender_clone.send(event).unwrap();
                        tx_hashes_cache.cache_set(tx_hash, true);
                    }
                }
            });
        }

        Ok(event_receiver)
    }

    pub async fn get_ws_providers(
        network: &ENetwork,
        is_external: bool,
    ) -> Result<Vec<Provider<Ws>>, WsClientError> {
        let Some(network) = NETWORKS.get(network) else {
            panic!("NETWORKS {:?} not found", network);
        };
        let urls = if is_external {
            &network.ws_url.external
        } else {
            &network.ws_url.internal
        };

        let mut providers = Vec::new();
        for url in urls {
            let ws = Ws::connect(url).await?;
            let provider = Provider::new(ws);
            providers.push(provider);
        }

        Ok(providers)
    }
}
