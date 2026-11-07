//! notai-store entegrasyon testleri: gerçek SQLite (bellek ve dosya) üzerinde.

// Yalnız test kodu: yardımcı fonksiyonlarda unwrap/panic serbest (üretim kodunda yasak).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use notai_core::entry::{EntryStatus, Source};
use notai_core::journal::JournalFilter;
use notai_core::morning::PlanReason;
use notai_core::suggestion::{Confidence, Suggestion, SuggestionKind, TopicTarget};
use notai_core::task::{Due, NewTask, Priority, Recurrence};
use notai_core::text::{EntryText, Label, TaskTitle, TopicName};
use notai_core::topic::{KindFilter, ParaKind};
use notai_core::{EntryId, TopicId};
use notai_store::{Captured, NewEntry, NewTopic, Store, StoreError, TaskView};

const TZ: Tz = chrono_tz::Europe::Istanbul;

fn at(day: u32, h: u32, m: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, day, h, m, 0).unwrap()
}

fn now() -> DateTime<Utc> {
    at(6, 9, 7)
}

async fn store() -> Store {
    Store::open_in_memory("test-cihaz").await.unwrap()
}

fn new_entry(text: &str, captured_at: DateTime<Utc>, topic: Option<TopicId>) -> NewEntry {
    NewEntry {
        text: EntryText::new(text).unwrap(),
        source: Source::App,
        topic,
        captured_at,
        dedupe_key: None,
    }
}

async fn capture(s: &Store, text: &str, when: DateTime<Utc>) -> EntryId {
    match s.capture(new_entry(text, when, None)).await.unwrap() {
        Captured::New(e) => e.id,
        Captured::Duplicate(_) => panic!("yeni kayıt bekleniyordu"),
    }
}

async fn topic(s: &Store, name: &str, kind: ParaKind) -> TopicId {
    s.create_topic(NewTopic::new(TopicName::new(name).unwrap(), kind), now())
        .await
        .unwrap()
        .id
}

fn conf(v: f64) -> Confidence {
    Confidence::new(v).unwrap()
}

#[tokio::test]
async fn capture_and_journal_newest_last() {
    let s = store().await;
    capture(
        &s,
        "Zeytinyağlı sabunda kür süresi 4–6 hafta",
        at(4, 10, 30),
    )
    .await;
    capture(&s, "Kostik tedarikçisini ara – perşembe", at(6, 6, 12)).await;
    capture(
        &s,
        "Pazar gözden geçirmede iş–ev dengesini konuş",
        at(6, 9, 5),
    )
    .await;

    let items = s.journal(JournalFilter::All, None, 50).await.unwrap();
    let texts: Vec<_> = items.iter().map(|i| i.entry.text.as_str()).collect();
    assert_eq!(
        texts,
        [
            "Zeytinyağlı sabunda kür süresi 4–6 hafta",
            "Kostik tedarikçisini ara – perşembe",
            "Pazar gözden geçirmede iş–ev dengesini konuş",
        ]
    );
    assert!(items.iter().all(|i| i.entry.is_unprocessed()));
    assert_eq!(s.unprocessed_count().await.unwrap(), 3);

    // Sayfalama: en yeni 2, sonra imleçten öncekiler.
    let page = s.journal(JournalFilter::All, None, 2).await.unwrap();
    assert_eq!(page.len(), 2);
    assert_eq!(
        page[0].entry.text.as_str(),
        "Kostik tedarikçisini ara – perşembe"
    );
    let older = s
        .journal(JournalFilter::All, Some(page[0].entry.captured_at), 2)
        .await
        .unwrap();
    assert_eq!(older.len(), 1);
}

