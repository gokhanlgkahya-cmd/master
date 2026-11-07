//! NotAI AI hattı.
//!
//! Akış: kayıt + bağlam → istem → LLM (yapılandırılmış JSON çıktı) → **katı doğrulama** →
//! `Suggestion` listesi. Doğrulamadan geçmeyen hiçbir şey depoya yazılmaz.
//!
//! Sağlayıcılar `LlmClient` trait'ini uygular: Anthropic Messages API, OpenAI-uyumlu yerel uç
//! (Ollama/Gemma) ve testler için `ScriptedLlm`.

#![forbid(unsafe_code)]

mod anthropic;
pub mod ask;
mod classify;
mod client;
mod error;
pub mod media;
mod openai_compat;

pub use anthropic::{AnthropicClient, AnthropicConfig};
pub use classify::{
    ClassifyContext, ClassifyExample, ClassifyQa, ContextTopic, Validated, build_request, classify,
    output_schema, validate,
};
pub use client::{JsonRequest, LlmClient, LlmFuture, ScriptedLlm, SecretKey};
pub use error::AiError;
pub use openai_compat::{OpenAiCompatClient, OpenAiCompatConfig};
