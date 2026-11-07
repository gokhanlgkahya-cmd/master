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

const SYNC_URL: &str = "sync_server_url";
const SYNC_TOKEN: &str = "sync_token";

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
