//! NotAI uygulama kabuğu (Tauri 2): Android + Windows/Linux masaüstü.
//!
//! Yerel öncelikli: tüm veri cihazdaki SQLite'tadır, uygulama internetsiz tam çalışır.
//! Ev sunucusu ayarlıysa arka planda dakikada bir senkronlanır (AI orada çalışır).

// Masaüstünde unsafe tamamen yasak. Mobilde Tauri'nin giriş noktası makrosu FFI
// yapıştırıcısı üretir (CLAUDE.md'de izin verilen tek istisna).
#![cfg_attr(not(mobile), forbid(unsafe_code))]

pub mod service;

use std::path::Path;
use std::time::Duration;

use chrono::Utc;
use chrono_tz::Tz;
use notai_api::{CaptureRequest, JournalView, MorningPlanView, TasksView, TopicsView};
use notai_store::Store;
use tauri::{Manager, State};

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
            sync_now
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
