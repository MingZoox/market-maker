// use ethers::core::types::U64;
use ethers_flashbots::BundleHash;
use reqwest::{Client, Error as ReqwestError};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;
use url::Url;

use super::jsonrpc::{JsonRpcError, Request, Response};

/// A Flashbots relay client.
///
/// The client automatically signs every request and sets the Flashbots
/// authorization header appropriately with the given signer.
///
/// **Note**: You probably do not want to use this directly, unless
/// you want to interact directly with the Relay. Most users should use
/// [`FlashbotsMiddleware`](crate::FlashbotsMiddleware) instead.
#[derive(Debug)]
pub struct Relay {
    id: AtomicU64,
    client: Client,
    url: Url,
    authorization_key: String,
}

/// Errors for relay requests.
#[derive(Error, Debug)]
pub enum RelayError {
    /// The request failed.
    #[error(transparent)]
    RequestError(#[from] ReqwestError),
    /// The request could not be parsed.
    #[error(transparent)]
    JsonRpcError(#[from] JsonRpcError),
    /// The request parameters were invalid.
    #[error("Client error: {text}")]
    ClientError { text: String },
    /// The request could not be serialized.
    #[error(transparent)]
    RequestSerdeJson(#[from] serde_json::Error),
    /// The response could not be deserialized.
    #[error("Deserialization error: {err}. Response: {text}")]
    ResponseSerdeJson {
        err: serde_json::Error,
        text: String,
    },
}

impl Relay {
    /// Initializes a new relay client.
    pub fn new(url: impl Into<Url>, authorization_key: &str) -> Self {
        Self {
            id: AtomicU64::new(0),
            client: Client::new(),
            url: url.into(),
            authorization_key: authorization_key.to_string(),
        }
    }

    /// Sends a request with the provided method to the relay, with the
    /// parameters serialized as JSON.
    pub async fn request<T: Serialize + Send + Sync, R: DeserializeOwned>(
        &self,
        method: &str,
        params: T,
    ) -> Result<R, RelayError> {
        let next_id = self.id.load(Ordering::SeqCst) + 1;
        self.id.store(next_id, Ordering::SeqCst);

        let payload = Request::new(next_id, method, params);

        let mut req = self.client.post(self.url.as_ref());

        req = req.header("Authorization", self.authorization_key.clone());

        let res = req.json(&payload).send().await?;
        let status = res.error_for_status_ref();

        match status {
            Err(err) => {
                let text = res.text().await?;
                let status_code = err.status().unwrap();
                if status_code.is_client_error() {
                    // Client error (400-499)
                    Err(RelayError::ClientError { text })
                } else {
                    // Internal server error (500-599)
                    Err(RelayError::RequestError(err))
                }
            }
            Ok(_) => {
                let text = res.text().await?;
                let res: Response<R> = serde_json::from_str(&text)
                    .map_err(|err| RelayError::ResponseSerdeJson { err, text })?;

                Ok(res.data.into_result()?)
            }
        }
    }
}

impl Clone for Relay {
    fn clone(&self) -> Self {
        Self {
            id: AtomicU64::new(0),
            client: self.client.clone(),
            url: self.url.clone(),
            authorization_key: self.authorization_key.clone(),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub(crate) struct SendBundleResponse {
    pub(crate) bundle_hash: BundleHash,
}

// #[derive(Serialize, Debug)]
// #[serde(rename_all = "camelCase")]
// pub(crate) struct GetBundleStatsParams {
//     pub(crate) bundle_hash: BundleHash,
//     pub(crate) block_number: U64,
// }

// #[derive(Serialize, Debug)]
// #[serde(rename_all = "camelCase")]
// pub(crate) struct GetUserStatsParams {
//     pub(crate) block_number: U64,
// }
