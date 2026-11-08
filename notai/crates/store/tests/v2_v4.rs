//! v0.2–v0.4 depo işlevleri: hatırlatma, takvim, gözden geçirme, ekler, vektörler, geri alma.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use notai_core::agenda::weekly_review;
use notai_core::entry::{EntryStatus, Source};
use notai_core::reminders::{Snooze, due_now};
use notai_core::suggestion::{Confidence, Suggestion, SuggestionKind, TopicTarget};
use notai_core::task::{Due, NewTask, Priority, Reminder, TaskState};
use notai_core::text::{EntryText, Label, TaskTitle, TopicName};
use notai_core::topic::ParaKind;
use notai_core::{EntryId, TopicId};
use notai_store::changes::ChangeRecord;
use notai_store::{AttachmentKind, NewAttachment, NewEntry, NewTopic, Store, StoreError};

const TZ: Tz = chrono_tz::Europe::Istanbul;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 6, 9, 0, 0).unwrap()
}

async fn store(origin: &str) -> Store {
    Store::open_in_memory(origin).await.unwrap()
}

async fn capture(s: &Store, text: &str, topic: Option<TopicId>) -> EntryId {
    s.capture(NewEntry {
        text: EntryText::new(text).unwrap(),
        source: Source::App,
        topic,
        captured_at: now(),
        dedupe_key: None,
    })
    .await
    .unwrap()
    .entry()
    .id
}

fn conf() -> Confidence {
    Confidence::new(0.95).unwrap()
}

#[tokio::test]
async fn reminders_fire_once_and_snooze() {
    let s = store("telefon").await;
    let mut n = NewTask::titled(TaskTitle::new("Muayene tarihini kontrol et").unwrap());
    n.reminder = Some(Reminder::At {
        at: now() - Duration::minutes(2),
    });
    let t = s.create_task(n, now()).await.unwrap();
    s.create_task(
        NewTask::titled(TaskTitle::new("Hatırlatmasız").unwrap()),
        now(),
    )
    .await
    .unwrap();

    let list = s.reminder_tasks().await.unwrap();
    assert_eq!(list.len(), 1);
    let due = due_now(&list, now(), TZ);
    assert_eq!(due.len(), 1);
    s.mark_reminder_fired(t.id, due[0].1).await.unwrap();
    assert!(
        due_now(&s.reminder_tasks().await.unwrap(), now(), TZ).is_empty(),
        "bir kez çalar"
    );

    let snoozed = s
        .snooze_reminder(t.id, Snooze::Minutes10, now(), TZ)
        .await
        .unwrap();
    assert_eq!(
        snoozed.reminder,
        Some(Reminder::At {
            at: now() + Duration::minutes(10)
        })
    );
    let later = now() + Duration::minutes(11);
    assert_eq!(
        due_now(&s.reminder_tasks().await.unwrap(), later, TZ).len(),
        1,
        "ertelenen yeniden çalar"
    );
}

#[tokio::test]
async fn calendar_range_and_review_inputs() {
    let s = store("telefon").await;
    let logo = s
        .create_topic(
            NewTopic::new(
                TopicName::new("Logo ERP geçişi").unwrap(),
                ParaKind::Project { due: None },
            ),
            now() - Duration::days(20),
        )
        .await
        .unwrap();
    let d = |day| NaiveDate::from_ymd_opt(2026, 11, day).unwrap();
    for (title, day) in [("Pazartesi", 2), ("Cuma", 6), ("Gelecek ay", 30)] {
        let mut n = NewTask::titled(TaskTitle::new(title).unwrap());
        n.due = Some(Due::on(d(day)));
        s.create_task(n, now()).await.unwrap();
    }
    let week = s.tasks_between(d(2), d(8)).await.unwrap();
    assert_eq!(week.len(), 2);

    let done = s.tasks_between(d(6), d(6)).await.unwrap()[0].id;
    s.complete_task(done, now()).await.unwrap();
    assert_eq!(
        s.tasks_between(d(6), d(6)).await.unwrap().len(),
        1,
        "tamamlananlar takvimde kalır"
    );
    assert_eq!(s.done_since(now() - Duration::days(7)).await.unwrap(), 1);

    let activity = s.topic_activity().await.unwrap();
    assert_eq!(activity.len(), 1);
    assert_eq!(activity[0].topic.id, logo.id);
    let review = weekly_review(0, &activity, &s.open_tasks().await.unwrap(), 1, now(), TZ);
    assert_eq!(review.stalled.len(), 1, "20 gündür hareketsiz proje");
}

