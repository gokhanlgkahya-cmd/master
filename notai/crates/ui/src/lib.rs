//! NotAI arayüzü (Leptos, CSR → WASM).
//!
//! Görünüm onaylı tasarımın birebir karşılığıdır: bileşen sınıfları `design/screens/screens.css`,
//! değerler `design/tokens.json`'dan üretilen CSS değişkenleri, ikonlar tasarım ekranlarındaki
//! `<symbol>`'ler (derleme sırasında gömülür; bkz. build.rs). İş mantığı burada yoktur.

#![forbid(unsafe_code)]

pub mod backend;
pub mod demo;
pub mod view;

use leptos::prelude::*;
use leptos::task::spawn_local;
use notai_api::{
    EntryView, JournalView, MorningPlanView, SuggestionView, TaskRowView, TopicRef, TopicsView,
};

use crate::backend::Backend;

const TOKENS_CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/tokens.css"));
const SCREENS_CSS: &str = include_str!("../../../design/screens/screens.css");
const APP_CSS: &str = include_str!("../style/app.css");
const SPRITE: &str = include_str!(concat!(env!("OUT_DIR"), "/sprite.svg"));

/// Uygulamanın gezinme durumu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    Home,
    /// Ana Günlük (`None`) ya da bir konunun akışı.
    Flow(Option<TopicRef>),
    Tasks,
    Calendar,
    Reminders,
    Search,
}

/// Paylaşılan durum (bağlam).
#[derive(Clone, Copy)]
struct Ctx {
    screen: RwSignal<Screen>,
    /// Her yazmadan sonra artar; veri kaynakları bunu izleyip yeniden yükler.
    version: RwSignal<u32>,
    /// Detay sayfası açık olan kayıt.
    selected: RwSignal<Option<EntryView>>,
    /// Kullanıcıya gösterilecek kısa hata.
    error: RwSignal<Option<String>>,
}

fn ctx() -> Ctx {
    use_context::<Ctx>().unwrap_or_else(|| Ctx {
        screen: RwSignal::new(Screen::Home),
        version: RwSignal::new(0),
        selected: RwSignal::new(None),
        error: RwSignal::new(None),
    })
}

fn backend() -> Backend {
    use_context::<Backend>().unwrap_or_else(Backend::demo)
}

/// Arka uca yazıp başarıda verileri yeniler; hatada mesaj gösterir.
fn run<F, Fut>(f: F)
where
    F: FnOnce(Backend) -> Fut + 'static,
    Fut: std::future::Future<Output = Result<(), String>> + 'static,
{
    let c = ctx();
    let b = backend();
    spawn_local(async move {
        match f(b).await {
            Ok(()) => {
                c.error.set(None);
                c.version.update(|v| *v += 1);
            }
            Err(e) => c.error.set(Some(e)),
        }
    });
}

#[component]
fn Icon(#[prop(into)] name: String, #[prop(optional, into)] size: String) -> impl IntoView {
    let class = if size.is_empty() {
        "ic".to_owned()
    } else {
        format!("ic ic--{size}")
    };
    view! { <svg class=class aria-hidden="true"><use href=format!("#i-{name}")></use></svg> }
}

#[component]
pub fn App() -> impl IntoView {
    provide_context(Ctx {
        screen: RwSignal::new(Screen::Home),
        version: RwSignal::new(0),
        selected: RwSignal::new(None),
        error: RwSignal::new(None),
    });
    provide_context(Backend::detect());
    let c = ctx();
    view! {
        <style>{TOKENS_CSS}{SCREENS_CSS}{APP_CSS}</style>
        <div inner_html=SPRITE></div>
        <div class="app phone">
            {move || match c.screen.get() {
                Screen::Home => view! { <Home/> }.into_any(),
                Screen::Flow(topic) => view! { <Flow topic=topic/> }.into_any(),
                Screen::Tasks => view! { <Tasks/> }.into_any(),
                Screen::Calendar => view! { <Soon title="Takvim" text="Gün ve hafta görünümü v0.2'de geliyor." icon="calendar"/> }.into_any(),
                Screen::Reminders => view! { <Soon title="Hatırlatmalar" text="Hatırlatmalar ve bildirimler v0.2'de geliyor." icon="bell"/> }.into_any(),
                Screen::Search => view! { <Soon title="Ara/Sor" text="Notlarına soru sorma v0.4'te geliyor." icon="search"/> }.into_any(),
            }}
            <EntrySheet/>
        </div>
    }
}

