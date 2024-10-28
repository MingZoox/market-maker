use std::{collections::HashMap, str::FromStr};

use ethers::{prelude::Lazy, types::Address};
use provider_utils::enums::ENetwork;

use super::ZERO_ADDRESS;

pub static DISPERSE_ROUTERS: Lazy<HashMap<ENetwork, Address>> = Lazy::new(|| {
    HashMap::from([
        (
            ENetwork::BlastMainnet,
            Address::from_str("0x9eF8e3f5113E34EF3Bf238361a418121f6b2a6F8").unwrap(),
        ),
        (
            ENetwork::BlastSepolia,
            Address::from_str("0x704eAD4302e5DEc19403babEC8dfF9c2843676cA").unwrap(),
        ),
        (
            ENetwork::EthSepolia,
            Address::from_str("0xDc5f5dc99cEa73fd0FD62a6d927067ffb5D54aDf").unwrap(),
        ),
        (ENetwork::BaseMainnet, *ZERO_ADDRESS),
        (
            ENetwork::BaseSepolia,
            Address::from_str("0x40Ca6376038b00Be9e4bB13B7f0CC91DB54E9638").unwrap(),
        ),
        (ENetwork::BscMainnet, *ZERO_ADDRESS),
        (ENetwork::BscTestnet, *ZERO_ADDRESS),
        (
            ENetwork::FtmTestnet,
            Address::from_str("0xfBCC3BF2f664D5512B08D30eFDf0D1E71fCc27e9").unwrap(),
        ),
        (
            ENetwork::FtmMainnet,
            Address::from_str("0x435466c2029A9DCD694bB34E3bAe44c02a404794").unwrap(),
        ),
    ])
});
