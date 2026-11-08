//! v0.2–v0.4 uçlarının HTTP entegrasyon testleri.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{DateTime, Duration, TimeZone, Utc};
use http_body_util::BodyExt;
use notai_ai::media::{AiFuture, Embedder, Transcriber};
use notai_ai::{LlmClient, ScriptedLlm};
use notai_server::{AppState, Config, ManualClock, router, scheduler, worker};
use notai_store::Store;
use serde_json::{Value, json};
use tower::ServiceExt;
use wiremock::matchers::{method, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 6, 6, 12, 0).unwrap()
}

struct FakeStt;
impl Transcriber for FakeStt {
    fn transcribe<'a>(&'a self, audio: &'a [u8], _mime: &'a str) -> AiFuture<'a, String> {
        let n = audio.len();
        Box::pin(async move { Ok(format!("Yarın İzmir'deki kostik firmasını ara ({n} bayt)")) })
    }
}

/// "kostik" geçen metinleri bir yöne, diğerlerini başka yöne koyan sahte vektörler.
struct FakeEmbed;
impl Embedder for FakeEmbed {
    fn model(&self) -> &str {
        "sahte"
    }
    fn embed<'a>(&'a self, texts: &'a [String]) -> AiFuture<'a, Vec<Vec<f32>>> {
        Box::pin(async move {
            Ok(texts
                .iter()
                .map(|t| {
                    if t.to_lowercase().contains("naoh") || t.contains("fiyat") {
                        vec![1.0, 0.0]
                    } else {
                        vec![0.0, 1.0]
                    }
                })
                .collect())
        })
    }
}

struct H {
    app: Router,
    state: AppState,
    clock: Arc<ManualClock>,
}

async fn h(cfg: Config, llm: ScriptedLlm, media: bool) -> H {
    let store = Store::open_in_memory("sunucu").await.unwrap();
    let clock = Arc::new(ManualClock::new(t0()));
    let state = AppState {
        store,
        llm: Some(Arc::new(llm) as Arc<dyn LlmClient>),
        stt: media.then(|| Arc::new(FakeStt) as Arc<dyn Transcriber>),
        embedder: media.then(|| Arc::new(FakeEmbed) as Arc<dyn Embedder>),
        cfg: Arc::new(cfg),
        clock: clock.clone(),
    };
    H {
        app: router(state.clone()),
        state,
        clock,
    }
}

impl H {
    async fn raw(
        &self,
        method: &str,
        uri: &str,
        ct: &str,
        body: Vec<u8>,
    ) -> (StatusCode, Vec<u8>, String) {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", ct)
            .body(Body::from(body))
            .unwrap();
        let resp = self.app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let ctype = resp
            .headers()
            .get("content-type")
            .map(|v| v.to_str().unwrap_or("").to_owned())
            .unwrap_or_default();
        (
            status,
            resp.into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
            ctype,
        )
    }
    async fn json(&self, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
        let (s, b, _) = self
            .raw(
                method,
                uri,
                "application/json",
                body.map(|b| b.to_string().into_bytes()).unwrap_or_default(),
            )
            .await;
        (
            s,
            if b.is_empty() {
                Value::Null
            } else {
                serde_json::from_slice(&b).unwrap()
            },
        )
    }
    async fn get(&self, uri: &str) -> (StatusCode, Value) {
        self.json("GET", uri, None).await
    }
    async fn post(&self, uri: &str, b: Value) -> (StatusCode, Value) {
        self.json("POST", uri, Some(b)).await
    }
}

