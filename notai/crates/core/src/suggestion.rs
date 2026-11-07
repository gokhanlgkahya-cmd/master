//! AI önerileri ve kabul edildiklerinde doğurdukları etkiler.
//!
//! AI hiçbir şeyi doğrudan değiştirmez: önerir. Kullanıcı (ya da v0.4'teki otonom mod)
//! kabul edince `accept` saf bir `Effect` üretir; depo bu etkiyi tek transaction'da uygular.

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::entry::Entry;
use crate::error::CoreError;
use crate::ids::{EntryId, SuggestionId, TopicId};
use crate::task::{Due, NewTask, Priority, Reminder};
use crate::text::{AnswerText, Label, QuestionText, TaskTitle, TopicName};
use crate::topic::ParaKind;
use crate::tr::{month_short, weekday_name};

/// 0–1 arası güven skoru. NaN ya da aralık dışı değer temsil edilemez.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct Confidence(f64);

impl Confidence {
    pub fn new(value: f64) -> Result<Self, CoreError> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(CoreError::ConfidenceOutOfRange(value.to_string()))
        }
    }

    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Confidence {
    type Error = CoreError;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Confidence> for f64 {
    fn from(value: Confidence) -> Self {
        value.0
    }
}

/// Önerilen konu: var olan ya da yeni açılacak.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "target", rename_all = "snake_case")]
pub enum TopicTarget {
    Existing {
        id: TopicId,
        name: TopicName,
    },
    New {
        name: TopicName,
        kind: ParaKind,
        /// Yeni konu bu konunun altına açılır (hiyerarşi).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent: Option<TopicId>,
    },
}

impl TopicTarget {
    #[must_use]
    pub const fn name(&self) -> &TopicName {
        match self {
            Self::Existing { name, .. } | Self::New { name, .. } => name,
        }
    }
}

/// Öneri türü ve içeriği.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SuggestionKind {
    /// Kaydı bir konuya bağla.
    Topic { target: TopicTarget },
    /// Kayıttan görev çıkar.
    Task {
        title: TaskTitle,
        due: Option<Due>,
        #[serde(default)]
        priority: Priority,
    },
    /// Etiket ekle (#sabun).
    Tag { name: Label },
    /// Kişi bağla (Ayhan Kimya).
    Person { name: Label },
    /// Belirli bir anda hatırlat.
    Reminder { at: DateTime<Utc> },
    /// Kayıtta para hareketi var ("Ayhan'a 1.850 ₺ ödendi"): kabul edilince hesap hareketi
    /// olur (elle giriş; ekstrede karşılığı gelince eşleştirilir, raporda bir kez sayılır).
    Expense {
        /// Kuruş; eksi = gider, artı = gelir.
        amount: crate::finance::Money,
        category: String,
        scope: crate::finance::Scope,
        date: NaiveDate,
    },
    /// Kayıt net değil: AI kullanıcıya kısa bir soru soruyor. Cevaplanınca kayıt cevapla
    /// birlikte yeniden sınıflandırılır; soru-cevaplar sonraki sınıflandırmalara örnek olur.
    Question {
        text: QuestionText,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        answer: Option<AnswerText>,
    },
}

impl SuggestionKind {
    /// Veritabanı/istatistik için kısa tür adı.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Topic { .. } => "topic",
            Self::Task { .. } => "task",
            Self::Tag { .. } => "tag",
            Self::Person { .. } => "person",
            Self::Reminder { .. } => "reminder",
            Self::Expense { .. } => "expense",
            Self::Question { .. } => "question",
        }
    }

    /// Çip metni (ör. "Görev · Perşembe 12 Kas", "→ GemOlive").
    #[must_use]
    pub fn chip_text(&self, tz: Tz) -> String {
        match self {
            Self::Topic { target } => format!("→ {}", target.name()),
            Self::Task { due: None, .. } => "Görev".to_owned(),
            Self::Task { due: Some(due), .. } => {
                let mut s = format!("Görev · {}", short_day(due.date));
                if let Some(t) = due.time {
                    s.push_str(&format!(" {}", t.format("%H:%M")));
                }
                s
            }
            Self::Tag { name } => format!("#{name}"),
            Self::Person { name } => format!("Kişi: {name}"),
            Self::Reminder { at } => {
                let local = at.with_timezone(&tz);
                format!(
                    "Hatırlatma · {} {}",
                    short_day(local.date_naive()),
                    local.format("%H:%M")
                )
            }
            Self::Expense { amount, scope, .. } => format!(
                "{} · {} · {}",
                if amount.0 < 0 { "Harcama" } else { "Gelir" },
                amount.format_abs(),
                scope.label()
            ),
            Self::Question { text, .. } => text.as_str().to_owned(),
        }
    }
}

