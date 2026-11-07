//! NotAI API sözleşmesi.
//!
//! Sunucu (`notai-server`), Tauri kabuğu (`notai-app`) ve Leptos arayüzü (`notai-ui`) aynı
//! JSON tiplerini kullanır. Türkçe etiketler (çip metni, saat, gün ayracı) üretici tarafta
//! hazırlanır; arayüz yalnız gösterir. Kimlikler metin olarak taşınır (WASM tarafında UUID
//! bağımlılığı gerekmesin diye).

#![forbid(unsafe_code)]

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(feature = "convert")]
pub mod convert;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuggestionView {
    pub id: String,
    /// topic | task | tag | person | reminder | question
    pub kind: String,
    /// Çip metni ("Görev · Perşembe 12 Kas", "→ GemOlive").
    pub chip: String,
    /// pending | accepted | dismissed
    pub decision: String,
    pub confidence: f64,
    /// AI sorusuna verilen cevap (yalnız `question`).
    #[serde(default)]
    pub answer: Option<String>,
}

impl SuggestionView {
    #[must_use]
    pub fn is_pending(&self) -> bool {
        self.decision == "pending"
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntryView {
    pub id: String,
    pub text: String,
    /// Yerel saat ("09:12").
    pub time: String,
    pub captured_at: DateTime<Utc>,
    /// app | voice | whats_app | share
    pub source: String,
    /// inbox | processed | archived | trashed
    pub status: String,
    pub topic: Option<TopicRef>,
    pub unprocessed: bool,
    pub ai_failed: bool,
    pub suggestions: Vec<SuggestionView>,
    pub tags: Vec<String>,
    pub people: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DayGroup<T> {
    pub date: NaiveDate,
    /// "Bugün", "Dün", "Çarşamba, 4 Kasım", "28 Ekim".
    pub label: String,
    pub items: Vec<T>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalView {
    pub groups: Vec<DayGroup<EntryView>>,
    pub unprocessed: u64,
    /// Daha eskileri yüklemek için `before` imleci (yoksa başa ulaşıldı).
    pub next_before: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicRowView {
    pub id: String,
    pub name: String,
    /// project | area | resource | archive
    pub kind: String,
    pub kind_label: String,
    pub due_date: Option<NaiveDate>,
    /// Gösterilecek simge: elle seçilen, yoksa addan/son kayıtlardan seçilen, yoksa türün
    /// varsayılanı (`notai_core::look`).
    pub icon: Option<String>,
    /// Kalıcı renk yuvası 1–8 (tokens `topic-N-*`); 0 = renksiz (eski sunucu).
    #[serde(default)]
    pub color: u8,
    /// Profil fotoğrafı (`data:image/…`); varsa simgenin yerine yuvarlak içinde gösterilir.
    #[serde(default)]
    pub avatar: Option<String>,
    /// Karar bekleyen önerisi olan kayıt sayısı (rozet; alt konular dahil).
    #[serde(default)]
    pub pending: u64,
    /// Üst konu (hiyerarşi; `None` = en üst).
    #[serde(default)]
    pub parent_id: Option<String>,
    /// Yol adı ("Sabun işi / Malzeme hareketleri").
    #[serde(default)]
    pub path: String,
    /// Doğrudan alt konu sayısı (>0 ise konu açılınca alt konular listelenir).
    #[serde(default)]
    pub children: u32,
    pub pinned: bool,
    /// "12:05", "Dün", "Çarşamba", "2 Kas".
    pub last_label: Option<String>,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicsView {
    pub topics: Vec<TopicRowView>,
    /// Ana Günlük rozeti.
    pub unprocessed: u64,
    /// Ana Günlük satırı: son kaydın önizlemesi ve zaman etiketi.
    pub journal_preview: Option<String>,
    pub journal_last_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRowView {
    pub id: String,
    pub title: String,
    pub topic_id: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub due_time: Option<NaiveTime>,
    /// low | normal | high
    pub priority: String,
    pub priority_label: String,
    pub overdue_days: i64,
    pub open: bool,
    pub recurring: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TasksView {
    pub overdue: Vec<TaskRowView>,
    pub today: Vec<TaskRowView>,
    pub upcoming: Vec<TaskRowView>,
    pub undated: Vec<TaskRowView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanItemView {
    pub task_id: String,
    pub title: String,
    /// "1 gün gecikti", "bugün", "öncelikli".
    pub reason: String,
    /// overdue | today | high_priority
    pub reason_kind: String,
    pub time: Option<NaiveTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MorningPlanView {
    pub date: NaiveDate,
    pub headline: String,
    pub items: Vec<PlanItemView>,
    /// "+N görev daha".
    pub more: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionView {
    pub suggestion: SuggestionView,
    pub entry_status: String,
    pub entry_topic: Option<String>,
    pub created_task: Option<TaskRowView>,
    pub created_topic: Option<TopicRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderView {
    pub task_id: String,
    pub title: String,
    pub topic_id: Option<String>,
    pub fire_at: DateTime<Utc>,
    /// "11:30", "Yarın 09:00", "Paz 8 Kas 19:00".
    pub when: String,
    pub recurring: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderGroupView {
    /// missed | today | tomorrow | this_week | later | recurring
    pub key: String,
    pub label: String,
    pub items: Vec<ReminderView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RemindersView {
    pub groups: Vec<ReminderGroupView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarDayView {
    pub date: NaiveDate,
    /// "Cum 6 Kas".
    pub label: String,
    pub is_today: bool,
    pub items: Vec<TaskRowView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CalendarView {
    pub days: Vec<CalendarDayView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StalledView {
    pub topic_id: String,
    pub name: String,
    /// "10 gündür hareket yok" / "Hiç hareket yok".
    pub idle: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeeklyReviewView {
    pub week_start: NaiveDate,
    pub unprocessed: u64,
    pub stalled: Vec<StalledView>,
    pub upcoming: Vec<TaskRowView>,
    pub overdue: usize,
    pub done_last_week: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceView {
    pub id: String,
    pub text: String,
    /// "3 Kasım 10:00".
    pub when: String,
    pub topic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskView {
    /// AI kapalıysa ya da cevap üretilemediyse `None`; ilgili kayıtlar yine döner.
    pub answer: Option<String>,
    pub not_found: bool,
    pub sources: Vec<SourceView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingsView {
    /// Yüksek güvenli önerileri kendiliğinden uygula (geri alınabilir).
    pub autonomous: bool,
    /// Otonom uygulama eşiği (0.5–1.0).
    pub threshold: f64,
    /// Sunucuda hangi AI sağlayıcı çalışıyor ("Kapalı", "Yerel model", "Claude API").
    pub ai_provider: String,
    pub voice_enabled: bool,
    pub semantic_search: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentView {
    pub id: String,
    /// photo | document | location | contact | audio
    pub kind: String,
    pub name: String,
    pub mime: String,
    pub size: u64,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactView {
    pub id: String,
    pub text: String,
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrashItemView {
    pub id: String,
    pub text: String,
    /// "3 gün önce silindi · 27 gün sonra kalıcı silinir".
    pub note: String,
}

// ---- İstekler ----

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureRequest {
    pub text: String,
    #[serde(default)]
    pub topic_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaskEditRequest {
    pub title: Option<String>,
    /// Boş dizgi tarihi kaldırır.
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub priority: Option<String>,
    /// day | weekday | week | month | none
    pub repeat: Option<String>,
    /// Görevden kaç dakika önce hatırlat: 0 = tam zamanında, negatif = hatırlatmayı kaldır.
    pub remind_before_minutes: Option<i32>,
    pub topic_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TopicEditRequest {
    pub name: Option<String>,
    pub kind: Option<String>,
    pub due_date: Option<String>,
    pub icon: Option<String>,
    /// Profil fotoğrafı (`data:image/…;base64,…`); boş dizgi kaldırır.
    #[serde(default)]
    pub avatar: Option<String>,
    /// Üst konu kimliği; boş dizgi en üste taşır.
    #[serde(default)]
    pub parent_id: Option<String>,
    pub pinned: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveRequest {
    pub topic_id: Option<String>,
}

/// Son bildirim ("son bildirim" şeridi; dokununca konusu açılır).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoticeView {
    /// reminder | question
    pub kind: String,
    pub title: String,
    pub body: String,
    pub topic_id: Option<String>,
    pub topic_name: Option<String>,
    pub entry_id: Option<String>,
}

/// AI sorusuna cevap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerRequest {
    pub text: String,
}

/// Konu menüsündeki "Simgeyi değiştir" seçenekleri: (simge, Türkçe ad). Otomatik seçimin
/// kullandığı simgelerle aynı küme (`notai_core::look::auto_icons`; testle eşitlenir) + türlerin
/// varsayılanları.
pub const TOPIC_ICON_CHOICES: &[(&str, &str)] = &[
    ("flask", "Kimya"),
    ("drop", "Yağ, sabun"),
    ("leaf", "Bahçe"),
    ("cube", "Ürün"),
    ("truck", "Kargo"),
    ("factory", "Üretim"),
    ("chart", "Satış, yatırım"),
    ("wallet", "Para"),
    ("cart", "Alışveriş"),
    ("house", "Ev"),
    ("car", "Araba"),
    ("plane", "Seyahat"),
    ("pill", "Sağlık"),
    ("dumbbell", "Spor"),
    ("heart", "İlişki"),
    ("users", "Aile"),
    ("grad", "Eğitim"),
    ("book", "Okuma"),
    ("code", "Yazılım"),
    ("server", "Sunucu"),
    ("cpu", "Bilgisayar"),
    ("wrench", "Tamir"),
    ("briefcase", "İş"),
    ("music", "Müzik"),
    ("gift", "Hediye"),
    ("paw", "Evcil hayvan"),
    ("utensils", "Yemek"),
    ("camera", "Fotoğraf"),
    ("calendar", "Etkinlik"),
    ("folder", "Klasör"),
    ("layers", "Katman"),
    ("archive", "Arşiv"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestion_pending_helper() {
        let s = SuggestionView {
            id: "1".into(),
            kind: "tag".into(),
            chip: "#sabun".into(),
            decision: "pending".into(),
            confidence: 0.8,
            answer: None,
        };
        assert!(s.is_pending());
    }

    #[cfg(feature = "convert")]
    #[test]
    fn icon_choices_cover_every_auto_icon() {
        let choices: Vec<&str> = TOPIC_ICON_CHOICES.iter().map(|(i, _)| *i).collect();
        for i in notai_core::look::auto_icons() {
            assert!(choices.contains(&i), "seçeneklerde yok: {i}");
        }
    }

    #[test]
    fn capture_request_topic_is_optional() {
        let r: CaptureRequest = serde_json::from_str(r#"{"text":"Kostik ara"}"#).unwrap();
        assert_eq!(r.topic_id, None);
    }
}
