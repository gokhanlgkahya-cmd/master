//! Alttan kayan sayfalar: not detayı, görev düzenleme, yeni konu, konu menüsü
//! (04-not-detayi.html; bileşen kuralları guidelines §3 "Bottom sheet").

use leptos::prelude::*;
use notai_api::{
    AttachmentView, EntryView, SuggestionView, TaskRowView, TopicEditRequest, TopicRef,
};

use crate::view::{self, TaskForm};
use crate::{Icon, Screen, backend, ctx, run, run_then, web};

/// Karartma + sayfa kabuğu. `class`: `sheet--peek` | `sheet--full` | `sheet--auto`.
fn shell(
    label: &'static str,
    class: impl Fn() -> &'static str + Send + Sync + 'static,
    close: impl Fn() + Clone + 'static,
    body: impl IntoView + 'static,
) -> impl IntoView {
    let close2 = close.clone();
    view! {
        <div class="scrim" on:click=move |_| close()></div>
        <section class=move || format!("sheet {}", class()) role="dialog" aria-modal="true" aria-label=label
            on:keydown=move |ev| if ev.key() == "Escape" { close2() }>
            <div class="sheet__handle"></div>
            {body}
        </section>
    }
}

// ---------------- Not detayı ----------------

/// Not detayı: tam metin, ekler, öneriler, uygulananlar (geri al), konuya taşı.
#[component]
pub fn EntrySheet() -> impl IntoView {
    let c = ctx();
    move || c.selected.get().map(|e| view! { <EntryDetail e=e/> })
}

#[component]
fn EntryDetail(e: EntryView) -> impl IntoView {
    let c = ctx();
    let moving = RwSignal::new(false);
    let eid = e.id.clone();
    let files = LocalResource::new(move || {
        let b = backend();
        let id = eid.clone();
        async move { b.attachments(id).await }
    });
    let close = move || c.selected.set(None);
    let meta = view::sheet_meta(&e);
    let pending = e
        .suggestions
        .iter()
        .filter(|s| s.is_pending())
        .cloned()
        .map(pending_row)
        .collect_view();
    let has_pending = e.suggestions.iter().any(SuggestionView::is_pending);
    let done = view::accepted(&e);
    let has_done = !done.is_empty();
    let done = done.into_iter().map(done_row).collect_view();
    let attachments = move || match files.get() {
        Some(Ok(list)) if !list.is_empty() => Some(
            view! {
                <h3 class="sheet__label">"Ekler"</h3>
                <div class="linkeds">{list.into_iter().map(attachment_row).collect_view()}</div>
            }
            .into_any(),
        ),
        Some(Err(err)) => Some(view! { <p class="setnote setnote--error">{err}</p> }.into_any()),
        _ => None,
    };
    let details = view! {
        <p class="sheet__meta">{meta}</p>
        <p class="sheet__text">{e.text.clone()}</p>
        {attachments}
        {has_pending.then(|| view! { <h3 class="sheet__label">"Öneriler"</h3><ul class="suggs">{pending}</ul> })}
        {has_done.then(|| view! { <h3 class="sheet__label">"Uygulananlar"</h3><ul class="suggs">{done}</ul> })}
    };
    let current_topic = e.topic.as_ref().map(|t| t.id.clone());
    let entry_id = e.id.clone();
    let body = move || {
        if moving.get() {
            view! { <TopicPicker entry=entry_id.clone() current=current_topic.clone()/> }.into_any()
        } else {
            ().into_any()
        }
    };
    let (arch, trash) = (e.id.clone(), e.id.clone());
    let content = view! {
        <div class="sheet__body scroll">
            <div class:hidden=move || moving.get()>{details}</div>
            {body}
        </div>
        <div class="sheet__foot">
            <div class="sheet__actions sheet__actions--3">
                <button aria-pressed=move || moving.get().to_string() on:click=move |_| moving.update(|m| *m = !*m)>
                    <Icon name="forward"/><span>"Konuya taşı"</span>
                </button>
                <button on:click=move |_| { let id = arch.clone(); c.selected.set(None); run(move |b| async move { b.entry_action(id, "archive").await }) }>
                    <Icon name="archive"/><span>"Arşivle"</span>
                </button>
                <button on:click=move |_| { let id = trash.clone(); c.selected.set(None); run(move |b| async move { b.entry_action(id, "trash").await }) }>
                    <Icon name="trash"/><span>"Sil"</span>
                </button>
            </div>
        </div>
    };
    shell(
        "Not detayı",
        move || {
            if moving.get() {
                "sheet--full"
            } else {
                "sheet--peek"
            }
        },
        close,
        content,
    )
}

