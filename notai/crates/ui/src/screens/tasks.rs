//! Görevler: geciken / bugün / yaklaşan / tarihsiz, ekleme ve düzenleme (08-gorevler.html).

use leptos::prelude::*;
use notai_api::TaskRowView;

use crate::composer::Composer;
use crate::{ErrorNote, Icon, TabBar, backend, ctx, failed, loading, run, view};

#[component]
pub fn Tasks() -> impl IntoView {
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
                None => loading(),
                Some(Err(e)) => failed(e),
                Some(Ok(t)) if view::task_ids(&t).is_empty() => view! {
                    <div class="empty">
                        <span class="empty__icon"><Icon name="task"/></span>
                        <h2 class="empty__title">"Görev yok"</h2>
                        <p class="empty__text">"Aşağıya yaz; ya da bir kayıttaki görev önerisini kabul et."</p>
                    </div>
                }.into_any(),
                Some(Ok(t)) => view! {
                    {task_group("Geciken", true, t.overdue)}
                    {task_group("Bugün", false, t.today)}
                    {task_group("Yaklaşan", false, t.upcoming)}
                    {task_group("Tarihsiz", false, t.undated)}
                }.into_any(),
            }}
        </div>
        <ErrorNote/>
        <Composer placeholder="Görev ekle…" topic=None task_mode=true/>
        <TabBar/>
    }
}

fn task_group(title: &'static str, late: bool, rows: Vec<TaskRowView>) -> impl IntoView {
    (!rows.is_empty()).then(|| {
        let n = rows.len();
        let head_class = if late {
            "grouphead grouphead--error"
        } else {
            "grouphead"
        };
        let warn = late.then(|| view! { <Icon name="warn" size="xs"/> });
        let items = rows.into_iter().map(task_row).collect_view();
        view! {
            <h2 class=head_class>{warn}{title}<span class="count">{n}</span></h2>
            <div class="card">{items}</div>
        }
    })
}

/// Görev satırı: yuvarlak onay kutusu tamamlar, gövdeye dokunmak düzenleme sayfasını açar.
pub fn task_row(t: TaskRowView) -> impl IntoView {
    let c = ctx();
    let id = t.id.clone();
    let open = t.clone();
    let late = (t.overdue_days > 0).then(|| {
        view! { <span class="late"><Icon name="warn" size="xs"/>{format!("{}\u{a0}gün gecikti", t.overdue_days)}</span> }
    });
    let mut rest = t.clone();
    rest.overdue_days = 0;
    if t.overdue_days > 0 {
        rest.due_date = None;
    }
    let meta = view::task_meta(&rest);
    let prio = view::priority_icon(&t.priority);
    view! {
        <div class="task">
            <button class="task__check" aria-label=format!("Tamamla: {}", t.title)
                on:click=move |_| { let id = id.clone(); run(move |b| async move { b.complete_task(id).await }) }>
                <span class="check"></span>
            </button>
            <button class="task__body" aria-label=format!("Düzenle: {}", t.title)
                on:click=move |_| c.editing.set(Some(open.clone()))>
                <span class="task__title">{t.title.clone()}</span>
                <span class="task__meta">{late}<span>{meta}</span></span>
            </button>
            <div class="task__side"><span class="tag"><Icon name=prio size="xs"/>{t.priority_label.clone()}</span></div>
        </div>
    }
}
