#![cfg(windows)]

use reqwest::{Client, StatusCode};
use serde_json::Value;
use thiserror::Error;

use super::{LcuCredentials, ACCEPT, DECLINE, READY_CHECK};

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("LCU request failed")]
    Request(#[source] reqwest::Error),
    #[error("LCU returned HTTP status {0}")]
    Status(StatusCode),
}

pub struct LcuClient {
    client: Client,
    base_url: String,
    password: String,
}

impl LcuClient {
    pub fn new(credentials: &LcuCredentials) -> Result<Self, reqwest::Error> {
        // The LCU uses a self-signed certificate. This client is constructed only
        // from a parsed lockfile and always targets literal loopback.
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;
        Ok(Self {
            client,
            base_url: credentials.base_url(),
            password: credentials.password.clone(),
        })
    }

    pub async fn ready_check(&self) -> Result<Option<Value>, TransportError> {
        let response = self
            .client
            .get(format!("{}{}", self.base_url, READY_CHECK))
            .basic_auth("riot", Some(&self.password))
            .send()
            .await
            .map_err(TransportError::Request)?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = response
            .error_for_status()
            .map_err(TransportError::Request)?;
        response
            .json()
            .await
            .map(Some)
            .map_err(TransportError::Request)
    }

    pub async fn accept(&self) -> Result<(), TransportError> {
        self.action(ACCEPT).await
    }
    pub async fn decline(&self) -> Result<(), TransportError> {
        self.action(DECLINE).await
    }

    async fn action(&self, path: &str) -> Result<(), TransportError> {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .basic_auth("riot", Some(&self.password))
            .send()
            .await
            .map_err(TransportError::Request)?;
        response
            .error_for_status()
            .map(|_| ())
            .map_err(TransportError::Request)
    }
}
