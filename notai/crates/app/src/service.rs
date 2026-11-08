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

/// Görev oluşturur; yeni görevin kimliğini döndürür.
pub async fn create_task(store: &Store, req: NewTaskReq, now: DateTime<Utc>) -> CmdResult<String> {
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
    store
        .create_task(n, now)
        .await
        .map(|t| t.id.to_string())
        .map_err(err)
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
    parent: Option<&str>,
    now: DateTime<Utc>,
) -> CmdResult<()> {
    let kind = ParaKind::from_parts(kind, None).map_err(err)?;
    let mut new = NewTopic::new(TopicName::new(name).map_err(err)?, kind);
    new.parent = parent
        .filter(|p| !p.is_empty())
        .map(id::<TopicId>)
        .transpose()?;
    store.create_topic(new, now).await.map(|_| ()).map_err(err)
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

// ---------------- AI sorusu ve bildirimler (2026-11-07) ----------------

/// AI sorusunu cevaplar; kayıt senkronla ev sunucusuna gider ve orada cevapla yeniden
/// sınıflandırılır.
pub async fn answer_question(
    store: &Store,
    suggestion: &str,
    text: &str,
    now: DateTime<Utc>,
) -> CmdResult<()> {
    let text = notai_core::text::AnswerText::new(text).map_err(err)?;
    store
        .answer_question(id::<SuggestionId>(suggestion)?, text, now)
        .await
        .map(|_| ())
        .map_err(err)
}

pub async fn notice(store: &Store, now: DateTime<Utc>) -> CmdResult<Option<notai_api::NoticeView>> {
    store
        .notice(now)
        .await
        .map(|n| n.as_ref().map(convert::notice))
        .map_err(err)
}

pub async fn clear_notice(store: &Store) -> CmdResult<()> {
    store.clear_notice().await.map_err(err)
}

const QUESTIONS_NOTIFIED_KEY: &str = "questions_notified_until";

async fn topic_named(store: &Store, topic: Option<TopicId>) -> Option<(TopicId, String)> {
    let id = topic?;
    store
        .topic(id)
        .await
        .ok()
        .map(|t| (t.id, t.name.as_str().to_owned()))
}

/// Bu turda gösterilecek bildirimler: zamanı gelen hatırlatmalar (konu adıyla; her biri bir
/// kez) ve senkronla gelen yeni AI soruları. Sonuncusu "son bildirim" olarak saklanır.
pub async fn due_notices(
    store: &Store,
    now: DateTime<Utc>,
    tz: Tz,
) -> CmdResult<Vec<notai_core::notice::Notice>> {
    use notai_core::notice::Notice;
    let mut out = Vec::new();
    let tasks = store.reminder_tasks().await.map_err(err)?;
    for (task, at) in notai_core::reminders::due_now(&tasks, now, tz) {
        store.mark_reminder_fired(task.id, at).await.map_err(err)?;
        let topic = topic_named(store, task.topic).await;
        out.push(Notice::reminder(
            task.title.as_str(),
            topic,
            task.entry,
            now,
        ));
    }
    // İlk açılışta eski soruları bildirme: sınırı şimdiye çek.
    let since = store
        .kv_get(QUESTIONS_NOTIFIED_KEY)
        .await
        .map_err(err)?
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| d.with_timezone(&Utc));
    let mut until = since.unwrap_or(now);
    if let Some(since) = since {
        for q in store.pending_questions_since(since).await.map_err(err)? {
            if let Some((text, None)) = q.suggestion.question_and_answer() {
                let topic = topic_named(store, q.topic).await;
                out.push(Notice::question(
                    text.as_str(),
                    topic,
                    q.suggestion.entry,
                    now,
                ));
            }
            until = until.max(q.created_at);
        }
    }
    store
        .kv_set(
            QUESTIONS_NOTIFIED_KEY,
            &until.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
        )
        .await
        .map_err(err)?;
    // Senkronla gelen NotAI mesajları (ör. fiyat takibi): her biri bir kez bildirilir.
    let since_notai = store
        .kv_get(NOTAI_NOTIFIED_KEY)
        .await
        .map_err(err)?
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| d.with_timezone(&Utc));
    let mut notai_until = since_notai.unwrap_or(now);
    if let Some(since) = since_notai {
        for e in store.notai_entries_since(since).await.map_err(err)? {
            if let Some(t) = e.topic() {
                let name = store
                    .topic(t)
                    .await
                    .map(|x| x.name.as_str().to_owned())
                    .unwrap_or_default();
                let headline = e.text.preview(80);
                out.push(notai_core::notice::Notice::watch(
                    &headline,
                    (t, name),
                    e.id,
                    now,
                ));
            }
            notai_until = notai_until.max(e.captured_at);
        }
    }
    store
        .kv_set(
            NOTAI_NOTIFIED_KEY,
            &notai_until.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
        )
        .await
        .map_err(err)?;
    if let Some(last) = out.last() {
        store.set_notice(last).await.map_err(err)?;
    }
    Ok(out)
}

const NOTAI_NOTIFIED_KEY: &str = "notai_notified_until";

