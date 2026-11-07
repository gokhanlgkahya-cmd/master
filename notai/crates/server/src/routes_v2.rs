//! v0.2–v0.4 uçları: hatırlatmalar, takvim, görev/konu düzenleme, haftalık gözden geçirme,
//! Ara/Sor, sesli yakalama, ekler, otonom mod ayarı + geri alma, çöp kutusu, profil bilgileri.

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{Days, Duration, NaiveDate, NaiveTime};
use notai_ai::ask::{AskContext, AskSource, ask};
use notai_ai::media::top_k;
use notai_api::{
    AskView, AttachmentView, CalendarView, FactView, RemindersView, SettingsView, TaskEditRequest,
    TopicEditRequest, TrashItemView, WeeklyReviewView, convert,
};
use notai_core::agenda::{agenda, week_start, weekly_review};
use notai_core::entry::Source;
use notai_core::reminders::{Snooze, group};
use notai_core::task::{Due, MonthDay, Priority, Recurrence, Reminder};
use notai_core::text::{EntryText, TaskTitle, TopicName};
use notai_core::topic::ParaKind;
use notai_core::{Entry, EntryId, ProfileFactId, SuggestionId, TaskId, TopicId};
use notai_store::{AttachmentKind, NewAttachment, NewEntry, Store};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::config::AiProvider;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Yükleme sınırı: 10 MB ek + başlık payı.
const UPLOAD_LIMIT: usize = 12 * 1024 * 1024;

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/reminders", get(reminders))
        .route("/tasks/{id}/snooze", post(snooze))
        .route("/tasks/{id}/edit", post(edit_task))
        .route("/topics/{id}/edit", post(edit_topic))
        .route("/calendar", get(calendar))
        .route("/weekly-review", get(review))
        .route("/ask", post(ask_notes))
        .route(
            "/voice",
            post(voice).layer(DefaultBodyLimit::max(UPLOAD_LIMIT)),
        )
        .route(
            "/entries/{id}/attachments",
            get(list_attachments)
                .post(upload_attachment)
                .layer(DefaultBodyLimit::max(UPLOAD_LIMIT)),
        )
        .route("/attachments/{id}", get(download_attachment))
        .route("/suggestions/{id}/undo", post(undo))
        .route("/settings", get(get_settings).post(set_settings))
        .route("/trash", get(trash))
        .route("/entries/{id}/purge", post(purge))
        .route("/facts/{id}/{action}", post(fact_action))
}

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError::BadRequest(msg.into())
}

fn id<T: std::str::FromStr>(s: &str) -> ApiResult<T> {
    s.parse().map_err(|_| bad(format!("geçersiz kimlik: {s}")))
}

fn today(st: &AppState) -> NaiveDate {
    st.now().with_timezone(&st.cfg.tz).date_naive()
}

// ---------- Ayarlar (otonom mod) ----------

pub const AUTONOMOUS_KEY: &str = "autonomous";
pub const THRESHOLD_KEY: &str = "autonomous_threshold";
pub const DEFAULT_THRESHOLD: f64 = 0.9;

/// Otonom mod ayarı: (açık mı, eşik).
pub async fn autonomy(store: &Store) -> Result<(bool, f64), notai_store::StoreError> {
    let on = store.kv_get(AUTONOMOUS_KEY).await?.as_deref() == Some("1");
    let th = store
        .kv_get(THRESHOLD_KEY)
        .await?
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|t| (0.5..=1.0).contains(t))
        .unwrap_or(DEFAULT_THRESHOLD);
    Ok((on, th))
}

async fn settings_view(st: &AppState) -> ApiResult<SettingsView> {
    let (autonomous, threshold) = autonomy(&st.store).await?;
    Ok(SettingsView {
        autonomous,
        threshold,
        ai_provider: match st.cfg.ai {
            AiProvider::None => "Kapalı",
            AiProvider::Local { .. } => "Yerel model",
            AiProvider::Anthropic { .. } => "Claude API",
        }
        .to_owned(),
        voice_enabled: st.stt.is_some(),
        semantic_search: st.embedder.is_some(),
    })
}

