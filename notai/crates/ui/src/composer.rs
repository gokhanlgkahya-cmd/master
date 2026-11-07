//! Yazma çubuğu + ekleme (ataç) paneli + ses şeridi
//! (guidelines §3; 05-ekleme-paneli.html, 06-sesli-yakalama.html).

use std::time::Duration;

use leptos::html::Input;
use leptos::prelude::*;
use leptos::task::spawn_local;
use notai_api::TaskEditRequest;

use crate::web::{self, Recorder};
use crate::{Icon, Screen, backend, ctx, run_in, view};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Panel {
    Closed,
    Grid,
    Contact,
    Timed(Timed),
    List,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Voice {
    Idle,
    Starting,
    Recording,
    Sending,
}

/// Ses şeridindeki sabit dalga çubukları (yükseklik birimi `--space-1` katı).
const WAVE: [u8; 13] = [2, 4, 7, 4, 8, 5, 3, 6, 8, 5, 3, 4, 2];

/// Altta sabit yazma çubuğu: 1 dokunuşla yakalama. Boşken mikrofon, doluyken Gönder.
/// `task_mode`: Görevler ekranı ("Görev ekle…"; Enter görev oluşturur, ataç yerine +).
#[component]
pub fn Composer(
    #[prop(into)] placeholder: String,
    topic: Option<String>,
    #[prop(optional)] task_mode: bool,
) -> impl IntoView {
    let c = ctx();
    let b = backend();
    let text = RwSignal::new(String::new());
    let panel = RwSignal::new(Panel::Closed);
    let voice = RwSignal::new(Voice::Idle);
    let secs = RwSignal::new(0_u32);
    let recorder = StoredValue::new_local(None::<Recorder>);
    let ticker = StoredValue::new(None::<IntervalHandle>);
    let field = NodeRef::<Input>::new();
    let photo = NodeRef::<Input>::new();
    let camera = NodeRef::<Input>::new();
    let document = NodeRef::<Input>::new();

    if task_mode {
        // Ataç → Görev ile gelindiyse ekleme alanına odaklan.
        Effect::new(move |_| {
            if let Some(d) = c.task_draft.get() {
                text.set(d);
                c.task_draft.set(None);
                web::focus_later(field);
            }
        });
    }

    let submit = {
        let (b, topic) = (b.clone(), topic.clone());
        move || {
            let t = text.get_untracked().trim().to_owned();
            if t.is_empty() {
                return;
            }
            text.set(String::new());
            let topic = topic.clone();
            if task_mode {
                run_in(
                    c,
                    b.clone(),
                    move |b| async move { b.create_task(t, None, topic).await.map(|_| ()) },
                    |()| {},
                );
            } else {
                run_in(
                    c,
                    b.clone(),
                    move |b| async move { b.capture(t, topic).await },
                    |()| {},
                );
            }
        }
    };

    // Ek: önce bir kayıt yakala (yazılmış metin ya da "Fotoğraf: ad.jpg"), sonra eki yükle.
    let attach = {
        let (b, topic) = (b.clone(), topic.clone());
        move |kind: &'static str,
              name: String,
              mime: String,
              bytes: Vec<u8>,
              value: Option<String>| {
            panel.set(Panel::Closed);
            // Önizlemede ek saklanamaz: eksiz, yarım bir kayıt bırakmadan baştan söyle.
            if !b.supports_attachments() {
                c.error.set(Some(crate::backend::NOT_IN_DEMO.to_owned()));
                return;
            }
            let draft = text.get_untracked();
            let typed = !draft.trim().is_empty();
            let entry_text = if typed {
                draft.trim().to_owned()
            } else {
                view::attach_entry_text(kind, &name)
            };
            text.set(String::new());
            let topic = topic.clone();
            run_in(
                c,
                b.clone(),
                move |b| async move {
                    let id = b.capture_id(entry_text, topic).await?;
                    match b
                        .upload_attachment(id.clone(), kind, name, mime, bytes, value)
                        .await
                    {
                        Ok(()) => Ok(()),
                        Err(e) => {
                            // Yalnız ek için açılmış kayıt ("Fotoğraf: …") eksiz kalmasın: çöpe
                            // (geri alınabilir). Kullanıcının yazdığı metin korunur.
                            if !typed {
                                let _ = b.entry_action(id, "trash").await;
                            }
                            Err(format!("Ek yüklenemedi: {e}"))
                        }
                    }
                },
                |()| {},
            );
        }
    };

    let on_file = {
        let attach = attach.clone();
        move |kind: &'static str, input: NodeRef<Input>| {
            let attach = attach.clone();
            move |_ev: leptos::ev::Event| {
                let Some(el) = input.get_untracked() else {
                    return;
                };
                let attach = attach.clone();
                spawn_local(async move {
                    match web::read_first_file(&el).await {
                        Ok(Some(p)) => {
                            el.set_value("");
                            attach(kind, p.name, p.mime, p.bytes, None);
                        }
                        Ok(None) => {}
                        Err(e) => c.error.set(Some(e)),
                    }
                });
            }
        }
    };

    let pick = move |input: NodeRef<Input>| {
        if let Some(el) = input.get_untracked() {
            el.click();
        }
    };

    let locate = {
        let attach = attach.clone();
        move || {
            let attach = attach.clone();
            panel.set(Panel::Closed);
            spawn_local(async move {
                match web::current_position().await {
                    Ok((lat, lon)) => attach(
                        "location",
                        "Konum".to_owned(),
                        "text/plain".to_owned(),
                        Vec::new(),
                        Some(view::location_value(lat, lon)),
                    ),
                    Err(e) => c.error.set(Some(e)),
                }
            });
        }
    };

    // ---- Ses ----
    let stop_ticker = move || {
        ticker.update_value(|t| {
            if let Some(h) = t.take() {
                h.clear();
            }
        });
    };
    let start_voice = move || {
        panel.set(Panel::Closed);
        voice.set(Voice::Starting);
        secs.set(0);
        spawn_local(async move {
            match Recorder::start().await {
                Ok(r) => {
                    recorder.set_value(Some(r));
                    voice.set(Voice::Recording);
                    let h = set_interval_with_handle(
                        move || secs.update(|s| *s += 1),
                        Duration::from_secs(1),
                    )
                    .ok();
                    ticker.set_value(h);
                }
                Err(e) => {
                    voice.set(Voice::Idle);
                    c.error.set(Some(e));
                }
            }
        });
    };
    let cancel_voice = move || {
        stop_ticker();
        let mut r = None;
        recorder.update_value(|x| r = x.take());
        if let Some(r) = r {
            r.cancel();
        }
        voice.set(Voice::Idle);
    };
    let send_voice = {
        let (b, topic) = (b.clone(), topic.clone());
        move || {
            stop_ticker();
            let mut r = None;
            recorder.update_value(|x| r = x.take());
            let Some(r) = r else {
                voice.set(Voice::Idle);
                return;
            };
            voice.set(Voice::Sending);
            let (b, topic) = (b.clone(), topic.clone());
            spawn_local(async move {
                match r.finish().await {
                    Ok((bytes, mime)) => run_in(
                        c,
                        b,
                        move |b| async move {
                            let r = b.voice(bytes, mime, topic).await;
                            voice.set(Voice::Idle);
                            r.map_err(|e| format!("Ses kaydı gönderilemedi: {e}"))
                        },
                        |()| {},
                    ),
                    Err(e) => {
                        voice.set(Voice::Idle);
                        c.error.set(Some(e));
                    }
                }
            });
        }
    };
    on_cleanup(move || {
        stop_ticker();
        let mut r = None;
        recorder.update_value(|x| r = x.take());
        if let Some(r) = r {
            r.cancel();
        }
    });

    let bar = {
        let submit = submit.clone();
        let placeholder = placeholder.clone();
        move || {
            let submit = submit.clone();
            let submit2 = submit.clone();
            let lead = if task_mode {
                view! {
                    <button class="iconbtn" aria-label="Görev ekle" on:click=move |_| web::focus_later(field)><Icon name="plus"/></button>
                }
                .into_any()
            } else {
                view! {
                    <button class="iconbtn" aria-expanded=move || (panel.get() != Panel::Closed).to_string()
                        aria-label=move || if panel.get() == Panel::Closed { "Ekle" } else { "Ekleme panelini kapat" }
                        on:click=move |_| panel.update(|p| *p = if *p == Panel::Closed { Panel::Grid } else { Panel::Closed })>
                        {move || if panel.get() == Panel::Closed { view! { <Icon name="clip"/> } } else { view! { <Icon name="x"/> } }}
                    </button>
                }
                .into_any()
            };
            let cam = (!task_mode).then(|| {
                view! { <button class="iconbtn" aria-label="Kamera" on:click=move |_| pick(camera)><Icon name="camera"/></button> }
            });
            view! {
                <div class="composer">
                    {lead}
                    <input
                        class="field"
                        type="text"
                        lang="tr"
                        enterkeyhint="send"
                        node_ref=field
                        aria-label=placeholder.clone()
                        placeholder=placeholder.clone()
                        prop:value=move || text.get()
                        on:focus=move |_| panel.set(Panel::Closed)
                        on:input=move |ev| text.set(event_target_value(&ev))
                        on:keydown=move |ev| if ev.key() == "Enter" { submit() }
                    />
                    {cam}
                    {move || if text.get().trim().is_empty() {
                        view! { <button class="mic" aria-label="Sesli yakala" on:click=move |_| start_voice()><Icon name="mic"/></button> }.into_any()
                    } else {
                        let s = submit2.clone();
                        view! { <button class="mic" aria-label="Gönder" on:click=move |_| s()><Icon name="send"/></button> }.into_any()
                    }}
                </div>
            }
        }
    };

    let strip = move || {
        let cancel = cancel_voice;
        let send = send_voice.clone();
        let hint = move || match voice.get() {
            Voice::Starting => "Mikrofon açılıyor…",
            Voice::Sending => "Yazıya çevriliyor…",
            _ => "Kaydediliyor",
        };
        let live = move || match voice.get() {
            Voice::Starting => "Mikrofon izni bekleniyor…",
            Voice::Sending => "Kayıt gönderiliyor; metin akışa düşecek.",
            _ => "Konuş; bitince Gönder’e dokun. Kayıt yazıya çevrilip kaydedilir.",
        };
        let busy = move || voice.get() != Voice::Recording;
        let bars = WAVE
            .iter()
            .map(|h| view! { <i style=format!("--h:{h}")></i> })
            .collect_view();
        view! {
            <section class="voicestrip" aria-label="Sesli yakalama">
                <div class="voicestrip__head">
                    <span class="wave" class:wave--live=move || voice.get() == Voice::Recording aria-hidden="true">{bars}</span>
                    <span class="voicestrip__time" aria-label="Kayıt süresi">{move || view::clock(secs.get())}</span>
                    <span class="voicestrip__hint">{hint}</span>
                </div>
                <p class="live" aria-live="polite">{live}</p>
                <div class="voicestrip__controls">
                    <button class="btn btn--text" on:click=move |_| cancel() disabled=move || voice.get() == Voice::Sending>"İptal"</button>
                    <button class="btn btn--primary voicestrip__send" on:click=move |_| send() disabled=busy>
                        <Icon name="send" size="sm"/>"Gönder"
                    </button>
                </div>
            </section>
        }
    };

    let tile = move |icon: &'static str, label: &'static str, f: Box<dyn Fn() + Send + Sync>| {
        view! {
            <button class="tile" on:click=move |_| f()>
                <span class="tile__box"><Icon name=icon/></span><span>{label}</span>
            </button>
        }
    };

    let attach_panel = {
        let locate = locate.clone();
        let attach = attach.clone();
        let b = b.clone();
        let topic = topic.clone();
        move || {
            let locate = locate.clone();
            match panel.get() {
                Panel::Closed => ().into_any(),
                Panel::Grid => view! {
                    <section class="attach" aria-label="Ekleme paneli">
                        <div class="attach__group">
                            <h2 class="attach__label">"Ekle"</h2>
                            <div class="attach__grid">
                                {tile("image", "Fotoğraf", Box::new(move || pick(photo)))}
                                {tile("camera", "Kamera", Box::new(move || pick(camera)))}
                                {tile("file", "Belge", Box::new(move || pick(document)))}
                                {tile("pin", "Konum", Box::new(locate))}
                                {tile("user", "Kişi", Box::new(move || panel.set(Panel::Contact)))}
                            </div>
                        </div>
                        <div class="attach__group">
                            <h2 class="attach__label">"Oluştur"</h2>
                            <div class="attach__grid">
                                {tile("task", "Görev", Box::new(move || {
                                    let d = text.get_untracked();
                                    text.set(String::new());
                                    c.task_draft.set(Some(d));
                                    c.screen.set(Screen::Tasks);
                                }))}
                                {tile("bell", "Hatırlatma", Box::new(move || panel.set(Panel::Timed(Timed::Reminder))))}
                                {tile("calendar", "Etkinlik", Box::new(move || panel.set(Panel::Timed(Timed::Event))))}
                                {tile("list", "Liste", Box::new(move || panel.set(Panel::List)))}
                            </div>
                        </div>
                    </section>
                }.into_any(),
                Panel::Contact => {
                    let attach = attach.clone();
                    view! { <ContactForm panel=panel on_done=move |name: String| {
                        attach("contact", name.clone(), "text/plain".to_owned(), Vec::new(), Some(name));
                    }/> }.into_any()
                }
                Panel::Timed(kind) => view! { <TimedForm panel=panel topic=topic.clone() draft=text kind=kind/> }.into_any(),
                Panel::List => {
                    let (b, topic) = (b.clone(), topic.clone());
                    view! { <ListForm panel=panel draft=text on_done=move |t: String| {
                        let topic = topic.clone();
                        run_in(c, b.clone(), move |b| async move { b.capture(t, topic).await }, |()| {});
                    }/> }.into_any()
                }
            }
        }
    };

    view! {
        <div class="filepick" aria-hidden="true">
            <input type="file" accept="image/*" tabindex="-1" node_ref=photo on:change=on_file("photo", photo)/>
            <input type="file" accept="image/*" capture="environment" tabindex="-1" node_ref=camera on:change=on_file("photo", camera)/>
            <input type="file" accept="*/*" tabindex="-1" node_ref=document on:change=on_file("document", document)/>
        </div>
        {move || if voice.get() == Voice::Idle { bar().into_any() } else { strip().into_any() }}
        {attach_panel}
    }
}

