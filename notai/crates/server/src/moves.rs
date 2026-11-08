//! Konum hareketleri uçları: aylık rapor, kaydın hareket durumu, arayüzden konum.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{Datelike, Duration};
use notai_api::{LocateRequest, LocateView, MovementCheckView, MovesView, convert};
use notai_core::EntryId;
use notai_core::movement::Fix;
use notai_core::notice::Notice;
use serde::Deserialize;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/movements", get(report))
        .route("/entries/{id}/movement", get(check))
        .route("/entries/{id}/location", post(locate))
}

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError::BadRequest(msg.into())
}

#[derive(Debug, Deserialize)]
struct MonthQuery {
    month: Option<String>,
}

fn parse_month(s: &str) -> Option<(i32, u32)> {
    let (y, m) = s.split_once('-')?;
    let (y, m) = (y.parse().ok()?, m.parse().ok()?);
    (1..=12).contains(&m).then_some((y, m))
}

async fn report(
    State(st): State<AppState>,
    Query(q): Query<MonthQuery>,
) -> ApiResult<Json<MovesView>> {
    let today = st.now().with_timezone(&st.cfg.tz).date_naive();
    let (y, m) = match q.month.as_deref() {
        Some(s) => parse_month(s).ok_or_else(|| bad(format!("geçersiz ay: {s}")))?,
        None => (today.year(), today.month()),
    };
    let first = chrono::NaiveDate::from_ymd_opt(y, m, 1).ok_or_else(|| bad("geçersiz ay"))?;
    // Ay sınırındaki kalış/yolculuk için birkaç gün geniş al.
    let from = (first - Duration::days(3))
        .and_hms_opt(0, 0, 0)
        .map(|d| d.and_utc());
    let to = (first + Duration::days(35))
        .and_hms_opt(0, 0, 0)
        .map(|d| d.and_utc());
    let (Some(from), Some(to)) = (from, to) else {
        return Err(bad("geçersiz ay"));
    };
    let moves = st.store.movements_between(from, to).await?;
    let places = st.store.places().await?;
    let spends = st.store.spends_between(from, to).await?;
    Ok(Json(convert::moves_view(
        &moves, &places, &spends, y, m, st.cfg.tz,
    )))
}

fn entry_id(id: &str) -> ApiResult<EntryId> {
    id.parse()
        .map_err(|_| bad(format!("geçersiz kimlik: {id}")))
}

async fn check(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<MovementCheckView>> {
    let m = st.store.movement_for(entry_id(&id)?).await?;
    Ok(Json(MovementCheckView {
        movement: m.is_some(),
        wants_location: m.is_some_and(|m| m.fix.is_none()),
    }))
}

async fn locate(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<LocateRequest>,
) -> ApiResult<Json<LocateView>> {
    let eid = entry_id(&id)?;
    let fix = match (b.lat, b.lon) {
        (Some(lat), Some(lon)) => Some(
            Fix::from_degrees(lat, lon, b.accuracy_m.unwrap_or(50.0))
                .map_err(|e| bad(e.to_string()))?,
        ),
        _ => None,
    };
    let now = st.now();
    let Some(out) = st.store.locate_movement(eid, fix, now).await? else {
        return Ok(Json(LocateView::default()));
    };
    after_locate(&st, eid, &out, now).await?;
    Ok(Json(LocateView {
        message: out.message,
        need_location: out.need_location,
    }))
}

/// Soru açıldıysa bildirir; konum elzemse "Konumu aç" bildirimi gönderir.
pub(crate) async fn after_locate(
    st: &AppState,
    entry: EntryId,
    out: &notai_store::LocateOutcome,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(), notai_store::StoreError> {
    if out.need_location {
        let e = st.store.entry(entry).await?;
        let topic = match e.topic() {
            Some(t) => Some((t, st.store.topic(t).await?.name.as_str().to_owned())),
            None => None,
        };
        let n = Notice::location(&out.message, topic, entry, now);
        st.store.set_notice(&n).await?;
        crate::scheduler::push(st, &n.title, &n.body, "round_pushpin").await;
    }
    if let Some(q) = &out.question {
        crate::scheduler::notify_question(st, entry, q).await?;
    }
    Ok(())
}
