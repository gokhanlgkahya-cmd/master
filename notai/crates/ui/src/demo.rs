//! Tarayıcı önizlemesi için örnek veri (design/directions/plan.md §5 ile aynı dünya).
//! Gerçek kurallar depodadır; burası yalnız arayüzü göstermek ve sınamak içindir.

use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use notai_api::{
    DayGroup, EntryView, JournalView, MorningPlanView, PlanItemView, SuggestionView, TaskRowView,
    TasksView, TopicRef, TopicRowView, TopicsView,
};

/// Demo'nun "bugün"ü: Cuma 6 Kasım 2026.
fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 6).unwrap_or_default()
}

/// İstanbul yerel saati (UTC+3) → UTC.
fn at(day: u32, h: u32, m: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, day, h, m, 0)
        .single()
        .unwrap_or_default()
        - chrono::Duration::hours(3)
}

fn label(date: NaiveDate) -> String {
    match (today() - date).num_days() {
        0 => "Bugün".into(),
        1 => "Dün".into(),
        _ => format!("Çarşamba, {} Kasım", chrono::Datelike::day(&date)),
    }
}

#[derive(Debug, Clone)]
pub struct Demo {
    topics: Vec<TopicRowView>,
    entries: Vec<EntryView>,
    tasks: Vec<TaskRowView>,
    next_id: u32,
}

fn topic(id: &str, name: &str, kind: &str, icon: &str, last: &str, preview: &str) -> TopicRowView {
    let kind_label = match kind {
        "project" => "Proje",
        "resource" => "Kaynak",
        _ => "Alan",
    };
    TopicRowView {
        id: id.into(),
        name: name.into(),
        kind: kind.into(),
        kind_label: kind_label.into(),
        due_date: None,
        icon: Some(icon.into()),
        pinned: false,
        last_label: Some(last.into()),
        preview: Some(preview.into()),
    }
}

fn sugg(id: &str, kind: &str, chip: &str, decision: &str) -> SuggestionView {
    SuggestionView {
        id: id.into(),
        kind: kind.into(),
        chip: chip.into(),
        decision: decision.into(),
        confidence: 0.8,
    }
}

struct E<'a> {
    id: &'a str,
    text: &'a str,
    when: DateTime<Utc>,
    topic: Option<(&'a str, &'a str)>,
    suggestions: Vec<SuggestionView>,
}

fn entry(e: E<'_>) -> EntryView {
    let unprocessed = e.topic.is_none() && e.suggestions.iter().any(SuggestionView::is_pending);
    EntryView {
        id: e.id.into(),
        text: e.text.into(),
        time: (e.when + chrono::Duration::hours(3))
            .format("%H:%M")
            .to_string(),
        captured_at: e.when,
        source: "app".into(),
        status: if unprocessed { "inbox" } else { "processed" }.into(),
        topic: e.topic.map(|(id, name)| TopicRef {
            id: id.into(),
            name: name.into(),
        }),
        unprocessed,
        ai_failed: false,
        suggestions: e.suggestions,
        tags: vec![],
        people: vec![],
    }
}

fn task(
    id: &str,
    title: &str,
    due: Option<(u32, Option<(u32, u32)>)>,
    priority: &str,
    topic: Option<&str>,
) -> TaskRowView {
    let due_date = due.and_then(|(d, _)| NaiveDate::from_ymd_opt(2026, 11, d));
    let overdue = due_date.map_or(0, |d| (today() - d).num_days().max(0));
    TaskRowView {
        id: id.into(),
        title: title.into(),
        topic_id: topic.map(Into::into),
        due_date,
        due_time: due
            .and_then(|(_, t)| t)
            .and_then(|(h, m)| NaiveTime::from_hms_opt(h, m, 0)),
        priority: priority.into(),
        priority_label: match priority {
            "high" => "Yüksek",
            "low" => "Düşük",
            _ => "Orta",
        }
        .into(),
        overdue_days: overdue,
        open: true,
        recurring: false,
    }
}