fn pending_row(s: SuggestionView) -> impl IntoView {
    let c = ctx();
    let (a, b) = (s.id.clone(), s.id.clone());
    view! {
        <li class="sugg">
            <Icon name="spark" size="sm"/>
            <span class="sugg__txt">{s.chip.clone()}</span>
            <button class="iconbtn sugg__ok" aria-label=format!("Kabul et: {}", s.chip)
                on:click=move |_| { let id = a.clone(); c.selected.set(None); run(move |b| async move { b.decide(id, true).await }) }>
                <Icon name="check"/>
            </button>
            <button class="iconbtn sugg__no" aria-label=format!("Yoksay: {}", s.chip)
                on:click=move |_| { let id = b.clone(); c.selected.set(None); run(move |bk| async move { bk.decide(id, false).await }) }>
                <Icon name="x"/>
            </button>
        </li>
    }
}

/// Kabul edilmiş (elle ya da otonom modda) öneri: geri alınabilir.
fn done_row(s: SuggestionView) -> impl IntoView {
    let c = ctx();
    let id = s.id.clone();
    view! {
        <li class="sugg sugg--done">
            <Icon name="check" size="sm"/>
            <span class="sugg__txt">{s.chip.clone()}</span>
            <button class="btn btn--text" aria-label=format!("Geri al: {}", s.chip)
                on:click=move |_| { let id = id.clone(); c.selected.set(None); run(move |b| async move { b.undo(id).await }) }>
                "Geri al"
            </button>
        </li>
    }
}

fn attachment_row(a: AttachmentView) -> impl IntoView {
    let sub = match (&a.value, a.size) {
        (Some(v), _) if !v.is_empty() => v.clone(),
        (_, n) => format!("{} · {}", view::size_label(n), a.mime),
    };
    view! {
        <div class="linked">
            <Icon name=view::attachment_icon(&a.kind) size="sm"/>
            <span class="linked__txt">{a.name.clone()}<small>{sub}</small></span>
        </div>
    }
}

/// Konu seçici: Ana Günlük + tüm konular.
#[component]
fn TopicPicker(entry: String, current: Option<String>) -> impl IntoView {
    let c = ctx();
    let topics = LocalResource::new(move || {
        let b = backend();
        async move { b.topics("all").await }
    });
    let pick = move |target: Option<String>| {
        let id = entry.clone();
        c.selected.set(None);
        run(move |b| async move { b.move_entry(id, target).await });
    };
    let pick_journal = pick.clone();
    let journal_current = current.is_none();
    view! {
        <h3 class="sheet__label">"Konuya taşı"</h3>
        <div class="card pick">
            <button class="topic" aria-current=journal_current.then_some("true") on:click=move |_| pick_journal(None)>
                <span class="topicicon"><Icon name="notebook"/></span>
                <span class="topic__body"><span class="topic__name">"Ana Günlük"</span></span>
                {journal_current.then(|| view! { <Icon name="check" size="sm"/> })}
            </button>
            {move || match topics.get() {
                None => crate::loading(),
                Some(Err(e)) => crate::failed(e),
                Some(Ok(v)) => v.topics.into_iter().map(|t| {
                    let is_current = current.as_deref() == Some(t.id.as_str());
                    let id = t.id.clone();
                    let pick = pick.clone();
                    let look = view::TopicLook::of(&t);
                    view! {
                        <button class="topic" aria-current=is_current.then_some("true") on:click=move |_| pick(Some(id.clone()))>
                            <span class=look.box_class()><Icon name=look.icon.clone()/></span>
                            <span class="topic__body">
                                <span class="topic__line"><span class="topic__name">{t.name}</span><span class="topic__kind">{t.kind_label}</span></span>
                            </span>
                            {is_current.then(|| view! { <Icon name="check" size="sm"/> })}
                        </button>
                    }
                }).collect_view().into_any(),
            }}
        </div>
    }
}

// ---------------- Görev düzenleme ----------------

#[component]
pub fn TaskSheet() -> impl IntoView {
    let c = ctx();
    move || c.editing.get().map(|t| view! { <TaskEditor t=t/> })
}