/// Ekleme paneli formu: başlık + içerik + Vazgeç/Ekle.
fn panel_form(
    title: &'static str,
    panel: RwSignal<Panel>,
    submit: impl Fn() + 'static,
    body: impl IntoView + 'static,
    action: &'static str,
) -> impl IntoView {
    view! {
        <form class="attach attach--form" aria-label=title on:submit=move |ev| { ev.prevent_default(); submit() }>
            <h2 class="attach__label">{title}</h2>
            {body}
            <div class="attach__acts">
                <button class="btn btn--text" type="button" on:click=move |_| panel.set(Panel::Grid)>"Vazgeç"</button>
                <button class="btn btn--outline" type="submit">{action}</button>
            </div>
        </form>
    }
}

#[component]
fn ContactForm(panel: RwSignal<Panel>, on_done: impl Fn(String) + 'static) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let input = NodeRef::<Input>::new();
    web::focus_later(input);
    let submit = move || {
        let n = name.get_untracked().trim().to_owned();
        if !n.is_empty() {
            on_done(n);
        }
    };
    let body = view! {
        <label class="formfield">
            <span class="formfield__label">"Kişinin adı"</span>
            <span class="input"><Icon name="user" size="sm"/><input type="text" lang="tr" node_ref=input autocomplete="name"
                placeholder="Örn. Ayhan Kimya – Selin Hanım"
                prop:value=move || name.get() on:input=move |ev| name.set(event_target_value(&ev))/></span>
        </label>
    };
    panel_form("Kişi ekle", panel, submit, body, "Ekle")
}

