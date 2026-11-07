//! Harcamalar: aylık rapor, ekstre yükleme (PDF/metin), kategori/kapsam düzeltme ve
//! kurallarla okunamayan ekstreler için dış ajan uçları.

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{Datelike, NaiveDate};
use notai_api::{ClassifyRequest, FinanceView, ImportView, convert};
use notai_core::finance::{Money, Scope, StatementLine};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Ekstre dosyası en fazla 10 MB.
const STATEMENT_LIMIT: usize = 10 * 1024 * 1024;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/finance", get(finance))
        .route(
            "/finance/statements",
            post(upload_statement).layer(DefaultBodyLimit::max(STATEMENT_LIMIT)),
        )
        .route("/finance/transactions/{id}/classify", post(classify))
        .route("/agent/statements", get(agent_statements))
        .route("/agent/statements/{id}/answer", post(agent_answer))
}

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError::BadRequest(msg.into())
}

#[derive(Debug, Deserialize)]
struct MonthQuery {
    /// "2026-10"; yoksa bu ay.
    month: Option<String>,
}

fn parse_month(s: &str) -> Option<(i32, u32)> {
    let (y, m) = s.split_once('-')?;
    let (y, m) = (y.parse().ok()?, m.parse().ok()?);
    (1..=12).contains(&m).then_some((y, m))
}

async fn finance(
    State(st): State<AppState>,
    Query(q): Query<MonthQuery>,
) -> ApiResult<Json<FinanceView>> {
    let today = st.now().with_timezone(&st.cfg.tz).date_naive();
    let (y, m) = match q.month.as_deref() {
        Some(s) => parse_month(s).ok_or_else(|| bad(format!("geçersiz ay: {s}")))?,
        None => (today.year(), today.month()),
    };
    // Önceki ay da gerekir (değişim ve düzenli ödemeler için).
    let from = NaiveDate::from_ymd_opt(y, m, 1)
        .and_then(|d| d.checked_sub_months(chrono::Months::new(1)))
        .ok_or_else(|| bad("geçersiz ay"))?;
    let to = NaiveDate::from_ymd_opt(y, m, 1)
        .and_then(|d| d.checked_add_months(chrono::Months::new(1)))
        .and_then(|d| d.pred_opt())
        .ok_or_else(|| bad("geçersiz ay"))?;
    let txs = st.store.transactions_between(from, to).await?;
    let pending = u32::try_from(st.store.pending_statements().await?.len()).unwrap_or(u32::MAX);
    Ok(Json(convert::finance_view(&txs, y, m, pending)))
}

#[derive(Debug, Deserialize)]
struct UploadQuery {
    /// Hesap adı/son haneleri (mükerrer anahtarının parçası); yoksa dosya adı.
    account: Option<String>,
    name: Option<String>,
}

/// PDF (application/pdf) ya da düz metin ekstre yükler.
async fn upload_statement(
    State(st): State<AppState>,
    Query(q): Query<UploadQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<(StatusCode, Json<ImportView>)> {
    if body.is_empty() {
        return Err(bad("ekstre dosyası boş"));
    }
    let name = q.name.unwrap_or_else(|| "ekstre".into());
    let is_pdf = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("pdf"))
        || body.starts_with(b"%PDF");
    let text = if is_pdf {
        notai_store::pdf_text(&body)?
    } else {
        String::from_utf8(body.to_vec()).map_err(|_| bad("ekstre metni UTF-8 değil"))?
    };
    let account = q
        .account
        .filter(|a| !a.trim().is_empty())
        .unwrap_or_else(|| name.clone());
    let (_, report) = st
        .store
        .import_statement(&account, &name, &text, st.now())
        .await?;
    Ok((StatusCode::CREATED, Json(convert::import_view(&report))))
}

async fn classify(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<ClassifyRequest>,
) -> ApiResult<Json<Value>> {
    if !notai_core::finance::CATEGORIES
        .iter()
        .any(|(k, _)| *k == b.category)
    {
        return Err(bad(format!("bilinmeyen kategori: {}", b.category)));
    }
    let scope = Scope::parse(&b.scope)?;
    let changed = st
        .store
        .classify_transaction(&id, &b.category, scope, st.now())
        .await?;
    Ok(Json(json!({ "changed": changed })))
}

// ---------- Dış ajan: kurallarla okunamayan ekstreler ----------

/// Ajanın cevabı: ekstredeki hareketler.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatementAnswer {
    pub lines: Vec<RawLine>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawLine {
    /// YYYY-AA-GG
    pub date: String,
    pub description: String,
    /// Eksi = gider ("-1850.00" ya da "-1.850,00").
    pub amount: String,
}

fn answer_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["lines"],
        "properties": {
            "lines": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["date", "description", "amount"],
                    "properties": {
                        "date": { "type": "string", "description": "YYYY-AA-GG" },
                        "description": { "type": "string" },
                        "amount": { "type": "string", "description": "Eksi = gider, artı = gelir; ör. -1850.00" }
                    }
                }
            }
        }
    })
}

async fn agent_statements(State(st): State<AppState>) -> ApiResult<Json<Value>> {
    let pending = st.store.pending_statements().await?;
    let jobs: Vec<Value> = pending
        .into_iter()
        .map(|p| {
            json!({
                "id": p.id,
                "account": p.account,
                "file_name": p.file_name,
                "text": p.text,
                "instructions": "Bu banka ekstresi metnindeki her hesap hareketini çıkar: tarih (YYYY-AA-GG), \
açıklama (satıcı/işlem adı olduğu gibi), tutar (gider eksi, gelir artı). Bakiye, toplam ve başlık \
satırlarını alma. Yalnız şemaya uyan JSON ver.",
                "schema": answer_schema(),
            })
        })
        .collect();
    Ok(Json(Value::Array(jobs)))
}

/// Ajanın satırlarını doğrular ve içe aktarır (aynı mükerrer/eşleştirme kurallarıyla).
async fn agent_answer(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(raw): Json<Value>,
) -> ApiResult<Json<ImportView>> {
    let ans: StatementAnswer =
        serde_json::from_value(raw).map_err(|e| bad(format!("cevap şemaya uymuyor: {e}")))?;
    let mut lines = Vec::new();
    let mut dropped = 0;
    for l in ans.lines {
        let date = NaiveDate::parse_from_str(&l.date, "%Y-%m-%d");
        let amount = Money::parse(&l.amount);
        match (date, amount) {
            (Ok(date), Ok(amount)) if amount.0 != 0 && !l.description.trim().is_empty() => {
                lines.push(StatementLine {
                    date,
                    description: l.description.trim().to_owned(),
                    amount,
                });
            }
            _ => dropped += 1,
        }
    }
    let report = st.store.answer_statement(&id, &lines, st.now()).await?;
    let mut view = convert::import_view(&report);
    if dropped > 0 {
        view.message
            .push_str(&format!(" {dropped} satır doğrulanamadı, atlandı."));
    }
    Ok(Json(view))
}
