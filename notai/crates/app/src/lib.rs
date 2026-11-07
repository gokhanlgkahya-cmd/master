//! NotAI uygulama kabuğu (Tauri 2): Android + Windows/Linux masaüstü.
//!
//! Yerel öncelikli: tüm veri cihazdaki SQLite'tadır, uygulama internetsiz tam çalışır.
//! Ev sunucusu ayarlıysa arka planda dakikada bir senkronlanır (AI orada çalışır).

// Masaüstünde unsafe tamamen yasak. Mobilde Tauri'nin giriş noktası makrosu FFI
// yapıştırıcısı üretir (CLAUDE.md'de izin verilen tek istisna).
#![cfg_attr(not(mobile), forbid(unsafe_code))]

pub mod remote;
pub mod service;

use std::path::Path;
use std::time::Duration;

use chrono::Utc;
use chrono_tz::Tz;
use notai_api::{
    AskView, AttachmentView, CalendarView, CaptureRequest, FactView, JournalView, MorningPlanView,
    RemindersView, SettingsView, TaskEditRequest, TasksView, TopicEditRequest, TopicsView,
    TrashItemView, WeeklyReviewView,
};
use notai_store::Store;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_notification::NotificationExt;

/// Uygulama durumu.
pub struct AppState {
    pub store: Store,
    pub tz: Tz,
    pub user: String,
}

#[tauri::command]
async fn topics(st: State<'_, AppState>, kind: String) -> Result<TopicsView, String> {
    service::topics(&st.store, &kind, Utc::now(), st.tz).await
}

#[tauri::command]
async fn journal(st: State<'_, AppState>, topic_id: Option<String>) -> Result<JournalView, String> {
    service::journal(&st.store, topic_id.as_deref(), Utc::now(), st.tz).await
}

#[tauri::command]
async fn capture(st: State<'_, AppState>, req: CaptureRequest) -> Result<String, String> {
    service::capture(&st.store, req, Utc::now()).await
}

#[tauri::command]
async fn decide(st: State<'_, AppState>, id: String, accept: bool) -> Result<(), String> {
    service::decide(&st.store, &id, accept, Utc::now(), st.tz).await
}

#[tauri::command]
async fn entry_action(st: State<'_, AppState>, id: String, action: String) -> Result<(), String> {
    service::entry_action(&st.store, &id, &action, Utc::now()).await
}

#[tauri::command]
async fn tasks(st: State<'_, AppState>) -> Result<TasksView, String> {
    service::tasks(&st.store, Utc::now(), st.tz).await
}

#[tauri::command]
async fn complete_task(st: State<'_, AppState>, id: String) -> Result<(), String> {
    service::complete_task(&st.store, &id, Utc::now()).await
}

#[tauri::command]
async fn morning_plan(st: State<'_, AppState>) -> Result<MorningPlanView, String> {
    service::morning_plan(&st.store, &st.user, Utc::now(), st.tz).await
}

#[tauri::command]
async fn set_sync(st: State<'_, AppState>, url: String, token: String) -> Result<(), String> {
    service::set_sync(&st.store, &url, &token).await
}

#[tauri::command]
async fn sync_now(st: State<'_, AppState>) -> Result<Option<(u32, u32)>, String> {
    service::sync_now(&st.store).await
}

#[tauri::command]
async fn create_task(st: State<'_, AppState>, req: service::NewTaskReq) -> Result<(), String> {
    service::create_task(&st.store, req, Utc::now()).await
}

#[tauri::command]
async fn edit_task(
    st: State<'_, AppState>,
    id: String,
    req: TaskEditRequest,
) -> Result<(), String> {
    service::edit_task(&st.store, &id, &req, Utc::now()).await
}

#[tauri::command]
async fn create_topic(st: State<'_, AppState>, name: String, kind: String) -> Result<(), String> {
    service::create_topic(&st.store, &name, &kind, Utc::now()).await
}