// ---------------- Ortak parçalar ----------------

#[component]
fn TabBar() -> impl IntoView {
    let c = ctx();
    let tab = move |screen: Screen, icon: &'static str, label: &'static str| {
        let target = screen.clone();
        let current = move || {
            if c.screen.get() == screen {
                Some("page")
            } else {
                None
            }
        };
        view! {
            <button class="tab" aria-current=current on:click=move |_| c.screen.set(target.clone())>
                <span class="tab__pill"><Icon name=icon/></span>
                <span>{label}</span>
            </button>
        }
    };
    view! {
        <nav class="tabbar" aria-label="Ana gezinme">
            {tab(Screen::Home, "notebook", "Günlük")}
            {tab(Screen::Tasks, "task", "Görevler")}
            {tab(Screen::Calendar, "calendar", "Takvim")}
            {tab(Screen::Reminders, "bell", "Hatırlatma")}
            {tab(Screen::Search, "search", "Ara/Sor")}
        </nav>
    }
}

/// Altta sabit yazma çubuğu: 1 dokunuşla yakalama. Boşken mikrofon, doluyken Gönder.
#[component]
fn Composer(#[prop(into)] placeholder: String, topic: Option<String>) -> impl IntoView {
    let text = RwSignal::new(String::new());
    let submit = move || {
        let t = text.get_untracked();
        if t.trim().is_empty() {
            return;
        }
        let topic = topic.clone();
        text.set(String::new());
        run(move |b| async move { b.capture(t, topic).await });
    };
    let submit2 = submit.clone();
    view! {
        <div class="composer">
            <button class="iconbtn" aria-label="Ekle"><Icon name="clip"/></button>
            <input
                class="field"
                type="text"
                lang="tr"
                placeholder=placeholder
                prop:value=move || text.get()
                on:input=move |ev| text.set(event_target_value(&ev))
                on:keydown=move |ev| if ev.key() == "Enter" { submit() }
            />
            {move || if text.get().trim().is_empty() {
                view! { <button class="mic" aria-label="Sesli yakala"><Icon name="mic"/></button> }.into_any()
            } else {
                let s = submit2.clone();
                view! { <button class="mic" aria-label="Gönder" on:click=move |_| s()><Icon name="send"/></button> }.into_any()
            }}
        </div>
    }
}

#[component]
fn ErrorNote() -> impl IntoView {
    let c = ctx();
    move || {
        c.error
            .get()
            .map(|e| view! { <p class="error-note" role="alert">{e}</p> })
    }
}

#[component]
fn Soon(title: &'static str, text: &'static str, icon: &'static str) -> impl IntoView {
    view! {
        <header class="topbar"><h1 class="topbar__title">{title}</h1></header>
        <div class="empty">
            <span class="empty__icon"><Icon name=icon/></span>
            <h2 class="empty__title">{title}</h2>
            <p class="empty__text">{text}</p>
        </div>
        <TabBar/>
    }
}

// ---------------- Ana ekran ----------------

#[component]
fn Home() -> impl IntoView {
    let c = ctx();
    let kind = RwSignal::new("all");
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
            </div>
        </header>
        <div class="segmented" role="tablist" aria-label="Konu türü">
            {seg("all", "Tümü")}{seg("projects", "Proje")}{seg("areas", "Alan")}{seg("resources", "Kaynak")}
        </div>
        <div class="listpane scroll">
            {move || match data.get() {
                None => view! { <p class="loading">"Yükleniyor…"</p> }.into_any(),
                Some(Err(e)) => view! { <p class="error-note">{e}</p> }.into_any(),
                Some(Ok(v)) => home_list(v).into_any(),
            }}
        </div>
        <ErrorNote/>
        <Composer placeholder="Ana Günlük’e yaz…" topic=None/>
        <TabBar/>
    }
}

