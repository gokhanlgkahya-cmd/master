//! Hatırlatmalar: kaçırılan / bugün / yarın / bu hafta / sonra / tekrarlayan; ertele
//! (10-hatirlatmalar.html).

use std::collections::HashMap;

use leptos::prelude::*;
use notai_api::{ReminderGroupView, ReminderView, RemindersView};

use crate::{ErrorNote, Icon, TabBar, backend, ctx, failed, loading, run, view};

#[component]
pub fn Reminders() -> impl IntoView {
    let c = ctx();
    let data = LocalResource::new(move || {
        let _ = c.version.get();
        let b = backend();
        async move {
            let r = b.reminders().await?;
            // Konu adları yalnız alt satır için; alınamazsa boş geçilir.
            let names: HashMap<String, String> = b
                .topics("all")
                .await
                .map(|t| t.topics.into_iter().map(|x| (x.id, x.name)).collect())
                .unwrap_or_default();
            Ok::<(RemindersView, HashMap<String, String>), String>((r, names))
        }
    });
    let sub = move || match data.get() {
        Some(Ok((r, _))) => view::reminders_line(&r.groups),
        _ => String::new(),
    };
    view! {
        <header class="topbar">
            <div class="topbar__text"><h1 class="topbar__title">"Hatırlatmalar"</h1><p class="topbar__sub">{sub}</p></div>
        </header>
        <div class="body scroll">
            {move || match data.get() {
                None => loading(),
                Some(Err(e)) => failed(e),
                Some(Ok((r, _))) if r.groups.iter().all(|g| g.items.is_empty()) => view! {
                    <div class="empty">
                        <span class="empty__icon"><Icon name="bell"/></span>
                        <h2 class="empty__title">"Hatırlatma yok"</h2>
                        <p class="empty__text">"Bir göreve “önce hatırlat” ekle ya da ataç → Hatırlatma ile kur."</p>
                    </div>
                }.into_any(),
                Some(Ok((r, names))) => r
                    .groups
                    .into_iter()
                    .filter(|g| !g.items.is_empty())
                    .map(|g| group(g, &names))
                    .collect_view()
                    .into_any(),
            }}
        </div>
        <ErrorNote/>
        <TabBar/>
    }
}

fn group(g: ReminderGroupView, names: &HashMap<String, String>) -> impl IntoView {
    let n = g.items.len();
    let recurring = g.key == "recurring";
    let due = g.key == "missed";
    let icon = recurring.then(|| view! { <Icon name="repeat" size="xs"/> });
    let head_class = if due {
        "grouphead grouphead--error"
    } else {
        "grouphead"
    };
    let rows = g
        .items
        .into_iter()
        .map(|r| {
            let topic = r
                .topic_id
                .as_ref()
                .and_then(|t| names.get(t))
                .cloned()
                .unwrap_or_default();
            row(r, topic, due || g.key == "today")
        })
        .collect_view();
    view! {
        <h2 class=head_class>{icon}{g.label}<span class="count">{n}</span></h2>
        <div class="card">{rows}</div>
    }
}

fn row(r: ReminderView, topic: String, due: bool) -> impl IntoView {
    let class = if due { "rem rem--due" } else { "rem" };
    let icon = if r.recurring { "repeat" } else { "bell" };
    let sub = if r.recurring && topic.is_empty() {
        "Tekrarlayan".to_owned()
    } else {
        topic
    };
    let snooze = view::SNOOZE_OPTIONS
        .iter()
        .map(|&(opt, label)| {
            let id = r.task_id.clone();
            view! {
                <button aria-label=format!("Ertele {label}: {}", r.title)
                    on:click=move |_| { let id = id.clone(); run(move |b| async move { b.snooze(id, opt).await }) }>
                    {label}
                </button>
            }
        })
        .collect_view();
    view! {
        <div class=class>
            <div class="rem__row">
                <span class="rem__ic"><Icon name=icon/></span>
                <span class="rem__txt">
                    <span class="rem__title">{r.title.clone()}</span>
                    <span class="rem__sub">{sub}</span>
                </span>
                <span class="rem__time">{r.when.clone()}</span>
            </div>
            <div class="snooze">
                <span class="snooze__lead"><Icon name="clock" size="xs"/>"Ertele"</span>
                {snooze}
            </div>
        </div>
    }
}
