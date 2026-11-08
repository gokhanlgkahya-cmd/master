//! Kayıt (Entry): yakalanan her şey. Durum makinesi burada.
//!
//! ```text
//!   capture ──► Gelen kutusu ──(öneriler karara bağlandı / konu atandı)──► İşlendi
//!                    │                                                    │  ▲
//!                    └──────────────► Arşiv ◄──────────────────────────────┘  │ (arşivden çıkar)
//!   her durum (çöp hariç) ──► Çöp kutusu (30 gün) ──(geri al)──► Gelen kutusu / İşlendi
//! ```

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::ids::{EntryId, TopicId};
use crate::text::EntryText;

/// Çöp kutusunda bekleme süresi; sonra kalıcı silinebilir.
pub const TRASH_RETENTION_DAYS: i64 = 30;

/// Kaydın nereden geldiği.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// Uygulamadaki yazma çubuğu.
    App,
    /// Sesli yakalama.
    Voice,
    /// Eski WhatsApp yakalaması (2026-11-08 kaldırıldı; eski kayıtlar okunabilsin diye duruyor).
    WhatsApp,
    /// Android "paylaş" menüsü.
    Share,
}

impl Source {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Voice => "voice",
            Self::WhatsApp => "whats_app",
            Self::Share => "share",
        }
    }

    pub fn parse(s: &str) -> Result<Self, CoreError> {
        match s {
            "app" => Ok(Self::App),
            "voice" => Ok(Self::Voice),
            "whats_app" => Ok(Self::WhatsApp),
            "share" => Ok(Self::Share),
            other => Err(CoreError::UnknownVariant {
                field: "kaynak",
                value: other.to_owned(),
            }),
        }
    }
}

/// Kaydın yaşam döngüsü durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum EntryStatus {
    /// Gelen kutusu: henüz sınıflandırılmadı (Ana Günlük'te işaretli görünür).
    Inbox,
    /// İşlendi: konu atandı ya da tüm öneriler karara bağlandı.
    Processed,
    /// Arşivlendi.
    Archived,
    /// Çöp kutusunda; `at` zamanından 30 gün sonra kalıcı silinebilir.
    Trashed { at: DateTime<Utc> },
}

impl EntryStatus {
    /// Veritabanındaki ad.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Inbox => "inbox",
            Self::Processed => "processed",
            Self::Archived => "archived",
            Self::Trashed { .. } => "trashed",
        }
    }

    /// Veritabanı sütunlarından durumu kurar; çelişkili birleşimleri reddeder.
    pub fn from_parts(name: &str, trashed_at: Option<DateTime<Utc>>) -> Result<Self, CoreError> {
        match (name, trashed_at) {
            ("inbox", None) => Ok(Self::Inbox),
            ("processed", None) => Ok(Self::Processed),
            ("archived", None) => Ok(Self::Archived),
            ("trashed", Some(at)) => Ok(Self::Trashed { at }),
            (other, _) => Err(CoreError::UnknownVariant {
                field: "kayıt durumu",
                value: other.to_owned(),
            }),
        }
    }

    #[must_use]
    pub const fn trashed_at(self) -> Option<DateTime<Utc>> {
        match self {
            Self::Trashed { at } => Some(at),
            _ => None,
        }
    }
}

/// Yakalanan bir kayıt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub id: EntryId,
    pub text: EntryText,
    pub source: Source,
    pub captured_at: DateTime<Utc>,
    topic: Option<TopicId>,
    status: EntryStatus,
    /// NotAI'nin yazdığı kayıt (ör. fiyat takibi mesajı); kullanıcınınkinden ayrı gösterilir.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub from_notai: bool,
}

impl Entry {
    /// Yeni kayıt yakalar. Doğrudan bir konuya yazılan kayıt kullanıcı tarafından zaten
    /// sınıflandırılmış sayılır ve `İşlendi` başlar; Ana Günlük'e yazılan `Gelen kutusu`.
    #[must_use]
    pub fn capture(
        text: EntryText,
        source: Source,
        captured_at: DateTime<Utc>,
        topic: Option<TopicId>,
    ) -> Self {
        let status = if topic.is_some() {
            EntryStatus::Processed
        } else {
            EntryStatus::Inbox
        };
        Self {
            id: EntryId::new(),
            text,
            source,
            captured_at,
            topic,
            status,
            from_notai: false,
        }
    }