async fn get_settings(State(st): State<AppState>) -> ApiResult<Json<SettingsView>> {
    Ok(Json(settings_view(&st).await?))
}

#[derive(Debug, Deserialize)]
struct SettingsBody {
    autonomous: Option<bool>,
    threshold: Option<f64>,
}

async fn set_settings(
    State(st): State<AppState>,
    Json(b): Json<SettingsBody>,
) -> ApiResult<Json<SettingsView>> {
    if let Some(t) = b.threshold {
        if !(0.5..=1.0).contains(&t) {
            return Err(bad("eşik 0,5 ile 1 arasında olmalı"));
        }
        st.store.kv_set(THRESHOLD_KEY, &t.to_string()).await?;
    }
    if let Some(on) = b.autonomous {
        st.store
            .kv_set(AUTONOMOUS_KEY, if on { "1" } else { "0" })
            .await?;
    }
    Ok(Json(settings_view(&st).await?))
}

async fn undo(State(st): State<AppState>, Path(sid): Path<String>) -> ApiResult<Json<Entry>> {
    Ok(Json(
        st.store
            .undo_decision(id::<SuggestionId>(&sid)?, st.now())
            .await?,
    ))
}

// ---------- Hatırlatmalar ----------

async fn reminders(State(st): State<AppState>) -> ApiResult<Json<RemindersView>> {
    let now = st.now();
    let tasks: Vec<_> = st
        .store
        .reminder_tasks()
        .await?
        .into_iter()
        .map(|(t, _)| t)
        .collect();
    Ok(Json(convert::reminders(
        group(&tasks, now, st.cfg.tz),
        now,
        st.cfg.tz,
    )))
}

#[derive(Debug, Deserialize)]
struct SnoozeBody {
    /// minutes10 | hour1 | tomorrow
    option: String,
}

async fn snooze(
    State(st): State<AppState>,
    Path(tid): Path<String>,
    Json(b): Json<SnoozeBody>,
) -> ApiResult<Json<Value>> {
    let task = st
        .store
        .snooze_reminder(
            id::<TaskId>(&tid)?,
            Snooze::parse(&b.option)?,
            st.now(),
            st.cfg.tz,
        )
        .await?;
    Ok(Json(json!({ "task": task })))
}

// ---------- Görev ve konu düzenleme ----------

/// Düzenleme isteğini göreve uygular (sunucu ve uygulama kabuğu ortak kullanır).
pub fn apply_task_edit(task: &mut notai_core::Task, e: &TaskEditRequest) -> Result<(), String> {
    if let Some(t) = &e.title {
        task.title = TaskTitle::new(t).map_err(|x| x.to_string())?;
    }
    if let Some(p) = &e.priority {
        task.priority = Priority::parse(p).map_err(|x| x.to_string())?;
    }
    if let Some(d) = &e.due_date {
        task.due = if d.is_empty() {
            None
        } else {
            let date = NaiveDate::parse_from_str(d, "%Y-%m-%d")
                .map_err(|_| format!("geçersiz tarih: {d}"))?;
            Some(Due {
                date,
                time: task.due.and_then(|x| x.time),
            })
        };
    }
    if let Some(t) = &e.due_time {
        let time = if t.is_empty() {
            None
        } else {
            Some(NaiveTime::parse_from_str(t, "%H:%M").map_err(|_| format!("geçersiz saat: {t}"))?)
        };
        match (&mut task.due, time) {
            (Some(d), time) => d.time = time,
            (None, None) => {}
            (None, Some(_)) => return Err("saat için tarih gerekli".into()),
        }
    }
    if let Some(r) = &e.repeat {
        let on = task.due.map(|d| chrono::Datelike::weekday(&d.date));
        task.recurrence = match (r.as_str(), on) {
            ("none" | "", _) => None,
            ("day", _) => Some(Recurrence::Day),
            ("weekday", _) => Some(Recurrence::Weekday),
            ("week", Some(on)) => Some(Recurrence::Week { on }),
            ("month", Some(_)) => {
                let day = task.due.map_or(1, |d| {
                    u8::try_from(chrono::Datelike::day(&d.date))
                        .unwrap_or(1)
                        .min(28)
                });
                Some(Recurrence::Month {
                    day: MonthDay::new(day).map_err(|x| x.to_string())?,
                })
            }
            (_, None) => return Err("tekrar için tarih gerekli".into()),
            (other, _) => return Err(format!("bilinmeyen tekrar: {other}")),
        };
    }
    if let Some(m) = e.remind_before_minutes {
        task.reminder = (m > 0).then_some(Reminder::Before { minutes: m });
    }
    if let Some(t) = &e.topic_id {
        task.topic = if t.is_empty() {
            None
        } else {
            Some(t.parse::<TopicId>().map_err(|x| x.to_string())?)
        };
    }
    // Tutarlılık: tarihsiz göreve tekrar ya da "önce" hatırlatması olamaz.
    notai_core::Task::from_parts(
        task.id,
        task.entry,
        task.title.clone(),
        task.topic,
        task.due,
        task.priority,
        task.recurrence,
        task.reminder,
        task.state(),
    )
    .map(|_| ())
    .map_err(|x| x.to_string())
}

