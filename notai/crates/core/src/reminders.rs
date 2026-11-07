//! Hatırlatmalar: gruplama (Bugün / Yarın / Bu hafta / Sonra / Tekrarlayan), erteleme,
//! zamanı gelenlerin seçimi.

use chrono::{DateTime, Datelike, Days, Duration, NaiveTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::ids::{TaskId, TopicId};
use crate::task::{DEFAULT_REMINDER_HOUR, Reminder, Task, local_to_utc};

/// Çok eski (24 saatten önce kaçırılmış) hatırlatmalar artık çalmaz; listede görünür.
pub const MAX_LATE_HOURS: i64 = 24;

/// Erteleme seçenekleri ("10 dk", "1 sa", "Yarın").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Snooze {
    Minutes10,
    Hour1,
    Tomorrow,
}

impl Snooze {
    pub fn parse(s: &str) -> Result<Self, CoreError> {
        match s {
            "minutes10" | "10m" => Ok(Self::Minutes10),
            "hour1" | "1h" => Ok(Self::Hour1),
            "tomorrow" => Ok(Self::Tomorrow),
            other => Err(CoreError::UnknownVariant {
                field: "erteleme",
                value: other.to_owned(),
            }),
        }
    }

    /// Yeni çalma anı.
    #[must_use]
    pub fn next(self, now: DateTime<Utc>, tz: Tz) -> DateTime<Utc> {
        match self {
            Self::Minutes10 => now + Duration::minutes(10),
            Self::Hour1 => now + Duration::hours(1),
            Self::Tomorrow => {
                let tomorrow = now.with_timezone(&tz).date_naive() + Days::new(1);
                let nine =
                    NaiveTime::from_hms_opt(DEFAULT_REMINDER_HOUR, 0, 0).unwrap_or(NaiveTime::MIN);
                local_to_utc(tz, tomorrow.and_time(nine))
            }
        }
    }
}

/// Görevin hatırlatmasını erteler (tekrarlı olsa da yalnız bu örneği).
pub fn snooze(task: &mut Task, s: Snooze, now: DateTime<Utc>, tz: Tz) -> Result<(), CoreError> {
    if !task.is_open() {
        return Err(CoreError::TaskClosed);
    }
    task.reminder = Some(Reminder::At {
        at: s.next(now, tz),
    });
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderGroup {
    Missed,
    Today,
    Tomorrow,
    ThisWeek,
    Later,
    Recurring,
}

impl ReminderGroup {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Missed => "Kaçırılan",
            Self::Today => "Bugün",
            Self::Tomorrow => "Yarın",
            Self::ThisWeek => "Bu hafta",
            Self::Later => "Sonra",
            Self::Recurring => "Tekrarlayan",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderItem {
    pub task: TaskId,
    pub title: String,
    pub topic: Option<TopicId>,
    pub fire_at: DateTime<Utc>,
    pub recurring: bool,
}

/// Açık görevlerin hatırlatmalarını gruplara ayırır; grup içinde zamana göre sıralı.
#[must_use]
pub fn group(
    tasks: &[Task],
    now: DateTime<Utc>,
    tz: Tz,
) -> Vec<(ReminderGroup, Vec<ReminderItem>)> {
    let today = now.with_timezone(&tz).date_naive();
    let days_to_sunday = 6 - i64::from(today.weekday().num_days_from_monday());
    let mut items: Vec<(ReminderGroup, ReminderItem)> = Vec::new();
    for t in tasks.iter().filter(|t| t.is_open()) {
        let Some(r) = t.reminder else { continue };
        let Ok(fire_at) = r.fire_at(t.due.as_ref(), tz) else {
            continue;
        };
        let date = fire_at.with_timezone(&tz).date_naive();
        let diff = (date - today).num_days();
        let group = if t.recurrence.is_some() {
            ReminderGroup::Recurring
        } else if fire_at < now && diff < 0 {
            ReminderGroup::Missed
        } else if diff <= 0 {
            ReminderGroup::Today
        } else if diff == 1 {
            ReminderGroup::Tomorrow
        } else if diff <= days_to_sunday {
            ReminderGroup::ThisWeek
        } else {
            ReminderGroup::Later
        };
        items.push((
            group,
            ReminderItem {
                task: t.id,
                title: t.title.as_str().to_owned(),
                topic: t.topic,
                fire_at,
                recurring: t.recurrence.is_some(),
            },
        ));
    }
    items.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.fire_at.cmp(&b.1.fire_at)));
    let mut out: Vec<(ReminderGroup, Vec<ReminderItem>)> = Vec::new();
    for (g, item) in items {
        match out.last_mut() {
            Some((last, v)) if *last == g => v.push(item),
            _ => out.push((g, vec![item])),
        }
    }
    out
}

