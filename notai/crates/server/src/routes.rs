//! HTTP uçları.

use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use notai_core::entry::Source;
use notai_core::journal::{JournalFilter, list_time_label};
use notai_core::profile::ProfileFact;
use notai_core::task::{Due, NewTask, Priority};
use notai_core::text::{EntryText, FactText, TaskTitle, TopicName};
use notai_core::topic::{KindFilter, ParaKind};
use notai_core::{Entry, EntryId, SuggestionId, TaskId, TopicId};
use notai_store::{Captured, NewEntry, NewTopic, TaskView};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use notai_api::{
    JournalView, MorningPlanView, SuggestionView, TaskRowView, TasksView, TopicRowView, TopicsView,
    convert,
};

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/entries", post(capture))
        .route("/entries/{id}", get(get_entry))
        .route("/entries/{id}/move", post(move_entry))
        .route("/entries/{id}/{action}", post(entry_action))
        .route("/journal", get(journal))
        .route("/topics", get(topics).post(create_topic))
        .route("/suggestions/{id}/{decision}", post(decide))
        .route("/tasks", get(tasks).post(create_task))
        .route("/tasks/{id}/{action}", post(task_action))
        .route("/morning-plan", get(morning_plan))
        .route("/search", get(search))
        .route("/facts", get(facts).post(add_fact))
        .route("/sync/push", post(sync_push))
        .route("/sync/pull", get(sync_pull))
        .merge(crate::routes_v2::api_routes())
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_token,
        ));

    let cors = cors_layer(&state.cfg.cors_origins);
    let app = Router::new()
        .route("/health", get(|| async { Json(json!({ "status": "ok" })) }))
        .route("/webhook/whatsapp", post(whatsapp_webhook))
        .nest("/api", api)
        .with_state(state);
    match cors {
        Some(layer) => app.layer(layer),
        None => app,
    }
}

/// Tarayıcıdan doğrudan bağlanma (`?api=http://sunucu:8787`) için CORS. Yalnız yapılandırılan
/// kökenler; belirteç yine zorunlu (çerez kullanılmaz, kimlik bilgisi taşınmaz).
fn cors_layer(origins: &[String]) -> Option<tower_http::cors::CorsLayer> {
    use axum::http::{HeaderValue, Method, header};
    use tower_http::cors::{AllowOrigin, CorsLayer};
    if origins.is_empty() {
        return None;
    }
    let allow = if origins.iter().any(|o| o == "*") {
        AllowOrigin::any()
    } else {
        AllowOrigin::list(
            origins
                .iter()
                .filter_map(|o| HeaderValue::from_str(o).ok())
                .collect::<Vec<_>>(),
        )
    };
    Some(
        CorsLayer::new()
            .allow_origin(allow)
            .allow_methods([Method::GET, Method::POST])
            .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
            .max_age(std::time::Duration::from_secs(600)),
    )
}

/// Zamanlama saldırılarına karşı sabit süreli karşılaştırma.
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

async fn require_token(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if let Some(expected) = state.cfg.api_token.as_deref() {
        let given = req
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .unwrap_or_default();
        if !ct_eq(given.as_bytes(), expected.as_bytes()) {
            return Err(ApiError::Unauthorized);
        }
    }
    Ok(next.run(req).await)
}

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError::BadRequest(msg.into())
}

fn parse_id<T: std::str::FromStr>(s: &str) -> ApiResult<T> {
    s.parse().map_err(|_| bad(format!("geçersiz kimlik: {s}")))
}

// ---------- Kayıtlar ----------

#[derive(Debug, Deserialize)]
struct CaptureBody {
    text: String,
    topic_id: Option<TopicId>,
    #[serde(default)]
    source: Option<Source>,
}

async fn capture(
    State(st): State<AppState>,
    Json(body): Json<CaptureBody>,
) -> ApiResult<(StatusCode, Json<Entry>)> {
    let captured = st
        .store
        .capture(NewEntry {
            text: EntryText::new(&body.text)?,
            source: body.source.unwrap_or(Source::App),
            topic: body.topic_id,
            captured_at: st.now(),
            dedupe_key: None,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(captured.entry().clone())))
}

async fn get_entry(State(st): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let id: EntryId = parse_id(&id)?;
    let entry = st.store.entry(id).await?;
    let suggestions: Vec<SuggestionView> = st
        .store
        .suggestions_for(id)
        .await?
        .iter()
        .map(|s| convert::suggestion(s, st.cfg.tz))
        .collect();
    Ok(Json(json!({ "entry": entry, "suggestions": suggestions })))
}

#[derive(Debug, Deserialize)]
struct MoveBody {
    topic_id: Option<TopicId>,
}

async fn move_entry(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<MoveBody>,
) -> ApiResult<Json<Entry>> {
    let id: EntryId = parse_id(&id)?;
    Ok(Json(
        st.store.move_entry(id, body.topic_id, st.now()).await?,
    ))
}

