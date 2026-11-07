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
    /// topic | task | tag | person | reminder
    pub kind: String,
    /// Çip metni ("Görev · Perşembe 12 Kas", "→ GemOlive").
    pub chip: String,
    /// pending | accepted | dismissed
    pub decision: String,
    pub confidence: f64,
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
    pub icon: Option<String>,
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

// ---- İstekler ----

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureRequest {
    pub text: String,
    #[serde(default)]
    pub topic_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveRequest {
    pub topic_id: Option<String>,
}

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
        };
        assert!(s.is_pending());
    }

    #[test]
    fn capture_request_topic_is_optional() {
        let r: CaptureRequest = serde_json::from_str(r#"{"text":"Kostik ara"}"#).unwrap();
        assert_eq!(r.topic_id, None);
    }
}
