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
        answer: s
            .question_and_answer()
            .and_then(|(_, a)| a.map(|a| a.as_str().to_owned())),
    }
}

/// Son bildirim → görünüm.
#[must_use]
pub fn notice(n: &notai_core::notice::Notice) -> crate::NoticeView {
    crate::NoticeView {
        kind: match n.kind {
            notai_core::notice::NoticeKind::Reminder => "reminder",
            notai_core::notice::NoticeKind::Question => "question",
            notai_core::notice::NoticeKind::Watch => "watch",
        }
        .to_owned(),
        title: n.title.clone(),
        body: n.body.clone(),
        topic_id: n.topic.map(|t| t.to_string()),
        topic_name: n.topic_name.clone(),
        entry_id: n.entry.map(|e| e.to_string()),
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
        from_notai: e.from_notai,
    }
}

/// Takip ayarı → form görünümü.
#[must_use]
pub fn watch_view(w: &notai_core::watch::WatchConfig) -> crate::WatchView {
    crate::WatchView {
        query: w.query.clone(),
        urls: w.urls.clone(),
        target_price: w.target_price.map_or_else(String::new, |p| {
            p.format_abs().trim_end_matches("\u{a0}₺").to_owned()
        }),
        every_hours: w.every_hours,
        active: w.active,
    }
}

/// Form isteği → takip ayarı (`query` boşsa takip kaldırılır: `None`).
pub fn watch_config(
    r: &crate::WatchRequest,
) -> Result<Option<notai_core::watch::WatchConfig>, String> {
    if r.query.trim().is_empty() {
        return Ok(None);
    }
    let target = if r.target_price.trim().is_empty() {
        None
    } else {
        Some(notai_core::finance::Money::parse(&r.target_price).map_err(|e| e.to_string())?)
    };
    notai_core::watch::WatchConfig::new(
        &r.query,
        r.urls.clone(),
        target,
        r.every_hours
            .unwrap_or(notai_core::watch::DEFAULT_EVERY_HOURS),
        r.active.unwrap_or(true),
    )
    .map(Some)
    .map_err(|e| e.to_string())
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
        icon: Some(notai_core::look::icon_for(&a.topic, &a.recent_texts)),
        color: a.color,
        avatar: a.topic.avatar.clone(),
        pending: a.pending,
        parent_id: a.topic.parent.map(|p| p.to_string()),
        path: a.path.clone(),
        children: a.children,
        watch: a.topic.watch.as_ref().map(watch_view),
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

// ---------- v0.2–v0.4 ----------

use notai_core::agenda::{AgendaDay, WeeklyReview};
use notai_core::reminders::{ReminderGroup, ReminderItem};

use crate::{
    CalendarDayView, CalendarView, ReminderGroupView, ReminderView, RemindersView, SourceView,
    StalledView, TrashItemView, WeeklyReviewView,
};

/// "11:30" (bugün), "Yarın 09:00", "Paz 8 Kas 19:00".
#[must_use]
pub fn when_label(at: DateTime<Utc>, now: DateTime<Utc>, tz: Tz) -> String {
    let local = at.with_timezone(&tz);
    let today = now.with_timezone(&tz).date_naive();
    let date = local.date_naive();
    let time = local.format("%H:%M");
    match (date - today).num_days() {
        0 => time.to_string(),
        1 => format!("Yarın {time}"),
        -1 => format!("Dün {time}"),
        _ => {
            let wd: String = notai_core::tr::weekday_name(chrono::Datelike::weekday(&date))
                .chars()
                .take(3)
                .collect();
            format!(
                "{wd} {} {} {time}",
                chrono::Datelike::day(&date),
                notai_core::tr::month_short(chrono::Datelike::month(&date))
            )
        }
    }
}

fn group_key(g: ReminderGroup) -> &'static str {
    match g {
        ReminderGroup::Missed => "missed",
        ReminderGroup::Today => "today",
        ReminderGroup::Tomorrow => "tomorrow",
        ReminderGroup::ThisWeek => "this_week",
        ReminderGroup::Later => "later",
        ReminderGroup::Recurring => "recurring",
    }
}

