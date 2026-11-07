//! Değişiklik günlüğü okuma (senkron ve denetim için).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Store;
use crate::codec::parse_ts;
use crate::error::Result;

/// Günlükteki bir değişiklik.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeRecord {
    pub seq: i64,
    pub entity: String,
    pub entity_id: String,
    /// `None` ise silme.
    pub payload: Option<String>,
    pub origin: String,
    pub at: DateTime<Utc>,
}

impl Store {
    /// `after` sırasından sonraki değişiklikler (en fazla `limit`).
    pub async fn changes_since(&self, after: i64, limit: u32) -> Result<Vec<ChangeRecord>> {
        let limit = i64::from(limit.clamp(1, 1000));
        let rows = sqlx::query!(
            r#"SELECT seq AS "seq!: i64", entity, entity_id, payload, origin, at
               FROM change_log WHERE seq > ?1 ORDER BY seq LIMIT ?2"#,
            after,
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|r| {
                Ok(ChangeRecord {
                    seq: r.seq,
                    entity: r.entity,
                    entity_id: r.entity_id,
                    payload: r.payload,
                    origin: r.origin,
                    at: parse_ts("değişiklik", &r.at)?,
                })
            })
            .collect()
    }
}
