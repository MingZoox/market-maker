use serde::{Deserialize, Serialize};
use strum_macros::{EnumString, VariantNames};

#[derive(
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    Clone,
    Copy,
    EnumString,
    VariantNames,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum ENetwork {
    #[default]
    BlastMainnet,
    BlastSepolia,
    EthMainnet,
    EthSepolia,
    BaseMainnet,
    BaseSepolia,
    BscMainnet,
    BscTestnet,
    FtmTestnet,
    FtmMainnet,
}
