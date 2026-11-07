//! AI işçisi: gelen kutusundaki kayıtları sırayla sınıflandırır, otonom moddaysa yüksek
//! güvenli önerileri uygular, boşta kalınca anlamsal arama vektörlerini hesaplar.
//!
//! Yakalama asla AI'ı beklemez; bu işçi arka planda kuyruğu boşaltır. Hata olursa kayıt
//! kaybolmaz: yeniden denenir ya da "AI işleyemedi" diye işaretlenir.

use std::time::Duration;

use notai_ai::{ClassifyContext, ClassifyExample, ContextTopic, classify};
use notai_core::profile::for_prompt;
use notai_core::topic::KindFilter;
use notai_store::StoreError;
use tokio::sync::watch;

use crate::routes_v2::autonomy;
use crate::state::AppState;

/// Bağlama giren son kayıt sayısı.
const RECENT_CONTEXT: u32 = 10;
/// İsteme giren geçmiş karar örneği sayısı (öğrenme sinyali).
const EXAMPLES: u32 = 12;
/// Bir turda vektörü hesaplanan kayıt sayısı.
const EMBED_BATCH: u32 = 16;

/// Kuyruktaki bir işi işler. Dönüş: bir iş işlendi mi?
pub async fn process_one(state: &AppState) -> Result<bool, StoreError> {
    let Some(llm) = state.llm.as_deref() else {
        return Ok(false);
    };
    let now = state.now();
    let Some(entry) = state.store.next_ai_job(now).await? else {
        return Ok(false);
    };
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

    let entry_id = entry.id;
    let ctx = ClassifyContext {
        entry,
        topics,
        facts,
        recent,
        tz,
        user_name: state.cfg.user_name.clone(),
        examples,
    };

    match classify(llm, &ctx).await {
        Ok(validated) => {
            state
                .store
                .save_ai_suggestions(entry_id, &validated.suggestions, state.now())
                .await?;
            tracing::info!(entry = %entry_id, count = validated.suggestions.len(), "AI önerileri kaydedildi");
            let (on, threshold) = autonomy(&state.store).await?;
            if on {
                for s in validated
                    .suggestions
                    .iter()
                    .filter(|s| s.confidence.get() >= threshold)
                {
                    match state
                        .store
                        .decide_with(s.id, true, true, state.now(), tz)
                        .await
                    {
                        Ok(_) => tracing::info!(suggestion = %s.id, "otonom mod öneriyi uyguladı"),
                        Err(e) => {
                            tracing::warn!(suggestion = %s.id, error = %e, "otonom uygulama atlandı")
                        }
                    }
                }
            }
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

/// Vektörü olmayan kayıtların vektörlerini hesaplar. Dönüş: hesaplanan sayı.
pub async fn embed_pending(state: &AppState) -> Result<usize, StoreError> {
    let Some(emb) = state.embedder.as_deref() else {
        return Ok(0);
    };
    let entries = state
        .store
        .entries_without_embedding(emb.model(), EMBED_BATCH)
        .await?;
    if entries.is_empty() {
        return Ok(0);
    }
    let texts: Vec<String> = entries.iter().map(|e| e.text.as_str().to_owned()).collect();
    match emb.embed(&texts).await {
        Ok(vectors) => {
            for (e, v) in entries.iter().zip(vectors) {
                state
                    .store
                    .save_embedding(e.id, emb.model(), &v, state.now())
                    .await?;
            }
            Ok(entries.len())
        }
        Err(e) => {
            tracing::warn!(error = %e, "vektör hesaplanamadı (sonra yeniden denenecek)");
            Ok(0)
        }
    }
}

/// Kapatma sinyaline kadar kuyruğu işler.
pub async fn run(state: AppState, mut shutdown: watch::Receiver<bool>) {
    loop {
        let worked = match process_one(&state).await {
            Ok(true) => true,
            Ok(false) => match embed_pending(&state).await {
                Ok(n) => n > 0,
                Err(e) => {
                    tracing::error!(error = %e, "vektör işçisi depo hatası");
                    false
                }
            },
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
