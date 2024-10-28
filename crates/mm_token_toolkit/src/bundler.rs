use anyhow::Result;
use chrono::Utc;
use ethers::prelude::*;
use ethers_flashbots::*;
use mm_token_utils::utils::format_lower_hex;
use provider_utils::enums::ENetwork;
use url::Url;

use crate::bloxroute::{BloxrouteBundleNetwork, BloxrouteMiddleware};

// type SignerProvider = SignerMiddleware<Provider<Http>, LocalWallet>;

#[derive(Debug)]
pub struct Bundler {
    pub network: ENetwork,
    pub bloxroute_builder: BloxrouteMiddleware,
}

#[allow(clippy::new_without_default)]
impl Bundler {
    pub fn new(network: ENetwork, bloxroute_config: BloxrouteConfig) -> Self {
        let bloxroute_builder = BloxrouteMiddleware::new(
            Url::parse(&bloxroute_config.relay_url).unwrap(),
            &bloxroute_config.authorization_key,
        );

        Self {
            network,
            bloxroute_builder,
        }
    }

    pub fn to_bundle<T: Into<BundleTransaction> + Clone>(
        &self,
        signed_txs: &Vec<T>,
        simulation_block: U64,
        target_block: U64,
    ) -> BundleRequest {
        let mut bundle = BundleRequest::new();

        for tx in signed_txs {
            let bundle_tx: BundleTransaction = tx.clone().into();
            bundle = bundle.push_transaction(bundle_tx);
        }

        let current_timestamp = Utc::now().timestamp();
        bundle
            .set_block(target_block)
            .set_simulation_block(simulation_block)
            .set_simulation_timestamp(0)
            .set_min_timestamp(current_timestamp as u64)
            .set_max_timestamp(current_timestamp as u64 + 60) // fixed 1 minute for now
    }

    pub async fn send_bundle(&self, bundle: &BundleRequest) -> Result<Vec<String>> {
        if [ENetwork::BscMainnet, ENetwork::BscTestnet].contains(&self.network) {
            let bloxroute_bundle_hash = self
                .bloxroute_builder
                .send_bundle(bundle, Some(BloxrouteBundleNetwork::BscMainnet))
                .await?;
            return Ok(vec![format_lower_hex(&bloxroute_bundle_hash)]);
        }

        Ok(vec![])
    }
}

#[derive(Debug, Default)]
pub struct BloxrouteConfig {
    pub relay_url: String,
    pub authorization_key: String,
}