#[tauri::command]
async fn edit_topic(
    st: State<'_, AppState>,
    id: String,
    req: TopicEditRequest,
) -> Result<(), String> {
    service::edit_topic(&st.store, &id, &req, Utc::now()).await
}

#[tauri::command]
async fn move_entry(
    st: State<'_, AppState>,
    id: String,
    topic_id: Option<String>,
) -> Result<(), String> {
    service::move_entry(&st.store, &id, topic_id.as_deref(), Utc::now()).await
}

#[tauri::command]
async fn reminders(st: State<'_, AppState>) -> Result<RemindersView, String> {
    service::reminders(&st.store, Utc::now(), st.tz).await
}

#[tauri::command]
async fn snooze(st: State<'_, AppState>, id: String, option: String) -> Result<(), String> {
    service::snooze(&st.store, &id, &option, Utc::now(), st.tz).await
}

#[tauri::command]
async fn calendar(
    st: State<'_, AppState>,
    from: Option<String>,
    days: u32,
) -> Result<CalendarView, String> {
    service::calendar(&st.store, from.as_deref(), days, Utc::now(), st.tz).await
}

#[tauri::command]
async fn weekly_review(st: State<'_, AppState>) -> Result<WeeklyReviewView, String> {
    service::weekly_review_view(&st.store, Utc::now(), st.tz).await
}

#[tauri::command]
async fn settings(st: State<'_, AppState>) -> Result<SettingsView, String> {
    service::settings(&st.store).await
}

#[tauri::command]
async fn set_settings(
    st: State<'_, AppState>,
    autonomous: Option<bool>,
    threshold: Option<f64>,
) -> Result<(), String> {
    service::set_settings(&st.store, autonomous, threshold).await
}

#[tauri::command]
async fn ask(st: State<'_, AppState>, question: String) -> Result<AskView, String> {
    service::ask(&st.store, &question, st.tz).await
}

#[tauri::command]
async fn voice(
    st: State<'_, AppState>,
    data: Vec<u8>,
    mime: String,
    topic_id: Option<String>,
) -> Result<(), String> {
    service::voice(&st.store, data, &mime, topic_id.as_deref()).await
}

#[tauri::command]
async fn undo(st: State<'_, AppState>, id: String) -> Result<(), String> {
    service::undo(&st.store, &id, Utc::now()).await
}

#[tauri::command]
async fn trash(st: State<'_, AppState>) -> Result<Vec<TrashItemView>, String> {
    service::trash(&st.store, Utc::now()).await
}

#[tauri::command]
async fn purge(st: State<'_, AppState>, id: String) -> Result<(), String> {
    service::purge(&st.store, &id, Utc::now()).await
}

#[tauri::command]
async fn facts(st: State<'_, AppState>) -> Result<Vec<FactView>, String> {
    service::facts(&st.store).await
}

#[tauri::command]
async fn add_fact(st: State<'_, AppState>, text: String) -> Result<(), String> {
    service::add_fact(&st.store, &text, Utc::now()).await
}

#[tauri::command]
async fn fact_action(st: State<'_, AppState>, id: String, action: String) -> Result<(), String> {
    service::fact_action(&st.store, &id, &action, Utc::now()).await
}