async fn edit_task(
    State(st): State<AppState>,
    Path(tid): Path<String>,
    Json(e): Json<TaskEditRequest>,
) -> ApiResult<Json<notai_api::TaskRowView>> {
    let mut task = st.store.task(id::<TaskId>(&tid)?).await?;
    apply_task_edit(&mut task, &e).map_err(ApiError::BadRequest)?;
    if let Some(topic) = task.topic {
        st.store.topic(topic).await?;
    }
    st.store.save_task(&task, st.now()).await?;
    Ok(Json(convert::task_row(&task, today(&st))))
}

/// Konu düzenleme isteğini uygular.
pub fn apply_topic_edit(topic: &mut notai_core::Topic, e: &TopicEditRequest) -> Result<(), String> {
    if let Some(n) = &e.name {
        topic.name = TopicName::new(n).map_err(|x| x.to_string())?;
    }
    if e.kind.is_some() || e.due_date.is_some() {
        let kind = e
            .kind
            .clone()
            .unwrap_or_else(|| topic.kind.name().to_owned());
        let due = match &e.due_date {
            Some(d) if d.is_empty() => None,
            Some(d) => Some(
                NaiveDate::parse_from_str(d, "%Y-%m-%d")
                    .map_err(|_| format!("geçersiz tarih: {d}"))?,
            ),
            None => topic.kind.due(),
        };
        topic.kind = ParaKind::from_parts(&kind, due).map_err(|x| x.to_string())?;
    }
    if let Some(i) = &e.icon {
        topic.icon = (!i.is_empty()).then(|| i.clone());
    }
    if let Some(p) = e.pinned {
        topic.pinned = p;
    }
    Ok(())
}

async fn edit_topic(
    State(st): State<AppState>,
    Path(tid): Path<String>,
    Json(e): Json<TopicEditRequest>,
) -> ApiResult<Json<notai_core::Topic>> {
    let mut topic = st.store.topic(id::<TopicId>(&tid)?).await?;
    apply_topic_edit(&mut topic, &e).map_err(ApiError::BadRequest)?;
    st.store.update_topic(&topic, st.now()).await?;
    Ok(Json(topic))
}

// ---------- Takvim ve haftalık gözden geçirme ----------

#[derive(Debug, Deserialize)]
struct CalendarQuery {
    from: Option<NaiveDate>,
    days: Option<u32>,
}

async fn calendar(
    State(st): State<AppState>,
    Query(q): Query<CalendarQuery>,
) -> ApiResult<Json<CalendarView>> {
    let today = today(&st);
    let from = q.from.unwrap_or_else(|| week_start(today));
    let days = q.days.unwrap_or(7).clamp(1, 42);
    let to = from + Days::new(u64::from(days - 1));
    let tasks = st.store.tasks_between(from, to).await?;
    Ok(Json(convert::calendar(
        agenda(&tasks, from, days, today),
        &tasks,
        today,
    )))
}