#[component]
fn TaskEditor(t: TaskRowView) -> impl IntoView {
    let c = ctx();
    let f = TaskForm::from_task(&t);
    let title = RwSignal::new(f.title.clone());
    let date = RwSignal::new(f.date.clone());
    let time = RwSignal::new(f.time.clone());
    let priority = RwSignal::new(f.priority.clone());
    let repeat = RwSignal::new(f.repeat.clone());
    let remind = RwSignal::new(String::from("keep"));
    let first = NodeRef::<leptos::html::Input>::new();
    web::focus_later(first);
    let close = move || c.editing.set(None);
    let orig = t.clone();
    let save = move || {
        let form = TaskForm {
            title: title.get_untracked(),
            date: date.get_untracked(),
            time: time.get_untracked(),
            priority: priority.get_untracked(),
            repeat: repeat.get_untracked(),
            remind: remind.get_untracked().parse().ok(),
        };
        let req = view::task_edit(&orig, &form);
        c.editing.set(None);
        if view::edit_is_empty(&req) {
            return;
        }
        let id = orig.id.clone();
        run(move |b| async move { b.edit_task(id, req).await });
    };
    let id = t.id.clone();
    let complete = move |_| {
        let id = id.clone();
        c.editing.set(None);
        run(move |b| async move { b.complete_task(id).await });
    };
    let repeat_note = if t.recurring {
        " (şu an tekrarlı)"
    } else {
        ""
    };
    let content = view! {
        <form class="sheet__body scroll" on:submit=move |ev| { ev.prevent_default(); save() } id="task-edit">
            <h2 class="sheet__title">"Görevi düzenle"</h2>
            <label class="formfield formfield--sheet">
                <span class="formfield__label">"Başlık"</span>
                <span class="input"><input type="text" lang="tr" node_ref=first prop:value=move || title.get()
                    on:input=move |ev| title.set(event_target_value(&ev))/></span>
            </label>
            <h3 class="sheet__label">"Görev"</h3>
            <div class="fields">
                <label class="fieldbox">
                    <span class="fieldbox__label">"Tarih"</span>
                    <span class="fieldbox__val"><Icon name="calendar" size="sm"/>
                        <input type="date" aria-label="Tarih" prop:value=move || date.get()
                            on:input=move |ev| date.set(event_target_value(&ev))/></span>
                </label>
                <label class="fieldbox">
                    <span class="fieldbox__label">"Saat"</span>
                    <span class="fieldbox__val"><Icon name="clock" size="sm"/>
                        <input type="time" aria-label="Saat" prop:value=move || time.get()
                            on:input=move |ev| time.set(event_target_value(&ev))/></span>
                </label>
                <label class="fieldbox">
                    <span class="fieldbox__label">"Öncelik"</span>
                    <span class="fieldbox__val">{move || view! { <Icon name=view::priority_icon(&priority.get()) size="sm"/> }}
                        <select aria-label="Öncelik" prop:value=move || priority.get() on:change=move |ev| priority.set(event_target_value(&ev))>
                            {view::PRIORITY_OPTIONS.iter().map(|&(v, l)| view! { <option value=v selected=move || priority.get() == v>{l}</option> }).collect_view()}
                        </select></span>
                </label>
                <label class="fieldbox">
                    <span class="fieldbox__label">{format!("Tekrar{repeat_note}")}</span>
                    <span class="fieldbox__val"><Icon name="repeat" size="sm"/>
                        <select aria-label="Tekrar" prop:value=move || repeat.get() on:change=move |ev| repeat.set(event_target_value(&ev))>
                            {view::REPEAT_OPTIONS.iter().map(|&(v, l)| view! { <option value=v selected=move || repeat.get() == v>{l}</option> }).collect_view()}
                        </select></span>
                </label>
                <label class="fieldbox fieldbox--wide">
                    <span class="fieldbox__label">"Önce hatırlat"</span>
                    <span class="fieldbox__val"><Icon name="bell" size="sm"/>
                        <select aria-label="Önce hatırlat" prop:value=move || remind.get() on:change=move |ev| remind.set(event_target_value(&ev))>
                            <option value="keep" selected=move || remind.get() == "keep">"Değiştirme"</option>
                            {view::REMIND_OPTIONS.iter().map(|&(v, l)| { let v = v.to_string(); let v2 = v.clone(); view! { <option value=v selected=move || remind.get() == v2>{l}</option> } }).collect_view()}
                        </select></span>
                </label>
            </div>
        </form>
        <div class="sheet__foot sheet__foot--row">
            <button class="btn btn--outline" type="button" on:click=complete><Icon name="check" size="sm"/>"Tamamla"</button>
            <button class="btn btn--primary" type="submit" form="task-edit">"Kaydet"</button>
        </div>
    };
    shell("Görev düzenleme", || "sheet--full", close, content)
}

// ---------------- Konular ----------------

/// Yeni konu: ad + tür (Proje / Alan / Kaynak).
#[component]
pub fn NewTopicSheet(open: RwSignal<bool>) -> impl IntoView {
    move || open.get().then(|| view! { <NewTopicForm open=open/> })
}

