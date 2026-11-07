//! Sınıflandırma bağlamı ve sonucun uygulanması: hem iç AI işçisi hem **dış ajan** modu
//! (`NOTAI_AI=external`: S24'teki Hermes / Claude Code gibi ajanlar işleri `notai` CLI ya da
//! `/api/agent/*` uçlarıyla alır ve cevaplar) aynı kodu kullanır. Dış ajanın cevabı da iç
//! modelinki gibi katı şema doğrulamasından geçer; doğrulanmayan hiçbir şey yazılmaz.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use notai_ai::{
    ClassifyContext, ClassifyExample, ContextTopic, Validated, build_request, validate,
};
use notai_core::profile::for_prompt;
use notai_core::topic::KindFilter;
use notai_core::{Entry, EntryId};
use notai_store::StoreError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::{ApiError, ApiResult};
use crate::routes_v2::autonomy;
use crate::state::AppState;

/// Bağlama giren son kayıt sayısı.
const RECENT_CONTEXT: u32 = 10;
/// İsteme giren geçmiş karar örneği sayısı (öğrenme sinyali).
const EXAMPLES: u32 = 12;

/// Bir kayıt için sınıflandırma bağlamı (konular, onaylı bilgiler, son kayıtlar, örnekler).
pub async fn build_context(state: &AppState, entry: Entry) -> Result<ClassifyContext, StoreError> {
    let tz = state.cfg.tz;
    let topics = state
        .store
        .topic_list(KindFilter::All)
        .await?
        .into_iter()
        .map(|a| ContextTopic {
            id: a.topic.id,
            name: a.topic.name,
            kind: a.topic.kind,
        })
        .collect();
    let facts_all = state.store.facts().await?;
    let facts = for_prompt(&facts_all)
        .map(|f| f.text.as_str().to_owned())
        .collect();
    let recent = state
        .store
        .recent_entries(RECENT_CONTEXT + 1)
        .await?
        .into_iter()
        .filter(|e| e.id != entry.id)
        .take(RECENT_CONTEXT as usize)
        .map(|e| e.text.as_str().to_owned())
        .collect();
    let examples = state
        .store
        .decision_examples(EXAMPLES)
        .await?
        .into_iter()
        .map(|x| ClassifyExample {
            entry_text: x.entry_text,
            chip: x.kind.chip_text(tz),
            accepted: x.accepted,
        })
        .collect();
    Ok(ClassifyContext {
        entry,
        topics,
        facts,
        recent,
        tz,
        user_name: state.cfg.user_name.clone(),
        examples,
    })
}

/// Doğrulanmış önerileri kaydeder; otonom mod açıksa eşik üstündekileri uygular.
pub async fn save_validated(
    state: &AppState,
    entry: EntryId,
    v: &Validated,
) -> Result<usize, StoreError> {
    state
        .store
        .save_ai_suggestions(entry, &v.suggestions, state.now())
        .await?;
    let (on, threshold) = autonomy(&state.store).await?;
    if on {
        for s in v
            .suggestions
            .iter()
            .filter(|s| s.confidence.get() >= threshold)
        {
            match state
                .store
                .decide_with(s.id, true, true, state.now(), state.cfg.tz)
                .await
            {
                Ok(_) => tracing::info!(suggestion = %s.id, "otonom mod öneriyi uyguladı"),
                Err(e) => tracing::warn!(suggestion = %s.id, error = %e, "otonom uygulama atlandı"),
            }
        }
    }
    Ok(v.suggestions.len())
}

// ---------------- Dış ajan uçları ----------------

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/agent/jobs", get(jobs))
        .route("/agent/jobs/{id}/answer", post(answer))
        .route("/agent/jobs/{id}/fail", post(fail))
}

/// Ajana verilen iş: istem (sistem + kullanıcı) ve cevabın uyması gereken JSON şeması.
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentJob {
    pub entry_id: String,
    pub text: String,
    /// İç modele verilecek sistem isteminin aynısı (kurallar + bağlam).
    pub system: String,
    pub user: String,
    pub schema: Value,
}

#[derive(Debug, Deserialize)]
struct JobsQuery {
    limit: Option<u32>,
}

async fn jobs(
    State(st): State<AppState>,
    Query(q): Query<JobsQuery>,
) -> ApiResult<Json<Vec<AgentJob>>> {
    let limit = q.limit.unwrap_or(5).clamp(1, 20) as usize;
    let mut out = Vec::new();
    // Kuyruk sırasıyla (en eski önce), bekleme süresi dolmuş işler.
    let mut seen = Vec::new();
    while out.len() < limit {
        let Some(entry) = st.store.next_ai_job_excluding(st.now(), &seen).await? else {
            break;
        };
        seen.push(entry.id);
        let text = entry.text.as_str().to_owned();
        let id = entry.id.to_string();
        let ctx = build_context(&st, entry).await?;
        let req = build_request(&ctx);
        out.push(AgentJob {
            entry_id: id,
            text,
            system: req.system,
            user: req.user,
            schema: req.schema,
        });
    }
    Ok(Json(out))
}

/// Ajanın cevabı: modelin üreteceği JSON nesnesinin kendisi (şemaya uymalı).
async fn answer(
    State(st): State<AppState>,
    Path(eid): Path<String>,
    Json(raw): Json<Value>,
) -> ApiResult<Json<Value>> {
    let id: EntryId = eid
        .parse()
        .map_err(|_| ApiError::BadRequest(format!("geçersiz kimlik: {eid}")))?;
    let entry = st.store.entry(id).await?;
    let ctx = build_context(&st, entry).await?;
    let v = validate(&raw.to_string(), &ctx)
        .map_err(|e| ApiError::BadRequest(format!("cevap şemaya uymuyor: {e}")))?;
    let saved = save_validated(&st, id, &v).await?;
    Ok(Json(json!({ "saved": saved, "dropped": v.dropped })))
}

#[derive(Debug, Deserialize)]
struct FailBody {
    error: String,
    #[serde(default)]
    retry: bool,
}

async fn fail(
    State(st): State<AppState>,
    Path(eid): Path<String>,
    Json(b): Json<FailBody>,
) -> ApiResult<Json<Value>> {
    let id: EntryId = eid
        .parse()
        .map_err(|_| ApiError::BadRequest(format!("geçersiz kimlik: {eid}")))?;
    let failed = st
        .store
        .record_ai_failure(id, &b.error, b.retry, st.now())
        .await?;
    Ok(Json(json!({ "failed": failed })))
}
