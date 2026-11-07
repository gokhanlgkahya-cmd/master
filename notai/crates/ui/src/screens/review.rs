//! Haftalık gözden geçirme: işlenmemişler, ilerlemeyen projeler, önümüzdeki 7 gün,
//! gecikenler, geçen hafta bitenler (12-haftalik-gozden-gecirme.html).

use leptos::prelude::*;
use notai_api::{TopicRef, WeeklyReviewView};

use crate::{ErrorNote, Icon, Screen, backend, ctx, failed, loading, view};

#[component]
pub fn Review() -> impl IntoView {
    let c = ctx();
    let data = LocalResource::new(move || {
        let _ = c.version.get();
        let b = backend();
        async move { b.weekly_review().await }
    });
    let sub = move || match data.get() {
        Some(Ok(r)) => view::week_range(r.week_start),
        _ => String::new(),
    };
    view! {
        <header class="flowbar">
            <button class="iconbtn" aria-label="Kapat" on:click=move |_| c.screen.set(Screen::Home)><Icon name="x"/></button>
            <div class="flowbar__who">
                <div class="flowbar__text">
                    <span class="flowbar__name">"Haftalık gözden geçirme"</span>
                    <span class="flowbar__sub">{sub}</span>
                </div>
            </div>
        </header>
        <div class="body scroll">
            {move || match data.get() {
                None => loading(),
                Some(Err(e)) => failed(e),
                Some(Ok(r)) => steps(r).into_any(),
            }}
        </div>
        <ErrorNote/>
    }
}

fn steps(r: WeeklyReviewView) -> impl IntoView {
    let c = ctx();
    let stalled_n = r.stalled.len();
    let stalled = r
        .stalled
        .into_iter()
        .map(|s| {
            let target = TopicRef {
                id: s.topic_id.clone(),
                name: s.name.clone(),
            };
            view! {
                <button class="urow" on:click=move |_| c.screen.set(Screen::Flow(Some(target.clone())))>
                    <span class="urow__txt">
                        <span class="urow__title">{s.name}</span>
                        <span class="urow__meta">{s.idle}</span>
                    </span>
                    <Icon name="chevr" size="sm"/>
                </button>
            }
        })
        .collect_view();
    let upcoming_n = r.upcoming.len();
    let upcoming = r
        .upcoming
        .into_iter()
        .map(|t| {
            let meta = view::task_meta(&t);
            let open = t.clone();
            view! {
                <button class="urow" on:click=move |_| c.editing.set(Some(open.clone()))>
                    <span class="urow__txt">
                        <span class="urow__title">{t.title}</span>
                        <span class="urow__meta">{meta}</span>
                    </span>
                    <span class="tag">{t.priority_label}</span>
                </button>
            }
        })
        .collect_view();
    let has_unprocessed = r.unprocessed > 0;
    let has_overdue = r.overdue > 0;
    view! {
        <section class="stepcard">
            <h2 class="stepcard__title">"İşlenmemiş kayıtlar"</h2>
            <p class="stepcard__lead">{view::unprocessed_lead(r.unprocessed)}</p>
            {has_unprocessed.then(|| view! {
                <div class="stepcard__act">
                    <button class="btn btn--outline" on:click=move |_| c.screen.set(Screen::Flow(None))>
                        <Icon name="notebook" size="sm"/>"Ana Günlük’e git"
                    </button>
                </div>
            })}
        </section>
        <section class="stepcard">
            <h2 class="stepcard__title">"İlerlemeyen projeler"</h2>
            <p class="stepcard__lead">{if stalled_n == 0 { "Tüm projeler bu hafta hareket etti.".to_owned() } else { format!("{stalled_n}\u{a0}projede hareket yok.") }}</p>
            {stalled}
        </section>
        <section class="stepcard">
            <h2 class="stepcard__title">"Önümüzdeki 7 gün"</h2>
            <p class="stepcard__lead">{if upcoming_n == 0 { "Tarihli görev yok.".to_owned() } else { format!("{upcoming_n}\u{a0}görev planlı.") }}</p>
            {upcoming}
        </section>
        <section class="stepcard">
            <h2 class="stepcard__title">"Gecikenler"</h2>
            <p class="stepcard__lead">{view::overdue_lead(r.overdue)}</p>
            {has_overdue.then(|| view! {
                <div class="stepcard__act">
                    <button class="btn btn--outline" on:click=move |_| c.screen.set(Screen::Tasks)>
                        <Icon name="task" size="sm"/>"Görevlere git"
                    </button>
                </div>
            })}
        </section>
        <section class="stepcard">
            <h2 class="stepcard__title">"Geçen hafta"</h2>
            <p class="stepcard__lead">{view::done_lead(r.done_last_week)}</p>
        </section>
    }
}
