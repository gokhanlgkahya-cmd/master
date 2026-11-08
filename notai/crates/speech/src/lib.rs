//! Android konuşma tanıma eklentisi (Gökhan, 2026-11-08: "ses girişini metne çevirip kaydetsin").
//!
//! Komutlar yerel (Kotlin) taraftadır; arayüz doğrudan çağırır:
//! - `plugin:notai-speech|start` — mikrofon izni ister, tanımayı (ve Android 13+'ta ses kaydını) başlatır.
//! - `plugin:notai-speech|stop` — bitirir: `{ text, path?, error? }` (path: önbellekteki WAV).
//! - `plugin:notai-speech|cancel` — vazgeçer.
//! - `partial` olayı — konuşurken anlık metin.
//!
//! Masaüstünde yerel taraf yoktur; arayüz tarayıcı kaydına (MediaRecorder) düşer.
#![forbid(unsafe_code)]

#[cfg(target_os = "android")]
use tauri::Manager;
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Android tarafının tutamacı (uygulama boyunca yaşar).
#[cfg(target_os = "android")]
struct Speech<R: Runtime>(tauri::plugin::PluginHandle<R>);

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("notai-speech")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle = api.register_android_plugin("app.notai.speech", "SpeechPlugin")?;
                app.manage(Speech(handle));
            }
            #[cfg(not(target_os = "android"))]
            {
                let _ = (app, api);
            }
            Ok(())
        })
        .build()
}
