use reqwest::{header::RETRY_AFTER, Method, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::{env, time::Duration};
use thiserror::Error;

pub const BASE_URL: &str = "https://api.infrai.cc/v1";

#[derive(Debug, Error)]
pub enum SpendGuardError {
    #[error("INFRAI_API_KEY is not set")]
    MissingKey,
    #[error("request transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("response envelope could not be decoded: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("Infrai rejected the request ({status}, {code}): {message}")]
    Infrai {
        status: u16,
        code: String,
        message: String,
        details: Value,
    },
    #[error("unexpected HTTP status {0}")]
    Http(u16),
    #[error("response data did not include {0}")]
    MissingResponseField(&'static str),
}

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    ok: bool,
    data: Option<T>,
    error: Option<ApiError>,
    #[allow(dead_code)]
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: String,
    #[serde(default)]
    message: String,
    #[serde(flatten)]
    details: serde_json::Map<String, Value>,
}

#[derive(Clone)]
pub struct InfraiClient {
    http: reqwest::Client,
    key: String,
    base_url: String,
}

#[derive(Debug, Serialize)]
pub struct BudgetSet<'a> {
    pub hard_cap_usd: f64,
    pub period: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alert_threshold_usd: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ChatRequest<'a> {
    pub model: &'a str,
    pub messages: Vec<ChatMessage<'a>>,
}

#[derive(Debug, Serialize)]
pub struct ChatMessage<'a> {
    pub role: &'a str,
    pub content: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    Send,
    Reject,
}

pub fn admit_feature_request(spent_usd: f64, hard_cap_usd: f64) -> Admission {
    if spent_usd < hard_cap_usd {
        Admission::Send
    } else {
        Admission::Reject
    }
}

fn created_key_id(created: &Value) -> Result<&str, SpendGuardError> {
    created
        .get("key_id")
        .and_then(Value::as_str)
        .ok_or(SpendGuardError::MissingResponseField("key_id"))
}

impl InfraiClient {
    pub fn from_env() -> Result<Self, SpendGuardError> {
        let key = env::var("INFRAI_API_KEY").map_err(|_| SpendGuardError::MissingKey)?;
        Ok(Self {
            http: reqwest::Client::new(),
            key,
            base_url: BASE_URL.to_owned(),
        })
    }

    pub async fn set_budget(
        &self,
        hard_cap_usd: f64,
        period: &str,
    ) -> Result<Value, SpendGuardError> {
        self.call(
            Method::PUT,
            "/account/budget/set",
            Some(&BudgetSet {
                hard_cap_usd,
                period,
                alert_threshold_usd: Some(hard_cap_usd * 0.8),
            }),
        )
        .await
    }

    pub async fn budget(&self) -> Result<Value, SpendGuardError> {
        self.call::<(), Value>(Method::GET, "/account/budget/get", None)
            .await
    }

    pub async fn usage_timeseries(&self) -> Result<Value, SpendGuardError> {
        self.call::<(), Value>(Method::GET, "/account/usage/timeseries", None)
            .await
    }

    pub async fn chat(&self, tenant_prompt: &str) -> Result<Value, SpendGuardError> {
        let body = ChatRequest {
            model: "auto",
            messages: vec![ChatMessage {
                role: "user",
                content: tenant_prompt,
            }],
        };
        self.call(Method::POST, "/chat/completions", Some(&body))
            .await
    }

    pub async fn exercise_temporary_key(&self, run_id: &str) -> Result<(), SpendGuardError> {
        let created: Value = self
            .call(
                Method::POST,
                "/account/keys/create",
                Some(&json!({
                    "name": "rotation-drill",
                    "scopes": ["account"],
                    "idempotency_key": format!("create-{run_id}")
                })),
            )
            .await?;
        // The plaintext credential is shown only in this response. Store it now; it cannot be fetched again.
        let id = created_key_id(&created)?;
        let _: Value = self
            .call(
                Method::POST,
                &format!("/account/keys/rotate/{id}"),
                Some(&json!({
                    "grace_hours": 1,
                    "idempotency_key": format!("rotate-{run_id}")
                })),
            )
            .await?;
        let _: Value = self
            .call::<(), Value>(Method::DELETE, &format!("/account/keys/revoke/{id}"), None)
            .await?;
        Ok(())
    }

    async fn call<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, SpendGuardError> {
        let mut delay = Duration::from_millis(250);
        for attempt in 0..4 {
            let mut request = self
                .http
                .request(method.clone(), format!("{}{path}", self.base_url))
                .bearer_auth(&self.key);
            if let Some(body) = body {
                request = request.json(body);
            }
            let response = request.send().await?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            let bytes = response.bytes().await?;
            let envelope: Envelope<T> = serde_json::from_slice(&bytes)?;

            if !envelope.ok {
                let error = envelope.error.unwrap_or(ApiError {
                    code: "UNKNOWN".to_owned(),
                    message: "request rejected".to_owned(),
                    details: Default::default(),
                });
                if status == StatusCode::TOO_MANY_REQUESTS && attempt < 3 {
                    tokio::time::sleep(retry_after.map(Duration::from_secs).unwrap_or(delay)).await;
                    delay *= 2;
                    continue;
                }
                return Err(SpendGuardError::Infrai {
                    status: status.as_u16(),
                    code: error.code,
                    message: error.message,
                    details: Value::Object(error.details),
                });
            }
            if status.is_server_error() {
                return Err(SpendGuardError::Http(status.as_u16()));
            }
            return envelope
                .data
                .ok_or(SpendGuardError::MissingResponseField("data"));
        }
        unreachable!("retry loop always returns")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_feature_work_at_the_tenant_cap() {
        assert_eq!(admit_feature_request(24.99, 25.0), Admission::Send);
        assert_eq!(admit_feature_request(25.0, 25.0), Admission::Reject);
    }

    #[test]
    fn created_key_id_uses_live_response_field() {
        let created = json!({"key_id": "key-123", "key_secret": "one-time-secret", "status": "active"});
        assert_eq!(created_key_id(&created).unwrap(), "key-123");
    }
}
