//! OpenAI-uyumlu sohbet ucu: ev sunucusundaki yerel model (Ollama + Gemma vb.).

use std::time::Duration;

use serde::Deserialize;
use serde_json::json;

use crate::client::{JsonRequest, LlmClient, LlmFuture, SecretKey};
use crate::error::AiError;

#[derive(Debug, Clone)]
pub struct OpenAiCompatConfig {
    /// Örn. `http://127.0.0.1:11434` (Ollama).
    pub base_url: String,
    pub model: String,
    /// Yerel uçlar çoğunlukla anahtar istemez.
    pub api_key: Option<SecretKey>,
    pub timeout: Duration,
}

impl OpenAiCompatConfig {
    #[must_use]
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            api_key: None,
            timeout: Duration::from_secs(180),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatClient {
    http: reqwest::Client,
    cfg: OpenAiCompatConfig,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChoiceMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: Option<String>,
}

impl OpenAiCompatClient {
    pub fn new(cfg: OpenAiCompatConfig) -> Result<Self, AiError> {
        let http = reqwest::Client::builder().timeout(cfg.timeout).build()?;
        Ok(Self { http, cfg })
    }

    async fn call(&self, req: &JsonRequest) -> Result<String, AiError> {
        let body = json!({
            "model": self.cfg.model,
            "max_tokens": req.max_tokens,
            "temperature": 0,
            "messages": [
                { "role": "system", "content": req.system },
                { "role": "user", "content": req.user }
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": { "name": req.schema_name, "strict": true, "schema": req.schema }
            }
        });
        let url = format!(
            "{}/v1/chat/completions",
            self.cfg.base_url.trim_end_matches('/')
        );
        let mut builder = self.http.post(url).json(&body);
        if let Some(key) = &self.cfg.api_key {
            builder = builder.bearer_auth(key.expose());
        }
        let resp = builder.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AiError::Status {
                status: status.as_u16(),
                body: body.chars().take(500).collect(),
            });
        }
        let chat: ChatResponse = resp.json().await?;
        let choice = chat.choices.into_iter().next().ok_or(AiError::NoContent)?;
        if choice.finish_reason.as_deref() == Some("length") {
            return Err(AiError::Truncated);
        }
        choice
            .message
            .content
            .filter(|t| !t.trim().is_empty())
            .ok_or(AiError::NoContent)
    }
}

impl LlmClient for OpenAiCompatClient {
    fn name(&self) -> &str {
        "openai-compat"
    }

    fn complete_json<'a>(&'a self, req: &'a JsonRequest) -> LlmFuture<'a> {
        Box::pin(self.call(req))
    }
}
