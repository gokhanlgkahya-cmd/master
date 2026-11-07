//! Uygulama komutlarının mantığı (Tauri olmadan, bellek içi depo).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{TimeZone, Utc};
use notai_api::CaptureRequest;
use notai_app::service;
use notai_core::text::TopicName;
use notai_core::topic::ParaKind;
use notai_store::{NewTopic, Store};

#[tokio::test]
async fn capture_journal_topics_and_actions() {
    let tz = chrono_tz::Europe::Istanbul;
    let now = Utc.with_ymd_and_hms(2026, 11, 6, 6, 12, 0).unwrap();
    let s = Store::open_in_memory("telefon").await.unwrap();
    let gem = s
        .create_topic(
            NewTopic::new(TopicName::new("GemOlive").unwrap(), ParaKind::Area),
            now,
        )
        .await
        .unwrap();

    let id = service::capture(
        &s,
        CaptureRequest {
            text: "Ambalaj faturası".into(),
            topic_id: None,
        },
        now,
    )
    .await
    .unwrap();
    service::capture(
        &s,
        CaptureRequest {
            text: "Numune".into(),
            topic_id: Some(gem.id.to_string()),
        },
        now,
    )
    .await
    .unwrap();
    assert!(
        service::capture(
            &s,
            CaptureRequest {
                text: " ".into(),
                topic_id: None
            },
            now
        )
        .await
        .is_err()
    );
    assert!(
        service::capture(
            &s,
            CaptureRequest {
                text: "x".into(),
                topic_id: Some("bozuk".into())
            },
            now
        )
        .await
        .is_err()
    );

    let j = service::journal(&s, None, now, tz).await.unwrap();
    assert_eq!(j.groups[0].label, "Bugün");
    assert_eq!(j.groups[0].items.len(), 2);
    assert_eq!(j.unprocessed, 1);

    let t = service::topics(&s, "all", now, tz).await.unwrap();
    assert_eq!(t.topics[0].name, "GemOlive");
    assert_eq!(t.unprocessed, 1);
    assert_eq!(t.journal_last_label.as_deref(), Some("09:12"));

    let flow = service::journal(&s, Some(&gem.id.to_string()), now, tz)
        .await
        .unwrap();
    assert_eq!(flow.groups[0].items.len(), 1);

    service::entry_action(&s, &id, "archive", now)
        .await
        .unwrap();
    assert!(service::entry_action(&s, &id, "patlat", now).await.is_err());
    assert_eq!(
        service::journal(&s, None, now, tz)
            .await
            .unwrap()
            .unprocessed,
        0
    );

    let tasks = service::tasks(&s, now, tz).await.unwrap();
    assert!(tasks.today.is_empty());
    let plan = service::morning_plan(&s, "Gökhan", now, tz).await.unwrap();
    assert!(plan.items.is_empty());

    // Sunucu ayarlı değilse senkron sessizce atlanır.
    assert_eq!(service::sync_now(&s).await.unwrap(), None);
}
