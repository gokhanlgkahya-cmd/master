//! Konu açılışı (Gökhan, 2026-11-07): alt konusu olan konu WhatsApp ana ekranı gibi açılır —
//! en üstte konunun kendi (alt konuları da kapsayan) yazışması iğneli satır, altında alt konular,
//! sağ altta "Yeni kayıt". Alt konusu olmayan konu doğrudan yazışma olarak açılır.

use leptos::prelude::*;
use notai_api::{TopicRef, TopicRowView};

use crate::screens::flow::Flow;
use crate::screens::home::{badge, pin_icon, topic_chat};
use crate::sheets::{NewTopicSheet, TopicMenu};
use crate::view::{self, TopicLook};
use crate::{Avatar, ErrorNote, Icon, Screen, TabBar, backend, ctx, failed, loading};

#[component]
pub fn Hub(topic: TopicRef) -> impl IntoView {
    let c = ctx();
    let id = topic.id.clone();
    let data = LocalResource::new(move || {
        let _ = c.version.get();
        let b = backend();
        async move { b.topics("all").await }
    });
    let t2 = topic.clone();
    move || match data.get() {
        None => loading(),
        Some(Err(e)) => failed(e),
        Some(Ok(v)) => {
            let me = v.topics.iter().find(|t| t.id == id).cloned();
            let children = view::children_of(&v.topics, &id);
            match me {
                // Alt konusu yoksa (ya da liste henüz bilmiyorsa) doğrudan yazışma.
                Some(row) if !children.is_empty() => hub_view(row, children, &v.topics).into_any(),
                _ => view! { <Flow topic=Some(t2.clone())/> }.into_any(),
            }
        }
    }
}

fn hub_view(me: TopicRowView, children: Vec<TopicRowView>, all: &[TopicRowView]) -> impl IntoView {
    let c = ctx();
    let new_sub = RwSignal::new(false);
    let menu = RwSignal::new(false);
    let me_ref = view::topic_ref(&me);
    let back = me
        .parent_id
        .as_deref()
        .and_then(|p| all.iter().find(|t| t.id == p))
        .map(view::topic_ref);
    let look = TopicLook::of(&me);
    let sub_line = format!("{} alt konu", children.len());
    let (flow_ref, fab_ref) = (me_ref.clone(), me_ref.clone());
    let pending = me.pending;
    let general = view! {
        <button class="chat" on:click=move |_| c.screen.set(Screen::Flow(Some(flow_ref.clone())))>
            <Avatar look=look.clone()/>
            <span class="chat__body">
                <span class="chat__top">
                    <span class="chat__name">{format!("{} · genel", me.name)}</span>
                    <span class="chat__time" class:chat__time--unread={pending > 0}>{me.last_label.clone().unwrap_or_default()}</span>
                </span>
                <span class="chat__bottom">
                    <span class="chat__preview">{me.preview.clone().unwrap_or_else(|| "Buraya yaz; NotAI alt konulara dağıtır.".into())}</span>
                    {pin_icon()}{badge(pending, "karar bekleyen kayıt")}
                </span>
            </span>
        </button>
    };
    let rows = children.into_iter().map(topic_chat).collect_view();
    view! {
        <header class="flowbar">
            <button class="iconbtn" aria-label="Geri"
                on:click=move |_| c.screen.set(back.clone().map_or(Screen::Home, Screen::Topic))>
                <Icon name="back"/>
            </button>
            <div class="flowbar__who">
                <Avatar look=look small=true/>
                <div class="flowbar__text"><span class="flowbar__name">{me.name.clone()}</span><span class="flowbar__sub">{sub_line}</span></div>
            </div>
            <button class="iconbtn" aria-label="Alt konu ekle" on:click=move |_| new_sub.set(true)><Icon name="plus"/></button>
            <button class="iconbtn" aria-label="Konu menüsü" on:click=move |_| menu.set(true)><Icon name="dots"/></button>
        </header>
        <div class="listpane scroll">{general}{rows}</div>
        <button class="fab" aria-label="Yeni kayıt"
            on:click=move |_| { c.compose_focus.set(true); c.screen.set(Screen::Flow(Some(fab_ref.clone()))); }>
            <Icon name="pencil"/>
        </button>
        <ErrorNote/>
        <TabBar/>
        <NewTopicSheet open=new_sub parent=me_ref.clone()/>
        <TopicMenu topic=me_ref open=menu/>
    }
}
