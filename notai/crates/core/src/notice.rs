//! Bildirimler (Gökhan, 2026-11-07): metin kuralları ve "son bildirim".
//!
//! - Bildirim başlığında konu adı görünür ("Hatırlatma · Ev").
//! - Her bildirim hangi konuya/kayda ait olduğunu taşır; dokununca (Android) ya da uygulama
//!   açılınca çıkan "son bildirim" şeridinden o konu açılır.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{EntryId, TopicId};

/// "Son bildirim" şeridi bu süreden eskiyse gösterilmez.
pub const FRESH_FOR_MINUTES: i64 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoticeKind {
    Reminder,
    Question,
}

/// Gönderilen bir bildirim ve gideceği yer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notice {
    pub kind: NoticeKind,
    pub title: String,
    pub body: String,
    pub topic: Option<TopicId>,
    pub topic_name: Option<String>,
    pub entry: Option<EntryId>,
    pub at: DateTime<Utc>,
}

fn titled(base: &str, topic_name: Option<&str>) -> String {
    match topic_name {
        Some(n) if !n.trim().is_empty() => format!("{base} · {n}"),
        _ => base.to_owned(),
    }
}

impl Notice {
    /// Hatırlatma: "Hatırlatma · Ev" / "Kombi servisini ara".
    #[must_use]
    pub fn reminder(
        task_title: &str,
        topic: Option<(TopicId, String)>,
        entry: Option<EntryId>,
        at: DateTime<Utc>,
    ) -> Self {
        let (topic, topic_name) = topic.map_or((None, None), |(id, n)| (Some(id), Some(n)));
        Self {
            kind: NoticeKind::Reminder,
            title: titled("Hatırlatma", topic_name.as_deref()),
            body: task_title.to_owned(),
            topic,
            topic_name,
            entry,
            at,
        }
    }

    /// AI sorusu: "NotAI soruyor · Kostik tedarik" / "Hangi tedarikçi için?".
    #[must_use]
    pub fn question(
        question: &str,
        topic: Option<(TopicId, String)>,
        entry: EntryId,
        at: DateTime<Utc>,
    ) -> Self {
        let (topic, topic_name) = topic.map_or((None, None), |(id, n)| (Some(id), Some(n)));
        Self {
            kind: NoticeKind::Question,
            title: titled("NotAI soruyor", topic_name.as_deref()),
            body: question.to_owned(),
            topic,
            topic_name,
            entry: Some(entry),
            at,
        }
    }

    /// Şeritte gösterilecek kadar yeni mi?
    #[must_use]
    pub fn is_fresh(&self, now: DateTime<Utc>) -> bool {
        now >= self.at && now - self.at <= Duration::minutes(FRESH_FOR_MINUTES)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use chrono::TimeZone;

    fn at(m: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 8, 7, 0, 0).unwrap() + Duration::minutes(m)
    }

    #[test]
    fn titles_carry_topic_name() {
        let ev = TopicId::new();
        let n = Notice::reminder("Kombi servisini ara", Some((ev, "Ev".into())), None, at(0));
        assert_eq!(n.title, "Hatırlatma · Ev");
        assert_eq!(n.body, "Kombi servisini ara");
        assert_eq!(n.topic, Some(ev));
        let plain = Notice::reminder("Su faturası", None, None, at(0));
        assert_eq!(plain.title, "Hatırlatma");
        let q = Notice::question("Hangi tedarikçi?", None, EntryId::new(), at(0));
        assert_eq!(q.title, "NotAI soruyor");
        assert_eq!(q.kind, NoticeKind::Question);
    }

    #[test]
    fn freshness_window() {
        let n = Notice::reminder("x", None, None, at(0));
        assert!(n.is_fresh(at(0)));
        assert!(n.is_fresh(at(FRESH_FOR_MINUTES)));
        assert!(!n.is_fresh(at(FRESH_FOR_MINUTES + 1)));
        assert!(!n.is_fresh(at(-1)), "gelecekteki bildirim gösterilmez");
    }
}