/// "Perşembe 12 Kas".
fn short_day(date: NaiveDate) -> String {
    format!(
        "{} {} {}",
        weekday_name(date.weekday()),
        date.day(),
        month_short(date.month())
    )
}

/// Kullanıcının kararı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum Decision {
    Pending,
    Accepted { at: DateTime<Utc> },
    Dismissed { at: DateTime<Utc> },
}

impl Decision {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted { .. } => "accepted",
            Self::Dismissed { .. } => "dismissed",
        }
    }

    #[must_use]
    pub const fn decided_at(self) -> Option<DateTime<Utc>> {
        match self {
            Self::Pending => None,
            Self::Accepted { at } | Self::Dismissed { at } => Some(at),
        }
    }

    pub fn from_parts(name: &str, at: Option<DateTime<Utc>>) -> Result<Self, CoreError> {
        match (name, at) {
            ("pending", None) => Ok(Self::Pending),
            ("accepted", Some(at)) => Ok(Self::Accepted { at }),
            ("dismissed", Some(at)) => Ok(Self::Dismissed { at }),
            (other, _) => Err(CoreError::UnknownVariant {
                field: "öneri kararı",
                value: other.to_owned(),
            }),
        }
    }
}

/// Bir AI önerisi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: SuggestionId,
    pub entry: EntryId,
    pub kind: SuggestionKind,
    pub confidence: Confidence,
    decision: Decision,
}

/// Kabul edilen önerinin depoya uygulanacak etkisi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "effect", rename_all = "snake_case")]
pub enum Effect {
    AssignTopic {
        entry: EntryId,
        topic: TopicId,
    },
    CreateTopicAndAssign {
        entry: EntryId,
        name: TopicName,
        kind: ParaKind,
        parent: Option<TopicId>,
    },
    CreateTask {
        task: NewTask,
    },
    AddTag {
        entry: EntryId,
        name: Label,
    },
    AddPerson {
        entry: EntryId,
        name: Label,
    },
    AddTransaction {
        tx: crate::finance::Transaction,
    },
}

impl Suggestion {
    #[must_use]
    pub fn new(entry: EntryId, kind: SuggestionKind, confidence: Confidence) -> Self {
        Self {
            id: SuggestionId::new(),
            entry,
            kind,
            confidence,
            decision: Decision::Pending,
        }
    }

    #[must_use]
    pub const fn from_parts(
        id: SuggestionId,
        entry: EntryId,
        kind: SuggestionKind,
        confidence: Confidence,
        decision: Decision,
    ) -> Self {
        Self {
            id,
            entry,
            kind,
            confidence,
            decision,
        }
    }

    #[must_use]
    pub const fn decision(&self) -> Decision {
        self.decision
    }

    #[must_use]
    pub const fn is_pending(&self) -> bool {
        matches!(self.decision, Decision::Pending)
    }

    /// Öneriyi kabul eder ve uygulanacak etkiyi döndürür.
    pub fn accept(
        &mut self,
        entry: &Entry,
        now: DateTime<Utc>,
        tz: Tz,
    ) -> Result<Effect, CoreError> {
        if !self.is_pending() {
            return Err(CoreError::AlreadyDecided);
        }
        if entry.is_trashed() {
            return Err(CoreError::InvalidTransition {
                from: "trashed",
                to: "accepted",
            });
        }
        let effect = match &self.kind {
            SuggestionKind::Question { .. } => {
                return Err(CoreError::InvalidTransition {
                    from: "question",
                    to: "accepted",
                });
            }
            SuggestionKind::Topic {
                target: TopicTarget::Existing { id, .. },
            } => Effect::AssignTopic {
                entry: entry.id,
                topic: *id,
            },
            SuggestionKind::Topic {
                target: TopicTarget::New { name, kind, parent },
            } => Effect::CreateTopicAndAssign {
                entry: entry.id,
                name: name.clone(),
                kind: *kind,
                parent: *parent,
            },
            SuggestionKind::Task {
                title,
                due,
                priority,
            } => Effect::CreateTask {
                task: NewTask {
                    entry: Some(entry.id),
                    title: title.clone(),
                    topic: entry.topic(),
                    due: *due,
                    priority: *priority,
                    recurrence: None,
                    // Tarihli görev zamanında hatırlatılsın (Gökhan, 2026-11-07).
                    reminder: due.and_then(|d| crate::task::default_reminder(&d, tz, now)),
                },
            },
            SuggestionKind::Tag { name } => Effect::AddTag {
                entry: entry.id,
                name: name.clone(),
            },
            SuggestionKind::Person { name } => Effect::AddPerson {
                entry: entry.id,
                name: name.clone(),
            },
            SuggestionKind::Expense {
                amount,
                category,
                scope,
                date,
            } => Effect::AddTransaction {
                tx: crate::finance::Transaction {
                    id: crate::TransactionId::new(),
                    date: *date,
                    amount: *amount,
                    description: entry.text.preview(120),
                    category: category.clone(),
                    scope: *scope,
                    source: crate::finance::TxSource::Manual,
                    account: None,
                    topic: entry.topic(),
                    entry: Some(entry.id),
                    dedupe_key: None,
                    matched_to: None,
                },
            },
            SuggestionKind::Reminder { at } => {
                let local = at.with_timezone(&tz);
                Effect::CreateTask {
                    task: NewTask {
                        entry: Some(entry.id),
                        title: TaskTitle::new(entry.text.preview(TaskTitle::MAX_CHARS))?,
                        topic: entry.topic(),
                        due: Some(Due::at(local.date_naive(), local.time())),
                        priority: Priority::Normal,
                        recurrence: None,
                        reminder: Some(Reminder::At { at: *at }),
                    },
                }
            }
        };
        self.decision = Decision::Accepted { at: now };
        Ok(effect)
    }

