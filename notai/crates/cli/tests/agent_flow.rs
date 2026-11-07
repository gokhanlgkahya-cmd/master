//! Dış ajan akışı: gerçek sunucu (NOTAI_AI=external) + notai CLI.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use notai_cli::{Client, Cmd, run};
use notai_server::{AiProvider, AppState, Config, ManualClock, router};
use notai_store::{NewTopic, Store};
use serde_json::{Value, json};

fn none() -> Result<String, String> {
    Err("stdin yok".into())
}

#[tokio::test]
async fn agent_takes_jobs_and_answers_through_cli() {
    let store = Store::open_in_memory("sunucu").await.unwrap();
    let now = Utc.with_ymd_and_hms(2026, 11, 6, 6, 12, 0).unwrap();
    let kostik = store
        .create_topic(
            NewTopic::new(
                notai_core::text::TopicName::new("Kostik tedarik").unwrap(),
                notai_core::topic::ParaKind::Project { due: None },
            ),
            now,
        )
        .await
        .unwrap();
    let mut cfg = Config::defaults();
    cfg.ai = AiProvider::External;
    cfg.api_token = Some("s24".into());
    let state = AppState {
        store,
        llm: None,
        stt: None,
        embedder: None,
        cfg: Arc::new(cfg),
        clock: Arc::new(ManualClock::new(now)),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router(state)).await });
    let c = Client::new(format!("http://{addr}"), Some("s24".into())).unwrap();

    // Yakala, iş al.
    let out = run(
        Cmd::Capture {
            text: "Kostik tedarikçisini ara – perşembe".into(),
        },
        &c,
        none,
    )
    .await
    .unwrap();
    assert!(out.starts_with("yakalandı: "));
    let jobs: Vec<Value> =
        serde_json::from_str(&run(Cmd::Jobs { limit: 5 }, &c, none).await.unwrap()).unwrap();
    assert_eq!(jobs.len(), 1);
    let job = &jobs[0];
    assert!(
        job["system"]
            .as_str()
            .unwrap()
            .contains(&kostik.id.to_string()),
        "istem konu listesini içerir"
    );
    assert!(job["user"].as_str().unwrap().contains("<kayit>"));
    assert_eq!(job["schema"]["required"].as_array().unwrap().len(), 6);
    let entry = job["entry_id"].as_str().unwrap().to_owned();

    // Bozuk cevap reddedilir, hiçbir şey yazılmaz.
    let bad = run(
        Cmd::Answer {
            entry: entry.clone(),
            file: None,
        },
        &c,
        || Ok(r#"{"topic":null}"#.into()),
    )
    .await;
    assert!(bad.unwrap_err().contains("şemaya uymuyor"));

    // Geçerli cevap (bir uydurma konu kimliğiyle birlikte) doğrulanıp kaydedilir.
    let answer = json!({
        "topic": { "existing_id": kostik.id.to_string(), "new_name": null, "new_kind": null, "confidence": 0.9 },
        "tasks": [{ "title": "Kostik firmasını ara", "due_date": "2026-11-12", "due_time": null, "priority": "high", "confidence": 0.8 }],
        "tags": [], "people": [], "reminder": null
    });
    let out = run(
        Cmd::Answer {
            entry: entry.clone(),
            file: None,
        },
        &c,
        || Ok(answer.to_string()),
    )
    .await
    .unwrap();
    assert_eq!(out, "2 öneri kaydedildi");
    let jobs: Vec<Value> =
        serde_json::from_str(&run(Cmd::Jobs { limit: 5 }, &c, none).await.unwrap()).unwrap();
    assert!(jobs.is_empty(), "iş kuyruktan çıktı");

    let inbox = run(Cmd::Inbox, &c, none).await.unwrap();
    assert!(inbox.contains("✧ → Kostik tedarik"));
    assert!(inbox.contains("✧ Görev · Perşembe 12 Kas"));

    // Yanlış belirteç.
    let wrong = Client::new(format!("http://{addr}"), Some("yanlis".into())).unwrap();
    assert!(
        run(Cmd::Inbox, &wrong, none)
            .await
            .unwrap_err()
            .contains("yetkisiz")
    );

    let plan = run(Cmd::Plan, &c, none).await.unwrap();
    assert!(plan.starts_with("Günaydın Gökhan."));
}
