//! Uygulama komutlarının mantığı (Tauri'den bağımsız; testlerle sınanır).
//! Tauri komutları (`lib.rs`) bu fonksiyonları yalnız sarar.

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use notai_api::{CaptureRequest, JournalView, MorningPlanView, TasksView, TopicsView, convert};
use notai_core::entry::Source;
use notai_core::journal::{JournalFilter, list_time_label};
use notai_core::text::EntryText;
use notai_core::topic::KindFilter;
use notai_core::{EntryId, SuggestionId, TaskId, TopicId};
use notai_store::{NewEntry, Store, TaskView};

/// Komut hatası: arayüze Türkçe mesaj olarak gider.
pub type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn id<T: std::str::FromStr>(s: &str) -> CmdResult<T> {
    s.parse().map_err(|_| format!("geçersiz kimlik: {s}"))
}

pub async fn topics(
    store: &Store,
    kind: &str,
    now: DateTime<Utc>,
    tz: Tz,
) -> CmdResult<TopicsView> {
    let filter = match kind {
        "projects" => KindFilter::Projects,
        "areas" => KindFilter::Areas,
        "resources" => KindFilter::Resources,
        "archived" => KindFilter::Archived,
        _ => KindFilter::All,
    };
    let topics = store
        .topic_list(filter)
        .await
        .map_err(err)?
        .into_iter()
        .map(|a| convert::topic_row(a, now, tz))
        .collect();
    let unprocessed = store.unprocessed_count().await.map_err(err)?;
    let last = store
        .journal(JournalFilter::All, None, 1)
        .await
        .map_err(err)?;
    let last = last.last();
    Ok(TopicsView {
        topics,
        unprocessed,
        journal_preview: last.map(|i| i.entry.text.preview(80)),
        journal_last_label: last.map(|i| list_time_label(i.entry.captured_at, now, tz)),
    })
}

pub async fn journal(
    store: &Store,
    topic: Option<&str>,
    now: DateTime<Utc>,
    tz: Tz,
) -> CmdResult<JournalView> {
    let filter = match topic {
        Some(t) => JournalFilter::Topic {
            id: id::<TopicId>(t)?,
        },
        None => JournalFilter::All,
    };
    const LIMIT: u32 = 100;
    let items = store.journal(filter, None, LIMIT).await.map_err(err)?;
    let unprocessed = store.unprocessed_count().await.map_err(err)?;
    Ok(convert::journal(items, LIMIT, unprocessed, now, tz))
}

pub async fn capture(store: &Store, req: CaptureRequest, now: DateTime<Utc>) -> CmdResult<String> {
    let topic = req.topic_id.as_deref().map(id::<TopicId>).transpose()?;
    let captured = store
        .capture(NewEntry {
            text: EntryText::new(&req.text).map_err(err)?,
            source: Source::App,
            topic,
            captured_at: now,
            dedupe_key: None,
        })
        .await
        .map_err(err)?;
    Ok(captured.entry().id.to_string())
}

pub async fn decide(
    store: &Store,
    suggestion: &str,
    accept: bool,
    now: DateTime<Utc>,
    tz: Tz,
) -> CmdResult<()> {
    store
        .decide(id::<SuggestionId>(suggestion)?, accept, now, tz)
        .await
        .map(|_| ())
        .map_err(err)
}

pub async fn entry_action(
    store: &Store,
    entry: &str,
    action: &str,
    now: DateTime<Utc>,
) -> CmdResult<()> {
    let e: EntryId = id(entry)?;
    let r = match action {
        "archive" => store.archive_entry(e, now).await,
        "unarchive" => store.unarchive_entry(e, now).await,
        "trash" => store.trash_entry(e, now).await,
        "restore" => store.restore_entry(e, now).await,
        other => return Err(format!("bilinmeyen eylem: {other}")),
    };
    r.map(|_| ()).map_err(err)
}

pub async fn tasks(store: &Store, now: DateTime<Utc>, tz: Tz) -> CmdResult<TasksView> {
    let today = now.with_timezone(&tz).date_naive();
    let rows =
        |v: Vec<notai_core::task::Task>| v.iter().map(|t| convert::task_row(t, today)).collect();
    Ok(TasksView {
        overdue: rows(store.tasks(TaskView::Overdue, today).await.map_err(err)?),
        today: rows(store.tasks(TaskView::Today, today).await.map_err(err)?),
        upcoming: rows(store.tasks(TaskView::Upcoming, today).await.map_err(err)?),
        undated: rows(store.tasks(TaskView::Undated, today).await.map_err(err)?),
    })
}

