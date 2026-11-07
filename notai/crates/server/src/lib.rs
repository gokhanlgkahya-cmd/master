//! NotAI ev sunucusu (S24 Ultra üzerinde 7/24).
//!
//! REST API + WhatsApp/MacroDroid webhook'u + AI işçisi + zamanlayıcı.
//! Erişim yalnız ev ağı / Tailscale üzerinden önerilir; API anahtarları yalnız burada.

#![forbid(unsafe_code)]

pub mod agent;
pub mod config;
pub mod error;
pub mod routes;
pub mod routes_v2;
pub mod scheduler;
pub mod state;
pub mod worker;

use std::sync::Arc;

use notai_ai::{
    AnthropicClient, AnthropicConfig, LlmClient, OpenAiCompatClient, OpenAiCompatConfig, SecretKey,
};

pub use config::{AiProvider, Config};
pub use routes::router;
pub use state::{AppState, Clock, ManualClock, SystemClock};

/// Yapılandırmadaki sağlayıcıdan LLM istemcisi kurar.
pub fn build_llm(
    provider: &AiProvider,
    anthropic_key: Option<String>,
) -> Result<Option<Arc<dyn LlmClient>>, notai_ai::AiError> {
    Ok(match provider {
        AiProvider::None | AiProvider::External => None,
        AiProvider::Anthropic { model } => {
            let key = anthropic_key.ok_or_else(|| {
                notai_ai::AiError::Config("ANTHROPIC_API_KEY tanımlı değil".into())
            })?;
            let mut cfg = AnthropicConfig::new(SecretKey::new(key)?);
            if let Some(m) = model {
                cfg.model.clone_from(m);
            }
            Some(Arc::new(AnthropicClient::new(cfg)?))
        }
        AiProvider::Local { base_url, model } => Some(Arc::new(OpenAiCompatClient::new(
            OpenAiCompatConfig::new(base_url.clone(), model.clone()),
        )?)),
    })
}

type Media = (
    Option<Arc<dyn notai_ai::media::Transcriber>>,
    Option<Arc<dyn notai_ai::media::Embedder>>,
);

/// Yapılandırmadan ses → yazı ve vektör istemcilerini kurar.
pub fn build_media(cfg: &Config) -> Result<Media, notai_ai::AiError> {
    use notai_ai::media::{EmbeddingClient, LocalServiceConfig, WhisperClient};
    let stt = match &cfg.stt {
        Some((url, model)) => Some(Arc::new(WhisperClient::new(LocalServiceConfig::new(
            url, model,
        ))?) as Arc<dyn notai_ai::media::Transcriber>),
        None => None,
    };
    let embed = match &cfg.embed {
        Some((url, model)) => Some(Arc::new(EmbeddingClient::new(LocalServiceConfig::new(
            url, model,
        ))?) as Arc<dyn notai_ai::media::Embedder>),
        None => None,
    };
    Ok((stt, embed))
}