#[tokio::test]
async fn capture_into_topic_is_processed_and_visible_in_both_flows() {
    let s = store().await;
    let gem = topic(&s, "GemOlive", ParaKind::Area).await;
    let e = s
        .capture(new_entry(
            "Ambalaj faturası · 1.850 ₺",
            at(6, 7, 47),
            Some(gem),
        ))
        .await
        .unwrap();
    assert_eq!(e.entry().status(), EntryStatus::Processed);

    let all = s.journal(JournalFilter::All, None, 10).await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].topic.as_ref().unwrap().name.as_str(), "GemOlive");
    let topic_flow = s
        .journal(JournalFilter::Topic { id: gem }, None, 10)
        .await
        .unwrap();
    assert_eq!(topic_flow.len(), 1);
    assert!(
        s.journal(JournalFilter::Unprocessed, None, 10)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn capture_into_missing_topic_fails_and_writes_nothing() {
    let s = store().await;
    let err = s
        .capture(new_entry("Hayalet konu", now(), Some(TopicId::new())))
        .await
        .unwrap_err();
    assert!(matches!(err, StoreError::NotFound { what: "konu", .. }));
    assert!(
        s.journal(JournalFilter::All, None, 10)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn whatsapp_duplicates_are_idempotent() {
    let s = store().await;
    let mut n = new_entry("Numune yolla", now(), None);
    n.source = Source::WhatsApp;
    n.dedupe_key = Some("wa:abc".into());
    let first = s.capture(n.clone()).await.unwrap();
    let second = s.capture(n).await.unwrap();
    assert!(matches!(first, Captured::New(_)));
    assert!(matches!(second, Captured::Duplicate(_)));
    assert_eq!(first.entry().id, second.entry().id);
    assert_eq!(
        s.journal(JournalFilter::All, None, 10).await.unwrap().len(),
        1
    );
}

#[tokio::test]
async fn topic_names_are_unique_with_turkish_case_folding() {
    let s = store().await;
    topic(&s, "İş", ParaKind::Area).await;
    let err = s
        .create_topic(
            NewTopic::new(TopicName::new("iş").unwrap(), ParaKind::Area),
            now(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, StoreError::Conflict(_)));
    // "Is" (noktasız büyük I → ı) farklı bir addır.
    topic(&s, "Is", ParaKind::Resource).await;
    let found = s
        .topic_by_name(&TopicName::new("İŞ").unwrap())
        .await
        .unwrap();
    assert_eq!(found.unwrap().name.as_str(), "İş");
}

#[tokio::test]
async fn topic_list_orders_by_pin_then_activity_with_preview() {
    let s = store().await;
    let ev = topic(&s, "Ev", ParaKind::Area).await;
    let gem = topic(&s, "GemOlive", ParaKind::Area).await;
    let _okuma = topic(&s, "Okuma notları", ParaKind::Resource).await;
    let logo = topic(&s, "Logo ERP geçişi", ParaKind::Project { due: None }).await;
    s.capture(new_entry(
        "Kombi bakımı – 14 Kasım Cumartesi",
        at(5, 5, 15),
        Some(ev),
    ))
    .await
    .unwrap();
    s.capture(new_entry("Ambalaj faturası", at(6, 7, 47), Some(gem)))
        .await
        .unwrap();

    let mut sabit = s.topic(logo).await.unwrap();
    sabit.pinned = true;
    s.update_topic(&sabit, now()).await.unwrap();

    let list = s.topic_list(KindFilter::All).await.unwrap();
    let names: Vec<_> = list.iter().map(|a| a.topic.name.as_str()).collect();
    assert_eq!(
        names,
        ["Logo ERP geçişi", "GemOlive", "Ev", "Okuma notları"]
    );
    assert_eq!(list[1].last_preview.as_deref(), Some("Ambalaj faturası"));

    let projects = s.topic_list(KindFilter::Projects).await.unwrap();
    assert_eq!(projects.len(), 1);
}

#[tokio::test]
async fn topic_icon_follows_entries_as_they_arrive() {
    use notai_core::look::icon_for;
    let s = store().await;
    let gem = topic(&s, "GemOlive", ParaKind::Area).await;
    let icon = |list: &[notai_core::journal::TopicActivity]| {
        let a = list.iter().find(|a| a.topic.id == gem).unwrap();
        icon_for(&a.topic, &a.recent_texts)
    };
    assert_eq!(
        icon(&s.topic_list(KindFilter::All).await.unwrap()),
        "layers",
        "kayıt yok: tür"
    );
    s.capture(new_entry(
        "Zeytinyağlı sabun denemesi",
        at(5, 8, 0),
        Some(gem),
    ))
    .await
    .unwrap();
    assert_eq!(
        icon(&s.topic_list(KindFilter::All).await.unwrap()),
        "layers",
        "tek kayıt yetmez"
    );
    s.capture(new_entry("Sabun kalıbı ölçüsü", at(6, 8, 0), Some(gem)))
        .await
        .unwrap();
    let list = s.topic_list(KindFilter::All).await.unwrap();
    let a = list.iter().find(|a| a.topic.id == gem).unwrap();
    assert_eq!(a.recent_texts[0], "Sabun kalıbı ölçüsü", "en yeni önce");
    assert_eq!(icon(&list), "drop");
}

#[tokio::test]
async fn ai_suggestions_then_decisions_process_the_entry() {
    let s = store().await;
    let kostik = topic(&s, "Kostik tedarik", ParaKind::Project { due: None }).await;
    let id = capture(
        &s,
        "Kostik tedarikçisini ara – perşembe. 25 kg'lık torba fiyatını sor.",
        at(6, 6, 12),
    )
    .await;

    let due = Due::at(
        NaiveDate::from_ymd_opt(2026, 11, 12).unwrap(),
        NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
    );
    let suggestions = vec![
        Suggestion::new(
            id,
            SuggestionKind::Task {
                title: TaskTitle::new("Kostik firmasını ara").unwrap(),
                due: Some(due),
                priority: Priority::High,
            },
            conf(0.92),
        ),
        Suggestion::new(
            id,
            SuggestionKind::Topic {
                target: TopicTarget::Existing {
                    id: kostik,
                    name: TopicName::new("Kostik tedarik").unwrap(),
                },
            },
            conf(0.88),
        ),
        Suggestion::new(
            id,
            SuggestionKind::Person {
                name: Label::new("Ayhan Kimya").unwrap(),
            },
            conf(0.6),
        ),
    ];
    s.save_ai_suggestions(id, &suggestions, now())
        .await
        .unwrap();
    assert!(
        s.next_ai_job(now()).await.unwrap().is_none(),
        "iş bitti, kuyruk boş"
    );

    // 1) Görev kabul: görev oluşur, kayıt hâlâ işlenmemiş (2 öneri bekliyor).
    let out = s.decide(suggestions[0].id, true, now(), TZ).await.unwrap();
    let task = out.created_task.unwrap();
    assert_eq!(task.title.as_str(), "Kostik firmasını ara");
    assert_eq!(task.entry, Some(id));
    assert!(out.entry.is_unprocessed());

    // 2) Konu kabul: kayıt konuya taşınır → kullanıcı sınıflandırdı → işlendi.
    let out = s.decide(suggestions[1].id, true, now(), TZ).await.unwrap();
    assert_eq!(out.entry.topic(), Some(kostik));
    assert_eq!(out.entry.status(), EntryStatus::Processed);

    // 3) Kişi yoksay: tek öneri reddedilir, diğerleri etkilenmez.
    let out = s.decide(suggestions[2].id, false, now(), TZ).await.unwrap();
    assert!(!out.suggestion.is_pending());

    // Aynı öneriye ikinci karar reddedilir.
    let err = s
        .decide(suggestions[2].id, true, now(), TZ)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        StoreError::Core(notai_core::CoreError::AlreadyDecided)
    ));

    let items = s.journal(JournalFilter::All, None, 10).await.unwrap();
    assert_eq!(items[0].suggestions.len(), 3);
    assert!(items[0].suggestions.iter().all(|s| !s.is_pending()));
    assert!(items[0].people.is_empty(), "yoksayılan kişi eklenmedi");
}

#[tokio::test]
async fn accepting_new_topic_creates_it_once_and_tags_are_labels() {
    let s = store().await;
    let a = capture(&s, "Lavanta uçucu yağı 2 kg kaldı", at(4, 18, 5)).await;
    let b = capture(&s, "Lavanta siparişi verildi", at(5, 8, 0)).await;
    let mk = |entry| {
        Suggestion::new(
            entry,
            SuggestionKind::Topic {
                target: TopicTarget::New {
                    name: TopicName::new("Hammadde").unwrap(),
                    kind: ParaKind::Area,
                },
            },
            conf(0.7),
        )
    };
    let (sa, sb) = (mk(a), mk(b));
    let tag = Suggestion::new(
        a,
        SuggestionKind::Tag {
            name: Label::new("sabun").unwrap(),
        },
        conf(0.8),
    );
    s.save_ai_suggestions(a, &[sa.clone(), tag.clone()], now())
        .await
        .unwrap();
    s.save_ai_suggestions(b, std::slice::from_ref(&sb), now())
        .await
        .unwrap();

    let first = s.decide(sa.id, true, now(), TZ).await.unwrap();
    let created = first.created_topic.unwrap();
    let second = s.decide(sb.id, true, now(), TZ).await.unwrap();
    assert!(
        second.created_topic.is_none(),
        "aynı adla ikinci konu açılmaz"
    );
    assert_eq!(second.entry.topic(), Some(created.id));

    s.decide(tag.id, true, now(), TZ).await.unwrap();
    let items = s
        .journal(JournalFilter::Topic { id: created.id }, None, 10)
        .await
        .unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].tags, ["sabun"]);
}

