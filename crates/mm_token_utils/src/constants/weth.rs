use ethers::{prelude::Lazy, types::Address};
use provider_utils::enums::ENetwork;
use std::{collections::HashMap, str::FromStr};

#[derive(Debug, Clone)]
pub struct Erc20Details {
    pub address: Address,
    pub name: String,
    pub symbol: String,
    pub decimals: u64,
}

pub static WRAPPED_NATIVE_TOKENS: Lazy<HashMap<ENetwork, Erc20Details>> = Lazy::new(|| {
    HashMap::from([
        (
            ENetwork::EthMainnet,
            Erc20Details {
                address: Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap(),
                name: String::from("Ethereum"),
                symbol: String::from("ETH"),
                decimals: 18,
            },
        ),
        (
            ENetwork::EthSepolia,
            Erc20Details {
                address: Address::from_str("0x7b79995e5f793A07Bc00c21412e50Ecae098E7f9").unwrap(),
                name: String::from("Sepolia ETH"),
                symbol: String::from("ETH"),
                decimals: 18,
            },
        ),
        (
            ENetwork::BlastMainnet,
            Erc20Details {
                address: Address::from_str("0x4300000000000000000000000000000000000004").unwrap(), // TODO: update this
                name: String::from("Ethereum"),
                symbol: String::from("ETH"),
                decimals: 18,
            },
        ),
        (
            ENetwork::BlastSepolia,
            Erc20Details {
                address: Address::from_str("0x4200000000000000000000000000000000000023").unwrap(),
                name: String::from("Ethereum"),
                symbol: String::from("ETH"),
                decimals: 18,
            },
        ),
        (
            ENetwork::BaseMainnet,
            Erc20Details {
                address: Address::from_str("0x4200000000000000000000000000000000000006").unwrap(),
                name: String::from("Ethereum"),
                symbol: String::from("ETH"),
                decimals: 18,
            },
        ),
        (
            ENetwork::BaseSepolia,
            Erc20Details {
                address: Address::from_str("0x4200000000000000000000000000000000000006").unwrap(),
                name: String::from("Ethereum"),
                symbol: String::from("ETH"),
                decimals: 18,
            },
        ),
        (
            ENetwork::BscMainnet,
            Erc20Details {
                address: Address::from_str("0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c").unwrap(),
                name: String::from("Wrapped BNB"),
                symbol: String::from("WBNB"),
                decimals: 18,
            },
        ),
        (
            ENetwork::BscTestnet,
            Erc20Details {
                address: Address::from_str("0xae13d989daC2f0dEbFf460aC112a837C89BAa7cd").unwrap(),
                name: String::from("Wrapped BNB"),
                symbol: String::from("WBNB"),
                decimals: 18,
            },
        ),
        (
            ENetwork::FtmTestnet,
            Erc20Details {
                address: Address::from_str("0xf1277d1ed8ad466beddf92ef448a132661956621").unwrap(),
                name: String::from("Wrapped Fantom"),
                symbol: String::from("WFTM"),
                decimals: 18,
            },
        ),
        (
            ENetwork::FtmMainnet,
            Erc20Details {
                address: Address::from_str("0x21be370D5312f44cB42ce377BC9b8a0cEF1A4C83").unwrap(),
                name: String::from("Wrapped Fantom"),
                symbol: String::from("WFTM"),
                decimals: 18,
            },
        ),
    ])
});
