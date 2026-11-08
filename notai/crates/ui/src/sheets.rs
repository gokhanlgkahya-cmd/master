//! Alttan kayan sayfalar: not detayı, görev düzenleme, yeni konu, konu menüsü
//! (04-not-detayi.html; bileşen kuralları guidelines §3 "Bottom sheet").

use leptos::prelude::*;
use notai_api::{
    AttachmentView, EntryView, SuggestionView, TaskRowView, TopicEditRequest, TopicRef,
};

use crate::view::{self, TaskForm};
use crate::{Avatar, Icon, Screen, backend, ctx, run, run_then, web};

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
                <Avatar look=view::TopicLook::plain("notebook") small=true/>
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
                            <Avatar look=look small=true/>
                            <span class="topic__body">
                                <span class="topic__line"><span class="topic__name">{if t.path.is_empty() { t.name } else { t.path }}</span><span class="topic__kind">{t.kind_label}</span></span>
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

/// Yeni konu: ad + tür (Proje / Alan / Kaynak). `parent` verilirse alt konu açılır.
#[component]
pub fn NewTopicSheet(
    open: RwSignal<bool>,
    #[prop(optional)] parent: Option<TopicRef>,
) -> impl IntoView {
    move || {
        let parent = parent.clone();
        open.get()
            .then(|| view! { <NewTopicForm open=open parent=parent/> })
    }
}

