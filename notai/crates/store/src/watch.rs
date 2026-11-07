//! Fiyat / ürün takibi: zamanı gelen takipler ve ajanın raporunun işlenmesi.

use chrono::{DateTime, Utc};
use notai_core::entry::Source;
use notai_core::text::EntryText;
use notai_core::topic::Topic;
use notai_core::watch::{Offer, Report, WatchState, evaluate, is_due};
use notai_core::{Entry, TopicId};

use crate::Store;
use crate::entries::NewEntry;
use crate::error::Result;

/// Zamanı gelmiş bir takip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueWatch {
    pub topic: Topic,
    pub state: WatchState,
}

impl Store {
    async fn watch_state(&self, topic: TopicId) -> Result<WatchState> {
        let id = topic.to_string();
        let row = sqlx::query_scalar!("SELECT state FROM watch_state WHERE topic_id = ?1", id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default())
    }

    /// Kontrol zamanı gelmiş takipler (etkin ve aralığı dolmuş).
    pub async fn due_watches(&self, now: DateTime<Utc>) -> Result<Vec<DueWatch>> {
        let snap = self.export_snapshot().await?;
        let mut out = Vec::new();
        for t in snap.topics {
            let Some(cfg) = &t.watch else {
                continue;
            };
            let state = self.watch_state(t.id).await?;
            if is_due(cfg, &state, now) {
                out.push(DueWatch { topic: t, state });
            }
        }
        Ok(out)
    }

    /// Ajanın bulduğu teklifleri işler: durum güncellenir; yalnız değişince konuya NotAI
    /// mesajı yazılır (mesaj ve özet döner).
    pub async fn record_watch(
        &self,
        topic: TopicId,
        offers: &[Offer],
        now: DateTime<Utc>,
    ) -> Result<Option<(Entry, Report)>> {
        let t = self.topic(topic).await?;
        let Some(cfg) = &t.watch else {
            return Err(crate::StoreError::Conflict(format!(
                "\"{}\" konusunda fiyat takibi yok",
                t.name
            )));
        };
        let prev = self.watch_state(topic).await?;
        let (state, report) = evaluate(cfg, &prev, offers, now);
        let id = topic.to_string();
        let json = serde_json::to_string(&state)?;
        sqlx::query!(
            "INSERT INTO watch_state (topic_id, state) VALUES (?1, ?2)
             ON CONFLICT (topic_id) DO UPDATE SET state = excluded.state",
            id,
            json
        )
        .execute(&self.pool)
        .await?;
        let Some(report) = report else {
            return Ok(None);
        };
        let entry = self
            .capture_notai(NewEntry {
                text: EntryText::new(&report.message)?,
                source: Source::App,
                topic: Some(topic),
                captured_at: now,
                dedupe_key: None,
            })
            .await?;
        Ok(Some((entry, report)))
    }

    /// `after`'dan sonra NotAI'nin yazdığı kayıtlar (eskiden yeniye; bildirim için).
    pub async fn notai_entries_since(&self, after: DateTime<Utc>) -> Result<Vec<Entry>> {
        let after_s = crate::codec::ts(after);
        let rows = sqlx::query_as!(
            crate::codec::EntryRow,
            "SELECT id, text, source, captured_at, topic_id, status, trashed_at, author
             FROM entries WHERE author = 'notai' AND captured_at > ?1 AND status != 'trashed'
             ORDER BY captured_at",
            after_s
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(Entry::try_from).collect()
    }

    /// Takibin son durumu (arayüzde "son kontrol" için).
    pub async fn watch_status(&self, topic: TopicId) -> Result<WatchState> {
        self.watch_state(topic).await
    }
}
