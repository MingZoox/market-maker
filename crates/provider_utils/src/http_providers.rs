use anyhow::{anyhow, Error};
use ethers::providers::{Http, Middleware, Provider, StreamExt};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    sync::RwLock,
    time::{self, timeout},
};
use tokio_stream::wrappers::IntervalStream;

use crate::{constants::DESERIALIZATION_ERROR_MSG, enums::ENetwork, networks::NETWORKS};

pub struct HttpProviders;

impl HttpProviders {
    pub fn get_providers(
        network: &ENetwork,
        is_external_rpc: bool,
    ) -> anyhow::Result<Vec<Provider<Http>>> {
        let Some(network) = NETWORKS.get(network) else {
            panic!("NETWORKS {:?} not found", network);
        };

        let urls = if is_external_rpc {
            &network.rpc_url.external
        } else {
            &network.rpc_url.internal
        };

        let mut providers = Vec::new();
        for url in urls {
            let provider = Provider::<Http>::try_from(url).unwrap();
            providers.push(provider);
        }

        Ok(providers)
    }

    pub async fn get_provider(
        network: &ENetwork,
        is_external_rpc: bool,
        provider_index: Arc<RwLock<usize>>,
    ) -> anyhow::Result<Provider<Http>, Error> {
        let provider_index = *provider_index.read().await;

        let providers = Self::get_providers(network, is_external_rpc).unwrap();

        if provider_index >= providers.len() {
            panic!("Provider Index out of providers list !!");
        }

        Ok(providers[provider_index].clone())
    }

    pub fn get_first_provider(
        network: &ENetwork,
        is_external_rpc: bool,
    ) -> anyhow::Result<Provider<Http>, Error> {
        let providers = Self::get_providers(network, is_external_rpc).unwrap();

        let provider_index = 0;
        if provider_index >= providers.len() {
            panic!("Provider Index out of providers list !!");
        }

        Ok(providers[provider_index].clone())
    }

    pub async fn get_healthy_provider(
        network: &ENetwork,
        is_external_rpc: bool,
    ) -> anyhow::Result<Provider<Http>, Error> {
        let providers = Self::get_providers(network, is_external_rpc).unwrap();

        for provider in providers {
            match provider.get_block_number().await {
                Ok(_) => {
                    return Ok(provider);
                }
                Err(err) => {
                    let err_string = err.to_string();
                    if err_string.contains("failed to lookup address information: nodename nor servname provided, or not known")
                    || err_string.contains(DESERIALIZATION_ERROR_MSG) {
                        log::info!("Provider {:?} is unavailable !!", provider.url().host());
                        continue;
                    }
                    return Err(err.into());
                }
            }
        }

        Err(anyhow!("All providers failed to retrieve the block number"))
    }

    pub async fn init_provider_index(
        network: &ENetwork,
        is_external_rpc: bool,
    ) -> anyhow::Result<usize, Error> {
        let providers = Self::get_providers(network, is_external_rpc).unwrap();

        for (index, provider) in providers.iter().enumerate() {
            match provider.get_block_number().await {
                Ok(_) => {
                    return Ok(index);
                }
                Err(err) => {
                    let err_string = err.to_string();
                    if err_string.contains("failed to lookup address information: nodename nor servname provided, or not known")
                        || err_string.contains(DESERIALIZATION_ERROR_MSG) {
                        log::info!("Provider {:?} is unavailable !!", provider.url().host());
                        continue;
                    }
                    return Err(err.into());
                }
            }
        }

        Err(anyhow!("All providers failed to retrieve the block number"))
    }

    // Update the provider index
    pub async fn fetch_periodically(
        network: ENetwork,
        is_external_rpc: bool,
        exit: Option<Arc<AtomicBool>>,
        provider_index: Arc<RwLock<usize>>,
    ) -> anyhow::Result<()> {
        let mut stream = IntervalStream::new(time::interval(Duration::from_millis(500)));
        let providers = Self::get_providers(&network, is_external_rpc).unwrap();
        loop {
            if let Some(exit) = &exit {
                if exit.load(Ordering::Relaxed) {
                    return Err(anyhow!("[HttpProviders] exit={:?}", exit));
                }
            }

            let Ok(_) = timeout(Duration::from_millis(100), stream.next()).await else {
                continue;
            };

            for (index, provider) in providers.iter().enumerate() {
                match provider.get_block_number().await {
                    Ok(_) => {
                        let mut provider_index = provider_index.write().await;
                        *provider_index = index;
                        drop(provider_index);
                        break;
                    }
                    Err(err) => {
                        let err_string = err.to_string();
                        if err_string.contains("failed to lookup address information: nodename nor servname provided, or not known")
                        || err_string.contains(DESERIALIZATION_ERROR_MSG) {
                            log::info!("Provider {:?} is down !!", provider.url().host());
                            if index == providers.len() - 1 {
                                if let Some(exit) = &exit {
                                    log::info!("All Providers are down !!");
                                    exit.store(true, Ordering::Relaxed);
                                }
                            }
                            continue;
                        }
                        if let Some(exit) = &exit {
                            exit.store(true, Ordering::Relaxed);
                        }
                        return Err(err.into());
                    }
                }
            }
        }
    }
}