#[must_use]
pub fn reminders(
    groups: Vec<(ReminderGroup, Vec<ReminderItem>)>,
    now: DateTime<Utc>,
    tz: Tz,
) -> RemindersView {
    RemindersView {
        groups: groups
            .into_iter()
            .map(|(g, items)| ReminderGroupView {
                key: group_key(g).to_owned(),
                label: g.label().to_owned(),
                items: items
                    .into_iter()
                    .map(|i| ReminderView {
                        task_id: i.task.to_string(),
                        title: i.title,
                        topic_id: i.topic.map(|t| t.to_string()),
                        when: when_label(i.fire_at, now, tz),
                        fire_at: i.fire_at,
                        recurring: i.recurring,
                    })
                    .collect(),
            })
            .collect(),
    }
}

#[must_use]
pub fn calendar(days: Vec<AgendaDay>, tasks: &[Task], today: NaiveDate) -> CalendarView {
    CalendarView {
        days: days
            .into_iter()
            .map(|d| CalendarDayView {
                date: d.date,
                label: d.label,
                is_today: d.is_today,
                items: d
                    .tasks
                    .iter()
                    .filter_map(|id| tasks.iter().find(|t| t.id == *id))
                    .map(|t| task_row(t, today))
                    .collect(),
            })
            .collect(),
    }
}

#[must_use]
pub fn weekly_review(r: WeeklyReview, tasks: &[Task], today: NaiveDate) -> WeeklyReviewView {
    WeeklyReviewView {
        week_start: r.week_start,
        unprocessed: r.unprocessed,
        stalled: r
            .stalled
            .into_iter()
            .map(|s| StalledView {
                topic_id: s.topic.to_string(),
                name: s.name,
                idle: s.idle_days.map_or_else(
                    || "Hiç hareket yok".to_owned(),
                    |d| format!("{d} gündür hareket yok"),
                ),
            })
            .collect(),
        upcoming: r
            .upcoming
            .iter()
            .filter_map(|id| tasks.iter().find(|t| t.id == *id))
            .map(|t| task_row(t, today))
            .collect(),
        overdue: r.overdue,
        done_last_week: r.done_last_week,
    }
}

#[must_use]
pub fn source(
    id: String,
    text: &str,
    at: DateTime<Utc>,
    topic: Option<String>,
    tz: Tz,
) -> SourceView {
    let local = at.with_timezone(&tz);
    SourceView {
        id,
        text: text.to_owned(),
        when: format!(
            "{} {}",
            notai_core::tr::date_long(
                local.date_naive(),
                chrono::Datelike::year(&local.date_naive())
            ),
            local.format("%H:%M")
        ),
        topic,
    }
}

#[must_use]
pub fn trash_item(e: &notai_core::Entry, now: DateTime<Utc>) -> TrashItemView {
    let ago = e.status().trashed_at().map_or(0, |t| (now - t).num_days());
    let left = (notai_core::entry::TRASH_RETENTION_DAYS - ago).max(0);
    TrashItemView {
        id: e.id.to_string(),
        text: e.text.preview(120),
        note: format!("{ago} gün önce silindi · {left} gün sonra kalıcı silinir"),
    }
}

#[must_use]
pub fn fact(f: &notai_core::profile::ProfileFact) -> crate::FactView {
    crate::FactView {
        id: f.id.to_string(),
        text: f.text.as_str().to_owned(),
        confirmed: f.status == notai_core::profile::FactStatus::Confirmed,
    }
}

// ---------- Düzenleme istekleri (sunucu ve uygulama kabuğu ortak) ----------

use chrono::NaiveTime;
use notai_core::TopicId;
use notai_core::task::{Due, MonthDay, Priority, Recurrence, Reminder};
use notai_core::text::{TaskTitle, TopicName};
use notai_core::topic::ParaKind;

use crate::{TaskEditRequest, TopicEditRequest};

