//! Obsidian kasası (Gökhan, 2026-11-07): NotAI verisi Markdown dosyalarına dökülür; Obsidian'da
//! açılır, Hermes / Claude Code doğrudan okur. Saf fonksiyon: girdi → (yol, içerik) listesi.
//!
//! Düzen (kasa kökünde `NotAI/` altında):
//! - `Konular/<Konu>/<Konu>.md` — klasör notu; alt konular iç içe klasörler. Konunun **kendi**
//!   kayıtları ve görevleri burada durur (her kayıt tek yerde, `^n…` blok kimliğiyle).
//! - `Ana Günlük/YYYY-AA-GG.md` — o günün tüm kayıtları: konusuz kayıtlar burada durur, konulu
//!   kayıtlar ise yalnız bağlantı + kısa önizleme olarak görünür (kopya yok; kimlik aynı).
//! - `Görevler.md` — açık görevler (Obsidian Tasks biçimi), `Profil.md` — onaylı bilgiler.
//!
//! Her dosyanın başında `notai_id` / `notai_type` ön bilgisi vardır: araçlar mükerrer saymaz.

use std::collections::{BTreeMap, HashMap};

use chrono::NaiveDate;
use chrono_tz::Tz;

use crate::entry::{Entry, EntryStatus};
use crate::profile::{FactStatus, ProfileFact};
use crate::task::{Priority, Task, TaskState};
use crate::topic::{ParaKind, Topic, Tree};
use crate::{EntryId, TopicId};

/// Kasadaki kök klasör.
pub const ROOT: &str = "NotAI";

/// Dışa aktarılacak bir kayıt (etiket ve kişileriyle).
#[derive(Debug, Clone)]
pub struct VaultEntry {
    pub entry: Entry,
    pub tags: Vec<String>,
    pub people: Vec<String>,
}

/// Kasaya yazılacak dosya.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultFile {
    /// Kasa köküne göre yol ("NotAI/Konular/Sabun işi/Sabun işi.md").
    pub path: String,
    pub content: String,
}

/// Dosya adında kullanılamayan karakterleri temizler (Obsidian + Windows + Android).
#[must_use]
pub fn file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '#' | '^' | '[' | ']' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .collect();
    let t = cleaned.trim().trim_matches('.').trim();
    if t.is_empty() {
        "-".to_owned()
    } else {
        t.to_owned()
    }
}

/// Kayıt blok kimliği (Obsidian `^…` başvurusu): "n" + kimliğin son 12 onaltılık hanesi.
#[must_use]
pub fn block_id(id: EntryId) -> String {
    let s = id.to_string().replace('-', "");
    let tail = s.get(s.len().saturating_sub(12)..).unwrap_or(&s);
    format!("n{tail}")
}

fn yaml_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn preview(s: &str, max: usize) -> String {
    let line = one_line(s);
    if line.chars().count() <= max {
        line
    } else {
        let mut p: String = line.chars().take(max).collect();
        p.push('…');
        p
    }
}

struct Paths {
    /// Konu → klasör yolu ("NotAI/Konular/Sabun işi/Para alışverişi").
    dir: HashMap<TopicId, String>,
    names: HashMap<TopicId, String>,
}

impl Paths {
    fn new(topics: &[Topic]) -> Self {
        let tree = Tree::new(topics.iter().map(|t| (t.id, t.parent)));
        let names: HashMap<TopicId, String> = topics
            .iter()
            .map(|t| (t.id, file_name(t.name.as_str())))
            .collect();
        let dir = topics
            .iter()
            .map(|t| {
                let mut parts: Vec<String> = tree
                    .ancestors(t.id)
                    .iter()
                    .rev()
                    .filter_map(|a| names.get(a).cloned())
                    .collect();
                parts.push(names.get(&t.id).cloned().unwrap_or_default());
                (t.id, format!("{ROOT}/Konular/{}", parts.join("/")))
            })
            .collect();
        Self { dir, names }
    }

    /// Konu notunun yolu (uzantısız; bağlantı için).
    fn note(&self, id: TopicId) -> Option<String> {
        Some(format!("{}/{}", self.dir.get(&id)?, self.names.get(&id)?))
    }

    /// `[[yol|ad]]` bağlantısı.
    fn link(&self, id: TopicId, display: &str) -> String {
        self.note(id)
            .map_or_else(|| display.to_owned(), |n| format!("[[{n}|{display}]]"))
    }
}

