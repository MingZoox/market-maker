use crate::types::{DefaultMmSettings, MmSettings};
use ethers::types::Address;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarketMakers {
    pub default_settings: DefaultMmSettings,
    pub status: MarketMakersStatus,
    pub list: Vec<MarketMakersGroup>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarketMakersStatus {
    pub total_balance: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarketMakersGroup {
    pub index: u8,
    pub settings: MmSettings,
    pub mm_wallet_info: Vec<MarketMakersWalletInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarketMakersWalletInfo {
    pub path: String,
    pub address: Address,
    pub balance: String,
    pub token_balance: String,
    pub approvals: ApprovalsMarketMakers,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalsMarketMakers {
    pub token_router: String,
    pub ava_router: String,
}
