//! Harcamalar (Gökhan, 2026-11-07): aylık gelir/gider/birikim, yorumlar ve öneriler, iş/ev/kişisel
//! dağılımı, kategoriler, düzenli ödemeler, hareketler (kategori/kapsam düzeltilir, öğrenilir) ve
//! banka ekstresi yükleme. Ekstre yerelde okunur; mükerrer satır eklenmez.
//! Varlıklar sekmesi (2026-11-08): menkul/gayrimenkul toplamı, 30 günlük değişim, tür dağılımı.

use leptos::html::Input;
use leptos::prelude::*;
use leptos::task::spawn_local;
use notai_api::{AssetRowView, AssetsView, FinanceView, ShareView, TransactionView};

use crate::sheets::AssetSheet;
use crate::view::TopicLook;
use crate::{Avatar, ErrorNote, Icon, Screen, TabBar, backend, ctx, failed, loading, run, web};

fn shift_month(m: &str, delta: i32) -> String {
    let (y, mo) = m
        .split_once('-')
        .and_then(|(y, mo)| Some((y.parse::<i32>().ok()?, mo.parse::<i32>().ok()?)))
        .unwrap_or((2026, 1));
    let total = y * 12 + (mo - 1) + delta;
    format!("{}-{:02}", total.div_euclid(12), total.rem_euclid(12) + 1)
}

#[component]
pub fn Finance() -> impl IntoView {
    let c = ctx();
    let assets_tab = RwSignal::new(false);
    let month = RwSignal::new(None::<String>);
    let message = RwSignal::new(None::<String>);
    let file = NodeRef::<Input>::new();
    let data = LocalResource::new(move || {
        let m = month.get();
        let _ = c.version.get();
        let b = backend();
        async move { b.finance(m).await }
    });
    let current = move || match data.get() {
        Some(Ok(v)) => Some(v.month),
        _ => None,
    };
    let go = move |d: i32| {
        if let Some(m) = current() {
            month.set(Some(shift_month(&m, d)));
        }
    };
    let b = backend();
    let on_file = move |_ev: leptos::ev::Event| {
        let Some(el) = file.get_untracked() else {
            return;
        };
        let b = b.clone();
        message.set(Some("Ekstre okunuyor…".into()));
        spawn_local(async move {
            match web::read_first_file(&el).await {
                Ok(Some(p)) => {
                    el.set_value("");
                    match b.import_statement(p.bytes, p.name, p.mime).await {
                        Ok(r) => {
                            message.set(Some(r.message));
                            c.version.update(|v| *v += 1);
                        }
                        Err(e) => {
                            message.set(None);
                            c.error.set(Some(e));
                        }
                    }
                }
                Ok(None) => message.set(None),
                Err(e) => {
                    message.set(None);
                    c.error.set(Some(e));
                }
            }
        });
    };
    view! {
        <header class="flowbar">
            <button class="iconbtn" aria-label="Geri" on:click=move |_| c.screen.set(Screen::Home)><Icon name="back"/></button>
            <div class="flowbar__who">
                <Avatar look=TopicLook::plain("wallet") small=true/>
                <div class="flowbar__text">
                    <span class="flowbar__name">{move || if assets_tab.get() { "Varlıklar" } else { "Harcamalar" }}</span>
                    <span class="flowbar__sub">{move || if assets_tab.get() { "Menkul ve gayrimenkuller".to_owned() } else {
                        match data.get() { Some(Ok(v)) => v.title, _ => String::new() } }}</span>
                </div>
            </div>
            {move || (!assets_tab.get()).then(|| view! {
                <button class="iconbtn" aria-label="Önceki ay" on:click=move |_| go(-1)><Icon name="back"/></button>
                <button class="iconbtn" aria-label="Sonraki ay" on:click=move |_| go(1)><Icon name="chevr"/></button>
            })}
        </header>
        <div class="fin-switch">
            <div class="switch2" role="tablist" aria-label="Bölüm">
                <button role="tab" aria-selected=move || (!assets_tab.get()).to_string()
                    on:click=move |_| assets_tab.set(false)>"Harcamalar"</button>
                <button role="tab" aria-selected=move || assets_tab.get().to_string()
                    on:click=move |_| assets_tab.set(true)>"Varlıklar"</button>
            </div>
        </div>
        {move || if assets_tab.get() { view! { <Assets/> }.into_any() } else { let on_file = on_file.clone(); view! {
        <div class="body scroll">
            <div class="fin-actions">
                <button class="btn btn--outline" on:click=move |_| { if let Some(el) = file.get_untracked() { el.click(); } }>
                    <Icon name="file" size="sm"/>"Ekstre yükle (PDF)"
                </button>
                <input class="filepick" type="file" accept="application/pdf,text/plain,.pdf,.txt,.csv" tabindex="-1"
                    aria-hidden="true" node_ref=file on:change=on_file/>
            </div>
            {move || message.get().map(|m| view! { <p class="note fin-msg" role="status">{m}</p> })}
            {move || match data.get() {
                None => loading(),
                Some(Err(e)) => failed(e),
                Some(Ok(v)) => report(v).into_any(),
            }}
        </div>
        }.into_any() }}
        <ErrorNote/>
        <TabBar/>
    }
}

