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

impl Store {
    /// Yalnız bu cihazda üretilmiş değişiklikler (sunucuya gönderilecekler).
    pub async fn own_changes_since(&self, after: i64, limit: u32) -> Result<Vec<ChangeRecord>> {
        self.changes_filtered(after, limit, true).await
    }

    /// Bu cihaz dışındaki kaynaklardan gelen değişiklikler (sunucuda: istemcinin çekeceği
    /// değişiklikler, istemcinin kendi gönderdikleri hariç).
    pub async fn changes_for_peer(
        &self,
        after: i64,
        peer: &str,
        limit: u32,
    ) -> Result<Vec<ChangeRecord>> {
        let limit = i64::from(limit.clamp(1, 1000));
        let rows = sqlx::query!(
            r#"SELECT seq AS "seq!: i64", entity, entity_id, payload, origin, at
               FROM change_log WHERE seq > ?1 AND origin != ?2 ORDER BY seq LIMIT ?3"#,
            after,
            peer,
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

    async fn changes_filtered(
        &self,
        after: i64,
        limit: u32,
        own: bool,
    ) -> Result<Vec<ChangeRecord>> {
        let limit = i64::from(limit.clamp(1, 1000));
        let own_i = i64::from(own);
        let rows = sqlx::query!(
            r#"SELECT seq AS "seq!: i64", entity, entity_id, payload, origin, at
               FROM change_log
               WHERE seq > ?1 AND ((?2 = 1 AND origin = ?3) OR (?2 = 0 AND origin != ?3))
               ORDER BY seq LIMIT ?4"#,
            after,
            own_i,
            self.origin,
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