fn home_list(v: TopicsView) -> impl IntoView {
    let c = ctx();
    let badge = (v.unprocessed > 0).then(|| {
        let n = v.unprocessed;
        view! { <span class="badge" aria-label=format!("{n} işlenmemiş kayıt")>{n}</span> }
    });
    let rows = v
        .topics
        .into_iter()
        .map(|t| {
            let target = TopicRef { id: t.id.clone(), name: t.name.clone() };
            let icon = view::topic_icon(t.icon.as_deref(), &t.kind);
            let kind = view::kind_line(&t.kind_label, t.due_date);
            view! {
                <button class="topic" on:click=move |_| c.screen.set(Screen::Flow(Some(target.clone())))>
                    <span class="topicicon"><Icon name=icon/></span>
                    <span class="topic__body">
                        <span class="topic__line"><span class="topic__name">{t.name}</span><span class="topic__kind">{kind}</span></span>
                        <span class="topic__preview">{t.preview.unwrap_or_default()}</span>
                    </span>
                    <span class="topic__side"><span class="topic__time">{t.last_label.unwrap_or_default()}</span></span>
                </button>
            }
        })
        .collect_view();
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
            <div class="topic" aria-disabled="true">
                <span class="topicicon"><Icon name="calcheck"/></span>
                <span class="topic__body">
                    <span class="topic__line"><span class="topic__name">"Haftalık gözden geçirme"</span></span>
                    <span class="topic__preview">"v0.4'te geliyor"</span>
                </span>
            </div>
        </div>
        <h2 class="grouphead">"Konular"</h2>
        <div class="topics">{rows}</div>
    }
}

// ---------------- Akış (Ana Günlük / konu) ----------------

#[component]
fn Flow(topic: Option<TopicRef>) -> impl IntoView {
    let c = ctx();
    let topic_id = topic.as_ref().map(|t| t.id.clone());
    let is_journal = topic.is_none();
    let title = topic
        .as_ref()
        .map_or_else(|| "Ana Günlük".to_owned(), |t| t.name.clone());
    let tid = topic_id.clone();
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
            Ok::<(JournalView, Option<MorningPlanView>), String>((j, plan))
        }
    });
    let scroller = NodeRef::<leptos::html::Main>::new();
    // WhatsApp gibi: açılışta ve yeni kayıtta en alta kaydır.
    Effect::new(move |_| {
        if data.get().is_some() {
            request_animation_frame(move || {
                if let Some(el) = scroller.get() {
                    el.set_scroll_top(el.scroll_height());
                }
            });
        }
    });
    let sub = move || match data.get() {
        Some(Ok((j, _))) if is_journal => view::unprocessed_line(j.unprocessed),
        _ => String::new(),
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
                <span class="topicicon"><Icon name=if is_journal { "notebook" } else { "folder" }/></span>
                <div class="flowbar__text"><span class="flowbar__name">{title.clone()}</span><span class="flowbar__sub">{sub}</span></div>
            </div>
        </header>
        <main class="flow scroll" node_ref=scroller>
            {move || match data.get() {
                None => view! { <p class="loading">"Yükleniyor…"</p> }.into_any(),
                Some(Err(e)) => view! { <p class="error-note">{e}</p> }.into_any(),
                Some(Ok((j, plan))) => flow_items(j, plan, is_journal).into_any(),
            }}
        </main>
        <ErrorNote/>
        <Composer placeholder=placeholder topic=topic_id/>
    }
}

