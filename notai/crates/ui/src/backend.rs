//! Arka uç: Tauri kabuğunda yerel depoya (Tauri komutları), tarayıcıda demo verisine bağlanır.
//!
//! Arayüz iş mantığı içermez: tüm kurallar `notai-core`/`notai-store`'da; burada yalnız çağrı.

use std::sync::{Arc, Mutex};

use notai_api::{CaptureRequest, JournalView, MorningPlanView, TasksView, TopicsView};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;
use wasm_bindgen::prelude::*;

use crate::demo::Demo;

pub type BResult<T> = Result<T, String>;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

fn has_tauri() -> bool {
    web_sys::window()
        .and_then(|w| js_sys::Reflect::has(&w, &JsValue::from_str("__TAURI__")).ok())
        .unwrap_or(false)
}

#[derive(Clone)]
pub enum Backend {
    /// Tauri kabuğu: komutlar yerel SQLite deposunu kullanır (internetsiz çalışır).
    Tauri,
    /// Tarayıcı önizlemesi ve testler: bellekteki örnek veri.
    Demo(Arc<Mutex<Demo>>),
}

fn lock(d: &Arc<Mutex<Demo>>) -> std::sync::MutexGuard<'_, Demo> {
    d.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Backend {
    #[must_use]
    pub fn detect() -> Self {
        if has_tauri() {
            Self::Tauri
        } else {
            Self::demo()
        }
    }

    #[must_use]
    pub fn demo() -> Self {
        Self::Demo(Arc::new(Mutex::new(Demo::new())))
    }

    async fn call<R: DeserializeOwned>(cmd: &str, args: impl Serialize) -> BResult<R> {
        let ser = serde_wasm_bindgen::Serializer::json_compatible();
        let args = args.serialize(&ser).map_err(|e| e.to_string())?;
        let v = tauri_invoke(cmd, args)
            .await
            .map_err(|e| e.as_string().unwrap_or_else(|| "bilinmeyen hata".into()))?;
        serde_wasm_bindgen::from_value(v).map_err(|e| e.to_string())
    }

    pub async fn topics(&self, kind: &str) -> BResult<TopicsView> {
        match self {
            Self::Tauri => Self::call("topics", json!({ "kind": kind })).await,
            Self::Demo(d) => Ok(lock(d).topics(kind)),
        }
    }

    pub async fn journal(&self, topic: Option<String>) -> BResult<JournalView> {
        match self {
            Self::Tauri => Self::call("journal", json!({ "topicId": topic })).await,
            Self::Demo(d) => Ok(lock(d).journal(topic.as_deref())),
        }
    }

    pub async fn capture(&self, text: String, topic: Option<String>) -> BResult<()> {
        match self {
            Self::Tauri => {
                let req = CaptureRequest {
                    text,
                    topic_id: topic,
                };
                Self::call::<serde_json::Value>("capture", json!({ "req": req }))
                    .await
                    .map(|_| ())
            }
            Self::Demo(d) => lock(d).capture(&text, topic.as_deref()),
        }
    }

    pub async fn decide(&self, id: String, accept: bool) -> BResult<()> {
        match self {
            Self::Tauri => {
                Self::call::<serde_json::Value>("decide", json!({ "id": id, "accept": accept }))
                    .await
                    .map(|_| ())
            }
            Self::Demo(d) => lock(d).decide(&id, accept),
        }
    }

    /// archive | trash | restore | unarchive
    pub async fn entry_action(&self, id: String, action: &str) -> BResult<()> {
        match self {
            Self::Tauri => Self::call::<serde_json::Value>(
                "entry_action",
                json!({ "id": id, "action": action }),
            )
            .await
            .map(|_| ()),
            Self::Demo(d) => lock(d).entry_action(&id, action),
        }
    }

    pub async fn tasks(&self) -> BResult<TasksView> {
        match self {
            Self::Tauri => Self::call("tasks", json!({})).await,
            Self::Demo(d) => Ok(lock(d).tasks()),
        }
    }

    pub async fn complete_task(&self, id: String) -> BResult<()> {
        match self {
            Self::Tauri => Self::call::<serde_json::Value>("complete_task", json!({ "id": id }))
                .await
                .map(|_| ()),
            Self::Demo(d) => lock(d).complete_task(&id),
        }
    }

    pub async fn morning_plan(&self) -> BResult<MorningPlanView> {
        match self {
            Self::Tauri => Self::call("morning_plan", json!({})).await,
            Self::Demo(d) => Ok(lock(d).morning_plan()),
        }
    }
}
