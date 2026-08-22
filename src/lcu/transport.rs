#![cfg(windows)]

use reqwest::{Client, StatusCode};
use serde_json::Value;
use thiserror::Error;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::{client::IntoClientRequest, Message}};

use super::{parse_ready_check_event, LcuCredentials, ReadyCheck, ACCEPT, DECLINE, READY_CHECK};

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("LCU request failed")]
    Request(#[source] reqwest::Error),
    #[error("LCU returned HTTP status {0}")]
    Status(StatusCode),
    #[error("LCU WebSocket failed")]
    WebSocket(#[source] Box<tokio_tungstenite::tungstenite::Error>),
    #[error("LCU event payload is invalid: {0}")]
    Event(String),
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

    pub async fn next_ready_check_event(&self) -> Result<ReadyCheck, TransportError> {
        let websocket_url = self.base_url.replacen("https://", "wss://", 1);
        let mut request = websocket_url.into_client_request().map_err(|error| TransportError::WebSocket(Box::new(error)))?;
        let value = format!("riot:{}", self.password);
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, value);
        request.headers_mut().insert("Authorization", encoded.parse().expect("basic auth header"));
        let (mut socket, _) = connect_async(request).await.map_err(|error| TransportError::WebSocket(Box::new(error)))?;
        socket.send(Message::Text(r#"[5,"OnJsonApiEvent"]"#.into())).await.map_err(|error| TransportError::WebSocket(Box::new(error)))?;
        while let Some(message) = socket.next().await {
            let message = message.map_err(|error| TransportError::WebSocket(Box::new(error)))?;
            if let Message::Text(text) = message {
                let event: Value = serde_json::from_str(&text).map_err(|error| TransportError::Event(error.to_string()))?;
                if let Some(payload) = event.get(2) {
                    if payload.get("uri").and_then(Value::as_str) == Some(READY_CHECK) {
                        return parse_ready_check_event(payload).map_err(|error| TransportError::Event(error.to_string()));
                    }
                }
            }
        }
        Err(TransportError::WebSocket(Box::new(tokio_tungstenite::tungstenite::Error::ConnectionClosed)))
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
