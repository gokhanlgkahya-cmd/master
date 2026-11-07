//! Takvim: Hafta şeridi ya da Ay ızgarası + seçili günün görevleri (09-takvim.html).

use chrono::{Datelike, NaiveDate};
use leptos::prelude::*;
use notai_api::{CalendarView, TaskRowView};

use crate::{ErrorNote, Icon, TabBar, backend, ctx, run, view, web};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Week,
    Month,
}

#[component]
pub fn Calendar() -> impl IntoView {
    let c = ctx();
    let today = web::today();
    let mode = RwSignal::new(Mode::Week);
    // Görünen dönemin başı: haftada pazartesi, ayda ayın ilk günü.
    let anchor = RwSignal::new(view::week_monday(today));
    let selected = RwSignal::new(today);
    let data = LocalResource::new(move || {
        let (from, days) = match mode.get() {
            Mode::Week => (anchor.get(), 7),
            Mode::Month => (view::month_grid(anchor.get())[0], 42),
        };
        let from = from.format("%Y-%m-%d").to_string();
        let _ = c.version.get();
        let b = backend();
        async move { b.calendar(Some(from), days).await }
    });
    let go = move |step: i32| match mode.get_untracked() {
        Mode::Week => {
            let m = view::shift_weeks(anchor.get_untracked(), step);
            anchor.set(m);
            selected.set(view::day_in_week(m, today));
        }
        Mode::Month => {
            let m = view::shift_months(anchor.get_untracked(), step);
            anchor.set(m);
            selected.set(view::day_in_month(m, today));
        }
    };
    let set_mode = move |m: Mode| {
        if mode.get_untracked() == m {
            return;
        }
        let d = selected.get_untracked();
        anchor.set(match m {
            Mode::Week => view::week_monday(d),
            Mode::Month => view::month_first(d),
        });
        mode.set(m);
    };
    let back_to_today = move |_| {
        anchor.set(match mode.get_untracked() {
            Mode::Week => view::week_monday(today),
            Mode::Month => view::month_first(today),
        });
        selected.set(today);
    };
    let count_on = move |d: NaiveDate| match data.get() {
        Some(Ok(v)) => day_items(&v, d).iter().filter(|t| t.open).count(),
        _ => 0,
    };
    let day_button = move |d: NaiveDate, outside: bool| {
        let class = match (d == today, outside) {
            (true, _) => "wday wday--today",
            (false, true) => "wday wday--out",
            (false, false) => "wday",
        };
        let dots = (0..count_on(d).min(3))
            .map(|_| view! { <i></i> })
            .collect_view();
        let label = format!(
            "{}{}",
            view::long_date(d),
            if d == today { " · Bugün" } else { "" }
        );
        let in_week = mode.get_untracked() == Mode::Week;
        view! {
            <button class=class role="tab" aria-label=label
                aria-selected=move || (selected.get() == d).to_string()
                on:click=move |_| selected.set(d)>
                {in_week.then(|| view! { <span class="wday__name">{view::weekday_short(d)}</span> })}
                <span class="wday__num">{d.format("%-d").to_string()}</span>
                <span class="wday__dots">{dots}</span>
            </button>
        }
    };
    let strip = move || {
        match mode.get() {
        Mode::Week => view! {
            <div class="weekstrip" role="tablist" aria-label="Hafta">
                {view::week_days(anchor.get()).into_iter().map(|d| day_button(d, false)).collect_view()}
            </div>
        }
        .into_any(),
        Mode::Month => {
            let month = anchor.get().month();
            let head = view::week_days(view::week_monday(today))
                .into_iter()
                .map(|d| view! { <span class="monthgrid__wd" aria-hidden="true">{view::weekday_short(d)}</span> })
                .collect_view();
            view! {
                <div class="monthgrid" role="tablist" aria-label="Ay">
                    {head}
                    {view::month_grid(anchor.get()).into_iter().map(|d| day_button(d, d.month() != month)).collect_view()}
                </div>
            }
            .into_any()
        }
    }
    };
    let day = move || {
        let d = selected.get();
        match data.get() {
            None => crate::loading(),
            Some(Err(e)) => crate::failed(e),
            Some(Ok(v)) => timeline(day_items(&v, d), d == today).into_any(),
        }
    };
    let (prev, next) = (
        move || match mode.get() {
            Mode::Week => "Önceki hafta",
            Mode::Month => "Önceki ay",
        },
        move || match mode.get() {
            Mode::Week => "Sonraki hafta",
            Mode::Month => "Sonraki ay",
        },
    );
    view! {
        <header class="topbar">
            <div class="topbar__text"><h1 class="topbar__title">"Takvim"</h1></div>
            <div class="topbar__actions">
                <button class="iconbtn" aria-label=prev on:click=move |_| go(-1)><Icon name="back"/></button>
                <button class="iconbtn" aria-label=next on:click=move |_| go(1)><Icon name="chevr"/></button>
                <button class="iconbtn" aria-label="Bugüne dön" on:click=back_to_today><Icon name="clock"/></button>
            </div>
        </header>
        <div class="weekhead">
            <span class="weekhead__title">{move || view::month_year(match mode.get() {
                Mode::Week => selected.get(),
                Mode::Month => anchor.get(),
            })}</span>
            <div class="switch2" role="tablist" aria-label="Görünüm">
                <button role="tab" aria-selected=move || (mode.get() == Mode::Week).to_string()
                    on:click=move |_| set_mode(Mode::Week)>"Hafta"</button>
                <button role="tab" aria-selected=move || (mode.get() == Mode::Month).to_string()
                    on:click=move |_| set_mode(Mode::Month)><Icon name="grid" size="sm"/>"Ay"</button>
            </div>
        </div>
        {strip}
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