pub async fn complete_task(store: &Store, task: &str, now: DateTime<Utc>) -> CmdResult<()> {
    store
        .complete_task(id::<TaskId>(task)?, now)
        .await
        .map(|_| ())
        .map_err(err)
}

pub async fn morning_plan(
    store: &Store,
    user: &str,
    now: DateTime<Utc>,
    tz: Tz,
) -> CmdResult<MorningPlanView> {
    let today = now.with_timezone(&tz).date_naive();
    let plan = store.morning_plan(today).await.map_err(err)?;
    Ok(convert::morning_plan(plan, user))
}

pub const SYNC_URL: &str = "sync_server_url";
pub const SYNC_TOKEN: &str = "sync_token";

/// Ev sunucusu adresini ve belirtecini kaydeder (Ayarlar → Senkron).
pub async fn set_sync(store: &Store, url: &str, token: &str) -> CmdResult<()> {
    store.kv_set(SYNC_URL, url.trim()).await.map_err(err)?;
    store.kv_set(SYNC_TOKEN, token.trim()).await.map_err(err)
}

/// Ev sunucusuyla bir senkron turu. Sunucu ayarlı değilse hiçbir şey yapmaz.
pub async fn sync_now(store: &Store) -> CmdResult<Option<(u32, u32)>> {
    let Some(url) = store
        .kv_get(SYNC_URL)
        .await
        .map_err(err)?
        .filter(|u| !u.is_empty())
    else {
        return Ok(None);
    };
    let token = store
        .kv_get(SYNC_TOKEN)
        .await
        .map_err(err)?
        .filter(|t| !t.is_empty());
    let http = notai_sync::Http::new(url, token).map_err(err)?;
    let r = notai_sync::sync_once(store, &http).await.map_err(err)?;
    Ok(Some((r.pushed, r.pulled_applied)))
}

// ======================= v0.2–v0.4 =======================

use chrono::{Days, Duration, NaiveDate};
use notai_api::{
    AskView, AttachmentView, CalendarView, FactView, RemindersView, SettingsView, TaskEditRequest,
    TopicEditRequest, TrashItemView, WeeklyReviewView,
};
use notai_core::ProfileFactId;
use notai_core::agenda::{agenda, week_start, weekly_review};
use notai_core::profile::ProfileFact;
use notai_core::reminders::{Snooze, group};
use notai_core::task::NewTask;
use notai_core::text::{FactText, TaskTitle, TopicName};
use notai_core::topic::ParaKind;
use notai_store::{AttachmentKind, NewAttachment, NewTopic};
use serde::Deserialize;

use crate::remote::Remote;

#[derive(Debug, Deserialize)]
pub struct NewTaskReq {
    pub title: String,
    pub due_date: Option<String>,
    pub topic_id: Option<String>,
}

pub async fn create_task(store: &Store, req: NewTaskReq, now: DateTime<Utc>) -> CmdResult<()> {
    let mut n = NewTask::titled(TaskTitle::new(&req.title).map_err(err)?);
    n.topic = req
        .topic_id
        .as_deref()
        .filter(|t| !t.is_empty())
        .map(id::<TopicId>)
        .transpose()?;
    n.due = req
        .due_date
        .as_deref()
        .filter(|d| !d.is_empty())
        .map(|d| {
            NaiveDate::parse_from_str(d, "%Y-%m-%d").map_err(|_| format!("geçersiz tarih: {d}"))
        })
        .transpose()?
        .map(notai_core::Due::on);
    store.create_task(n, now).await.map(|_| ()).map_err(err)
}

pub async fn edit_task(
    store: &Store,
    task: &str,
    req: &TaskEditRequest,
    now: DateTime<Utc>,
) -> CmdResult<()> {
    let mut t = store.task(id::<TaskId>(task)?).await.map_err(err)?;
    convert::apply_task_edit(&mut t, req)?;
    if let Some(topic) = t.topic {
        store.topic(topic).await.map_err(err)?;
    }
    store.save_task(&t, now).await.map_err(err)
}

