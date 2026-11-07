//! Akış: Ana Günlük ya da bir konunun kayıtları (02-ana-gunluk.html, 03-konu-akisi.html).

use std::collections::HashMap;

use leptos::prelude::*;
use notai_api::{EntryView, JournalView, MorningPlanView, SuggestionView, TopicRef, TopicRowView};

use crate::composer::Composer;
use crate::sheets::TopicMenu;
use crate::{Avatar, ErrorNote, Icon, Screen, backend, ctx, failed, loading, run, view};

#[component]
pub fn Flow(topic: Option<TopicRef>) -> impl IntoView {
    let c = ctx();
    let topic_id = topic.as_ref().map(|t| t.id.clone());
    let is_journal = topic.is_none();
    let title = topic
        .as_ref()
        .map_or_else(|| "Ana Günlük".to_owned(), |t| t.name.clone());
    let tid = topic_id.clone();
    let menu = RwSignal::new(false);
    let menu_btn = topic.clone().map(|_| {
        view! { <button class="iconbtn" aria-label="Konu menüsü" on:click=move |_| menu.set(true)><Icon name="dots"/></button> }
    });
    let menu_sheet = topic
        .clone()
        .map(|t| view! { <TopicMenu topic=t open=menu/> });
    let data = LocalResource::new(move || {
        let _ = c.version.get();
        let b = backend();
        let t = tid.clone();
        async move {
            let j = b.journal(t).await?;
            let plan = if is_journal {
                Some(b.morning_plan().await?)
            } else {
                None
            };
            // Konu simgeleri (başlık + kayıt üstündeki konu adı) ana ekranla aynı olsun.
            let topics = b.topics("all").await?.topics;
            Ok::<Flowdata, String>((j, plan, topics))
        }
    });
    let scroller = NodeRef::<leptos::html::Main>::new();
    // WhatsApp gibi: açılışta ve yeni kayıtta en alta kaydır.
    Effect::new(move |_| {
        if data.get().is_some() {
            request_animation_frame(move || {
                if let Some(el) = scroller.get_untracked() {
                    el.set_scroll_top(el.scroll_height());
                }
            });
        }
    });
    let this_topic = {
        let id = topic_id.clone();
        move || match data.get() {
            Some(Ok((_, _, topics))) => topics.into_iter().find(|t| Some(&t.id) == id.as_ref()),
            _ => None,
        }
    };
    let sub = {
        let this_topic = this_topic.clone();
        move || match data.get() {
            Some(Ok((j, _, _))) if is_journal => view::unprocessed_line(j.unprocessed),
            _ => this_topic()
                .map(|t| view::kind_line(&t.kind_label, t.due_date))
                .unwrap_or_default(),
        }
    };
    let header_look = move || {
        if is_journal {
            view::TopicLook::plain("notebook")
        } else {
            this_topic().map_or_else(
                || view::TopicLook::plain("folder"),
                |t| view::TopicLook::of(&t),
            )
        }
    };
    let placeholder = if is_journal {
        "Aklındakini yaz…".to_owned()
    } else {
        format!("{title} konusuna yaz…")
    };
    view! {
        <header class="flowbar">
            <button class="iconbtn" aria-label="Geri" on:click=move |_| c.screen.set(Screen::Home)><Icon name="back"/></button>
            <div class="flowbar__who">
                {move || view! { <Avatar look=header_look() small=true/> }}
                <div class="flowbar__text"><span class="flowbar__name">{title.clone()}</span><span class="flowbar__sub">{sub}</span></div>
            </div>
            {menu_btn}
        </header>
        <main class="flow scroll" node_ref=scroller>
            {move || match data.get() {
                None => loading(),
                Some(Err(e)) => failed(e),
                Some(Ok((j, plan, topics))) => flow_items(j, plan, is_journal, &view::topic_looks(&topics)).into_any(),
            }}
        </main>
        <ErrorNote/>
        <Composer placeholder=placeholder topic=topic_id/>
        {menu_sheet}
    }
}

type Flowdata = (JournalView, Option<MorningPlanView>, Vec<TopicRowView>);

fn flow_items(
    j: JournalView,
    plan: Option<MorningPlanView>,
    is_journal: bool,
    looks: &HashMap<String, view::TopicLook>,
) -> impl IntoView {
    if j.groups.is_empty() {
        return view! {
            <div class="empty">
                <span class="empty__icon"><Icon name="pencil"/></span>
                <h2 class="empty__title">"Henüz kayıt yok"</h2>
                <p class="empty__text">"Aşağıya yaz ya da mikrofona dokun. Sınıflandırmayı NotAI önerir."</p>
            </div>
        }
        .into_any();
    }
    let mut plan = plan.filter(|p| !p.items.is_empty());
    j.groups
        .into_iter()
        .map(|g| {
            let card = if g.label == "Bugün" {
                plan.take().map(plan_card)
            } else {
                None
            };
            let items = g
                .items
                .into_iter()
                .map(|e| entry_row(e, is_journal, looks))
                .collect_view();
            view! {
                <div class="daysep"><span>{g.label}</span></div>
                {card}
                {items}
            }
        })
        .collect_view()
        .into_any()
}