/// Varlıklar sekmesi.
#[component]
fn Assets() -> impl IntoView {
    let c = ctx();
    let adding = RwSignal::new(false);
    let editing = RwSignal::new(None::<notai_api::TopicRef>);
    let edit_open = RwSignal::new(false);
    let data = LocalResource::new(move || {
        let _ = c.version.get();
        let b = backend();
        async move { b.assets().await }
    });
    view! {
        <div class="body scroll">
            <div class="fin-actions">
                <button class="btn btn--outline" on:click=move |_| adding.set(true)>
                    <Icon name="plus" size="sm"/>"Varlık ekle"
                </button>
            </div>
            {move || match data.get() {
                None => loading(),
                Some(Err(e)) => failed(e),
                Some(Ok(v)) => assets_report(v, editing, edit_open).into_any(),
            }}
        </div>
        {move || adding.get().then(|| view! { <AssetSheet topic=None open=adding/> })}
        {move || (edit_open.get()).then(|| editing.get().map(|t| view! { <AssetSheet topic=Some(t) open=edit_open/> }))}
    }
}

fn assets_report(
    v: AssetsView,
    editing: RwSignal<Option<notai_api::TopicRef>>,
    edit_open: RwSignal<bool>,
) -> impl IntoView {
    if v.assets.is_empty() {
        return view! {
            <div class="empty">
                <span class="empty__icon"><Icon name="house"/></span>
                <h2 class="empty__title">"Henüz varlık yok"</h2>
                <p class="empty__text">"Ev, araç, hisse, altın, döviz ya da mevduatını ekle. Sembol verirsen NotAI ajanı günde bir fiyatına bakar."</p>
            </div>
        }
        .into_any();
    }
    let change_class = if v.change_negative {
        "fin-tile__val fin-tile__val--neg"
    } else {
        "fin-tile__val"
    };
    let change = v.change_30d.clone().unwrap_or_else(|| "—".to_owned());
    let rows = v
        .assets
        .into_iter()
        .map(|r| asset_row(r, editing, edit_open))
        .collect_view();
    view! {
        <div class="fin-tiles fin-tiles--2">
            <div class="fin-tile"><span class="fin-tile__label">"Toplam"</span><span class="fin-tile__val tnum">{v.total}</span></div>
            <div class="fin-tile"><span class="fin-tile__label">"Son 30 gün"</span><span class=change_class>{change}</span>
                <span class="fin-tile__sub">"yeni eklenen varlık kazanç sayılmaz"</span></div>
        </div>
        {shares("Türlere göre", v.by_kind, true)}
        <h2 class="grouphead">"Varlıklar"</h2>
        <ul class="card fin-txs">{rows}</ul>
    }
    .into_any()
}