#[tauri::command]
async fn attachments(st: State<'_, AppState>, id: String) -> Result<Vec<AttachmentView>, String> {
    service::attachments(&st.store, &id).await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn upload_attachment(
    st: State<'_, AppState>,
    id: String,
    kind: String,
    name: String,
    mime: String,
    data: Vec<u8>,
    value: Option<String>,
) -> Result<(), String> {
    service::upload_attachment(&st.store, &id, &kind, name, mime, data, value, Utc::now()).await
}

/// Yerel bildirimler: zamanı gelen hatırlatmalar ve sabah planı (uygulama açıkken / arka planda).
async fn notify_tick(app: &AppHandle, store: &Store, tz: Tz, user: &str) -> Result<(), String> {
    let now = Utc::now();
    let tasks = store.reminder_tasks().await.map_err(|e| e.to_string())?;
    for (task, at) in notai_core::reminders::due_now(&tasks, now, tz) {
        store
            .mark_reminder_fired(task.id, at)
            .await
            .map_err(|e| e.to_string())?;
        app.notification()
            .builder()
            .title("Hatırlatma")
            .body(task.title.as_str())
            .show()
            .map_err(|e| e.to_string())?;
    }
    let last = store
        .kv_get(MORNING_KEY)
        .await
        .map_err(|e| e.to_string())?
        .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
    let at = chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap_or(chrono::NaiveTime::MIN);
    if notai_core::morning::is_due(now, tz, at, last) {
        let today = now.with_timezone(&tz).date_naive();
        store
            .kv_set(MORNING_KEY, &today.format("%Y-%m-%d").to_string())
            .await
            .map_err(|e| e.to_string())?;
        let plan = store.morning_plan(today).await.map_err(|e| e.to_string())?;
        if !plan.items.is_empty() {
            let body: Vec<String> = plan
                .items
                .iter()
                .map(|i| format!("• {} — {}", i.title, i.reason.label()))
                .collect();
            app.notification()
                .builder()
                .title(plan.headline(user))
                .body(body.join("\n"))
                .show()
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

const MORNING_KEY: &str = "local_morning_plan_last_sent";

/// Bu cihazın kalıcı kimliği (senkron günlüğünde kaynak adı). İlk açılışta üretilir.
fn device_id(dir: &Path) -> std::io::Result<String> {
    let file = dir.join("device_id");
    match std::fs::read_to_string(&file) {
        Ok(s) if !s.trim().is_empty() => Ok(s.trim().to_owned()),
        _ => {
            let id = format!("cihaz-{}", uuid::Uuid::now_v7());
            std::fs::write(&file, &id)?;
            Ok(id)
        }
    }
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let origin = device_id(&dir)?;
    let url = format!("sqlite://{}", dir.join("notai.db").display());
    let store = tauri::async_runtime::block_on(Store::open(&url, origin))?;

    // Ev sunucusu ayarlıysa arka planda senkron (ayarlı değilse sessizce bekler).
    let bg = store.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            if let Err(e) = service::sync_now(&bg).await {
                eprintln!("senkron başarısız (sonra yeniden denenecek): {e}");
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });

    // Yerel bildirim döngüsü (hatırlatmalar + sabah planı).
    let (nstore, handle) = (store.clone(), app.handle().clone());
    tauri::async_runtime::spawn(async move {
        loop {
            if let Err(e) = notify_tick(&handle, &nstore, notai_core::DEFAULT_TZ, "Gökhan").await {
                eprintln!("bildirim turu başarısız: {e}");
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });

    app.manage(AppState {
        store,
        tz: notai_core::DEFAULT_TZ,
        user: "Gökhan".into(),
    });
    Ok(())
}

/// Uygulamayı başlatır (masaüstü `main` ve mobil giriş noktası).
#[cfg_attr(mobile, allow(unsafe_code))]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(setup)
        .invoke_handler(tauri::generate_handler![
            topics,
            journal,
            capture,
            decide,
            entry_action,
            tasks,
            complete_task,
            morning_plan,
            set_sync,
            sync_now,
            create_task,
            edit_task,
            create_topic,
            edit_topic,
            move_entry,
            reminders,
            snooze,
            calendar,
            weekly_review,
            settings,
            set_settings,
            ask,
            voice,
            undo,
            trash,
            purge,
            facts,
            add_fact,
            fact_action,
            attachments,
            upload_attachment
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        eprintln!("NotAI başlatılamadı: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_id_is_stable() {
        let dir = std::env::temp_dir().join(format!("notai-test-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = device_id(&dir).unwrap();
        let b = device_id(&dir).unwrap();
        assert_eq!(a, b);
        assert!(a.starts_with("cihaz-"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