#[tokio::test]
async fn empty_ai_result_processes_inbox_entry() {
    let s = store().await;
    let id = capture(&s, "Bugün hava güzel", now()).await;
    s.save_ai_suggestions(id, &[], now()).await.unwrap();
    assert_eq!(s.entry(id).await.unwrap().status(), EntryStatus::Processed);
}

#[tokio::test]
async fn foreign_suggestion_rolls_back_whole_batch() {
    let s = store().await;
    let a = capture(&s, "Kayıt A", now()).await;
    let b = capture(&s, "Kayıt B", now()).await;
    let good = Suggestion::new(
        a,
        SuggestionKind::Tag {
            name: Label::new("a").unwrap(),
        },
        conf(0.5),
    );
    let foreign = Suggestion::new(
        b,
        SuggestionKind::Tag {
            name: Label::new("b").unwrap(),
        },
        conf(0.5),
    );
    let err = s
        .save_ai_suggestions(a, &[good, foreign], now())
        .await
        .unwrap_err();
    assert!(matches!(err, StoreError::Conflict(_)));
    assert!(
        s.suggestions_for(a).await.unwrap().is_empty(),
        "transaction geri alındı"
    );
    assert!(
        s.next_ai_job(now()).await.unwrap().is_some(),
        "iş hâlâ kuyrukta"
    );
}

