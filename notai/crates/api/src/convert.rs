//! Alan/depo tiplerinden API görünümleri (sunucu ve Tauri kabuğu ortak kullanır).

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;
use notai_core::journal::{TopicActivity, group_by_day, list_time_label};
use notai_core::morning::{MorningPlan, PlanReason};
use notai_core::suggestion::Suggestion;
use notai_core::task::Task;
use notai_store::{DecisionOutcome, JournalItem};

use crate::{
    DayGroup, DecisionView, EntryView, JournalView, MorningPlanView, PlanItemView, SuggestionView,
    TaskRowView, TopicRef, TopicRowView,
};

#[must_use]
pub fn suggestion(s: &Suggestion, tz: Tz) -> SuggestionView {
    SuggestionView {
        id: s.id.to_string(),
        kind: s.kind.name().to_owned(),
        chip: s.kind.chip_text(tz),
        decision: s.decision().name().to_owned(),
        confidence: s.confidence.get(),
    }
}

#[must_use]
pub fn entry(item: JournalItem, tz: Tz) -> EntryView {
    let e = &item.entry;
    EntryView {
        id: e.id.to_string(),
        text: e.text.as_str().to_owned(),
        time: e.captured_at.with_timezone(&tz).format("%H:%M").to_string(),
        captured_at: e.captured_at,
        source: e.source.as_str().to_owned(),
        status: e.status().name().to_owned(),
        topic: item.topic.map(|t| TopicRef {
            id: t.id.to_string(),
            name: t.name.as_str().to_owned(),
        }),
        unprocessed: e.is_unprocessed(),
        ai_failed: item.ai_failed,
        suggestions: item.suggestions.iter().map(|s| suggestion(s, tz)).collect(),
        tags: item.tags,
        people: item.people,
    }
}

/// Akış görünümü. `limit` kadar kayıt geldiyse daha eskileri için imleç verilir.
#[must_use]
pub fn journal(
    items: Vec<JournalItem>,
    limit: u32,
    unprocessed: u64,
    now: DateTime<Utc>,
    tz: Tz,
) -> JournalView {
    let full_page = items.len() >= usize::try_from(limit).unwrap_or(usize::MAX);
    let next_before = if full_page {
        items.first().map(|i| i.entry.captured_at)
    } else {
        None
    };
    let views: Vec<EntryView> = items.into_iter().map(|i| entry(i, tz)).collect();
    let groups = group_by_day(views, |v| v.captured_at, now, tz)
        .into_iter()
        .map(|g| DayGroup {
            date: g.date,
            label: g.label,
            items: g.items,
        })
        .collect();
    JournalView {
        groups,
        unprocessed,
        next_before,
    }
}

#[must_use]
pub fn topic_row(a: TopicActivity, now: DateTime<Utc>, tz: Tz) -> TopicRowView {
    TopicRowView {
        id: a.topic.id.to_string(),
        name: a.topic.name.as_str().to_owned(),
        kind: a.topic.kind.name().to_owned(),
        kind_label: a.topic.kind.label().to_owned(),
        due_date: a.topic.kind.due(),
        icon: a.topic.icon,
        pinned: a.topic.pinned,
        last_label: a.last_at.map(|t| list_time_label(t, now, tz)),
        preview: a.last_preview,
    }
}

#[must_use]
pub fn task_row(t: &Task, today: NaiveDate) -> TaskRowView {
    TaskRowView {
        id: t.id.to_string(),
        title: t.title.as_str().to_owned(),
        topic_id: t.topic.map(|x| x.to_string()),
        due_date: t.due.map(|d| d.date),
        due_time: t.due.and_then(|d| d.time),
        priority: t.priority.name().to_owned(),
        priority_label: t.priority.label().to_owned(),
        overdue_days: t.days_overdue(today),
        open: t.is_open(),
        recurring: t.recurrence.is_some(),
    }
}

#[must_use]
pub fn morning_plan(plan: MorningPlan, user: &str) -> MorningPlanView {
    MorningPlanView {
        date: plan.date,
        headline: plan.headline(user),
        items: plan
            .items
            .iter()
            .map(|i| PlanItemView {
                task_id: i.task.to_string(),
                title: i.title.clone(),
                reason: i.reason.label(),
                reason_kind: match i.reason {
                    PlanReason::Overdue { .. } => "overdue",
                    PlanReason::Today => "today",
                    PlanReason::HighPriority => "high_priority",
                }
                .to_owned(),
                time: i.time,
            })
            .collect(),
        more: plan.more,
    }
}

#[must_use]
pub fn decision(out: &DecisionOutcome, today: NaiveDate, tz: Tz) -> DecisionView {
    DecisionView {
        suggestion: suggestion(&out.suggestion, tz),
        entry_status: out.entry.status().name().to_owned(),
        entry_topic: out.entry.topic().map(|t| t.to_string()),
        created_task: out.created_task.as_ref().map(|t| task_row(t, today)),
        created_topic: out.created_topic.as_ref().map(|t| TopicRef {
            id: t.id.to_string(),
            name: t.name.as_str().to_owned(),
        }),
    }
}