#[component]
fn NewTopicForm(open: RwSignal<bool>, parent: Option<TopicRef>) -> impl IntoView {
    let parent_id = parent.as_ref().map(|p| p.id.clone());
    let title = parent.as_ref().map_or_else(
        || "Yeni konu".to_owned(),
        |p| format!("{} · yeni alt konu", p.name),
    );
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
        let parent = parent_id.clone();
        run(move |b| async move { b.create_topic(n, k, parent).await });
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
            <h2 class="sheet__title">{title}</h2>
            <label class="formfield formfield--sheet">
                <span class="formfield__label">"Ad"</span>
                <span class="input"><input type="text" lang="tr" node_ref=input placeholder="Örn. Malzeme hareketleri"
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

/// Konunun ⋯ menüsü: sabitle, yeniden adlandır, alt konu ekle, üst konuyu değiştir, profil
/// fotoğrafı, simge.
#[component]
pub fn TopicMenu(topic: TopicRef, open: RwSignal<bool>) -> impl IntoView {
    let new_sub = RwSignal::new(false);
    let watching = RwSignal::new(false);
    let valuing = RwSignal::new(false);
    let t_sheet = topic.clone();
    let t_watch = topic.clone();
    let t_asset = topic.clone();
    view! {
        {move || {
            let t = topic.clone();
            open.get()
                .then(|| view! { <TopicMenuBody topic=t open=open new_sub=new_sub watching=watching valuing=valuing/> })
        }}
        <NewTopicSheet open=new_sub parent=t_sheet/>
        {move || watching.get().then(|| view! { <WatchSheet topic=t_watch.clone() open=watching/> })}
        {move || valuing.get().then(|| view! { <AssetSheet topic=Some(t_asset.clone()) open=valuing/> })}
    }
}

/// Varlık formu (2026-11-08): tür, miktar + birim, fiyat sembolü, toplam değer. `topic` yoksa
/// "Varlıklar" altında yeni konu açılır; varsa o konunun varlık bilgisi düzenlenir/kaldırılır.
#[component]
pub fn AssetSheet(topic: Option<TopicRef>, open: RwSignal<bool>) -> impl IntoView {
    let c = ctx();
    let b = backend();
    let form = RwSignal::new(notai_api::AssetRequest {
        kind: "gayrimenkul".into(),
        ..Default::default()
    });
    let existing = RwSignal::new(false);
    let kinds = RwSignal::new(Vec::<(String, String)>::new());
    let tid = topic.as_ref().map(|t| t.id.clone());
    let data = LocalResource::new(move || {
        let b = backend();
        async move { b.assets().await }
    });
    let tid_load = tid.clone();
    Effect::new(move |_| {
        if let Some(Ok(v)) = data.get() {
            kinds.set(v.kind_options.clone());
            if let Some(id) = &tid_load
                && let Some(r) = v.assets.iter().find(|r| &r.topic_id == id)
            {
                form.set(r.form.clone());
                existing.set(true);
            }
        }
    });
    let is_new = topic.is_none();
    let title = topic.as_ref().map_or_else(
        || "Yeni varlık".to_owned(),
        |t| format!("{} · varlık bilgisi", t.name),
    );
    let tid_save = tid.clone();
    let b_save = b.clone();
    let save = move || {
        let req = form.get_untracked();
        if req.value.trim().is_empty() || (is_new && req.name.trim().is_empty()) {
            c.error.set(Some("Ad ve değer gerekli.".into()));
            return;
        }
        open.set(false);
        match tid_save.clone() {
            Some(id) => crate::run_in(
                c,
                b_save.clone(),
                move |b| async move { b.set_asset(id, req).await },
                |()| {},
            ),
            None => crate::run_in(
                c,
                b_save.clone(),
                move |b| async move { b.create_asset(req).await },
                |_| {},
            ),
        }
    };
    let remove = move |_| {
        let Some(id) = tid.clone() else {
            return;
        };
        open.set(false);
        crate::run_in(
            c,
            b.clone(),
            move |b| async move { b.set_asset(id, notai_api::AssetRequest::default()).await },
            |()| {},
        );
    };
    let input = NodeRef::<leptos::html::Input>::new();
    web::focus_later(input);
    let field = move |label: &'static str,
                      aria: &'static str,
                      mode: &'static str,
                      ph: &'static str,
                      get: fn(&notai_api::AssetRequest) -> String,
                      set: fn(&mut notai_api::AssetRequest, String)| {
        view! {
            <label class="fieldbox">
                <span class="fieldbox__label">{label}</span>
                <span class="fieldbox__val"><input type="text" lang="tr" inputmode=mode aria-label=aria placeholder=ph
                    prop:value=move || form.with(get) on:input=move |ev| form.update(|f| set(f, event_target_value(&ev)))/></span>
            </label>
        }
    };
    let content = view! {
        <form class="sheet__body" on:submit=move |ev| { ev.prevent_default(); save() }>
            <h2 class="sheet__title">{title}</h2>
            <p class="setnote">"Sembol girersen (ör. THYAO, gram altın, USD) Hermes / Claude Code günde bir birim fiyata bakar; değer miktar × fiyat olur, %2'den büyük değişim bu konuya yazılır. Sembolsüz varlığı (ev, araç) arada sen güncellersin."</p>
            {is_new.then(|| view! {
                <label class="formfield formfield--sheet">
                    <span class="formfield__label">"Ad"</span>
                    <span class="input"><input type="text" lang="tr" node_ref=input placeholder="Örn. Kadıköy daire, Altın birikimi"
                        prop:value=move || form.with(|f| f.name.clone()) on:input=move |ev| form.update(|f| f.name = event_target_value(&ev))/></span>
                </label>
            })}
            <div class="fields">
                <label class="fieldbox">
                    <span class="fieldbox__label">"Tür"</span>
                    <span class="fieldbox__val"><select aria-label="Tür" on:change=move |ev| form.update(|f| f.kind = event_target_value(&ev))>
                        {move || kinds.get().into_iter().map(|(k, l)| {
                            let k2 = k.clone();
                            view! { <option value=k selected=move || form.with(|f| f.kind == k2)>{l}</option> }
                        }).collect_view()}
                    </select></span>
                </label>
                {field("Değer (₺)", "Değer", "decimal", "Örn. 1.250.000", |f| f.value.clone(), |f, v| f.value = v)}
            </div>
            <div class="fields">
                {field("Miktar", "Miktar", "decimal", "İsteğe bağlı", |f| f.quantity.clone(), |f, v| f.quantity = v)}
                {field("Birim", "Birim", "text", "gram, adet, USD", |f| f.unit.clone(), |f, v| f.unit = v)}
            </div>
            <label class="formfield formfield--sheet">
                <span class="formfield__label">"Fiyat sembolü (isteğe bağlı)"</span>
                <span class="input"><input type="text" lang="tr" placeholder="THYAO · gram altın · USD/TRY · BTC"
                    prop:value=move || form.with(|f| f.symbol.clone()) on:input=move |ev| form.update(|f| f.symbol = event_target_value(&ev))/></span>
            </label>
            <div class="sheet__foot sheet__foot--flat">
                <button class="btn btn--primary btn--block" type="submit">"Kaydet"</button>
                {move || existing.get().then(|| {
                    let remove = remove.clone();
                    view! { <button class="btn btn--text btn--block" type="button" on:click=remove>"Varlık bilgisini kaldır"</button> }
                })}
            </div>
        </form>
    };
    shell("Varlık", || "sheet--auto", move || open.set(false), content)
}

/// Fiyat / ürün takibi formu: ürün, bağlantılar, hedef fiyat, sıklık. Ajan (Hermes / Claude
/// Code) bu sıklıkla bakar; yalnız fiyat düşünce ya da daha uygun bağlantı bulununca konuya yazar.
#[component]
fn WatchSheet(topic: TopicRef, open: RwSignal<bool>) -> impl IntoView {
    let c = ctx();
    let b = backend();
    let query = RwSignal::new(String::new());
    let urls = RwSignal::new(String::new());
    let target = RwSignal::new(String::new());
    let every = RwSignal::new(String::from("24"));
    let existing = RwSignal::new(false);
    let tid = topic.id.clone();
    let rows = LocalResource::new(move || {
        let b = backend();
        async move { b.topics("all").await }
    });
    Effect::new(move |_| {
        if let Some(Ok(v)) = rows.get()
            && let Some(w) = v
                .topics
                .iter()
                .find(|t| t.id == tid)
                .and_then(|t| t.watch.clone())
        {
            query.set(w.query);
            urls.set(w.urls.join("\n"));
            target.set(w.target_price);
            every.set(w.every_hours.to_string());
            existing.set(true);
        }
    });
    let id_save = topic.id.clone();
    let b_save = b.clone();
    let save = move || {
        let req = notai_api::WatchRequest {
            query: query.get_untracked(),
            urls: urls.get_untracked().lines().map(str::to_owned).collect(),
            target_price: target.get_untracked(),
            every_hours: every.get_untracked().parse().ok(),
            active: Some(true),
        };
        if req.query.trim().is_empty() {
            return;
        }
        let id = id_save.clone();
        open.set(false);
        crate::run_in(
            c,
            b_save.clone(),
            move |b| async move { b.set_watch(id, req).await },
            |()| {},
        );
    };
    let id_stop = topic.id.clone();
    let stop = move |_| {
        let id = id_stop.clone();
        open.set(false);
        crate::run_in(
            c,
            b.clone(),
            move |b| async move { b.set_watch(id, notai_api::WatchRequest::default()).await },
            |()| {},
        );
    };
    let input = NodeRef::<leptos::html::Input>::new();
    web::focus_later(input);
    let options = view::WATCH_EVERY
        .iter()
        .map(|&(h, l)| {
            let v = h.to_string();
            let v2 = v.clone();
            view! { <option value=v selected=move || every.get() == v2>{l}</option> }
        })
        .collect_view();
    let content = view! {
        <form class="sheet__body" on:submit=move |ev| { ev.prevent_default(); save() }>
            <h2 class="sheet__title">{format!("{} · fiyat takibi", topic.name)}</h2>
            <p class="setnote">"Hermes / Claude Code seçtiğin sıklıkla bakar; yalnız fiyat düşünce, daha uygun bağlantı bulununca ya da hedefe inince bu konuya yazar ve bildirim gönderir."</p>
            <label class="formfield formfield--sheet">
                <span class="formfield__label">"Ürün"</span>
                <span class="input"><input type="text" lang="tr" node_ref=input placeholder="Örn. Pixel 10 Pro 256 GB"
                    prop:value=move || query.get() on:input=move |ev| query.set(event_target_value(&ev))/></span>
            </label>
            <label class="formfield formfield--sheet">
                <span class="formfield__label">"Bağlantılar (isteğe bağlı, her satıra bir tane)"</span>
                <textarea class="textarea" rows="3" placeholder="https://…"
                    prop:value=move || urls.get() on:input=move |ev| urls.set(event_target_value(&ev))></textarea>
            </label>
            <div class="fields">
                <label class="fieldbox">
                    <span class="fieldbox__label">"Hedef fiyat (₺)"</span>
                    <span class="fieldbox__val"><input type="text" inputmode="decimal" aria-label="Hedef fiyat" placeholder="İsteğe bağlı"
                        prop:value=move || target.get() on:input=move |ev| target.set(event_target_value(&ev))/></span>
                </label>
                <label class="fieldbox">
                    <span class="fieldbox__label">"Sıklık"</span>
                    <span class="fieldbox__val"><select aria-label="Sıklık" on:change=move |ev| every.set(event_target_value(&ev))>{options}</select></span>
                </label>
            </div>
            <div class="sheet__foot sheet__foot--flat">
                <button class="btn btn--primary btn--block" type="submit">"Takibi kaydet"</button>
                {move || existing.get().then(|| {
                    let stop = stop.clone();
                    view! { <button class="btn btn--text btn--block" type="button" on:click=stop>"Takibi kapat"</button> }
                })}
            </div>
        </form>
    };
    shell(
        "Fiyat takibi",
        || "sheet--auto",
        move || open.set(false),
        content,
    )
}

#[component]
fn TopicMenuBody(
    topic: TopicRef,
    open: RwSignal<bool>,
    new_sub: RwSignal<bool>,
    watching: RwSignal<bool>,
    valuing: RwSignal<bool>,
) -> impl IntoView {
    let c = ctx();
    let renaming = RwSignal::new(false);
    let name = RwSignal::new(topic.name.clone());
    let tid = topic.id.clone();
    let rows = LocalResource::new(move || {
        let b = backend();
        async move { b.topics("all").await }
    });
    let tid2 = tid.clone();
    let pinned = Memo::new(move |_| match rows.get() {
        Some(Ok(v)) => v.topics.iter().find(|t| t.id == tid).map(|t| t.pinned),
        _ => None,
    });
    let row = Memo::new(move |_| match rows.get() {
        Some(Ok(v)) => v.topics.into_iter().find(|t| t.id == tid2),
        _ => None,
    });
    let choosing_icon = RwSignal::new(false);
    let choosing_parent = RwSignal::new(false);
    let photo = NodeRef::<leptos::html::Input>::new();
    // Bağlam (bağlantı) burada yakalanır: fotoğraf küçültme `await`'inden sonra `use_context`
    // okunamaz (`run_then` notu).
    let edit = {
        let id = topic.id.clone();
        let b = backend();
        move |req: TopicEditRequest| {
            let id = id.clone();
            open.set(false);
            crate::run_in(
                c,
                b.clone(),
                move |b| async move { b.edit_topic(id, req).await },
                |()| {},
            );
        }
    };
    let on_photo = {
        let edit = edit.clone();
        move |_ev: leptos::ev::Event| {
            let Some(el) = photo.get_untracked() else {
                return;
            };
            let edit = edit.clone();
            leptos::task::spawn_local(async move {
                match web::avatar_from_input(&el, 96).await {
                    Ok(Some(data)) => edit(TopicEditRequest {
                        avatar: Some(data),
                        ..TopicEditRequest::default()
                    }),
                    Ok(None) => {}
                    Err(e) => c.error.set(Some(e)),
                }
            });
        }
    };
    let remove_photo = {
        let edit = edit.clone();
        move |_| {
            edit(TopicEditRequest {
                avatar: Some(String::new()),
                ..TopicEditRequest::default()
            });
        }
    };
    let set_icon = {
        let edit = edit.clone();
        move |icon: &str| {
            edit(TopicEditRequest {
                icon: Some(icon.to_owned()),
                ..TopicEditRequest::default()
            });
        }
    };
    let parent_list = {
        let edit = edit.clone();
        let me = topic.id.clone();
        move || {
            let Some(Ok(v)) = rows.get() else {
                return ().into_any();
            };
            // Kendi dalının içine taşınamaz (döngü olur).
            let blocked = view::subtree_ids(&v.topics, &me);
            let current = v
                .topics
                .iter()
                .find(|t| t.id == me)
                .and_then(|t| t.parent_id.clone());
            let pick = {
                let edit = edit.clone();
                move |p: String| {
                    edit(TopicEditRequest {
                        parent_id: Some(p),
                        ..TopicEditRequest::default()
                    });
                }
            };
            let top = {
                let pick = pick.clone();
                let is_cur = current.is_none();
                view! {
                    <button class="topic" aria-current=is_cur.then_some("true") on:click=move |_| pick(String::new())>
                        <span class="topic__body"><span class="topic__name">"En üst (ana ekranda)"</span></span>
                        {is_cur.then(|| view! { <Icon name="check" size="sm"/> })}
                    </button>
                }
            };
            let items = v
                .topics
                .into_iter()
                .filter(|t| !blocked.contains(&t.id))
                .map(|t| {
                    let pick = pick.clone();
                    let is_cur = current.as_deref() == Some(t.id.as_str());
                    let id = t.id.clone();
                    let look = view::TopicLook::of(&t);
                    view! {
                        <button class="topic" aria-current=is_cur.then_some("true") on:click=move |_| pick(id.clone())>
                            <Avatar look=look small=true/>
                            <span class="topic__body"><span class="topic__name">{t.path}</span></span>
                            {is_cur.then(|| view! { <Icon name="check" size="sm"/> })}
                        </button>
                    }
                })
                .collect_view();
            view! { <div class="card pick" role="group" aria-label="Üst konu seç">{top}{items}</div> }.into_any()
        }
    };
    let icon_grid = {
        let set_icon = set_icon.clone();
        move || {
            let current = row.get();
            let color = current.as_ref().map_or(0, |t| t.color);
            let auto = {
                let set_icon = set_icon.clone();
                view! {
                    <button class="icongrid__item" aria-label="Otomatik simge" on:click=move |_| set_icon("")>
                        <span class="avatar avatar--sm"><Icon name="sparkle"/></span><span>"Otomatik"</span>
                    </button>
                }
            };
            let items = notai_api::TOPIC_ICON_CHOICES
                .iter()
                .map(|&(icon, label)| {
                    let set_icon = set_icon.clone();
                    let look = view::TopicLook {
                        icon: icon.to_owned(),
                        color,
                        avatar: None,
                    };
                    view! {
                        <button class="icongrid__item" aria-label=format!("Simge: {label}") on:click=move |_| set_icon(icon)>
                            <Avatar look=look small=true/><span>{label}</span>
                        </button>
                    }
                })
                .collect_view();
            view! { <div class="icongrid" role="group" aria-label="Simge seç">{auto}{items}</div> }
        }
    };
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
            move |()| c.screen.set(Screen::Topic(target)),
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
                <button class="setrow" on:click=move |_| { if let Some(el) = photo.get_untracked() { el.click(); } }>
                    <span class="setrow__ic"><Icon name="image"/></span>
                    <span class="setrow__txt"><span class="setrow__title">"Profil fotoğrafı"</span>
                        <span class="setrow__sub">"Yuvarlak içinde simgenin yerine görünür"</span></span>
                </button>
                {move || row.get().and_then(|t| t.avatar).map(|_| {
                    let remove_photo = remove_photo.clone();
                    view! {
                        <button class="setrow" on:click=remove_photo>
                            <span class="setrow__ic"><Icon name="trash"/></span>
                            <span class="setrow__txt"><span class="setrow__title">"Fotoğrafı kaldır"</span></span>
                        </button>
                    }
                })}
                <button class="setrow" on:click=move |_| { open.set(false); new_sub.set(true); }>
                    <span class="setrow__ic"><Icon name="plus"/></span>
                    <span class="setrow__txt"><span class="setrow__title">"Alt konu ekle"</span>
                        <span class="setrow__sub">"Örn. Malzeme hareketleri, Yapılacaklar, Para alışverişi"</span></span>
                </button>
                <button class="setrow" on:click=move |_| { open.set(false); watching.set(true); }>
                    <span class="setrow__ic"><Icon name="cart"/></span>
                    <span class="setrow__txt"><span class="setrow__title">"Fiyat takibi"</span>
                        <span class="setrow__sub">{move || match row.get().and_then(|t| t.watch) {
                            Some(w) => format!("Takipte: {}", w.query),
                            None => "Bir ürünün fiyatını takip et; düşünce haber ver".to_owned(),
                        }}</span></span>
                </button>
                <button class="setrow" on:click=move |_| { open.set(false); valuing.set(true); }>
                    <span class="setrow__ic"><Icon name="wallet"/></span>
                    <span class="setrow__txt"><span class="setrow__title">"Varlık bilgisi"</span>
                        <span class="setrow__sub">"Ev, araç, hisse, altın… değerini tut, Varlıklar'da topla"</span></span>
                </button>
                <button class="setrow" aria-expanded=move || choosing_parent.get().to_string()
                    on:click=move |_| choosing_parent.update(|v| *v = !*v)>
                    <span class="setrow__ic"><Icon name="layers"/></span>
                    <span class="setrow__txt"><span class="setrow__title">"Üst konuyu değiştir"</span>
                        <span class="setrow__sub">"Bu konuyu başka bir konunun altına taşı"</span></span>
                </button>
                <button class="setrow" aria-expanded=move || choosing_icon.get().to_string()
                    on:click=move |_| choosing_icon.update(|v| *v = !*v)>
                    <span class="setrow__ic"><Icon name="grid"/></span>
                    <span class="setrow__txt"><span class="setrow__title">"Simgeyi değiştir"</span>
                        <span class="setrow__sub">"Seçmezsen addan ve kayıtlardan otomatik seçilir"</span></span>
                </button>
            </div>
            <input class="filepick" type="file" accept="image/*" tabindex="-1" aria-hidden="true" node_ref=photo on:change=on_photo/>
            {move || choosing_parent.get().then(parent_list.clone())}
            {move || choosing_icon.get().then(icon_grid.clone())}
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
