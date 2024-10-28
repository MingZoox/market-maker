use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::anyhow;
use ethers::{
    providers::{Middleware, StreamExt},
    types::U256,
};
use provider_utils::{enums::ENetwork, http_providers::HttpProviders};
use tokio::{
    sync::RwLock,
    time::{self, timeout},
};
use tokio_stream::wrappers::IntervalStream;

pub struct GasPrice;

impl GasPrice {
    pub async fn fetch_periodically(
        exit: Arc<AtomicBool>,
        network: ENetwork,
        provider_index: Arc<RwLock<usize>>,
        gas_price: Arc<RwLock<U256>>,
        duration: Duration,
    ) -> anyhow::Result<()> {
        let mut stream = IntervalStream::new(time::interval(duration));
        loop {
            if exit.load(Ordering::Relaxed) {
                return Err(anyhow!("[GasPrice] exit={:?}", exit));
            }
            let Ok(_) = timeout(Duration::from_millis(100), stream.next()).await else {
                continue;
            };

            // get healthy provider
            let http_provider = Arc::new(
                HttpProviders::get_provider(&network, false, provider_index.clone()).await?,
            );

            let fetched_gas_price = match http_provider.get_gas_price().await {
                Ok(gas_price) => gas_price,
                Err(err) => {
                    if err
                        .to_string()
                        .contains("Deserialization Error: expected value at line 1 column 1.")
                    {
                        continue;
                    }
                    return Err(err.into());
                }
            };

            let mut gas_price = gas_price.write().await;
            *gas_price = fetched_gas_price;
            drop(gas_price);

            log::info!("[GasPrice] new gas price {:?}", fetched_gas_price);
        }
    }
}
