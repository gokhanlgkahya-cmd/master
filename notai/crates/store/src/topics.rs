//! Konular (PARA).

use chrono::{DateTime, Utc};
use notai_core::TopicId;
use notai_core::journal::{TopicActivity, order_topics};
use notai_core::text::{EntryText, TopicName};
use notai_core::topic::{KindFilter, ParaKind, Topic};
use notai_core::tr::tr_lower;
use sqlx::SqliteConnection;

use crate::codec::{TopicRow, date_str, parse_ts, ts};
use crate::error::{Result, StoreError};
use crate::{Store, log_change};

/// Yeni konu girdileri.
#[derive(Debug, Clone)]
pub struct NewTopic {
    pub name: TopicName,
    pub kind: ParaKind,
    pub icon: Option<String>,
    pub pinned: bool,
}

impl NewTopic {
    #[must_use]
    pub const fn new(name: TopicName, kind: ParaKind) -> Self {
        Self {
            name,
            kind,
            icon: None,
            pinned: false,
        }
    }
}

pub(crate) fn name_key(name: &TopicName) -> String {
    tr_lower(name.as_str())
}

/// Konuyu ekler; aynı adda (Türkçe büyük/küçük harf duyarsız) konu varsa `Conflict`.
pub(crate) async fn insert_topic(
    conn: &mut SqliteConnection,
    origin: &str,
    topic: &Topic,
    now: DateTime<Utc>,
) -> Result<()> {
    let id = topic.id.to_string();
    let key = name_key(&topic.name);
    let name = topic.name.as_str();
    let kind = topic.kind.name();
    let due = topic.kind.due().map(date_str);
    let pinned = i64::from(topic.pinned);
    let now_s = ts(now);
    let res = sqlx::query!(
        "INSERT INTO topics (id, name, name_key, kind, due_date, icon, avatar, pinned, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?9, ?7, ?8, ?8)
         ON CONFLICT (name_key) DO NOTHING",
        id,
        name,
        key,
        kind,
        due,
        topic.icon,
        pinned,
        now_s,
        topic.avatar
    )
    .execute(&mut *conn)
    .await?;
    if res.rows_affected() == 0 {
        return Err(StoreError::Conflict(format!(
            "\"{name}\" adında bir konu zaten var"
        )));
    }
    let payload = serde_json::to_string(topic)?;
    log_change(conn, origin, "topic", &id, Some(&payload), now).await
}

pub(crate) async fn find_by_name(
    conn: &mut SqliteConnection,
    name: &TopicName,
) -> Result<Option<Topic>> {
    let key = name_key(name);
    let row = sqlx::query_as!(
        TopicRow,
        "SELECT id, name, kind, due_date, icon, avatar, pinned FROM topics WHERE name_key = ?1",
        key
    )
    .fetch_optional(&mut *conn)
    .await?;
    row.map(Topic::try_from).transpose()
}

pub(crate) async fn get_topic(conn: &mut SqliteConnection, id: TopicId) -> Result<Topic> {
    let id_s = id.to_string();
    let row = sqlx::query_as!(
        TopicRow,
        "SELECT id, name, kind, due_date, icon, avatar, pinned FROM topics WHERE id = ?1",
        id_s
    )
    .fetch_optional(&mut *conn)
    .await?;
    row.map(Topic::try_from)
        .transpose()?
        .ok_or_else(|| StoreError::not_found("konu", id))
}

impl Store {
    /// Yeni konu açar.
    pub async fn create_topic(&self, new: NewTopic, now: DateTime<Utc>) -> Result<Topic> {
        let mut topic = Topic::new(new.name, new.kind);
        topic.icon = new.icon;
        topic.pinned = new.pinned;
        let mut tx = self.pool.begin().await?;
        insert_topic(&mut tx, &self.origin, &topic, now).await?;
        tx.commit().await?;
        Ok(topic)
    }

    pub async fn topic(&self, id: TopicId) -> Result<Topic> {
        let mut conn = self.pool.acquire().await?;
        get_topic(&mut conn, id).await
    }

    /// Adıyla konu bulur (Türkçe büyük/küçük harf duyarsız).
    pub async fn topic_by_name(&self, name: &TopicName) -> Result<Option<Topic>> {
        let mut conn = self.pool.acquire().await?;
        find_by_name(&mut conn, name).await
    }

