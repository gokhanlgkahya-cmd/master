//! notai-server HTTP entegrasyon testleri (bellek içi SQLite + senaryolu LLM + elle saat).

// Yalnız test kodu: yardımcı fonksiyonlarda unwrap/panic serbest (üretim kodunda yasak).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{DateTime, Duration, TimeZone, Utc};
use http_body_util::BodyExt;
use notai_ai::{LlmClient, ScriptedLlm};
use notai_server::{AppState, Config, ManualClock, router, scheduler, worker};
use notai_store::Store;
use serde_json::{Value, json};
use tower::ServiceExt;
use wiremock::matchers::{method, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn t0() -> DateTime<Utc> {
    // Cuma 6 Kasım 2026, 09:12 İstanbul.
    Utc.with_ymd_and_hms(2026, 11, 6, 6, 12, 0).unwrap()
}

struct Harness {
    app: Router,
    state: AppState,
    clock: Arc<ManualClock>,
    llm: Arc<ScriptedLlm>,
}

async fn harness(cfg: Config, llm: ScriptedLlm) -> Harness {
    let store = Store::open_in_memory("sunucu").await.unwrap();
    let clock = Arc::new(ManualClock::new(t0()));
    let llm = Arc::new(llm);
    let state = AppState {
        store,
        llm: Some(llm.clone() as Arc<dyn LlmClient>),
        stt: None,
        embedder: None,
        cfg: Arc::new(cfg),
        clock: clock.clone(),
    };
    Harness {
        app: router(state.clone()),
        state,
        clock,
        llm,
    }
}

impl Harness {
    async fn call(
        &self,
        method: &str,
        uri: &str,
        body: Option<Value>,
        headers: &[(&str, &str)],
    ) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(uri);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let req = match body {
            Some(b) => req
                .header("content-type", "application/json")
                .body(Body::from(b.to_string()))
                .unwrap(),
            None => req.body(Body::empty()).unwrap(),
        };
        let resp = self.app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, json)
    }

    async fn get(&self, uri: &str) -> (StatusCode, Value) {
        self.call("GET", uri, None, &[]).await
    }

    async fn post(&self, uri: &str, body: Value) -> (StatusCode, Value) {
        self.call("POST", uri, Some(body), &[]).await
    }
}

#[tokio::test]
async fn health() {
    let h = harness(Config::defaults(), ScriptedLlm::new()).await;
    let (s, b) = h.get("/health").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["status"], "ok");
}