#[tokio::test]
async fn ai_failures_back_off_then_fail_without_losing_the_entry() {
    let s = store().await;
    let id = capture(&s, "Şirket aracının muayenesi ne zamandı?", at(6, 8, 30)).await;
    let t0 = now();
    assert_eq!(s.next_ai_job(t0).await.unwrap().unwrap().id, id);

    assert!(
        !s.record_ai_failure(id, "zaman aşımı", true, t0)
            .await
            .unwrap()
    );
    assert!(s.next_ai_job(t0).await.unwrap().is_none(), "1 dk beklemeli");
    let t1 = t0 + Duration::minutes(1);
    assert!(s.next_ai_job(t1).await.unwrap().is_some());
    assert!(
        !s.record_ai_failure(id, "zaman aşımı", true, t1)
            .await
            .unwrap()
    );
    let t2 = t1 + Duration::minutes(5);
    assert!(
        !s.record_ai_failure(id, "şema hatası", true, t2)
            .await
            .unwrap()
    );
    let t3 = t2 + Duration::minutes(25);
    assert!(
        s.record_ai_failure(id, "şema hatası", true, t3)
            .await
            .unwrap(),
        "denemeler tükendi"
    );
    assert!(
        s.next_ai_job(t3 + Duration::days(1))
            .await
            .unwrap()
            .is_none()
    );

    let items = s.journal(JournalFilter::All, None, 10).await.unwrap();
    assert!(items[0].ai_failed);
    assert!(
        items[0].entry.is_unprocessed(),
        "kayıt kaybolmadı, gelen kutusunda"
    );

    s.retry_ai(id, t3).await.unwrap();
    assert!(s.next_ai_job(t3).await.unwrap().is_some());
    // Kalıcı hata (ör. model reddi) beklemeden başarısız sayılır.
    assert!(
        s.record_ai_failure(id, "reddedildi", false, t3)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn trash_restore_and_purge_after_30_days() {
    let s = store().await;
    let keep = capture(&s, "Geri alınacak", now()).await;
    let gone = capture(&s, "Kalıcı silinecek", now()).await;
    s.trash_entry(keep, now()).await.unwrap();
    s.trash_entry(gone, now()).await.unwrap();
    assert!(
        s.journal(JournalFilter::All, None, 10)
            .await
            .unwrap()
            .is_empty()
    );

    s.restore_entry(keep, now()).await.unwrap();
    assert_eq!(s.purge_trash(now() + Duration::days(29)).await.unwrap(), 0);
    assert_eq!(s.purge_trash(now() + Duration::days(30)).await.unwrap(), 1);
    assert!(matches!(
        s.entry(gone).await.unwrap_err(),
        StoreError::NotFound { .. }
    ));
    assert_eq!(s.entry(keep).await.unwrap().status(), EntryStatus::Inbox);
}

#[tokio::test]
async fn archive_hides_from_journal_but_search_finds_it() {
    let s = store().await;
    let id = capture(&s, "Kostik için en son 1.850 ₺ teklif aldım", at(3, 10, 0)).await;
    s.archive_entry(id, now()).await.unwrap();
    assert!(
        s.journal(JournalFilter::All, None, 10)
            .await
            .unwrap()
            .is_empty()
    );
    let hits = s.search("kostik teklif", 10).await.unwrap();
    assert_eq!(hits.len(), 1);
    // Aksan katlama ve önek: "tedarikci" → "tedarikçisini".
    capture(&s, "Kostik tedarikçisini ara", now()).await;
    assert_eq!(s.search("tedarikci", 10).await.unwrap().len(), 1);
    assert!(s.search("   ", 10).await.unwrap().is_empty());
    assert!(
        s.search("\"\" OR", 10).await.is_ok(),
        "FTS sözdizimi kullanıcı girdisinden korunur"
    );
}

#[tokio::test]
async fn move_entry_between_topics() {
    let s = store().await;
    let ev = topic(&s, "Ev", ParaKind::Area).await;
    let id = capture(&s, "Kombi bakımı", now()).await;
    let moved = s.move_entry(id, Some(ev), now()).await.unwrap();
    assert_eq!(moved.topic(), Some(ev));
    assert_eq!(moved.status(), EntryStatus::Processed);
    assert!(s.move_entry(id, Some(TopicId::new()), now()).await.is_err());
}

#[tokio::test]
async fn tasks_views_completion_and_morning_plan() {
    let s = store().await;
    let today = NaiveDate::from_ymd_opt(2026, 11, 6).unwrap();
    let mk = |title: &str, due: Option<Due>, p: Priority| {
        let mut n = NewTask::titled(TaskTitle::new(title).unwrap());
        n.due = due;
        n.priority = p;
        n
    };
    let overdue = s
        .create_task(
            mk(
                "Kostik tekliflerini karşılaştır",
                Some(Due::on(today - Duration::days(1))),
                Priority::Normal,
            ),
            now(),
        )
        .await
        .unwrap();
    let mut walk = mk(
        "19:00 akşam yürüyüşü",
        Some(Due::at(today, NaiveTime::from_hms_opt(19, 0, 0).unwrap())),
        Priority::Normal,
    );
    walk.recurrence = Some(Recurrence::Day);
    let walk = s.create_task(walk, now()).await.unwrap();
    s.create_task(
        mk(
            "Logo: 37 hatalı stok kartını düzelt",
            Some(Due::on(today)),
            Priority::High,
        ),
        now(),
    )
    .await
    .unwrap();
    s.create_task(mk("Numune yolla", None, Priority::High), now())
        .await
        .unwrap();

    assert_eq!(s.tasks(TaskView::Overdue, today).await.unwrap().len(), 1);
    assert_eq!(s.tasks(TaskView::Today, today).await.unwrap().len(), 2);
    assert_eq!(s.tasks(TaskView::Undated, today).await.unwrap().len(), 1);

    let plan = s.morning_plan(today).await.unwrap();
    assert_eq!(plan.items.len(), 3);
    assert_eq!(plan.items[0].task, overdue.id);
    assert_eq!(plan.items[0].reason, PlanReason::Overdue { days: 1 });
    assert_eq!(plan.more, 1);

    let (done, next) = s.complete_task(walk.id, now()).await.unwrap();
    assert!(!done.is_open());
    let next = next.unwrap();
    assert_eq!(next.due.unwrap().date, today + Duration::days(1));
    assert_eq!(s.tasks(TaskView::Upcoming, today).await.unwrap().len(), 1);
    assert!(s.complete_task(walk.id, now()).await.is_err());

    let reopened = s.reopen_task(walk.id, now()).await.unwrap();
    assert!(reopened.is_open());
}

#[tokio::test]
async fn change_log_records_every_write_in_order() {
    let s = store().await;
    let gem = topic(&s, "GemOlive", ParaKind::Area).await;
    let id = capture(&s, "Ambalaj faturası", now()).await;
    s.move_entry(id, Some(gem), now()).await.unwrap();
    let changes = s.changes_since(0, 100).await.unwrap();
    let kinds: Vec<_> = changes.iter().map(|c| c.entity.as_str()).collect();
    assert_eq!(kinds, ["topic", "entry", "entry"]);
    assert!(changes.iter().all(|c| c.origin == "test-cihaz"));
    assert!(changes.windows(2).all(|w| w[0].seq < w[1].seq));
    let later = s.changes_since(changes[1].seq, 100).await.unwrap();
    assert_eq!(later.len(), 1);
}

#[tokio::test]
async fn kv_and_facts() {
    use notai_core::profile::{FactStatus, ProfileFact};
    use notai_core::text::FactText;
    let s = store().await;
    assert_eq!(s.kv_get("morning_plan_last_sent").await.unwrap(), None);
    s.kv_set("morning_plan_last_sent", "2026-11-06")
        .await
        .unwrap();
    s.kv_set("morning_plan_last_sent", "2026-11-07")
        .await
        .unwrap();
    assert_eq!(
        s.kv_get("morning_plan_last_sent").await.unwrap().as_deref(),
        Some("2026-11-07")
    );

    let f = ProfileFact::proposed(
        FactText::new("GemOlive = sabun markası, Alan: İş").unwrap(),
        None,
    );
    s.add_fact(&f, now()).await.unwrap();
    s.confirm_fact(f.id, now()).await.unwrap();
    let facts = s.facts().await.unwrap();
    assert_eq!(facts[0].status, FactStatus::Confirmed);
    s.delete_fact(f.id, now()).await.unwrap();
    assert!(s.facts().await.unwrap().is_empty());
}

#[tokio::test]
async fn file_database_persists_across_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("notai.db").display());
    let id = {
        let s = Store::open(&url, "telefon").await.unwrap();
        let id = capture(&s, "Kalıcı mı?", now()).await;
        s.close().await;
        id
    };
    let s = Store::open(&url, "telefon").await.unwrap();
    assert_eq!(s.entry(id).await.unwrap().text.as_str(), "Kalıcı mı?");
}

