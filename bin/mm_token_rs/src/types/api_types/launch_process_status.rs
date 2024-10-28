use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum StepStatus {
    Pending,
    Activated,
    Error(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LaunchStatus {
    pub active_trading: StepStatus,
    pub buyers_bot_launch: StepStatus,
    pub migrate_tokens_to_seller: StepStatus,
    pub start_auto_sell: StepStatus,
    pub market_making_launch: StepStatus,
}