impl Default for Demo {
    fn default() -> Self {
        Self::new()
    }
}

impl Demo {
    #[must_use]
    pub fn new() -> Self {
        let topics = vec![
            topic(
                "t-gem",
                "GemOlive",
                "area",
                "drop",
                "10:47",
                "Ambalaj faturası · 1.850 ₺",
            ),
            topic(
                "t-kostik",
                "Kostik tedarik",
                "project",
                "flask",
                "09:12",
                "Ayhan Kimya teklifi: NaOH %99, 25 kg torba",
            ),
            topic(
                "t-logo",
                "Logo ERP geçişi",
                "project",
                "layers",
                "Dün",
                "Stok kartı aktarımı: 1.240 kalem, 37 hatalı",
            ),
            topic(
                "t-ev",
                "Ev",
                "area",
                "house",
                "Dün",
                "Kombi bakımı – 14 Kasım Cumartesi",
            ),
            topic(
                "t-saglik",
                "Sağlık",
                "area",
                "heart",
                "Dün",
                "Tansiyon 12/8 · sabah yürüyüşü 35 dk",
            ),
            topic(
                "t-s24",
                "Ev sunucusu (S24)",
                "project",
                "server",
                "Çarşamba",
                "n8n: WhatsApp → NotAI webhook akışı çalıştı",
            ),
            topic(
                "t-okuma",
                "Okuma notları",
                "resource",
                "book",
                "Çarşamba",
                "Ohno, Toyota Üretim Sistemi – s. 84",
            ),
            topic(
                "t-vsm",
                "Değer akışı haritalandırması",
                "resource",
                "flow",
                "2 Kas",
                "Çevrim süresi 42 sn, takt süresi 55 sn",
            ),
        ];
        let entries = vec![
            entry(E {
                id: "e1",
                text: "Logo’da stok kartlarının birim alanı boş geliyor, Murat Bey’e sor.",
                when: at(4, 18, 42),
                topic: Some(("t-logo", "Logo ERP geçişi")),
                suggestions: vec![],
            }),
            entry(E {
                id: "e2",
                text: "Lavanta uçucu yağı 2 kg kaldı, yeni sipariş ver.",
                when: at(4, 21, 5),
                topic: Some(("t-gem", "GemOlive")),
                suggestions: vec![sugg("s0", "task", "Görev", "accepted")],
            }),
            entry(E {
                id: "e3",
                text: "Zeytinyağlı sabunda kür süresi 4–6 hafta. Raf etiketine üretim tarihi şart.",
                when: at(5, 13, 30),
                topic: Some(("t-gem", "GemOlive")),
                suggestions: vec![],
            }),
            entry(E {
                id: "e4",
                text: "Kostik tedarikçisini ara – perşembe. 25 kg’lık torba fiyatını ve teslim süresini sor.",
                when: at(6, 9, 12),
                topic: None,
                suggestions: vec![
                    sugg("s1", "task", "Görev · Perşembe 12 Kas", "pending"),
                    sugg("s2", "topic", "→ Kostik tedarik", "pending"),
                    sugg("s3", "person", "Kişi: Ayhan Kimya", "pending"),
                ],
            }),
            entry(E {
                id: "e5",
                text: "Şirket aracının muayenesi ne zamandı?",
                when: at(6, 11, 30),
                topic: None,
                suggestions: vec![sugg(
                    "s4",
                    "reminder",
                    "Hatırlatma · Pazartesi 9 Kas 09:00",
                    "pending",
                )],
            }),
            entry(E {
                id: "e6",
                text: "Pazar gözden geçirmede iş–ev dengesini konuş.",
                when: at(6, 12, 5),
                topic: None,
                suggestions: vec![sugg("s5", "tag", "#haftalık", "pending")],
            }),
        ];
        let tasks = vec![
            task(
                "k1",
                "Kostik tekliflerini karşılaştır",
                Some((5, None)),
                "normal",
                Some("t-kostik"),
            ),
            task(
                "k2",
                "Logo: 37 hatalı stok kartını düzelt",
                Some((6, None)),
                "high",
                Some("t-logo"),
            ),
            task(
                "k3",
                "Akşam yürüyüşü",
                Some((6, Some((19, 0)))),
                "normal",
                Some("t-saglik"),
            ),
            task(
                "k4",
                "Kombi servisini ara",
                Some((14, Some((10, 0)))),
                "normal",
                Some("t-ev"),
            ),
            task("k5", "Numune yolla", None, "high", Some("t-gem")),
        ];
        Self {
            topics,
            entries,
            tasks,
            next_id: 100,
        }
    }

