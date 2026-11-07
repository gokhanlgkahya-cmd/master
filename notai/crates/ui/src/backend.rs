//! Arka uç: üç yol, aynı sözleşme.
//! - **Tauri** (uygulama): yerel SQLite deposu; AI/ses gerektirenler ev sunucusuna iletilir.
//! - **Http** (tarayıcıda `?api=http://sunucu:8787&token=…`): doğrudan `notai-server`.
//! - **Demo** (tarayıcı önizlemesi): bellekteki örnek veri (yalnız v0.1 ekranları).
//!
//! Arayüz iş mantığı içermez: tüm kurallar `notai-core`/`notai-store`'da; burada yalnız çağrı.
//! Tauri komut adları bu yöntemlerin adlarıyla aynıdır (`crates/app/src/lib.rs`).

use std::sync::{Arc, Mutex};

use notai_api::{
    AskView, AttachmentView, CalendarView, CaptureRequest, FactView, JournalView, MorningPlanView,
    RemindersView, SettingsView, TaskEditRequest, TasksView, TopicEditRequest, TopicsView,
    TrashItemView, WeeklyReviewView,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
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

/// `?api=…&token=…` sorgu parametreleri (tarayıcıda sunucuya doğrudan bağlanmak için).
fn query_param(name: &str) -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    search.trim_start_matches('?').split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == name).then(|| {
            urlencoding::decode(v)
                .map(|s| s.into_owned())
                .unwrap_or_default()
        })
    })
}

/// `notai-server` HTTP istemcisi.
#[derive(Clone, Debug)]
pub struct Http {
    base: String,
    token: Option<String>,
}

impl Http {
    fn req(&self, method: &str, path: &str) -> gloo_net::http::RequestBuilder {
        let url = format!("{}/api{path}", self.base);
        let b = match method {
            "POST" => gloo_net::http::Request::post(&url),
            _ => gloo_net::http::Request::get(&url),
        };
        match &self.token {
            Some(t) => b.header("Authorization", &format!("Bearer {t}")),
            None => b,
        }
    }

    async fn read<R: DeserializeOwned>(resp: gloo_net::http::Response) -> BResult<R> {
        if !resp.ok() {
            let body: Value = resp.json().await.unwrap_or(Value::Null);
            return Err(body["error"].as_str().map_or_else(
                || format!("sunucu hatası ({})", resp.status()),
                str::to_owned,
            ));
        }
        if resp.status() == 204 {
            return serde_json::from_value(Value::Null).map_err(|e| e.to_string());
        }
        resp.json().await.map_err(|e| e.to_string())
    }

    async fn get<R: DeserializeOwned>(&self, path: &str) -> BResult<R> {
        Self::read(
            self.req("GET", path)
                .send()
                .await
                .map_err(|e| e.to_string())?,
        )
        .await
    }

    async fn post<R: DeserializeOwned>(&self, path: &str, body: &impl Serialize) -> BResult<R> {
        let r = self
            .req("POST", path)
            .json(body)
            .map_err(|e| e.to_string())?;
        Self::read(r.send().await.map_err(|e| e.to_string())?).await
    }

    async fn post_bytes<R: DeserializeOwned>(
        &self,
        path: &str,
        mime: &str,
        bytes: &[u8],
    ) -> BResult<R> {
        let arr = js_sys::Uint8Array::from(bytes);
        let r = self
            .req("POST", path)
            .header("Content-Type", mime)
            .body(arr)
            .map_err(|e| e.to_string())?;
        Self::read(r.send().await.map_err(|e| e.to_string())?).await
    }
}

#[derive(Clone)]
pub enum Backend {
    Tauri,
    Http(Http),
    Demo(Arc<Mutex<Demo>>),
}

fn lock(d: &Arc<Mutex<Demo>>) -> std::sync::MutexGuard<'_, Demo> {
    d.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(crate) const NOT_IN_DEMO: &str =
    "Bu özellik önizlemede yok: uygulamada ya da ?api=… ile sunucuya bağlanarak dene.";

fn enc(s: &str) -> String {
    urlencoding::encode(s).into_owned()
}

impl Backend {
    #[must_use]
    pub fn detect() -> Self {
        if has_tauri() {
            Self::Tauri
        } else if let Some(base) = query_param("api") {
            Self::Http(Http {
                base: base.trim_end_matches('/').to_owned(),
                token: query_param("token"),
            })
        } else {
            Self::demo()
        }
    }

    #[must_use]
    pub fn demo() -> Self {
        Self::Demo(Arc::new(Mutex::new(Demo::new())))
    }

