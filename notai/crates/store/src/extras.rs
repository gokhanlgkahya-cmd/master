//! v0.2–v0.4 depo işlevleri: hatırlatmalar, takvim, haftalık gözden geçirme verisi, ekler,
//! anlamsal arama vektörleri, öğrenme örnekleri, çöp kutusu.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;
use notai_core::agenda::TopicActivityTimes;
use notai_core::reminders::{Snooze, snooze};
use notai_core::suggestion::SuggestionKind;
use notai_core::task::Task;
use notai_core::topic::Topic;
use notai_core::{Entry, EntryId, TaskId};
use serde::{Deserialize, Serialize};

use crate::codec::{EntryRow, TaskRow, TopicRow, date_str, from_json, parse_opt_ts, parse_ts, ts};
use crate::error::{Result, StoreError};
use crate::{Store, log_change};

/// En büyük ek boyutu (10 MB).
pub const MAX_ATTACHMENT_BYTES: usize = 10 * 1024 * 1024;

/// Ek türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentKind {
    Photo,
    Document,
    Location,
    Contact,
    Audio,
}

impl AttachmentKind {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Photo => "photo",
            Self::Document => "document",
            Self::Location => "location",
            Self::Contact => "contact",
            Self::Audio => "audio",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "photo" => Self::Photo,
            "document" => Self::Document,
            "location" => Self::Location,
            "contact" => Self::Contact,
            "audio" => Self::Audio,
            other => return Err(StoreError::corrupt("ek", other)),
        })
    }
}

/// Yeni ek.
#[derive(Debug, Clone)]
pub struct NewAttachment {
    pub entry: EntryId,
    pub kind: AttachmentKind,
    pub name: String,
    pub mime: String,
    pub data: Option<Vec<u8>>,
    pub value: Option<String>,
}

/// Ek özeti (içerik olmadan).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentMeta {
    pub id: String,
    pub entry: String,
    pub kind: AttachmentKind,
    pub name: String,
    pub mime: String,
    pub size: u64,
    pub value: Option<String>,
}

/// Senkron yükü (içerik base64).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AttachmentPayload {
    pub id: String,
    pub entry: String,
    pub kind: AttachmentKind,
    pub name: String,
    pub mime: String,
    pub data_b64: Option<String>,
    pub value: Option<String>,
}

/// Öğrenme örneği: geçmiş bir kayıt ve kullanıcının bir öneriye kararı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionExample {
    pub entry_text: String,
    pub kind: SuggestionKind,
    pub accepted: bool,
}

impl Store {
    // ---------- Hatırlatmalar ----------

    /// Hatırlatması olan açık görevler ve son bildirim anları.
    pub async fn reminder_tasks(&self) -> Result<Vec<(Task, Option<DateTime<Utc>>)>> {
        let rows = sqlx::query!(
            "SELECT id, entry_id, title, topic_id, due_date, due_time, priority, recurrence, reminder,
                    state, closed_at, reminder_fired_at
             FROM tasks WHERE state = 'open' AND reminder IS NOT NULL"
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|r| {
                let fired = parse_opt_ts("görev", r.reminder_fired_at.as_deref())?;
                let task = Task::try_from(TaskRow {
                    id: r.id,
                    entry_id: r.entry_id,
                    title: r.title,
                    topic_id: r.topic_id,
                    due_date: r.due_date,
                    due_time: r.due_time,
                    priority: r.priority,
                    recurrence: r.recurrence,
                    reminder: r.reminder,
                    state: r.state,
                    closed_at: r.closed_at,
                })?;
                Ok((task, fired))
            })
            .collect()
    }

