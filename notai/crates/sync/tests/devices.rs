//! Üç cihaz senaryosu: telefon + masaüstü istemci, ev sunucusu merkez.

// Yalnız test kodu: yardımcı fonksiyonlarda unwrap/panic serbest (üretim kodunda yasak).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{DateTime, Duration, TimeZone, Utc};
use chrono_tz::Tz;
use notai_core::entry::{EntryStatus, Source};
use notai_core::journal::JournalFilter;
use notai_core::suggestion::{Confidence, Suggestion, SuggestionKind, TopicTarget};
use notai_core::text::{EntryText, Label, TopicName};
use notai_core::topic::{KindFilter, ParaKind};
use notai_core::{EntryId, TopicId};
use notai_store::changes::ChangeRecord;
use notai_store::{NewEntry, NewTopic, Store};
use notai_sync::{InProcess, PushRequest, SyncError, handle_push, sync_once};

const TZ: Tz = chrono_tz::Europe::Istanbul;

fn t(min: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 6, 6, 0, 0).unwrap() + Duration::minutes(min)
}

struct World {
    server: Store,
    phone: Store,
    desktop: Store,
    hub: InProcess,
}

async fn world() -> World {
    let server = Store::open_in_memory("sunucu").await.unwrap();
    World {
        hub: InProcess {
            server: server.clone(),
        },
        server,
        phone: Store::open_in_memory("telefon").await.unwrap(),
        desktop: Store::open_in_memory("masaustu").await.unwrap(),
    }
}

async fn capture(s: &Store, text: &str, at: DateTime<Utc>, topic: Option<TopicId>) -> EntryId {
    s.capture(NewEntry {
        text: EntryText::new(text).unwrap(),
        source: Source::App,
        topic,
        captured_at: at,
        dedupe_key: None,
    })
    .await
    .unwrap()
    .entry()
    .id
}

async fn topic(s: &Store, name: &str, at: DateTime<Utc>) -> TopicId {
    s.create_topic(
        NewTopic::new(TopicName::new(name).unwrap(), ParaKind::Area),
        at,
    )
    .await
    .unwrap()
    .id
}

async fn texts(s: &Store) -> Vec<String> {
    s.journal(JournalFilter::All, None, 100)
        .await
        .unwrap()
        .into_iter()
        .map(|i| i.entry.text.as_str().to_owned())
        .collect()
}

#[tokio::test]
async fn capture_on_phone_ai_on_server_decide_on_desktop() {
    let w = world().await;
    let gem = topic(&w.phone, "GemOlive", t(0)).await;
    let id = capture(&w.phone, "Ambalaj faturası · 1.850 ₺", t(1), None).await;

    let r = sync_once(&w.phone, &w.hub).await.unwrap();
    assert_eq!(r.pushed, 2);
    // Sunucuda yeni kayıt AI kuyruğuna girdi.
    assert_eq!(w.server.next_ai_job(t(2)).await.unwrap().unwrap().id, id);

    // Sunucudaki AI bir konu önerir.
    let s = Suggestion::new(
        id,
        SuggestionKind::Topic {
            target: TopicTarget::Existing {
                id: gem,
                name: TopicName::new("GemOlive").unwrap(),
            },
        },
        Confidence::new(0.9).unwrap(),
    );
    w.server
        .save_ai_suggestions(id, std::slice::from_ref(&s), t(2))
        .await
        .unwrap();

    // Masaüstü her şeyi alır ve öneriyi kabul eder.
    sync_once(&w.desktop, &w.hub).await.unwrap();
    assert_eq!(texts(&w.desktop).await, ["Ambalaj faturası · 1.850 ₺"]);
    assert!(
        w.desktop.next_ai_job(t(3)).await.unwrap().is_none(),
        "istemcide AI kuyruğu yok"
    );
    let out = w.desktop.decide(s.id, true, t(3), TZ).await.unwrap();
    assert_eq!(out.entry.topic(), Some(gem));
    sync_once(&w.desktop, &w.hub).await.unwrap();

    // Telefon karar ve konu atamasını alır.
    sync_once(&w.phone, &w.hub).await.unwrap();
    let e = w.phone.entry(id).await.unwrap();
    assert_eq!(e.topic(), Some(gem));
    assert_eq!(e.status(), EntryStatus::Processed);
    let sugg = w.phone.suggestions_for(id).await.unwrap();
    assert_eq!(sugg.len(), 1);
    assert!(!sugg[0].is_pending());

    // İkinci tur: yeni bir şey yok, yankı yok.
    let r = sync_once(&w.phone, &w.hub).await.unwrap();
    assert_eq!((r.pushed, r.pulled_applied), (0, 0));
    let r = sync_once(&w.desktop, &w.hub).await.unwrap();
    assert_eq!((r.pushed, r.pulled_applied), (0, 0));
}

