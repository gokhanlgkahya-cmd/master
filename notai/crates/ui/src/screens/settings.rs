//! Ayarlar: görünüm, ev sunucusu, yapay zekâ, seni tanıma, çöp kutusu (13-ayarlar.html).

use leptos::prelude::*;
use leptos::task::spawn_local;
use notai_api::{FactView, SettingsView, TrashItemView};

use crate::backend::Backend;
use crate::view::{self, Theme};
use crate::{Icon, Screen, backend, ctx, web};

type Msg = RwSignal<Option<Result<String, String>>>;

/// Bölüm içi işlem: sonucu (başarı metni ya da hata) bölümün altında gösterir.
fn act<F, Fut, T>(msg: Msg, f: F, ok: impl FnOnce(T) -> String + 'static)
where
    F: FnOnce(Backend) -> Fut + 'static,
    Fut: std::future::Future<Output = Result<T, String>> + 'static,
    T: 'static,
{
    let c = ctx();
    let b = backend();
    msg.set(None);
    spawn_local(async move {
        match f(b).await {
            Ok(v) => {
                c.version.update(|v| *v += 1);
                msg.set(Some(Ok(ok(v))));
            }
            Err(e) => msg.set(Some(Err(e))),
        }
    });
}

fn msg_view(msg: Msg) -> impl IntoView {
    move || {
        msg.get().map(|m| match m {
            Ok(t) => view! { <p class="setnote setnote--ok" role="status">{t}</p> }.into_any(),
            Err(e) => view! { <p class="setnote setnote--error" role="alert">{e}</p> }.into_any(),
        })
    }
}

#[component]
pub fn Settings() -> impl IntoView {
    let c = ctx();
    view! {
        <header class="flowbar">
            <button class="iconbtn" aria-label="Geri" on:click=move |_| c.screen.set(Screen::Home)><Icon name="back"/></button>
            <div class="flowbar__who"><div class="flowbar__text"><span class="flowbar__name">"Ayarlar"</span></div></div>
        </header>
        <div class="body scroll">
            <Appearance/>
            <SyncSection/>
            <AiSection/>
            <Facts/>
            <Trash/>
            <p class="setnote">"Silinenler önce çöp kutusuna gider; geri alınabilir. Kalıcı silme geri alınamaz."</p>
        </div>
    }
}

#[component]
fn Appearance() -> impl IntoView {
    let theme = RwSignal::new(web::load_theme());
    let opt = move |t: Theme, icon: &'static str, label: &'static str| {
        view! {
            <button role="tab" aria-selected=move || (theme.get() == t).to_string()
                on:click=move |_| { theme.set(t); web::save_theme(t); }>
                <Icon name=icon size="sm"/>{label}
            </button>
        }
    };
    view! {
        <h2 class="grouphead">"Görünüm"</h2>
        <div class="card setgroup">
            <div class="setrow setrow--stack">
                <div class="seg3" role="tablist" aria-label="Tema">
                    {opt(Theme::System, "phone", "Sistem")}
                    {opt(Theme::Light, "sun", "Açık")}
                    {opt(Theme::Dark, "moon", "Koyu")}
                </div>
            </div>
        </div>
    }
}

#[component]
fn SyncSection() -> impl IntoView {
    let b = backend();
    let url = RwSignal::new(String::new());
    let token = RwSignal::new(String::new());
    let msg: Msg = RwSignal::new(None);
    let save = move |_| {
        let (u, t) = (url.get_untracked(), token.get_untracked());
        act(
            msg,
            move |b| async move { b.set_sync(u, t).await },
            |()| "Sunucu ayarı kaydedildi.".to_owned(),
        );
    };
    let sync = move |_| act(msg, |b| async move { b.sync_now().await }, |s| s);
    view! {
        <h2 class="grouphead">"Ev sunucusu"</h2>
        <div class="card setgroup">
            <div class="setrow">
                <span class="setrow__ic"><Icon name="server"/></span>
                <span class="setrow__txt">
                    <span class="setrow__title">"Bağlantı"</span>
                    <span class="setrow__sub">{b.describe()}</span>
                </span>
            </div>
            <div class="setrow setrow--stack">
                <label class="formfield">
                    <span class="formfield__label">"Sunucu adresi"</span>
                    <span class="input"><input type="url" inputmode="url" placeholder="http://100.64.0.2:8787"
                        prop:value=move || url.get() on:input=move |ev| url.set(event_target_value(&ev))/></span>
                </label>
                <label class="formfield">
                    <span class="formfield__label">"Belirteç"</span>
                    <span class="input"><input type="password" autocomplete="off" placeholder="NOTAI_API_TOKEN"
                        prop:value=move || token.get() on:input=move |ev| token.set(event_target_value(&ev))/></span>
                </label>
                <div class="setrow__acts">
                    <button class="btn btn--outline" on:click=save>"Kaydet"</button>
                    <button class="btn btn--outline" on:click=sync><Icon name="sync" size="sm"/>"Şimdi senkronla"</button>
                </div>
                {msg_view(msg)}
            </div>
        </div>
    }
}

#[component]
fn AiSection() -> impl IntoView {
    let c = ctx();
    let msg: Msg = RwSignal::new(None);
    let data = LocalResource::new(move || {
        let _ = c.version.get();
        let b = backend();
        async move { b.settings().await }
    });
    view! {
        <h2 class="grouphead">"Yapay zekâ"</h2>
        <div class="card setgroup">
            {move || match data.get() {
                None => view! { <p class="note">"Yükleniyor…"</p> }.into_any(),
                Some(Err(e)) => view! { <p class="setnote setnote--error" role="alert">{e}</p> }.into_any(),
                Some(Ok(s)) => ai_rows(s, msg).into_any(),
            }}
            {msg_view(msg)}
        </div>
    }
}

fn ai_rows(s: SettingsView, msg: Msg) -> impl IntoView {
    let on = s.autonomous;
    let switch_class = if on { "switch switch--on" } else { "switch" };
    let toggle = move |_| {
        act(
            msg,
            move |b| async move { b.set_settings(Some(!on), None).await },
            move |()| {
                if on {
                    "Otonom mod kapatıldı.".to_owned()
                } else {
                    "Otonom mod açıldı; uygulananlar not detayında geri alınabilir.".to_owned()
                }
            },
        );
    };
    let set_threshold = move |ev: leptos::ev::Event| {
        if let Some(t) = view::threshold_from_input(&event_target_value(&ev)) {
            act(
                msg,
                move |b| async move { b.set_settings(None, Some(t)).await },
                move |()| format!("Eşik {} olarak kaydedildi.", view::threshold_label(t)),
            );
        }
    };
    let pct = format!("{}", (s.threshold * 100.0).round());
    let flag = |v: bool| {
        if v {
            view! { <span class="tag tag--ok"><span class="dotstate"></span>"Açık"</span> }
                .into_any()
        } else {
            view! { <span class="tag">"Kapalı"</span> }.into_any()
        }
    };
    view! {
        <div class="setrow">
            <span class="setrow__ic"><Icon name="cpu"/></span>
            <span class="setrow__txt"><span class="setrow__title">"Sağlayıcı"</span><span class="setrow__sub">"Sunucuda çalışan model"</span></span>
            <span class="setrow__val">{s.ai_provider.clone()}</span>
        </div>
        <div class="setrow">
            <span class="setrow__ic"><Icon name="bolt"/></span>
            <span class="setrow__txt">
                <span class="setrow__title">"Otonom mod"</span>
                <span class="setrow__sub">"Yüksek güvenli önerileri kendisi uygular; geri alınabilir"</span>
            </span>
            <button class=switch_class role="switch" aria-checked=on.to_string() aria-label="Otonom mod" on:click=toggle></button>
        </div>
        <div class="setrow setrow--stack">
            <label class="formfield">
                <span class="formfield__label">{format!("Uygulama eşiği · {}", view::threshold_label(s.threshold))}</span>
                <input class="range" type="range" min="50" max="100" step="5" prop:value=pct on:change=set_threshold/>
            </label>
        </div>
        <div class="setrow">
            <span class="setrow__ic"><Icon name="mic"/></span>
            <span class="setrow__txt"><span class="setrow__title">"Sesli yakalama"</span></span>
            {flag(s.voice_enabled)}
        </div>
        <div class="setrow">
            <span class="setrow__ic"><Icon name="search"/></span>
            <span class="setrow__txt"><span class="setrow__title">"Anlamsal arama"</span></span>
            {flag(s.semantic_search)}
        </div>
    }
}

#[component]
fn Facts() -> impl IntoView {
    let c = ctx();
    let msg: Msg = RwSignal::new(None);
    let text = RwSignal::new(String::new());
    let data = LocalResource::new(move || {
        let _ = c.version.get();
        let b = backend();
        async move { b.facts().await }
    });
    let add = move || {
        let t = text.get_untracked().trim().to_owned();
        if t.is_empty() {
            return;
        }
        text.set(String::new());
        act(
            msg,
            move |b| async move { b.add_fact(t).await },
            |()| "Bilgi eklendi.".to_owned(),
        );
    };
    let count = move || match data.get() {
        Some(Ok(f)) => format!("{}\u{a0}bilgi", f.len()),
        _ => String::new(),
    };
    view! {
        <h2 class="grouphead">"Seni tanıma"<span class="count">{count}</span></h2>
        <div class="card setgroup">
            {move || match data.get() {
                None => view! { <p class="note">"Yükleniyor…"</p> }.into_any(),
                Some(Err(e)) => view! { <p class="setnote setnote--error" role="alert">{e}</p> }.into_any(),
                Some(Ok(f)) if f.is_empty() => view! { <p class="note">"Henüz bilgi yok."</p> }.into_any(),
                Some(Ok(f)) => f.into_iter().map(|x| fact_row(x, msg)).collect_view().into_any(),
            }}
            <form class="setrow" on:submit=move |ev| { ev.prevent_default(); add() }>
                <span class="input input__grow"><input type="text" lang="tr" aria-label="Yeni bilgi"
                    placeholder="Örn. GemOlive = sabun markam"
                    prop:value=move || text.get() on:input=move |ev| text.set(event_target_value(&ev))/></span>
                <button class="btn btn--outline" type="submit">"Ekle"</button>
            </form>
            {msg_view(msg)}
        </div>
        <p class="setnote">"Bu bilgiler yapay zekânın kayıtlarını sınıflandırmasına yardım eder."</p>
    }
}

fn fact_row(f: FactView, msg: Msg) -> impl IntoView {
    let (a, b) = (f.id.clone(), f.id.clone());
    let confirm = (!f.confirmed).then(|| {
        view! {
            <span class="tag tag--info">"Öneri"</span>
            <button class="iconbtn" aria-label=format!("Onayla: {}", f.text)
                on:click=move |_| { let id = a.clone(); act(msg, move |bk| async move { bk.fact_action(id, "confirm").await }, |()| "Bilgi onaylandı.".to_owned()) }>
                <Icon name="check"/>
            </button>
        }
    });
    view! {
        <div class="fact">
            <span class="fact__txt">{f.text.clone()}</span>
            {confirm}
            <button class="iconbtn" aria-label=format!("Sil: {}", f.text)
                on:click=move |_| { let id = b.clone(); act(msg, move |bk| async move { bk.fact_action(id, "delete").await }, |()| "Bilgi silindi.".to_owned()) }>
                <Icon name="trash"/>
            </button>
        </div>
    }
}

#[component]
fn Trash() -> impl IntoView {
    let c = ctx();
    let msg: Msg = RwSignal::new(None);
    let confirming = RwSignal::new(None::<String>);
    let data = LocalResource::new(move || {
        let _ = c.version.get();
        let b = backend();
        async move { b.trash().await }
    });
    let count = move || match data.get() {
        Some(Ok(t)) => format!("{}\u{a0}kayıt", t.len()),
        _ => String::new(),
    };
    view! {
        <h2 class="grouphead">"Çöp kutusu"<span class="count">{count}</span></h2>
        <div class="card setgroup">
            {move || match data.get() {
                None => view! { <p class="note">"Yükleniyor…"</p> }.into_any(),
                Some(Err(e)) => view! { <p class="setnote setnote--error" role="alert">{e}</p> }.into_any(),
                Some(Ok(t)) if t.is_empty() => view! { <p class="note">"Çöp kutusu boş."</p> }.into_any(),
                Some(Ok(t)) => t.into_iter().map(|x| trash_row(x, msg, confirming)).collect_view().into_any(),
            }}
            {msg_view(msg)}
        </div>
    }
}

fn trash_row(t: TrashItemView, msg: Msg, confirming: RwSignal<Option<String>>) -> impl IntoView {
    let id = t.id.clone();
    let title = view::first_line(&t.text);
    let acts = move || {
        let id = id.clone();
        if confirming.get().as_deref() == Some(id.as_str()) {
            view! {
                <span class="setrow__sub setrow__ask">"Kalıcı silinsin mi? Geri alınamaz."</span>
                <button class="btn btn--text" on:click=move |_| confirming.set(None)>"Vazgeç"</button>
                <button class="btn btn--danger" on:click=move |_| {
                    let id = id.clone();
                    confirming.set(None);
                    act(msg, move |b| async move { b.purge(id).await }, |()| "Kayıt kalıcı olarak silindi.".to_owned());
                }>"Sil"</button>
            }
            .into_any()
        } else {
            let (a, b) = (id.clone(), id);
            view! {
                <button class="btn btn--outline" on:click=move |_| {
                    let id = a.clone();
                    act(msg, move |bk| async move { bk.entry_action(id, "restore").await }, |()| "Kayıt geri alındı.".to_owned());
                }>"Geri al"</button>
                <button class="btn btn--text btn--danger-text" on:click=move |_| confirming.set(Some(b.clone()))>"Kalıcı sil"</button>
            }
            .into_any()
        }
    };
    view! {
        <div class="setrow setrow--stack">
            <span class="setrow__txt">
                <span class="setrow__title">{title}</span>
                <span class="setrow__sub">{t.note.clone()}</span>
            </span>
            <div class="setrow__acts">{acts}</div>
        </div>
    }
}
