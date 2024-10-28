use ethers::types::{Bytes, H256, U64};
use ethers_flashbots::{BundleRequest, BundleTransaction};
use itertools::Itertools;
use serde::{Serialize, Serializer};
use strum_macros::EnumString;

#[derive(Clone, Debug, Default, Serialize, EnumString)]
pub enum BloxrouteBundleNetwork {
    #[default]
    #[serde(rename = "Mainnet")]
    #[strum(serialize = "Mainnet")]
    Mainnet,
    #[serde(rename = "BSC-Mainnet")]
    #[strum(serialize = "BSC-Mainnet")]
    BscMainnet,
    #[serde(rename = "Polygon-Mainnet")]
    #[strum(serialize = "Polygon-Mainnet")]
    PolygonMainnet,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct BloxrouteBundleRequest {
    #[serde(rename = "transaction")]
    #[serde(serialize_with = "bloxroute_serialize_txs")]
    pub transactions: Vec<BundleTransaction>,
    #[serde(rename = "reverting_hashes")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub revertible_transaction_hashes: Vec<H256>,

    #[serde(rename = "block_number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_block: Option<U64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_timestamp: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_timestamp: Option<u64>,

    #[serde(rename = "state_block_number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simulation_block: Option<U64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "timestamp")]
    pub simulation_timestamp: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "baseFee")]
    pub simulation_basefee: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub blockchain_network: Option<BloxrouteBundleNetwork>,
}

impl From<&BundleRequest> for BloxrouteBundleRequest {
    fn from(value: &BundleRequest) -> Self {
        Self {
            transactions: value.transactions().clone(),
            revertible_transaction_hashes: Default::default(),
            target_block: value.block(),
            min_timestamp: value.min_timestamp(),
            max_timestamp: value.max_timestamp(),
            simulation_block: value.simulation_block(),
            simulation_timestamp: value.simulation_timestamp(),
            simulation_basefee: value.simulation_basefee(),
            blockchain_network: Default::default(),
        }
    }
}

pub fn bloxroute_serialize_txs<S>(txs: &[BundleTransaction], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let raw_txs: Vec<Bytes> = txs
        .iter()
        .map(|tx| match tx {
            BundleTransaction::Signed(inner) => inner.rlp(),
            BundleTransaction::Raw(inner) => inner.clone(),
        })
        .collect();

    // bloxroute require hex without 0x
    let raw_txs_string: Vec<String> = raw_txs
        .iter()
        .map(|x| String::from(&x.to_string()[2..]))
        .collect_vec();

    raw_txs_string.serialize(s)
}
