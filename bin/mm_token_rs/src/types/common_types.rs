use ethers::types::{Address, H160, U256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TokenInfo {
    pub address: H160,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub total_supply: U256,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct CheckMnemonicWalletInfo {
    pub path: String,
    pub address: Address,
    pub private_key: String,
}