    /// Hatırlatmanın `at` anı için bildirildiğini işaretler (yerel durum; senkronlanmaz).
    pub async fn mark_reminder_fired(&self, task: TaskId, at: DateTime<Utc>) -> Result<()> {
        let id = task.to_string();
        let at_s = ts(at);
        sqlx::query!(
            "UPDATE tasks SET reminder_fired_at = ?2 WHERE id = ?1",
            id,
            at_s
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Hatırlatmayı erteler.
    pub async fn snooze_reminder(
        &self,
        id: TaskId,
        s: Snooze,
        now: DateTime<Utc>,
        tz: Tz,
    ) -> Result<Task> {
        let mut task = self.task(id).await?;
        snooze(&mut task, s, now, tz)?;
        self.save_task(&task, now).await?;
        Ok(task)
    }

    // ---------- Takvim ve gözden geçirme ----------

    /// Tarihi `[from, to]` aralığındaki (açık ya da kapalı) görevler.
    pub async fn tasks_between(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<Task>> {
        let (f, t) = (date_str(from), date_str(to));
        let rows = sqlx::query_as!(
            TaskRow,
            "SELECT id, entry_id, title, topic_id, due_date, due_time, priority, recurrence, reminder,
                    state, closed_at
             FROM tasks WHERE due_date >= ?1 AND due_date <= ?2 AND state != 'cancelled'
             ORDER BY due_date, due_time IS NULL, due_time",
            f,
            t
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(Task::try_from).collect()
    }

    /// Her konunun son hareketi (son kayıt ya da son görev güncellemesi).
    pub async fn topic_activity(&self) -> Result<Vec<TopicActivityTimes>> {
        let rows = sqlx::query!(
            r#"SELECT t.id, t.name, t.kind, t.due_date, t.icon, t.avatar, t.pinned,
                      (SELECT MAX(x) FROM (
                          SELECT MAX(e.captured_at) AS x FROM entries e WHERE e.topic_id = t.id
                          UNION ALL
                          SELECT MAX(k.updated_at) AS x FROM tasks k WHERE k.topic_id = t.id
                      )) AS "last: String"
               FROM topics t"#
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|r| {
                Ok(TopicActivityTimes {
                    topic: Topic::try_from(TopicRow {
                        id: r.id,
                        name: r.name,
                        kind: r.kind,
                        due_date: r.due_date,
                        icon: r.icon,
                        avatar: r.avatar,
                        pinned: r.pinned,
                    })?,
                    last_activity: r.last.as_deref().map(|s| parse_ts("konu", s)).transpose()?,
                })
            })
            .collect()
    }

    /// `since` anından sonra tamamlanan görev sayısı.
    pub async fn done_since(&self, since: DateTime<Utc>) -> Result<u64> {
        let s = ts(since);
        let n = sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "n!: i64" FROM tasks WHERE state = 'done' AND closed_at >= ?1"#,
            s
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    // ---------- Ekler ----------

    /// Kayda ek iliştirir.
    pub async fn add_attachment(
        &self,
        a: NewAttachment,
        now: DateTime<Utc>,
    ) -> Result<AttachmentMeta> {
        if a.name.trim().is_empty() {
            return Err(StoreError::Conflict("ek adı boş olamaz".into()));
        }
        if a.data.is_none() && a.value.is_none() {
            return Err(StoreError::Conflict("ek içeriği yok".into()));
        }
        if a.data
            .as_ref()
            .is_some_and(|d| d.len() > MAX_ATTACHMENT_BYTES)
        {
            return Err(StoreError::Conflict("ek 10 MB'tan büyük olamaz".into()));
        }
        let id = notai_core::ids::EntryId::new().to_string();
        let payload = AttachmentPayload {
            id: id.clone(),
            entry: a.entry.to_string(),
            kind: a.kind,
            name: a.name.clone(),
            mime: a.mime.clone(),
            data_b64: a.data.as_ref().map(|d| B64.encode(d)),
            value: a.value.clone(),
        };
        let mut tx = self.pool.begin().await?;
        crate::entries::load_entry(&mut tx, a.entry).await?;
        insert_attachment(&mut tx, &payload, a.data.as_deref(), now).await?;
        log_change(
            &mut tx,
            &self.origin,
            "attachment",
            &id,
            Some(&serde_json::to_string(&payload)?),
            now,
        )
        .await?;
        tx.commit().await?;
        Ok(AttachmentMeta {
            id,
            entry: payload.entry,
            kind: a.kind,
            name: a.name,
            mime: a.mime,
            size: a.data.map_or(0, |d| d.len() as u64),
            value: a.value,
        })
    }

    pub async fn attachments_for(&self, entry: EntryId) -> Result<Vec<AttachmentMeta>> {
        let e = entry.to_string();
        let rows = sqlx::query!(
            r#"SELECT id, entry_id, kind, name, mime, COALESCE(length(data), 0) AS "size!: i64", value
               FROM attachments WHERE entry_id = ?1 ORDER BY created_at"#,
            e
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|r| {
                Ok(AttachmentMeta {
                    id: r.id,
                    entry: r.entry_id,
                    kind: AttachmentKind::parse(&r.kind)?,
                    name: r.name,
                    mime: r.mime,
                    size: u64::try_from(r.size).unwrap_or(0),
                    value: r.value,
                })
            })
            .collect()
    }

    /// Ekin içeriği: (mime, bayt).
    pub async fn attachment_data(&self, id: &str) -> Result<(String, Vec<u8>)> {
        let r = sqlx::query!("SELECT mime, data FROM attachments WHERE id = ?1", id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| StoreError::not_found("ek", id))?;
        let data = r
            .data
            .ok_or_else(|| StoreError::not_found("ek içeriği", id))?;
        Ok((r.mime, data))
    }

    // ---------- Anlamsal arama vektörleri ----------

    /// Vektörü olmayan (ya da başka modelle üretilmiş) son kayıtlar.
    pub async fn entries_without_embedding(&self, model: &str, limit: u32) -> Result<Vec<Entry>> {
        let limit = i64::from(limit.clamp(1, 200));
        let rows = sqlx::query_as!(
            EntryRow,
            "SELECT e.id, e.text, e.source, e.captured_at, e.topic_id, e.status, e.trashed_at
             FROM entries e LEFT JOIN entry_embeddings v ON v.entry_id = e.id AND v.model = ?1
             WHERE v.entry_id IS NULL AND e.status != 'trashed'
             ORDER BY e.captured_at DESC LIMIT ?2",
            model,
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(Entry::try_from).collect()
    }

    pub async fn save_embedding(
        &self,
        entry: EntryId,
        model: &str,
        vector: &[f32],
        now: DateTime<Utc>,
    ) -> Result<()> {
        if vector.is_empty() {
            return Err(StoreError::Conflict("boş vektör".into()));
        }
        let bytes: Vec<u8> = vector.iter().flat_map(|f| f.to_le_bytes()).collect();
        let dims = i64::try_from(vector.len()).unwrap_or(i64::MAX);
        let (e, at) = (entry.to_string(), ts(now));
        sqlx::query!(
            "INSERT INTO entry_embeddings (entry_id, model, dims, vector, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (entry_id) DO UPDATE SET model = excluded.model, dims = excluded.dims,
                 vector = excluded.vector, created_at = excluded.created_at",
            e,
            model,
            dims,
            bytes,
            at
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Bir modelin tüm vektörleri (çöp ve arşiv dışı kayıtlar).
    pub async fn embeddings(&self, model: &str) -> Result<Vec<(EntryId, Vec<f32>)>> {
        let rows = sqlx::query!(
            "SELECT v.entry_id, v.vector FROM entry_embeddings v JOIN entries e ON e.id = v.entry_id
             WHERE v.model = ?1 AND e.status != 'trashed'",
            model
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|r| {
                let v = r
                    .vector
                    .chunks_exact(4)
                    .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect();
                Ok((crate::codec::parse_id("vektör", &r.entry_id)?, v))
            })
            .collect()
    }

    // ---------- Öğrenme örnekleri ----------

    /// Kullanıcının son kararları (AI istemine örnek olarak girer). Otonom kararlar hariç.
    pub async fn decision_examples(&self, limit: u32) -> Result<Vec<DecisionExample>> {
        let limit = i64::from(limit.clamp(1, 50));
        let rows = sqlx::query!(
            "SELECT e.text, s.payload, s.decision FROM suggestions s JOIN entries e ON e.id = s.entry_id
             WHERE s.decision != 'pending' AND s.auto = 0 AND s.kind != 'question'
             ORDER BY s.decided_at DESC LIMIT ?1",
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|r| {
                Ok(DecisionExample {
                    entry_text: r.text,
                    kind: from_json("öneri", &r.payload)?,
                    accepted: r.decision == "accepted",
                })
            })
            .collect()
    }

    // ---------- Çöp kutusu ----------

    pub async fn trash(&self) -> Result<Vec<Entry>> {
        let rows = sqlx::query_as!(
            EntryRow,
            "SELECT id, text, source, captured_at, topic_id, status, trashed_at
             FROM entries WHERE status = 'trashed' ORDER BY trashed_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(Entry::try_from).collect()
    }

    /// Çöpteki tek kaydı kalıcı siler (kullanıcının ayrı onayıyla; yalnız çöptekiler).
    pub async fn purge_entry(&self, id: EntryId, now: DateTime<Utc>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let entry = crate::entries::load_entry(&mut tx, id).await?;
        if !entry.is_trashed() {
            return Err(StoreError::Conflict(
                "yalnız çöp kutusundaki kayıt kalıcı silinebilir".into(),
            ));
        }
        let id_s = id.to_string();
        sqlx::query!("DELETE FROM entries WHERE id = ?1", id_s)
            .execute(&mut *tx)
            .await?;
        log_change(&mut tx, &self.origin, "entry", &id_s, None, now).await?;
        tx.commit().await?;
        Ok(())
    }
}

pub(crate) async fn insert_attachment(
    conn: &mut sqlx::SqliteConnection,
    p: &AttachmentPayload,
    data: Option<&[u8]>,
    now: DateTime<Utc>,
) -> Result<()> {
    let kind = p.kind.name();
    let at = ts(now);
    sqlx::query!(
        "INSERT OR IGNORE INTO attachments (id, entry_id, kind, name, mime, data, value, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        p.id,
        p.entry,
        kind,
        p.name,
        p.mime,
        data,
        p.value,
        at
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub(crate) fn decode_attachment(p: &AttachmentPayload) -> Result<Option<Vec<u8>>> {
    p.data_b64
        .as_deref()
        .map(|s| B64.decode(s).map_err(|e| StoreError::corrupt("ek", e)))
        .transpose()
}