    #[must_use]
    pub const fn is_demo(&self) -> bool {
        matches!(self, Self::Demo(_))
    }

    async fn call<R: DeserializeOwned>(cmd: &str, args: impl Serialize) -> BResult<R> {
        let ser = serde_wasm_bindgen::Serializer::json_compatible();
        let args = args.serialize(&ser).map_err(|e| e.to_string())?;
        let v = tauri_invoke(cmd, args)
            .await
            .map_err(|e| e.as_string().unwrap_or_else(|| "bilinmeyen hata".into()))?;
        serde_wasm_bindgen::from_value(v).map_err(|e| e.to_string())
    }

    // ---------------- v0.1 ----------------

    pub async fn topics(&self, kind: &str) -> BResult<TopicsView> {
        match self {
            Self::Tauri => Self::call("topics", json!({ "kind": kind })).await,
            Self::Http(h) => h.get(&format!("/topics?kind={kind}")).await,
            Self::Demo(d) => Ok(lock(d).topics(kind)),
        }
    }

    pub async fn journal(&self, topic: Option<String>) -> BResult<JournalView> {
        match self {
            Self::Tauri => Self::call("journal", json!({ "topicId": topic })).await,
            Self::Http(h) => match topic {
                Some(t) => {
                    h.get(&format!("/journal?filter=topic&topic_id={t}&limit=100"))
                        .await
                }
                None => h.get("/journal?limit=100").await,
            },
            Self::Demo(d) => Ok(lock(d).journal(topic.as_deref())),
        }
    }

    pub async fn capture(&self, text: String, topic: Option<String>) -> BResult<()> {
        let req = CaptureRequest {
            text,
            topic_id: topic,
        };
        match self {
            Self::Tauri => Self::call::<Value>("capture", json!({ "req": req }))
                .await
                .map(|_| ()),
            Self::Http(h) => h.post::<Value>("/entries", &req).await.map(|_| ()),
            Self::Demo(d) => lock(d).capture(&req.text, req.topic_id.as_deref()),
        }
    }

