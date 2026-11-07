//! Ana Günlük ve konu akışlarının zaman çizelgesi kuralları.
//!
//! WhatsApp mantığı: günler ayraçla ayrılır, en yeni kayıt en altta.

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::entry::{Entry, EntryStatus};
use crate::ids::TopicId;
use crate::topic::Topic;
use crate::tr::{date_long, month_short, tr_lower, weekday_name};

/// Akış süzgeci.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "filter", rename_all = "snake_case")]
pub enum JournalFilter {
    /// Ana Günlük: çöp ve arşiv dışındaki tüm kayıtlar.
    #[default]
    All,
    /// Yalnız işlenmemişler.
    Unprocessed,
    /// Bir konunun akışı.
    Topic { id: TopicId },
}

impl JournalFilter {
    #[must_use]
    pub fn matches(self, entry: &Entry) -> bool {
        let visible = matches!(entry.status(), EntryStatus::Inbox | EntryStatus::Processed);
        visible
            && match self {
                Self::All => true,
                Self::Unprocessed => entry.is_unprocessed(),
                Self::Topic { id } => entry.topic() == Some(id),
            }
    }
}

/// Bir günün kayıtları.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayGroup<T> {
    pub date: NaiveDate,
    /// Ayraç metni: "Bugün", "Dün", "Çarşamba, 4 Kasım", "28 Ekim".
    pub label: String,
    pub items: Vec<T>,
}

/// Gün ayracı metni.
#[must_use]
pub fn day_label(date: NaiveDate, today: NaiveDate) -> String {
    let diff = (today - date).num_days();
    match diff {
        0 => "Bugün".to_owned(),
        1 => "Dün".to_owned(),
        2..=6 => format!(
            "{}, {}",
            weekday_name(date.weekday()),
            date_long(date, today.year())
        ),
        _ => date_long(date, today.year()),
    }
}

/// Konu listesindeki zaman etiketi: bugün "12:05", dün "Dün", bu hafta "Çarşamba",
/// daha eski "2 Kas" (yıl farklıysa "2 Kas 2025").
#[must_use]
pub fn list_time_label(at: DateTime<Utc>, now: DateTime<Utc>, tz: Tz) -> String {
    let local = at.with_timezone(&tz);
    let date = local.date_naive();
    let today = now.with_timezone(&tz).date_naive();
    match (today - date).num_days() {
        i64::MIN..=0 => local.format("%H:%M").to_string(),
        1 => "Dün".to_owned(),
        2..=6 => weekday_name(date.weekday()).to_owned(),
        _ if date.year() == today.year() => format!("{} {}", date.day(), month_short(date.month())),
        _ => format!(
            "{} {} {}",
            date.day(),
            month_short(date.month()),
            date.year()
        ),
    }
}

/// Kayıtları yerel güne göre gruplar. Gruplar ve grup içi kayıtlar eskiden yeniye sıralıdır
/// (en yeni en altta). Aynı andaki kayıtların sırası korunur.
pub fn group_by_day<T, F>(mut items: Vec<T>, at: F, now: DateTime<Utc>, tz: Tz) -> Vec<DayGroup<T>>
where
    F: Fn(&T) -> DateTime<Utc>,
{
    items.sort_by_key(|i| at(i));
    let today = now.with_timezone(&tz).date_naive();
    let mut groups: Vec<DayGroup<T>> = Vec::new();
    for item in items {
        let date = at(&item).with_timezone(&tz).date_naive();
        match groups.last_mut() {
            Some(g) if g.date == date => g.items.push(item),
            _ => groups.push(DayGroup {
                date,
                label: day_label(date, today),
                items: vec![item],
            }),
        }
    }
    groups
}

/// İşlenmemiş kayıt sayısı (Ana Günlük rozeti).
#[must_use]
pub fn unprocessed_count<'a>(entries: impl IntoIterator<Item = &'a Entry>) -> usize {
    entries.into_iter().filter(|e| e.is_unprocessed()).count()
}

/// Ana ekrandaki konu satırı için sıralama girdisi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicActivity {
    pub topic: Topic,
    pub last_at: Option<DateTime<Utc>>,
    pub last_preview: Option<String>,
}

