//! Sabah planı: geciken → bugünkü → yüksek öncelikli tarihsiz görevler, en fazla 3 madde.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::ids::{TaskId, TopicId};
use crate::task::{Priority, Task};

/// Plandaki en fazla madde (Faz 2 kararı: ferah yoğunluk).
pub const MORNING_PLAN_MAX: usize = 3;

/// Maddenin plana girme nedeni.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum PlanReason {
    Overdue { days: i64 },
    Today,
    HighPriority,
}

impl PlanReason {
    /// Kart üzerindeki kısa açıklama ("1 gün gecikti", "bugün", "öncelikli").
    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::Overdue { days } => format!("{days} gün gecikti"),
            Self::Today => "bugün".to_owned(),
            Self::HighPriority => "öncelikli".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanItem {
    pub task: TaskId,
    pub title: String,
    pub reason: PlanReason,
    pub topic: Option<TopicId>,
    pub time: Option<NaiveTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MorningPlan {
    pub date: NaiveDate,
    pub items: Vec<PlanItem>,
    /// Plana sığmayan aday sayısı ("+N görev daha").
    pub more: usize,
}

impl MorningPlan {
    /// Bildirim başlığı ("Günaydın Gökhan. Bugün 3 iş öne çıkıyor").
    #[must_use]
    pub fn headline(&self, name: &str) -> String {
        match self.items.len() {
            0 => format!("Günaydın {name}. Bugün için planlanmış iş yok."),
            n => format!("Günaydın {name}. Bugün {n} iş öne çıkıyor"),
        }
    }
}

/// Açık görevlerden sabah planını kurar.
#[must_use]
pub fn build(tasks: &[Task], today: NaiveDate) -> MorningPlan {
    let open = || tasks.iter().filter(|t| t.is_open());

    let mut overdue: Vec<&Task> = open().filter(|t| t.is_overdue(today)).collect();
    overdue.sort_by(|a, b| {
        b.days_overdue(today)
            .cmp(&a.days_overdue(today))
            .then_with(|| b.priority.cmp(&a.priority))
    });

    let mut due_today: Vec<&Task> = open().filter(|t| t.is_due_on(today)).collect();
    // Saatliler saat sırasıyla önce; saatsizler önceliğe göre sonra.
    due_today.sort_by(|a, b| {
        let ta = a.due.and_then(|d| d.time);
        let tb = b.due.and_then(|d| d.time);
        match (ta, tb) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => b.priority.cmp(&a.priority),
        }
    });

    let high_undated = open().filter(|t| t.due.is_none() && t.priority == Priority::High);

    let candidates: Vec<(&Task, PlanReason)> = overdue
        .into_iter()
        .map(|t| {
            (
                t,
                PlanReason::Overdue {
                    days: t.days_overdue(today),
                },
            )
        })
        .chain(due_today.into_iter().map(|t| (t, PlanReason::Today)))
        .chain(high_undated.map(|t| (t, PlanReason::HighPriority)))
        .collect();

    let more = candidates.len().saturating_sub(MORNING_PLAN_MAX);
    let items = candidates
        .into_iter()
        .take(MORNING_PLAN_MAX)
        .map(|(t, reason)| PlanItem {
            task: t.id,
            title: t.title.as_str().to_owned(),
            reason,
            topic: t.topic,
            time: t.due.and_then(|d| d.time),
        })
        .collect();

    MorningPlan {
        date: today,
        items,
        more,
    }
}

/// Zamanlayıcı için: sabah planı şimdi gönderilmeli mi?
/// Yerel saat gönderim saatini geçtiyse ve bugün henüz gönderilmediyse evet.
#[must_use]
pub fn is_due(
    now: DateTime<Utc>,
    tz: Tz,
    send_at: NaiveTime,
    last_sent: Option<NaiveDate>,
) -> bool {
    let local = now.with_timezone(&tz);
    let today = local.date_naive();
    local.time() >= send_at && last_sent != Some(today)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Due, NewTask};
    use crate::text::TaskTitle;
    use chrono::TimeZone;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    fn task(title: &str, due: Option<Due>, priority: Priority) -> Task {
        let mut n = NewTask::titled(TaskTitle::new(title).unwrap());
        n.due = due;
        n.priority = priority;
        Task::create(n).unwrap()
    }

    fn hm(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    #[test]
    fn order_overdue_then_today_then_high_priority() {
        let tasks = vec![
            task(
                "Akşam yürüyüşü",
                Some(Due::at(d(6), hm(19, 0))),
                Priority::Normal,
            ),
            task("Kitap notu", None, Priority::Low),
            task(
                "Logo: 37 hatalı stok kartını düzelt",
                Some(Due::on(d(6))),
                Priority::High,
            ),
            task(
                "Kostik tekliflerini karşılaştır",
                Some(Due::on(d(5))),
                Priority::Normal,
            ),
            task("Numune yolla", None, Priority::High),
        ];
        let plan = build(&tasks, d(6));
        let titles: Vec<_> = plan.items.iter().map(|i| i.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "Kostik tekliflerini karşılaştır",
                "Akşam yürüyüşü",
                "Logo: 37 hatalı stok kartını düzelt"
            ]
        );
        assert_eq!(plan.items[0].reason, PlanReason::Overdue { days: 1 });
        assert_eq!(plan.items[0].reason.label(), "1 gün gecikti");
        assert_eq!(plan.items[1].time, Some(hm(19, 0)));
        assert_eq!(plan.more, 1, "Numune yolla sığmadı");
        assert_eq!(
            plan.headline("Gökhan"),
            "Günaydın Gökhan. Bugün 3 iş öne çıkıyor"
        );
    }

    #[test]
    fn closed_and_future_tasks_are_excluded() {
        let now = Utc.with_ymd_and_hms(2026, 11, 6, 5, 0, 0).unwrap();
        let mut done = task("Bitti", Some(Due::on(d(6))), Priority::High);
        done.complete(now).unwrap();
        let future = task("Haftaya", Some(Due::on(d(13))), Priority::High);
        let plan = build(&[done, future], d(6));
        assert!(plan.items.is_empty());
        assert_eq!(plan.more, 0);
        assert_eq!(
            plan.headline("Gökhan"),
            "Günaydın Gökhan. Bugün için planlanmış iş yok."
        );
    }

    #[test]
    fn scheduler_fires_once_per_day_after_send_time() {
        let tz = chrono_tz::Europe::Istanbul;
        let at = |h| Utc.with_ymd_and_hms(2026, 11, 6, h, 0, 0).unwrap();
        // 04:00 UTC = 07:00 İstanbul → henüz değil.
        assert!(!is_due(at(4), tz, hm(8, 0), None));
        // 05:00 UTC = 08:00 İstanbul → gönder.
        assert!(is_due(at(5), tz, hm(8, 0), None));
        assert!(is_due(at(5), tz, hm(8, 0), Some(d(5))));
        // Bugün gönderildiyse tekrar gönderme.
        assert!(!is_due(at(9), tz, hm(8, 0), Some(d(6))));
    }
}