fn priority_mark(p: Priority) -> &'static str {
    match p {
        Priority::High => " ⏫",
        Priority::Normal => "",
        Priority::Low => " 🔽",
    }
}

fn task_line(t: &Task, tz: Tz) -> String {
    let (box_, done) = match t.state() {
        TaskState::Open => ("[ ]", String::new()),
        TaskState::Done { at } => (
            "[x]",
            format!(" ✅ {}", at.with_timezone(&tz).format("%Y-%m-%d")),
        ),
        TaskState::Cancelled { at } => (
            "[-]",
            format!(" ❌ {}", at.with_timezone(&tz).format("%Y-%m-%d")),
        ),
    };
    let due = t.due.map_or_else(String::new, |d| {
        let time = d
            .time
            .map_or_else(String::new, |x| format!(" ⏰ {}", x.format("%H:%M")));
        format!(" 📅 {}{time}", d.date.format("%Y-%m-%d"))
    });
    let repeat = t.recurrence.map_or("", |_| " 🔁");
    format!(
        "- {box_} {}{due}{repeat}{}{done}",
        one_line(t.title.as_str()),
        priority_mark(t.priority)
    )
}

fn labels(e: &VaultEntry) -> String {
    let mut s = String::new();
    for t in &e.tags {
        s.push_str(&format!(" #{}", t.replace(' ', "-")));
    }
    for p in &e.people {
        s.push_str(&format!(" @{}", p.replace(' ', "_")));
    }
    s
}

