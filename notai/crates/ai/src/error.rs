use thiserror::Error;

/// AI hattı hataları.
#[derive(Debug, Error)]
pub enum AiError {
    #[error("ağ hatası: {0}")]
    Http(#[from] reqwest::Error),

    #[error("sağlayıcı {status} döndü: {body}")]
    Status { status: u16, body: String },

    #[error("model isteği reddetti ({category})")]
    Refused { category: String },

    #[error("yanıt yarıda kesildi (max_tokens)")]
    Truncated,

    #[error("yanıtta metin yok")]
    NoContent,

    #[error("geçersiz JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("şemaya uymayan çıktı: {0}")]
    Schema(String),

    #[error("yapılandırma hatası: {0}")]
    Config(String),
}

impl AiError {
    /// Yeniden denemek anlamlı mı? (Ağ, 429, 5xx, bozuk çıktı: evet; 4xx, red: hayır.)
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Http(_)
            | Self::Truncated
            | Self::NoContent
            | Self::InvalidJson(_)
            | Self::Schema(_) => true,
            Self::Status { status, .. } => *status == 429 || *status >= 500,
            Self::Refused { .. } | Self::Config(_) => false,
        }
    }
}
