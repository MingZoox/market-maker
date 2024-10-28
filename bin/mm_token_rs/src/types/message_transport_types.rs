use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TelegramConfig {
    pub telegram_bot_token: String,
    pub telegram_channel_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EmailConfig {
    pub user_name: String,
    pub password: String,
}