/// Kasanın tüm dosyaları. Çıktı deterministiktir (zaman damgası yazılmaz): veri değişmedikçe
/// dosyalar da değişmez, Obsidian/senkron gereksiz yere tetiklenmez.
#[must_use]
pub fn render(
    topics: &[Topic],
    entries: &[VaultEntry],
    tasks: &[Task],
    facts: &[ProfileFact],
    tz: Tz,
) -> Vec<VaultFile> {
    let paths = Paths::new(topics);
    let tree = Tree::new(topics.iter().map(|t| (t.id, t.parent)));
    let by_id: HashMap<TopicId, &Topic> = topics.iter().map(|t| (t.id, t)).collect();
    let visible = |e: &&VaultEntry| !matches!(e.entry.status(), EntryStatus::Trashed { .. });
    let mut files = Vec::new();

    // ---- Konu notları ----
    for t in topics {
        let Some(dir) = paths.dir.get(&t.id) else {
            continue;
        };
        let name = paths.names.get(&t.id).cloned().unwrap_or_default();
        let mut md = String::from("---\n");
        md.push_str(&format!("notai_type: konu\nnotai_id: {}\n", t.id));
        md.push_str(&format!("tür: {}\n", t.kind.label()));
        if let ParaKind::Project { due: Some(d) } = t.kind {
            md.push_str(&format!("bitiş: {}\n", d.format("%Y-%m-%d")));
        }
        if let Some(p) = t.parent {
            md.push_str(&format!(
                "üst: {}\n",
                yaml_str(&paths.link(p, by_id.get(&p).map_or("", |x| x.name.as_str())))
            ));
        }
        md.push_str("---\n\n");
        md.push_str(&format!("# {}\n", t.name));
        let ancestors = tree.ancestors(t.id);
        if !ancestors.is_empty() {
            let crumbs: Vec<String> = ancestors
                .iter()
                .rev()
                .map(|a| paths.link(*a, by_id.get(a).map_or("", |x| x.name.as_str())))
                .collect();
            md.push_str(&format!("\n{} › **{}**\n", crumbs.join(" › "), t.name));
        }
        let children = tree.children(t.id);
        if !children.is_empty() {
            md.push_str("\n## Alt konular\n");
            let mut kids: Vec<&Topic> = children
                .iter()
                .filter_map(|c| by_id.get(c).copied())
                .collect();
            kids.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
            for k in kids {
                md.push_str(&format!("- {}\n", paths.link(k.id, k.name.as_str())));
            }
        }
        let own: Vec<&VaultEntry> = entries
            .iter()
            .filter(visible)
            .filter(|e| e.entry.topic() == Some(t.id))
            .collect();
        if !own.is_empty() {
            md.push_str("\n## Kayıtlar\n");
            let mut by_day: BTreeMap<NaiveDate, Vec<&VaultEntry>> = BTreeMap::new();
            for e in own {
                by_day
                    .entry(e.entry.captured_at.with_timezone(&tz).date_naive())
                    .or_default()
                    .push(e);
            }
            for (day, list) in by_day {
                md.push_str(&format!("\n### [[{ROOT}/Ana Günlük/{day}|{day}]]\n"));
                for e in list {
                    let local = e.entry.captured_at.with_timezone(&tz);
                    let archived = if matches!(e.entry.status(), EntryStatus::Archived) {
                        " (arşiv)"
                    } else {
                        ""
                    };
                    let who = if e.entry.from_notai {
                        "**NotAI:** "
                    } else {
                        ""
                    };
                    md.push_str(&format!(
                        "- {} {who}{}{}{archived} ^{}\n",
                        local.format("%H:%M"),
                        one_line(e.entry.text.as_str()),
                        labels(e),
                        block_id(e.entry.id)
                    ));
                }
            }
        }
        let own_tasks: Vec<&Task> = tasks.iter().filter(|x| x.topic == Some(t.id)).collect();
        if !own_tasks.is_empty() {
            md.push_str("\n## Görevler\n");
            for x in own_tasks {
                md.push_str(&task_line(x, tz));
                md.push('\n');
            }
        }
        files.push(VaultFile {
            path: format!("{dir}/{name}.md"),
            content: md,
        });
    }

    // ---- Ana Günlük (günlük notlar) ----
    let mut days: BTreeMap<NaiveDate, Vec<&VaultEntry>> = BTreeMap::new();
    for e in entries.iter().filter(visible) {
        days.entry(e.entry.captured_at.with_timezone(&tz).date_naive())
            .or_default()
            .push(e);
    }
    for (day, mut list) in days {
        list.sort_by_key(|e| e.entry.captured_at);
        let mut md = format!("---\nnotai_type: günlük\ntarih: {day}\n---\n\n# {day}\n\n");
        for e in list {
            let local = e.entry.captured_at.with_timezone(&tz);
            match e.entry.topic() {
                // Konulu kayıt: tek kopyası konu notunda; burada bağlantı + önizleme.
                Some(t) => {
                    let target = paths
                        .note(t)
                        .map(|n| {
                            format!(
                                "[[{n}#^{}|{}]]",
                                block_id(e.entry.id),
                                by_id.get(&t).map_or("", |x| x.name.as_str())
                            )
                        })
                        .unwrap_or_default();
                    md.push_str(&format!(
                        "- {} · {target} — {}\n",
                        local.format("%H:%M"),
                        preview(e.entry.text.as_str(), 80)
                    ));
                }
                // Konusuz (işlenmemiş) kayıt: tek kopyası burada.
                None => md.push_str(&format!(
                    "- {} {}{} #işlenmemiş ^{}\n",
                    local.format("%H:%M"),
                    one_line(e.entry.text.as_str()),
                    labels(e),
                    block_id(e.entry.id)
                )),
            }
        }
        files.push(VaultFile {
            path: format!("{ROOT}/Ana Günlük/{day}.md"),
            content: md,
        });
    }

    // ---- Görevler (konusuz + özet) ----
    let mut md = String::from("---\nnotai_type: görevler\n---\n\n# Görevler\n\n");
    md.push_str(
        "Konulu görevler kendi konu notlarında durur; burada yalnız konusuz görevler var.\n\n",
    );
    let loose: Vec<&Task> = tasks.iter().filter(|t| t.topic.is_none()).collect();
    if loose.is_empty() {
        md.push_str("_Konusuz görev yok._\n");
    }
    for t in loose {
        md.push_str(&task_line(t, tz));
        md.push('\n');
    }
    files.push(VaultFile {
        path: format!("{ROOT}/Görevler.md"),
        content: md,
    });

    // ---- Profil ----
    let mut md = String::from("---\nnotai_type: profil\n---\n\n# Profil (NotAI seni tanıyor)\n\n");
    for f in facts.iter().filter(|f| f.status == FactStatus::Confirmed) {
        md.push_str(&format!("- {}\n", one_line(f.text.as_str())));
    }
    files.push(VaultFile {
        path: format!("{ROOT}/Profil.md"),
        content: md,
    });

    // ---- Açıklama ----
    files.push(VaultFile {
        path: format!("{ROOT}/NotAI hakkında.md"),
        content: format!(
            "# NotAI kasası\n\nBu klasörü NotAI ev sunucusu kendiliğinden günceller. `{ROOT}/` içindeki NotAI \
dosyalarını burada değiştirme; değişikliğin bir sonraki güncellemede üzerine yazılır. Kendi notlarını \
bu klasörün dışına ya da yeni dosyalara yazabilirsin — onlara dokunulmaz.\n\n\
- Konular: `Konular/` (alt konular iç içe klasörler)\n\
- Günlük: `Ana Günlük/` (her gün bir not; konulu kayıtlar konu notuna bağlanır)\n\
- Her kayıt tek yerde durur (`^n…` blok kimliği); raporlarda mükerrer sayılmaz.\n"
        ),
    });
    files
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::entry::Source;
    use crate::task::{Due, NewTask};
    use crate::text::{EntryText, FactText, TaskTitle, TopicName};
    use chrono::{TimeZone, Utc};

    const TZ: Tz = chrono_tz::Europe::Istanbul;

    #[test]
    fn vault_layout_links_and_single_copy() {
        let sabun = Topic::new(TopicName::new("Sabun işi").unwrap(), ParaKind::Area);
        let mut para = Topic::new(TopicName::new("Para: alışveriş").unwrap(), ParaKind::Area);
        para.parent = Some(sabun.id);
        let at = Utc.with_ymd_and_hms(2026, 11, 7, 6, 12, 0).unwrap();
        let in_topic = Entry::capture(
            EntryText::new("Ayhan Kimya'ya 1.850 ₺ ödendi").unwrap(),
            Source::App,
            at,
            Some(para.id),
        );
        let loose = Entry::capture(EntryText::new("Kombi").unwrap(), Source::App, at, None);
        let mut nt = NewTask::titled(TaskTitle::new("Fatura öde").unwrap());
        nt.topic = Some(para.id);
        nt.due = Some(Due::on(NaiveDate::from_ymd_opt(2026, 11, 15).unwrap()));
        nt.priority = Priority::High;
        let task = Task::create(nt).unwrap();
        let files = render(
            &[sabun.clone(), para.clone()],
            &[
                VaultEntry {
                    entry: in_topic.clone(),
                    tags: vec!["ödeme".into()],
                    people: vec!["Ayhan Kimya".into()],
                },
                VaultEntry {
                    entry: loose.clone(),
                    tags: vec![],
                    people: vec![],
                },
            ],
            &[task],
            &[ProfileFact::confirmed(
                FactText::new("GemOlive = sabun markam").unwrap(),
                None,
            )],
            TZ,
        );
        let get = |p: &str| {
            files
                .iter()
                .find(|f| f.path == p)
                .map(|f| f.content.clone())
        };
        let para_note = get("NotAI/Konular/Sabun işi/Para- alışveriş/Para- alışveriş.md").unwrap();
        assert!(para_note.contains("notai_type: konu"));
        assert!(
            para_note.contains("[[NotAI/Konular/Sabun işi/Sabun işi|Sabun işi]]"),
            "üst bağlantı"
        );
        let bid = block_id(in_topic.id);
        assert!(para_note.contains(&format!(
            "09:12 Ayhan Kimya'ya 1.850 ₺ ödendi #ödeme @Ayhan_Kimya ^{bid}"
        )));
        assert!(para_note.contains("- [ ] Fatura öde 📅 2026-11-15 ⏫"));
        let sabun_note = get("NotAI/Konular/Sabun işi/Sabun işi.md").unwrap();
        assert!(sabun_note.contains("## Alt konular"));
        assert!(
            !sabun_note.contains("1.850"),
            "alt konu kaydı üst notta kopyalanmaz"
        );
        let day = get("NotAI/Ana Günlük/2026-11-07.md").unwrap();
        assert!(
            day.contains(&format!("#^{bid}|Para: alışveriş]]")),
            "günlükte bağlantı"
        );
        assert_eq!(
            day.matches(&format!("^{bid}")).count(),
            1,
            "blok kimliği yalnız konu notunda tanımlı"
        );
        assert!(day.contains(&format!("Kombi #işlenmemiş ^{}", block_id(loose.id))));
        assert!(
            get("NotAI/Profil.md")
                .unwrap()
                .contains("GemOlive = sabun markam")
        );
        assert!(
            get("NotAI/Görevler.md")
                .unwrap()
                .contains("Konusuz görev yok")
        );
    }

    #[test]
    fn file_names_are_safe() {
        assert_eq!(file_name("Para: alışveriş"), "Para- alışveriş");
        assert_eq!(file_name("a/b\\c?"), "a-b-c-");
        assert_eq!(file_name("  ..  "), "-");
        assert_eq!(file_name("İzmir tatili"), "İzmir tatili");
    }
}
