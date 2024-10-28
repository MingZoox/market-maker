use ethers::types::Address;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStatus {
    pub network: NetworkStatusNetworkInfo,
    pub token: NetworkStatusTokenInfo,
    pub router: NetworkStatusRouterInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStatusNetworkInfo {
    pub name: String,
    pub chain_id: u64,
    pub block_number: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStatusTokenInfo {
    pub address: Address,
    pub is_deployed: bool,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub total_supply: u128,
    pub token_template: TokenTemplate,
    pub router_contract: Address,
    pub pair_contract: Address,
    pub weth: Address,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub enum TokenTemplate {
    #[default]
    BaseMemeTokenV1,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStatusRouterInfo {
    pub avabot: Address,
}