    /// Konunun adını, türünü, simgesini, sabitlemesini günceller.
    pub async fn update_topic(&self, topic: &Topic, now: DateTime<Utc>) -> Result<()> {
        let id = topic.id.to_string();
        let key = name_key(&topic.name);
        let name = topic.name.as_str();
        let kind = topic.kind.name();
        let due = topic.kind.due().map(date_str);
        let pinned = i64::from(topic.pinned);
        let now_s = ts(now);
        let mut tx = self.pool.begin().await?;
        let res = sqlx::query!(
            "UPDATE topics SET name = ?2, name_key = ?3, kind = ?4, due_date = ?5, icon = ?6,
                    pinned = ?7, updated_at = ?8, avatar = ?9
             WHERE id = ?1",
            id,
            name,
            key,
            kind,
            due,
            topic.icon,
            pinned,
            now_s,
            topic.avatar
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                StoreError::Conflict(format!("\"{name}\" adında bir konu zaten var"))
            }
            other => StoreError::Db(other),
        })?;
        if res.rows_affected() == 0 {
            return Err(StoreError::not_found("konu", id));
        }
        let payload = serde_json::to_string(topic)?;
        log_change(&mut tx, &self.origin, "topic", &id, Some(&payload), now).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Ana ekran konu listesi: son kayıt önizlemesiyle, sabitlenenler önce, sonra son etkinliğe göre.
    pub async fn topic_list(&self, filter: KindFilter) -> Result<Vec<TopicActivity>> {
        let rows = sqlx::query!(
            r#"SELECT t.id, t.name, t.kind, t.due_date, t.icon, t.avatar, t.pinned,
                      (SELECT e.captured_at FROM entries e
                        WHERE e.topic_id = t.id AND e.status IN ('inbox', 'processed')
                        ORDER BY e.captured_at DESC LIMIT 1) AS "last_at: String",
                      (SELECT e.text FROM entries e
                        WHERE e.topic_id = t.id AND e.status IN ('inbox', 'processed')
                        ORDER BY e.captured_at DESC LIMIT 1) AS "last_text: String",
                      (SELECT COUNT(DISTINCT s.entry_id) FROM suggestions s
                         JOIN entries e ON e.id = s.entry_id
                        WHERE e.topic_id = t.id AND s.decision = 'pending'
                          AND e.status IN ('inbox', 'processed')) AS "pending!: i64"
               FROM topics t"#
        )
        .fetch_all(&self.pool)
        .await?;
        // Simge seçimi için her konunun son kayıtları (en yeni önce, konu başına en fazla 20).
        let recent_rows = sqlx::query!(
            r#"SELECT topic_id AS "topic_id!: String", text AS "text!: String"
               FROM (SELECT e.topic_id, e.text,
                            ROW_NUMBER() OVER (PARTITION BY e.topic_id
                                               ORDER BY e.captured_at DESC) AS rn
                       FROM entries e
                      WHERE e.topic_id IS NOT NULL AND e.status IN ('inbox', 'processed'))
              WHERE rn <= 20
              ORDER BY topic_id, rn"#
        )
        .fetch_all(&self.pool)
        .await?;
        let mut recent: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for r in recent_rows {
            recent.entry(r.topic_id).or_default().push(r.text);
        }
        // Renk sırası filtreden önce, tüm konular üzerinden: filtre ya da arşiv rengi kaydırmaz.
        let slots = notai_core::look::color_slots(rows.iter().filter_map(|r| r.id.parse().ok()));
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let recent_texts = recent.remove(&r.id).unwrap_or_default();
            let topic = Topic::try_from(TopicRow {
                id: r.id,
                name: r.name,
                kind: r.kind,
                due_date: r.due_date,
                icon: r.icon,
                avatar: r.avatar,
                pinned: r.pinned,
            })?;
            if !filter.matches(topic.kind) {
                continue;
            }
            let color = slots.get(&topic.id).copied().unwrap_or(0);
            let pending = u64::try_from(r.pending).unwrap_or(0);
            let last_at = r
                .last_at
                .as_deref()
                .map(|s| parse_ts("konu", s))
                .transpose()?;
            let last_preview = r
                .last_text
                .as_deref()
                .map(EntryText::new)
                .transpose()?
                .map(|t| t.preview(80));
            out.push(TopicActivity {
                topic,
                last_at,
                last_preview,
                recent_texts,
                color,
                pending,
            });
        }
        order_topics(&mut out);
        Ok(out)
    }
}
