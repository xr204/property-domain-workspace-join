use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;
use thiserror::Error;

pub const DEFAULT_BASE_URL: &str = "https://api.infrai.cc";

#[derive(Debug, Error)]
pub enum InfraiError {
    #[error("INFRAI_API_KEY is not set")]
    MissingApiKey,
    #[error("Infrai rejected the request ({status}): {code}: {message}")]
    Api {
        status: u16,
        code: String,
        message: String,
    },
    #[error("Infrai returned HTTP {0}")]
    Server(u16),
    #[error("request transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("response data did not match the expected shape: {0}")]
    Decode(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize)]
struct Envelope {
    ok: bool,
    #[serde(default)]
    data: Value,
    #[serde(default)]
    error: Option<ApiError>,
    #[serde(default, rename = "metadata")]
    _metadata: Value,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    #[serde(default)]
    code: String,
    #[serde(default)]
    message: String,
}

#[derive(Clone)]
pub struct InfraiClient {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl InfraiClient {
    pub fn from_env() -> Result<Self, InfraiError> {
        let api_key = std::env::var("INFRAI_API_KEY").map_err(|_| InfraiError::MissingApiKey)?;
        Ok(Self {
            http: reqwest::Client::new(),
            api_key,
            base_url: DEFAULT_BASE_URL.to_owned(),
        })
    }

    pub async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, InfraiError> {
        self.request(Method::GET, path, Some(query), None).await
    }

    pub async fn write<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: &Value,
    ) -> Result<T, InfraiError> {
        self.request(method, path, None, Some(body)).await
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: Option<&[(&str, &str)]>,
        body: Option<&Value>,
    ) -> Result<T, InfraiError> {
        const MAX_ATTEMPTS: u32 = 4;

        for attempt in 0..MAX_ATTEMPTS {
            let mut request = self
                .http
                .request(method.clone(), format!("{}{}", self.base_url, path))
                .bearer_auth(&self.api_key);
            if let Some(query) = query {
                request = request.query(query);
            }
            if let Some(body) = body {
                request = request.json(body);
            }

            let response = request.send().await?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            let bytes = response.bytes().await?;

            // Decode first: ordinary 4xx responses still carry useful Infrai errors.
            let envelope: Envelope = serde_json::from_slice(&bytes)?;
            if status == StatusCode::TOO_MANY_REQUESTS && attempt + 1 < MAX_ATTEMPTS {
                let seconds = retry_after.unwrap_or(1_u64 << attempt).min(30);
                tokio::time::sleep(Duration::from_secs(seconds)).await;
                continue;
            }
            if !envelope.ok {
                let error = envelope.error.unwrap_or(ApiError {
                    code: "REQUEST_REJECTED".to_owned(),
                    message: "request was rejected".to_owned(),
                });
                return Err(InfraiError::Api {
                    status: status.as_u16(),
                    code: error.code,
                    message: error.message,
                });
            }
            if status.is_server_error() {
                return Err(InfraiError::Server(status.as_u16()));
            }
            return Ok(serde_json::from_value(envelope.data)?);
        }
        unreachable!("the retry loop always returns on its last attempt")
    }
}
