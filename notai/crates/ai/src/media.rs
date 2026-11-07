//! Ses → metin (Whisper uyumlu uç) ve metin → vektör (embedding) istemcileri.
//! İkisi de ev sunucusundaki yerel servislerle çalışır (ör. faster-whisper-server, Ollama).

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use serde::Deserialize;
use serde_json::json;

use crate::client::SecretKey;
use crate::error::AiError;

pub type AiFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, AiError>> + Send + 'a>>;

/// Ses kaydını yazıya çevirir.
pub trait Transcriber: Send + Sync {
    fn transcribe<'a>(&'a self, audio: &'a [u8], mime: &'a str) -> AiFuture<'a, String>;
}

/// Metinleri vektöre çevirir (anlamsal arama).
pub trait Embedder: Send + Sync {
    /// Model adı (vektörler modele göre saklanır).
    fn model(&self) -> &str;
    fn embed<'a>(&'a self, texts: &'a [String]) -> AiFuture<'a, Vec<Vec<f32>>>;
}

/// OpenAI uyumlu yerel servis yapılandırması.
#[derive(Debug, Clone)]
pub struct LocalServiceConfig {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<SecretKey>,
    pub timeout: Duration,
}

impl LocalServiceConfig {
    #[must_use]
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            api_key: None,
            timeout: Duration::from_secs(120),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url.trim_end_matches('/'))
    }
}

async fn check(resp: reqwest::Response) -> Result<reqwest::Response, AiError> {
    let status = resp.status();
    if status.is_success() {
        Ok(resp)
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(AiError::Status {
            status: status.as_u16(),
            body: body.chars().take(500).collect(),
        })
    }
}

/// `POST /v1/audio/transcriptions` (Whisper uyumlu).
#[derive(Debug, Clone)]
pub struct WhisperClient {
    http: reqwest::Client,
    cfg: LocalServiceConfig,
}

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

impl WhisperClient {
    pub fn new(cfg: LocalServiceConfig) -> Result<Self, AiError> {
        Ok(Self {
            http: reqwest::Client::builder().timeout(cfg.timeout).build()?,
            cfg,
        })
    }

    async fn call(&self, audio: &[u8], mime: &str) -> Result<String, AiError> {
        if audio.is_empty() {
            return Err(AiError::Schema("boş ses kaydı".into()));
        }
        let ext = match mime {
            m if m.contains("ogg") => "ogg",
            m if m.contains("mp4") || m.contains("m4a") => "m4a",
            m if m.contains("wav") => "wav",
            _ => "webm",
        };
        let part = reqwest::multipart::Part::bytes(audio.to_vec())
            .file_name(format!("kayit.{ext}"))
            .mime_str(mime)
            .map_err(|e| AiError::Config(e.to_string()))?;
        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", self.cfg.model.clone())
            .text("language", "tr")
            .text("response_format", "json");
        let mut b = self
            .http
            .post(self.cfg.url("/v1/audio/transcriptions"))
            .multipart(form);
        if let Some(k) = &self.cfg.api_key {
            b = b.bearer_auth(k.expose());
        }
        let r: TranscriptionResponse = check(b.send().await?).await?.json().await?;
        let text = r.text.trim().to_owned();
        if text.is_empty() {
            return Err(AiError::NoContent);
        }
        Ok(text)
    }
}

impl Transcriber for WhisperClient {
    fn transcribe<'a>(&'a self, audio: &'a [u8], mime: &'a str) -> AiFuture<'a, String> {
        Box::pin(self.call(audio, mime))
    }
}

/// `POST /v1/embeddings` (OpenAI uyumlu; Ollama destekler).
#[derive(Debug, Clone)]
pub struct EmbeddingClient {
    http: reqwest::Client,
    cfg: LocalServiceConfig,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingItem>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingItem {
    index: usize,
    embedding: Vec<f32>,
}

impl EmbeddingClient {
    pub fn new(cfg: LocalServiceConfig) -> Result<Self, AiError> {
        Ok(Self {
            http: reqwest::Client::builder().timeout(cfg.timeout).build()?,
            cfg,
        })
    }

    async fn call(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, AiError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let mut b = self
            .http
            .post(self.cfg.url("/v1/embeddings"))
            .json(&json!({ "model": self.cfg.model, "input": texts }));
        if let Some(k) = &self.cfg.api_key {
            b = b.bearer_auth(k.expose());
        }
        let mut r: EmbeddingResponse = check(b.send().await?).await?.json().await?;
        if r.data.len() != texts.len() {
            return Err(AiError::Schema(format!(
                "{} metin için {} vektör geldi",
                texts.len(),
                r.data.len()
            )));
        }
        r.data.sort_by_key(|d| d.index);
        let dims = r.data.first().map_or(0, |d| d.embedding.len());
        if dims == 0
            || r.data
                .iter()
                .any(|d| d.embedding.len() != dims || d.embedding.iter().any(|x| !x.is_finite()))
        {
            return Err(AiError::Schema(
                "vektör boyutları tutarsız ya da geçersiz".into(),
            ));
        }
        Ok(r.data.into_iter().map(|d| d.embedding).collect())
    }
}

impl Embedder for EmbeddingClient {
    fn model(&self) -> &str {
        &self.cfg.model
    }

    fn embed<'a>(&'a self, texts: &'a [String]) -> AiFuture<'a, Vec<Vec<f32>>> {
        Box::pin(self.call(texts))
    }
}

/// Kosinüs benzerliği (boyut farklıysa ya da sıfır vektörse 0).
#[must_use]
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0.0f32, 0.0f32, 0.0f32);
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}

/// Sorgu vektörüne en benzer `k` kaydı (benzerliği `min` üstündekiler), azalan sırada.
#[must_use]
pub fn top_k<T: Clone>(
    query: &[f32],
    items: &[(T, Vec<f32>)],
    k: usize,
    min: f32,
) -> Vec<(T, f32)> {
    let mut scored: Vec<(T, f32)> = items
        .iter()
        .map(|(id, v)| (id.clone(), cosine(query, v)))
        .filter(|(_, s)| *s >= min)
        .collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    scored.truncate(k);
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_and_ranking() {
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
        assert_eq!(cosine(&[1.0], &[1.0, 2.0]), 0.0);
        assert_eq!(cosine(&[0.0, 0.0], &[1.0, 2.0]), 0.0);
        let items = vec![
            ("a", vec![1.0, 0.0]),
            ("b", vec![0.7, 0.7]),
            ("c", vec![0.0, 1.0]),
        ];
        let r = top_k(&[1.0, 0.1], &items, 2, 0.1);
        let ids: Vec<_> = r.iter().map(|x| x.0).collect();
        assert_eq!(ids, ["a", "b"]);
    }
}