#[tokio::test]
async fn attachments_store_read_and_sync() {
    let phone = store("telefon").await;
    let server = store("sunucu").await;
    let id = capture(&phone, "Ambalaj faturası", None).await;
    let png = vec![0x89, b'P', b'N', b'G', 1, 2, 3];
    let meta = phone
        .add_attachment(
            NewAttachment {
                entry: id,
                kind: AttachmentKind::Photo,
                name: "fatura.png".into(),
                mime: "image/png".into(),
                data: Some(png.clone()),
                value: None,
            },
            now(),
        )
        .await
        .unwrap();
    phone
        .add_attachment(
            NewAttachment {
                entry: id,
                kind: AttachmentKind::Location,
                name: "Konum".into(),
                mime: "text/plain".into(),
                data: None,
                value: Some("41.0082,28.9784".into()),
            },
            now(),
        )
        .await
        .unwrap();
    assert_eq!(meta.size, 7);
    assert_eq!(phone.attachments_for(id).await.unwrap().len(), 2);
    assert_eq!(
        phone.attachment_data(&meta.id).await.unwrap(),
        ("image/png".into(), png.clone())
    );

    let too_big = phone
        .add_attachment(
            NewAttachment {
                entry: id,
                kind: AttachmentKind::Document,
                name: "büyük.pdf".into(),
                mime: "application/pdf".into(),
                data: Some(vec![0; notai_store::MAX_ATTACHMENT_BYTES + 1]),
                value: None,
            },
            now(),
        )
        .await;
    assert!(matches!(too_big, Err(StoreError::Conflict(_))));

    // Senkron: ek içeriğiyle birlikte sunucuya taşınır.
    for c in phone.own_changes_since(0, 100).await.unwrap() {
        server.apply_remote(&c, false).await.unwrap();
    }
    assert_eq!(server.attachment_data(&meta.id).await.unwrap().1, png);
}

#[tokio::test]
async fn embeddings_round_trip_and_queue() {
    let s = store("sunucu").await;
    let a = capture(&s, "Kostik fiyatı", None).await;
    let _b = capture(&s, "Kombi bakımı", None).await;
    assert_eq!(
        s.entries_without_embedding("m1", 10).await.unwrap().len(),
        2
    );
    s.save_embedding(a, "m1", &[0.5, -1.0, 2.25], now())
        .await
        .unwrap();
    assert_eq!(
        s.entries_without_embedding("m1", 10).await.unwrap().len(),
        1
    );
    assert_eq!(
        s.entries_without_embedding("m2", 10).await.unwrap().len(),
        2,
        "model değişince yeniden"
    );
    let v = s.embeddings("m1").await.unwrap();
    assert_eq!(v, vec![(a, vec![0.5, -1.0, 2.25])]);
    assert!(s.save_embedding(a, "m1", &[], now()).await.is_err());
}