    fn unprocessed(&self) -> u64 {
        self.entries.iter().filter(|e| e.unprocessed).count() as u64
    }

    #[must_use]
    pub fn topics(&self, kind: &str) -> TopicsView {
        let topics = self
            .topics
            .iter()
            .filter(|t| match kind {
                "projects" => t.kind == "project",
                "areas" => t.kind == "area",
                "resources" => t.kind == "resource",
                _ => true,
            })
            .cloned()
            .collect();
        let last = self.entries.iter().max_by_key(|e| e.captured_at);
        TopicsView {
            topics,
            unprocessed: self.unprocessed(),
            journal_preview: last.map(|e| e.text.chars().take(80).collect()),
            journal_last_label: last.map(|e| e.time.clone()),
        }
    }

    #[must_use]
    pub fn journal(&self, topic: Option<&str>) -> JournalView {
        let mut items: Vec<&EntryView> = self
            .entries
            .iter()
            .filter(|e| topic.is_none_or(|t| e.topic.as_ref().is_some_and(|x| x.id == t)))
            .collect();
        items.sort_by_key(|e| e.captured_at);
        let mut groups: Vec<DayGroup<EntryView>> = Vec::new();
        for e in items {
            let date = (e.captured_at + chrono::Duration::hours(3)).date_naive();
            match groups.last_mut() {
                Some(g) if g.date == date => g.items.push(e.clone()),
                _ => groups.push(DayGroup {
                    date,
                    label: label(date),
                    items: vec![e.clone()],
                }),
            }
        }
        JournalView {
            groups,
            unprocessed: self.unprocessed(),
            next_before: None,
        }
    }