fn flow_items(j: JournalView, plan: Option<MorningPlanView>, is_journal: bool) -> impl IntoView {
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
                .map(|e| entry_row(e, is_journal))
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

fn entry_row(e: EntryView, show_topic: bool) -> impl IntoView {
    let c = ctx();
    let dot = e
        .unprocessed
        .then(|| view! { <span class="dot" role="img" aria-label="İşlenmedi"></span> });
    let topic = e.topic.clone().filter(|_| show_topic).map(|t| {
        view! { <span aria-hidden="true">"·"</span><span class="meta__topic"><Icon name="folder" size="xs"/><span>{t.name}</span></span> }
    });
    let failed = e
        .ai_failed
        .then(|| view! { <span aria-hidden="true">"·"</span><span>"AI işleyemedi"</span> });
    let chips = e.suggestions.clone().into_iter().map(chip).collect_view();
    let open = e.clone();
    view! {
        <article class="entry">
            <div class="entry__gutter">{dot}</div>
            <div class="entry__main">
                <div class="meta"><time class="tnum">{e.time.clone()}</time>{topic}{failed}</div>
                <div class="bubble" on:click=move |_| c.selected.set(Some(open.clone()))><p>{e.text.clone()}</p></div>
                <div class="chips">{chips}</div>
            </div>
        </article>
    }
}

/// Not detayı: alttan kayan sayfa (%55 durağı).
#[component]
fn EntrySheet() -> impl IntoView {
    let c = ctx();
    move || {
        c.selected.get().map(|e| {
            let close = move |_| c.selected.set(None);
            let meta = view::sheet_meta(&e);
            let suggs = e
                .suggestions
                .clone()
                .into_iter()
                .filter(SuggestionView::is_pending)
                .map(|s| {
                    let (a, b) = (s.id.clone(), s.id.clone());
                    view! {
                        <li class="sugg">
                            <Icon name="spark"/>
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
                })
                .collect_view();
            let (arch, trash) = (e.id.clone(), e.id.clone());
            view! {
                <div class="scrim" on:click=close></div>
                <section class="sheet sheet--peek" role="dialog" aria-modal="true" aria-label="Not detayı">
                    <div class="sheet__handle"></div>
                    <div class="sheet__body scroll">
                        <p class="sheet__meta">{meta}</p>
                        <p class="sheet__text">{e.text.clone()}</p>
                        <h3 class="sheet__label">"Öneriler"</h3>
                        <ul class="suggs">{suggs}</ul>
                    </div>
                    <div class="sheet__actions">
                        <button on:click=move |_| { let id = arch.clone(); c.selected.set(None); run(move |b| async move { b.entry_action(id, "archive").await }) }>
                            <Icon name="archive"/><span>"Arşivle"</span>
                        </button>
                        <button on:click=move |_| { let id = trash.clone(); c.selected.set(None); run(move |b| async move { b.entry_action(id, "trash").await }) }>
                            <Icon name="x"/><span>"Sil"</span>
                        </button>
                    </div>
                </section>
            }
        })
    }
}

// ---------------- Görevler ----------------

#[component]
fn Tasks() -> impl IntoView {
    let c = ctx();
    let data = LocalResource::new(move || {
        let _ = c.version.get();
        let b = backend();
        async move { b.tasks().await }
    });
    let sub = move || match data.get() {
        Some(Ok(t)) => view::tasks_line(t.overdue.len(), t.today.len()),
        _ => String::new(),
    };
    view! {
        <header class="topbar">
            <div class="topbar__text"><h1 class="topbar__title">"Görevler"</h1><p class="topbar__sub">{sub}</p></div>
        </header>
        <div class="body scroll">
            {move || match data.get() {
                None => view! { <p class="loading">"Yükleniyor…"</p> }.into_any(),
                Some(Err(e)) => view! { <p class="error-note">{e}</p> }.into_any(),
                Some(Ok(t)) => view! {
                    {task_group("Geciken", true, t.overdue)}
                    {task_group("Bugün", false, t.today)}
                    {task_group("Yaklaşan", false, t.upcoming)}
                    {task_group("Tarihsiz", false, t.undated)}
                }.into_any(),
            }}
        </div>
        <ErrorNote/>
        <TabBar/>
    }
}

fn task_group(title: &'static str, late: bool, rows: Vec<TaskRowView>) -> impl IntoView {
    (!rows.is_empty()).then(|| {
        let n = rows.len();
        let head_class = if late { "grouphead grouphead--error" } else { "grouphead" };
        let items = rows
            .into_iter()
            .map(|t| {
                let id = t.id.clone();
                let meta = view::task_meta(&t);
                view! {
                    <div class="task">
                        <button class="task__check" aria-label=format!("Tamamla: {}", t.title)
                            on:click=move |_| { let id = id.clone(); run(move |b| async move { b.complete_task(id).await }) }>
                            <span class="check"></span>
                        </button>
                        <div class="task__body">
                            <span class="task__title">{t.title.clone()}</span>
                            <span class="task__meta">{meta}</span>
                        </div>
                        <div class="task__side"><span class="tag">{t.priority_label.clone()}</span></div>
                    </div>
                }
            })
            .collect_view();
        view! {
            <h2 class=head_class>{title}<span class="count">{n}</span></h2>
            <div class="card">{items}</div>
        }
    })
}