/// Sabitlenenler önce; sonra son etkinliğe göre (yeni üstte); hiç kaydı olmayanlar en altta,
/// Türkçe küçük harfe göre ada göre.
pub fn order_topics(topics: &mut [TopicActivity]) {
    topics.sort_by(|a, b| {
        b.topic
            .pinned
            .cmp(&a.topic.pinned)
            .then_with(|| b.last_at.cmp(&a.last_at))
            .then_with(|| tr_lower(a.topic.name.as_str()).cmp(&tr_lower(b.topic.name.as_str())))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::Source;
    use crate::text::{EntryText, TopicName};
    use crate::topic::ParaKind;
    use chrono::TimeZone;

    const TZ: Tz = chrono_tz::Europe::Istanbul;

    fn utc(m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, m, d, h, min, 0).unwrap()
    }

    fn now() -> DateTime<Utc> {
        // Cuma 6 Kasım 2026, 12:07 İstanbul.
        utc(11, 6, 9, 7)
    }

    fn date(m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, m, d).unwrap()
    }

    #[test]
    fn day_labels() {
        let today = date(11, 6);
        assert_eq!(day_label(date(11, 6), today), "Bugün");
        assert_eq!(day_label(date(11, 5), today), "Dün");
        assert_eq!(day_label(date(11, 4), today), "Çarşamba, 4 Kasım");
        assert_eq!(day_label(date(10, 28), today), "28 Ekim");
        assert_eq!(
            day_label(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(), today),
            "31 Aralık 2025"
        );
    }

    #[test]
    fn list_labels() {
        assert_eq!(list_time_label(utc(11, 6, 9, 5), now(), TZ), "12:05");
        assert_eq!(list_time_label(utc(11, 5, 9, 5), now(), TZ), "Dün");
        assert_eq!(list_time_label(utc(11, 4, 9, 5), now(), TZ), "Çarşamba");
        assert_eq!(list_time_label(utc(11, 2, 9, 5), now(), TZ), "Pazartesi");
        assert_eq!(list_time_label(utc(10, 30, 9, 5), now(), TZ), "30 Eki");
    }

    #[test]
    fn local_midnight_decides_the_day() {
        // 5 Kasım 21:30 UTC = 6 Kasım 00:30 İstanbul → "Bugün".
        let groups = group_by_day(vec![utc(11, 5, 21, 30)], |t| *t, now(), TZ);
        assert_eq!(groups[0].label, "Bugün");
    }

    #[test]
    fn groups_oldest_first_newest_last() {
        let items = vec![
            utc(11, 6, 9, 5),
            utc(11, 4, 15, 42),
            utc(11, 6, 6, 12),
            utc(11, 5, 5, 15),
        ];
        let groups = group_by_day(items, |t| *t, now(), TZ);
        let labels: Vec<_> = groups.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(labels, ["Çarşamba, 4 Kasım", "Dün", "Bugün"]);
        assert_eq!(groups[2].items, vec![utc(11, 6, 6, 12), utc(11, 6, 9, 5)]);
    }

    #[test]
    fn filters() {
        let topic = TopicId::new();
        let text = |s: &str| EntryText::new(s).unwrap();
        let inbox = Entry::capture(text("Şirket aracının muayenesi"), Source::App, now(), None);
        let in_topic = Entry::capture(text("Ambalaj faturası"), Source::App, now(), Some(topic));
        let mut archived = Entry::capture(text("Eski not"), Source::App, now(), None);
        archived.archive().unwrap();
        let mut trashed = Entry::capture(text("Silinen"), Source::App, now(), None);
        trashed.trash(now()).unwrap();

        let all = [&inbox, &in_topic, &archived, &trashed];
        let count = |f: JournalFilter| all.iter().filter(|e| f.matches(e)).count();
        assert_eq!(count(JournalFilter::All), 2);
        assert_eq!(count(JournalFilter::Unprocessed), 1);
        assert_eq!(count(JournalFilter::Topic { id: topic }), 1);
        assert_eq!(unprocessed_count(all), 1);
    }

    #[test]
    fn topic_ordering() {
        let t = |name: &str, pinned: bool| {
            let mut t = Topic::new(TopicName::new(name).unwrap(), ParaKind::Area);
            t.pinned = pinned;
            t
        };
        let mut list = vec![
            TopicActivity {
                topic: t("Sağlık", false),
                last_at: Some(utc(11, 5, 6, 0)),
                last_preview: None,
            },
            TopicActivity {
                topic: t("İş", false),
                last_at: None,
                last_preview: None,
            },
            TopicActivity {
                topic: t("GemOlive", false),
                last_at: Some(utc(11, 6, 7, 47)),
                last_preview: None,
            },
            TopicActivity {
                topic: t("Ev", true),
                last_at: Some(utc(11, 1, 6, 0)),
                last_preview: None,
            },
            TopicActivity {
                topic: t("ılık", false),
                last_at: None,
                last_preview: None,
            },
        ];
        order_topics(&mut list);
        let names: Vec<_> = list.iter().map(|a| a.topic.name.as_str()).collect();
        // "ılık" < "iş" Türkçe küçük harfte (ı U+0131 > i U+0069 ama karşılaştırma tr_lower ile tutarlı).
        assert_eq!(names[..3], ["Ev", "GemOlive", "Sağlık"]);
        assert_eq!(names.len(), 5);
    }
}