#[tokio::test]
async fn timestamps_round_trip_exactly() {
    let s = store().await;
    let precise =
        Utc.with_ymd_and_hms(2026, 11, 6, 6, 12, 0).unwrap() + Duration::nanoseconds(123_456_789);
    let captured = s
        .capture(new_entry("Hassas zaman", precise, None))
        .await
        .unwrap();
    let loaded = s.entry(captured.entry().id).await.unwrap();
    assert_eq!(&loaded, captured.entry());
}

#[tokio::test]
async fn topic_list_counts_entries_waiting_for_a_decision() {
    let s = store().await;
    let kostik = topic(&s, "Kostik tedarik", ParaKind::Area).await;
    let id = s
        .capture(new_entry("Ayhan Kimya teklifi", at(6, 8, 0), Some(kostik)))
        .await
        .unwrap()
        .entry()
        .id;
    let pending = |list: &[notai_core::journal::TopicActivity]| {
        list.iter().find(|a| a.topic.id == kostik).unwrap().pending
    };
    assert_eq!(pending(&s.topic_list(KindFilter::All).await.unwrap()), 0);
    let tag = |n: &str| {
        Suggestion::new(
            id,
            SuggestionKind::Tag {
                name: Label::new(n).unwrap(),
            },
            conf(0.8),
        )
    };
    let (a, b) = (tag("teklif"), tag("kostik"));
    s.save_ai_suggestions(id, &[a.clone(), b.clone()], at(6, 8, 1))
        .await
        .unwrap();
    assert_eq!(
        pending(&s.topic_list(KindFilter::All).await.unwrap()),
        1,
        "iki öneri, tek kayıt"
    );
    s.decide(a.id, true, at(6, 8, 2), chrono_tz::Europe::Istanbul)
        .await
        .unwrap();
    assert_eq!(pending(&s.topic_list(KindFilter::All).await.unwrap()), 1);
    s.decide(b.id, false, at(6, 8, 3), chrono_tz::Europe::Istanbul)
        .await
        .unwrap();
    assert_eq!(pending(&s.topic_list(KindFilter::All).await.unwrap()), 0);
}
