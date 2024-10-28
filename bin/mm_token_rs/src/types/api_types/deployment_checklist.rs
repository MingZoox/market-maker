use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentChecklist {
    pub token_deployed: TokenDeployed,
    pub whitelist_added: WhitelistAdded,
    pub buyer_balance: BuyerBalance,
    pub seller_approval: SellerApproval,
    pub liquidity_added: LiquidityAdded,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TokenDeployed {
    pub status: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WhitelistAdded {
    pub status: bool,
    pub info: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BuyerBalance {
    pub status: bool,
    pub info: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SellerApproval {
    pub status: bool,
    pub info: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityAdded {
    pub status: bool,
    pub info: String,
}
