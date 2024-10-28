use std::{collections::HashMap, str::FromStr};

use ethers::{prelude::Lazy, types::Address};
use provider_utils::enums::ENetwork;

use super::ZERO_ADDRESS;

pub static AVABOT_ROUTERS: Lazy<HashMap<ENetwork, Address>> = Lazy::new(|| {
    HashMap::from([
        (
            ENetwork::BlastMainnet,
            Address::from_str("0xfe12b63decb6BdC5E99f5f9d8379057D4421FEec").unwrap(),
        ),
        (ENetwork::BlastSepolia, *ZERO_ADDRESS),
        (ENetwork::EthSepolia, *ZERO_ADDRESS),
        (ENetwork::BaseMainnet, *ZERO_ADDRESS),
        (ENetwork::FtmTestnet, *ZERO_ADDRESS),
        (ENetwork::FtmMainnet, *ZERO_ADDRESS),
        (
            ENetwork::BaseSepolia,
            Address::from_str("0x921b752eF985dbf4cfD5b1b18ae0BF396a8C5d1B").unwrap(),
        ),
        (
            ENetwork::BscMainnet,
            Address::from_str("0xd1e21e79Ed1dC73d9eA05C879c037C3F6B770474").unwrap(),
        ),
        (
            ENetwork::BscTestnet,
            Address::from_str("0x1F6E2D4123A872F8F476B18787b542a8F061f983").unwrap(),
        ),
    ])
});
