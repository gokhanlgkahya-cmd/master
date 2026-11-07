//! Satır ↔ alan tipi dönüşümleri. Veritabanından okunan her değer yeniden doğrulanır;
//! bozuk bir satır sessizce kabul edilmez, `StoreError::Corrupt` döner.

use chrono::{DateTime, NaiveDate, NaiveTime, SecondsFormat, Utc};
use notai_core::entry::{Entry, EntryStatus, Source};
use notai_core::suggestion::{Confidence, Decision, Suggestion, SuggestionKind};
use notai_core::task::{Due, Priority, Recurrence, Reminder, Task, TaskState};
use notai_core::text::{EntryText, TaskTitle, TopicName};
use notai_core::topic::{ParaKind, Topic};
use serde::de::DeserializeOwned;

use crate::error::{Result, StoreError};

/// Sabit biçimli UTC zaman metni (sözlük sırası = zaman sırası).
pub(crate) fn ts(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Nanos, true)
}

pub(crate) fn parse_ts(what: &'static str, s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| StoreError::corrupt(what, format!("{s}: {e}")))
}

pub(crate) fn parse_opt_ts(what: &'static str, s: Option<&str>) -> Result<Option<DateTime<Utc>>> {
    s.map(|s| parse_ts(what, s)).transpose()
}

pub(crate) fn date_str(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

pub(crate) fn parse_date(what: &'static str, s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| StoreError::corrupt(what, format!("{s}: {e}")))
}

pub(crate) fn time_str(t: NaiveTime) -> String {
    t.format("%H:%M").to_string()
}

pub(crate) fn parse_time(what: &'static str, s: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H:%M")
        .map_err(|e| StoreError::corrupt(what, format!("{s}: {e}")))
}

pub(crate) fn parse_id<T: std::str::FromStr<Err = notai_core::CoreError>>(
    what: &'static str,
    s: &str,
) -> Result<T> {
    s.parse().map_err(|e| StoreError::corrupt(what, e))
}

pub(crate) fn parse_opt_id<T: std::str::FromStr<Err = notai_core::CoreError>>(
    what: &'static str,
    s: Option<&str>,
) -> Result<Option<T>> {
    s.map(|s| parse_id(what, s)).transpose()
}

pub(crate) fn from_json<T: DeserializeOwned>(what: &'static str, s: &str) -> Result<T> {
    serde_json::from_str(s).map_err(|e| StoreError::corrupt(what, e))
}

pub(crate) fn bool_from(i: i64) -> bool {
    i != 0
}

pub(crate) struct TopicRow {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub due_date: Option<String>,
    pub icon: Option<String>,
    pub avatar: Option<String>,
    pub pinned: i64,
}

impl TryFrom<TopicRow> for Topic {
    type Error = StoreError;
    fn try_from(r: TopicRow) -> Result<Self> {
        let due = r
            .due_date
            .as_deref()
            .map(|d| parse_date("konu", d))
            .transpose()?;
        Ok(Self {
            id: parse_id("konu", &r.id)?,
            name: TopicName::new(&r.name)?,
            kind: ParaKind::from_parts(&r.kind, due)?,
            icon: r.icon,
            avatar: r.avatar,
            pinned: bool_from(r.pinned),
        })
    }
}

pub(crate) struct EntryRow {
    pub id: String,
    pub text: String,
    pub source: String,
    pub captured_at: String,
    pub topic_id: Option<String>,
    pub status: String,
    pub trashed_at: Option<String>,
}

impl TryFrom<EntryRow> for Entry {
    type Error = StoreError;
    fn try_from(r: EntryRow) -> Result<Self> {
        let trashed_at = parse_opt_ts("kayıt", r.trashed_at.as_deref())?;
        Ok(Self::from_parts(
            parse_id("kayıt", &r.id)?,
            EntryText::new(&r.text)?,
            Source::parse(&r.source)?,
            parse_ts("kayıt", &r.captured_at)?,
            parse_opt_id("kayıt", r.topic_id.as_deref())?,
            EntryStatus::from_parts(&r.status, trashed_at)?,
        ))
    }
}

pub(crate) struct SuggestionRow {
    pub id: String,
    pub entry_id: String,
    pub payload: String,
    pub confidence: f64,
    pub decision: String,
    pub decided_at: Option<String>,
}

impl TryFrom<SuggestionRow> for Suggestion {
    type Error = StoreError;
    fn try_from(r: SuggestionRow) -> Result<Self> {
        let kind: SuggestionKind = from_json("öneri", &r.payload)?;
        let decided_at = parse_opt_ts("öneri", r.decided_at.as_deref())?;
        Ok(Self::from_parts(
            parse_id("öneri", &r.id)?,
            parse_id("öneri", &r.entry_id)?,
            kind,
            Confidence::new(r.confidence)?,
            Decision::from_parts(&r.decision, decided_at)?,
        ))
    }
}

pub(crate) struct TaskRow {
    pub id: String,
    pub entry_id: Option<String>,
    pub title: String,
    pub topic_id: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub priority: String,
    pub recurrence: Option<String>,
    pub reminder: Option<String>,
    pub state: String,
    pub closed_at: Option<String>,
}

impl TryFrom<TaskRow> for Task {
    type Error = StoreError;
    fn try_from(r: TaskRow) -> Result<Self> {
        let due = match (r.due_date.as_deref(), r.due_time.as_deref()) {
            (Some(d), t) => Some(Due {
                date: parse_date("görev", d)?,
                time: t.map(|t| parse_time("görev", t)).transpose()?,
            }),
            (None, None) => None,
            (None, Some(_)) => return Err(StoreError::corrupt("görev", "saat var, tarih yok")),
        };
        let recurrence: Option<Recurrence> = r
            .recurrence
            .as_deref()
            .map(|s| from_json("görev", s))
            .transpose()?;
        let reminder: Option<Reminder> = r
            .reminder
            .as_deref()
            .map(|s| from_json("görev", s))
            .transpose()?;
        let closed_at = parse_opt_ts("görev", r.closed_at.as_deref())?;
        Ok(Self::from_parts(
            parse_id("görev", &r.id)?,
            parse_opt_id("görev", r.entry_id.as_deref())?,
            TaskTitle::new(&r.title)?,
            parse_opt_id("görev", r.topic_id.as_deref())?,
            due,
            Priority::parse(&r.priority)?,
            recurrence,
            reminder,
            TaskState::from_parts(&r.state, closed_at)?,
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn timestamps_sort_lexically() {
        let a = ts(Utc.with_ymd_and_hms(2026, 11, 6, 9, 5, 0).unwrap());
        let b = ts(Utc.with_ymd_and_hms(2026, 11, 6, 10, 0, 0).unwrap());
        assert!(a < b);
        assert_eq!(a, "2026-11-06T09:05:00.000000000Z");
        assert_eq!(
            parse_ts("t", &a).unwrap(),
            Utc.with_ymd_and_hms(2026, 11, 6, 9, 5, 0).unwrap()
        );
    }

    #[test]
    fn corrupt_values_are_reported() {
        assert!(matches!(
            parse_ts("t", "dün"),
            Err(StoreError::Corrupt { .. })
        ));
        assert!(parse_date("t", "2026-13-01").is_err());
        assert!(parse_time("t", "25:00").is_err());
    }
}