async fn entry_action(
    State(st): State<AppState>,
    Path((id, action)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    let id: EntryId = parse_id(&id)?;
    let now = st.now();
    let entry = match action.as_str() {
        "archive" => st.store.archive_entry(id, now).await?,
        "unarchive" => st.store.unarchive_entry(id, now).await?,
        "trash" => st.store.trash_entry(id, now).await?,
        "restore" => st.store.restore_entry(id, now).await?,
        "retry-ai" => {
            st.store.retry_ai(id, now).await?;
            st.store.entry(id).await?
        }
        other => return Err(bad(format!("bilinmeyen eylem: {other}"))),
    };
    Ok(Json(json!(entry)))
}

#[derive(Debug, Deserialize)]
struct JournalQuery {
    #[serde(default)]
    filter: Option<String>,
    topic_id: Option<TopicId>,
    before: Option<DateTime<Utc>>,
    limit: Option<u32>,
}

async fn journal(
    State(st): State<AppState>,
    Query(q): Query<JournalQuery>,
) -> ApiResult<Json<JournalView>> {
    let filter = match (q.filter.as_deref().unwrap_or("all"), q.topic_id) {
        ("all", None) => JournalFilter::All,
        ("unprocessed", None) => JournalFilter::Unprocessed,
        ("topic", Some(id)) => JournalFilter::Topic { id },
        _ => return Err(bad("filter: all | unprocessed | topic (+topic_id)")),
    };
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let items = st.store.journal(filter, q.before, limit).await?;
    let unprocessed = st.store.unprocessed_count().await?;
    Ok(Json(convert::journal(
        items,
        limit,
        unprocessed,
        st.now(),
        st.cfg.tz,
    )))
}

// ---------- Konular ----------

#[derive(Debug, Deserialize)]
struct TopicsQuery {
    kind: Option<KindFilter>,
}

async fn topics(
    State(st): State<AppState>,
    Query(q): Query<TopicsQuery>,
) -> ApiResult<Json<TopicsView>> {
    let now = st.now();
    let rows: Vec<TopicRowView> = st
        .store
        .topic_list(q.kind.unwrap_or_default())
        .await?
        .into_iter()
        .map(|a| convert::topic_row(a, now, st.cfg.tz))
        .collect();
    let unprocessed = st.store.unprocessed_count().await?;
    let last = st.store.journal(JournalFilter::All, None, 1).await?;
    let last = last.last();
    Ok(Json(TopicsView {
        topics: rows,
        unprocessed,
        journal_preview: last.map(|i| i.entry.text.preview(80)),
        journal_last_label: last.map(|i| list_time_label(i.entry.captured_at, now, st.cfg.tz)),
    }))
}

#[derive(Debug, Deserialize)]
struct TopicBody {
    name: String,
    kind: String,
    due_date: Option<NaiveDate>,
    icon: Option<String>,
    #[serde(default)]
    pinned: bool,
}

async fn create_topic(
    State(st): State<AppState>,
    Json(b): Json<TopicBody>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let mut new = NewTopic::new(
        TopicName::new(&b.name)?,
        ParaKind::from_parts(&b.kind, b.due_date)?,
    );
    new.icon = b.icon;
    new.pinned = b.pinned;
    let topic = st.store.create_topic(new, st.now()).await?;
    Ok((StatusCode::CREATED, Json(json!(topic))))
}

// ---------- Öneriler ----------

async fn decide(
    State(st): State<AppState>,
    Path((id, decision)): Path<(String, String)>,
) -> ApiResult<Json<notai_api::DecisionView>> {
    let id: SuggestionId = parse_id(&id)?;
    let accept = match decision.as_str() {
        "accept" => true,
        "dismiss" => false,
        other => return Err(bad(format!("accept | dismiss beklenirdi: {other}"))),
    };
    let now = st.now();
    let out = st.store.decide(id, accept, now, st.cfg.tz).await?;
    let today = now.with_timezone(&st.cfg.tz).date_naive();
    Ok(Json(convert::decision(&out, today, st.cfg.tz)))
}

// ---------- Görevler ----------

#[derive(Debug, Deserialize)]
struct TasksQuery {
    view: Option<TaskView>,
}

async fn tasks(
    State(st): State<AppState>,
    Query(q): Query<TasksQuery>,
) -> ApiResult<Json<TasksView>> {
    let today = st.now().with_timezone(&st.cfg.tz).date_naive();
    let mut out = TasksView::default();
    for v in [
        TaskView::Overdue,
        TaskView::Today,
        TaskView::Upcoming,
        TaskView::Undated,
    ] {
        if q.view.is_some_and(|only| only != v) {
            continue;
        }
        let rows: Vec<TaskRowView> = st
            .store
            .tasks(v, today)
            .await?
            .iter()
            .map(|t| convert::task_row(t, today))
            .collect();
        match v {
            TaskView::Overdue => out.overdue = rows,
            TaskView::Today => out.today = rows,
            TaskView::Upcoming => out.upcoming = rows,
            TaskView::Undated => out.undated = rows,
        }
    }
    Ok(Json(out))
}

#[derive(Debug, Deserialize)]
struct TaskBody {
    title: String,
    topic_id: Option<TopicId>,
    due_date: Option<NaiveDate>,
    due_time: Option<NaiveTime>,
    #[serde(default)]
    priority: Priority,
}

async fn create_task(
    State(st): State<AppState>,
    Json(b): Json<TaskBody>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let mut new = NewTask::titled(TaskTitle::new(&b.title)?);
    new.topic = b.topic_id;
    new.priority = b.priority;
    new.due = match (b.due_date, b.due_time) {
        (Some(date), time) => Some(Due { date, time }),
        (None, None) => None,
        (None, Some(_)) => return Err(bad("saat için tarih gerekli")),
    };
    let task = st.store.create_task(new, st.now()).await?;
    Ok((StatusCode::CREATED, Json(json!(task))))
}

async fn task_action(
    State(st): State<AppState>,
    Path((id, action)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    let id: TaskId = parse_id(&id)?;
    let now = st.now();
    match action.as_str() {
        "complete" => {
            let (task, next) = st.store.complete_task(id, now).await?;
            Ok(Json(json!({ "task": task, "next": next })))
        }
        "reopen" => Ok(Json(
            json!({ "task": st.store.reopen_task(id, now).await? }),
        )),
        "cancel" => Ok(Json(
            json!({ "task": st.store.cancel_task(id, now).await? }),
        )),
        other => Err(bad(format!("bilinmeyen eylem: {other}"))),
    }
}

async fn morning_plan(State(st): State<AppState>) -> ApiResult<Json<MorningPlanView>> {
    let today = st.now().with_timezone(&st.cfg.tz).date_naive();
    let plan = st.store.morning_plan(today).await?;
    Ok(Json(convert::morning_plan(plan, &st.cfg.user_name)))
}

// ---------- Arama ve profil ----------

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
}

async fn search(
    State(st): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({ "results": st.store.search(&q.q, 30).await? })))
}