#[tokio::test]
async fn undo_reverses_topic_task_and_label() {
    let s = store("sunucu").await;
    let kostik = s
        .create_topic(
            NewTopic::new(TopicName::new("Kostik tedarik").unwrap(), ParaKind::Area),
            now(),
        )
        .await
        .unwrap();
    let id = capture(&s, "Kostik tedarikçisini ara – perşembe", None).await;
    let topic = Suggestion::new(
        id,
        SuggestionKind::Topic {
            target: TopicTarget::Existing {
                id: kostik.id,
                name: kostik.name.clone(),
            },
        },
        conf(),
    );
    let task = Suggestion::new(
        id,
        SuggestionKind::Task {
            title: TaskTitle::new("Kostik firmasını ara").unwrap(),
            due: None,
            priority: Priority::High,
        },
        conf(),
    );
    let tag = Suggestion::new(
        id,
        SuggestionKind::Tag {
            name: Label::new("tedarik").unwrap(),
        },
        conf(),
    );
    s.save_ai_suggestions(id, &[topic.clone(), task.clone(), tag.clone()], now())
        .await
        .unwrap();

    // Otonom mod üçünü de uygular.
    for x in [&topic, &task, &tag] {
        s.decide_with(x.id, true, true, now(), TZ).await.unwrap();
    }
    assert_eq!(s.entry(id).await.unwrap().topic(), Some(kostik.id));
    assert!(
        s.decision_examples(10).await.unwrap().is_empty(),
        "otonom kararlar örnek sayılmaz"
    );

    // Kullanıcı konu ve görevi geri alır.
    let restored = s.undo_decision(topic.id, now()).await.unwrap();
    assert_eq!(restored.topic(), None);
    assert_eq!(restored.status(), EntryStatus::Inbox);
    s.undo_decision(task.id, now()).await.unwrap();
    let open: Vec<_> = s.open_tasks().await.unwrap();
    assert!(open.is_empty(), "oluşan görev iptal edildi");
    s.undo_decision(tag.id, now()).await.unwrap();
    let suggs = s.suggestions_for(id).await.unwrap();
    assert!(suggs.iter().all(|x| x.decision().name() == "dismissed"));
    // İkinci geri alma reddedilir.
    assert!(s.undo_decision(topic.id, now()).await.is_err());

    // Kullanıcının kendi kararları öğrenme örneği olur.
    let id2 = capture(&s, "Lavanta yağı siparişi", None).await;
    let t2 = Suggestion::new(
        id2,
        SuggestionKind::Tag {
            name: Label::new("sabun").unwrap(),
        },
        conf(),
    );
    s.save_ai_suggestions(id2, std::slice::from_ref(&t2), now())
        .await
        .unwrap();
    s.decide(t2.id, true, now(), TZ).await.unwrap();
    let ex = s.decision_examples(10).await.unwrap();
    assert_eq!(ex.len(), 1);
    assert!(ex[0].accepted);
    assert_eq!(ex[0].entry_text, "Lavanta yağı siparişi");
    let _ = TaskState::Open;
}

#[tokio::test]
async fn trash_list_and_purge_only_trashed() {
    let s = store("telefon").await;
    let a = capture(&s, "Silinecek", None).await;
    let b = capture(&s, "Kalacak", None).await;
    assert!(
        s.purge_entry(b, now()).await.is_err(),
        "çöpte olmayan kalıcı silinmez"
    );
    s.trash_entry(a, now()).await.unwrap();
    assert_eq!(s.trash().await.unwrap().len(), 1);
    s.purge_entry(a, now()).await.unwrap();
    assert!(s.trash().await.unwrap().is_empty());
    let last: ChangeRecord = s.changes_since(0, 100).await.unwrap().pop().unwrap();
    assert_eq!((last.entity.as_str(), last.payload), ("entry", None));
}