pub async fn create_topic(
    store: &Store,
    name: &str,
    kind: &str,
    now: DateTime<Utc>,
) -> CmdResult<()> {
    let kind = ParaKind::from_parts(kind, None).map_err(err)?;
    store
        .create_topic(NewTopic::new(TopicName::new(name).map_err(err)?, kind), now)
        .await
        .map(|_| ())
        .map_err(err)
}

pub async fn edit_topic(
    store: &Store,
    topic: &str,
    req: &TopicEditRequest,
    now: DateTime<Utc>,
) -> CmdResult<()> {
    let mut t = store.topic(id::<TopicId>(topic)?).await.map_err(err)?;
    convert::apply_topic_edit(&mut t, req)?;
    store.update_topic(&t, now).await.map_err(err)
}

pub async fn move_entry(
    store: &Store,
    entry: &str,
    topic: Option<&str>,
    now: DateTime<Utc>,
) -> CmdResult<()> {
    let topic = topic
        .filter(|t| !t.is_empty())
        .map(id::<TopicId>)
        .transpose()?;
    store
        .move_entry(id::<EntryId>(entry)?, topic, now)
        .await
        .map(|_| ())
        .map_err(err)
}

pub async fn reminders(store: &Store, now: DateTime<Utc>, tz: Tz) -> CmdResult<RemindersView> {
    let tasks: Vec<_> = store
        .reminder_tasks()
        .await
        .map_err(err)?
        .into_iter()
        .map(|(t, _)| t)
        .collect();
    Ok(convert::reminders(group(&tasks, now, tz), now, tz))
}

pub async fn snooze(
    store: &Store,
    task: &str,
    option: &str,
    now: DateTime<Utc>,
    tz: Tz,
) -> CmdResult<()> {
    let s = Snooze::parse(option).map_err(err)?;
    store
        .snooze_reminder(id::<TaskId>(task)?, s, now, tz)
        .await
        .map(|_| ())
        .map_err(err)
}

pub async fn calendar(
    store: &Store,
    from: Option<&str>,
    days: u32,
    now: DateTime<Utc>,
    tz: Tz,
) -> CmdResult<CalendarView> {
    let today = now.with_timezone(&tz).date_naive();
    let from = match from.filter(|f| !f.is_empty()) {
        Some(f) => {
            NaiveDate::parse_from_str(f, "%Y-%m-%d").map_err(|_| format!("geçersiz tarih: {f}"))?
        }
        None => week_start(today),
    };
    let days = days.clamp(1, 42);
    let tasks = store
        .tasks_between(from, from + Days::new(u64::from(days - 1)))
        .await
        .map_err(err)?;
    Ok(convert::calendar(
        agenda(&tasks, from, days, today),
        &tasks,
        today,
    ))
}

pub async fn weekly_review_view(
    store: &Store,
    now: DateTime<Utc>,
    tz: Tz,
) -> CmdResult<WeeklyReviewView> {
    let activity = store.topic_activity().await.map_err(err)?;
    let open = store.open_tasks().await.map_err(err)?;
    let done = store
        .done_since(now - Duration::days(7))
        .await
        .map_err(err)?;
    let unprocessed = store.unprocessed_count().await.map_err(err)?;
    let r = weekly_review(unprocessed, &activity, &open, done, now, tz);
    Ok(convert::weekly_review(
        r,
        &open,
        now.with_timezone(&tz).date_naive(),
    ))
}

/// Ayarlar: otonom mod ve AI ev sunucusunda çalışır; sunucu yoksa yerel varsayılanlar.
pub async fn settings(store: &Store) -> CmdResult<SettingsView> {
    match Remote::from_store(store).await? {
        Some(r) => r.get("/settings").await,
        None => Ok(SettingsView {
            autonomous: false,
            threshold: 0.9,
            ai_provider: "Ev sunucusu ayarlı değil".into(),
            voice_enabled: false,
            semantic_search: false,
        }),
    }
}

pub async fn set_settings(
    store: &Store,
    autonomous: Option<bool>,
    threshold: Option<f64>,
) -> CmdResult<()> {
    let r = Remote::require(store).await?;
    r.post::<serde_json::Value>(
        "/settings",
        &serde_json::json!({ "autonomous": autonomous, "threshold": threshold }),
    )
    .await
    .map(|_| ())
}