#[component]
fn ListForm(
    panel: RwSignal<Panel>,
    draft: RwSignal<String>,
    on_done: impl Fn(String) + 'static,
) -> impl IntoView {
    let raw = RwSignal::new(draft.get_untracked());
    let submit = move || {
        if let Some(t) = view::list_text(&raw.get_untracked()) {
            draft.set(String::new());
            panel.set(Panel::Closed);
            on_done(t);
        }
    };
    let body = view! {
        <label class="formfield">
            <span class="formfield__label">"Her satıra bir madde"</span>
            <textarea class="textarea" lang="tr" rows="5" placeholder="Süt\nYumurta\nZeytinyağı"
                prop:value=move || raw.get() on:input=move |ev| raw.set(event_target_value(&ev))></textarea>
        </label>
    };
    panel_form("Liste", panel, submit, body, "Kaydet")
}

/// Saatli iki form: Hatırlatma (varsayılan tam zamanında) ve Etkinlik (takvimde saatli kayıt,
/// varsayılan 15 dk önce hatırlatır). İkisi de tarih + saatli bir görev oluşturur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Timed {
    Reminder,
    Event,
}

impl Timed {
    const fn title(self) -> &'static str {
        match self {
            Self::Reminder => "Hatırlatma",
            Self::Event => "Etkinlik",
        }
    }
    const fn question(self) -> &'static str {
        match self {
            Self::Reminder => "Ne hatırlatılsın?",
            Self::Event => "Etkinlik adı",
        }
    }
    const fn placeholder(self) -> &'static str {
        match self {
            Self::Reminder => "Örn. Kombi servisini ara",
            Self::Event => "Örn. Tedarikçiyle toplantı",
        }
    }
    const fn default_remind(self) -> i32 {
        match self {
            Self::Reminder => 0,
            Self::Event => 15,
        }
    }
    const fn submit(self) -> &'static str {
        match self {
            Self::Reminder => "Kur",
            Self::Event => "Ekle",
        }
    }
}