#[tokio::test]
async fn statement_import_is_deduplicated_matched_and_learns() {
    use notai_core::finance::{Money, Scope, Transaction, TxSource, month_report};
    let s = notai_store::Store::open_in_memory("sunucu").await.unwrap();
    let now = chrono::Utc::now();
    let d = |m: u32, day: u32| chrono::NaiveDate::from_ymd_opt(2026, m, day).unwrap();
    // Elle girilmiş ödeme (kayıttan): "Ayhan'a 1.850 ödendi" — iş.
    let manual = Transaction {
        id: notai_core::TransactionId::new(),
        date: d(10, 2),
        amount: Money(-185_000),
        description: "Ayhan Kimya'ya ödeme".into(),
        category: "tedarik".into(),
        scope: Scope::Is,
        source: TxSource::Manual,
        account: None,
        topic: None,
        entry: None,
        dedupe_key: None,
        matched_to: None,
    };
    s.add_transaction(&manual, now).await.unwrap();
    let text = "01.10.2026 MİGROS ŞİŞLİ -1.245,60\n03.10.2026 AYHAN KİMYA SAN. TİC. LTD -1.850,00\n\
                05.10.2026 MAAŞ ÖDEMESİ +85.000,00\n12.10.2026 NETFLIX.COM -229,99\n15.10.2026 TRENDYOL -640,00";
    let (_, r1) = s
        .import_statement("****1234", "ekim.pdf", text, now)
        .await
        .unwrap();
    assert_eq!(
        (r1.parsed, r1.imported, r1.skipped, r1.matched),
        (5, 5, 0, 1)
    );
    let (_, r2) = s
        .import_statement("****1234", "ekim-yine.pdf", text, now)
        .await
        .unwrap();
    assert_eq!(
        (r2.imported, r2.skipped),
        (0, 5),
        "aynı ekstre ikinci kez: mükerrer yok"
    );

    let all = s.transactions_between(d(10, 1), d(10, 31)).await.unwrap();
    assert_eq!(all.len(), 6);
    let mv: Vec<_> = all.iter().map(Transaction::movement).collect();
    let rep = month_report(&mv, 2026, 10);
    assert_eq!(
        rep.expense,
        Money(124_560 + 185_000 + 22_999 + 64_000),
        "elle giriş bir kez sayılır"
    );
    assert_eq!(rep.by_scope[0], (Scope::Is, Money(185_000)));

    // Öğrenme: Trendyol'u iş/tedarik yap → sonraki ekstrelerde de öyle.
    let trendyol = all
        .iter()
        .find(|t| t.description.contains("TRENDYOL"))
        .unwrap();
    s.classify_transaction(&trendyol.id.to_string(), "tedarik", Scope::Is, now)
        .await
        .unwrap();
    let (_, _) = s
        .import_statement(
            "****1234",
            "kasim.pdf",
            "03.11.2026 TRENDYOL ÖDEME -99,00",
            now,
        )
        .await
        .unwrap();
    let nov = s.transactions_between(d(11, 1), d(11, 30)).await.unwrap();
    assert_eq!(
        (nov[0].category.as_str(), nov[0].scope),
        ("tedarik", Scope::Is)
    );

    // Tanınmayan biçim: AI/ajan işi olarak bekler.
    let (sid, r) = s
        .import_statement("****1234", "garip.pdf", "bambaşka bir düzen 1.000", now)
        .await
        .unwrap();
    assert_eq!(r.parsed, 0);
    let pend = s.pending_statements().await.unwrap();
    assert_eq!(pend.len(), 1);
    let lines = notai_core::finance::parse_statement("20.11.2026 A101 -50,00");
    let r = s.answer_statement(&sid, &lines, now).await.unwrap();
    assert_eq!(r.imported, 1);
    assert!(s.pending_statements().await.unwrap().is_empty());
}

