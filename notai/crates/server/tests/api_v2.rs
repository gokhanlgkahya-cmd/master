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