    pub fn capture(&mut self, text: &str, topic: Option<&str>) -> Result<(), String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("kayıt metni boş olamaz".into());
        }
        self.next_id += 1;
        let topic_ref = topic
            .and_then(|id| self.topics.iter().find(|t| t.id == id))
            .map(|t| (t.id.clone(), t.name.clone()));
        let minute = 7 + (self.next_id % 50);
        let mut e = entry(E {
            id: "tmp",
            text,
            when: at(6, 12, minute),
            topic: None,
            suggestions: vec![],
        });
        e.id = format!("e{}", self.next_id);
        e.topic = topic_ref.map(|(id, name)| TopicRef { id, name });
        e.unprocessed = e.topic.is_none();
        e.status = if e.unprocessed { "inbox" } else { "processed" }.into();
        self.entries.push(e);
        Ok(())
    }

    /// En son yakalanan kaydın kimliği.
    #[must_use]
    pub fn last_entry_id(&self) -> Option<String> {
        self.entries.last().map(|e| e.id.clone())
    }

    pub fn decide(&mut self, suggestion_id: &str, accept: bool) -> Result<(), String> {
        let topics = self.topics.clone();
        let entry = self
            .entries
            .iter_mut()
            .find(|e| e.suggestions.iter().any(|s| s.id == suggestion_id))
            .ok_or("öneri bulunamadı")?;
        let s = entry
            .suggestions
            .iter_mut()
            .find(|s| s.id == suggestion_id)
            .ok_or("öneri bulunamadı")?;
        if !s.is_pending() {
            return Err("öneri zaten karara bağlanmış".into());
        }
        s.decision = if accept { "accepted" } else { "dismissed" }.into();
        if accept && s.kind == "topic" {
            let name = s.chip.trim_start_matches("→ ").to_owned();
            if let Some(t) = topics.iter().find(|t| t.name == name) {
                entry.topic = Some(TopicRef {
                    id: t.id.clone(),
                    name: t.name.clone(),
                });
                entry.unprocessed = false;
            }
        }
        if entry.suggestions.iter().all(|s| !s.is_pending()) {
            entry.unprocessed = false;
        }
        entry.status = if entry.unprocessed {
            "inbox"
        } else {
            "processed"
        }
        .into();
        Ok(())
    }

    pub fn entry_action(&mut self, id: &str, action: &str) -> Result<(), String> {
        match action {
            "archive" | "trash" => {
                let before = self.entries.len();
                self.entries.retain(|e| e.id != id);
                if self.entries.len() == before {
                    return Err("kayıt bulunamadı".into());
                }
                Ok(())
            }
            other => Err(format!("demo'da desteklenmeyen eylem: {other}")),
        }
    }

    #[must_use]
    pub fn tasks(&self) -> TasksView {
        let mut v = TasksView::default();
        for t in self.tasks.iter().filter(|t| t.open) {
            match t.due_date {
                Some(d) if d < today() => v.overdue.push(t.clone()),
                Some(d) if d == today() => v.today.push(t.clone()),
                Some(_) => v.upcoming.push(t.clone()),
                None => v.undated.push(t.clone()),
            }
        }
        v
    }

    pub fn complete_task(&mut self, id: &str) -> Result<(), String> {
        let t = self
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or("görev bulunamadı")?;
        t.open = false;
        Ok(())
    }

    #[must_use]
    pub fn morning_plan(&self) -> MorningPlanView {
        let v = self.tasks();
        let mut items: Vec<PlanItemView> = Vec::new();
        for (t, kind) in v
            .overdue
            .iter()
            .map(|t| (t, "overdue"))
            .chain(v.today.iter().map(|t| (t, "today")))
            .chain(
                v.undated
                    .iter()
                    .filter(|t| t.priority == "high")
                    .map(|t| (t, "high_priority")),
            )
        {
            items.push(PlanItemView {
                task_id: t.id.clone(),
                title: t.title.clone(),
                reason: match kind {
                    "overdue" => format!("{} gün gecikti", t.overdue_days),
                    "today" => "bugün".into(),
                    _ => "öncelikli".into(),
                },
                reason_kind: kind.into(),
                time: t.due_time,
            });
        }
        let more = items.len().saturating_sub(3);
        items.truncate(3);
        MorningPlanView {
            date: today(),
            headline: format!("Günaydın Gökhan. Bugün {} iş öne çıkıyor", items.len()),
            items,
            more,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_groups_newest_last_with_labels() {
        let d = Demo::new();
        let j = d.journal(None);
        let labels: Vec<_> = j.groups.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(labels, ["Çarşamba, 4 Kasım", "Dün", "Bugün"]);
        assert_eq!(j.unprocessed, 3);
        assert_eq!(j.groups[2].items[0].time, "09:12");
    }

    #[test]
    fn deciding_all_suggestions_processes_entry() {
        let mut d = Demo::new();
        d.decide("s2", true).unwrap();
        let e = d.journal(None).groups[2].items[0].clone();
        assert_eq!(e.topic.unwrap().name, "Kostik tedarik");
        assert!(!e.unprocessed);
        assert!(d.decide("s2", true).is_err());
    }

    #[test]
    fn capture_and_topic_flow() {
        let mut d = Demo::new();
        d.capture("Numune kutusu hazırla", Some("t-gem")).unwrap();
        assert!(d.capture("   ", None).is_err());
        let flow = d.journal(Some("t-gem"));
        let n: usize = flow.groups.iter().map(|g| g.items.len()).sum();
        assert_eq!(n, 3);
    }

    #[test]
    fn plan_caps_at_three() {
        let d = Demo::new();
        let p = d.morning_plan();
        assert_eq!(p.items.len(), 3);
        assert_eq!(p.more, 1);
        assert_eq!(p.items[0].reason, "1 gün gecikti");
    }
}