fn asset_row(
    r: AssetRowView,
    editing: RwSignal<Option<notai_api::TopicRef>>,
    edit_open: RwSignal<bool>,
) -> impl IntoView {
    let meta = [
        Some(r.kind_label.clone()),
        Some(r.quantity.clone()).filter(|q| !q.is_empty()),
        r.symbol.clone().map(|s| format!("fiyat: {s}")),
        Some(r.as_of.clone()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    let t = notai_api::TopicRef {
        id: r.topic_id.clone(),
        name: r.name.clone(),
    };
    let c = ctx();
    let open_chat = {
        let t = t.clone();
        move |_| c.screen.set(Screen::Topic(t.clone()))
    };
    view! {
        <li class="fin-tx fin-asset">
            <button class="fin-asset__main" aria-label=format!("{} yazışmasını aç", r.name) on:click=open_chat>
                <span class="fin-tx__body">
                    <span class="fin-tx__desc">{r.name.clone()}</span>
                    <span class="fin-tx__meta"><span>{meta}</span></span>
                </span>
                <span class="fin-tx__amt tnum">{r.value}</span>
            </button>
            <button class="iconbtn" aria-label=format!("{} varlık bilgisini düzenle", r.name)
                on:click=move |_| { editing.set(Some(t.clone())); edit_open.set(true); }>
                <Icon name="pencil" size="sm"/>
            </button>
        </li>
    }
}

fn shares(title: &'static str, items: Vec<ShareView>, bars: bool) -> impl IntoView {
    (!items.is_empty()).then(|| {
        let rows = items
            .into_iter()
            .map(|s| {
                let bar = bars.then(|| {
                    view! { <span class="fin-bar" aria-hidden="true"><i style=format!("width:{}%", s.pct.clamp(0, 100))></i></span> }
                });
                view! {
                    <li class="fin-row">
                        <span class="fin-row__label">{s.label}</span>
                        {bar}
                        <span class="fin-row__amt tnum">{s.amount}</span>
                    </li>
                }
            })
            .collect_view();
        view! {
            <h2 class="grouphead">{title}</h2>
            <ul class="card fin-list">{rows}</ul>
        }
    })
}

fn report(v: FinanceView) -> impl IntoView {
    let empty = v.transactions.is_empty();
    let pending = (v.pending_statements > 0).then(|| {
        view! { <p class="note">{format!("{} ekstre biçimi tanınmadı; AI'a (ajana) okutulmayı bekliyor.", v.pending_statements)}</p> }
    });
    if empty {
        return view! {
            {pending}
            <div class="empty">
                <span class="empty__icon"><Icon name="wallet"/></span>
                <h2 class="empty__title">"Bu ay hareket yok"</h2>
                <p class="empty__text">"Banka ekstresini (PDF) yükle ya da günlüğe \"markete 640 ₺ verdim\" gibi yaz; NotAI harcama olarak önerir."</p>
            </div>
        }
        .into_any();
    }
    let net_class = if v.net_negative {
        "fin-tile__val fin-tile__val--neg"
    } else {
        "fin-tile__val"
    };
    let savings = v
        .savings_pct
        .map_or_else(String::new, |p| format!("gelirin %{p}'i"));
    let insights = (!v.insights.is_empty()).then(|| {
        let items = v
            .insights
            .iter()
            .map(|i| view! { <li class="fin-insight"><Icon name="spark" size="xs"/><span>{i.clone()}</span></li> })
            .collect_view();
        view! { <ul class="card fin-insights" aria-label="Yorumlar">{items}</ul> }
    });
    let options = v.category_options.clone();
    let txs = v
        .transactions
        .into_iter()
        .map(|t| tx_row(t, options.clone()))
        .collect_view();
    view! {
        {pending}
        <div class="fin-tiles">
            <div class="fin-tile"><span class="fin-tile__label">"Gelir"</span><span class="fin-tile__val tnum">{v.income}</span></div>
            <div class="fin-tile"><span class="fin-tile__label">"Gider"</span><span class="fin-tile__val tnum">{v.expense}</span></div>
            <div class="fin-tile"><span class="fin-tile__label">"Birikim"</span><span class=net_class>{v.net}</span><span class="fin-tile__sub">{savings}</span></div>
        </div>
        {insights}
        {shares("İş / Ev / Kişisel", v.scopes, true)}
        {shares("Kategoriler", v.categories, true)}
        {shares("Düzenli ödemeler", v.recurring, false)}
        <h2 class="grouphead">"Hareketler"</h2>
        <ul class="card fin-txs">{txs}</ul>
    }
    .into_any()
}

fn tx_row(t: TransactionView, options: Vec<(String, String)>) -> impl IntoView {
    let (id_c, id_s) = (t.id.clone(), t.id.clone());
    let (cat_now, scope_now) = (t.category.clone(), t.scope.clone());
    let (cat_for_scope, scope_for_cat) = (t.category.clone(), t.scope.clone());
    let cats = options
        .into_iter()
        .map(|(k, l)| {
            let sel = k == cat_now;
            view! { <option value=k selected=sel>{l}</option> }
        })
        .collect_view();
    let scopes = [("is", "İş"), ("ev", "Ev"), ("kisisel", "Kişisel")]
        .into_iter()
        .map(|(k, l)| view! { <option value=k selected={k == scope_now}>{l}</option> })
        .collect_view();
    let meta = if t.matched {
        "elle girildi · ekstreyle eşleşti (bir kez sayılır)".to_owned()
    } else if t.source == "manual" {
        "elle girildi".to_owned()
    } else {
        "ekstre".to_owned()
    };
    view! {
        <li class="fin-tx" class:fin-tx--matched=t.matched>
            <span class="fin-tx__date tnum">{t.date_label}</span>
            <span class="fin-tx__body">
                <span class="fin-tx__desc">{t.description.clone()}</span>
                <span class="fin-tx__meta">
                    <select aria-label=format!("Kategori: {}", t.description) on:change=move |ev| {
                        let (id, cat, scope) = (id_c.clone(), event_target_value(&ev), scope_for_cat.clone());
                        run(move |b| async move { b.classify_transaction(id, cat, scope).await });
                    }>{cats}</select>
                    <select aria-label=format!("Kapsam: {}", t.description) on:change=move |ev| {
                        let (id, cat, scope) = (id_s.clone(), cat_for_scope.clone(), event_target_value(&ev));
                        run(move |b| async move { b.classify_transaction(id, cat, scope).await });
                    }>{scopes}</select>
                    <span>{meta}</span>
                </span>
            </span>
            <span class="fin-tx__amt tnum" class:fin-tx__amt--in={!t.expense}>{t.amount}</span>
        </li>
    }
}

#[cfg(test)]
mod tests {
    use super::shift_month;

    #[test]
    fn month_navigation() {
        assert_eq!(shift_month("2026-10", 1), "2026-11");
        assert_eq!(shift_month("2026-12", 1), "2027-01");
        assert_eq!(shift_month("2026-01", -1), "2025-12");
    }
}