#[component]
fn TimedForm(
    panel: RwSignal<Panel>,
    topic: Option<String>,
    draft: RwSignal<String>,
    kind: Timed,
) -> impl IntoView {
    let c = ctx();
    let b = backend();
    let title = RwSignal::new(draft.get_untracked().trim().to_owned());
    let date = RwSignal::new(web::today().format("%Y-%m-%d").to_string());
    let time = RwSignal::new(String::from("09:00"));
    let remind = RwSignal::new(kind.default_remind().to_string());
    let input = NodeRef::<Input>::new();
    web::focus_later(input);
    let submit = move || {
        let t = title.get_untracked().trim().to_owned();
        let (d, tm) = (date.get_untracked(), time.get_untracked());
        let minutes = view::timed_remind(kind == Timed::Reminder, &remind.get_untracked());
        if t.is_empty() || d.is_empty() || tm.is_empty() {
            return;
        }
        draft.set(String::new());
        panel.set(Panel::Closed);
        let topic = topic.clone();
        run_in(
            c,
            b.clone(),
            move |b| async move {
                let id = b.create_task(t, Some(d), topic).await?;
                let req = TaskEditRequest {
                    due_time: Some(tm),
                    remind_before_minutes: minutes,
                    ..TaskEditRequest::default()
                };
                b.edit_task(id, req).await
            },
            |()| {},
        );
    };
    // Hatırlatma formunda "Yok" anlamsız: yalnız zamanlı seçenekler.
    let options = view::REMIND_OPTIONS
        .iter()
        .filter(|(m, _)| kind == Timed::Event || *m >= 0)
        .map(|&(m, l)| {
            let v = m.to_string();
            let v2 = v.clone();
            view! { <option value=v selected=move || remind.get() == v2>{l}</option> }
        })
        .collect_view();
    let body = view! {
        <label class="formfield">
            <span class="formfield__label">{kind.question()}</span>
            <span class="input"><input type="text" lang="tr" node_ref=input placeholder=kind.placeholder()
                prop:value=move || title.get() on:input=move |ev| title.set(event_target_value(&ev))/></span>
        </label>
        <div class="fields">
            <label class="fieldbox">
                <span class="fieldbox__label">"Tarih"</span>
                <span class="fieldbox__val"><input type="date" aria-label="Tarih" prop:value=move || date.get()
                    on:input=move |ev| date.set(event_target_value(&ev))/></span>
            </label>
            <label class="fieldbox">
                <span class="fieldbox__label">"Saat"</span>
                <span class="fieldbox__val"><input type="time" aria-label="Saat" prop:value=move || time.get()
                    on:input=move |ev| time.set(event_target_value(&ev))/></span>
            </label>
            <label class="fieldbox fieldbox--wide">
                <span class="fieldbox__label">"Hatırlat"</span>
                <span class="fieldbox__val"><Icon name="bell" size="sm"/>
                    <select aria-label="Hatırlat" on:change=move |ev| remind.set(event_target_value(&ev))>{options}</select></span>
            </label>
        </div>
    };
    panel_form(kind.title(), panel, submit, body, kind.submit())
}