/// Düzenleme isteğini göreve uygular (sunucu ve uygulama kabuğu ortak kullanır).
pub fn apply_task_edit(task: &mut notai_core::Task, e: &TaskEditRequest) -> Result<(), String> {
    if let Some(t) = &e.title {
        task.title = TaskTitle::new(t).map_err(|x| x.to_string())?;
    }
    if let Some(p) = &e.priority {
        task.priority = Priority::parse(p).map_err(|x| x.to_string())?;
    }
    if let Some(d) = &e.due_date {
        task.due = if d.is_empty() {
            None
        } else {
            let date = NaiveDate::parse_from_str(d, "%Y-%m-%d")
                .map_err(|_| format!("geçersiz tarih: {d}"))?;
            Some(Due {
                date,
                time: task.due.and_then(|x| x.time),
            })
        };
    }
    if let Some(t) = &e.due_time {
        let time = if t.is_empty() {
            None
        } else {
            Some(NaiveTime::parse_from_str(t, "%H:%M").map_err(|_| format!("geçersiz saat: {t}"))?)
        };
        match (&mut task.due, time) {
            (Some(d), time) => d.time = time,
            (None, None) => {}
            (None, Some(_)) => return Err("saat için tarih gerekli".into()),
        }
    }
    if let Some(r) = &e.repeat {
        let on = task.due.map(|d| chrono::Datelike::weekday(&d.date));
        task.recurrence = match (r.as_str(), on) {
            ("none" | "", _) => None,
            ("day", _) => Some(Recurrence::Day),
            ("weekday", _) => Some(Recurrence::Weekday),
            ("week", Some(on)) => Some(Recurrence::Week { on }),
            ("month", Some(_)) => {
                let day = task.due.map_or(1, |d| {
                    u8::try_from(chrono::Datelike::day(&d.date))
                        .unwrap_or(1)
                        .min(28)
                });
                Some(Recurrence::Month {
                    day: MonthDay::new(day).map_err(|x| x.to_string())?,
                })
            }
            (_, None) => return Err("tekrar için tarih gerekli".into()),
            (other, _) => return Err(format!("bilinmeyen tekrar: {other}")),
        };
    }
    if let Some(m) = e.remind_before_minutes {
        task.reminder = u32::try_from(m)
            .ok()
            .map(|minutes| Reminder::Before { minutes });
    }
    if let Some(t) = &e.topic_id {
        task.topic = if t.is_empty() {
            None
        } else {
            Some(t.parse::<TopicId>().map_err(|x| x.to_string())?)
        };
    }
    // Tutarlılık: tarihsiz göreve tekrar ya da "önce" hatırlatması olamaz.
    notai_core::Task::from_parts(
        task.id,
        task.entry,
        task.title.clone(),
        task.topic,
        task.due,
        task.priority,
        task.recurrence,
        task.reminder,
        task.state(),
    )
    .map(|_| ())
    .map_err(|x| x.to_string())
}

/// Konu düzenleme isteğini uygular.
pub fn apply_topic_edit(topic: &mut notai_core::Topic, e: &TopicEditRequest) -> Result<(), String> {
    if let Some(n) = &e.name {
        topic.name = TopicName::new(n).map_err(|x| x.to_string())?;
    }
    if e.kind.is_some() || e.due_date.is_some() {
        let kind = e
            .kind
            .clone()
            .unwrap_or_else(|| topic.kind.name().to_owned());
        let due = match &e.due_date {
            Some(d) if d.is_empty() => None,
            Some(d) => Some(
                NaiveDate::parse_from_str(d, "%Y-%m-%d")
                    .map_err(|_| format!("geçersiz tarih: {d}"))?,
            ),
            None => topic.kind.due(),
        };
        topic.kind = ParaKind::from_parts(&kind, due).map_err(|x| x.to_string())?;
    }
    if let Some(i) = &e.icon {
        topic.icon = (!i.is_empty()).then(|| i.clone());
    }
    if let Some(p) = &e.parent_id {
        topic.parent = if p.is_empty() {
            None
        } else {
            Some(p.parse::<TopicId>().map_err(|x| x.to_string())?)
        };
    }
    if let Some(av) = &e.avatar {
        topic.avatar = if av.is_empty() {
            None
        } else {
            notai_core::Topic::check_avatar(av).map_err(|x| x.to_string())?;
            Some(av.clone())
        };
    }
    if let Some(p) = e.pinned {
        topic.pinned = p;
    }
    Ok(())
}

// ---------- Harcamalar ----------