async fn facts(State(st): State<AppState>) -> ApiResult<Json<Vec<notai_api::FactView>>> {
    Ok(Json(
        st.store.facts().await?.iter().map(convert::fact).collect(),
    ))
}

#[derive(Debug, Deserialize)]
struct FactBody {
    text: String,
    topic_id: Option<TopicId>,
}

async fn add_fact(
    State(st): State<AppState>,
    Json(b): Json<FactBody>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let fact = ProfileFact::confirmed(FactText::new(&b.text)?, b.topic_id);
    st.store.add_fact(&fact, st.now()).await?;
    Ok((StatusCode::CREATED, Json(json!(convert::fact(&fact)))))
}

// ---------- Senkron ----------

async fn sync_push(
    State(st): State<AppState>,
    Json(req): Json<notai_sync::PushRequest>,
) -> ApiResult<Json<notai_sync::PushResponse>> {
    Ok(Json(notai_sync::handle_push(&st.store, &req).await?))
}

#[derive(Debug, Deserialize)]
struct PullQuery {
    device: String,
    #[serde(default)]
    after: i64,
    limit: Option<u32>,
}

async fn sync_pull(
    State(st): State<AppState>,
    Query(q): Query<PullQuery>,
) -> ApiResult<Json<notai_sync::PullResponse>> {
    let limit = q.limit.unwrap_or(notai_sync::BATCH);
    Ok(Json(
        notai_sync::handle_pull(&st.store, &q.device, q.after, limit).await?,
    ))
}

// ---------- WhatsApp (MacroDroid) webhook ----------

#[derive(Debug, Deserialize)]
struct WhatsAppBody {
    text: String,
    /// MacroDroid'in bildirim zamanı (yoksa sunucu saati).
    sent_at: Option<DateTime<Utc>>,
}

/// Aynı metin 5 dakikalık pencerede ikinci kez gelirse yineleme sayılır.
fn dedupe_key(text: &EntryText, at: DateTime<Utc>) -> String {
    let bucket = at.timestamp() / 300;
    format!("wa:{bucket}:{}", text.as_str())
}

async fn whatsapp_webhook(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<WhatsAppBody>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let Some(expected) = st.cfg.webhook_secret.as_deref() else {
        return Err(ApiError::Unauthorized);
    };
    let given = headers
        .get("x-notai-secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if !ct_eq(given.as_bytes(), expected.as_bytes()) {
        return Err(ApiError::Unauthorized);
    }
    let text = EntryText::new(&body.text)?;
    let at = body.sent_at.unwrap_or_else(|| st.now());
    let key = dedupe_key(&text, at);
    let captured = st
        .store
        .capture(NewEntry {
            text,
            source: Source::WhatsApp,
            topic: None,
            captured_at: at,
            dedupe_key: Some(key),
        })
        .await?;
    let (status, duplicate) = match &captured {
        Captured::New(_) => (StatusCode::CREATED, false),
        Captured::Duplicate(_) => (StatusCode::OK, true),
    };
    Ok((
        status,
        Json(json!({ "id": captured.entry().id, "duplicate": duplicate })),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq() {
        assert!(ct_eq(b"gizli", b"gizli"));
        assert!(!ct_eq(b"gizli", b"gizlI"));
        assert!(!ct_eq(b"gizli", b"gizli2"));
        assert!(ct_eq(b"", b""));
    }
}
