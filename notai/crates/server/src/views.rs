//! API'nin döndürdüğü görünüm tipleri. Türkçe etiketler (çip metni, saat, gün ayracı)
//! sunucuda hazırlanır; arayüz yalnız gösterir.

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;
use notai_core::journal::{DayGroup, TopicActivity, group_by_day, list_time_label};
use notai_core::morning::MorningPlan;
use notai_core::suggestion::Suggestion;
use notai_core::task::Task;
use notai_core::{EntryId, SuggestionId, TopicId};
use notai_store::{JournalItem, TopicRef};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SuggestionView {
    pub id: SuggestionId,
    pub kind: &'static str,
    pub chip: String,
    pub decision: &'static str,
    pub confidence: f64,
}

impl SuggestionView {
    #[must_use]
    pub fn new(s: &Suggestion, tz: Tz) -> Self {
        Self {
            id: s.id,
            kind: s.kind.name(),
            chip: s.kind.chip_text(tz),
            decision: s.decision().name(),
            confidence: s.confidence.get(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct EntryView {
    pub id: EntryId,
    pub text: String,
    /// Yerel saat ("09:12").
    pub time: String,
    pub captured_at: DateTime<Utc>,
    pub source: &'static str,
    pub status: &'static str,
    pub topic: Option<TopicRef>,
    pub unprocessed: bool,
    pub ai_failed: bool,
    pub suggestions: Vec<SuggestionView>,
    pub tags: Vec<String>,
    pub people: Vec<String>,
}

impl EntryView {
    #[must_use]
    pub fn new(item: JournalItem, tz: Tz) -> Self {
        let e = &item.entry;
        Self {
            id: e.id,
            text: e.text.as_str().to_owned(),
            time: e.captured_at.with_timezone(&tz).format("%H:%M").to_string(),
            captured_at: e.captured_at,
            source: e.source.as_str(),
            status: e.status().name(),
            topic: item.topic,
            unprocessed: e.is_unprocessed(),
            ai_failed: item.ai_failed,
            suggestions: item
                .suggestions
                .iter()
                .map(|s| SuggestionView::new(s, tz))
                .collect(),
            tags: item.tags,
            people: item.people,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct JournalView {
    pub groups: Vec<DayGroup<EntryView>>,
    pub unprocessed: u64,
    /// Daha eskileri yüklemek için `before` imleci (yoksa başa ulaşıldı).
    pub next_before: Option<DateTime<Utc>>,
}

impl JournalView {
    #[must_use]
    pub fn new(
        items: Vec<JournalItem>,
        limit: u32,
        unprocessed: u64,
        now: DateTime<Utc>,
        tz: Tz,
    ) -> Self {
        let full_page = items.len() >= usize::try_from(limit).unwrap_or(usize::MAX);
        let next_before = if full_page {
            items.first().map(|i| i.entry.captured_at)
        } else {
            None
        };
        let views: Vec<EntryView> = items.into_iter().map(|i| EntryView::new(i, tz)).collect();
        Self {
            groups: group_by_day(views, |v| v.captured_at, now, tz),
            unprocessed,
            next_before,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TopicRowView {
    pub id: TopicId,
    pub name: String,
    pub kind: &'static str,
    pub kind_label: &'static str,
    pub due_date: Option<NaiveDate>,
    pub icon: Option<String>,
    pub pinned: bool,
    pub last_label: Option<String>,
    pub preview: Option<String>,
}

impl TopicRowView {
    #[must_use]
    pub fn new(a: TopicActivity, now: DateTime<Utc>, tz: Tz) -> Self {
        Self {
            id: a.topic.id,
            name: a.topic.name.as_str().to_owned(),
            kind: a.topic.kind.name(),
            kind_label: a.topic.kind.label(),
            due_date: a.topic.kind.due(),
            icon: a.topic.icon,
            pinned: a.topic.pinned,
            last_label: a.last_at.map(|t| list_time_label(t, now, tz)),
            preview: a.last_preview,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TaskRowView {
    #[serde(flatten)]
    pub task: Task,
    pub overdue_days: i64,
    pub priority_label: &'static str,
}

impl TaskRowView {
    #[must_use]
    pub fn new(task: Task, today: NaiveDate) -> Self {
        Self {
            overdue_days: task.days_overdue(today),
            priority_label: task.priority.label(),
            task,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MorningPlanView {
    pub headline: String,
    #[serde(flatten)]
    pub plan: MorningPlan,
    pub reasons: Vec<String>,
}

impl MorningPlanView {
    #[must_use]
    pub fn new(plan: MorningPlan, user: &str) -> Self {
        Self {
            headline: plan.headline(user),
            reasons: plan.items.iter().map(|i| i.reason.label()).collect(),
            plan,
        }
    }
}
