use ethers::{prelude::Lazy, types::Address};
use provider_utils::enums::ENetwork;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};
use strum_macros::{EnumString, VariantNames};

use super::ZERO_ADDRESS;

pub static UNISWAP2_ROUTERS: Lazy<HashMap<ENetwork, Address>> = Lazy::new(|| {
    HashMap::from([
        (
            ENetwork::BlastMainnet,
            Address::from_str("0xe486EdC84CA7D2579AC9b64e89D9633481A24f11").unwrap(),
        ),
        (
            ENetwork::BlastSepolia,
            Address::from_str("0x3079362534781C17c436cdeaBE2e4BCd92e36e49").unwrap(),
        ),
        (
            ENetwork::EthSepolia,
            Address::from_str("0xC532a74256D3Db42D0Bf7a0400fEFDbad7694008").unwrap(),
        ),
        (
            ENetwork::BaseSepolia,
            Address::from_str("0x78AcC7eD93F0A0f84e9c1B34baFA29Bb26b1383D").unwrap(),
        ),
        (
            ENetwork::BaseMainnet,
            Address::from_str("0x4752ba5DBc23f44D87826276BF6Fd6b1C372aD24").unwrap(),
        ),
        (
            ENetwork::BscMainnet,
            Address::from_str("0x10ED43C718714eb63d5aA57B78B54704E256024E").unwrap(),
        ),
        (
            ENetwork::BscTestnet,
            Address::from_str("0xD99D1c33F9fC3444f8101754aBC46c52416550D1").unwrap(),
        ),
        (
            ENetwork::EthMainnet,
            Address::from_str("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D").unwrap(),
        ),
        (
            ENetwork::FtmTestnet,
            Address::from_str("0xa6AD18C2aC47803E193F75c3677b14BF19B94883").unwrap(),
        ),
        (
            ENetwork::FtmMainnet,
            Address::from_str("0xF491e7B69E4244ad4002BC14e878a34207E38c29").unwrap(),
        ),
    ])
});

pub static UNIVERSAL_ROUTERS: Lazy<HashMap<ENetwork, Address>> = Lazy::new(|| {
    HashMap::from([
        (
            ENetwork::EthSepolia,
            Address::from_str("0x3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD").unwrap(),
        ),
        (
            ENetwork::EthMainnet,
            Address::from_str("0x3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD").unwrap(),
        ),
        (ENetwork::FtmTestnet, *ZERO_ADDRESS),
        (
            ENetwork::FtmMainnet,
            Address::from_str("0x6AB0CA9c94FDE313a3A1d34A8247ae6065Bd2E73").unwrap(),
        ),
        (ENetwork::BlastSepolia, *ZERO_ADDRESS),
        (ENetwork::BlastMainnet, *ZERO_ADDRESS),
    ])
});

pub static UNISWAP3_ROUTERS: Lazy<HashMap<ENetwork, Address>> = Lazy::new(|| {
    HashMap::from([
        (
            ENetwork::EthSepolia,
            Address::from_str("0x3bFA4769FB09eefC5a80d6E87c3B9C650f7Ae48E").unwrap(),
        ),
        (
            ENetwork::EthMainnet,
            Address::from_str("0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45").unwrap(),
        ),
        (ENetwork::FtmTestnet, *ZERO_ADDRESS),
        (
            ENetwork::FtmMainnet,
            Address::from_str("0x40F70B72796C30f355dF859B2c8F94f18c38AdF8").unwrap(),
        ),
        (ENetwork::BlastSepolia, *ZERO_ADDRESS),
        (ENetwork::BlastMainnet, *ZERO_ADDRESS),
    ])
});

pub static UNISWAP3_QUOTER_V2: Lazy<HashMap<ENetwork, Address>> = Lazy::new(|| {
    HashMap::from([
        (
            ENetwork::EthSepolia,
            Address::from_str("0xEd1f6473345F45b75F8179591dd5bA1888cf2FB3").unwrap(),
        ),
        (ENetwork::EthMainnet, *ZERO_ADDRESS),
        (ENetwork::FtmTestnet, *ZERO_ADDRESS),
        (ENetwork::FtmMainnet, *ZERO_ADDRESS),
        (ENetwork::BlastSepolia, *ZERO_ADDRESS),
        (ENetwork::BlastMainnet, *ZERO_ADDRESS),
    ])
});

pub const V3_SWAP_EXACT_IN: u8 = 0;
pub const V3_SWAP_EXACT_OUT: u8 = 1;
pub const V2_SWAP_EXACT_IN: u8 = 8;
pub const V2_SWAP_EXACT_OUT: u8 = 9;

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
pub enum ERouter {
    #[default]
    Uniswap2Routers,
    UniversalRouters,
    Uniswap3Routers,
}
