//! NotAI kalıcı depo.
//!
//! Kurallar:
//! - Her yazma tek bir transaction içinde yapılır; ya hepsi olur ya hiçbiri.
//! - Her yazma `change_log` tablosuna da düşer (senkron ve denetim için).
//! - Silme = çöp kutusu. Kalıcı silme yalnız çöpte 30 günü dolmuş kayıtlar için.
//! - Sorgular `sqlx::query!` ile derleme zamanında şemaya karşı denetlenir.

#![forbid(unsafe_code)]

mod codec;
mod entries;
mod error;
mod facts;
mod suggestions;
mod sync_apply;
mod tasks;
mod topics;

pub mod changes;

use std::str::FromStr;
use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{SqliteConnection, SqlitePool};

pub use entries::{Captured, JournalItem, NewEntry, TopicRef};
pub use error::{Result, StoreError};
pub use suggestions::DecisionOutcome;
pub use sync_apply::Applied;
pub use tasks::TaskView;
pub use topics::NewTopic;

/// AI işi başarısız olursa yeniden deneme bekleme süreleri (dakika). Bu kadar denemeden
/// sonra kayıt "AI işleyemedi" olarak işaretlenir ama asla kaybolmaz.
pub const AI_RETRY_MINUTES: [i64; 3] = [1, 5, 25];

/// SQLite deposu.
#[derive(Clone, Debug)]
pub struct Store {
    pool: SqlitePool,
    /// Bu cihazın kimliği; değişiklik günlüğüne "kaynak" olarak yazılır.
    origin: String,
}

impl Store {
    /// Dosya veritabanını açar (yoksa oluşturur) ve migration'ları uygular.
    /// `url` örneği: `sqlite://notai.db`.
    pub async fn open(url: &str, origin: impl Into<String>) -> Result<Self> {
        let opts = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(opts)
            .await?;
        Self::init(pool, origin.into()).await
    }

    /// Bellekte geçici veritabanı (testler için). Tek bağlantı: bellek DB'si bağlantıya özeldir.
    pub async fn open_in_memory(origin: impl Into<String>) -> Result<Self> {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await?;
        Self::init(pool, origin.into()).await
    }

    async fn init(pool: SqlitePool, origin: String) -> Result<Self> {
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool, origin })
    }

    #[must_use]
    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// Ayar/durum değeri okur.
    pub async fn kv_get(&self, key: &str) -> Result<Option<String>> {
        let row = sqlx::query!("SELECT value FROM kv WHERE key = ?1", key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.value))
    }

    /// Ayar/durum değeri yazar.
    pub async fn kv_set(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query!(
            "INSERT INTO kv (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            key,
            value
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Bağlantı havuzunu kapatır.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

/// Değişiklik günlüğüne bir satır yazar (çağıranın transaction'ı içinde).
pub(crate) async fn log_change(
    conn: &mut SqliteConnection,
    origin: &str,
    entity: &str,
    entity_id: &str,
    payload: Option<&str>,
    at: DateTime<Utc>,
) -> Result<()> {
    let op = if payload.is_some() {
        "upsert"
    } else {
        "delete"
    };
    let at = codec::ts(at);
    sqlx::query!(
        "INSERT INTO change_log (entity, entity_id, op, payload, origin, at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        entity,
        entity_id,
        op,
        payload,
        origin,
        at
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}
