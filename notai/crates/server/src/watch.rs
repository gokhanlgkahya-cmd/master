//! Fiyat / ürün takibi uçları: konuya takip ayarı ve dış ajan (Hermes / Claude Code) için
//! zamanı gelen takipler + rapor. Ajanın raporu doğrulanır; yalnız değişince konuya NotAI
//! mesajı yazılır ve bildirim gider.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use notai_api::{WatchRequest, convert};
use notai_core::TopicId;
use notai_core::finance::Money;
use notai_core::notice::Notice;
use notai_core::watch::{MAX_OFFERS, Offer};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/topics/{id}/watch", post(set_watch))
        .route("/agent/watches", get(due))
        .route("/agent/watches/{id}/report", post(report))
}

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError::BadRequest(msg.into())
}

fn topic_id(s: &str) -> ApiResult<TopicId> {
    s.parse().map_err(|_| bad(format!("geçersiz kimlik: {s}")))
}

/// Takibi açar/günceller; `query` boşsa kapatır.
async fn set_watch(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(r): Json<WatchRequest>,
) -> ApiResult<Json<Value>> {
    let mut t = st.store.topic(topic_id(&id)?).await?;
    t.watch = convert::watch_config(&r).map_err(bad)?;
    st.store.update_topic(&t, st.now()).await?;
    Ok(Json(json!({ "watching": t.watch.is_some() })))
}

fn report_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["offers"],
        "properties": {
            "offers": {
                "type": "array",
                "maxItems": MAX_OFFERS,
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["title", "price", "url", "store"],
                    "properties": {
                        "title": { "type": "string" },
                        "price": { "type": "string", "description": "TL, ör. 52999.00 ya da 52.999,00" },
                        "url": { "type": "string", "description": "https:// ile başlayan ürün sayfası" },
                        "store": { "type": ["string", "null"] }
                    }
                }
            }
        }
    })
}

/// Zamanı gelmiş takipler (ajan günde bir çağırır).
async fn due(State(st): State<AppState>) -> ApiResult<Json<Value>> {
    let list = st.store.due_watches(st.now()).await?;
    let all = st
        .store
        .topic_list(notai_core::topic::KindFilter::All)
        .await?;
    let path = |id: TopicId| {
        all.iter()
            .find(|a| a.topic.id == id)
            .map(|a| a.path.clone())
            .unwrap_or_default()
    };
    let jobs: Vec<Value> = list
        .into_iter()
        .filter_map(|d| {
            let cfg = d.topic.watch.as_ref()?;
            Some(json!({
                "topic_id": d.topic.id.to_string(),
                "topic": path(d.topic.id),
                "query": cfg.query,
                "urls": cfg.urls,
                "target_price": cfg.target_price.map(Money::format_abs),
                "last_best": d.state.best.as_ref().map(|b| json!({
                    "price": b.price.format_abs(), "url": b.url, "store": b.store
                })),
                "instructions": "Ürünün güncel satış fiyatlarını bul (verilen bağlantılar + Türkiye'deki \
güvenilir mağazalar). Her teklif için ürün adı, fiyat (TL), ürün sayfası bağlantısı ve mağaza adı ver; \
stokta olmayanları ve ikinci elleri alma. En fazla 20 teklif. Yalnız şemaya uyan JSON ver.",
                "schema": report_schema(),
            }))
        })
        .collect();
    Ok(Json(Value::Array(jobs)))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportBody {
    offers: Vec<RawOffer>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOffer {
    title: String,
    price: String,
    url: String,
    #[serde(default)]
    store: Option<String>,
}

/// Ajanın raporu: doğrulanır, değişiklik varsa konuya yazılır + bildirim.
async fn report(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(raw): Json<Value>,
) -> ApiResult<Json<Value>> {
    let tid = topic_id(&id)?;
    let body: ReportBody =
        serde_json::from_value(raw).map_err(|e| bad(format!("rapor şemaya uymuyor: {e}")))?;
    let mut offers = Vec::new();
    let mut dropped = 0;
    for o in body.offers.into_iter().take(MAX_OFFERS) {
        match Money::parse(&o.price)
            .and_then(|p| Offer::new(&o.title, p, &o.url, o.store.as_deref()))
        {
            Ok(x) => offers.push(x),
            Err(_) => dropped += 1,
        }
    }
    let now = st.now();
    let result = st.store.record_watch(tid, &offers, now).await?;
    let Some((entry, rep)) = result else {
        return Ok(Json(json!({
            "changed": false,
            "message": "değişiklik yok; konuya yazılmadı",
            "dropped": dropped
        })));
    };
    let topic = st.store.topic(tid).await?;
    let notice = Notice::watch(
        &rep.headline,
        (topic.id, topic.name.as_str().to_owned()),
        entry.id,
        now,
    );
    st.store.set_notice(&notice).await?;
    crate::scheduler::push(&st, &notice.title, &notice.body, "shopping_cart").await;
    Ok(Json(
        json!({ "changed": true, "message": rep.message, "dropped": dropped }),
    ))
}
