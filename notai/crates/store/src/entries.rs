//! Kayıtlar: yakalama, akış, durum geçişleri, AI kuyruğu.

use chrono::{DateTime, Duration, Utc};
use notai_core::entry::{Entry, Source};
use notai_core::journal::JournalFilter;
use notai_core::suggestion::Suggestion;
use notai_core::text::{EntryText, TopicName};
use notai_core::{EntryId, TopicId};
use serde::{Deserialize, Serialize};
use sqlx::SqliteConnection;

use crate::codec::{EntryRow, SuggestionRow, parse_id, ts};
use crate::error::{Result, StoreError};
use crate::topics::get_topic;
use crate::{AI_RETRY_MINUTES, Store, log_change};

/// Yakalama girdileri.
#[derive(Debug, Clone)]
pub struct NewEntry {
    pub text: EntryText,
    pub source: Source,
    pub topic: Option<TopicId>,
    pub captured_at: DateTime<Utc>,
    /// Aynı anahtarla ikinci yakalama yeni kayıt açmaz (WhatsApp yinelemeleri).
    pub dedupe_key: Option<String>,
}

/// Yakalama sonucu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Captured {
    New(Entry),
    /// Aynı yineleme anahtarıyla daha önce yakalanmış kayıt.
    Duplicate(Entry),
}

impl Captured {
    #[must_use]
    pub const fn entry(&self) -> &Entry {
        match self {
            Self::New(e) | Self::Duplicate(e) => e,
        }
    }
}

/// Akışta gösterilecek konu bilgisi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicRef {
    pub id: TopicId,
    pub name: TopicName,
}

/// Akıştaki bir kayıt ve çevresindeki her şey.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalItem {
    pub entry: Entry,
    pub topic: Option<TopicRef>,
    pub suggestions: Vec<Suggestion>,
    pub tags: Vec<String>,
    pub people: Vec<String>,
    /// AI tüm denemelerde başarısız oldu ("AI işleyemedi").
    pub ai_failed: bool,
}

pub(crate) async fn load_entry(conn: &mut SqliteConnection, id: EntryId) -> Result<Entry> {
    let id_s = id.to_string();
    let row = sqlx::query_as!(
        EntryRow,
        "SELECT id, text, source, captured_at, topic_id, status, trashed_at, author
         FROM entries WHERE id = ?1",
        id_s
    )
    .fetch_optional(&mut *conn)
    .await?;
    row.map(Entry::try_from)
        .transpose()?
        .ok_or_else(|| StoreError::not_found("kayıt", id))
}