#[component]
fn NewTopicForm(open: RwSignal<bool>) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let kind = RwSignal::new("project");
    let input = NodeRef::<leptos::html::Input>::new();
    web::focus_later(input);
    let save = move || {
        let n = name.get_untracked().trim().to_owned();
        if n.is_empty() {
            return;
        }
        let k = kind.get_untracked().to_owned();
        open.set(false);
        run(move |b| async move { b.create_topic(n, k).await });
    };
    let kinds = view::TOPIC_KINDS
        .iter()
        .map(|&(v, l)| {
            view! {
                <button type="button" role="tab" aria-selected=move || (kind.get() == v).to_string()
                    on:click=move |_| kind.set(v)>{l}</button>
            }
        })
        .collect_view();
    let content = view! {
        <form class="sheet__body" on:submit=move |ev| { ev.prevent_default(); save() }>
            <h2 class="sheet__title">"Yeni konu"</h2>
            <label class="formfield formfield--sheet">
                <span class="formfield__label">"Ad"</span>
                <span class="input"><input type="text" lang="tr" node_ref=input placeholder="Örn. Kostik tedarik"
                    prop:value=move || name.get() on:input=move |ev| name.set(event_target_value(&ev))/></span>
            </label>
            <div class="formfield formfield--sheet">
                <span class="formfield__label">"Tür"</span>
                <div class="seg3" role="tablist" aria-label="Konu türü">{kinds}</div>
            </div>
            <div class="sheet__foot sheet__foot--flat">
                <button class="btn btn--primary btn--block" type="submit">"Konuyu oluştur"</button>
            </div>
        </form>
    };
    shell(
        "Yeni konu",
        || "sheet--auto",
        move || open.set(false),
        content,
    )
}

/// Konu akışının ⋯ menüsü: sabitle / kaldır, yeniden adlandır.
#[component]
pub fn TopicMenu(topic: TopicRef, open: RwSignal<bool>) -> impl IntoView {
    move || {
        let t = topic.clone();
        open.get()
            .then(|| view! { <TopicMenuBody topic=t open=open/> })
    }
}

#[component]
fn TopicMenuBody(topic: TopicRef, open: RwSignal<bool>) -> impl IntoView {
    let c = ctx();
    let renaming = RwSignal::new(false);
    let name = RwSignal::new(topic.name.clone());
    let tid = topic.id.clone();
    let rows = LocalResource::new(move || {
        let b = backend();
        async move { b.topics("all").await }
    });
    let pinned = Memo::new(move |_| match rows.get() {
        Some(Ok(v)) => v.topics.iter().find(|t| t.id == tid).map(|t| t.pinned),
        _ => None,
    });
    let id_pin = topic.id.clone();
    let toggle_pin = move |_| {
        let Some(p) = pinned.get_untracked() else {
            return;
        };
        let id = id_pin.clone();
        open.set(false);
        let req = TopicEditRequest {
            pinned: Some(!p),
            ..TopicEditRequest::default()
        };
        run(move |b| async move { b.edit_topic(id, req).await });
    };
    let id_name = topic.id.clone();
    let rename = move || {
        let n = name.get_untracked().trim().to_owned();
        if n.is_empty() {
            return;
        }
        let id = id_name.clone();
        let target = TopicRef {
            id: id.clone(),
            name: n.clone(),
        };
        open.set(false);
        let req = TopicEditRequest {
            name: Some(n),
            ..TopicEditRequest::default()
        };
        run_then(
            move |b| async move { b.edit_topic(id, req).await },
            move |()| c.screen.set(Screen::Flow(Some(target))),
        );
    };
    let input = NodeRef::<leptos::html::Input>::new();
    let content = view! {
        <div class="sheet__body">
            <h2 class="sheet__title">{topic.name.clone()}</h2>
            <div class="card menu">
                <button class="setrow" on:click=toggle_pin disabled=move || pinned.get().is_none()>
                    <span class="setrow__ic"><Icon name="pinned"/></span>
                    <span class="setrow__txt"><span class="setrow__title">{move || match pinned.get() {
                        Some(true) => "Sabitlemeyi kaldır",
                        _ => "Sabitle",
                    }}</span></span>
                </button>
                <button class="setrow" on:click=move |_| { renaming.set(true); web::focus_later(input); }>
                    <span class="setrow__ic"><Icon name="pencil"/></span>
                    <span class="setrow__txt"><span class="setrow__title">"Yeniden adlandır"</span></span>
                </button>
            </div>
            <form class="formfield formfield--sheet" class:hidden=move || !renaming.get()
                on:submit=move |ev| { ev.prevent_default(); rename() }>
                <span class="formfield__label">"Yeni ad"</span>
                <span class="input"><input type="text" lang="tr" node_ref=input aria-label="Yeni ad"
                    prop:value=move || name.get() on:input=move |ev| name.set(event_target_value(&ev))/></span>
                <button class="btn btn--primary btn--block" type="submit">"Kaydet"</button>
            </form>
        </div>
    };
    shell(
        "Konu menüsü",
        || "sheet--auto",
        move || open.set(false),
        content,
    )
}
