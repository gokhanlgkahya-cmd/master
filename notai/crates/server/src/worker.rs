//! AI işçisi: gelen kutusundaki kayıtları sırayla sınıflandırır.
//!
//! Yakalama asla AI'ı beklemez; bu işçi arka planda kuyruğu boşaltır. Hata olursa kayıt
//! kaybolmaz: yeniden denenir ya da "AI işleyemedi" diye işaretlenir.

use std::time::Duration;

use notai_ai::{ClassifyContext, ContextTopic, classify};
use notai_core::profile::for_prompt;
use notai_core::topic::KindFilter;
use notai_store::StoreError;
use tokio::sync::watch;

use crate::state::AppState;

/// Bağlama giren son kayıt sayısı.
const RECENT_CONTEXT: u32 = 10;

/// Kuyruktaki bir işi işler. Dönüş: bir iş işlendi mi?
pub async fn process_one(state: &AppState) -> Result<bool, StoreError> {
    let Some(llm) = state.llm.as_deref() else {
        return Ok(false);
    };
    let now = state.now();
    let Some(entry) = state.store.next_ai_job(now).await? else {
        return Ok(false);
    };

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

    let entry_id = entry.id;
    let ctx = ClassifyContext {
        entry,
        topics,
        facts,
        recent,
        tz: state.cfg.tz,
        user_name: state.cfg.user_name.clone(),
    };

    match classify(llm, &ctx).await {
        Ok(validated) => {
            state
                .store
                .save_ai_suggestions(entry_id, &validated.suggestions, state.now())
                .await?;
            tracing::info!(entry = %entry_id, count = validated.suggestions.len(), "AI önerileri kaydedildi");
        }
        Err(e) => {
            let failed = state
                .store
                .record_ai_failure(entry_id, &e.to_string(), e.is_retryable(), state.now())
                .await?;
            tracing::warn!(entry = %entry_id, error = %e, failed, "AI işi başarısız");
        }
    }
    Ok(true)
}

/// Kapatma sinyaline kadar kuyruğu işler.
pub async fn run(state: AppState, mut shutdown: watch::Receiver<bool>) {
    loop {
        let worked = match process_one(&state).await {
            Ok(w) => w,
            Err(e) => {
                tracing::error!(error = %e, "AI işçisi depo hatası");
                false
            }
        };
        let pause = if worked {
            Duration::from_millis(50)
        } else {
            Duration::from_secs(5)
        };
        tokio::select! {
            () = tokio::time::sleep(pause) => {}
            _ = shutdown.changed() => break,
        }
    }
}