    /// Depodan okunan parçalardan kaydı yeniden kurar.
    #[must_use]
    pub const fn from_parts(
        id: EntryId,
        text: EntryText,
        source: Source,
        captured_at: DateTime<Utc>,
        topic: Option<TopicId>,
        status: EntryStatus,
    ) -> Self {
        Self {
            id,
            text,
            source,
            captured_at,
            topic,
            status,
            from_notai: false,
        }
    }

    #[must_use]
    pub const fn status(&self) -> EntryStatus {
        self.status
    }

    #[must_use]
    pub const fn topic(&self) -> Option<TopicId> {
        self.topic
    }

    /// Ana Günlük'te "işlenmemiş" işareti gösterilmeli mi?
    #[must_use]
    pub const fn is_unprocessed(&self) -> bool {
        matches!(self.status, EntryStatus::Inbox)
    }

    #[must_use]
    pub const fn is_trashed(&self) -> bool {
        matches!(self.status, EntryStatus::Trashed { .. })
    }

    /// Gelen kutusundan çıkarır. İşlenmiş kayıtta etkisizdir (tekrar çağrılabilir).
    pub fn mark_processed(&mut self) -> Result<(), CoreError> {
        match self.status {
            EntryStatus::Inbox | EntryStatus::Processed => {
                self.status = EntryStatus::Processed;
                Ok(())
            }
            other => Err(self.invalid(other, "processed")),
        }
    }

    /// Kaydı bir konuya taşır (ya da konusundan çıkarır). Kullanıcı konu seçtiyse kayıt
    /// işlenmiş sayılır. Çöpteki kayıt taşınamaz.
    pub fn assign_topic(&mut self, topic: Option<TopicId>) -> Result<(), CoreError> {
        match self.status {
            EntryStatus::Trashed { .. } => Err(self.invalid(self.status, "topic")),
            EntryStatus::Inbox => {
                self.topic = topic;
                if topic.is_some() {
                    self.status = EntryStatus::Processed;
                }
                Ok(())
            }
            EntryStatus::Processed | EntryStatus::Archived => {
                self.topic = topic;
                Ok(())
            }
        }
    }

    pub fn archive(&mut self) -> Result<(), CoreError> {
        match self.status {
            EntryStatus::Inbox | EntryStatus::Processed => {
                self.status = EntryStatus::Archived;
                Ok(())
            }
            other => Err(self.invalid(other, "archived")),
        }
    }

    pub fn unarchive(&mut self) -> Result<(), CoreError> {
        match self.status {
            EntryStatus::Archived => {
                self.status = EntryStatus::Processed;
                Ok(())
            }
            other => Err(self.invalid(other, "processed")),
        }
    }

    /// "Sil" = çöp kutusu. Kalıcı silme değildir.
    pub fn trash(&mut self, now: DateTime<Utc>) -> Result<(), CoreError> {
        match self.status {
            EntryStatus::Trashed { .. } => Err(self.invalid(self.status, "trashed")),
            _ => {
                self.status = EntryStatus::Trashed { at: now };
                Ok(())
            }
        }
    }

    /// Çöp kutusundan geri alır: konusu varsa İşlendi, yoksa Gelen kutusu.
    pub fn restore(&mut self) -> Result<(), CoreError> {
        match self.status {
            EntryStatus::Trashed { .. } => {
                self.status = if self.topic.is_some() {
                    EntryStatus::Processed
                } else {
                    EntryStatus::Inbox
                };
                Ok(())
            }
            other => Err(self.invalid(other, "restored")),
        }
    }

    /// Kalıcı silinebilir mi? Yalnız çöpteki ve 30 günü dolmuş kayıtlar.
    #[must_use]
    pub fn can_purge(&self, now: DateTime<Utc>) -> bool {
        match self.status {
            EntryStatus::Trashed { at } => now - at >= Duration::days(TRASH_RETENTION_DAYS),
            _ => false,
        }
    }

