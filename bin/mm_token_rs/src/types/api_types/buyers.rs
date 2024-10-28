use ethers::types::Address;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Buyers {
    pub settings: BuyersSettings,
    pub status: BuyersStatus,
    pub list: Vec<BuyersWalletInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BuyersSettings {
    pub surplus_amount: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BuyersStatus {
    pub total_balance: String,
    pub total_token_balance: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BuyersWalletInfo {
    pub path: String,
    pub address: Address,
    pub balance: String,
    pub token_balance: String,
}