fn plan_card(p: MorningPlanView) -> impl IntoView {
    let c = ctx();
    let items = p
        .items
        .into_iter()
        .map(|i| {
            let sub_class = view::plan_reason_class(&i.reason_kind);
            let id = i.task_id.clone();
            view! {
                <li class="plan__item">
                    <button class="task__check" aria-label=format!("Tamamla: {}", i.title)
                        on:click=move |_| { let id = id.clone(); run(move |b| async move { b.complete_task(id).await }) }>
                        <span class="check"></span>
                    </button>
                    <span class="plan__txt">{i.title}<span class=sub_class>{i.reason}</span></span>
                </li>
            }
        })
        .collect_view();
    let more = (p.more > 0).then(|| {
        view! { <button class="plan__more" on:click=move |_| c.screen.set(Screen::Tasks)>{format!("+{} görev daha", p.more)}</button> }
    });
    view! {
        <article class="entry entry--left">
            <div class="entry__gutter"></div>
            <div class="entry__main">
                <div class="plan">
                    <div class="plan__eyebrow"><Icon name="spark" size="xs"/>"Sabah planı"</div>
                    <h2 class="plan__title">{format!("{}:", p.headline)}</h2>
                    <ul class="plan__list">{items}</ul>
                    {more}
                </div>
            </div>
        </article>
    }
}

/// AI'ın sana yazdığı netleştirme sorusu: WhatsApp'taki karşı taraf mesajı gibi solda bir
/// balon; altında cevap alanı. Cevap verilince kayıt cevapla yeniden sınıflandırılır.
fn ai_question(s: SuggestionView) -> impl IntoView {
    let c = ctx();
    let b = crate::backend();
    let text = RwSignal::new(String::new());
    let (id_send, id_skip) = (s.id.clone(), s.id.clone());
    let pending = s.is_pending();
    let send = move || {
        let t = text.get_untracked().trim().to_owned();
        if t.is_empty() {
            return;
        }
        let id = id_send.clone();
        crate::run_in(
            c,
            b.clone(),
            move |b| async move { b.answer_question(id, t).await },
            |()| {},
        );
    };
    let send2 = send.clone();
    let footer = if pending {
        view! {
            <form class="aimsg__reply" on:submit=move |ev| { ev.prevent_default(); send() }>
                <input class="aimsg__input" type="text" lang="tr" enterkeyhint="send"
                    aria-label="NotAI'ye cevap" placeholder="Cevabını yaz…"
                    prop:value=move || text.get() on:input=move |ev| text.set(event_target_value(&ev))/>
                <button class="iconbtn" type="button" aria-label="Cevabı gönder" on:click=move |_| send2()><Icon name="send" size="sm"/></button>
            </form>
            <button class="btn btn--text aimsg__skip" aria-label="Soruyu geç"
                on:click=move |_| { let id = id_skip.clone(); run(move |b| async move { b.decide(id, false).await }) }>"Geç"</button>
        }
        .into_any()
    } else {
        match s.answer.clone() {
            Some(a) => {
                view! { <p class="aimsg__answer"><span class="aimsg__you">"Sen: "</span>{a}</p> }
                    .into_any()
            }
            None => view! { <p class="aimsg__answer aimsg__answer--skip">"Geçildi"</p> }.into_any(),
        }
    };
    view! {
        <div class="aimsg" role="group" aria-label="NotAI soruyor">
            <span class="aimsg__who"><Icon name="spark" size="xs"/>"NotAI"</span>
            <p class="aimsg__text">{s.chip.clone()}</p>
            {footer}
        </div>
    }
}

fn chip(s: SuggestionView) -> impl IntoView {
    if !s.is_pending() {
        let accepted = s.decision == "accepted";
        return accepted
            .then(|| {
                view! {
                    <span class="chip chip--accepted">
                        <span class="chip__body"><Icon name="check" size="xs"/><span>{s.chip}</span></span>
                    </span>
                }
            })
            .into_any();
    }
    let (id_ok, id_no) = (s.id.clone(), s.id.clone());
    view! {
        <span class="chip">
            <button class="chip__body" aria-label=format!("Kabul et: {}", s.chip)
                on:click=move |_| { let id = id_ok.clone(); run(move |b| async move { b.decide(id, true).await }) }>
                <Icon name="spark" size="xs"/><span>{s.chip.clone()}</span>
            </button>
            <button class="chip__x" aria-label=format!("Yoksay: {}", s.chip)
                on:click=move |_| { let id = id_no.clone(); run(move |b| async move { b.decide(id, false).await }) }>
                <Icon name="x" size="xs"/>
            </button>
        </span>
    }
    .into_any()
}

fn entry_row(
    e: EntryView,
    show_topic: bool,
    looks: &HashMap<String, view::TopicLook>,
) -> impl IntoView {
    let c = ctx();
    let dot = e
        .unprocessed
        .then(|| view! { <span class="dot" role="img" aria-label="İşlenmedi"></span> });
    let topic = e.topic.clone().filter(|_| show_topic).map(|t| {
        let look = looks
            .get(&t.id)
            .cloned()
            .unwrap_or_else(|| view::TopicLook::plain("folder"));
        view! { <span aria-hidden="true">"·"</span><span class=look.meta_class()><Icon name=look.icon.clone() size="xs"/><span>{t.name}</span></span> }
    });
    let failed = e
        .ai_failed
        .then(|| view! { <span aria-hidden="true">"·"</span><span>"AI işleyemedi"</span> });
    let (questions, others): (Vec<_>, Vec<_>) = e
        .suggestions
        .clone()
        .into_iter()
        .partition(|s| s.kind == "question");
    let chips = others.into_iter().map(chip).collect_view();
    let questions = questions.into_iter().map(ai_question).collect_view();
    let open = e.clone();
    view! {
        <article class="entry">
            <div class="entry__gutter">{dot}</div>
            <div class="entry__main">
                <div class="meta"><time class="tnum">{e.time.clone()}</time>{topic}{failed}</div>
                <div class="bubble" on:click=move |_| c.selected.set(Some(open.clone()))><p>{e.text.clone()}</p></div>
                {questions}
                <div class="chips">{chips}</div>
            </div>
        </article>
    }
}
