use notai_core::CoreError;
use thiserror::Error;

/// Depo hataları.
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("veritabanı hatası: {0}")]
    Db(#[from] sqlx::Error),

    #[error("migration hatası: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[error(transparent)]
    Core(#[from] CoreError),

    #[error("JSON hatası: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{what} bulunamadı: {id}")]
    NotFound { what: &'static str, id: String },

    #[error("çakışma: {0}")]
    Conflict(String),

    #[error("bozuk kayıt ({what}): {detail}")]
    Corrupt { what: &'static str, detail: String },
}

impl StoreError {
    pub(crate) fn not_found(what: &'static str, id: impl ToString) -> Self {
        Self::NotFound {
            what,
            id: id.to_string(),
        }
    }

    pub(crate) fn corrupt(what: &'static str, detail: impl ToString) -> Self {
        Self::Corrupt {
            what,
            detail: detail.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, StoreError>;
