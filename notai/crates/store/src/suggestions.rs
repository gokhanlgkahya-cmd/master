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

/// Kabulün geri alınması için gereken önceki durum.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Undo {
    prev_topic: Option<String>,
    prev_status: String,
    created_task: Option<String>,
    label: Option<(String, String)>,
}

/// Kararın sonucu: güncel öneri ve kayıt, varsa oluşturulan görev/konu.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionOutcome {
    pub suggestion: Suggestion,
    pub entry: Entry,
    pub created_task: Option<Task>,
    pub created_topic: Option<Topic>,
}

/// Öneriyi yazar (günlüğe yazmaz).
pub(crate) async fn insert_suggestion_raw(
    conn: &mut SqliteConnection,
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
    Ok(())
}

pub(crate) async fn insert_suggestion(
    conn: &mut SqliteConnection,
    origin: &str,
    s: &Suggestion,
    now: DateTime<Utc>,
) -> Result<()> {
    insert_suggestion_raw(conn, s, now).await?;
    let snapshot = serde_json::to_string(s)?;
    log_change(
        conn,
        origin,
        "suggestion",
        &s.id.to_string(),
        Some(&snapshot),
        now,
    )
    .await
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

/// Öneri kararını yazar (günlüğe yazmaz).
pub(crate) async fn write_decision_raw(conn: &mut SqliteConnection, s: &Suggestion) -> Result<()> {
    let id = s.id.to_string();
    let decision = s.decision().name();
    let decided_at = s.decision().decided_at().map(ts);
    // İçerik de yazılır: AI sorusunun cevabı öneri içeriğindedir.
    let payload = serde_json::to_string(&s.kind)?;
    sqlx::query!(
        "UPDATE suggestions SET decision = ?2, decided_at = ?3, payload = ?4 WHERE id = ?1",
        id,
        decision,
        decided_at,
        payload
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn write_decision(
    conn: &mut SqliteConnection,
    origin: &str,
    s: &Suggestion,
    now: DateTime<Utc>,
) -> Result<()> {
    write_decision_raw(conn, s).await?;
    let snapshot = serde_json::to_string(s)?;
    log_change(
        conn,
        origin,
        "suggestion",
        &s.id.to_string(),
        Some(&snapshot),
        now,
    )
    .await
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

/// Kaydı yeniden AI kuyruğuna koyar (ör. AI sorusu cevaplandı; cevapla yeniden sınıflandırılsın).
pub(crate) async fn requeue_ai(conn: &mut SqliteConnection, entry: EntryId) -> Result<()> {
    let id = entry.to_string();
    sqlx::query!(
        "UPDATE entries SET ai_state = 'pending', ai_attempts = 0, ai_next_at = NULL, ai_error = NULL
         WHERE id = ?1 AND status != 'trashed'",
        id
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Bir kayıt için en fazla bu kadar soru sorulur (sonsuz soru döngüsü olmasın).
pub const MAX_QUESTIONS_PER_ENTRY: usize = 2;

/// Sınıflandırma istemi için soru-cevap bağlamı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaContext {
    /// (kayıt metni, soru, cevap, bu kayda mı ait)
    pub items: Vec<(String, String, String, bool)>,
    /// Bu kayıt için yeni soru sorulabilir mi?
    pub can_ask: bool,
}

/// Bildirilecek yeni AI sorusu.
#[derive(Debug, Clone, PartialEq)]
pub struct NewQuestion {
    pub suggestion: Suggestion,
    pub topic: Option<notai_core::TopicId>,
    pub created_at: DateTime<Utc>,
}

impl Store {
    /// AI sorusunu cevaplar: cevap öneriye yazılır (senkronlanır) ve kayıt cevapla birlikte
    /// yeniden AI kuyruğuna girer.
    pub async fn answer_question(
        &self,
        id: SuggestionId,
        answer: notai_core::text::AnswerText,
        now: DateTime<Utc>,
    ) -> Result<Entry> {
        let mut tx = self.pool.begin().await?;
        let mut suggestion = load_suggestion(&mut tx, id).await?;
        let entry = load_entry(&mut tx, suggestion.entry).await?;
        suggestion.answer(answer, now)?;
        write_decision(&mut tx, &self.origin, &suggestion, now).await?;
        requeue_ai(&mut tx, entry.id).await?;
        tx.commit().await?;
        Ok(entry)
    }

    /// Sınıflandırma için soru-cevap bağlamı: bu kaydın cevaplanmış soruları + son
    /// `others` başka cevap (öğrenme). Bekleyen soru varsa ya da sınıra ulaşıldıysa yeni soru yok.
    pub async fn qa_context(&self, entry: EntryId, others: u32) -> Result<QaContext> {
        let entry_s = entry.to_string();
        let mine = sqlx::query!(
            r#"SELECT s.payload, s.decision FROM suggestions s
                WHERE s.entry_id = ?1 AND s.kind = 'question'
                ORDER BY s.created_at"#,
            entry_s
        )
        .fetch_all(&self.pool)
        .await?;
        let entry_text = load_entry(&mut *self.pool.acquire().await?, entry)
            .await?
            .text
            .as_str()
            .to_owned();
        let mut items = Vec::new();
        let mut pending = false;
        for r in &mine {
            let kind: notai_core::SuggestionKind = crate::codec::from_json("soru", &r.payload)?;
            if let notai_core::SuggestionKind::Question { text, answer } = kind {
                match answer {
                    Some(a) => items.push((
                        entry_text.clone(),
                        text.as_str().to_owned(),
                        a.as_str().to_owned(),
                        true,
                    )),
                    None if r.decision == "pending" => pending = true,
                    None => {}
                }
            }
        }
        let can_ask = !pending && mine.len() < MAX_QUESTIONS_PER_ENTRY;
        let rows = sqlx::query!(
            r#"SELECT e.text, s.payload FROM suggestions s JOIN entries e ON e.id = s.entry_id
                WHERE s.kind = 'question' AND s.decision = 'accepted' AND s.entry_id != ?1
                ORDER BY s.decided_at DESC LIMIT ?2"#,
            entry_s,
            others
        )
        .fetch_all(&self.pool)
        .await?;
        for r in rows {
            let kind: notai_core::SuggestionKind = crate::codec::from_json("soru", &r.payload)?;
            if let notai_core::SuggestionKind::Question {
                text,
                answer: Some(a),
            } = kind
            {
                items.push((
                    r.text,
                    text.as_str().to_owned(),
                    a.as_str().to_owned(),
                    false,
                ));
            }
        }
        Ok(QaContext { items, can_ask })
    }

    /// `after`'dan sonra oluşan, hâlâ bekleyen AI soruları (bildirim için; eskiden yeniye).
    pub async fn pending_questions_since(&self, after: DateTime<Utc>) -> Result<Vec<NewQuestion>> {
        let after_s = ts(after);
        let rows = sqlx::query!(
            r#"SELECT s.id, s.created_at, e.topic_id FROM suggestions s
                 JOIN entries e ON e.id = s.entry_id
                WHERE s.kind = 'question' AND s.decision = 'pending' AND s.created_at > ?1
                  AND e.status != 'trashed'
                ORDER BY s.created_at"#,
            after_s
        )
        .fetch_all(&self.pool)
        .await?;
        let mut out = Vec::new();
        let mut conn = self.pool.acquire().await?;
        for r in rows {
            let id: SuggestionId = crate::codec::parse_id("soru", &r.id)?;
            out.push(NewQuestion {
                suggestion: load_suggestion(&mut conn, id).await?,
                topic: r
                    .topic_id
                    .as_deref()
                    .map(|t| crate::codec::parse_id("konu", t))
                    .transpose()?,
                created_at: crate::codec::parse_ts("soru", &r.created_at)?,
            });
        }
        Ok(out)
    }

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
        self.decide_with(id, accept, false, now, tz).await
    }

    /// `decide` ile aynı; `auto = true` ise kararı otonom mod verdi (geri alınabilir).
    pub async fn decide_with(
        &self,
        id: SuggestionId,
        accept: bool,
        auto: bool,
        now: DateTime<Utc>,
        tz: Tz,
    ) -> Result<DecisionOutcome> {
        let mut tx = self.pool.begin().await?;
        let mut suggestion = load_suggestion(&mut tx, id).await?;
        let mut entry = load_entry(&mut tx, suggestion.entry).await?;
        let mut created_task = None;
        let mut created_topic = None;
        let mut undo = Undo {
            prev_topic: entry.topic().map(|t| t.to_string()),
            prev_status: entry.status().name().to_owned(),
            created_task: None,
            label: None,
        };

        if accept {
            match suggestion.accept(&entry, now, tz)? {
                Effect::AssignTopic { topic, .. } => {
                    get_topic(&mut tx, topic).await?;
                    entry.assign_topic(Some(topic))?;
                }
                Effect::CreateTopicAndAssign {
                    name, kind, parent, ..
                } => {
                    if let Some(p) = parent {
                        get_topic(&mut tx, p).await?;
                    }
                    let topic = match find_by_name(&mut tx, parent, &name).await? {
                        Some(existing) => existing,
                        None => {
                            let mut t = Topic::new(name, kind);
                            t.parent = parent;
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
                    undo.created_task = Some(task.id.to_string());
                    created_task = Some(task);
                }
                Effect::AddTag { entry: e, name } => {
                    add_label(&mut tx, &self.origin, e, "tag", &name, now).await?;
                    undo.label = Some(("tag".into(), name.as_str().to_owned()));
                }
                Effect::AddPerson { entry: e, name } => {
                    add_label(&mut tx, &self.origin, e, "person", &name, now).await?;
                    undo.label = Some(("person".into(), name.as_str().to_owned()));
                }
            }
        } else {
            suggestion.dismiss(now)?;
        }
        write_decision(&mut tx, &self.origin, &suggestion, now).await?;
        if accept {
            let id_s = suggestion.id.to_string();
            let undo_json = serde_json::to_string(&undo)?;
            let auto_i = i64::from(auto);
            sqlx::query!(
                "UPDATE suggestions SET undo = ?2, auto = ?3 WHERE id = ?1",
                id_s,
                undo_json,
                auto_i
            )
            .execute(&mut *tx)
            .await?;
        }

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

    /// Kabul edilmiş öneriyi geri alır: etkinin tersini uygular (konu eski haline döner,
    /// oluşan görev iptal edilir, eklenen etiket kaldırılır) ve kararı "yoksayıldı" yapar.
    pub async fn undo_decision(&self, id: SuggestionId, now: DateTime<Utc>) -> Result<Entry> {
        let mut tx = self.pool.begin().await?;
        let mut suggestion = load_suggestion(&mut tx, id).await?;
        let id_s = id.to_string();
        let undo_json = sqlx::query_scalar!("SELECT undo FROM suggestions WHERE id = ?1", id_s)
            .fetch_one(&mut *tx)
            .await?
            .ok_or_else(|| StoreError::Conflict("bu öneri geri alınamaz".into()))?;
        let undo: Undo = crate::codec::from_json("geri alma", &undo_json)?;
        suggestion.undo_accept(now)?;
        let entry = load_entry(&mut tx, suggestion.entry).await?;

        let prev_topic = undo
            .prev_topic
            .as_deref()
            .map(|t| crate::codec::parse_id("geri alma", t))
            .transpose()?;
        let prev_status = if undo.prev_status == "inbox" {
            notai_core::EntryStatus::Inbox
        } else {
            entry.status()
        };
        let restored = Entry::from_parts(
            entry.id,
            entry.text.clone(),
            entry.source,
            entry.captured_at,
            prev_topic,
            if entry.is_trashed() {
                entry.status()
            } else {
                prev_status
            },
        );
        if let Some(task_id) = undo.created_task.as_deref() {
            let task_id: notai_core::TaskId = crate::codec::parse_id("geri alma", task_id)?;
            crate::tasks::cancel_in_tx(&mut tx, &self.origin, task_id, now).await?;
        }
        if let Some((kind, name)) = &undo.label {
            let entry_s = entry.id.to_string();
            sqlx::query!(
                "DELETE FROM entry_labels WHERE entry_id = ?1 AND kind = ?2 AND name = ?3",
                entry_s,
                kind,
                name
            )
            .execute(&mut *tx)
            .await?;
        }
        write_decision(&mut tx, &self.origin, &suggestion, now).await?;
        sqlx::query!("UPDATE suggestions SET undo = NULL WHERE id = ?1", id_s)
            .execute(&mut *tx)
            .await?;
        update_entry(&mut tx, &self.origin, &restored, now).await?;
        tx.commit().await?;
        Ok(restored)
    }
}
