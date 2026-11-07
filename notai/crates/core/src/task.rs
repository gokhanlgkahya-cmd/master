//! Görevler: tarih, öncelik, tekrar, hatırlatma.

use chrono::{
    DateTime, Datelike, Days, Duration, Months, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc,
    Weekday,
};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::ids::{EntryId, TaskId, TopicId};
use crate::text::TaskTitle;

/// Saati verilmemiş görevlerin hatırlatması yerel saatle bu saatte çalar.
pub const DEFAULT_REMINDER_HOUR: u32 = 9;

/// Görev önceliği (Düşük < Normal < Yüksek).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
}

impl Priority {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Low => "Düşük",
            Self::Normal => "Orta",
            Self::High => "Yüksek",
        }
    }

    pub fn parse(s: &str) -> Result<Self, CoreError> {
        match s {
            "low" => Ok(Self::Low),
            "normal" => Ok(Self::Normal),
            "high" => Ok(Self::High),
            other => Err(CoreError::UnknownVariant {
                field: "öncelik",
                value: other.to_owned(),
            }),
        }
    }
}

/// Görevin tarihi; saat isteğe bağlı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Due {
    pub date: NaiveDate,
    pub time: Option<NaiveTime>,
}

impl Due {
    #[must_use]
    pub const fn on(date: NaiveDate) -> Self {
        Self { date, time: None }
    }

    #[must_use]
    pub const fn at(date: NaiveDate, time: NaiveTime) -> Self {
        Self {
            date,
            time: Some(time),
        }
    }

    /// Yerel saatle bu tarihin anı (saat yoksa varsayılan hatırlatma saati).
    #[must_use]
    pub fn instant(&self, tz: Tz) -> DateTime<Utc> {
        let time = self
            .time
            .or_else(|| NaiveTime::from_hms_opt(DEFAULT_REMINDER_HOUR, 0, 0))
            .unwrap_or(NaiveTime::MIN);
        local_to_utc(tz, self.date.and_time(time))
    }
}

/// AI'ın önerdiği tarihli görev için varsayılan hatırlatma (Gökhan, 2026-11-07): saatliyse
/// 15 dk önce, saatsizse o gün varsayılan saatte (09:00). Çalacağı an geçmişse kurulmaz
/// (hemen çalan anlamsız bildirim olmasın).
#[must_use]
pub fn default_reminder(due: &Due, tz: Tz, now: DateTime<Utc>) -> Option<Reminder> {
    let reminder = if due.time.is_some() {
        Reminder::Before { minutes: 15 }
    } else {
        Reminder::Before { minutes: 0 }
    };
    let fire = reminder.fire_at(Some(due), tz).ok()?;
    (fire > now).then_some(reminder)
}

/// Yerel tarih-saati UTC'ye çevirir. Yaz saati geçişindeki "olmayan" saatlerde
/// bir sonraki geçerli ana kayar; asla panik yapmaz.
#[must_use]
pub fn local_to_utc(tz: Tz, naive: NaiveDateTime) -> DateTime<Utc> {
    match tz.from_local_datetime(&naive).earliest() {
        Some(t) => t.with_timezone(&Utc),
        None => {
            let shifted = naive + Duration::hours(1);
            tz.from_local_datetime(&shifted)
                .earliest()
                .map_or_else(|| Utc.from_utc_datetime(&naive), |t| t.with_timezone(&Utc))
        }
    }
}

/// Ayın günü (1–28): her ayda var olan günlerle sınırlı, "31 Şubat" temsil edilemez.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct MonthDay(u8);