/// Kaydın konu/durum alanlarını yazar (günlüğe yazmaz).
pub(crate) async fn update_entry_fields(
    conn: &mut SqliteConnection,
    entry: &Entry,
    now: DateTime<Utc>,
) -> Result<()> {
    let id = entry.id.to_string();
    let topic = entry.topic().map(|t| t.to_string());
    let status = entry.status().name();
    let trashed = entry.status().trashed_at().map(ts);
    let now_s = ts(now);
    sqlx::query!(
        "UPDATE entries SET topic_id = ?2, status = ?3, trashed_at = ?4, updated_at = ?5
         WHERE id = ?1",
        id,
        topic,
        status,
        trashed,
        now_s
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Kaydın konu/durum alanlarını yazar ve günlüğe düşer.
pub(crate) async fn update_entry(
    conn: &mut SqliteConnection,
    origin: &str,
    entry: &Entry,
    now: DateTime<Utc>,
) -> Result<()> {
    update_entry_fields(conn, entry, now).await?;
    let payload = serde_json::to_string(entry)?;
    log_change(
        conn,
        origin,
        "entry",
        &entry.id.to_string(),
        Some(&payload),
        now,
    )
    .await
}

impl Store {
    /// Kaydı yakalar. AI'ı beklemez: kayıt hemen yazılır, AI işi kuyruğa düşer.
    pub async fn capture(&self, new: NewEntry) -> Result<Captured> {
        let c = self.capture_as(new, false).await?;
        if let Captured::New(e) = &c {
            // Kayıt yazıldı; hareket çıkarma hatası kaydı geri almaz (yalnız günlüğe).
            match self.detect_movement(e, e.captured_at).await {
                Ok(Some(_)) => return Ok(Captured::New(self.entry(e.id).await?)),
                Ok(None) => {}
                Err(err) => tracing::warn!(error = %err, "hareket çıkarılamadı"),
            }
        }
        Ok(c)
    }

    /// NotAI'nin bir konuya yazdığı mesaj (ör. fiyat takibi). Sınıflandırılmaz (AI işi yok),
    /// kullanıcının kaydından ayrı gösterilir.
    pub async fn capture_notai(&self, new: NewEntry) -> Result<Entry> {
        match self.capture_as(new, true).await? {
            Captured::New(e) | Captured::Duplicate(e) => Ok(e),
        }
    }

    async fn capture_as(&self, new: NewEntry, from_notai: bool) -> Result<Captured> {
        let mut tx = self.pool.begin().await?;

        if let Some(key) = new.dedupe_key.as_deref() {
            let existing = sqlx::query_scalar!("SELECT id FROM entries WHERE dedupe_key = ?1", key)
                .fetch_optional(&mut *tx)
                .await?;
            if let Some(id) = existing {
                let entry = load_entry(&mut tx, parse_id("kayıt", &id)?).await?;
                return Ok(Captured::Duplicate(entry));
            }
        }
        if let Some(topic) = new.topic {
            // Var olmayan konuya kayıt yazılamaz.
            get_topic(&mut tx, topic).await?;
        }

        let mut entry = Entry::capture(new.text, new.source, new.captured_at, new.topic);
        entry.from_notai = from_notai;
        let author = from_notai.then_some("notai");
        let ai_state = if from_notai { "done" } else { "pending" };
        let id = entry.id.to_string();
        let text = entry.text.as_str();
        let source = entry.source.as_str();
        let captured = ts(entry.captured_at);
        let topic = entry.topic().map(|t| t.to_string());
        let status = entry.status().name();
        sqlx::query!(
            "INSERT INTO entries (id, text, source, captured_at, topic_id, status, dedupe_key, updated_at,
                                  author, ai_state)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?4, ?8, ?9)",
            id,
            text,
            source,
            captured,
            topic,
            status,
            new.dedupe_key,
            author,
            ai_state
        )
        .execute(&mut *tx)
        .await?;
        let payload = serde_json::to_string(&entry)?;
        log_change(
            &mut tx,
            &self.origin,
            "entry",
            &id,
            Some(&payload),
            entry.captured_at,
        )
        .await?;
        tx.commit().await?;
        Ok(Captured::New(entry))
    }

    pub async fn entry(&self, id: EntryId) -> Result<Entry> {
        let mut conn = self.pool.acquire().await?;
        load_entry(&mut conn, id).await
    }

    /// Akış: süzgece uyan en yeni `limit` kayıt (isteğe bağlı `before` imlecinden önce),
    /// eskiden yeniye sıralı döner (en yeni en altta).
    pub async fn journal(
        &self,
        filter: JournalFilter,
        before: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<JournalItem>> {
        let (mode, topic) = match filter {
            JournalFilter::All => ("all", None),
            JournalFilter::Unprocessed => ("unprocessed", None),
            JournalFilter::Topic { id } => ("topic", Some(id.to_string())),
        };
        let before = before.map(ts);
        let limit = i64::from(limit.clamp(1, 500));
        // Konu akışı alt konuları da kapsar (hiyerarşi: alt konuya yazılan üstte de görünür).
        let rows = sqlx::query!(
            r#"WITH RECURSIVE sub(id) AS (
                   SELECT ?2 WHERE ?2 IS NOT NULL
                   UNION SELECT t.id FROM topics t JOIN sub ON t.parent_id = sub.id
               )
               SELECT e.id, e.text, e.source, e.captured_at, e.topic_id, e.status, e.trashed_at, e.author,
                      e.ai_state, t.name AS "topic_name?: String"
               FROM entries e LEFT JOIN topics t ON t.id = e.topic_id
               WHERE e.status IN ('inbox', 'processed')
                 AND (?1 = 'all'
                      OR (?1 = 'unprocessed' AND e.status = 'inbox')
                      OR (?1 = 'topic' AND e.topic_id IN (SELECT id FROM sub)))
                 AND (?3 IS NULL OR e.captured_at < ?3)
               ORDER BY e.captured_at DESC, e.id DESC
               LIMIT ?4"#,
            mode,
            topic,
            before,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        let ids_json = serde_json::to_string(&ids)?;
        let suggestion_rows = sqlx::query_as!(
            SuggestionRow,
            "SELECT id, entry_id, payload, confidence, decision, decided_at
             FROM suggestions WHERE entry_id IN (SELECT value FROM json_each(?1))
             ORDER BY created_at, id",
            ids_json
        )
        .fetch_all(&self.pool)
        .await?;
        let label_rows = sqlx::query!(
            "SELECT entry_id, kind, name FROM entry_labels
             WHERE entry_id IN (SELECT value FROM json_each(?1)) ORDER BY name",
            ids_json
        )
        .fetch_all(&self.pool)
        .await?;

        let mut items = Vec::with_capacity(rows.len());
        for r in rows.into_iter().rev() {
            let topic = match (&r.topic_id, &r.topic_name) {
                (Some(id), Some(name)) => Some(TopicRef {
                    id: parse_id("kayıt", id)?,
                    name: TopicName::new(name)?,
                }),
                _ => None,
            };
            let suggestions = suggestion_rows
                .iter()
                .filter(|s| s.entry_id == r.id)
                .map(|s| {
                    Suggestion::try_from(SuggestionRow {
                        id: s.id.clone(),
                        entry_id: s.entry_id.clone(),
                        payload: s.payload.clone(),
                        confidence: s.confidence,
                        decision: s.decision.clone(),
                        decided_at: s.decided_at.clone(),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let labels = |kind: &str| -> Vec<String> {
                label_rows
                    .iter()
                    .filter(|l| l.entry_id == r.id && l.kind == kind)
                    .map(|l| l.name.clone())
                    .collect()
            };
            let tags = labels("tag");
            let people = labels("person");
            let ai_failed = r.ai_state == "failed";
            let entry = Entry::try_from(EntryRow {
                id: r.id,
                text: r.text,
                source: r.source,
                captured_at: r.captured_at,
                topic_id: r.topic_id,
                status: r.status,
                trashed_at: r.trashed_at,
                author: r.author,
            })?;
            items.push(JournalItem {
                entry,
                topic,
                suggestions,
                tags,
                people,
                ai_failed,
            });
        }
        Ok(items)
    }

    /// Ana Günlük rozeti: işlenmemiş kayıt sayısı.
    pub async fn unprocessed_count(&self) -> Result<u64> {
        let n = sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "n!: i64" FROM entries WHERE status = 'inbox'"#
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// Bir kaydı uygular ve kaydeder (ortak iskelet).
    async fn transition<F>(&self, id: EntryId, now: DateTime<Utc>, f: F) -> Result<Entry>
    where
        F: FnOnce(&mut Entry) -> std::result::Result<(), notai_core::CoreError>,
    {
        let mut tx = self.pool.begin().await?;
        let mut entry = load_entry(&mut tx, id).await?;
        f(&mut entry)?;
        update_entry(&mut tx, &self.origin, &entry, now).await?;
        tx.commit().await?;
        Ok(entry)
    }

    /// Kaydı bir konuya taşır ("konuya taşı") ya da konusundan çıkarır.
    pub async fn move_entry(
        &self,
        id: EntryId,
        topic: Option<TopicId>,
        now: DateTime<Utc>,
    ) -> Result<Entry> {
        if let Some(t) = topic {
            self.topic(t).await?;
        }
        self.transition(id, now, |e| e.assign_topic(topic)).await
    }

    pub async fn archive_entry(&self, id: EntryId, now: DateTime<Utc>) -> Result<Entry> {
        self.transition(id, now, Entry::archive).await
    }

    pub async fn unarchive_entry(&self, id: EntryId, now: DateTime<Utc>) -> Result<Entry> {
        self.transition(id, now, Entry::unarchive).await
    }

    /// "Sil": çöp kutusuna atar.
    pub async fn trash_entry(&self, id: EntryId, now: DateTime<Utc>) -> Result<Entry> {
        self.transition(id, now, |e| e.trash(now)).await
    }

    /// Çöp kutusundan geri alır.
    pub async fn restore_entry(&self, id: EntryId, now: DateTime<Utc>) -> Result<Entry> {
        self.transition(id, now, Entry::restore).await
    }

    /// Çöpte 30 günü dolmuş kayıtları kalıcı siler. Silinen sayısını döner.
    pub async fn purge_trash(&self, now: DateTime<Utc>) -> Result<u64> {
        let mut tx = self.pool.begin().await?;
        let rows = sqlx::query_as!(
            EntryRow,
            "SELECT id, text, source, captured_at, topic_id, status, trashed_at, author
             FROM entries WHERE status = 'trashed'"
        )
        .fetch_all(&mut *tx)
        .await?;
        let mut purged = 0;
        for row in rows {
            let entry = Entry::try_from(row)?;
            if !entry.can_purge(now) {
                continue;
            }
            let id = entry.id.to_string();
            sqlx::query!("DELETE FROM entries WHERE id = ?1", id)
                .execute(&mut *tx)
                .await?;
            log_change(&mut tx, &self.origin, "entry", &id, None, now).await?;
            purged += 1;
        }
        tx.commit().await?;
        Ok(purged)
    }

    /// Tam metin arama (Türkçe aksan duyarsız). En yeni üstte.
    pub async fn search(&self, query: &str, limit: u32) -> Result<Vec<Entry>> {
        // FTS sözdizimi hatalarını önlemek için her kelimeyi tırnak içine al; önek eşleşmesi (*)
        // "tedarik" → "tedarikçisini" bulunur. Aksanlar (ç, ş, ğ, ö, ü) dizinde katlanır.
        let fts: String = query
            .split_whitespace()
            .map(|w| w.replace('"', ""))
            .filter(|w| !w.is_empty())
            .map(|w| format!("\"{w}\"*"))
            .collect::<Vec<_>>()
            .join(" ");
        if fts.is_empty() {
            return Ok(Vec::new());
        }
        let limit = i64::from(limit.clamp(1, 200));
        let rows = sqlx::query_as!(
            EntryRow,
            "SELECT e.id, e.text, e.source, e.captured_at, e.topic_id, e.status, e.trashed_at, e.author
             FROM entries_fts f JOIN entries e ON e.rowid = f.rowid
             WHERE entries_fts MATCH ?1 AND e.status IN ('inbox', 'processed', 'archived')
             ORDER BY e.captured_at DESC LIMIT ?2",
            fts,
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(Entry::try_from).collect()
    }

    /// Bağlam için son kayıtlar (AI istemine girer). En yeni üstte.
    pub async fn recent_entries(&self, limit: u32) -> Result<Vec<Entry>> {
        let limit = i64::from(limit.clamp(1, 100));
        let rows = sqlx::query_as!(
            EntryRow,
            "SELECT id, text, source, captured_at, topic_id, status, trashed_at, author
             FROM entries WHERE status IN ('inbox', 'processed')
             ORDER BY captured_at DESC LIMIT ?1",
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(Entry::try_from).collect()
    }

    /// Sıradaki AI işi: bekleyen ve bekleme süresi dolmuş en eski kayıt.
    pub async fn next_ai_job(&self, now: DateTime<Utc>) -> Result<Option<Entry>> {
        let now_s = ts(now);
        let row = sqlx::query_as!(
            EntryRow,
            "SELECT id, text, source, captured_at, topic_id, status, trashed_at, author
             FROM entries
             WHERE ai_state = 'pending' AND status != 'trashed'
               AND (ai_next_at IS NULL OR ai_next_at <= ?1)
             ORDER BY captured_at LIMIT 1",
            now_s
        )
        .fetch_optional(&self.pool)
        .await?;
        row.map(Entry::try_from).transpose()
    }

    /// Sıradaki AI işi, verilen kayıtlar hariç (dış ajana birden çok iş vermek için).
    pub async fn next_ai_job_excluding(
        &self,
        now: DateTime<Utc>,
        exclude: &[EntryId],
    ) -> Result<Option<Entry>> {
        let now_s = ts(now);
        let ids: Vec<String> = exclude.iter().map(ToString::to_string).collect();
        let ids_json = serde_json::to_string(&ids)?;
        let row = sqlx::query_as!(
            EntryRow,
            "SELECT id, text, source, captured_at, topic_id, status, trashed_at, author
             FROM entries
             WHERE ai_state = 'pending' AND status != 'trashed'
               AND (ai_next_at IS NULL OR ai_next_at <= ?1)
               AND id NOT IN (SELECT value FROM json_each(?2))
             ORDER BY captured_at LIMIT 1",
            now_s,
            ids_json
        )
        .fetch_optional(&self.pool)
        .await?;
        row.map(Entry::try_from).transpose()
    }

    /// AI'ın doğrulanmış önerilerini kaydeder ve işi bitirir. Öneri yoksa (AI'ın söyleyecek
    /// bir şeyi yoksa) gelen kutusundaki kayıt işlenmiş sayılır.
    pub async fn save_ai_suggestions(
        &self,
        entry_id: EntryId,
        suggestions: &[Suggestion],
        now: DateTime<Utc>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let mut entry = load_entry(&mut tx, entry_id).await?;
        let now_s = ts(now);
        for s in suggestions {
            if s.entry != entry_id {
                return Err(StoreError::Conflict("öneri başka bir kayda ait".into()));
            }
            crate::suggestions::insert_suggestion(&mut tx, &self.origin, s, now).await?;
        }
        let id = entry_id.to_string();
        sqlx::query!(
            "UPDATE entries SET ai_state = 'done', ai_error = NULL, ai_next_at = NULL, updated_at = ?2
             WHERE id = ?1",
            id,
            now_s
        )
        .execute(&mut *tx)
        .await?;
        if suggestions.is_empty() && entry.is_unprocessed() {
            entry.mark_processed()?;
            update_entry(&mut tx, &self.origin, &entry, now).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// AI denemesinin başarısız olduğunu kaydeder. Yeniden denenebilir hatada bekleme süresiyle
    /// tekrar kuyruğa alınır; denemeler tükenince ya da hata kalıcıysa (`retryable = false`)
    /// kayıt "AI işleyemedi" olur (kayıt kaybolmaz). Dönüş: iş tamamen başarısız oldu mu?
    pub async fn record_ai_failure(
        &self,
        entry_id: EntryId,
        error: &str,
        retryable: bool,
        now: DateTime<Utc>,
    ) -> Result<bool> {
        let id = entry_id.to_string();
        let mut tx = self.pool.begin().await?;
        let attempts = sqlx::query_scalar!("SELECT ai_attempts FROM entries WHERE id = ?1", id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| StoreError::not_found("kayıt", entry_id))?;
        let attempts = attempts + 1;
        let idx = usize::try_from(attempts - 1).unwrap_or(usize::MAX);
        let (state, next_at) = match AI_RETRY_MINUTES.get(idx) {
            Some(mins) if retryable => ("pending", Some(ts(now + Duration::minutes(*mins)))),
            _ => ("failed", None),
        };
        let now_s = ts(now);
        sqlx::query!(
            "UPDATE entries SET ai_state = ?2, ai_attempts = ?3, ai_next_at = ?4, ai_error = ?5,
                    updated_at = ?6
             WHERE id = ?1",
            id,
            state,
            attempts,
            next_at,
            error,
            now_s
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(state == "failed")
    }

    /// Başarısız AI işini yeniden kuyruğa alır (kullanıcı "tekrar dene" dedi).
    pub async fn retry_ai(&self, entry_id: EntryId, now: DateTime<Utc>) -> Result<()> {
        let id = entry_id.to_string();
        let now_s = ts(now);
        let res = sqlx::query!(
            "UPDATE entries SET ai_state = 'pending', ai_attempts = 0, ai_next_at = NULL,
                    ai_error = NULL, updated_at = ?2
             WHERE id = ?1",
            id,
            now_s
        )
        .execute(&self.pool)
        .await?;
        if res.rows_affected() == 0 {
            return Err(StoreError::not_found("kayıt", entry_id));
        }
        Ok(())
    }
}
