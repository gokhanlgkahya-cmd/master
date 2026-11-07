//! HTTP hata eşlemesi: her hata JSON `{ "error": "…" }` ve doğru durum koduyla döner.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use notai_core::CoreError;
use notai_store::StoreError;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("{0} geçersiz: {1}")]
    Invalid(&'static str, String),
    #[error("{0} gerekli")]
    Missing(&'static str),
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("yetkisiz")]
    Unauthorized,
    #[error("{0}")]
    BadRequest(String),
    #[error(transparent)]
    Sync(#[from] notai_sync::SyncError),
}

impl ApiError {
    fn status(&self) -> StatusCode {
        match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Core(e) | Self::Store(StoreError::Core(e)) => core_status(e),
            Self::Store(StoreError::NotFound { .. }) => StatusCode::NOT_FOUND,
            Self::Store(StoreError::Conflict(_)) => StatusCode::CONFLICT,
            Self::Store(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Sync(notai_sync::SyncError::Rejected(_)) => StatusCode::FORBIDDEN,
            Self::Sync(notai_sync::SyncError::Store(StoreError::Corrupt { .. })) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            Self::Sync(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

const fn core_status(e: &CoreError) -> StatusCode {
    match e {
        CoreError::InvalidTransition { .. } | CoreError::AlreadyDecided | CoreError::TaskClosed => {
            StatusCode::CONFLICT
        }
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        let message = if status == StatusCode::INTERNAL_SERVER_ERROR {
            // İç ayrıntıyı istemciye sızdırma; günlüğe yaz.
            tracing::error!(error = %self, "iç hata");
            "sunucu hatası".to_owned()
        } else {
            self.to_string()
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
