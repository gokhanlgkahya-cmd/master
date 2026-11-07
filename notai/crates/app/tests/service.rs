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

#[tokio::test]
async fn v2_v4_local_features() {
    use notai_api::{TaskEditRequest, TopicEditRequest};
    let tz = chrono_tz::Europe::Istanbul;
    let now = Utc.with_ymd_and_hms(2026, 11, 6, 6, 12, 0).unwrap();
    let s = Store::open_in_memory("telefon").await.unwrap();

    // Konu oluştur, düzenle (sabitle), kaydı konuya taşı.
    service::create_topic(&s, "Ev", "area", now).await.unwrap();
    assert!(service::create_topic(&s, "X", "klasör", now).await.is_err());
    let topics = service::topics(&s, "all", now, tz).await.unwrap();
    let ev = topics.topics[0].id.clone();
    service::edit_topic(
        &s,
        &ev,
        &TopicEditRequest {
            pinned: Some(true),
            ..Default::default()
        },
        now,
    )
    .await
    .unwrap();
    assert!(service::topics(&s, "all", now, tz).await.unwrap().topics[0].pinned);
    let eid = service::capture(
        &s,
        CaptureRequest {
            text: "Kombi bakımı".into(),
            topic_id: None,
        },
        now,
    )
    .await
    .unwrap();
    service::move_entry(&s, &eid, Some(&ev), now).await.unwrap();
    assert_eq!(
        service::journal(&s, Some(&ev), now, tz)
            .await
            .unwrap()
            .groups[0]
            .items
            .len(),
        1
    );

    // Görev ekle, düzenle (saat + 15 dk önce hatırlat), hatırlatmalar ve takvim.
    let tid = service::create_task(
        &s,
        service::NewTaskReq {
            title: "Kombi servisini ara".into(),
            due_date: Some("2026-11-06".into()),
            topic_id: Some(ev.clone()),
        },
        now,
    )
    .await
    .unwrap();
    assert_eq!(service::tasks(&s, now, tz).await.unwrap().today[0].id, tid);
    let edit = TaskEditRequest {
        due_time: Some("10:00".into()),
        remind_before_minutes: Some(15),
        ..Default::default()
    };
    service::edit_task(&s, &tid, &edit, now).await.unwrap();
    let r = service::reminders(&s, now, tz).await.unwrap();
    assert_eq!(r.groups[0].items[0].when, "09:45");
    service::snooze(&s, &tid, "tomorrow", now, tz)
        .await
        .unwrap();
    assert_eq!(
        service::reminders(&s, now, tz).await.unwrap().groups[0].key,
        "tomorrow"
    );
    let cal = service::calendar(&s, None, 7, now, tz).await.unwrap();
    assert_eq!(cal.days[4].items[0].title, "Kombi servisini ara");
    assert!(
        service::weekly_review_view(&s, now, tz)
            .await
            .unwrap()
            .upcoming
            .len()
            == 1
    );

    // Ekler (yerel), profil bilgileri, çöp kutusu.
    service::upload_attachment(
        &s,
        &eid,
        "location",
        "Konum".into(),
        "text/plain".into(),
        vec![],
        Some("41.0,28.9".into()),
        now,
    )
    .await
    .unwrap();
    assert_eq!(
        service::attachments(&s, &eid).await.unwrap()[0]
            .value
            .as_deref(),
        Some("41.0,28.9")
    );
    service::add_fact(&s, "GemOlive = sabun markası", now)
        .await
        .unwrap();
    let f = service::facts(&s).await.unwrap();
    service::fact_action(&s, &f[0].id, "delete", now)
        .await
        .unwrap();
    service::entry_action(&s, &eid, "trash", now).await.unwrap();
    assert_eq!(service::trash(&s, now).await.unwrap().len(), 1);
    service::purge(&s, &eid, now).await.unwrap();

    // Sunucu yokken: Ara/Sor yerel aramaya düşer, ses ve otonom ayarı açık hata verir.
    service::capture(
        &s,
        CaptureRequest {
            text: "Kostik fiyatı 1.850".into(),
            topic_id: None,
        },
        now,
    )
    .await
    .unwrap();
    let a = service::ask(&s, "kostik", tz).await.unwrap();
    assert!(a.answer.is_none());
    assert_eq!(a.sources.len(), 1);
    assert!(
        service::voice(&s, vec![1, 2], "audio/webm", None)
            .await
            .unwrap_err()
            .contains("ev sunucusu")
    );
    assert!(service::set_settings(&s, Some(true), None).await.is_err());
    assert_eq!(
        service::settings(&s).await.unwrap().ai_provider,
        "Ev sunucusu ayarlı değil"
    );
}