    /// `capture` ile aynı; yakalanan kaydın kimliğini döndürür (ek yüklemek için).
    /// Tauri komutu kimliği (String), sunucu `Entry` (`id` alanı) döndürür.
    pub async fn capture_id(&self, text: String, topic: Option<String>) -> BResult<String> {
        let req = CaptureRequest {
            text,
            topic_id: topic,
        };
        match self {
            Self::Tauri => Self::call::<String>("capture", json!({ "req": req })).await,
            Self::Http(h) => {
                let v: Value = h.post("/entries", &req).await?;
                v["id"]
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| "sunucu kayıt kimliği döndürmedi".to_owned())
            }
            Self::Demo(d) => {
                let mut demo = lock(d);
                demo.capture(&req.text, req.topic_id.as_deref())?;
                demo.last_entry_id()
                    .ok_or_else(|| "kayıt bulunamadı".to_owned())
            }
        }
    }

    /// Ek yüklenebilir mi? (Önizleme verisi ek saklamaz.)
    #[must_use]
    pub const fn supports_attachments(&self) -> bool {
        !matches!(self, Self::Demo(_))
    }

    /// Bağlantı türünün kısa açıklaması (Ayarlar ekranı için).
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Tauri => "Uygulama · yerel veritabanı".to_owned(),
            Self::Http(h) => format!("Doğrudan sunucu · {}", h.base),
            Self::Demo(_) => "Önizleme · örnek veri".to_owned(),
        }
    }

    pub async fn decide(&self, id: String, accept: bool) -> BResult<()> {
        match self {
            Self::Tauri => Self::call::<Value>("decide", json!({ "id": id, "accept": accept }))
                .await
                .map(|_| ()),
            Self::Http(h) => {
                let d = if accept { "accept" } else { "dismiss" };
                h.post::<Value>(&format!("/suggestions/{id}/{d}"), &json!({}))
                    .await
                    .map(|_| ())
            }
            Self::Demo(d) => lock(d).decide(&id, accept),
        }
    }

    /// archive | trash | restore | unarchive
    pub async fn entry_action(&self, id: String, action: &str) -> BResult<()> {
        match self {
            Self::Tauri => {
                Self::call::<Value>("entry_action", json!({ "id": id, "action": action }))
                    .await
                    .map(|_| ())
            }
            Self::Http(h) => h
                .post::<Value>(&format!("/entries/{id}/{action}"), &json!({}))
                .await
                .map(|_| ()),
            Self::Demo(d) => lock(d).entry_action(&id, action),
        }
    }

    pub async fn tasks(&self) -> BResult<TasksView> {
        match self {
            Self::Tauri => Self::call("tasks", json!({})).await,
            Self::Http(h) => h.get("/tasks").await,
            Self::Demo(d) => Ok(lock(d).tasks()),
        }
    }

    pub async fn complete_task(&self, id: String) -> BResult<()> {
        match self {
            Self::Tauri => Self::call::<Value>("complete_task", json!({ "id": id }))
                .await
                .map(|_| ()),
            Self::Http(h) => h
                .post::<Value>(&format!("/tasks/{id}/complete"), &json!({}))
                .await
                .map(|_| ()),
            Self::Demo(d) => lock(d).complete_task(&id),
        }
    }

    pub async fn morning_plan(&self) -> BResult<MorningPlanView> {
        match self {
            Self::Tauri => Self::call("morning_plan", json!({})).await,
            Self::Http(h) => h.get("/morning-plan").await,
            Self::Demo(d) => Ok(lock(d).morning_plan()),
        }
    }

    // ---------------- v0.2: görev, konu, hatırlatma, takvim, ayarlar ----------------

    pub async fn create_task(
        &self,
        title: String,
        due_date: Option<String>,
        topic: Option<String>,
    ) -> BResult<String> {
        let body = json!({ "title": title, "due_date": due_date, "topic_id": topic });
        match self {
            // Kabuk kimliği (String), sunucu `Task` (`id` alanı) döndürür.
            Self::Tauri => Self::call::<String>("create_task", json!({ "req": body })).await,
            Self::Http(h) => {
                let v: Value = h.post("/tasks", &body).await?;
                v["id"]
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| "sunucu görev kimliği döndürmedi".to_owned())
            }
            Self::Demo(_) => Err(NOT_IN_DEMO.into()),
        }
    }

    pub async fn edit_task(&self, id: String, req: TaskEditRequest) -> BResult<()> {
        match self {
            Self::Tauri => Self::call::<Value>("edit_task", json!({ "id": id, "req": req }))
                .await
                .map(|_| ()),
            Self::Http(h) => h
                .post::<Value>(&format!("/tasks/{id}/edit"), &req)
                .await
                .map(|_| ()),
            Self::Demo(_) => Err(NOT_IN_DEMO.into()),
        }
    }

    pub async fn create_topic(&self, name: String, kind: String) -> BResult<()> {
        let body = json!({ "name": name, "kind": kind });
        match self {
            Self::Tauri => {
                Self::call::<Value>("create_topic", json!({ "name": name, "kind": kind }))
                    .await
                    .map(|_| ())
            }
            Self::Http(h) => h.post::<Value>("/topics", &body).await.map(|_| ()),
            Self::Demo(_) => Err(NOT_IN_DEMO.into()),
        }
    }

    pub async fn edit_topic(&self, id: String, req: TopicEditRequest) -> BResult<()> {
        match self {
            Self::Tauri => Self::call::<Value>("edit_topic", json!({ "id": id, "req": req }))
                .await
                .map(|_| ()),
            Self::Http(h) => h
                .post::<Value>(&format!("/topics/{id}/edit"), &req)
                .await
                .map(|_| ()),
            Self::Demo(_) => Err(NOT_IN_DEMO.into()),
        }
    }

    /// Kaydı başka konuya taşır (`None`: Ana Günlük'e geri).
    pub async fn move_entry(&self, id: String, topic: Option<String>) -> BResult<()> {
        match self {
            Self::Tauri => Self::call::<Value>("move_entry", json!({ "id": id, "topicId": topic }))
                .await
                .map(|_| ()),
            Self::Http(h) => h
                .post::<Value>(
                    &format!("/entries/{id}/move"),
                    &json!({ "topic_id": topic }),
                )
                .await
                .map(|_| ()),
            Self::Demo(_) => Err(NOT_IN_DEMO.into()),
        }
    }

    pub async fn reminders(&self) -> BResult<RemindersView> {
        match self {
            Self::Tauri => Self::call("reminders", json!({})).await,
            Self::Http(h) => h.get("/reminders").await,
            Self::Demo(_) => Ok(RemindersView::default()),
        }
    }

    /// minutes10 | hour1 | tomorrow
    pub async fn snooze(&self, task: String, option: &str) -> BResult<()> {
        match self {
            Self::Tauri => Self::call::<Value>("snooze", json!({ "id": task, "option": option }))
                .await
                .map(|_| ()),
            Self::Http(h) => h
                .post::<Value>(
                    &format!("/tasks/{task}/snooze"),
                    &json!({ "option": option }),
                )
                .await
                .map(|_| ()),
            Self::Demo(_) => Err(NOT_IN_DEMO.into()),
        }
    }

    /// `from`: "YYYY-AA-GG" (yoksa bu haftanın pazartesisi).
    pub async fn calendar(&self, from: Option<String>, days: u32) -> BResult<CalendarView> {
        match self {
            Self::Tauri => Self::call("calendar", json!({ "from": from, "days": days })).await,
            Self::Http(h) => match from {
                Some(f) => h.get(&format!("/calendar?from={f}&days={days}")).await,
                None => h.get(&format!("/calendar?days={days}")).await,
            },
            Self::Demo(_) => Ok(CalendarView::default()),
        }
    }

    pub async fn settings(&self) -> BResult<SettingsView> {
        match self {
            Self::Tauri => Self::call("settings", json!({})).await,
            Self::Http(h) => h.get("/settings").await,
            Self::Demo(_) => Err(NOT_IN_DEMO.into()),
        }
    }

    pub async fn set_settings(
        &self,
        autonomous: Option<bool>,
        threshold: Option<f64>,
    ) -> BResult<()> {
        let body = json!({ "autonomous": autonomous, "threshold": threshold });
        match self {
            Self::Tauri => Self::call::<Value>(
                "set_settings",
                json!({ "autonomous": autonomous, "threshold": threshold }),
            )
            .await
            .map(|_| ()),
            Self::Http(h) => h.post::<Value>("/settings", &body).await.map(|_| ()),
            Self::Demo(_) => Err(NOT_IN_DEMO.into()),
        }
    }

    /// Yalnız uygulamada: ev sunucusu adresi + belirteç (senkron ve AI/ses iletimi için).
    pub async fn set_sync(&self, url: String, token: String) -> BResult<()> {
        match self {
            Self::Tauri => Self::call::<Value>("set_sync", json!({ "url": url, "token": token }))
                .await
                .map(|_| ()),
            _ => Err("Senkron ayarı yalnız uygulamada.".into()),
        }
    }

    pub async fn sync_now(&self) -> BResult<String> {
        match self {
            Self::Tauri => Self::call::<Option<(u32, u32)>>("sync_now", json!({}))
                .await
                .map(|r| {
                    r.map_or_else(
                        || "Sunucu ayarlı değil".into(),
                        |(p, g)| format!("{p} gönderildi · {g} alındı"),
                    )
                }),
            _ => Ok("Tarayıcıda senkron gerekmez".into()),
        }
    }

    pub async fn trash(&self) -> BResult<Vec<TrashItemView>> {
        match self {
            Self::Tauri => Self::call("trash", json!({})).await,
            Self::Http(h) => h.get("/trash").await,
            Self::Demo(_) => Ok(Vec::new()),
        }
    }

    pub async fn purge(&self, id: String) -> BResult<()> {
        match self {
            Self::Tauri => Self::call::<Value>("purge", json!({ "id": id }))
                .await
                .map(|_| ()),
            Self::Http(h) => h
                .post::<Value>(&format!("/entries/{id}/purge"), &json!({}))
                .await
                .map(|_| ()),
            Self::Demo(_) => Err(NOT_IN_DEMO.into()),
        }
    }

    pub async fn facts(&self) -> BResult<Vec<FactView>> {
        match self {
            Self::Tauri => Self::call("facts", json!({})).await,
            Self::Http(h) => h.get("/facts").await,
            Self::Demo(_) => Ok(Vec::new()),
        }
    }

    pub async fn add_fact(&self, text: String) -> BResult<()> {
        match self {
            Self::Tauri => Self::call::<Value>("add_fact", json!({ "text": text }))
                .await
                .map(|_| ()),
            Self::Http(h) => h
                .post::<Value>("/facts", &json!({ "text": text }))
                .await
                .map(|_| ()),
            Self::Demo(_) => Err(NOT_IN_DEMO.into()),
        }
    }

    /// confirm | delete
    pub async fn fact_action(&self, id: String, action: &str) -> BResult<()> {
        match self {
            Self::Tauri => {
                Self::call::<Value>("fact_action", json!({ "id": id, "action": action }))
                    .await
                    .map(|_| ())
            }
            Self::Http(h) => h
                .post::<Value>(&format!("/facts/{id}/{action}"), &json!({}))
                .await
                .map(|_| ()),
            Self::Demo(_) => Err(NOT_IN_DEMO.into()),
        }
    }

    // ---------------- v0.3: ses ve ekler ----------------

    pub async fn attachments(&self, entry: String) -> BResult<Vec<AttachmentView>> {
        match self {
            Self::Tauri => Self::call("attachments", json!({ "id": entry })).await,
            Self::Http(h) => h.get(&format!("/entries/{entry}/attachments")).await,
            Self::Demo(_) => Ok(Vec::new()),
        }
    }

    /// kind: photo | document | location | contact
    pub async fn upload_attachment(
        &self,
        entry: String,
        kind: &str,
        name: String,
        mime: String,
        bytes: Vec<u8>,
        value: Option<String>,
    ) -> BResult<()> {
        match self {
            Self::Tauri => Self::call::<Value>(
                "upload_attachment",
                json!({ "id": entry, "kind": kind, "name": name, "mime": mime, "data": bytes, "value": value }),
            )
            .await
            .map(|_| ()),
            Self::Http(h) => {
                let mut path = format!("/entries/{entry}/attachments?kind={kind}&name={}", enc(&name));
                if let Some(v) = &value {
                    path.push_str(&format!("&value={}", enc(v)));
                }
                h.post_bytes::<Value>(&path, &mime, &bytes).await.map(|_| ())
            }
            Self::Demo(_) => Err(NOT_IN_DEMO.into()),
        }
    }

    /// Ses kaydını gönderir: sunucu yazıya çevirip kayıt olarak yakalar.
    pub async fn voice(&self, bytes: Vec<u8>, mime: String, topic: Option<String>) -> BResult<()> {
        match self {
            Self::Tauri => Self::call::<Value>(
                "voice",
                json!({ "data": bytes, "mime": mime, "topicId": topic }),
            )
            .await
            .map(|_| ()),
            Self::Http(h) => {
                let path =
                    topic.map_or_else(|| "/voice".to_owned(), |t| format!("/voice?topic_id={t}"));
                h.post_bytes::<Value>(&path, &mime, &bytes)
                    .await
                    .map(|_| ())
            }
            Self::Demo(_) => Err(NOT_IN_DEMO.into()),
        }
    }

    // ---------------- v0.4: Ara/Sor, gözden geçirme, geri alma ----------------

    pub async fn ask(&self, question: String) -> BResult<AskView> {
        match self {
            Self::Tauri => Self::call("ask", json!({ "question": question })).await,
            Self::Http(h) => h.post("/ask", &json!({ "question": question })).await,
            Self::Demo(_) => Err(NOT_IN_DEMO.into()),
        }
    }

    pub async fn weekly_review(&self) -> BResult<WeeklyReviewView> {
        match self {
            Self::Tauri => Self::call("weekly_review", json!({})).await,
            Self::Http(h) => h.get("/weekly-review").await,
            Self::Demo(_) => Err(NOT_IN_DEMO.into()),
        }
    }

    /// AI sorusunu cevaplar (kayıt cevapla yeniden sınıflandırılır).
    pub async fn answer_question(&self, suggestion: String, text: String) -> BResult<()> {
        match self {
            Self::Tauri => {
                Self::call::<Value>("answer_question", json!({ "id": suggestion, "text": text }))
                    .await
                    .map(|_| ())
            }
            Self::Http(h) => h
                .post::<Value>(
                    &format!("/suggestions/{suggestion}/answer"),
                    &json!({ "text": text }),
                )
                .await
                .map(|_| ()),
            Self::Demo(_) => Err(NOT_IN_DEMO.into()),
        }
    }

    /// Son bildirim (uygulama açılınca gösterilen şerit). Önizlemede yok.
    pub async fn notice(&self) -> BResult<Option<notai_api::NoticeView>> {
        match self {
            Self::Tauri => Self::call("notice", json!({})).await,
            Self::Http(h) => h.get("/notice").await,
            Self::Demo(_) => Ok(None),
        }
    }

    pub async fn clear_notice(&self) -> BResult<()> {
        match self {
            Self::Tauri => Self::call::<Value>("clear_notice", json!({}))
                .await
                .map(|_| ()),
            Self::Http(h) => h
                .post::<Value>("/notice/clear", &json!({}))
                .await
                .map(|_| ()),
            Self::Demo(_) => Ok(()),
        }
    }

    /// Otonom modun uyguladığı öneriyi geri alır.
    pub async fn undo(&self, suggestion: String) -> BResult<()> {
        match self {
            Self::Tauri => Self::call::<Value>("undo", json!({ "id": suggestion }))
                .await
                .map(|_| ()),
            Self::Http(h) => h
                .post::<Value>(&format!("/suggestions/{suggestion}/undo"), &json!({}))
                .await
                .map(|_| ()),
            Self::Demo(_) => Err(NOT_IN_DEMO.into()),
        }
    }
}