/// Şimdi çalması gereken hatırlatmalar: zamanı geldi, henüz bu an için çalmadı ve
/// 24 saatten eski değil. `fired_at`: görevin son bildirim anı.
#[must_use]
pub fn due_now<'a>(
    tasks: &'a [(Task, Option<DateTime<Utc>>)],
    now: DateTime<Utc>,
    tz: Tz,
) -> Vec<(&'a Task, DateTime<Utc>)> {
    tasks
        .iter()
        .filter(|(t, _)| t.is_open())
        .filter_map(|(t, fired)| {
            let at = t.reminder?.fire_at(t.due.as_ref(), tz).ok()?;
            let fresh = now - at <= Duration::hours(MAX_LATE_HOURS);
            let not_fired = fired.is_none_or(|f| f < at);
            (at <= now && fresh && not_fired).then_some((t, at))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Due, NewTask, Recurrence};
    use crate::text::TaskTitle;
    use chrono::{NaiveDate, TimeZone};

    const TZ: Tz = chrono_tz::Europe::Istanbul;

    fn now() -> DateTime<Utc> {
        // Cuma 6 Kasım 2026, 12:00 İstanbul.
        Utc.with_ymd_and_hms(2026, 11, 6, 9, 0, 0).unwrap()
    }

    fn task_at(title: &str, at: DateTime<Utc>) -> Task {
        let mut n = NewTask::titled(TaskTitle::new(title).unwrap());
        n.reminder = Some(Reminder::At { at });
        Task::create(n).unwrap()
    }

    #[test]
    fn groups_by_day_and_recurring() {
        let mut walk = NewTask::titled(TaskTitle::new("Akşam yürüyüşü").unwrap());
        walk.due = Some(Due::at(
            NaiveDate::from_ymd_opt(2026, 11, 6).unwrap(),
            NaiveTime::from_hms_opt(19, 0, 0).unwrap(),
        ));
        walk.recurrence = Some(Recurrence::Day);
        walk.reminder = Some(Reminder::Before { minutes: 15 });
        let tasks = vec![
            task_at(
                "Tansiyon ölçümü",
                Utc.with_ymd_and_hms(2026, 11, 6, 17, 0, 0).unwrap(),
            ),
            task_at(
                "Sabun kalıbı siparişi",
                Utc.with_ymd_and_hms(2026, 11, 7, 6, 0, 0).unwrap(),
            ),
            task_at(
                "Haftalık gözden geçirme",
                Utc.with_ymd_and_hms(2026, 11, 8, 16, 0, 0).unwrap(),
            ),
            task_at(
                "Kostik tedarikçisini ara",
                Utc.with_ymd_and_hms(2026, 11, 12, 6, 0, 0).unwrap(),
            ),
            task_at(
                "Dün kaçtı",
                Utc.with_ymd_and_hms(2026, 11, 5, 6, 0, 0).unwrap(),
            ),
            Task::create(walk).unwrap(),
        ];
        let g = group(&tasks, now(), TZ);
        let shape: Vec<_> = g.iter().map(|(g, v)| (g.label(), v.len())).collect();
        assert_eq!(
            shape,
            [
                ("Kaçırılan", 1),
                ("Bugün", 1),
                ("Yarın", 1),
                ("Bu hafta", 1),
                ("Sonra", 1),
                ("Tekrarlayan", 1)
            ]
        );
    }

    #[test]
    fn snooze_options() {
        let mut t = task_at("Muayene", now());
        snooze(&mut t, Snooze::Minutes10, now(), TZ).unwrap();
        assert_eq!(
            t.reminder,
            Some(Reminder::At {
                at: now() + Duration::minutes(10)
            })
        );
        snooze(&mut t, Snooze::Tomorrow, now(), TZ).unwrap();
        assert_eq!(
            t.reminder,
            Some(Reminder::At {
                at: Utc.with_ymd_and_hms(2026, 11, 7, 6, 0, 0).unwrap()
            })
        );
        assert_eq!(Snooze::parse("1h").unwrap(), Snooze::Hour1);
        assert!(Snooze::parse("asla").is_err());
        t.complete(now()).unwrap();
        assert!(snooze(&mut t, Snooze::Hour1, now(), TZ).is_err());
    }

    #[test]
    fn due_now_fires_once_and_skips_stale() {
        let at = now() - Duration::minutes(1);
        let fresh = task_at("Şimdi", at);
        let future = task_at("Sonra", now() + Duration::hours(1));
        let stale = task_at("Çok eski", now() - Duration::hours(30));
        let list = vec![(fresh.clone(), None), (future, None), (stale, None)];
        let due = due_now(&list, now(), TZ);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].0.title.as_str(), "Şimdi");
        // Bu an için zaten çaldıysa tekrar çalmaz; ertelenince yeni an için çalar.
        assert!(due_now(&[(fresh.clone(), Some(at))], now(), TZ).is_empty());
        assert_eq!(
            due_now(&[(fresh, Some(at - Duration::hours(2)))], now(), TZ).len(),
            1
        );
    }
}
