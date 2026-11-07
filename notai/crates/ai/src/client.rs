//! Sağlayıcıdan bağımsız LLM arayüzü.

use std::collections::VecDeque;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

use serde_json::Value;

use crate::error::AiError;

/// Yapılandırılmış JSON çıktı isteği.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonRequest {
    pub system: String,
    pub user: String,
    /// Çıktının uyması gereken JSON şeması.
    pub schema: Value,
    pub schema_name: &'static str,
    pub max_tokens: u32,
}

pub type LlmFuture<'a> = Pin<Box<dyn Future<Output = Result<String, AiError>> + Send + 'a>>;

/// Bir LLM sağlayıcısı. Dönen metin şemaya uyması *beklenen* ham JSON'dur; doğrulama
/// her zaman çağıranın işidir (`classify::validate`).
pub trait LlmClient: Send + Sync {
    /// Sağlayıcı adı (günlükler için).
    fn name(&self) -> &str;

    fn complete_json<'a>(&'a self, req: &'a JsonRequest) -> LlmFuture<'a>;
}

/// API anahtarı: `Debug` çıktısında asla görünmez.
#[derive(Clone)]
pub struct SecretKey(String);

impl SecretKey {
    pub fn new(key: impl Into<String>) -> Result<Self, AiError> {
        let key = key.into();
        if key.trim().is_empty() {
            return Err(AiError::Config("API anahtarı boş".into()));
        }
        Ok(Self(key))
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretKey(***)")
    }
}

/// Testler için: sırayla önceden verilmiş yanıtları döndürür ve istekleri kaydeder.
#[derive(Debug, Default)]
pub struct ScriptedLlm {
    replies: Mutex<VecDeque<Result<String, String>>>,
    seen: Mutex<Vec<JsonRequest>>,
}

impl ScriptedLlm {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Başarılı yanıt ekler.
    #[must_use]
    pub fn reply(self, json: impl Into<String>) -> Self {
        if let Ok(mut q) = self.replies.lock() {
            q.push_back(Ok(json.into()));
        }
        self
    }

    /// Yeniden denenebilir hata ekler (ör. 503).
    #[must_use]
    pub fn fail(self, message: impl Into<String>) -> Self {
        if let Ok(mut q) = self.replies.lock() {
            q.push_back(Err(message.into()));
        }
        self
    }

    /// Şimdiye kadar gelen istekler.
    #[must_use]
    pub fn requests(&self) -> Vec<JsonRequest> {
        self.seen.lock().map(|v| v.clone()).unwrap_or_default()
    }
}

impl LlmClient for ScriptedLlm {
    fn name(&self) -> &str {
        "scripted"
    }

    fn complete_json<'a>(&'a self, req: &'a JsonRequest) -> LlmFuture<'a> {
        Box::pin(async move {
            if let Ok(mut seen) = self.seen.lock() {
                seen.push(req.clone());
            }
            let next = self.replies.lock().ok().and_then(|mut q| q.pop_front());
            match next {
                Some(Ok(json)) => Ok(json),
                Some(Err(body)) => Err(AiError::Status { status: 503, body }),
                None => Err(AiError::Status {
                    status: 503,
                    body: "senaryoda yanıt kalmadı".into(),
                }),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_is_redacted_and_non_empty() {
        let k = SecretKey::new("sk-ant-gizli").unwrap();
        assert_eq!(format!("{k:?}"), "SecretKey(***)");
        assert!(SecretKey::new("  ").is_err());
    }
}