/// Ara/Sor: sunucu varsa AI cevaplı (sunucu senkronla aynı verilere sahip), yoksa yerel
/// tam metin aramasının sonuçları.
pub async fn ask(store: &Store, question: &str, tz: Tz) -> CmdResult<AskView> {
    if question.trim().is_empty() {
        return Err("soru boş olamaz".into());
    }
    if let Some(r) = Remote::from_store(store).await? {
        // Önce yerel değişiklikleri gönder ki sunucu en güncel notlarla cevaplasın.
        let _ = sync_now(store).await;
        return r
            .post("/ask", &serde_json::json!({ "question": question }))
            .await;
    }
    let found = store.search(question, 12).await.map_err(err)?;
    Ok(AskView {
        answer: None,
        not_found: found.is_empty(),
        sources: found
            .iter()
            .map(|e| convert::source(e.id.to_string(), e.text.as_str(), e.captured_at, None, tz))
            .collect(),
    })
}

/// Sesli yakalama: ses ev sunucusunda yazıya çevrilir, kayıt senkronla gelir.
pub async fn voice(store: &Store, data: Vec<u8>, mime: &str, topic: Option<&str>) -> CmdResult<()> {
    let r = Remote::require(store).await?;
    let path = topic
        .filter(|t| !t.is_empty())
        .map_or_else(|| "/voice".to_owned(), |t| format!("/voice?topic_id={t}"));
    r.post_bytes::<serde_json::Value>(&path, mime, data).await?;
    sync_now(store).await.map(|_| ())
}

pub async fn undo(store: &Store, suggestion: &str, now: DateTime<Utc>) -> CmdResult<()> {
    store
        .undo_decision(id::<SuggestionId>(suggestion)?, now)
        .await
        .map(|_| ())
        .map_err(err)
}

pub async fn trash(store: &Store, now: DateTime<Utc>) -> CmdResult<Vec<TrashItemView>> {
    Ok(store
        .trash()
        .await
        .map_err(err)?
        .iter()
        .map(|e| convert::trash_item(e, now))
        .collect())
}

pub async fn purge(store: &Store, entry: &str, now: DateTime<Utc>) -> CmdResult<()> {
    store
        .purge_entry(id::<EntryId>(entry)?, now)
        .await
        .map_err(err)
}

pub async fn facts(store: &Store) -> CmdResult<Vec<FactView>> {
    Ok(store
        .facts()
        .await
        .map_err(err)?
        .iter()
        .map(convert::fact)
        .collect())
}

pub async fn add_fact(store: &Store, text: &str, now: DateTime<Utc>) -> CmdResult<()> {
    let f = ProfileFact::confirmed(FactText::new(text).map_err(err)?, None);
    store.add_fact(&f, now).await.map_err(err)
}

pub async fn fact_action(
    store: &Store,
    fact: &str,
    action: &str,
    now: DateTime<Utc>,
) -> CmdResult<()> {
    let f: ProfileFactId = id(fact)?;
    match action {
        "confirm" => store.confirm_fact(f, now).await.map_err(err),
        "delete" => store.delete_fact(f, now).await.map_err(err),
        other => Err(format!("bilinmeyen eylem: {other}")),
    }
}

pub async fn attachments(store: &Store, entry: &str) -> CmdResult<Vec<AttachmentView>> {
    Ok(store
        .attachments_for(id::<EntryId>(entry)?)
        .await
        .map_err(err)?
        .into_iter()
        .map(|m| AttachmentView {
            id: m.id,
            kind: m.kind.name().to_owned(),
            name: m.name,
            mime: m.mime,
            size: m.size,
            value: m.value,
        })
        .collect())
}

#[allow(clippy::too_many_arguments)]
pub async fn upload_attachment(
    store: &Store,
    entry: &str,
    kind: &str,
    name: String,
    mime: String,
    data: Vec<u8>,
    value: Option<String>,
    now: DateTime<Utc>,
) -> CmdResult<()> {
    let kind = AttachmentKind::parse(kind).map_err(|_| format!("bilinmeyen ek türü: {kind}"))?;
    store
        .add_attachment(
            NewAttachment {
                entry: id::<EntryId>(entry)?,
                kind,
                name,
                mime,
                data: (!data.is_empty()).then_some(data),
                value,
            },
            now,
        )
        .await
        .map(|_| ())
        .map_err(err)
}