/// Aylık harcama görünümü.
#[must_use]
pub fn finance_view(
    all: &[notai_core::finance::Transaction],
    year: i32,
    month: u32,
    pending_statements: u32,
) -> crate::FinanceView {
    use notai_core::finance::{CATEGORIES, Money, category_label, month_report};
    let moves: Vec<_> = all
        .iter()
        .map(notai_core::finance::Transaction::movement)
        .collect();
    let r = month_report(&moves, year, month);
    let total = r.expense.0.max(1);
    let share = |key: String, label: String, amount: Money| crate::ShareView {
        key,
        label,
        amount: amount.format_abs(),
        pct: amount.0 * 100 / total,
    };
    let net = Money(r.income.0 - r.expense.0);
    let mut txs: Vec<&notai_core::finance::Transaction> = all
        .iter()
        .filter(|t| {
            chrono::Datelike::year(&t.date) == year && chrono::Datelike::month(&t.date) == month
        })
        .collect();
    txs.sort_by(|a, b| b.date.cmp(&a.date).then(a.description.cmp(&b.description)));
    crate::FinanceView {
        month: format!("{year}-{month:02}"),
        title: format!("{} {year}", notai_core::tr::month_name(month)),
        income: r.income.format_abs(),
        expense: r.expense.format_abs(),
        net: net.format_signed(),
        net_negative: net.0 < 0,
        savings_pct: (r.income.0 > 0).then(|| net.0 * 100 / r.income.0),
        scopes: r
            .by_scope
            .iter()
            .map(|(s, m)| share(s.name().to_owned(), s.label().to_owned(), *m))
            .collect(),
        categories: r
            .by_category
            .iter()
            .map(|(k, m)| share(k.clone(), category_label(k).to_owned(), *m))
            .collect(),
        recurring: r
            .recurring
            .iter()
            .map(|(d, m)| share(d.clone(), d.clone(), *m))
            .collect(),
        insights: r.insights,
        transactions: txs
            .into_iter()
            .map(|t| crate::TransactionView {
                id: t.id.to_string(),
                date: t.date,
                date_label: format!(
                    "{} {}",
                    chrono::Datelike::day(&t.date),
                    notai_core::tr::month_short(chrono::Datelike::month(&t.date))
                ),
                description: t.description.clone(),
                amount: t.amount.format_signed(),
                expense: t.amount.0 < 0,
                category: t.category.clone(),
                category_label: category_label(&t.category).to_owned(),
                scope: t.scope.name().to_owned(),
                scope_label: t.scope.label().to_owned(),
                source: t.source.name().to_owned(),
                matched: t.matched_to.is_some(),
            })
            .collect(),
        pending_statements,
        category_options: CATEGORIES
            .iter()
            .map(|(k, l)| ((*k).to_owned(), (*l).to_owned()))
            .collect(),
    }
}

/// Ekstre yükleme sonucunun Türkçe özeti.
#[must_use]
pub fn import_view(r: &notai_store::ImportReport) -> crate::ImportView {
    let pending = r.parsed == 0;
    let message = if pending {
        "Ekstre biçimi tanınmadı; AI'a (ajana) okutulmak üzere bekliyor.".to_owned()
    } else {
        let mut m = format!("{} hareket eklendi", r.imported);
        if r.skipped > 0 {
            m.push_str(&format!(", {} zaten vardı (mükerrer eklenmedi)", r.skipped));
        }
        if r.matched > 0 {
            m.push_str(&format!(", {} elle girilen ödemeyle eşleşti", r.matched));
        }
        m.push('.');
        m
    };
    crate::ImportView {
        parsed: r.parsed,
        imported: r.imported,
        skipped: r.skipped,
        matched: r.matched,
        pending,
        message,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn dated_task() -> notai_core::Task {
        let mut n = notai_core::task::NewTask::titled(TaskTitle::new("Kombi servisi").unwrap());
        n.due = Some(Due {
            date: NaiveDate::from_ymd_opt(2026, 11, 9).unwrap(),
            time: NaiveTime::from_hms_opt(10, 0, 0),
        });
        notai_core::Task::create(n).unwrap()
    }

    fn remind(m: i32) -> TaskEditRequest {
        TaskEditRequest {
            remind_before_minutes: Some(m),
            ..TaskEditRequest::default()
        }
    }

    #[test]
    fn reminder_on_time_before_and_off() {
        let mut t = dated_task();
        apply_task_edit(&mut t, &remind(0)).unwrap();
        assert_eq!(
            t.reminder,
            Some(Reminder::Before { minutes: 0 }),
            "tam zamanında"
        );
        apply_task_edit(&mut t, &remind(15)).unwrap();
        assert_eq!(t.reminder, Some(Reminder::Before { minutes: 15 }));
        apply_task_edit(&mut t, &remind(-1)).unwrap();
        assert_eq!(t.reminder, None, "negatif = kaldır");
        // Belirtilmezse dokunulmaz.
        apply_task_edit(&mut t, &remind(30)).unwrap();
        apply_task_edit(&mut t, &TaskEditRequest::default()).unwrap();
        assert_eq!(t.reminder, Some(Reminder::Before { minutes: 30 }));
    }

    #[test]
    fn on_time_reminder_needs_a_date() {
        let mut t = notai_core::Task::create(notai_core::task::NewTask::titled(
            TaskTitle::new("Ara").unwrap(),
        ))
        .unwrap();
        assert!(apply_task_edit(&mut t, &remind(0)).is_err());
    }
}
