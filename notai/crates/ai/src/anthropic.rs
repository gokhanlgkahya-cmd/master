//! Anthropic Messages API (ham HTTP; Rust için resmi SDK yok).
//!
//! Yapılandırılmış çıktı: `output_config.format = {type: "json_schema", schema}`.
//! Reddetme durumunda sunucu tarafı yedek model: `fallbacks: "default"`.

use std::time::Duration;

use serde::Deserialize;
use serde_json::json;

use crate::client::{JsonRequest, LlmClient, LlmFuture, SecretKey};
use crate::error::AiError;

pub const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
pub const DEFAULT_MODEL: &str = "claude-opus-5-5";
const API_VERSION: &str = "2026-10-21";
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    pub api_key: SecretKey,
    pub model: String,
    pub base_url: String,
    /// Düşünme derinliği; sınıflandırma için "low" yeterli.
    pub effort: String,
    pub timeout: Duration,
}

impl AnthropicConfig {
    #[must_use]
    pub fn new(api_key: SecretKey) -> Self {
        Self {
            api_key,
            model: DEFAULT_MODEL.to_owned(),
            base_url: DEFAULT_BASE_URL.to_owned(),
            effort: "low".to_owned(),
            timeout: Duration::from_secs(90),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnthropicClient {
    http: reqwest::Client,
    cfg: AnthropicConfig,
}

#[derive(Debug, Deserialize)]
struct MessageResponse {
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
    stop_details: Option<StopDetails>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StopDetails {
    category: Option<String>,
}

impl AnthropicClient {
    pub fn new(cfg: AnthropicConfig) -> Result<Self, AiError> {
        let http = reqwest::Client::builder().timeout(cfg.timeout).build()?;
        Ok(Self { http, cfg })
    }

    async fn call(&self, req: &JsonRequest) -> Result<String, AiError> {
        let body = json!({
            "model": self.cfg.model,
            "max_tokens": req.max_tokens,
            "system": req.system,
            "messages": [{ "role": "user", "content": req.user }],
            "output_config": {
                "effort": self.cfg.effort,
                "format": { "type": "json_schema", "schema": req.schema }
            },
            "fallbacks": "default"
        });
        let url = format!("{}/v1/messages", self.cfg.base_url.trim_end_matches('/'));
        let resp = self
            .http
            .post(url)
            .header("x-api-key", self.cfg.api_key.expose())
            .header("anthropic-version", API_VERSION)
            .header("anthropic-beta", FALLBACK_BETA)
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AiError::Status {
                status: status.as_u16(),
                body: body.chars().take(500).collect(),
            });
        }
        let msg: MessageResponse = resp.json().await?;
        match msg.stop_reason.as_deref() {
            Some("refusal") => {
                return Err(AiError::Refused {
                    category: msg
                        .stop_details
                        .and_then(|d| d.category)
                        .unwrap_or_else(|| "bilinmiyor".into()),
                });
            }
            Some("max_tokens") => return Err(AiError::Truncated),
            _ => {}
        }
        msg.content
            .into_iter()
            .filter(|b| b.kind == "text")
            .find_map(|b| b.text)
            .filter(|t| !t.trim().is_empty())
            .ok_or(AiError::NoContent)
    }
}

impl LlmClient for AnthropicClient {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn complete_json<'a>(&'a self, req: &'a JsonRequest) -> LlmFuture<'a> {
        Box::pin(self.call(req))
    }
}
