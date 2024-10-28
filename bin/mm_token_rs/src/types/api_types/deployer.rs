use ethers::types::Address;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Deployer {
    pub address: Address,
    pub balance: String,
}
