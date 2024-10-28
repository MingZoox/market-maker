use std::{
    str::FromStr,
    sync::{atomic::AtomicBool, Arc},
};

use ethers::types::{Address, U64};
use mm_token_utils::env::get_env;
use provider_utils::{enums::ENetwork, networks::NETWORKS};

#[derive(Debug, Clone, Default)]
pub struct Env {
    pub listen_network: ENetwork,
    pub chain_id: U64,
    pub token_address: Address,
    pub exit: Arc<AtomicBool>,
}

impl Env {
    pub fn new() -> Self {
        let network_str = get_env("LISTEN_NETWORK", None);
        let Ok(listen_network) = ENetwork::from_str(&network_str) else {
            panic!("LISTEN_NETWORK {:?} invalid", network_str);
        };
        let Some(network_config) = NETWORKS.get(&listen_network) else {
            panic!("NETWORKS {:?} not found", listen_network);
        };

        let token_address = Address::from_str(&get_env("TOKEN_ADDRESS", None)).unwrap();

        Self {
            listen_network,
            chain_id: U64::from(network_config.chain_id),
            token_address,
            exit: Arc::new(AtomicBool::new(false)),
        }
    }
}
