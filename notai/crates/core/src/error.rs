//! Alan kurallarının ihlallerini anlatan tipli hatalar.

use thiserror::Error;

/// Alan modeli hataları. Her biri bir iş kuralının ihlalidir; I/O hatası burada yoktur.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    #[error("{field} boş olamaz")]
    Empty { field: &'static str },

    #[error("{field} en fazla {max} karakter olabilir (gelen: {actual})")]
    TooLong {
        field: &'static str,
        max: usize,
        actual: usize,
    },

    #[error("geçersiz durum geçişi: {from} → {to}")]
    InvalidTransition {
        from: &'static str,
        to: &'static str,
    },

    #[error("güven skoru 0 ile 1 arasında olmalı (gelen: {0})")]
    ConfidenceOutOfRange(String),

    #[error("öneri zaten karara bağlanmış")]
    AlreadyDecided,

    #[error("bitiş tarihi yalnız Proje türündeki konulara verilebilir")]
    DueDateOnlyForProjects,

    #[error("ayın günü 1 ile 28 arasında olmalı (gelen: {0})")]
    InvalidMonthDay(u8),

    #[error("hatırlatma için görevin bir tarihi olmalı")]
    ReminderNeedsDue,

    #[error("geçersiz kimlik: {0}")]
    InvalidId(String),

    #[error("bilinmeyen değer `{value}` ({field})")]
    UnknownVariant { field: &'static str, value: String },

    #[error("görev zaten kapalı")]
    TaskClosed,
}