#[tokio::test]
async fn reminders_snooze_and_notifications() {
    let ntfy = MockServer::start().await;
    Mock::given(method("POST"))
        .and(query_param("title", "Hatırlatma"))
        .respond_with(ResponseTemplate::new(200))
        .expect(2)
        .mount(&ntfy)
        .await;
    let mut cfg = Config::defaults();
    cfg.ntfy_url = Some(ntfy.uri());
    let h = h(cfg, ScriptedLlm::new(), false).await;
    let (_, t) = h
        .post("/api/tasks", json!({ "title": "Muayene tarihini kontrol et", "due_date": "2026-11-06", "due_time": "09:30" }))
        .await;
    let id = t["id"].as_str().unwrap().to_owned();
    let (s, _) = h
        .post(
            &format!("/api/tasks/{id}/edit"),
            json!({ "remind_before_minutes": 15 }),
        )
        .await;
    assert_eq!(s, StatusCode::OK);

    let (_, r) = h.get("/api/reminders").await;
    assert_eq!(r["groups"][0]["label"], "Bugün");
    assert_eq!(r["groups"][0]["items"][0]["when"], "09:15");

    // 09:16'da çalar, ikinci turda çalmaz.
    h.clock
        .set(Utc.with_ymd_and_hms(2026, 11, 6, 6, 16, 0).unwrap());
    assert_eq!(scheduler::fire_reminders(&h.state).await.unwrap(), 1);
    assert_eq!(scheduler::fire_reminders(&h.state).await.unwrap(), 0);
    // Erteleme: 10 dk sonra yeniden çalar.
    let (s, _) = h
        .post(
            &format!("/api/tasks/{id}/snooze"),
            json!({ "option": "minutes10" }),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    h.clock
        .set(Utc.with_ymd_and_hms(2026, 11, 6, 6, 27, 0).unwrap());
    assert_eq!(scheduler::fire_reminders(&h.state).await.unwrap(), 1);
    let (s, _) = h
        .post(
            &format!("/api/tasks/{id}/snooze"),
            json!({ "option": "asla" }),
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn calendar_task_and_topic_editing() {
    let h = h(Config::defaults(), ScriptedLlm::new(), false).await;
    let (_, t) = h
        .post(
            "/api/tasks",
            json!({ "title": "Akşam yürüyüşü", "due_date": "2026-11-06" }),
        )
        .await;
    let id = t["id"].as_str().unwrap().to_owned();
    let (s, row) = h
        .post(
            &format!("/api/tasks/{id}/edit"),
            json!({ "due_time": "19:00", "repeat": "day", "priority": "high" }),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(row["recurring"], true);
    assert_eq!(row["priority_label"], "Yüksek");
    let (s, _) = h
        .post(
            &format!("/api/tasks/{id}/edit"),
            json!({ "due_date": "", "repeat": "day" }),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "tarihsiz görev tekrarlanamaz");

    let (_, cal) = h.get("/api/calendar").await;
    let days = cal["days"].as_array().unwrap();
    assert_eq!(days.len(), 7);
    assert_eq!(days[4]["label"], "Cum 6 Kas");
    assert_eq!(days[4]["is_today"], true);
    assert_eq!(days[4]["items"][0]["title"], "Akşam yürüyüşü");

    let (_, topic) = h
        .post(
            "/api/topics",
            json!({ "name": "Logo ERP geçişi", "kind": "project" }),
        )
        .await;
    let tid = topic["id"].as_str().unwrap().to_owned();
    let (s, edited) = h
        .post(
            &format!("/api/topics/{tid}/edit"),
            json!({ "pinned": true, "due_date": "2026-12-15", "icon": "layers" }),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(edited["pinned"], true);
    assert_eq!(edited["kind"]["due"], "2026-12-15");
    let (s, _) = h
        .post(
            &format!("/api/topics/{tid}/edit"),
            json!({ "kind": "area" }),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "bitiş tarihli konu Alan olamaz");

    // Profil fotoğrafı (Revizyon 3): küçük data URL kabul, başka şey ret, boş dizgi kaldırır.
    let jpeg = "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQ==";
    let (s, _) = h
        .post(
            &format!("/api/topics/{tid}/edit"),
            json!({ "avatar": jpeg }),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let (_, list) = h.get("/api/topics").await;
    assert_eq!(list["topics"][0]["avatar"], jpeg);
    let (s, _) = h
        .post(
            &format!("/api/topics/{tid}/edit"),
            json!({ "avatar": "https://example.com/x.jpg" }),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "yalnız data:image");
    let big = format!("data:image/png;base64,{}", "A".repeat(130_000));
    let (s, _) = h
        .post(&format!("/api/topics/{tid}/edit"), json!({ "avatar": big }))
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "boyut sınırı");
    let (s, _) = h
        .post(&format!("/api/topics/{tid}/edit"), json!({ "avatar": "" }))
        .await;
    assert_eq!(s, StatusCode::OK);
    let (_, list) = h.get("/api/topics").await;
    assert!(list["topics"][0]["avatar"].is_null());

    h.clock.set(t0() + Duration::days(10));
    let (_, review) = h.get("/api/weekly-review").await;
    assert_eq!(review["stalled"][0]["name"], "Logo ERP geçişi");
}

#[tokio::test]
async fn ask_uses_text_and_semantic_search() {
    let h0 = h(Config::defaults(), ScriptedLlm::new(), true).await;
    let (_, e) = h0
        .post(
            "/api/entries",
            json!({ "text": "Ayhan Kimya teklifi: NaOH %99, 25 kg torba 1.850 ₺" }),
        )
        .await;
    h0.post("/api/entries", json!({ "text": "Kombi bakımı cumartesi" }))
        .await;
    assert_eq!(worker::embed_pending(&h0.state).await.unwrap(), 2);
    let eid = e["id"].as_str().unwrap().to_owned();

    let llm = ScriptedLlm::new()
        .reply(json!({ "answer": "En son 1.850 ₺ teklif almıştın (Ayhan Kimya).", "source_ids": [eid], "found": true }).to_string());
    let state = AppState {
        llm: Some(Arc::new(llm) as Arc<dyn LlmClient>),
        ..h0.state.clone()
    };
    let h = H {
        app: router(state.clone()),
        state,
        clock: h0.clock.clone(),
    };
    // "kostik fiyatı" tam metinde yok ama anlamsal vektörle bulunur.
    let (s, a) = h
        .post(
            "/api/ask",
            json!({ "question": "Kostik için en son hangi fiyatı almıştım?" }),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(a["answer"], "En son 1.850 ₺ teklif almıştın (Ayhan Kimya).");
    assert_eq!(a["sources"][0]["id"], eid.as_str());
    assert_eq!(a["sources"][0]["when"], "6 Kasım 09:12");
    let (s, _) = h.post("/api/ask", json!({ "question": "  " })).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn voice_capture_and_attachments() {
    let h = h(Config::defaults(), ScriptedLlm::new(), true).await;
    let (s, v, _) = h
        .raw(
            "POST",
            "/api/voice",
            "audio/webm",
            b"sahte-ses-verisi".to_vec(),
        )
        .await;
    assert_eq!(s, StatusCode::CREATED);
    let v: Value = serde_json::from_slice(&v).unwrap();
    assert!(v["text"].as_str().unwrap().starts_with("Yarın İzmir'deki"));
    assert_eq!(v["entry"]["source"], "voice");
    let eid = v["entry"]["id"].as_str().unwrap().to_owned();

    let (_, list) = h.get(&format!("/api/entries/{eid}/attachments")).await;
    assert_eq!(list[0]["kind"], "audio");

    let png = vec![0x89, b'P', b'N', b'G'];
    let (s, up, _) = h
        .raw(
            "POST",
            &format!("/api/entries/{eid}/attachments?kind=photo&name=fatura.png"),
            "image/png",
            png.clone(),
        )
        .await;
    assert_eq!(s, StatusCode::CREATED);
    let up: Value = serde_json::from_slice(&up).unwrap();
    let (s, data, ct) = h
        .raw(
            "GET",
            &format!("/api/attachments/{}", up["id"].as_str().unwrap()),
            "",
            vec![],
        )
        .await;
    assert_eq!((s, data, ct.as_str()), (StatusCode::OK, png, "image/png"));
    let (s, _, _) = h
        .raw(
            "POST",
            &format!(
                "/api/entries/{eid}/attachments?kind=location&name=Konum&value=41.0082,28.9784"
            ),
            "text/plain",
            vec![],
        )
        .await;
    assert_eq!(s, StatusCode::CREATED);
    let (s, _, _) = h
        .raw(
            "POST",
            &format!("/api/entries/{eid}/attachments?kind=video&name=x"),
            "video/mp4",
            vec![1],
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    let no_media = self::h(Config::defaults(), ScriptedLlm::new(), false).await;
    let (s, _, _) = no_media
        .raw("POST", "/api/voice", "audio/webm", vec![1, 2, 3])
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn autonomous_mode_applies_and_undo_reverts() {
    let h0 = h(Config::defaults(), ScriptedLlm::new(), false).await;
    let (_, topic) = h0
        .post(
            "/api/topics",
            json!({ "name": "Kostik tedarik", "kind": "project" }),
        )
        .await;
    let tid = topic["id"].as_str().unwrap().to_owned();
    let reply = json!({
        "topic": { "existing_id": tid, "new_name": null, "new_kind": null, "confidence": 0.95 },
        "tasks": [], "tags": [{ "name": "tedarik", "confidence": 0.5 }], "people": [], "reminder": null
    });
    let state = AppState {
        llm: Some(Arc::new(ScriptedLlm::new().reply(reply.to_string())) as Arc<dyn LlmClient>),
        ..h0.state.clone()
    };
    let h = H {
        app: router(state.clone()),
        state,
        clock: h0.clock.clone(),
    };
    let (_, st) = h
        .post(
            "/api/settings",
            json!({ "autonomous": true, "threshold": 0.9 }),
        )
        .await;
    assert_eq!(st["autonomous"], true);
    let (s, _) = h.post("/api/settings", json!({ "threshold": 0.2 })).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    h.post(
        "/api/entries",
        json!({ "text": "Kostik tedarikçisini ara" }),
    )
    .await;
    assert!(worker::process_one(&h.state).await.unwrap());
    let (_, j) = h.get("/api/journal").await;
    let item = &j["groups"][0]["items"][0];
    assert_eq!(
        item["topic"]["name"], "Kostik tedarik",
        "yüksek güvenli konu kendiliğinden uygulandı"
    );
    let suggs = item["suggestions"].as_array().unwrap();
    let topic_s = suggs.iter().find(|s| s["kind"] == "topic").unwrap();
    let tag_s = suggs.iter().find(|s| s["kind"] == "tag").unwrap();
    assert_eq!(topic_s["decision"], "accepted");
    assert_eq!(
        tag_s["decision"], "pending",
        "düşük güvenli öneri kullanıcıya bırakıldı"
    );

    let (s, e) = h
        .post(
            &format!("/api/suggestions/{}/undo", topic_s["id"].as_str().unwrap()),
            json!({}),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(e["topic"], Value::Null);
    let (s, _) = h
        .post(
            &format!("/api/suggestions/{}/undo", tag_s["id"].as_str().unwrap()),
            json!({}),
        )
        .await;
    assert_eq!(
        s,
        StatusCode::CONFLICT,
        "kabul edilmemiş öneri geri alınamaz"
    );
}

#[tokio::test]
async fn trash_and_facts() {
    let h = h(Config::defaults(), ScriptedLlm::new(), false).await;
    let (_, e) = h.post("/api/entries", json!({ "text": "Eski not" })).await;
    let id = e["id"].as_str().unwrap().to_owned();
    let (s, _) = h.post(&format!("/api/entries/{id}/purge"), json!({})).await;
    assert_eq!(s, StatusCode::CONFLICT);
    h.post(&format!("/api/entries/{id}/trash"), json!({})).await;
    h.clock.set(t0() + Duration::days(3));
    let (_, t) = h.get("/api/trash").await;
    assert_eq!(
        t[0]["note"],
        "3 gün önce silindi · 27 gün sonra kalıcı silinir"
    );
    let (s, _) = h.post(&format!("/api/entries/{id}/purge"), json!({})).await;
    assert_eq!(s, StatusCode::NO_CONTENT);

    let (_, f) = h
        .post(
            "/api/facts",
            json!({ "text": "GemOlive = sabun markası, Alan: İş" }),
        )
        .await;
    let fid = f["id"].as_str().unwrap().to_owned();
    let (_, list) = h
        .post(&format!("/api/facts/{fid}/confirm"), json!({}))
        .await;
    assert_eq!(list[0]["confirmed"], true);
    let (_, list) = h.post(&format!("/api/facts/{fid}/delete"), json!({})).await;
    assert_eq!(list, json!([]));
    let (_, settings) = h.get("/api/settings").await;
    assert_eq!(settings["ai_provider"], "Kapalı");
}

#[tokio::test]
async fn cors_only_for_configured_origins_and_token_still_required() {
    let mut cfg = Config::defaults();
    cfg.api_token = Some("gizli".into());
    cfg.cors_origins = vec!["http://localhost:8080".into()];
    let h = h(cfg, ScriptedLlm::new(), false).await;

    let preflight = |origin: &str| {
        Request::builder()
            .method("OPTIONS")
            .uri("/api/entries")
            .header("origin", origin)
            .header("access-control-request-method", "POST")
            .header(
                "access-control-request-headers",
                "authorization,content-type",
            )
            .body(Body::empty())
            .unwrap()
    };
    // İzinli köken: ön kontrol kimlik doğrulamasız geçer.
    let r = h
        .app
        .clone()
        .oneshot(preflight("http://localhost:8080"))
        .await
        .unwrap();
    assert!(r.status().is_success());
    assert_eq!(
        r.headers()["access-control-allow-origin"],
        "http://localhost:8080"
    );
    // Başka köken izin başlığı almaz.
    let r = h
        .app
        .clone()
        .oneshot(preflight("http://kotu.example"))
        .await
        .unwrap();
    assert!(!r.headers().contains_key("access-control-allow-origin"));

    // Asıl istek: belirteç yine zorunlu.
    let get = |auth: Option<&str>| {
        let mut b = Request::builder()
            .uri("/api/journal")
            .header("origin", "http://localhost:8080");
        if let Some(a) = auth {
            b = b.header("authorization", a);
        }
        b.body(Body::empty()).unwrap()
    };
    let r = h.app.clone().oneshot(get(None)).await.unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    let r = h
        .app
        .clone()
        .oneshot(get(Some("Bearer gizli")))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(
        r.headers()["access-control-allow-origin"],
        "http://localhost:8080"
    );
}

#[tokio::test]
async fn cors_is_off_by_default() {
    let h = h(Config::defaults(), ScriptedLlm::new(), false).await;
    let r = h
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/journal")
                .header("origin", "http://localhost:8080")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(!r.headers().contains_key("access-control-allow-origin"));
}

#[tokio::test]
async fn ai_asks_user_answers_and_entry_is_reclassified_with_notifications() {
    let ntfy = MockServer::start().await;
    Mock::given(method("POST"))
        .and(query_param("title", "NotAI soruyor"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&ntfy)
        .await;
    Mock::given(method("POST"))
        .and(query_param("title", "Hatırlatma · Kostik tedarik"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&ntfy)
        .await;
    let mut cfg = Config::defaults();
    cfg.ai = notai_server::AiProvider::External;
    cfg.ntfy_url = Some(ntfy.uri());
    let h = h(cfg, ScriptedLlm::new(), false).await;
    let (_, topic) = h
        .post(
            "/api/topics",
            json!({ "name": "Kostik tedarik", "kind": "project" }),
        )
        .await;
    let tid = topic["id"].as_str().unwrap().to_owned();
    let (_, e) = h
        .post("/api/entries", json!({ "text": "Ayhan'ı ara" }))
        .await;
    let eid = e["id"].as_str().unwrap().to_owned();

    // 1) Ajan emin değil: soru sorar.
    let (_, jobs) = h.get("/api/agent/jobs").await;
    assert_eq!(jobs[0]["entry_id"], eid.as_str());
    let (s, _) = h
        .post(
            &format!("/api/agent/jobs/{eid}/answer"),
            json!({ "topic": null, "tasks": [], "tags": [], "people": [], "reminder": null,
                    "question": "Hangi Ayhan? Ayhan Kimya mı?" }),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let (_, j) = h.get("/api/journal").await;
    let sugg = &j["groups"][0]["items"][0]["suggestions"][0];
    assert_eq!(sugg["kind"], "question");
    assert_eq!(sugg["chip"], "Hangi Ayhan? Ayhan Kimya mı?");
    let qid = sugg["id"].as_str().unwrap().to_owned();
    let (_, n) = h.get("/api/notice").await;
    assert_eq!(n["kind"], "question");
    assert_eq!(n["entry_id"], eid.as_str());
    let (_, jobs) = h.get("/api/agent/jobs").await;
    assert!(
        jobs.as_array().unwrap().is_empty(),
        "cevap beklenirken iş yok"
    );

    // Soru "kabul" edilemez; boş cevap reddedilir.
    let (s, _) = h
        .post(&format!("/api/suggestions/{qid}/accept"), json!({}))
        .await;
    assert!(s.is_client_error());
    let (s, _) = h
        .post(
            &format!("/api/suggestions/{qid}/answer"),
            json!({ "text": "  " }),
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);

    // 2) Kullanıcı cevaplar → kayıt cevapla yeniden ajana gider.
    let (s, _) = h
        .post(
            &format!("/api/suggestions/{qid}/answer"),
            json!({ "text": "Evet Ayhan Kimya, kostik için perşembe" }),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let (_, jobs) = h.get("/api/agent/jobs").await;
    let user = jobs[0]["user"].as_str().unwrap();
    assert!(user.contains("<cevap>Evet Ayhan Kimya, kostik için perşembe</cevap>"));
    assert!(
        !jobs[0]["system"]
            .as_str()
            .unwrap()
            .contains("yeni soru sorma")
    );

    // 3) Artık net: konu + saatli görev (varsayılan 15 dk önce hatırlatma).
    let (s, _) = h
        .post(
            &format!("/api/agent/jobs/{eid}/answer"),
            json!({ "topic": { "existing_id": tid, "new_name": null, "new_kind": null, "confidence": 0.9 },
                    "tasks": [{ "title": "Ayhan Kimya'yı ara", "due_date": "2026-11-12", "due_time": "10:00",
                                "priority": "normal", "confidence": 0.85 }],
                    "tags": [], "people": [], "reminder": null, "question": null }),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let (_, j) = h.get("/api/journal").await;
    let items = j["groups"][0]["items"][0]["suggestions"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["answer"], "Evet Ayhan Kimya, kostik için perşembe");
    for s in &items[1..] {
        let id = s["id"].as_str().unwrap();
        let (st, _) = h
            .post(&format!("/api/suggestions/{id}/accept"), json!({}))
            .await;
        assert_eq!(st, StatusCode::OK);
    }
    let (_, r) = h.get("/api/reminders").await;
    let item = &r["groups"][0]["items"][0];
    assert_eq!(item["title"], "Ayhan Kimya'yı ara");
    assert_eq!(
        item["when"], "Per 12 Kas 09:45",
        "AI görevine varsayılan hatırlatma: 15 dk önce"
    );

    // 4) Hatırlatma çalınca bildirim başlığında konu adı; son bildirim o konuyu açar.
    h.clock
        .set(Utc.with_ymd_and_hms(2026, 11, 12, 6, 46, 0).unwrap());
    assert_eq!(scheduler::fire_reminders(&h.state).await.unwrap(), 1);
    let (_, n) = h.get("/api/notice").await;
    assert_eq!(n["title"], "Hatırlatma · Kostik tedarik");
    assert_eq!(n["topic_id"], tid.as_str());
    let (s, _) = h.post("/api/notice/clear", json!({})).await;
    assert_eq!(s, StatusCode::OK);
    let (_, n) = h.get("/api/notice").await;
    assert!(n.is_null());
}

#[tokio::test]
async fn obsidian_vault_is_written_and_kept_in_sync_without_touching_user_notes() {
    let dir = std::env::temp_dir().join(format!("notai-kasa-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut cfg = Config::defaults();
    cfg.vault_dir = Some(dir.clone());
    let h = h(cfg, ScriptedLlm::new(), false).await;
    let (_, sabun) = h
        .post(
            "/api/topics",
            json!({ "name": "Sabun işi", "kind": "area" }),
        )
        .await;
    let sid = sabun["id"].as_str().unwrap().to_owned();
    let (_, para) = h
        .post(
            "/api/topics",
            json!({ "name": "Para alışverişi", "kind": "area", "parent_id": sid }),
        )
        .await;
    let pid = para["id"].as_str().unwrap().to_owned();
    h.post(
        "/api/entries",
        json!({ "text": "Ayhan Kimya'ya 1.850 ₺ ödendi", "topic_id": pid }),
    )
    .await;

    assert!(
        notai_server::vault::sync_vault(&h.state)
            .await
            .unwrap()
            .is_some()
    );
    let note = dir.join("NotAI/Konular/Sabun işi/Para alışverişi/Para alışverişi.md");
    let text = std::fs::read_to_string(&note).unwrap();
    assert!(text.contains("Ayhan Kimya'ya 1.850 ₺ ödendi"));
    assert!(dir.join("NotAI/Ana Günlük/2026-11-06.md").exists());
    // Kullanıcının kendi notu.
    let mine = dir.join("NotAI/Benim notum.md");
    std::fs::write(&mine, "kendi notum").unwrap();
    // Değişiklik yoksa yazılmaz.
    assert_eq!(
        notai_server::vault::sync_vault(&h.state).await.unwrap(),
        None
    );

    // Konu adı değişince eski dosya/klasör silinir, yenisi yazılır; kullanıcı notu kalır.
    let (s, _) = h
        .post(
            &format!("/api/topics/{pid}/edit"),
            json!({ "name": "Ödemeler" }),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert!(
        notai_server::vault::sync_vault(&h.state)
            .await
            .unwrap()
            .unwrap()
            >= 2
    );
    assert!(!note.exists());
    assert!(!dir.join("NotAI/Konular/Sabun işi/Para alışverişi").exists());
    assert!(
        dir.join("NotAI/Konular/Sabun işi/Ödemeler/Ödemeler.md")
            .exists()
    );
    assert_eq!(std::fs::read_to_string(&mine).unwrap(), "kendi notum");

    // Elle "şimdi yaz" ucu.
    let (s, b) = h.post("/api/vault/export", json!({})).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["written"], 0, "içerik aynıysa dosya yeniden yazılmaz");
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn bank_statement_pdf_import_report_and_dedup() {
    let h = h(Config::defaults(), ScriptedLlm::new(), false).await;
    let pdf = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ornek-ekstre.pdf"
    ))
    .unwrap();

    // Kayıttan gelen elle harcama (AI çipi kabul): Ayhan Kimya ödemesi, iş.
    let (_, e) = h
        .post(
            "/api/entries",
            json!({ "text": "Ayhan Kimya'ya 1.850 ₺ ödendi" }),
        )
        .await;
    let eid = e["id"].as_str().unwrap().to_owned();
    let ctx_entry = h.state.store.entry(eid.parse().unwrap()).await.unwrap();
    let s = notai_core::Suggestion::new(
        ctx_entry.id,
        notai_core::SuggestionKind::Expense {
            amount: notai_core::finance::Money(-185_000),
            category: "tedarik".into(),
            scope: notai_core::finance::Scope::Is,
            date: chrono::NaiveDate::from_ymd_opt(2026, 10, 3).unwrap(),
        },
        notai_core::Confidence::new(0.9).unwrap(),
    );
    h.state
        .store
        .save_ai_suggestions(ctx_entry.id, std::slice::from_ref(&s), t0())
        .await
        .unwrap();
    let (st, _) = h
        .post(&format!("/api/suggestions/{}/accept", s.id), json!({}))
        .await;
    assert_eq!(st, StatusCode::OK);

    // PDF ekstre: yerel olarak okunur, 10 hareket; Ayhan Kimya ödemesi elle girişle eşleşir.
    let (st, b, _) = h
        .raw(
            "POST",
            "/api/finance/statements?account=****1234&name=ekim.pdf",
            "application/pdf",
            pdf.clone(),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED);
    let r: Value = serde_json::from_slice(&b).unwrap();
    assert_eq!(
        (
            r["parsed"].as_u64(),
            r["imported"].as_u64(),
            r["matched"].as_u64()
        ),
        (Some(10), Some(10), Some(1))
    );

    // Aynı PDF ikinci kez: hiçbiri eklenmez.
    let (_, b, _) = h
        .raw(
            "POST",
            "/api/finance/statements?account=****1234&name=ekim.pdf",
            "application/pdf",
            pdf,
        )
        .await;
    let r: Value = serde_json::from_slice(&b).unwrap();
    assert_eq!(
        (r["imported"].as_u64(), r["skipped"].as_u64()),
        (Some(0), Some(10))
    );
    assert!(
        r["message"]
            .as_str()
            .unwrap()
            .contains("mükerrer eklenmedi")
    );

    // Aylık rapor: gider bir kez sayılır, iş ayrı, öneriler var.
    let (_, f) = h.get("/api/finance?month=2026-10").await;
    assert_eq!(f["title"], "Ekim 2026");
    assert_eq!(f["income"], "85.000,00\u{a0}₺");
    let expense_kurus =
        124_560 + 185_000 + 210_000 + 132_045 + 22_999 + 64_000 + 98_030 + 115_000 + 340_000;
    assert_eq!(
        f["expense"],
        notai_core::finance::Money(expense_kurus).format_abs()
    );
    assert_eq!(f["scopes"][0]["label"], "İş");
    assert_eq!(
        f["transactions"].as_array().unwrap().len(),
        11,
        "10 ekstre + 1 elle (eşleşmiş)"
    );
    assert!(
        f["transactions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["matched"] == true)
    );
    assert!(
        f["insights"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i.as_str().unwrap().contains("biriktirdin"))
    );

    // Düzeltme + öğrenme.
    let trendyol = f["transactions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["description"].as_str().unwrap().contains("TRENDYOL"))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (st, _) = h
        .post(
            &format!("/api/finance/transactions/{trendyol}/classify"),
            json!({ "category": "tedarik", "scope": "is" }),
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = h
        .post(
            &format!("/api/finance/transactions/{trendyol}/classify"),
            json!({ "category": "uydurma", "scope": "is" }),
        )
        .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // Tanınmayan biçim → ajan işi → ajan cevabı doğrulanıp içe aktarılır.
    let (_, b, _) = h
        .raw(
            "POST",
            "/api/finance/statements?name=garip.txt",
            "text/plain",
            b"tarih yok tutar yok".to_vec(),
        )
        .await;
    let r: Value = serde_json::from_slice(&b).unwrap();
    assert_eq!(r["pending"], true);
    let (_, jobs) = h.get("/api/agent/statements").await;
    let sid = jobs[0]["id"].as_str().unwrap().to_owned();
    assert!(
        jobs[0]["schema"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x == "lines")
    );
    let (st, r) = h
        .post(
            &format!("/api/agent/statements/{sid}/answer"),
            json!({ "lines": [
                { "date": "2026-10-28", "description": "A101", "amount": "-50,00" },
                { "date": "bozuk", "description": "X", "amount": "-1" }
            ] }),
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(r["imported"], 1);
    assert!(
        r["message"]
            .as_str()
            .unwrap()
            .contains("1 satır doğrulanamadı")
    );
    let (_, jobs) = h.get("/api/agent/statements").await;
    assert!(jobs.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn price_watch_reports_only_on_change_and_notifies() {
    let ntfy = MockServer::start().await;
    Mock::given(method("POST"))
        .and(query_param("title", "Takip · Pixel 10 Pro"))
        .respond_with(ResponseTemplate::new(200))
        .expect(2)
        .mount(&ntfy)
        .await;
    let mut cfg = Config::defaults();
    cfg.ntfy_url = Some(ntfy.uri());
    let h = h(cfg, ScriptedLlm::new(), false).await;
    let (_, t) = h
        .post(
            "/api/topics",
            json!({ "name": "Pixel 10 Pro", "kind": "resource" }),
        )
        .await;
    let tid = t["id"].as_str().unwrap().to_owned();
    // Geçersiz bağlantı reddedilir; geçerli ayar kaydedilir.
    let (s, _) = h
        .post(
            &format!("/api/topics/{tid}/watch"),
            json!({ "query": "Pixel 10 Pro", "urls": ["ftp://x"] }),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = h
        .post(
            &format!("/api/topics/{tid}/watch"),
            json!({ "query": "Pixel 10 Pro 256 GB", "urls": ["https://example.com/p10"], "target_price": "50.000" }),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let (_, topics) = h.get("/api/topics").await;
    assert_eq!(topics["topics"][0]["watch"]["target_price"], "50.000,00");

    let (_, jobs) = h.get("/api/agent/watches").await;
    assert_eq!(jobs[0]["topic_id"], tid.as_str());
    assert_eq!(jobs[0]["query"], "Pixel 10 Pro 256 GB");
    assert!(
        jobs[0]["schema"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x == "offers")
    );

    let report = |price: &str| {
        json!({ "offers": [
            { "title": "Pixel 10 Pro 256 GB", "price": price, "url": "https://t.com/p10", "store": "Trendyol" },
            { "title": "Pixel 10 Pro", "price": "abc", "url": "https://x.com", "store": null }
        ] })
    };
    // 1) İlk sonuç: konuya NotAI mesajı + bildirim.
    let (_, r) = h
        .post(
            &format!("/api/agent/watches/{tid}/report"),
            report("54.999,00"),
        )
        .await;
    assert_eq!(r["changed"], true);
    assert_eq!(r["dropped"], 1, "bozuk fiyatlı teklif atılır");
    let (_, j) = h
        .get(&format!("/api/journal?filter=topic&topic_id={tid}"))
        .await;
    let item = &j["groups"][0]["items"][0];
    assert_eq!(item["from_notai"], true);
    assert!(item["text"].as_str().unwrap().contains("https://t.com/p10"));
    let (_, jobs) = h.get("/api/agent/watches").await;
    assert!(jobs.as_array().unwrap().is_empty(), "bugün tekrar bakılmaz");

    // 2) Ertesi gün aynı fiyat: sessiz.
    h.clock.set(t0() + Duration::hours(25));
    let (_, r) = h
        .post(
            &format!("/api/agent/watches/{tid}/report"),
            report("54.999,00"),
        )
        .await;
    assert_eq!(r["changed"], false);

    // 3) Düşüş: mesaj + bildirim; son bildirim takip.
    h.clock.set(t0() + Duration::hours(50));
    let (_, r) = h
        .post(
            &format!("/api/agent/watches/{tid}/report"),
            report("52.499,00"),
        )
        .await;
    assert_eq!(r["changed"], true);
    assert!(r["message"].as_str().unwrap().starts_with("Fiyat düştü"));
    let (_, n) = h.get("/api/notice").await;
    assert_eq!(n["kind"], "watch");
    assert_eq!(n["topic_id"], tid.as_str());
    let (_, j) = h
        .get(&format!("/api/journal?filter=topic&topic_id={tid}"))
        .await;
    let n_msgs: usize = j["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["items"].as_array().unwrap().len())
        .sum();
    assert_eq!(n_msgs, 2, "aynı fiyat konuya yazılmadı");
    // NotAI mesajı AI sınıflandırmasına girmez.
    assert!(
        h.state
            .store
            .next_ai_job(t0() + Duration::hours(50))
            .await
            .unwrap()
            .is_none()
    );

    // Takibi kapat.
    let (_, _) = h
        .post(&format!("/api/topics/{tid}/watch"), json!({ "query": "" }))
        .await;
    let (_, topics) = h.get("/api/topics").await;
    assert!(topics["topics"][0]["watch"].is_null());
}

#[tokio::test]
async fn assets_are_added_priced_by_agent_and_summarized() {
    let ntfy = MockServer::start().await;
    Mock::given(method("POST"))
        .and(query_param("title", "Varlık · Altın birikimi"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&ntfy)
        .await;
    let mut cfg = Config::defaults();
    cfg.ntfy_url = Some(ntfy.uri());
    let h = h(cfg, ScriptedLlm::new(), false).await;
    let (s, _) = h
        .post(
            "/api/assets",
            json!({ "name": "Bozuk", "kind": "uzay gemisi", "value": "1" }),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, r) = h
        .post(
            "/api/assets",
            json!({ "name": "Altın birikimi", "kind": "altin", "quantity": "12,5",
                    "unit": "gram", "symbol": "gram altın", "value": "40.000" }),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let gold = r["topic_id"].as_str().unwrap().to_owned();
    let (_, r) = h
        .post(
            "/api/assets",
            json!({ "name": "Kadıköy daire", "kind": "gayrimenkul", "value": "5.000.000" }),
        )
        .await;
    let flat = r["topic_id"].as_str().unwrap().to_owned();
    // Varlıklar ana konusunun altında, simgesi türünden.
    let (_, topics) = h.get("/api/topics").await;
    let row = topics["topics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == flat.as_str())
        .unwrap()
        .clone();
    assert_eq!(row["path"], "Varlıklar / Kadıköy daire");
    assert_eq!(row["icon"], "house");

    let (_, v) = h.get("/api/assets").await;
    assert_eq!(v["total"], "5.040.000,00\u{a0}₺");
    assert_eq!(v["assets"][0]["name"], "Kadıköy daire", "büyükten küçüğe");
    assert_eq!(v["assets"][1]["quantity"], "12,5 gram");
    assert_eq!(v["assets"][1]["form"]["value"], "40.000,00");
    assert_eq!(v["by_kind"][0]["key"], "gayrimenkul");

    // Ajan: bugün eklenenler fiyatlanmış sayılır; ertesi gün yalnız sembollü altın.
    let (_, jobs) = h.get("/api/agent/assets").await;
    assert!(jobs.as_array().unwrap().is_empty());
    h.clock.set(t0() + Duration::hours(25));
    let (_, jobs) = h.get("/api/agent/assets").await;
    assert_eq!(jobs.as_array().unwrap().len(), 1);
    assert_eq!(jobs[0]["topic_id"], gold.as_str());
    assert_eq!(jobs[0]["quantity"], "12,5");
    // Küçük değişim sessiz; büyük değişim konuya yazılır + bildirim.
    let (_, r) = h
        .post(
            &format!("/api/agent/assets/{gold}/price"),
            json!({ "unit_price": "3.220" }),
        )
        .await;
    assert_eq!(r["changed"], false);
    h.clock.set(t0() + Duration::hours(49));
    let (_, r) = h
        .post(
            &format!("/api/agent/assets/{gold}/price"),
            json!({ "unit_price": "3.400" }),
        )
        .await;
    assert_eq!(r["changed"], true, "{r}");
    assert!(r["message"].as_str().unwrap().contains("42.500,00"));
    let (_, n) = h.get("/api/notice").await;
    assert_eq!(n["topic_id"], gold.as_str());
    let (s, _) = h
        .post(
            &format!("/api/agent/assets/{gold}/price"),
            json!({ "unit_price": "0" }),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Elle düzenleme ve kaldırma.
    let (s, _) = h
        .post(
            &format!("/api/topics/{flat}/asset"),
            json!({ "kind": "gayrimenkul", "value": "5.250.000" }),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    let (_, v) = h.get("/api/assets").await;
    assert_eq!(v["total"], "5.292.500,00\u{a0}₺");
    let (_, _) = h
        .post(
            &format!("/api/topics/{flat}/asset"),
            json!({ "kind": "", "value": "" }),
        )
        .await;
    let (_, v) = h.get("/api/assets").await;
    assert_eq!(v["assets"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn movements_are_detected_located_asked_and_reported() {
    let ntfy = MockServer::start().await;
    Mock::given(method("POST"))
        .and(query_param("title", "Konumu aç · İş"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&ntfy)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&ntfy)
        .await;
    let mut cfg = Config::defaults();
    cfg.ntfy_url = Some(ntfy.uri());
    let h = h(cfg, ScriptedLlm::new(), false).await;
    let cap = |text: &'static str| {
        let h = &h;
        async move {
            let (s, e) = h.post("/api/entries", json!({ "text": text })).await;
            assert_eq!(s, StatusCode::CREATED, "{e}");
            e["id"].as_str().unwrap().to_owned()
        }
    };
    // Sabah: evden çıkış (konum açık) → Ev öğrenilir, ulaşım sorulur.
    let e1 = cap("Evden çıkıyorum").await;
    let (_, c) = h.get(&format!("/api/entries/{e1}/movement")).await;
    assert_eq!(c, json!({ "movement": true, "wants_location": true }));
    let (_, l) = h
        .post(
            &format!("/api/entries/{e1}/location"),
            json!({ "lat": 40.99, "lon": 29.03, "accuracy_m": 12.0 }),
        )
        .await;
    assert!(
        l["message"]
            .as_str()
            .unwrap()
            .contains("Ev konumu öğrenildi"),
        "{l}"
    );
    let (_, n) = h.get("/api/notice").await;
    assert_eq!(n["kind"], "question");
    assert!(n["body"].as_str().unwrap().starts_with("Nereye, neyle"));
    // Cevap: Ana Günlük'teki soru balonundan.
    let (_, j) = h.get("/api/journal").await;
    let item = j["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["items"].as_array().unwrap().clone())
        .find(|i| i["id"] == e1.as_str())
        .unwrap();
    assert_eq!(item["topic"]["name"], "Ev");
    assert_eq!(item["unprocessed"], false, "yalnız hareket: AI beklemez");
    let q = item["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["kind"] == "question")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (s, _) = h
        .post(
            &format!("/api/suggestions/{q}/answer"),
            json!({ "text": "metroyla işe" }),
        )
        .await;
    assert_eq!(s, StatusCode::OK);

    // İşe varış, konum kapalı: İş'in yeri bilinmiyor → "Konumu aç · İş".
    h.clock.set(t0() + Duration::minutes(50));
    let e2 = cap("işe geldim").await;
    let (_, l) = h
        .post(&format!("/api/entries/{e2}/location"), json!({}))
        .await;
    assert_eq!(l["need_location"], true);
    let (_, n) = h.get("/api/notice").await;
    assert_eq!(n["kind"], "location");
    assert_eq!(n["title"], "Konumu aç · İş");

    // Akşam WhatsApp'tan: konum gelmez; zamanlayıcı soruyu sorar.
    h.clock.set(t0() + Duration::hours(10));
    let e3 = cap("işten çıktım").await;
    h.clock
        .set(t0() + Duration::hours(10) + Duration::minutes(3));
    scheduler::tick(&h.state).await.unwrap();
    let (_, n) = h.get("/api/notice").await;
    assert_eq!(n["kind"], "question", "{n}");
    assert_eq!(n["entry_id"], e3.as_str());
    h.clock.set(t0() + Duration::hours(11));
    let _ = cap("eve geldim").await;

    let month = t0().format("%Y-%m").to_string();
    let (_, r) = h.get(&format!("/api/movements?month={month}")).await;
    assert_eq!(r["trips"], 2, "{r}");
    assert_eq!(r["trips_is"], 2);
    let work = r["places"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "İş")
        .unwrap()
        .clone();
    assert_eq!(work["visits"], 1);
    assert!(
        work["stay"]
            .as_str()
            .unwrap()
            .starts_with("toplam 9 sa 10 dk"),
        "{work}"
    );
    assert_eq!(r["recent"][3]["text"], "Ev → İş · Metro");
    assert!(r["insights"].as_array().unwrap().iter().any(|i| {
        i.as_str()
            .unwrap()
            .contains("Konumu henüz öğrenilmemiş: İş")
    }));
}
