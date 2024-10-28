use ethers::prelude::Lazy;
use std::collections::HashMap;

use crate::enums::ENetwork;

#[derive(Debug, Clone, Default)]
pub struct NetworkConfig {
    pub network: ENetwork,
    pub chain_id: u64,
    pub rpc_url: UrlConfig,
    pub ws_url: UrlConfig,
}

#[derive(Debug, Clone, Default)]
pub struct UrlConfig {
    pub internal: Vec<String>,
    pub external: Vec<String>,
}

pub static NETWORKS: Lazy<HashMap<ENetwork, NetworkConfig>> = Lazy::new(|| {
    HashMap::from([
        (
            ENetwork::BlastMainnet,
            NetworkConfig {
                network: ENetwork::BlastMainnet,
                chain_id: 81457,
                rpc_url: UrlConfig {
                    // Local node: http://10.2.15.108:9545/
                    internal: vec!["http://34.171.16.239:9545".to_string()],
                    external: vec!["http://34.171.16.239:9545".to_string()],
                },
                ws_url: UrlConfig {
                    // Local node: ws://10.2.15.108:9546/
                    internal: vec!["ws://34.171.16.239:9546".to_string()],
                    external: vec!["ws://34.171.16.239:9546".to_string()],
                },
            },
        ),
        (
            ENetwork::BlastSepolia,
            NetworkConfig {
                network: ENetwork::BlastSepolia,
                chain_id: 168587773,
                rpc_url: UrlConfig {
                    // Local node: http://10.2.15.108:9545/
                    internal: vec![
                        "http://10.2.15.108:9545/".to_string(),
                        "https://smart-burned-shape.blast-sepolia.quiknode.pro/6c705029086a38c8cd49a9d7a4f4942adcfec60f/".to_string(),
                        ],
                    external: vec![
                        "http://10.2.15.108:9545/".to_string(),
                        "https://smart-burned-shape.blast-sepolia.quiknode.pro/6c705029086a38c8cd49a9d7a4f4942adcfec60f/".to_string(),
                        ],
                },
                ws_url: UrlConfig {
                    // Local node: ws://10.2.15.108:9546/
                    internal: vec!["wss://smart-burned-shape.blast-sepolia.quiknode.pro/6c705029086a38c8cd49a9d7a4f4942adcfec60f/".to_string()],
                    external: vec!["wss://smart-burned-shape.blast-sepolia.quiknode.pro/6c705029086a38c8cd49a9d7a4f4942adcfec60f/".to_string()],
                },
            },
        ),
        (
            ENetwork::EthMainnet,
            NetworkConfig {
                network: ENetwork::EthMainnet,
                chain_id: 1,
                rpc_url: UrlConfig {
                    internal: vec!["https://rough-old-energy.quiknode.pro/aafda10004c4ec79c017ba6a6ca44f18dff50321/".to_string()],
                    external: vec!["https://rough-old-energy.quiknode.pro/aafda10004c4ec79c017ba6a6ca44f18dff50321/".to_string()],
                },
                ws_url: UrlConfig {
                    internal: vec!["wss://rough-old-energy.quiknode.pro/aafda10004c4ec79c017ba6a6ca44f18dff50321/".to_string()],
                    external: vec!["wss://rough-old-energy.quiknode.pro/aafda10004c4ec79c017ba6a6ca44f18dff50321/".to_string()],
                },
            },
        ),
        (
            ENetwork::EthSepolia,
            NetworkConfig {
                network: ENetwork::EthSepolia,
                chain_id: 11155111,
                rpc_url: UrlConfig {
                    // internal: vec!["https://sepolia.infura.io/v3/dc281b28d74c4fd081fba6587f46da54".to_string()],
                    internal: vec!["https://ethereum-sepolia.core.chainstack.com/106a8b6b6a7119fe092006ef4c9d5a0b".to_string()],
                    external: vec!["https://ethereum-sepolia.core.chainstack.com/106a8b6b6a7119fe092006ef4c9d5a0b".to_string()],
                },
                ws_url: UrlConfig {
                    // internal: vec!["wss://sepolia.infura.io/ws/v3/dc281b28d74c4fd081fba6587f46da54".to_string()],
                    internal: vec!["wss://ethereum-sepolia.core.chainstack.com/106a8b6b6a7119fe092006ef4c9d5a0b".to_string()],
                    external: vec!["wss://ethereum-sepolia.core.chainstack.com/106a8b6b6a7119fe092006ef4c9d5a0b".to_string()],
                },
            },
        ),
        (
            ENetwork::BaseSepolia,
            NetworkConfig {
                network: ENetwork::BaseSepolia,
                chain_id: 84532,
                rpc_url: UrlConfig {
                    internal: vec!["https://sepolia.base.org".to_string()],
                    external: vec!["https://sepolia.base.org".to_string()],
                },
                ws_url: UrlConfig {
                    internal: vec!["wss://base-sepolia-rpc.publicnode.com".to_string()],
                    external: vec!["wss://base-sepolia-rpc.publicnode.com".to_string()],
                },
            },
        ),
        (
            ENetwork::BaseMainnet,
            NetworkConfig {
                network: ENetwork::BaseMainnet,
                chain_id: 8453,
                rpc_url: UrlConfig {
                    internal: vec!["https://old-cool-yard.base-mainnet.quiknode.pro/4c5bd39e42d1461627f4e97b0eeb5d52b2441971/".to_string()],
                    external: vec!["https://old-cool-yard.base-mainnet.quiknode.pro/4c5bd39e42d1461627f4e97b0eeb5d52b2441971/".to_string()],
                },
                ws_url: UrlConfig {
                    internal: vec!["wss://old-cool-yard.base-mainnet.quiknode.pro/4c5bd39e42d1461627f4e97b0eeb5d52b2441971/".to_string()],
                    external: vec!["wss://old-cool-yard.base-mainnet.quiknode.pro/4c5bd39e42d1461627f4e97b0eeb5d52b2441971/".to_string()],
                },
            },
        ),
        (
            ENetwork::BscMainnet,
            NetworkConfig {
                network: ENetwork::BscMainnet,
                chain_id: 56,
                rpc_url: UrlConfig {
                    internal: vec!["http://172.16.199.37:8545".to_string()],
                    external: vec!["https://bsc-mainnet.rpc.sotatek.works".to_string()],
                },
                ws_url: UrlConfig {
                    internal: vec!["ws://172.16.199.37:8546".to_string()],
                    external: vec!["ws://172.16.199.37:8546".to_string()],
                },
            },
        ),
        (
            ENetwork::BscTestnet,
            NetworkConfig {
                network: ENetwork::BscTestnet,
                chain_id: 97,
                rpc_url: UrlConfig {
                    internal: vec!["http://10.2.15.100:8575".to_string()],
                    external: vec!["https://bsc-testnet.rpc.sotatek.works".to_string()],
                },
                ws_url: UrlConfig {
                    internal: vec!["ws://10.2.15.100:8576".to_string()],
                    external: vec!["ws://10.2.15.100:8576".to_string()],
                },
            },
        ),
        (
            ENetwork::FtmTestnet,
            NetworkConfig {
                network: ENetwork::FtmTestnet,
                chain_id: 4002,
                rpc_url: UrlConfig {
                    internal: vec!["https://nd-939-960-369.p2pify.com/350b0296114dda6d9d407f7f39e88da0".to_string()],
                    external: vec!["https://nd-939-960-369.p2pify.com/350b0296114dda6d9d407f7f39e88da0".to_string()],
                },
                ws_url: UrlConfig {
                    internal: vec!["wss://ws-nd-939-960-369.p2pify.com/350b0296114dda6d9d407f7f39e88da0".to_string()],
                    external: vec!["wss://ws-nd-939-960-369.p2pify.com/350b0296114dda6d9d407f7f39e88da0".to_string()],
                },
            },
        ),
        (
            ENetwork::FtmMainnet,
            NetworkConfig {
                network: ENetwork::FtmMainnet,
                chain_id: 250,
                rpc_url: UrlConfig {
                    internal: vec!["https://thrumming-icy-aura.fantom.quiknode.pro/ddf6ae7c9b63de374380e8459d2a17b049b7e49b/".to_string()],
                    external: vec!["https://thrumming-icy-aura.fantom.quiknode.pro/ddf6ae7c9b63de374380e8459d2a17b049b7e49b/".to_string()],
                },
                ws_url: UrlConfig {
                    internal: vec!["wss://thrumming-icy-aura.fantom.quiknode.pro/ddf6ae7c9b63de374380e8459d2a17b049b7e49b/".to_string()],
                    external: vec!["wss://thrumming-icy-aura.fantom.quiknode.pro/ddf6ae7c9b63de374380e8459d2a17b049b7e49b/".to_string()],
                },
            },
        ),
    ])
});