#[tokio::test]
async fn assets_value_history_and_price_messages() {
    use notai_core::asset::{AssetInfo, AssetKind};
    use notai_core::finance::Money;
    let s = store("sunucu").await;
    let d = |m: u32, day: u32| NaiveDate::from_ymd_opt(2026, m, day).unwrap();
    let root = s.assets_root(now()).await.unwrap();
    assert_eq!(root.name.as_str(), notai_store::ASSETS_ROOT);
    assert_eq!(s.assets_root(now()).await.unwrap().id, root.id, "tek kök");
    let mut n = NewTopic::new(
        TopicName::new("Altın birikimi").unwrap(),
        ParaKind::Resource,
    );
    n.parent = Some(root.id);
    let mut gold = s.create_topic(n, now()).await.unwrap();
    gold.asset = Some(
        AssetInfo::new(
            AssetKind::Altin,
            Some("12,5"),
            Some("gram"),
            Some("gram altın"),
            Money(4_000_000),
            d(10, 1),
        )
        .unwrap(),
    );
    s.update_topic(&gold, now()).await.unwrap();
    assert!(
        s.assets_total_on(d(9, 30)).await.unwrap().is_none(),
        "geçmiş yok"
    );
    // Sonradan eklenen daire: 30 gün önceki toplamda kendi ilk değeriyle sayılır.
    let mut flat = s
        .create_topic(
            NewTopic::new(TopicName::new("Kadıköy daire").unwrap(), ParaKind::Resource),
            now(),
        )
        .await
        .unwrap();
    flat.asset = Some(
        AssetInfo::new(
            AssetKind::Gayrimenkul,
            None,
            None,
            None,
            Money(500_000_000),
            d(11, 5),
        )
        .unwrap(),
    );
    s.update_topic(&flat, now()).await.unwrap();
    assert_eq!(
        s.assets_to_price(d(11, 6)).await.unwrap().len(),
        1,
        "yalnız sembollü"
    );
    // Küçük değişim: mesaj yok.
    assert!(
        s.price_asset(gold.id, Money(322_000), now(), d(11, 6))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        s.assets_to_price(d(11, 6)).await.unwrap().is_empty(),
        "bugün fiyatlandı"
    );
    // Büyük değişim: NotAI mesajı.
    let msg = s
        .price_asset(gold.id, Money(340_000), now(), d(11, 7))
        .await
        .unwrap()
        .expect("mesaj");
    assert!(msg.from_notai);
    assert!(msg.text.as_str().contains("arttı"), "{}", msg.text.as_str());
    assert_eq!(
        s.topic(gold.id).await.unwrap().asset.unwrap().value,
        Money(4_250_000)
    );
    // 1 Kas: altın 40.000, daire ilk değeriyle 5.000.000 → bugünkü fark yalnız altın artışı.
    let past = s.assets_total_on(d(11, 1)).await.unwrap().unwrap();
    assert_eq!(past, Money(504_000_000));
    let topics = s.asset_topics().await.unwrap();
    let total: i64 = topics
        .iter()
        .map(|t| t.asset.as_ref().unwrap().value.0)
        .sum();
    assert_eq!(total - past.0, 250_000);
    assert!(
        s.price_asset(root.id, Money(1), now(), d(11, 7))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn movements_from_text_location_questions_and_sync() {
    use notai_core::movement::{Check, Fix, Mode, MoveKind};
    let s = store("telefon").await;
    let at = |m: i64| now() + Duration::minutes(m);
    let cap = |text: &'static str, m: i64| {
        let s = &s;
        async move {
            s.capture(NewEntry {
                text: EntryText::new(text).unwrap(),
                source: Source::App,
                topic: None,
                captured_at: at(m),
                dedupe_key: None,
            })
            .await
            .unwrap()
            .entry()
            .clone()
        }
    };
    let question_of = |e: EntryId| {
        let s = &s;
        async move {
            s.suggestions_for(e)
                .await
                .unwrap()
                .into_iter()
                .find(|x| x.question_and_answer().is_some_and(|(_, a)| a.is_none()))
        }
    };
    // 1) Evden çıkış: Ev ve İş yerleri açılır, kayıt Hareketler/Ev'e yazılır, AI'a gitmez.
    let e1 = cap("Evden çıkıyorum", 0).await;
    let places = s.places().await.unwrap();
    let ev = places.iter().find(|p| p.name == "Ev").unwrap().topic;
    let is = places.iter().find(|p| p.name == "İş").unwrap().topic;
    assert_eq!(e1.topic(), Some(ev));
    assert!(
        s.next_ai_job(at(1)).await.unwrap().is_none(),
        "yalnız hareket"
    );
    let m1 = s.movement_for(e1.id).await.unwrap().unwrap();
    assert_eq!((m1.kind, m1.place), (MoveKind::Depart, Some(ev)));
    // Konum geldi: Ev'in yeri öğrenilir, sonra eksikler sorulur.
    let home = Fix::from_degrees(40.99, 29.03, 15.0).unwrap();
    let out = s
        .locate_movement(e1.id, Some(home), at(0))
        .await
        .unwrap()
        .unwrap();
    assert!(out.message.contains("öğrenildi"), "{}", out.message);
    assert!(!out.need_location);
    let q = question_of(e1.id).await.expect("soru");
    assert!(
        q.question_and_answer()
            .unwrap()
            .0
            .as_str()
            .starts_with("Nereye, neyle")
    );
    s.answer_question(
        q.id,
        notai_core::text::AnswerText::new("metroyla işe").unwrap(),
        at(2),
    )
    .await
    .unwrap();
    let m1 = s.movement_for(e1.id).await.unwrap().unwrap();
    assert_eq!((m1.mode, m1.to), (Some(Mode::Metro), Some(is)));
    assert!(
        s.next_ai_job(at(3)).await.unwrap().is_none(),
        "cevap AI'a gitmez"
    );

    // 2) İşe varış: konum kapalı ve İş'in yeri bilinmiyor → elzem.
    let e2 = cap("işe geldim", 50).await;
    let out = s
        .locate_movement(e2.id, None, at(50))
        .await
        .unwrap()
        .unwrap();
    assert!(out.need_location, "{}", out.message);
    assert!(
        question_of(e2.id).await.is_none(),
        "kalkışta ulaşım vardı, yer belli"
    );

    // 3) Ertesi sefer evden çıkışta konum teyit edilir.
    let e3 = cap("evden çıktım", 24 * 60).await;
    let near = Fix::from_degrees(40.9905, 29.03, 20.0).unwrap();
    s.locate_movement(e3.id, Some(near), at(24 * 60))
        .await
        .unwrap();
    assert!(matches!(
        s.movement_for(e3.id).await.unwrap().unwrap().check,
        Check::Confirmed { .. }
    ));

    // 4) Yeni yer, konum hiç gelmez (WhatsApp): bekleme sonrası amaç sorulur.
    let e4 = cap("Ayhan Kimya'ya geldim taksiyle", 24 * 60 + 40).await;
    let sup = s
        .places()
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.name == "Ayhan Kimya")
        .unwrap();
    assert_eq!(e4.topic(), Some(sup.topic));
    assert!(
        s.settle_movements(at(24 * 60 + 41))
            .await
            .unwrap()
            .is_empty(),
        "henüz erken"
    );
    assert_eq!(s.settle_movements(at(24 * 60 + 45)).await.unwrap().len(), 1);
    let q = question_of(e4.id).await.unwrap();
    assert_eq!(
        q.question_and_answer().unwrap().0.as_str(),
        "Ayhan Kimya iş için mi, özel mi?"
    );
    s.answer_question(
        q.id,
        notai_core::text::AnswerText::new("iş").unwrap(),
        at(24 * 60 + 46),
    )
    .await
    .unwrap();
    let sup = s
        .places()
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.name == "Ayhan Kimya")
        .unwrap();
    assert_eq!(
        sup.info.scope,
        Some(notai_core::finance::Scope::Is),
        "amaç öğrenildi"
    );

    // Hareket olmayan kayıt etkilenmez.
    let plain = cap("Kostik siparişini ver", 24 * 60 + 50).await;
    assert!(s.movement_for(plain.id).await.unwrap().is_none());
    assert_eq!(
        s.movements_between(at(-1), at(3 * 24 * 60))
            .await
            .unwrap()
            .len(),
        4
    );

    // Senkron: yerler ve hareketler sunucuya gider; yalnız hareket kayıtları AI'a girmez.
    let server = store("sunucu").await;
    for c in s.own_changes_since(0, 1000).await.unwrap() {
        server.apply_remote(&c, true).await.unwrap();
    }
    assert_eq!(server.places().await.unwrap().len(), 3);
    assert_eq!(
        server
            .movements_between(at(-1), at(3 * 24 * 60))
            .await
            .unwrap()
            .len(),
        4
    );
    let job = server.next_ai_job(at(3 * 24 * 60)).await.unwrap();
    assert_eq!(
        job.map(|e| e.id),
        Some(plain.id),
        "yalnız hareket olmayan kayıt"
    );
}