    /// Kabul edilmiş öneriyi geri alır (ör. otonom modun uyguladığını kullanıcı beğenmedi).
    /// Karar "yoksayıldı" olur: senkronda "karar asla bekliyor'a dönmez" kuralıyla tutarlı.
    /// Etkinin tersini uygulamak deponun işidir.
    pub fn undo_accept(&mut self, now: DateTime<Utc>) -> Result<(), CoreError> {
        match self.decision {
            Decision::Accepted { .. } => {
                self.decision = Decision::Dismissed { at: now };
                Ok(())
            }
            _ => Err(CoreError::InvalidTransition {
                from: self.decision.name(),
                to: "undone",
            }),
        }
    }

    /// AI sorusunu cevaplar: cevap kaydedilir, öneri "kabul edildi" sayılır (etkisi yoktur;
    /// depo kaydı cevapla birlikte yeniden AI kuyruğuna koyar).
    pub fn answer(&mut self, text: AnswerText, now: DateTime<Utc>) -> Result<(), CoreError> {
        if !self.is_pending() {
            return Err(CoreError::AlreadyDecided);
        }
        match &mut self.kind {
            SuggestionKind::Question { answer, .. } => {
                *answer = Some(text);
                self.decision = Decision::Accepted { at: now };
                Ok(())
            }
            other => Err(CoreError::InvalidTransition {
                from: other.name(),
                to: "answered",
            }),
        }
    }

    /// Cevaplanmış soru ise (soru, cevap).
    #[must_use]
    pub fn question_and_answer(&self) -> Option<(&QuestionText, Option<&AnswerText>)> {
        match &self.kind {
            SuggestionKind::Question { text, answer } => Some((text, answer.as_ref())),
            _ => None,
        }
    }

    /// Öneriyi yoksayar (yalnız bu öneri; toplu ret yoktur).
    pub fn dismiss(&mut self, now: DateTime<Utc>) -> Result<(), CoreError> {
        if !self.is_pending() {
            return Err(CoreError::AlreadyDecided);
        }
        self.decision = Decision::Dismissed { at: now };
        Ok(())
    }
}