#[tokio::test]
async fn offline_conflict_last_writer_wins_everywhere() {
    let w = world().await;
    let id = capture(&w.phone, "Eski not", t(0), None).await;
    sync_once(&w.phone, &w.hub).await.unwrap();
    sync_once(&w.desktop, &w.hub).await.unwrap();

    // İkisi de çevrimdışı: telefon 10. dk'da arşivler, masaüstü 20. dk'da çöpe atar.
    w.phone.archive_entry(id, t(10)).await.unwrap();
    w.desktop.trash_entry(id, t(20)).await.unwrap();

    // Önce masaüstü, sonra telefon senkronlansa da sonuç "en son yazan": çöp.
    sync_once(&w.desktop, &w.hub).await.unwrap();
    sync_once(&w.phone, &w.hub).await.unwrap();
    sync_once(&w.desktop, &w.hub).await.unwrap();

    for s in [&w.server, &w.phone, &w.desktop] {
        assert_eq!(
            s.entry(id).await.unwrap().status(),
            EntryStatus::Trashed { at: t(20) }
        );
    }
}

#[tokio::test]
async fn decided_suggestion_is_not_reverted_by_stale_copy() {
    let w = world().await;
    let id = capture(&w.phone, "Kostik ara", t(0), None).await;
    sync_once(&w.phone, &w.hub).await.unwrap();
    let s = Suggestion::new(
        id,
        SuggestionKind::Tag {
            name: Label::new("tedarik").unwrap(),
        },
        Confidence::new(0.8).unwrap(),
    );
    w.server
        .save_ai_suggestions(id, std::slice::from_ref(&s), t(1))
        .await
        .unwrap();
    sync_once(&w.phone, &w.hub).await.unwrap();
    sync_once(&w.desktop, &w.hub).await.unwrap();

    // Telefon yoksayar; masaüstündeki "bekliyor" kopyası daha sonra (yeni zamanla) bir şey
    // yazmasa da kararı ezemez.
    w.phone.decide(s.id, false, t(5), TZ).await.unwrap();
    sync_once(&w.phone, &w.hub).await.unwrap();
    sync_once(&w.desktop, &w.hub).await.unwrap();
    assert!(!w.desktop.suggestions_for(id).await.unwrap()[0].is_pending());
    assert!(!w.server.suggestions_for(id).await.unwrap()[0].is_pending());
}

#[tokio::test]
async fn same_topic_name_created_offline_converges() {
    let w = world().await;
    // Telefon önce oluşturur (kimliği küçük), masaüstü sonra; ikisi de çevrimdışı.
    let first = topic(&w.phone, "Ev", t(0)).await;
    let second = topic(&w.desktop, "Ev", t(1)).await;
    capture(&w.phone, "Kombi bakımı", t(2), Some(first)).await;
    capture(&w.desktop, "Fatura öde", t(3), Some(second)).await;

    for _ in 0..2 {
        sync_once(&w.phone, &w.hub).await.unwrap();
        sync_once(&w.desktop, &w.hub).await.unwrap();
    }
    for s in [&w.server, &w.phone, &w.desktop] {
        let mut names: Vec<(TopicId, String)> = s
            .topic_list(KindFilter::All)
            .await
            .unwrap()
            .into_iter()
            .map(|a| (a.topic.id, a.topic.name.as_str().to_owned()))
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![(first, "Ev".to_owned()), (second, "Ev (2)".to_owned())]
        );
        assert_eq!(texts(s).await.len(), 2, "iki konunun kayıtları da korundu");
    }
}

#[tokio::test]
async fn push_cannot_impersonate_another_device() {
    let w = world().await;
    let forged = ChangeRecord {
        seq: 1,
        entity: "entry".into(),
        entity_id: EntryId::new().to_string(),
        payload: None,
        origin: "masaustu".into(),
        at: t(0),
    };
    let err = handle_push(
        &w.server,
        &PushRequest {
            device: "telefon".into(),
            changes: vec![forged],
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(err, SyncError::Rejected(_)));
    let err = handle_push(
        &w.server,
        &PushRequest {
            device: "sunucu".into(),
            changes: vec![],
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(err, SyncError::Rejected(_)));
}

#[tokio::test]
async fn large_backlog_syncs_in_batches_and_resumes() {
    let w = world().await;
    for i in 0..450 {
        capture(&w.phone, &format!("Not {i}"), t(i), None).await;
    }
    let r = sync_once(&w.phone, &w.hub).await.unwrap();
    assert_eq!(r.pushed, 450);
    let r = sync_once(&w.desktop, &w.hub).await.unwrap();
    assert_eq!(r.pulled_applied, 450);
    assert_eq!(
        w.desktop
            .journal(JournalFilter::All, None, 500)
            .await
            .unwrap()
            .len(),
        450
    );
}