#[tokio::test]
async fn capture_classify_accept_flow() {
    let h0 = harness(Config::defaults(), ScriptedLlm::new()).await;
    // Konu oluştur.
    let (s, topic) = h0
        .post(
            "/api/topics",
            json!({ "name": "Kostik tedarik", "kind": "project" }),
        )
        .await;
    assert_eq!(s, StatusCode::CREATED);
    let topic_id = topic["id"].as_str().unwrap().to_owned();

    // Model bu konuyu ve bir görevi önerecek.
    let reply = json!({
        "topic": { "existing_id": topic_id, "new_name": null, "new_kind": null, "confidence": 0.9 },
        "tasks": [{ "title": "Kostik firmasını ara", "due_date": "2026-11-12", "due_time": "10:00",
                    "priority": "high", "confidence": 0.85 }],
        "tags": [], "people": [], "reminder": null
    });
    let h = Harness {
        llm: Arc::new(ScriptedLlm::new().reply(reply.to_string())),
        ..h0
    };
    let state = AppState {
        llm: Some(h.llm.clone() as Arc<dyn LlmClient>),
        ..h.state.clone()
    };
    let h = Harness {
        app: router(state.clone()),
        state,
        ..h
    };

    // Yakala → hemen gelen kutusunda (AI'ı beklemez).
    let (s, e) = h
        .post(
            "/api/entries",
            json!({ "text": "Kostik tedarikçisini ara – perşembe" }),
        )
        .await;
    assert_eq!(s, StatusCode::CREATED);
    let entry_id = e["id"].as_str().unwrap().to_owned();
    let (_, j) = h.get("/api/journal").await;
    assert_eq!(j["unprocessed"], 1);
    assert_eq!(j["groups"][0]["label"], "Bugün");
    assert_eq!(j["groups"][0]["items"][0]["time"], "09:12");
    assert_eq!(j["groups"][0]["items"][0]["unprocessed"], true);

    // AI işçisi kuyruğu işler.
    assert!(worker::process_one(&h.state).await.unwrap());
    assert!(!worker::process_one(&h.state).await.unwrap(), "kuyruk boş");
    let req = &h.llm.requests()[0];
    assert!(req.system.contains("Kostik tedarik"));

    let (_, j) = h.get("/api/journal").await;
    let chips: Vec<_> = j["groups"][0]["items"][0]["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["chip"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(chips, ["→ Kostik tedarik", "Görev · Perşembe 12 Kas 10:00"]);
    let ids: Vec<String> = j["groups"][0]["items"][0]["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_owned())
        .collect();

    // Görev önerisini kabul et → görev oluşur.
    let (s, out) = h
        .post(&format!("/api/suggestions/{}/accept", ids[1]), json!({}))
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(out["created_task"]["title"], "Kostik firmasını ara");
    // Konu önerisini yoksay → tüm öneriler karara bağlandı → işlendi.
    let (s, out) = h
        .post(&format!("/api/suggestions/{}/dismiss", ids[0]), json!({}))
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(out["entry_status"], "processed");
    // İkinci karar çakışmadır.
    let (s, _) = h
        .post(&format!("/api/suggestions/{}/accept", ids[0]), json!({}))
        .await;
    assert_eq!(s, StatusCode::CONFLICT);

    let (_, tasks) = h.get("/api/tasks").await;
    assert_eq!(tasks["upcoming"][0]["title"], "Kostik firmasını ara");
    assert_eq!(tasks["upcoming"][0]["priority_label"], "Yüksek");

    let (s, detail) = h.get(&format!("/api/entries/{entry_id}")).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(detail["suggestions"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn ai_failure_keeps_entry_and_marks_failed() {
    let h = harness(
        Config::defaults(),
        ScriptedLlm::new()
            .reply("çöp")
            .reply("çöp")
            .reply("çöp")
            .reply("çöp"),
    )
    .await;
    h.post(
        "/api/entries",
        json!({ "text": "Şirket aracının muayenesi ne zamandı?" }),
    )
    .await;
    for mins in [0, 1, 6, 31] {
        h.clock.set(t0() + Duration::minutes(mins));
        assert!(worker::process_one(&h.state).await.unwrap());
    }
    let (_, j) = h.get("/api/journal").await;
    let item = &j["groups"][0]["items"][0];
    assert_eq!(item["ai_failed"], true);
    assert_eq!(item["unprocessed"], true);
    assert_eq!(item["text"], "Şirket aracının muayenesi ne zamandı?");
}

#[tokio::test]
async fn validation_and_not_found_errors_are_json() {
    let h = harness(Config::defaults(), ScriptedLlm::new()).await;
    let (s, b) = h.post("/api/entries", json!({ "text": "   " })).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(b["error"].as_str().unwrap().contains("boş"));
    let (s, _) = h
        .get("/api/entries/00000000-0000-7000-8000-000000000000")
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, _) = h.get("/api/entries/kostik").await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = h
        .post(
            "/api/topics",
            json!({ "name": "Ev", "kind": "area", "due_date": "2026-12-01" }),
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    h.post("/api/topics", json!({ "name": "Ev", "kind": "area" }))
        .await;
    let (s, _) = h
        .post("/api/topics", json!({ "name": "EV", "kind": "area" }))
        .await;
    assert_eq!(s, StatusCode::CONFLICT);
    let (s, _) = h.get("/api/journal?filter=topic").await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn api_token_is_enforced_but_health_is_open() {
    let mut cfg = Config::defaults();
    cfg.api_token = Some("s24-gizli".into());
    let h = harness(cfg, ScriptedLlm::new()).await;
    assert_eq!(h.get("/health").await.0, StatusCode::OK);
    assert_eq!(h.get("/api/journal").await.0, StatusCode::UNAUTHORIZED);
    let (s, _) = h
        .call(
            "GET",
            "/api/journal",
            None,
            &[("authorization", "Bearer yanlis")],
        )
        .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = h
        .call(
            "GET",
            "/api/journal",
            None,
            &[("authorization", "Bearer s24-gizli")],
        )
        .await;
    assert_eq!(s, StatusCode::OK);
}

#[tokio::test]
async fn whatsapp_webhook_requires_secret_and_dedupes() {
    let mut cfg = Config::defaults();
    cfg.webhook_secret = Some("macrodroid".into());
    let h = harness(cfg, ScriptedLlm::new()).await;
    let body = json!({ "text": "Numune yolla – GemOlive", "sent_at": "2026-11-06T06:20:00Z" });

    let (s, _) = h
        .call("POST", "/webhook/whatsapp", Some(body.clone()), &[])
        .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, first) = h
        .call(
            "POST",
            "/webhook/whatsapp",
            Some(body.clone()),
            &[("x-notai-secret", "macrodroid")],
        )
        .await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(first["duplicate"], false);
    let (s, second) = h
        .call(
            "POST",
            "/webhook/whatsapp",
            Some(body),
            &[("x-notai-secret", "macrodroid")],
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(second["duplicate"], true);
    assert_eq!(first["id"], second["id"]);

    let (_, j) = h.get("/api/journal").await;
    assert_eq!(j["groups"][0]["items"][0]["source"], "whats_app");
    assert_eq!(j["unprocessed"], 1);
}

#[tokio::test]
async fn webhook_disabled_without_secret() {
    let h = harness(Config::defaults(), ScriptedLlm::new()).await;
    let (s, _) = h
        .call(
            "POST",
            "/webhook/whatsapp",
            Some(json!({ "text": "x" })),
            &[("x-notai-secret", "")],
        )
        .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn morning_plan_scheduler_fires_once_and_notifies() {
    let ntfy = MockServer::start().await;
    Mock::given(method("POST"))
        .and(query_param(
            "title",
            "Günaydın Gökhan. Bugün 1 iş öne çıkıyor",
        ))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&ntfy)
        .await;
    let mut cfg = Config::defaults();
    cfg.ntfy_url = Some(format!("{}/notai", ntfy.uri()));
    let h = harness(cfg, ScriptedLlm::new()).await;
    h.post(
        "/api/tasks",
        json!({ "title": "Logo: 37 hatalı stok kartını düzelt", "due_date": "2026-11-06" }),
    )
    .await;

    // 08:00 İstanbul: henüz değil.
    h.clock
        .set(Utc.with_ymd_and_hms(2026, 11, 6, 5, 0, 0).unwrap());
    assert!(!scheduler::tick(&h.state).await.unwrap());
    // 09:01 İstanbul (varsayılan sabah saati 09:00): gönder, sonra aynı gün tekrar gönderme.
    h.clock
        .set(Utc.with_ymd_and_hms(2026, 11, 6, 6, 1, 0).unwrap());
    assert!(scheduler::tick(&h.state).await.unwrap());
    assert!(!scheduler::tick(&h.state).await.unwrap());

    let (_, plan) = h.get("/api/morning-plan").await;
    assert_eq!(plan["headline"], "Günaydın Gökhan. Bugün 1 iş öne çıkıyor");
    assert_eq!(plan["items"][0]["reason"], "bugün");
    let latest = h
        .state
        .store
        .kv_get(scheduler::LATEST_PLAN_KEY)
        .await
        .unwrap()
        .unwrap();
    assert!(latest.contains("Logo: 37 hatalı stok kartını düzelt"));
}

#[tokio::test]
async fn entry_actions_and_topics_list() {
    let h = harness(Config::defaults(), ScriptedLlm::new()).await;
    let (_, gem) = h
        .post(
            "/api/topics",
            json!({ "name": "GemOlive", "kind": "area", "icon": "drop" }),
        )
        .await;
    let gem_id = gem["id"].as_str().unwrap();
    let (_, e) = h
        .post(
            "/api/entries",
            json!({ "text": "Ambalaj faturası · 1.850 ₺" }),
        )
        .await;
    let id = e["id"].as_str().unwrap();

    let (s, moved) = h
        .post(
            &format!("/api/entries/{id}/move"),
            json!({ "topic_id": gem_id }),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(moved["status"]["state"], "processed");

    let (_, topics) = h.get("/api/topics").await;
    assert_eq!(topics["topics"][0]["name"], "GemOlive");
    assert_eq!(topics["topics"][0]["last_label"], "09:12");
    assert_eq!(topics["topics"][0]["preview"], "Ambalaj faturası · 1.850 ₺");
    assert_eq!(topics["journal_preview"], "Ambalaj faturası · 1.850 ₺");
    assert_eq!(topics["journal_last_label"], "09:12");

    let (_, flow) = h
        .get(&format!("/api/journal?filter=topic&topic_id={gem_id}"))
        .await;
    assert_eq!(flow["groups"][0]["items"].as_array().unwrap().len(), 1);

    let (s, _) = h.post(&format!("/api/entries/{id}/trash"), json!({})).await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = h.post(&format!("/api/entries/{id}/trash"), json!({})).await;
    assert_eq!(s, StatusCode::CONFLICT);
    let (s, _) = h
        .post(&format!("/api/entries/{id}/restore"), json!({}))
        .await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = h
        .post(&format!("/api/entries/{id}/patlat"), json!({}))
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    let (_, found) = h.get("/api/search?q=fatura").await;
    assert_eq!(found["results"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn http_sync_between_phone_and_real_server() {
    use notai_core::entry::Source;
    use notai_core::text::EntryText;
    use notai_store::NewEntry;

    let mut cfg = Config::defaults();
    cfg.api_token = Some("s24".into());
    let h = harness(cfg, ScriptedLlm::new()).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = h.app.clone();
    tokio::spawn(async move { axum::serve(listener, app).await });

    let phone = Store::open_in_memory("telefon").await.unwrap();
    phone
        .capture(NewEntry {
            text: EntryText::new("Telefondan: numune yolla").unwrap(),
            source: Source::App,
            topic: None,
            captured_at: t0(),
            dedupe_key: None,
        })
        .await
        .unwrap();

    // Yanlış belirteç reddedilir.
    let bad = notai_sync::Http::new(format!("http://{addr}"), Some("yanlis".into())).unwrap();
    assert!(matches!(
        notai_sync::sync_once(&phone, &bad).await,
        Err(notai_sync::SyncError::Status { status: 401, .. })
    ));

    let http = notai_sync::Http::new(format!("http://{addr}"), Some("s24".into())).unwrap();
    let r = notai_sync::sync_once(&phone, &http).await.unwrap();
    assert_eq!(r.pushed, 1);
    let (_, j) = h
        .call(
            "GET",
            "/api/journal",
            None,
            &[("authorization", "Bearer s24")],
        )
        .await;
    assert_eq!(
        j["groups"][0]["items"][0]["text"],
        "Telefondan: numune yolla"
    );

    // Sunucuya WhatsApp'tan gelen kayıt telefona iner.
    h.call(
        "POST",
        "/api/entries",
        Some(json!({ "text": "Sunucuda yakalandı" })),
        &[("authorization", "Bearer s24")],
    )
    .await;
    let r = notai_sync::sync_once(&phone, &http).await.unwrap();
    assert_eq!(r.pulled_applied, 1);
}
