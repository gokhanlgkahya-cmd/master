//! Varlıklar uçları: özet, ekleme/düzenleme, dış ajan için fiyatı güncellenecek varlıklar.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Duration;
use notai_api::{AssetRequest, AssetsView, convert};
use notai_core::TopicId;
use notai_core::finance::Money;
use notai_core::notice::Notice;
use notai_core::text::TopicName;
use notai_core::topic::ParaKind;
use notai_store::NewTopic;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/assets", get(list).post(create))
        .route("/topics/{id}/asset", post(set))
        .route("/agent/assets", get(to_price))
        .route("/agent/assets/{id}/price", post(price))
}

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError::BadRequest(msg.into())
}

async fn list(State(st): State<AppState>) -> ApiResult<Json<AssetsView>> {
    let today = st.now().with_timezone(&st.cfg.tz).date_naive();
    let topics = st.store.asset_topics().await?;
    let past = st.store.assets_total_on(today - Duration::days(30)).await?;
    Ok(Json(convert::assets_view(&topics, past)))
}

/// Yeni varlık: "Varlıklar" ana konusunun altında yeni konu.
async fn create(State(st): State<AppState>, Json(r): Json<AssetRequest>) -> ApiResult<Json<Value>> {
    let now = st.now();
    let today = now.with_timezone(&st.cfg.tz).date_naive();
    let info = convert::asset_info(&r, today).map_err(bad)?;
    let root = st.store.assets_root(now).await?;
    let mut n = NewTopic::new(TopicName::new(&r.name)?, ParaKind::Resource);
    n.parent = Some(root.id);
    let mut t = st.store.create_topic(n, now).await?;
    t.asset = Some(info);
    st.store.update_topic(&t, now).await?;
    Ok(Json(json!({ "topic_id": t.id.to_string() })))
}

/// Var olan konunun varlık bilgisini günceller; `kind` boşsa kaldırır.
async fn set(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(r): Json<AssetRequest>,
) -> ApiResult<Json<Value>> {
    let tid: TopicId = id
        .parse()
        .map_err(|_| bad(format!("geçersiz kimlik: {id}")))?;
    let now = st.now();
    let today = now.with_timezone(&st.cfg.tz).date_naive();
    let mut t = st.store.topic(tid).await?;
    t.asset = if r.kind.trim().is_empty() {
        None
    } else {
        Some(convert::asset_info(&r, today).map_err(bad)?)
    };
    st.store.update_topic(&t, now).await?;
    Ok(Json(json!({ "asset": t.asset.is_some() })))
}

/// Bugün fiyatı güncellenmemiş, sembollü varlıklar (ajan günde bir çağırır).
async fn to_price(State(st): State<AppState>) -> ApiResult<Json<Value>> {
    let today = st.now().with_timezone(&st.cfg.tz).date_naive();
    let list = st.store.assets_to_price(today).await?;
    Ok(Json(Value::Array(
        list.into_iter()
            .map(|(t, a)| {
                json!({
                    "topic_id": t.id.to_string(),
                    "name": t.name.as_str(),
                    "kind": a.kind.label(),
                    "symbol": a.symbol,
                    "unit": a.unit,
                    "quantity": a.quantity_milli.map(notai_core::asset::format_quantity),
                    "last_value": a.value.format_abs(),
                    "instructions": "Sembolün güncel BİRİM fiyatını TL olarak bul (hisse: son fiyat; altın: gram \
alış; döviz: TL karşılığı). `notai asset-price <topic_id> <fiyat>` ile bildir. Miktarla çarpımı sunucu yapar.",
                })
            })
            .collect(),
    )))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PriceBody {
    /// Birim fiyat (TL).
    unit_price: String,
}

async fn price(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<PriceBody>,
) -> ApiResult<Json<Value>> {
    let tid: TopicId = id
        .parse()
        .map_err(|_| bad(format!("geçersiz kimlik: {id}")))?;
    let unit = Money::parse(&b.unit_price)?;
    if unit.0 <= 0 {
        return Err(bad("birim fiyat sıfırdan büyük olmalı"));
    }
    let now = st.now();
    let today = now.with_timezone(&st.cfg.tz).date_naive();
    let msg = st.store.price_asset(tid, unit, now, today).await?;
    let Some(entry) = msg else {
        return Ok(Json(
            json!({ "changed": false, "message": "değer güncellendi (küçük değişim; konuya yazılmadı)" }),
        ));
    };
    let t = st.store.topic(tid).await?;
    let notice = Notice::asset(
        &entry.text.preview(80),
        (t.id, t.name.as_str().to_owned()),
        entry.id,
        now,
    );
    st.store.set_notice(&notice).await?;
    crate::scheduler::push(&st, &notice.title, &notice.body, "chart_with_upwards_trend").await;
    Ok(Json(
        json!({ "changed": true, "message": entry.text.as_str() }),
    ))
}