async fn review(State(st): State<AppState>) -> ApiResult<Json<WeeklyReviewView>> {
    let now = st.now();
    let activity = st.store.topic_activity().await?;
    let open = st.store.open_tasks().await?;
    let done = st.store.done_since(now - Duration::days(7)).await?;
    let unprocessed = st.store.unprocessed_count().await?;
    let r = weekly_review(unprocessed, &activity, &open, done, now, st.cfg.tz);
    Ok(Json(convert::weekly_review(r, &open, today(&st))))
}

// ---------- Ara/Sor ----------

#[derive(Debug, Deserialize)]
struct AskBody {
    question: String,
}

/// Kaynak sayısı üst sınırı.
const ASK_SOURCES: usize = 12;

async fn ask_notes(State(st): State<AppState>, Json(b): Json<AskBody>) -> ApiResult<Json<AskView>> {
    let q = b.question.trim();
    if q.is_empty() {
        return Err(bad("soru boş olamaz"));
    }
    // Karma arama: tam metin + (varsa) anlamsal benzerlik.
    let mut found: Vec<Entry> = st.store.search(q, 8).await?;
    if let Some(emb) = st.embedder.as_deref()
        && let Ok(mut v) = emb.embed(&[q.to_owned()]).await
        && let Some(qv) = v.pop()
    {
        let all = st.store.embeddings(emb.model()).await?;
        for (eid, _) in top_k(&qv, &all, 8, 0.3) {
            if !found.iter().any(|e| e.id == eid)
                && let Ok(e) = st.store.entry(eid).await
            {
                found.push(e);
            }
        }
    }
    found.truncate(ASK_SOURCES);
    let topic_names = st
        .store
        .topic_list(notai_core::topic::KindFilter::All)
        .await?;
    let topic_of = |e: &Entry| {
        e.topic()
            .and_then(|t| topic_names.iter().find(|a| a.topic.id == t))
            .map(|a| a.topic.name.as_str().to_owned())
    };
    let sources_view = |ids: &[EntryId]| -> Vec<notai_api::SourceView> {
        ids.iter()
            .filter_map(|i| found.iter().find(|e| e.id == *i))
            .map(|e| {
                convert::source(
                    e.id.to_string(),
                    e.text.as_str(),
                    e.captured_at,
                    topic_of(e),
                    st.cfg.tz,
                )
            })
            .collect()
    };
    let all_ids: Vec<EntryId> = found.iter().map(|e| e.id).collect();
    let Some(llm) = st.llm.as_deref() else {
        return Ok(Json(AskView {
            answer: None,
            not_found: found.is_empty(),
            sources: sources_view(&all_ids),
        }));
    };
    let ctx = AskContext {
        question: q.to_owned(),
        sources: found
            .iter()
            .map(|e| AskSource {
                id: e.id,
                captured_at: e.captured_at,
                text: e.text.as_str().to_owned(),
                topic: topic_of(e),
            })
            .collect(),
        tz: st.cfg.tz,
        user_name: st.cfg.user_name.clone(),
    };
    match ask(llm, &ctx).await {
        Ok(a) => Ok(Json(AskView {
            answer: Some(a.text),
            not_found: a.not_found,
            sources: sources_view(&a.sources),
        })),
        Err(e) => {
            tracing::warn!(error = %e, "Ara/Sor cevabı üretilemedi");
            Ok(Json(AskView {
                answer: None,
                not_found: found.is_empty(),
                sources: sources_view(&all_ids),
            }))
        }
    }
}

// ---------- Sesli yakalama ----------

#[derive(Debug, Deserialize)]
struct VoiceQuery {
    topic_id: Option<TopicId>,
}

fn content_type(h: &HeaderMap) -> String {
    h.get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_owned()
}