impl MonthDay {
    pub fn new(day: u8) -> Result<Self, CoreError> {
        if (1..=28).contains(&day) {
            Ok(Self(day))
        } else {
            Err(CoreError::InvalidMonthDay(day))
        }
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for MonthDay {
    type Error = CoreError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<MonthDay> for u8 {
    fn from(value: MonthDay) -> Self {
        value.0
    }
}

/// Tekrar kuralı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "every", rename_all = "snake_case")]
pub enum Recurrence {
    Day,
    /// Pazartesi–Cuma.
    Weekday,
    Week {
        on: Weekday,
    },
    Month {
        day: MonthDay,
    },
}

impl Recurrence {
    /// `after` tarihinden SONRAKİ ilk tekrar tarihi.
    #[must_use]
    pub fn next_after(&self, after: NaiveDate) -> NaiveDate {
        match *self {
            Self::Day => after + Days::new(1),
            Self::Weekday => {
                let mut d = after + Days::new(1);
                while matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
                    d = d + Days::new(1);
                }
                d
            }
            Self::Week { on } => {
                let mut d = after + Days::new(1);
                while d.weekday() != on {
                    d = d + Days::new(1);
                }
                d
            }
            Self::Month { day } => {
                let day = u32::from(day.get());
                let this_month = after.with_day(day);
                match this_month {
                    Some(d) if d > after => d,
                    _ => after
                        .with_day(1)
                        .and_then(|first| first.checked_add_months(Months::new(1)))
                        .and_then(|next| next.with_day(day))
                        .unwrap_or(after + Days::new(28)),
                }
            }
        }
    }
}

/// Hatırlatma.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Reminder {
    /// Görev tarihinden `minutes` dakika önce.
    Before { minutes: u32 },
    /// Belirli bir anda.
    At { at: DateTime<Utc> },
}

impl Reminder {
    /// Hatırlatmanın çalacağı an.
    pub fn fire_at(&self, due: Option<&Due>, tz: Tz) -> Result<DateTime<Utc>, CoreError> {
        match *self {
            Self::At { at } => Ok(at),
            Self::Before { minutes } => due
                .map(|d| d.instant(tz) - Duration::minutes(i64::from(minutes)))
                .ok_or(CoreError::ReminderNeedsDue),
        }
    }
}

/// Görevin yaşam döngüsü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TaskState {
    Open,
    Done { at: DateTime<Utc> },
    Cancelled { at: DateTime<Utc> },
}

impl TaskState {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Done { .. } => "done",
            Self::Cancelled { .. } => "cancelled",
        }
    }

    #[must_use]
    pub const fn closed_at(self) -> Option<DateTime<Utc>> {
        match self {
            Self::Open => None,
            Self::Done { at } | Self::Cancelled { at } => Some(at),
        }
    }

    pub fn from_parts(name: &str, closed_at: Option<DateTime<Utc>>) -> Result<Self, CoreError> {
        match (name, closed_at) {
            ("open", None) => Ok(Self::Open),
            ("done", Some(at)) => Ok(Self::Done { at }),
            ("cancelled", Some(at)) => Ok(Self::Cancelled { at }),
            (other, _) => Err(CoreError::UnknownVariant {
                field: "görev durumu",
                value: other.to_owned(),
            }),
        }
    }
}

/// Bir görev.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    /// Görevin türediği kayıt (varsa).
    pub entry: Option<EntryId>,
    pub title: TaskTitle,
    pub topic: Option<TopicId>,
    pub due: Option<Due>,
    pub priority: Priority,
    pub recurrence: Option<Recurrence>,
    pub reminder: Option<Reminder>,
    state: TaskState,
}

/// Yeni görev için girdiler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewTask {
    pub entry: Option<EntryId>,
    pub title: TaskTitle,
    pub topic: Option<TopicId>,
    pub due: Option<Due>,
    #[serde(default)]
    pub priority: Priority,
    pub recurrence: Option<Recurrence>,
    pub reminder: Option<Reminder>,
}

impl NewTask {
    #[must_use]
    pub const fn titled(title: TaskTitle) -> Self {
        Self {
            entry: None,
            title,
            topic: None,
            due: None,
            priority: Priority::Normal,
            recurrence: None,
            reminder: None,
        }
    }
}

impl Task {
    /// Yeni açık görev oluşturur. Tarihsiz göreve "önce" hatırlatması ya da tekrar verilemez.
    pub fn create(new: NewTask) -> Result<Self, CoreError> {
        Self::validate(
            new.due.as_ref(),
            new.reminder.as_ref(),
            new.recurrence.as_ref(),
        )?;
        Ok(Self {
            id: TaskId::new(),
            entry: new.entry,
            title: new.title,
            topic: new.topic,
            due: new.due,
            priority: new.priority,
            recurrence: new.recurrence,
            reminder: new.reminder,
            state: TaskState::Open,
        })
    }