/// Konuya fiyat/ürün takibi ekler, günceller ya da (`query` boşsa) kaldırır.
pub async fn set_watch(
    store: &Store,
    topic: &str,
    req: &notai_api::WatchRequest,
    now: DateTime<Utc>,
) -> CmdResult<()> {
    let mut t = store.topic(id::<TopicId>(topic)?).await.map_err(err)?;
    t.watch = convert::watch_config(req)?;
    store.update_topic(&t, now).await.map_err(err)
}

// ---------------- Varlıklar (2026-11-08) ----------------

pub async fn assets(store: &Store, now: DateTime<Utc>, tz: Tz) -> CmdResult<notai_api::AssetsView> {
    let today = now.with_timezone(&tz).date_naive();
    let topics = store.asset_topics().await.map_err(err)?;
    let past = store
        .assets_total_on(today - chrono::Duration::days(30))
        .await
        .map_err(err)?;
    Ok(convert::assets_view(&topics, past))
}

/// Yeni varlık: "Varlıklar" ana konusunun altında yeni konu; kimliği döner.
pub async fn create_asset(
    store: &Store,
    req: &notai_api::AssetRequest,
    now: DateTime<Utc>,
    tz: Tz,
) -> CmdResult<String> {
    let info = convert::asset_info(req, now.with_timezone(&tz).date_naive())?;
    let root = store.assets_root(now).await.map_err(err)?;
    let mut n = NewTopic::new(
        TopicName::new(&req.name).map_err(|e| e.to_string())?,
        ParaKind::Resource,
    );
    n.parent = Some(root.id);
    let mut t = store.create_topic(n, now).await.map_err(err)?;
    t.asset = Some(info);
    store.update_topic(&t, now).await.map_err(err)?;
    Ok(t.id.to_string())
}

/// Konunun varlık bilgisini günceller; `kind` boşsa kaldırır.
pub async fn set_asset(
    store: &Store,
    topic: &str,
    req: &notai_api::AssetRequest,
    now: DateTime<Utc>,
    tz: Tz,
) -> CmdResult<()> {
    let mut t = store.topic(id::<TopicId>(topic)?).await.map_err(err)?;
    t.asset = if req.kind.trim().is_empty() {
        None
    } else {
        Some(convert::asset_info(
            req,
            now.with_timezone(&tz).date_naive(),
        )?)
    };
    store.update_topic(&t, now).await.map_err(err)
}

// ---------------- Harcamalar (2026-11-07) ----------------

fn month_range(y: i32, m: u32) -> CmdResult<(NaiveDate, NaiveDate)> {
    let first = NaiveDate::from_ymd_opt(y, m, 1).ok_or("geçersiz ay")?;
    let from = first
        .checked_sub_months(chrono::Months::new(1))
        .ok_or("geçersiz ay")?;
    let to = first
        .checked_add_months(chrono::Months::new(1))
        .and_then(|d| d.pred_opt())
        .ok_or("geçersiz ay")?;
    Ok((from, to))
}

/// Aylık harcama görünümü ("2026-10"; yoksa bu ay).
pub async fn finance(
    store: &Store,
    month: Option<&str>,
    now: DateTime<Utc>,
    tz: Tz,
) -> CmdResult<notai_api::FinanceView> {
    use chrono::Datelike;
    let today = now.with_timezone(&tz).date_naive();
    let (y, m) = match month {
        Some(s) => s
            .split_once('-')
            .and_then(|(y, m)| Some((y.parse().ok()?, m.parse().ok()?)))
            .filter(|(_, m): &(i32, u32)| (1..=12).contains(m))
            .ok_or_else(|| format!("geçersiz ay: {s}"))?,
        None => (today.year(), today.month()),
    };
    let (from, to) = month_range(y, m)?;
    let txs = store.transactions_between(from, to).await.map_err(err)?;
    let pending =
        u32::try_from(store.pending_statements().await.map_err(err)?.len()).unwrap_or(u32::MAX);
    Ok(convert::finance_view(&txs, y, m, pending))
}

/// Ekstre yükler (PDF ya da metin); PDF bu cihazda okunur, hiçbir yere gönderilmez.
pub async fn import_statement(
    store: &Store,
    data: &[u8],
    name: &str,
    account: Option<&str>,
    now: DateTime<Utc>,
) -> CmdResult<notai_api::ImportView> {
    if data.is_empty() {
        return Err("ekstre dosyası boş".into());
    }
    let text = if data.starts_with(b"%PDF") {
        notai_store::pdf_text(data).map_err(err)?
    } else {
        String::from_utf8(data.to_vec()).map_err(|_| "ekstre metni UTF-8 değil".to_owned())?
    };
    let account = account.filter(|a| !a.trim().is_empty()).unwrap_or(name);
    let (_, report) = store
        .import_statement(account, name, &text, now)
        .await
        .map_err(err)?;
    Ok(convert::import_view(&report))
}

/// Hareketin kategori/kapsamını düzeltir (satıcı kuralı öğrenilir).
pub async fn classify_transaction(
    store: &Store,
    id: &str,
    category: &str,
    scope: &str,
    now: DateTime<Utc>,
) -> CmdResult<()> {
    if !notai_core::finance::CATEGORIES
        .iter()
        .any(|(k, _)| *k == category)
    {
        return Err(format!("bilinmeyen kategori: {category}"));
    }
    let scope = notai_core::finance::Scope::parse(scope).map_err(err)?;
    store
        .classify_transaction(id, category, scope, now)
        .await
        .map(|_| ())
        .map_err(err)
}
