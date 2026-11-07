//! AI önerileri: kaydetme ve kullanıcı kararı (kabul / yoksay).

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use notai_core::suggestion::{Effect, Suggestion, all_decided};
use notai_core::task::Task;
use notai_core::text::Label;
use notai_core::topic::Topic;
use notai_core::{Entry, EntryId, SuggestionId};
use sqlx::SqliteConnection;

use crate::codec::{SuggestionRow, ts};
use crate::entries::{load_entry, update_entry};
use crate::error::{Result, StoreError};
use crate::tasks::insert_task;
use crate::topics::{find_by_name, get_topic, insert_topic};
use crate::{Store, log_change};

/// Kararın sonucu: güncel öneri ve kayıt, varsa oluşturulan görev/konu.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionOutcome {
    pub suggestion: Suggestion,
    pub entry: Entry,
    pub created_task: Option<Task>,
    pub created_topic: Option<Topic>,
}

pub(crate) async fn insert_suggestion(
    conn: &mut SqliteConnection,
    origin: &str,
    s: &Suggestion,
    now: DateTime<Utc>,
) -> Result<()> {
    let id = s.id.to_string();
    let entry = s.entry.to_string();
    let kind = s.kind.name();
    let payload = serde_json::to_string(&s.kind)?;
    let confidence = s.confidence.get();
    let decision = s.decision().name();
    let decided_at = s.decision().decided_at().map(ts);
    let now_s = ts(now);
    sqlx::query!(
        "INSERT INTO suggestions (id, entry_id, kind, payload, confidence, decision, decided_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        id,
        entry,
        kind,
        payload,
        confidence,
        decision,
        decided_at,
        now_s
    )
    .execute(&mut *conn)
    .await?;
    let snapshot = serde_json::to_string(s)?;
    log_change(conn, origin, "suggestion", &id, Some(&snapshot), now).await
}

async fn load_suggestion(conn: &mut SqliteConnection, id: SuggestionId) -> Result<Suggestion> {
    let id_s = id.to_string();
    let row = sqlx::query_as!(
        SuggestionRow,
        "SELECT id, entry_id, payload, confidence, decision, decided_at FROM suggestions WHERE id = ?1",
        id_s
    )
    .fetch_optional(&mut *conn)
    .await?;
    row.map(Suggestion::try_from)
        .transpose()?
        .ok_or_else(|| StoreError::not_found("öneri", id))
}

async fn entry_suggestions(conn: &mut SqliteConnection, entry: EntryId) -> Result<Vec<Suggestion>> {
    let id = entry.to_string();
    let rows = sqlx::query_as!(
        SuggestionRow,
        "SELECT id, entry_id, payload, confidence, decision, decided_at
         FROM suggestions WHERE entry_id = ?1 ORDER BY created_at, id",
        id
    )
    .fetch_all(&mut *conn)
    .await?;
    rows.into_iter().map(Suggestion::try_from).collect()
}

async fn write_decision(
    conn: &mut SqliteConnection,
    origin: &str,
    s: &Suggestion,
    now: DateTime<Utc>,
) -> Result<()> {
    let id = s.id.to_string();
    let decision = s.decision().name();
    let decided_at = s.decision().decided_at().map(ts);
    sqlx::query!(
        "UPDATE suggestions SET decision = ?2, decided_at = ?3 WHERE id = ?1",
        id,
        decision,
        decided_at
    )
    .execute(&mut *conn)
    .await?;
    let snapshot = serde_json::to_string(s)?;
    log_change(conn, origin, "suggestion", &id, Some(&snapshot), now).await
}

async fn add_label(
    conn: &mut SqliteConnection,
    origin: &str,
    entry: EntryId,
    kind: &str,
    name: &Label,
    now: DateTime<Utc>,
) -> Result<()> {
    let entry_s = entry.to_string();
    let name_s = name.as_str();
    sqlx::query!(
        "INSERT OR IGNORE INTO entry_labels (entry_id, kind, name) VALUES (?1, ?2, ?3)",
        entry_s,
        kind,
        name_s
    )
    .execute(&mut *conn)
    .await?;
    let payload = serde_json::json!({ "entry": entry_s, "kind": kind, "name": name_s }).to_string();
    let label_id = format!("{entry_s}/{kind}/{name_s}");
    log_change(conn, origin, "label", &label_id, Some(&payload), now).await
}

impl Store {
    /// Bir kaydın önerileri (oluşturulma sırasıyla).
    pub async fn suggestions_for(&self, entry: EntryId) -> Result<Vec<Suggestion>> {
        let mut conn = self.pool.acquire().await?;
        entry_suggestions(&mut conn, entry).await
    }

    /// Öneriyi kabul eder ya da yoksayar; kabulde etkisini aynı transaction'da uygular.
    /// Kaydın tüm önerileri karara bağlanınca kayıt "işlendi" olur.
    pub async fn decide(
        &self,
        id: SuggestionId,
        accept: bool,
        now: DateTime<Utc>,
        tz: Tz,
    ) -> Result<DecisionOutcome> {
        let mut tx = self.pool.begin().await?;
        let mut suggestion = load_suggestion(&mut tx, id).await?;
        let mut entry = load_entry(&mut tx, suggestion.entry).await?;
        let mut created_task = None;
        let mut created_topic = None;

        if accept {
            match suggestion.accept(&entry, now, tz)? {
                Effect::AssignTopic { topic, .. } => {
                    get_topic(&mut tx, topic).await?;
                    entry.assign_topic(Some(topic))?;
                }
                Effect::CreateTopicAndAssign { name, kind, .. } => {
                    let topic = match find_by_name(&mut tx, &name).await? {
                        Some(existing) => existing,
                        None => {
                            let t = Topic::new(name, kind);
                            insert_topic(&mut tx, &self.origin, &t, now).await?;
                            created_topic = Some(t.clone());
                            t
                        }
                    };
                    entry.assign_topic(Some(topic.id))?;
                }
                Effect::CreateTask { task } => {
                    let task = Task::create(task)?;
                    insert_task(&mut tx, &self.origin, &task, now).await?;
                    created_task = Some(task);
                }
                Effect::AddTag { entry: e, name } => {
                    add_label(&mut tx, &self.origin, e, "tag", &name, now).await?;
                }
                Effect::AddPerson { entry: e, name } => {
                    add_label(&mut tx, &self.origin, e, "person", &name, now).await?;
                }
            }
        } else {
            suggestion.dismiss(now)?;
        }
        write_decision(&mut tx, &self.origin, &suggestion, now).await?;

        let all = entry_suggestions(&mut tx, entry.id).await?;
        if all_decided(&all) && entry.is_unprocessed() {
            entry.mark_processed()?;
        }
        update_entry(&mut tx, &self.origin, &entry, now).await?;
        tx.commit().await?;
        Ok(DecisionOutcome {
            suggestion,
            entry,
            created_task,
            created_topic,
        })
    }
}
