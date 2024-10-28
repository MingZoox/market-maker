use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MmConfig {
    pub default_settings: DefaultMmSettings,
    pub groups: Vec<MmSettings>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DefaultMmSettings {
    pub max_wallets_count: u32,
    pub min_buy_volume: f32,
    pub max_buy_volume: f32,
    pub min_delay_time: u64,
    pub max_delay_time: u64,
    pub min_retain_token: u32,
    pub max_retain_token: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MmSettings {
    pub mnemonic: String,
    pub max_wallets_count: Option<u32>,
    pub min_buy_volume: Option<f32>,
    pub max_buy_volume: Option<f32>,
    pub min_delay_time: Option<u64>,
    pub max_delay_time: Option<u64>,
    pub min_retain_token: Option<u32>,
    pub max_retain_token: Option<u32>,
}
