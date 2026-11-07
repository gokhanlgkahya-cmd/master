//! Ana ekran: Ana Günlük, haftalık gözden geçirme, konular (01-ana-ekran.html).

use leptos::prelude::*;
use notai_api::{TopicRef, TopicsView};

use crate::composer::Composer;
use crate::sheets::NewTopicSheet;
use crate::{ErrorNote, Icon, Screen, TabBar, backend, ctx, failed, loading, view};

#[component]
pub fn Home() -> impl IntoView {
    let c = ctx();
    let kind = RwSignal::new("all");
    let new_topic = RwSignal::new(false);
    let data = LocalResource::new(move || {
        let k = kind.get();
        let _ = c.version.get();
        let b = backend();
        async move { b.topics(k).await }
    });
    let seg = move |value: &'static str, label: &'static str| {
        view! {
            <button class="segmented__btn" role="tab"
                aria-selected=move || (kind.get() == value).to_string()
                on:click=move |_| kind.set(value)>{label}</button>
        }
    };
    view! {
        <header class="topbar">
            <h1 class="topbar__title">"Günlük"</h1>
            <div class="topbar__actions">
                <button class="iconbtn" aria-label="Ara" on:click=move |_| c.screen.set(Screen::Search)><Icon name="search"/></button>
                <button class="iconbtn" aria-label="Yeni konu" on:click=move |_| new_topic.set(true)><Icon name="plus"/></button>
                <button class="iconbtn" aria-label="Ayarlar" on:click=move |_| c.screen.set(Screen::Settings)><Icon name="dots"/></button>
            </div>
        </header>
        <div class="segmented" role="tablist" aria-label="Konu türü">
            {seg("all", "Tümü")}{seg("projects", "Proje")}{seg("areas", "Alan")}{seg("resources", "Kaynak")}
        </div>
        <div class="listpane scroll">
            {move || match data.get() {
                None => loading(),
                Some(Err(e)) => failed(e),
                Some(Ok(v)) => home_list(v).into_any(),
            }}
        </div>
        <ErrorNote/>
        <Composer placeholder="Ana Günlük’e yaz…" topic=None/>
        <TabBar/>
        <NewTopicSheet open=new_topic/>
    }
}

fn home_list(v: TopicsView) -> impl IntoView {
    let c = ctx();
    let badge = (v.unprocessed > 0).then(|| {
        let n = v.unprocessed;
        view! { <span class="badge" aria-label=format!("{n} işlenmemiş kayıt")>{n}</span> }
    });
    let no_topics = v.topics.is_empty();
    let rows = v
        .topics
        .into_iter()
        .map(|t| {
            let target = TopicRef {
                id: t.id.clone(),
                name: t.name.clone(),
            };
            let icon = view::topic_icon(t.icon.as_deref(), &t.kind);
            let kind = view::kind_line(&t.kind_label, t.due_date);
            let pin = t.pinned.then(|| {
                view! { <span class="topic__pin" aria-label="Sabitlendi"><Icon name="pinned" size="xs"/></span> }
            });
            view! {
                <button class="topic" on:click=move |_| c.screen.set(Screen::Flow(Some(target.clone())))>
                    <span class="topicicon"><Icon name=icon/></span>
                    <span class="topic__body">
                        <span class="topic__line"><span class="topic__name">{t.name}</span><span class="topic__kind">{kind}</span></span>
                        <span class="topic__preview">{t.preview.unwrap_or_default()}</span>
                    </span>
                    <span class="topic__side"><span class="topic__time">{t.last_label.unwrap_or_default()}</span>{pin}</span>
                </button>
            }
        })
        .collect_view();
    let empty = no_topics.then(|| {
        view! { <p class="setnote">"Henüz konu yok. Sağ üstteki + ile proje, alan ya da kaynak ekle."</p> }
    });
    view! {
        <div class="card card--top">
            <button class="topic" on:click=move |_| c.screen.set(Screen::Flow(None))>
                <span class="topicicon"><Icon name="notebook"/></span>
                <span class="topic__body">
                    <span class="topic__line"><span class="topic__name">"Ana Günlük"</span></span>
                    <span class="topic__preview">{v.journal_preview.unwrap_or_else(|| "Aklındakini yaz; gerisini NotAI düzenler.".into())}</span>
                </span>
                <span class="topic__side"><span class="topic__time">{v.journal_last_label.unwrap_or_default()}</span>{badge}</span>
            </button>
            <button class="topic" on:click=move |_| c.screen.set(Screen::Review)>
                <span class="topicicon"><Icon name="calcheck"/></span>
                <span class="topic__body">
                    <span class="topic__line"><span class="topic__name">"Haftalık gözden geçirme"</span></span>
                    <span class="topic__preview">"İşlenmemişler, projeler, önümüzdeki hafta"</span>
                </span>
            </button>
        </div>
        <h2 class="grouphead">"Konular"</h2>
        <div class="topics">{rows}</div>
        {empty}
    }
}
