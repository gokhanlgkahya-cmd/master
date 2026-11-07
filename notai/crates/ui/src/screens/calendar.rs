//! Takvim: hafta şeridi + seçili günün görevleri (09-takvim.html).

use chrono::NaiveDate;
use leptos::prelude::*;
use notai_api::{CalendarView, TaskRowView};

use crate::{ErrorNote, Icon, TabBar, backend, ctx, run, view, web};

#[component]
pub fn Calendar() -> impl IntoView {
    let c = ctx();
    let today = web::today();
    let monday = RwSignal::new(view::week_monday(today));
    let selected = RwSignal::new(today);
    let data = LocalResource::new(move || {
        let from = monday.get().format("%Y-%m-%d").to_string();
        let _ = c.version.get();
        let b = backend();
        async move { b.calendar(Some(from), 7).await }
    });
    let go = move |weeks: i32| {
        let m = view::shift_weeks(monday.get_untracked(), weeks);
        monday.set(m);
        selected.set(view::day_in_week(m, today));
    };
    let back_to_today = move |_| {
        monday.set(view::week_monday(today));
        selected.set(today);
    };
    let count_on = move |d: NaiveDate| match data.get() {
        Some(Ok(v)) => day_items(&v, d).iter().filter(|t| t.open).count(),
        _ => 0,
    };
    let strip = move || {
        view::week_days(monday.get())
            .into_iter()
            .map(|d| {
                let class = if d == today {
                    "wday wday--today"
                } else {
                    "wday"
                };
                let dots = (0..count_on(d).min(3))
                    .map(|_| view! { <i></i> })
                    .collect_view();
                let label = format!(
                    "{}{}",
                    view::long_date(d),
                    if d == today { " · Bugün" } else { "" }
                );
                view! {
                    <button class=class role="tab" aria-label=label
                        aria-selected=move || (selected.get() == d).to_string()
                        on:click=move |_| selected.set(d)>
                        <span class="wday__name">{view::weekday_short(d)}</span>
                        <span class="wday__num">{d.format("%-d").to_string()}</span>
                        <span class="wday__dots">{dots}</span>
                    </button>
                }
            })
            .collect_view()
    };
    let day = move || {
        let d = selected.get();
        match data.get() {
            None => crate::loading(),
            Some(Err(e)) => crate::failed(e),
            Some(Ok(v)) => timeline(day_items(&v, d), d == today).into_any(),
        }
    };
    view! {
        <header class="topbar">
            <div class="topbar__text"><h1 class="topbar__title">"Takvim"</h1></div>
            <div class="topbar__actions">
                <button class="iconbtn" aria-label="Bugüne dön" on:click=back_to_today><Icon name="clock"/></button>
            </div>
        </header>
        <div class="weekhead">
            <span class="weekhead__title">{move || view::month_year(selected.get())}</span>
            <div class="topbar__actions">
                <button class="iconbtn" aria-label="Önceki hafta" on:click=move |_| go(-1)><Icon name="back"/></button>
                <button class="iconbtn" aria-label="Sonraki hafta" on:click=move |_| go(1)><Icon name="chevr"/></button>
            </div>
        </div>
        <div class="weekstrip" role="tablist" aria-label="Hafta">{strip}</div>
        <div class="body body--flush">
            <h2 class="daytitle">
                {move || view::long_date(selected.get())}
                <small>{move || (selected.get() == today).then_some("Bugün")}</small>
            </h2>
            <div class="timeline scroll">{day}</div>
        </div>
        <ErrorNote/>
        <TabBar/>
    }
}

fn day_items(v: &CalendarView, d: NaiveDate) -> Vec<TaskRowView> {
    v.days
        .iter()
        .find(|x| x.date == d)
        .map(|x| x.items.clone())
        .unwrap_or_default()
}

fn timeline(items: Vec<TaskRowView>, is_today: bool) -> impl IntoView {
    let c = ctx();
    if items.is_empty() {
        return view! { <p class="note">"Bu gün için görev yok."</p> }.into_any();
    }
    let times: Vec<_> = items.iter().map(|t| t.due_time).collect();
    let now = web::now_time();
    let now_at = is_today.then(|| view::nowline_index(&times, now));
    let mut rows: Vec<AnyView> = Vec::new();
    for (i, t) in items.into_iter().enumerate() {
        if now_at == Some(i) {
            rows.push(nowline(now));
        }
        let id = t.id.clone();
        let edit = t.clone();
        let done = !t.open;
        let sub = format!("Görev · {}", t.priority_label);
        let check = if done {
            view! {
                <span class="task__check" role="img" aria-label=format!("Tamamlandı: {}", t.title)>
                    <span class="check check--done"><Icon name="check" size="xs"/></span>
                </span>
            }
            .into_any()
        } else {
            view! {
                <button class="task__check" aria-label=format!("Tamamla: {}", t.title)
                    on:click=move |_| { let id = id.clone(); run(move |b| async move { b.complete_task(id).await }) }>
                    <span class="check"></span>
                </button>
            }
            .into_any()
        };
        rows.push(
            view! {
                <div class="tl">
                    <span class="tl__time">{view::timeline_time(&t)}</span>
                    <div class="tl__item" class:tl__item--done=done>
                        {check}
                        <button class="tl__txt" aria-label=format!("Düzenle: {}", t.title)
                            on:click=move |_| c.editing.set(Some(edit.clone()))>
                            <span class="tl__title">{t.title.clone()}</span>
                            <span class="tl__sub">{sub}</span>
                        </button>
                    </div>
                </div>
            }
            .into_any(),
        );
    }
    if now_at.is_some_and(|i| i >= times.len()) {
        rows.push(nowline(now));
    }
    rows.into_iter().collect_view().into_any()
}

fn nowline(now: chrono::NaiveTime) -> AnyView {
    view! { <div class="nowline" aria-label="Şimdi">{now.format("%H:%M").to_string()}</div> }
        .into_any()
}
