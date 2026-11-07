//! Ana ekran (Revizyon 3: WhatsApp sohbet listesi düzeni): arama çubuğu, hap filtreler, tek
//! liste — Ana Günlük ve Haftalık gözden geçirme iğneli satırlar, ardından konular (yuvarlak
//! profil, ayraçsız, işlenmemiş rozeti) — ve sağ altta "Yeni kayıt" düğmesi.

use leptos::prelude::*;
use notai_api::{TopicRef, TopicsView};

use crate::sheets::NewTopicSheet;
use crate::view::{HomeFilter, TopicLook};
use crate::{Avatar, ErrorNote, Icon, Screen, TabBar, backend, ctx, failed, loading, view};

#[component]
pub fn Home() -> impl IntoView {
    let c = ctx();
    let filter = RwSignal::new(HomeFilter::All);
    let new_topic = RwSignal::new(false);
    let data = LocalResource::new(move || {
        let k = filter.get().kind_param();
        let _ = c.version.get();
        let b = backend();
        async move { b.topics(k).await }
    });
    let notice = LocalResource::new(move || {
        let _ = c.version.get();
        let b = backend();
        async move { b.notice().await }
    });
    let notice_bar = move || {
        let Some(Ok(Some(n))) = notice.get() else {
            return None;
        };
        let target = view::notice_target(&n);
        let icon = if n.kind == "question" {
            "spark"
        } else {
            "bell"
        };
        Some(view! {
            <div class="notice" role="status">
                <button class="notice__open" aria-label=format!("Aç: {}", n.title)
                    on:click=move |_| {
                        let target = target.clone();
                        crate::run_then(move |b| async move { b.clear_notice().await }, move |()| c.screen.set(Screen::Flow(target)));
                    }>
                    <Icon name=icon/>
                    <span class="notice__text">
                        <span class="notice__title">{n.title.clone()}</span>
                        <span class="notice__body">{n.body.clone()}</span>
                    </span>
                </button>
                <button class="iconbtn" aria-label="Bildirimi kapat"
                    on:click=move |_| crate::run(move |b| async move { b.clear_notice().await })>
                    <Icon name="x" size="sm"/>
                </button>
            </div>
        })
    };
    let unprocessed = move || match data.get() {
        Some(Ok(v)) => v.unprocessed,
        _ => 0,
    };
    let pills = HomeFilter::ALL
        .into_iter()
        .map(|f| {
            let badge = move || {
                let n = unprocessed();
                (f == HomeFilter::Unprocessed && n > 0)
                    .then(|| view! { <span class="badge">{n}</span> })
            };
            view! {
                <button class="pill" role="tab" aria-selected=move || (filter.get() == f).to_string()
                    on:click=move |_| filter.set(f)>{f.label()}{badge}</button>
            }
        })
        .collect_view();
    view! {
        <header class="topbar">
            <h1 class="topbar__title">"NotAI"</h1>
            <div class="topbar__actions">
                <button class="iconbtn" aria-label="Yeni konu" on:click=move |_| new_topic.set(true)><Icon name="plus"/></button>
                <button class="iconbtn" aria-label="Ayarlar" on:click=move |_| c.screen.set(Screen::Settings)><Icon name="dots"/></button>
            </div>
        </header>
        {notice_bar}
        <button class="searchbar" aria-label="Ara" on:click=move |_| c.screen.set(Screen::Search)>
            <Icon name="search"/><span>"Ara ya da notlarına sor"</span>
        </button>
        <div class="pills" role="tablist" aria-label="Filtre">{pills}</div>
        <div class="listpane scroll">
            {move || match data.get() {
                None => loading(),
                Some(Err(e)) => failed(e),
                Some(Ok(v)) => home_list(v, filter.get()).into_any(),
            }}
        </div>
        <button class="fab" aria-label="Yeni kayıt"
            on:click=move |_| { c.compose_focus.set(true); c.screen.set(Screen::Flow(None)); }>
            <Icon name="pencil"/>
        </button>
        <ErrorNote/>
        <TabBar/>
        <NewTopicSheet open=new_topic/>
    }
}

fn pin_icon() -> impl IntoView {
    view! { <span class="chat__pin" aria-label="Sabitlendi"><Icon name="pinned" size="xs"/></span> }
}

fn badge(n: u64, what: &'static str) -> Option<impl IntoView> {
    (n > 0).then(|| view! { <span class="badge" aria-label=format!("{n} {what}")>{n}</span> })
}

fn home_list(v: TopicsView, filter: HomeFilter) -> impl IntoView {
    let c = ctx();
    let journal = filter.shows_journal(v.unprocessed).then(|| {
        let n = v.unprocessed;
        view! {
            <button class="chat" on:click=move |_| c.screen.set(Screen::Flow(None))>
                <Avatar look=TopicLook::plain("notebook")/>
                <span class="chat__body">
                    <span class="chat__top">
                        <span class="chat__name">"Ana Günlük"</span>
                        <span class="chat__time" class:chat__time--unread={n > 0}>{v.journal_last_label.clone().unwrap_or_default()}</span>
                    </span>
                    <span class="chat__bottom">
                        <span class="chat__preview">{v.journal_preview.clone().unwrap_or_else(|| "Aklındakini yaz; gerisini NotAI düzenler.".into())}</span>
                        {pin_icon()}{badge(n, "işlenmemiş kayıt")}
                    </span>
                </span>
            </button>
        }
    });
    let review = filter.shows_review().then(|| {
        view! {
            <button class="chat" on:click=move |_| c.screen.set(Screen::Review)>
                <Avatar look=TopicLook::plain("calcheck")/>
                <span class="chat__body">
                    <span class="chat__top"><span class="chat__name">"Haftalık gözden geçirme"</span></span>
                    <span class="chat__bottom">
                        <span class="chat__preview">"İşlenmemişler, projeler, önümüzdeki hafta"</span>
                        {pin_icon()}
                    </span>
                </span>
            </button>
        }
    });
    let topics: Vec<_> = v.topics.into_iter().filter(|t| filter.keeps(t)).collect();
    let empty = (topics.is_empty() && journal.is_none()).then(|| {
        let text = match filter {
            HomeFilter::Unprocessed => "Karar bekleyen kayıt yok.",
            HomeFilter::All => "Henüz konu yok. Sağ üstteki + ile proje, alan ya da kaynak ekle.",
            _ => "Bu türde konu yok.",
        };
        view! { <p class="setnote">{text}</p> }
    });
    let rows = topics
        .into_iter()
        .map(|t| {
            let target = TopicRef {
                id: t.id.clone(),
                name: t.name.clone(),
            };
            let look = TopicLook::of(&t);
            let pending = t.pending;
            let pin = t.pinned.then(pin_icon);
            let preview = t.preview.clone().unwrap_or_else(|| view::kind_line(&t.kind_label, t.due_date));
            view! {
                <button class="chat" on:click=move |_| c.screen.set(Screen::Flow(Some(target.clone())))>
                    <Avatar look=look/>
                    <span class="chat__body">
                        <span class="chat__top">
                            <span class="chat__name">{t.name}</span>
                            <span class="chat__time" class:chat__time--unread={pending > 0}>{t.last_label.unwrap_or_default()}</span>
                        </span>
                        <span class="chat__bottom">
                            <span class="chat__preview">{preview}</span>
                            {pin}{badge(pending, "karar bekleyen kayıt")}
                        </span>
                    </span>
                </button>
            }
        })
        .collect_view();
    view! { {journal}{review}{rows}{empty} }
}
