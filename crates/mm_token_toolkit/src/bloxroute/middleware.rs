use ethers_flashbots::{BundleHash, BundleRequest, SimulatedBundle, SimulatedTransaction};
use thiserror::Error;
use url::Url;

use crate::{
    common::{Relay, RelayError, SendBundleResponse},
    utils::clone_bundle_request_without_txs,
};

use super::bundle::{BloxrouteBundleNetwork, BloxrouteBundleRequest};

/// Errors for the Flashbots middleware.
#[derive(Error, Debug)]
pub enum BloxrouteMiddlewareError {
    /// Some parameters were missing.
    ///
    /// For bundle simulation, check that the following are set:
    /// - `simulation_block`
    /// - `simulation_timestamp`
    /// - `block`
    ///
    /// For bundle submission, check that the following are set:
    /// - `block`
    ///
    /// Additionally, `min_timestamp` and `max_timestamp` must
    /// both be set or unset.
    #[error("Some parameters were missing")]
    MissingParameters,
    /// The relay responded with an error.
    #[error(transparent)]
    RelayError(#[from] RelayError),
}

/// A middleware used to send bundles to a Flashbots relay.
#[derive(Debug)]
pub struct BloxrouteMiddleware {
    relay: Relay,
    simulation_relay: Option<Relay>,
    max_txs_in_bundle: usize,
}

impl BloxrouteMiddleware {
    /// Initialize a new Flashbots middleware.
    ///
    /// The signer is used to sign requests to the relay.
    pub fn new(relay_url: impl Into<Url>, authorization_key: &str) -> Self {
        Self {
            relay: Relay::new(relay_url, authorization_key),
            simulation_relay: None,
            max_txs_in_bundle: 15,
        }
    }

    /// Get the relay client used by the middleware.
    pub fn relay(&self) -> &Relay {
        &self.relay
    }

    /// Get the relay client used by the middleware to simulate
    /// bundles if set.
    pub fn simulation_relay(&self) -> Option<&Relay> {
        self.simulation_relay.as_ref()
    }

    /// Bloxroute require Elite account to simulation
    /// ignore, return default value
    pub async fn simulate_bundle(
        &self,
        bundle: &BundleRequest,
        blockchain_network: Option<BloxrouteBundleNetwork>,
    ) -> Result<SimulatedBundle, BloxrouteMiddlewareError> {
        bundle
            .block()
            .and(bundle.simulation_block())
            .and(bundle.simulation_timestamp())
            .ok_or(BloxrouteMiddlewareError::MissingParameters)?;

        let mut bloxroute_bundle = BloxrouteBundleRequest::from(bundle);
        bloxroute_bundle.blockchain_network = blockchain_network;

        let simulated_bundle = SimulatedBundle {
            hash: Default::default(),
            coinbase_diff: Default::default(),
            coinbase_tip: Default::default(),
            gas_price: Default::default(),
            gas_used: Default::default(),
            gas_fees: Default::default(),
            simulation_block: Default::default(),
            transactions: vec![
                SimulatedTransaction {
                    hash: Default::default(),
                    coinbase_diff: Default::default(),
                    coinbase_tip: Default::default(),
                    gas_price: Default::default(),
                    gas_used: Default::default(),
                    gas_fees: Default::default(),
                    from: Default::default(),
                    to: Default::default(),
                    value: Default::default(),
                    error: None,
                    revert: None,
                };
                bundle.transactions().len()
            ],
        };

        // self.simulation_relay
        //     .as_ref()
        //     .unwrap_or(&self.relay)
        //     .request("blxr_simulate_bundle", bloxroute_bundle)
        //     .await
        //     .map_err(BloxrouteMiddlewareError::RelayError)
        Ok(simulated_bundle)
    }

    /// Send a bundle to the relayer.
    ///
    /// See [`eth_sendBundle`][fb_sendBundle] for more information.
    ///
    /// [fb_sendBundle]: https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#eth_sendbundle
    pub async fn send_bundle(
        &self,
        bundle: &BundleRequest,
        blockchain_network: Option<BloxrouteBundleNetwork>,
    ) -> Result<BundleHash, BloxrouteMiddlewareError> {
        // The target block must be set
        bundle
            .block()
            .ok_or(BloxrouteMiddlewareError::MissingParameters)?;

        // `min_timestamp` and `max_timestamp` must both either be unset or set.
        if bundle.min_timestamp().xor(bundle.max_timestamp()).is_some() {
            return Err(BloxrouteMiddlewareError::MissingParameters);
        }

        if bundle.transactions().len() > self.max_txs_in_bundle {
            let bundle_transactions = &bundle.transactions()[..self.max_txs_in_bundle];
            let mut new_bundle = clone_bundle_request_without_txs(bundle);
            for tx in bundle_transactions {
                new_bundle = new_bundle.push_transaction(tx.clone());
            }

            let mut bloxroute_bundle = BloxrouteBundleRequest::from(&new_bundle);
            bloxroute_bundle.blockchain_network = blockchain_network;
            let response: SendBundleResponse = self
                .relay
                .request("blxr_submit_bundle", bloxroute_bundle)
                .await
                .map_err(BloxrouteMiddlewareError::RelayError)?;
            return Ok(response.bundle_hash);
        }

        let mut bloxroute_bundle = BloxrouteBundleRequest::from(bundle);
        bloxroute_bundle.blockchain_network = blockchain_network;
        let response: SendBundleResponse = self
            .relay
            .request("blxr_submit_bundle", bloxroute_bundle)
            .await
            .map_err(BloxrouteMiddlewareError::RelayError)?;
        Ok(response.bundle_hash)
    }
}
