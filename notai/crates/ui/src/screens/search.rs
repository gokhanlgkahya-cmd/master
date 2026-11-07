//! Ara/Sor: notlara soru sor; cevap + kaynak kayıtlar (11-ara-sor.html).

use leptos::prelude::*;
use leptos::task::spawn_local;
use notai_api::AskView;

use crate::{ErrorNote, Icon, TabBar, backend, view, web};

#[derive(Clone)]
enum State {
    Idle,
    Asking,
    Done(AskView),
    Failed(String),
}

#[component]
pub fn Search() -> impl IntoView {
    let b = backend();
    let q = RwSignal::new(String::new());
    let state = RwSignal::new(State::Idle);
    let input = NodeRef::<leptos::html::Input>::new();
    web::focus_later(input);
    let ask = move || {
        let question = q.get_untracked().trim().to_owned();
        if question.is_empty() {
            return;
        }
        state.set(State::Asking);
        let b = b.clone();
        spawn_local(async move {
            state.set(match b.ask(question).await {
                Ok(v) => State::Done(v),
                Err(e) => State::Failed(e),
            });
        });
    };
    view! {
        <header class="topbar">
            <div class="topbar__text">
                <h1 class="topbar__title">"Ara / Sor"</h1>
                <p class="topbar__sub">"Notlarına soru sor veya ara"</p>
            </div>
        </header>
        <form class="askbar" on:submit=move |ev| { ev.prevent_default(); ask() }>
            <label class="field field--input">
                <Icon name="search"/>
                <input type="search" lang="tr" enterkeyhint="search" node_ref=input
                    aria-label="Soru ya da arama"
                    placeholder="Kostik için en son hangi fiyatı almıştım?"
                    prop:value=move || q.get()
                    on:input=move |ev| q.set(event_target_value(&ev))/>
                {move || (!q.get().is_empty()).then(|| view! {
                    <button class="iconbtn" type="button" aria-label="Temizle"
                        on:click=move |_| { q.set(String::new()); state.set(State::Idle); web::focus_later(input); }>
                        <Icon name="x" size="sm"/>
                    </button>
                })}
            </label>
            <button class="iconbtn" type="submit" aria-label="Sor">
                <Icon name="send"/>
            </button>
        </form>
        <div class="body scroll">
            {move || match state.get() {
                State::Idle => view! {
                    <p class="note">"Örnek: “Kombi bakımı ne zaman?”, “Ayhan Kimya teklifi”. Cevap yalnız kendi notlarından gelir."</p>
                }.into_any(),
                State::Asking => view! { <p class="loading">"Notların taranıyor…"</p> }.into_any(),
                State::Failed(e) => view! { <p class="error-note" role="alert">{e}</p> }.into_any(),
                State::Done(v) => answer(v).into_any(),
            }}
        </div>
        <ErrorNote/>
        <TabBar/>
    }
}

fn answer(v: AskView) -> impl IntoView {
    let n = v.sources.len();
    let eyebrow = view::answer_eyebrow(v.answer.is_some(), n);
    let text = match (&v.answer, v.not_found || n == 0) {
        (Some(a), _) => Some(a.clone()),
        (None, true) => Some("Notlarında bununla ilgili bir kayıt bulunamadı.".to_owned()),
        (None, false) => None,
    };
    let chips = v
        .sources
        .iter()
        .map(|s| {
            let label = format!("{} · {}", view::first_line(&s.text), s.when);
            view! { <span class="src"><Icon name="note" size="xs"/><span>{label}</span></span> }
        })
        .collect_view();
    let results = v
        .sources
        .into_iter()
        .map(|s| {
            let topic = s.topic.clone().map(|t| {
                view! { <Icon name="folder" size="xs"/><span>{t}</span><span>"·"</span> }
            });
            view! {
                <div class="result">
                    <span class="result__title">{view::first_line(&s.text)}</span>
                    <span class="result__snip">{s.text.clone()}</span>
                    <span class="result__meta">{topic}<span>{s.when.clone()}</span></span>
                </div>
            }
        })
        .collect_view();
    let has_sources = n > 0;
    view! {
        <section class="answer" aria-label="Cevap" aria-live="polite">
            <div class="answer__eyebrow"><Icon name="sparkle" size="xs"/>{eyebrow}</div>
            {text.map(|t| view! { <p class="answer__text">{t}</p> })}
            {has_sources.then(|| view! { <div class="sources">{chips}</div> })}
        </section>
        {has_sources.then(|| view! {
            <h2 class="grouphead">"Kayıtlar"<span class="count">{n}</span></h2>
            <div class="card">{results}</div>
        })}
    }
}
