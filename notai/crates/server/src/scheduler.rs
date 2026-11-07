//! Zamanlayıcı: sabah planı bildirimi ve çöp kutusu temizliği.

use std::time::Duration;

use chrono::NaiveDate;
use notai_core::morning;
use notai_store::StoreError;
use tokio::sync::watch;

use crate::state::AppState;
use notai_api::MorningPlanView;
use notai_api::convert;

const LAST_SENT_KEY: &str = "morning_plan_last_sent";
/// Son üretilen plan (uygulama açılış kartında da gösterilir).
pub const LATEST_PLAN_KEY: &str = "morning_plan_latest";

/// Zamanı gelen hatırlatmaları bildirir (her biri bir kez). Dönüş: bildirilen sayı.
pub async fn fire_reminders(state: &AppState) -> Result<usize, StoreError> {
    let now = state.now();
    let tasks = state.store.reminder_tasks().await?;
    let due = notai_core::reminders::due_now(&tasks, now, state.cfg.tz);
    for (task, at) in &due {
        // Önce işaretle: bildirim başarısız olsa da aynı hatırlatma tekrar tekrar çalmasın.
        state.store.mark_reminder_fired(task.id, *at).await?;
        if let Some(url) = state.cfg.ntfy_url.as_deref() {
            send(url, "Hatırlatma", task.title.as_str(), "bell").await;
        }
        tracing::info!(task = %task.id, "hatırlatma bildirildi");
    }
    Ok(due.len())
}

/// Zamanlayıcının bir turu. Dönüş: sabah planı bu turda üretildi mi?
pub async fn tick(state: &AppState) -> Result<bool, StoreError> {
    let now = state.now();
    state.store.purge_trash(now).await?;
    fire_reminders(state).await?;

    let last_sent = state
        .store
        .kv_get(LAST_SENT_KEY)
        .await?
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
    if !morning::is_due(now, state.cfg.tz, state.cfg.morning_at, last_sent) {
        return Ok(false);
    }
    let today = now.with_timezone(&state.cfg.tz).date_naive();
    let plan = state.store.morning_plan(today).await?;
    let view = convert::morning_plan(plan, &state.cfg.user_name);
    state
        .store
        .kv_set(LATEST_PLAN_KEY, &serde_json::to_string(&view)?)
        .await?;
    // Önce "gönderildi" işaretle: bildirim başarısız olsa da aynı gün iki kez gönderme.
    state
        .store
        .kv_set(LAST_SENT_KEY, &today.format("%Y-%m-%d").to_string())
        .await?;
    if let Some(url) = state.cfg.ntfy_url.as_deref() {
        notify(url, &view).await;
    }
    Ok(true)
}

/// Sabah planını ntfy bildirimine çevirir.
async fn notify(url: &str, view: &MorningPlanView) {
    let mut body = String::new();
    for item in &view.items {
        body.push_str(&format!("• {} — {}\n", item.title, item.reason));
    }
    if view.more > 0 {
        body.push_str(&format!("+{} görev daha", view.more));
    }
    send(url, &view.headline, &body, "sunrise").await;
}

/// ntfy'ye bildirim gönderir (ev sunucusundan telefona). Hata yalnız günlüğe yazılır.
async fn send(url: &str, title: &str, body: &str, tag: &str) {
    let client = reqwest::Client::new();
    let res = client
        .post(url)
        // HTTP başlıkları yalnız ASCII taşır; Türkçe başlık ntfy'nin sorgu parametresiyle gider.
        .query(&[("title", title), ("tags", tag)])
        .body(body.to_owned())
        .timeout(Duration::from_secs(10))
        .send()
        .await;
    match res {
        Ok(r) if r.status().is_success() => tracing::info!(title, "bildirim gönderildi"),
        Ok(r) => tracing::warn!(status = %r.status(), "bildirim reddedildi"),
        Err(e) => tracing::warn!(error = %e, "bildirim gönderilemedi"),
    }
}

pub async fn run(state: AppState, mut shutdown: watch::Receiver<bool>) {
    loop {
        if let Err(e) = tick(&state).await {
            tracing::error!(error = %e, "zamanlayıcı hatası");
        }
        tokio::select! {
            () = tokio::time::sleep(Duration::from_secs(30)) => {}
            _ = shutdown.changed() => break,
        }
    }
}
