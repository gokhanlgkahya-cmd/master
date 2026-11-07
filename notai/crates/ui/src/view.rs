//! Saf görünüm yardımcıları (DOM'suz; yerel testlerle sınanır).

use chrono::{Datelike, NaiveDate};
use notai_api::{EntryView, TaskRowView};

const MONTHS: [&str; 12] = [
    "Oca", "Şub", "Mar", "Nis", "May", "Haz", "Tem", "Ağu", "Eyl", "Eki", "Kas", "Ara",
];

fn short_date(d: NaiveDate) -> String {
    let m = MONTHS.get(d.month0() as usize).copied().unwrap_or("");
    format!("{}\u{a0}{m}", d.day())
}

/// Konunun simgesi: kullanıcının seçtiği, yoksa türüne göre.
#[must_use]
pub fn topic_icon(icon: Option<&str>, kind: &str) -> String {
    icon.map_or_else(
        || match kind {
            "resource" => "book".to_owned(),
            "area" => "layers".to_owned(),
            _ => "folder".to_owned(),
        },
        str::to_owned,
    )
}

/// Konu satırındaki tür ("Proje · 20 Kas").
#[must_use]
pub fn kind_line(label: &str, due: Option<NaiveDate>) -> String {
    match due {
        Some(d) => format!("{label} · {}", short_date(d)),
        None => label.to_owned(),
    }
}

/// Akış başlığının alt satırı.
#[must_use]
pub fn unprocessed_line(n: u64) -> String {
    match n {
        0 => "Hepsi işlendi".to_owned(),
        n => format!("{n} işlenmemiş kayıt"),
    }
}

/// Görevler başlığının alt satırı ("2 geciken · 3 bugün").
#[must_use]
pub fn tasks_line(overdue: usize, today: usize) -> String {
    match (overdue, today) {
        (0, 0) => "Bugün için iş yok".to_owned(),
        (0, t) => format!("{t} bugün"),
        (o, 0) => format!("{o} geciken"),
        (o, t) => format!("{o} geciken · {t} bugün"),
    }
}

/// Sabah planı maddesinin alt satır sınıfı (renk yalnız anlam taşır: gecikme = hata).
#[must_use]
pub fn plan_reason_class(kind: &str) -> &'static str {
    match kind {
        "overdue" => "plan__sub plan__sub--error",
        "today" => "plan__sub plan__sub--warning",
        _ => "plan__sub",
    }
}

/// Görev satırının meta metni ("1 gün gecikti", "Cmt 14 Kas 10:00").
#[must_use]
pub fn task_meta(t: &TaskRowView) -> String {
    let mut parts = Vec::new();
    if t.overdue_days > 0 {
        parts.push(format!("{}\u{a0}gün gecikti", t.overdue_days));
    } else if let Some(d) = t.due_date {
        parts.push(short_date(d));
    }
    if let Some(time) = t.due_time {
        parts.push(time.format("%H:%M").to_string());
    }
    if t.recurring {
        parts.push("tekrarlı".to_owned());
    }
    parts.join(" · ")
}

/// Detay sayfasının meta satırı ("GemOlive · 09:12 · İşlenmedi").
#[must_use]
pub fn sheet_meta(e: &EntryView) -> String {
    let place = e.topic.as_ref().map_or("Ana Günlük", |t| t.name.as_str());
    let state = if e.unprocessed {
        "İşlenmedi"
    } else {
        "İşlendi"
    };
    format!("{place} · {} · {state}", e.time)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        assert_eq!(unprocessed_line(0), "Hepsi işlendi");
        assert_eq!(unprocessed_line(4), "4 işlenmemiş kayıt");
        assert_eq!(tasks_line(2, 3), "2 geciken · 3 bugün");
        assert_eq!(tasks_line(0, 0), "Bugün için iş yok");
        assert_eq!(
            kind_line("Proje", NaiveDate::from_ymd_opt(2026, 11, 20)),
            "Proje · 20\u{a0}Kas"
        );
        assert_eq!(topic_icon(None, "resource"), "book");
        assert_eq!(topic_icon(Some("drop"), "area"), "drop");
        assert_eq!(plan_reason_class("overdue"), "plan__sub plan__sub--error");
    }

    #[test]
    fn task_meta_text() {
        let mut t = TaskRowView {
            id: "1".into(),
            title: "Kombi servisini ara".into(),
            topic_id: None,
            due_date: NaiveDate::from_ymd_opt(2026, 11, 14),
            due_time: chrono::NaiveTime::from_hms_opt(10, 0, 0),
            priority: "normal".into(),
            priority_label: "Normal".into(),
            overdue_days: 0,
            open: true,
            recurring: true,
        };
        assert_eq!(task_meta(&t), "14\u{a0}Kas · 10:00 · tekrarlı");
        t.overdue_days = 1;
        t.due_time = None;
        t.recurring = false;
        assert_eq!(task_meta(&t), "1\u{a0}gün gecikti");
    }
}
