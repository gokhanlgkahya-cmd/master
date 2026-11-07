//! Takvim gündemi ve haftalık gözden geçirme (saf mantık).

use chrono::{DateTime, Datelike, Days, Duration, NaiveDate, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::ids::{TaskId, TopicId};
use crate::task::Task;
use crate::topic::{ParaKind, Topic};
use crate::tr::{month_short, weekday_name};

/// Takvimde bir gün.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgendaDay {
    pub date: NaiveDate,
    /// "Cum 6 Kas".
    pub label: String,
    pub is_today: bool,
    /// O günün görevleri: saatliler saate göre önce, sonra saatsizler.
    pub tasks: Vec<TaskId>,
}

/// `from` gününden başlayarak `days` günlük gündem.
#[must_use]
pub fn agenda(tasks: &[Task], from: NaiveDate, days: u32, today: NaiveDate) -> Vec<AgendaDay> {
    (0..days)
        .map(|i| {
            let date = from + Days::new(u64::from(i));
            let mut list: Vec<&Task> = tasks
                .iter()
                .filter(|t| t.due.is_some_and(|d| d.date == date))
                .collect();
            list.sort_by_key(|t| {
                (
                    t.due.and_then(|d| d.time).is_none(),
                    t.due.and_then(|d| d.time),
                )
            });
            let wd: String = weekday_name(date.weekday()).chars().take(3).collect();
            AgendaDay {
                date,
                label: format!("{wd} {} {}", date.day(), month_short(date.month())),
                is_today: date == today,
                tasks: list.into_iter().map(|t| t.id).collect(),
            }
        })
        .collect()
}

/// Haftanın pazartesisi.
#[must_use]
pub fn week_start(date: NaiveDate) -> NaiveDate {
    date - Days::new(u64::from(date.weekday().num_days_from_monday()))
}

/// İlerlemeyen proje: bu kadar gündür kayıt ya da görev hareketi yok.
pub const STALLED_DAYS: i64 = 7;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StalledProject {
    pub topic: TopicId,
    pub name: String,
    /// Son hareketten bu yana gün (hiç hareket yoksa `None`).
    pub idle_days: Option<i64>,
}

/// Konunun son hareketi (kayıt ya da görev).
#[derive(Debug, Clone)]
pub struct TopicActivityTimes {
    pub topic: Topic,
    pub last_activity: Option<DateTime<Utc>>,
}

/// Haftalık gözden geçirme özeti.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeeklyReview {
    pub week_start: NaiveDate,
    /// İşlenmemiş kayıt sayısı (1. adım: gelen kutusunu boşalt).
    pub unprocessed: u64,
    /// 2. adım: ilerlemeyen projeler.
    pub stalled: Vec<StalledProject>,
    /// 3. adım: önümüzdeki 7 günün görevleri.
    pub upcoming: Vec<TaskId>,
    pub overdue: usize,
    /// Son 7 günde tamamlanan görev sayısı.
    pub done_last_week: u64,
}

#[must_use]
pub fn weekly_review(
    unprocessed: u64,
    topics: &[TopicActivityTimes],
    open_tasks: &[Task],
    done_last_week: u64,
    now: DateTime<Utc>,
    tz: Tz,
) -> WeeklyReview {
    let today = now.with_timezone(&tz).date_naive();
    let mut stalled: Vec<StalledProject> = topics
        .iter()
        .filter(|a| matches!(a.topic.kind, ParaKind::Project { .. }))
        .filter_map(|a| {
            let idle = a.last_activity.map(|t| (now - t).num_days());
            idle.is_none_or(|d| d >= STALLED_DAYS)
                .then(|| StalledProject {
                    topic: a.topic.id,
                    name: a.topic.name.as_str().to_owned(),
                    idle_days: idle,
                })
        })
        .collect();
    stalled.sort_by_key(|s| std::cmp::Reverse(s.idle_days.unwrap_or(i64::MAX)));
    let horizon = today + Duration::days(7);
    let mut upcoming: Vec<&Task> = open_tasks
        .iter()
        .filter(|t| t.is_open() && t.due.is_some_and(|d| d.date >= today && d.date <= horizon))
        .collect();
    upcoming.sort_by_key(|t| t.due);
    WeeklyReview {
        week_start: week_start(today),
        unprocessed,
        stalled,
        upcoming: upcoming.into_iter().map(|t| t.id).collect(),
        overdue: open_tasks.iter().filter(|t| t.is_overdue(today)).count(),
        done_last_week,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Due, NewTask};
    use crate::text::{TaskTitle, TopicName};
    use chrono::{NaiveTime, TimeZone};

    const TZ: Tz = chrono_tz::Europe::Istanbul;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    fn task(title: &str, due: Option<Due>) -> Task {
        let mut n = NewTask::titled(TaskTitle::new(title).unwrap());
        n.due = due;
        Task::create(n).unwrap()
    }

    #[test]
    fn agenda_days_and_order() {
        let tasks = vec![
            task(
                "Akşam",
                Some(Due::at(d(6), NaiveTime::from_hms_opt(19, 0, 0).unwrap())),
            ),
            task("Tarihsiz saat", Some(Due::on(d(6)))),
            task(
                "Sabah",
                Some(Due::at(d(6), NaiveTime::from_hms_opt(9, 0, 0).unwrap())),
            ),
            task("Yarın", Some(Due::on(d(7)))),
        ];
        let a = agenda(&tasks, week_start(d(6)), 7, d(6));
        assert_eq!(a.len(), 7);
        assert_eq!(a[0].label, "Paz 2 Kas");
        let fri = &a[4];
        assert!(fri.is_today);
        assert_eq!(fri.label, "Cum 6 Kas");
        let titles: Vec<_> = fri
            .tasks
            .iter()
            .map(|id| tasks.iter().find(|t| t.id == *id).unwrap().title.as_str())
            .collect();
        assert_eq!(titles, ["Sabah", "Akşam", "Tarihsiz saat"]);
        assert_eq!(a[5].tasks.len(), 1);
    }

    #[test]
    fn review_finds_stalled_projects_and_upcoming() {
        let now = Utc.with_ymd_and_hms(2026, 11, 8, 17, 0, 0).unwrap();
        let topic = |name: &str, kind| Topic::new(TopicName::new(name).unwrap(), kind);
        let topics = vec![
            TopicActivityTimes {
                topic: topic("Logo ERP geçişi", ParaKind::Project { due: None }),
                last_activity: Some(now - Duration::days(10)),
            },
            TopicActivityTimes {
                topic: topic("Kostik tedarik", ParaKind::Project { due: None }),
                last_activity: Some(now - Duration::days(1)),
            },
            TopicActivityTimes {
                topic: topic("Ev sunucusu", ParaKind::Project { due: None }),
                last_activity: None,
            },
            TopicActivityTimes {
                topic: topic("Ev", ParaKind::Area),
                last_activity: None,
            },
        ];
        let tasks = vec![
            task("Geciken", Some(Due::on(d(5)))),
            task("Pazartesi", Some(Due::on(d(9)))),
            task("Uzak", Some(Due::on(d(30)))),
        ];
        let r = weekly_review(6, &topics, &tasks, 14, now, TZ);
        let names: Vec<_> = r.stalled.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Ev sunucusu", "Logo ERP geçişi"]);
        assert_eq!(r.upcoming.len(), 1);
        assert_eq!(r.overdue, 1);
        assert_eq!(r.week_start, d(2));
        assert_eq!(r.unprocessed, 6);
    }
}