    /// Depodan okunan parçalardan yeniden kurar (doğrulamayı yine yapar).
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: TaskId,
        entry: Option<EntryId>,
        title: TaskTitle,
        topic: Option<TopicId>,
        due: Option<Due>,
        priority: Priority,
        recurrence: Option<Recurrence>,
        reminder: Option<Reminder>,
        state: TaskState,
    ) -> Result<Self, CoreError> {
        Self::validate(due.as_ref(), reminder.as_ref(), recurrence.as_ref())?;
        Ok(Self {
            id,
            entry,
            title,
            topic,
            due,
            priority,
            recurrence,
            reminder,
            state,
        })
    }

    fn validate(
        due: Option<&Due>,
        reminder: Option<&Reminder>,
        recurrence: Option<&Recurrence>,
    ) -> Result<(), CoreError> {
        if due.is_none()
            && (matches!(reminder, Some(Reminder::Before { .. })) || recurrence.is_some())
        {
            return Err(CoreError::ReminderNeedsDue);
        }
        Ok(())
    }

    #[must_use]
    pub const fn state(&self) -> TaskState {
        self.state
    }

    #[must_use]
    pub const fn is_open(&self) -> bool {
        matches!(self.state, TaskState::Open)
    }

    /// Tarihi bugünden önceyse ve hâlâ açıksa gecikmiştir.
    #[must_use]
    pub fn is_overdue(&self, today: NaiveDate) -> bool {
        self.is_open() && self.due.is_some_and(|d| d.date < today)
    }

    /// Kaç gün gecikti (gecikmediyse 0).
    #[must_use]
    pub fn days_overdue(&self, today: NaiveDate) -> i64 {
        match self.due {
            Some(d) if self.is_open() && d.date < today => (today - d.date).num_days(),
            _ => 0,
        }
    }

    #[must_use]
    pub fn is_due_on(&self, day: NaiveDate) -> bool {
        self.is_open() && self.due.is_some_and(|d| d.date == day)
    }

    /// Görevi tamamlar. Tekrarlayan görevse bir sonraki örneği döndürür.
    pub fn complete(&mut self, now: DateTime<Utc>) -> Result<Option<Self>, CoreError> {
        if !self.is_open() {
            return Err(CoreError::TaskClosed);
        }
        self.state = TaskState::Done { at: now };
        let next = match (self.recurrence, self.due) {
            (Some(rule), Some(due)) => Some(Self {
                id: TaskId::new(),
                due: Some(Due {
                    date: rule.next_after(due.date),
                    time: due.time,
                }),
                state: TaskState::Open,
                ..self.clone()
            }),
            _ => None,
        };
        Ok(next)
    }

    pub fn cancel(&mut self, now: DateTime<Utc>) -> Result<(), CoreError> {
        if !self.is_open() {
            return Err(CoreError::TaskClosed);
        }
        self.state = TaskState::Cancelled { at: now };
        Ok(())
    }

    /// Kapalı görevi yeniden açar ("geri al").
    pub fn reopen(&mut self) {
        self.state = TaskState::Open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn title(s: &str) -> TaskTitle {
        TaskTitle::new(s).unwrap()
    }

    #[test]
    fn priority_orders_and_parses() {
        assert!(Priority::High > Priority::Normal && Priority::Normal > Priority::Low);
        for p in [Priority::Low, Priority::Normal, Priority::High] {
            assert_eq!(Priority::parse(p.name()).unwrap(), p);
        }
        assert_eq!(Priority::High.label(), "Yüksek");
    }

    #[test]
    fn recurrence_next_dates() {
        let fri = d(2026, 11, 6);
        assert_eq!(Recurrence::Day.next_after(fri), d(2026, 11, 7));
        assert_eq!(Recurrence::Weekday.next_after(fri), d(2026, 11, 9));
        assert_eq!(
            Recurrence::Week { on: Weekday::Sun }.next_after(fri),
            d(2026, 11, 8)
        );
        assert_eq!(
            Recurrence::Week { on: Weekday::Fri }.next_after(fri),
            d(2026, 11, 13)
        );
        let m = |n| Recurrence::Month {
            day: MonthDay::new(n).unwrap(),
        };
        assert_eq!(m(10).next_after(fri), d(2026, 11, 10));
        assert_eq!(m(6).next_after(fri), d(2026, 12, 6));
        assert_eq!(m(1).next_after(d(2026, 12, 15)), d(2027, 1, 1));
    }

    #[test]
    fn month_day_is_bounded() {
        assert!(MonthDay::new(0).is_err());
        assert!(MonthDay::new(29).is_err());
        assert!(MonthDay::new(28).is_ok());
        assert!(serde_json::from_str::<MonthDay>("31").is_err());
    }

    #[test]
    fn reminder_before_due_in_istanbul() {
        let due = Due::at(d(2026, 11, 12), NaiveTime::from_hms_opt(10, 0, 0).unwrap());
        let r = Reminder::Before { minutes: 60 };
        let fire = r.fire_at(Some(&due), chrono_tz::Europe::Istanbul).unwrap();
        // İstanbul UTC+3: 10:00 yerel = 07:00 UTC; bir saat önce = 06:00 UTC.
        assert_eq!(fire, Utc.with_ymd_and_hms(2026, 11, 12, 6, 0, 0).unwrap());
        assert_eq!(
            r.fire_at(None, chrono_tz::Europe::Istanbul).unwrap_err(),
            CoreError::ReminderNeedsDue
        );
    }

    #[test]
    fn date_only_due_uses_default_hour() {
        let due = Due::on(d(2026, 11, 14));
        assert_eq!(
            due.instant(chrono_tz::Europe::Istanbul),
            Utc.with_ymd_and_hms(2026, 11, 14, 6, 0, 0).unwrap()
        );
    }

    #[test]
    fn undated_task_cannot_recur_or_remind_before() {
        let mut n = NewTask::titled(title("Kitap notu"));
        n.recurrence = Some(Recurrence::Day);
        assert_eq!(Task::create(n).unwrap_err(), CoreError::ReminderNeedsDue);
        let mut n = NewTask::titled(title("Kitap notu"));
        n.reminder = Some(Reminder::Before { minutes: 10 });
        assert!(Task::create(n).is_err());
    }

    #[test]
    fn overdue_logic() {
        let mut n = NewTask::titled(title("Kostik tekliflerini karşılaştır"));
        n.due = Some(Due::on(d(2026, 11, 5)));
        let t = Task::create(n).unwrap();
        assert!(t.is_overdue(d(2026, 11, 6)));
        assert_eq!(t.days_overdue(d(2026, 11, 6)), 1);
        assert!(!t.is_overdue(d(2026, 11, 5)));
        assert!(t.is_due_on(d(2026, 11, 5)));
    }

    #[test]
    fn completing_recurring_task_spawns_next() {
        let now = Utc.with_ymd_and_hms(2026, 11, 6, 18, 0, 0).unwrap();
        let mut n = NewTask::titled(title("Akşam yürüyüşü"));
        n.due = Some(Due::at(
            d(2026, 11, 6),
            NaiveTime::from_hms_opt(19, 0, 0).unwrap(),
        ));
        n.recurrence = Some(Recurrence::Day);
        let mut t = Task::create(n).unwrap();
        let next = t.complete(now).unwrap().unwrap();
        assert_eq!(t.state(), TaskState::Done { at: now });
        assert!(next.is_open());
        assert_ne!(next.id, t.id);
        assert_eq!(next.due.unwrap().date, d(2026, 11, 7));
        assert_eq!(next.due.unwrap().time, t.due.unwrap().time);
        assert_eq!(t.complete(now).unwrap_err(), CoreError::TaskClosed);
    }

    #[test]
    fn non_recurring_completion_and_reopen() {
        let now = Utc.with_ymd_and_hms(2026, 11, 6, 18, 0, 0).unwrap();
        let mut t = Task::create(NewTask::titled(title("Fatura öde"))).unwrap();
        assert!(t.complete(now).unwrap().is_none());
        assert!(t.cancel(now).is_err());
        t.reopen();
        assert!(t.is_open());
    }

    #[test]
    fn state_parts_round_trip() {
        let at = Utc.with_ymd_and_hms(2026, 11, 6, 18, 0, 0).unwrap();
        for s in [
            TaskState::Open,
            TaskState::Done { at },
            TaskState::Cancelled { at },
        ] {
            assert_eq!(TaskState::from_parts(s.name(), s.closed_at()).unwrap(), s);
        }
        assert!(TaskState::from_parts("done", None).is_err());
    }

    #[test]
    fn recurrence_serde_shape() {
        let r = Recurrence::Week { on: Weekday::Sun };
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, r#"{"every":"week","on":"Sun"}"#);
        let back: Recurrence = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }
}
