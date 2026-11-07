//! NotAI arayüzü (Leptos, CSR → WASM).
//!
//! Görünüm onaylı tasarımın birebir karşılığıdır: bileşen sınıfları `design/screens/screens.css`,
//! değerler `design/tokens.json`'dan üretilen CSS değişkenleri, ikonlar tasarım ekranlarındaki
//! `<symbol>`'ler (derleme sırasında gömülür; bkz. build.rs). İş mantığı burada yoktur.

#![forbid(unsafe_code)]

pub mod backend;
mod composer;
pub mod demo;
mod screens;
mod sheets;
pub mod view;
mod web;

use leptos::prelude::*;
use leptos::task::spawn_local;
use notai_api::{EntryView, TaskRowView, TopicRef};

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
    /// Haftalık gözden geçirme.
    Review,
    Settings,
}

/// Paylaşılan durum (bağlam).
#[derive(Clone, Copy)]
struct Ctx {
    screen: RwSignal<Screen>,
    /// Her yazmadan sonra artar; veri kaynakları bunu izleyip yeniden yükler.
    version: RwSignal<u32>,
    /// Detay sayfası açık olan kayıt.
    selected: RwSignal<Option<EntryView>>,
    /// Düzenleme sayfası açık olan görev.
    editing: RwSignal<Option<TaskRowView>>,
    /// Kullanıcıya gösterilecek kısa hata.
    error: RwSignal<Option<String>>,
    /// Görevler ekranı açılınca ekleme alanına odaklan (ön metinle).
    task_draft: RwSignal<Option<String>>,
    /// Ana ekrandaki "Yeni kayıt" düğmesiyle gelindi: akışın yazma alanına odaklan.
    compose_focus: RwSignal<bool>,
}

fn new_ctx() -> Ctx {
    Ctx {
        screen: RwSignal::new(Screen::Home),
        version: RwSignal::new(0),
        selected: RwSignal::new(None),
        editing: RwSignal::new(None),
        error: RwSignal::new(None),
        task_draft: RwSignal::new(None),
        compose_focus: RwSignal::new(false),
    }
}

fn ctx() -> Ctx {
    use_context::<Ctx>().unwrap_or_else(new_ctx)
}

fn backend() -> Backend {
    use_context::<Backend>().unwrap_or_else(Backend::demo)
}

/// Arka uca yazıp başarıda verileri yeniler ve `done`'ı çağırır; hatada mesaj gösterir.
/// Bağlam (`use_context`) yalnız olay işleyicisinde/bileşende okunur; `await` sonrası okunamaz.
fn run_then<F, Fut, T, D>(f: F, done: D)
where
    F: FnOnce(Backend) -> Fut + 'static,
    Fut: std::future::Future<Output = Result<T, String>> + 'static,
    T: 'static,
    D: FnOnce(T) + 'static,
{
    run_in(ctx(), backend(), f, done);
}

/// `run_then`'in bağlamı önceden alınmış hâli (zaman uyumsuz akışın içinden çağırmak için).
fn run_in<F, Fut, T, D>(c: Ctx, b: Backend, f: F, done: D)
where
    F: FnOnce(Backend) -> Fut + 'static,
    Fut: std::future::Future<Output = Result<T, String>> + 'static,
    T: 'static,
    D: FnOnce(T) + 'static,
{
    spawn_local(async move {
        match f(b).await {
            Ok(v) => {
                c.error.set(None);
                c.version.update(|v| *v += 1);
                done(v);
            }
            Err(e) => c.error.set(Some(e)),
        }
    });
}

/// Arka uca yazıp başarıda verileri yeniler; hatada mesaj gösterir.
fn run<F, Fut>(f: F)
where
    F: FnOnce(Backend) -> Fut + 'static,
    Fut: std::future::Future<Output = Result<(), String>> + 'static,
{
    run_then(f, |()| {});
}

/// Yuvarlak konu profili: fotoğraf varsa o, yoksa konunun renginde daire içinde simge.
#[component]
fn Avatar(look: view::TopicLook, #[prop(optional)] small: bool) -> impl IntoView {
    let class = look.avatar_class(small);
    match look.avatar {
        Some(src) => view! { <span class=class><img src=src alt=""/></span> }.into_any(),
        None => view! { <span class=class><Icon name=look.icon/></span> }.into_any(),
    }
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
    provide_context(new_ctx());
    provide_context(Backend::detect());
    web::apply_theme(web::load_theme());
    let c = ctx();
    // Ekran değişince eski hata mesajı ve açık sayfalar kapanır.
    Effect::new(move |_| {
        let _ = c.screen.get();
        c.error.set(None);
        c.selected.set(None);
        c.editing.set(None);
    });
    view! {
        <style>{TOKENS_CSS}{SCREENS_CSS}{APP_CSS}</style>
        <div inner_html=SPRITE></div>
        <div class="app phone">
            {move || match c.screen.get() {
                Screen::Home => view! { <screens::home::Home/> }.into_any(),
                Screen::Flow(topic) => view! { <screens::flow::Flow topic=topic/> }.into_any(),
                Screen::Tasks => view! { <screens::tasks::Tasks/> }.into_any(),
                Screen::Calendar => view! { <screens::calendar::Calendar/> }.into_any(),
                Screen::Reminders => view! { <screens::reminders::Reminders/> }.into_any(),
                Screen::Search => view! { <screens::search::Search/> }.into_any(),
                Screen::Review => view! { <screens::review::Review/> }.into_any(),
                Screen::Settings => view! { <screens::settings::Settings/> }.into_any(),
            }}
            <sheets::EntrySheet/>
            <sheets::TaskSheet/>
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

/// Son işlemin hatası (dokununca kapanır).
#[component]
fn ErrorNote() -> impl IntoView {
    let c = ctx();
    move || {
        c.error.get().map(|e| {
            view! {
                <p class="error-note" role="alert" on:click=move |_| c.error.set(None)>{e}</p>
            }
        })
    }
}

/// Yükleniyor / hata / içerik üçlüsünün ilk ikisi.
fn loading() -> AnyView {
    view! { <p class="loading">"Yükleniyor…"</p> }.into_any()
}

fn failed(e: String) -> AnyView {
    view! { <p class="error-note" role="alert">{e}</p> }.into_any()
}