    fn invalid(&self, from: EntryStatus, to: &'static str) -> CoreError {
        let _ = self;
        CoreError::InvalidTransition {
            from: from.name(),
            to,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, day, 9, 12, 0).unwrap()
    }

    fn inbox() -> Entry {
        Entry::capture(
            EntryText::new("Kostik tedarikçisini ara – perşembe").unwrap(),
            Source::App,
            at(6),
            None,
        )
    }

    #[test]
    fn capture_to_journal_starts_in_inbox() {
        let e = inbox();
        assert_eq!(e.status(), EntryStatus::Inbox);
        assert!(e.is_unprocessed());
    }

    #[test]
    fn capture_to_topic_starts_processed() {
        let e = Entry::capture(
            EntryText::new("Ambalaj faturası").unwrap(),
            Source::App,
            at(6),
            Some(TopicId::new()),
        );
        assert_eq!(e.status(), EntryStatus::Processed);
        assert!(!e.is_unprocessed());
    }

    #[test]
    fn assigning_topic_processes_inbox_entry() {
        let mut e = inbox();
        let topic = TopicId::new();
        e.assign_topic(Some(topic)).unwrap();
        assert_eq!(e.topic(), Some(topic));
        assert_eq!(e.status(), EntryStatus::Processed);
    }

    #[test]
    fn mark_processed_is_idempotent() {
        let mut e = inbox();
        e.mark_processed().unwrap();
        e.mark_processed().unwrap();
        assert_eq!(e.status(), EntryStatus::Processed);
    }

    #[test]
    fn archive_and_unarchive() {
        let mut e = inbox();
        e.archive().unwrap();
        assert_eq!(e.status(), EntryStatus::Archived);
        assert!(e.archive().is_err());
        e.unarchive().unwrap();
        assert_eq!(e.status(), EntryStatus::Processed);
    }

    #[test]
    fn trash_restore_cycle_keeps_data() {
        let mut e = inbox();
        e.trash(at(6)).unwrap();
        assert!(e.is_trashed());
        assert_eq!(
            e.trash(at(7)).unwrap_err(),
            CoreError::InvalidTransition {
                from: "trashed",
                to: "trashed"
            }
        );
        e.restore().unwrap();
        assert_eq!(e.status(), EntryStatus::Inbox);
        assert_eq!(e.text.as_str(), "Kostik tedarikçisini ara – perşembe");
    }

    #[test]
    fn restore_with_topic_goes_to_processed() {
        let mut e = inbox();
        e.assign_topic(Some(TopicId::new())).unwrap();
        e.trash(at(6)).unwrap();
        e.restore().unwrap();
        assert_eq!(e.status(), EntryStatus::Processed);
    }

    #[test]
    fn trashed_entry_cannot_change_topic_or_be_processed() {
        let mut e = inbox();
        e.trash(at(6)).unwrap();
        assert!(e.assign_topic(Some(TopicId::new())).is_err());
        assert!(e.mark_processed().is_err());
        assert!(e.archive().is_err());
    }

    #[test]
    fn purge_only_after_retention() {
        let mut e = inbox();
        assert!(!e.can_purge(at(30)));
        e.trash(at(1)).unwrap();
        assert!(!e.can_purge(at(30)));
        assert!(e.can_purge(Utc.with_ymd_and_hms(2026, 12, 1, 9, 12, 0).unwrap()));
    }

    #[test]
    fn status_parts_round_trip_and_reject_inconsistency() {
        for s in [
            EntryStatus::Inbox,
            EntryStatus::Processed,
            EntryStatus::Archived,
            EntryStatus::Trashed { at: at(3) },
        ] {
            assert_eq!(
                EntryStatus::from_parts(s.name(), s.trashed_at()).unwrap(),
                s
            );
        }
        assert!(EntryStatus::from_parts("trashed", None).is_err());
        assert!(EntryStatus::from_parts("inbox", Some(at(1))).is_err());
        assert!(EntryStatus::from_parts("silindi", None).is_err());
    }

    #[test]
    fn source_round_trip() {
        for s in [Source::App, Source::Voice, Source::WhatsApp, Source::Share] {
            assert_eq!(Source::parse(s.as_str()).unwrap(), s);
        }
        assert!(Source::parse("telgraf").is_err());
    }
}
