use ethers::types::Address;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Sellers {
    pub settings: SellersSettings,
    pub status: SellersStatus,
    pub list: Vec<SellersWalletInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SellersSettings {
    pub volume_threshold: String,
    pub min_percent: u32,
    pub max_percent: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SellersStatus {
    pub total_balance: String,
    pub total_token_balance: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SellersWalletInfo {
    pub path: String,
    pub address: Address,
    pub balance: String,
    pub token_balance: String,
    pub approvals: ApprovalsSellers,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalsSellers {
    pub token_router: String,
    pub ava_router: String,
}