/// Bir kaydın tüm önerileri karara bağlandı mı? (Öneri yoksa `false`: AI henüz işlemedi.)
#[must_use]
pub fn all_decided(suggestions: &[Suggestion]) -> bool {
    !suggestions.is_empty() && suggestions.iter().all(|s| !s.is_pending())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::Source;
    use crate::text::EntryText;
    use chrono::{NaiveTime, TimeZone};

    const TZ: Tz = chrono_tz::Europe::Istanbul;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 6, 9, 30, 0).unwrap()
    }

    fn entry() -> Entry {
        Entry::capture(
            EntryText::new("Kostik tedarikçisini ara – perşembe. 25 kg'lık torba fiyatını sor.")
                .unwrap(),
            Source::App,
            now(),
            None,
        )
    }

    fn conf() -> Confidence {
        Confidence::new(0.9).unwrap()
    }

    #[test]
    fn question_is_answered_not_accepted() {
        let e = entry();
        let mut q = Suggestion::new(
            e.id,
            SuggestionKind::Question {
                text: QuestionText::new("Hangi tedarikçi için?").unwrap(),
                answer: None,
            },
            conf(),
        );
        assert_eq!(q.kind.chip_text(TZ), "Hangi tedarikçi için?");
        assert!(
            q.accept(&e, now(), TZ).is_err(),
            "soru kabul edilemez, cevaplanır"
        );
        assert!(q.is_pending());
        q.answer(AnswerText::new("Ayhan Kimya").unwrap(), now())
            .unwrap();
        assert!(!q.is_pending());
        let (text, answer) = q.question_and_answer().unwrap();
        assert_eq!(
            (text.as_str(), answer.map(AnswerText::as_str)),
            ("Hangi tedarikçi için?", Some("Ayhan Kimya"))
        );
        assert_eq!(
            q.answer(AnswerText::new("x").unwrap(), now()).unwrap_err(),
            CoreError::AlreadyDecided
        );
        // JSON: cevapsız soru eski biçimle uyumlu; cevaplı soru cevabı taşır.
        let json = serde_json::to_string(&q.kind).unwrap();
        assert!(json.contains("\"answer\":\"Ayhan Kimya\""));
        let mut tag = Suggestion::new(
            e.id,
            SuggestionKind::Tag {
                name: Label::new("x").unwrap(),
            },
            conf(),
        );
        assert!(tag.answer(AnswerText::new("y").unwrap(), now()).is_err());
    }

    #[test]
    fn accepted_dated_task_gets_default_reminder() {
        let e = entry();
        let task = |due: Option<Due>| {
            let mut s = Suggestion::new(
                e.id,
                SuggestionKind::Task {
                    title: TaskTitle::new("Kostik tedarikçisini ara").unwrap(),
                    due,
                    priority: Priority::Normal,
                },
                conf(),
            );
            match s.accept(&e, now(), TZ).unwrap() {
                Effect::CreateTask { task } => task.reminder,
                other => panic!("{other:?}"),
            }
        };
        let thu = NaiveDate::from_ymd_opt(2026, 11, 12).unwrap();
        assert_eq!(
            task(Some(Due::at(
                thu,
                NaiveTime::from_hms_opt(14, 0, 0).unwrap()
            ))),
            Some(Reminder::Before { minutes: 15 }),
            "saatli: 15 dk önce"
        );
        assert_eq!(
            task(Some(Due::on(thu))),
            Some(Reminder::Before { minutes: 0 }),
            "saatsiz: o gün varsayılan saatte (09:00)"
        );
        assert_eq!(task(None), None, "tarihsiz: hatırlatma yok");
        // Bugüne saatsiz ve 09:00 geçmişse: hemen çalan anlamsız hatırlatma kurulmaz.
        let today = NaiveDate::from_ymd_opt(2026, 11, 6).unwrap();
        assert_eq!(task(Some(Due::on(today))), None);
    }

    #[test]
    fn confidence_bounds() {
        assert!(Confidence::new(-0.1).is_err());
        assert!(Confidence::new(1.01).is_err());
        assert!(Confidence::new(f64::NAN).is_err());
        assert!(serde_json::from_str::<Confidence>("1.5").is_err());
        assert_eq!(Confidence::new(1.0).unwrap().get(), 1.0);
    }

    #[test]
    fn accepting_existing_topic_assigns() {
        let e = entry();
        let topic = TopicId::new();
        let mut s = Suggestion::new(
            e.id,
            SuggestionKind::Topic {
                target: TopicTarget::Existing {
                    id: topic,
                    name: TopicName::new("Kostik tedarik").unwrap(),
                },
            },
            conf(),
        );
        assert_eq!(
            s.accept(&e, now(), TZ).unwrap(),
            Effect::AssignTopic { entry: e.id, topic }
        );
        assert_eq!(s.decision(), Decision::Accepted { at: now() });
        assert_eq!(
            s.accept(&e, now(), TZ).unwrap_err(),
            CoreError::AlreadyDecided
        );
        assert_eq!(s.dismiss(now()).unwrap_err(), CoreError::AlreadyDecided);
    }

    #[test]
    fn accepting_task_carries_entry_and_topic() {
        let mut e = entry();
        let topic = TopicId::new();
        e.assign_topic(Some(topic)).unwrap();
        let due = Due::at(
            NaiveDate::from_ymd_opt(2026, 11, 12).unwrap(),
            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
        );
        let mut s = Suggestion::new(
            e.id,
            SuggestionKind::Task {
                title: TaskTitle::new("Kostik firmasını ara").unwrap(),
                due: Some(due),
                priority: Priority::High,
            },
            conf(),
        );
        let Effect::CreateTask { task } = s.accept(&e, now(), TZ).unwrap() else {
            panic!("görev etkisi bekleniyordu");
        };
        assert_eq!(task.entry, Some(e.id));
        assert_eq!(task.topic, Some(topic));
        assert_eq!(task.due, Some(due));
        assert_eq!(task.priority, Priority::High);
    }

    #[test]
    fn reminder_becomes_task_with_reminder_at_local_time() {
        let e = entry();
        let at = Utc.with_ymd_and_hms(2026, 11, 14, 7, 0, 0).unwrap();
        let mut s = Suggestion::new(e.id, SuggestionKind::Reminder { at }, conf());
        let Effect::CreateTask { task } = s.accept(&e, now(), TZ).unwrap() else {
            panic!("görev etkisi bekleniyordu");
        };
        assert_eq!(task.reminder, Some(Reminder::At { at }));
        let due = task.due.unwrap();
        assert_eq!(due.date, NaiveDate::from_ymd_opt(2026, 11, 14).unwrap());
        assert_eq!(due.time, NaiveTime::from_hms_opt(10, 0, 0));
    }

    #[test]
    fn trashed_entry_suggestions_cannot_be_accepted() {
        let mut e = entry();
        e.trash(now()).unwrap();
        let mut s = Suggestion::new(
            e.id,
            SuggestionKind::Tag {
                name: Label::new("sabun").unwrap(),
            },
            conf(),
        );
        assert!(s.accept(&e, now(), TZ).is_err());
        assert!(s.is_pending());
    }

    #[test]
    fn all_decided_rule() {
        let e = entry();
        assert!(!all_decided(&[]));
        let mut a = Suggestion::new(
            e.id,
            SuggestionKind::Tag {
                name: Label::new("tedarik").unwrap(),
            },
            conf(),
        );
        let mut b = Suggestion::new(
            e.id,
            SuggestionKind::Person {
                name: Label::new("Ayhan Kimya").unwrap(),
            },
            conf(),
        );
        a.accept(&e, now(), TZ).unwrap();
        assert!(!all_decided(&[a.clone(), b.clone()]));
        b.dismiss(now()).unwrap();
        assert!(all_decided(&[a, b]));
    }

    #[test]
    fn chip_texts() {
        let due = Due::on(NaiveDate::from_ymd_opt(2026, 11, 12).unwrap());
        let task = SuggestionKind::Task {
            title: TaskTitle::new("Kostik ara").unwrap(),
            due: Some(due),
            priority: Priority::Normal,
        };
        assert_eq!(task.chip_text(TZ), "Görev · Perşembe 12 Kas");
        let topic = SuggestionKind::Topic {
            target: TopicTarget::New {
                name: TopicName::new("GemOlive").unwrap(),
                kind: ParaKind::Area,
                parent: None,
            },
        };
        assert_eq!(topic.chip_text(TZ), "→ GemOlive");
        let rem = SuggestionKind::Reminder {
            at: Utc.with_ymd_and_hms(2026, 11, 14, 7, 0, 0).unwrap(),
        };
        assert_eq!(rem.chip_text(TZ), "Hatırlatma · Cumartesi 14 Kas 10:00");
        let person = SuggestionKind::Person {
            name: Label::new("Ayhan Kimya").unwrap(),
        };
        assert_eq!(person.chip_text(TZ), "Kişi: Ayhan Kimya");
    }

    #[test]
    fn undo_only_after_accept() {
        let e = entry();
        let mut s = Suggestion::new(
            e.id,
            SuggestionKind::Tag {
                name: Label::new("sabun").unwrap(),
            },
            conf(),
        );
        assert!(s.undo_accept(now()).is_err());
        s.accept(&e, now(), TZ).unwrap();
        s.undo_accept(now()).unwrap();
        assert_eq!(s.decision(), Decision::Dismissed { at: now() });
        assert!(s.undo_accept(now()).is_err());
    }

    #[test]
    fn decision_parts_round_trip() {
        for d in [
            Decision::Pending,
            Decision::Accepted { at: now() },
            Decision::Dismissed { at: now() },
        ] {
            assert_eq!(Decision::from_parts(d.name(), d.decided_at()).unwrap(), d);
        }
        assert!(Decision::from_parts("accepted", None).is_err());
    }

    #[test]
    fn kind_serde_round_trip() {
        let k = SuggestionKind::Topic {
            target: TopicTarget::New {
                name: TopicName::new("Ev sunucusu (S24)").unwrap(),
                kind: ParaKind::Project { due: None },
                parent: None,
            },
        };
        let json = serde_json::to_string(&k).unwrap();
        let back: SuggestionKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, k);
    }
}