/// Ses kaydını yazıya çevirip kayıt olarak yakalar; ses dosyası da ek olarak saklanır.
async fn voice(
    State(st): State<AppState>,
    Query(q): Query<VoiceQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let Some(stt) = st.stt.as_deref() else {
        return Err(bad(
            "sesli yakalama için ses servisi tanımlı değil (NOTAI_STT_MODEL)",
        ));
    };
    if body.is_empty() {
        return Err(bad("ses kaydı boş"));
    }
    let mime = content_type(&headers);
    let text = stt
        .transcribe(&body, &mime)
        .await
        .map_err(|e| bad(format!("ses yazıya çevrilemedi: {e}")))?;
    let captured = st
        .store
        .capture(NewEntry {
            text: EntryText::new(&text)?,
            source: Source::Voice,
            topic: q.topic_id,
            captured_at: st.now(),
            dedupe_key: None,
        })
        .await?;
    let entry = captured.entry().clone();
    st.store
        .add_attachment(
            NewAttachment {
                entry: entry.id,
                kind: AttachmentKind::Audio,
                name: "ses kaydı".into(),
                mime,
                data: Some(body.to_vec()),
                value: None,
            },
            st.now(),
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "entry": entry, "text": text })),
    ))
}

// ---------- Ekler ----------

fn attachment_view(m: notai_store::AttachmentMeta) -> AttachmentView {
    AttachmentView {
        id: m.id,
        kind: m.kind.name().to_owned(),
        name: m.name,
        mime: m.mime,
        size: m.size,
        value: m.value,
    }
}

async fn list_attachments(
    State(st): State<AppState>,
    Path(eid): Path<String>,
) -> ApiResult<Json<Vec<AttachmentView>>> {
    let list = st.store.attachments_for(id::<EntryId>(&eid)?).await?;
    Ok(Json(list.into_iter().map(attachment_view).collect()))
}

#[derive(Debug, Deserialize)]
struct UploadQuery {
    /// photo | document | location | contact | audio
    kind: String,
    name: String,
    /// Konum/kişi gibi metin ekler için (gövde boş olabilir).
    value: Option<String>,
}

async fn upload_attachment(
    State(st): State<AppState>,
    Path(eid): Path<String>,
    Query(q): Query<UploadQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<(StatusCode, Json<AttachmentView>)> {
    let kind = AttachmentKind::parse(&q.kind)
        .map_err(|_| bad(format!("bilinmeyen ek türü: {}", q.kind)))?;
    let meta = st
        .store
        .add_attachment(
            NewAttachment {
                entry: id::<EntryId>(&eid)?,
                kind,
                name: q.name,
                mime: content_type(&headers),
                data: (!body.is_empty()).then(|| body.to_vec()),
                value: q.value,
            },
            st.now(),
        )
        .await?;
    Ok((StatusCode::CREATED, Json(attachment_view(meta))))
}

async fn download_attachment(
    State(st): State<AppState>,
    Path(aid): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let (mime, data) = st.store.attachment_data(&aid).await?;
    Ok(([(header::CONTENT_TYPE, mime)], data))
}

// ---------- Çöp kutusu ve profil ----------

async fn trash(State(st): State<AppState>) -> ApiResult<Json<Vec<TrashItemView>>> {
    let now = st.now();
    Ok(Json(
        st.store
            .trash()
            .await?
            .iter()
            .map(|e| convert::trash_item(e, now))
            .collect(),
    ))
}

async fn purge(State(st): State<AppState>, Path(eid): Path<String>) -> ApiResult<StatusCode> {
    st.store.purge_entry(id::<EntryId>(&eid)?, st.now()).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn fact_action(
    State(st): State<AppState>,
    Path((fid, action)): Path<(String, String)>,
) -> ApiResult<Json<Vec<FactView>>> {
    let f: ProfileFactId = id(&fid)?;
    match action.as_str() {
        "confirm" => st.store.confirm_fact(f, st.now()).await?,
        "delete" => st.store.delete_fact(f, st.now()).await?,
        other => return Err(bad(format!("bilinmeyen eylem: {other}"))),
    }
    let facts = st.store.facts().await?;
    Ok(Json(
        facts
            .into_iter()
            .map(|f| FactView {
                id: f.id.to_string(),
                text: f.text.as_str().to_owned(),
                confirmed: f.status == notai_core::profile::FactStatus::Confirmed,
            })
            .collect(),
    ))
}
