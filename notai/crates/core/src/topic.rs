//! Konular: PARA yöntemindeki Proje / Alan / Kaynak / Arşiv.
//!
//! Ana Günlük bir konu değildir; tüm kayıtların zaman sıralı görünümüdür.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::ids::TopicId;
use crate::text::TopicName;

/// PARA türü. Bitiş tarihi yalnız Proje'de olabilir; bu tipte başka türlü temsil edilemez.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ParaKind {
    /// Sonu olan iş (ör. "Logo ERP geçişi").
    Project { due: Option<NaiveDate> },
    /// Süreklilik isteyen sorumluluk alanı (ör. "Ev", "Sağlık", "GemOlive").
    Area,
    /// İlgi/başvuru konusu (ör. "Okuma notları").
    Resource,
    /// Etkin olmayan konu.
    Archive,
}

impl ParaKind {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Project { .. } => "project",
            Self::Area => "area",
            Self::Resource => "resource",
            Self::Archive => "archive",
        }
    }

    /// Türkçe görünen ad.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Project { .. } => "Proje",
            Self::Area => "Alan",
            Self::Resource => "Kaynak",
            Self::Archive => "Arşiv",
        }
    }

    pub fn from_parts(name: &str, due: Option<NaiveDate>) -> Result<Self, CoreError> {
        match (name, due) {
            ("project", due) => Ok(Self::Project { due }),
            ("area", None) => Ok(Self::Area),
            ("resource", None) => Ok(Self::Resource),
            ("archive", None) => Ok(Self::Archive),
            ("area" | "resource" | "archive", Some(_)) => Err(CoreError::DueDateOnlyForProjects),
            (other, _) => Err(CoreError::UnknownVariant {
                field: "konu türü",
                value: other.to_owned(),
            }),
        }
    }

    #[must_use]
    pub const fn due(self) -> Option<NaiveDate> {
        match self {
            Self::Project { due } => due,
            _ => None,
        }
    }
}

/// Ana ekrandaki bölmeli kontrol: Tümü | Proje | Alan | Kaynak.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KindFilter {
    /// Arşiv hariç tümü.
    #[default]
    All,
    Projects,
    Areas,
    Resources,
    /// Yalnız arşivlenmiş konular.
    Archived,
}

impl KindFilter {
    #[must_use]
    pub const fn matches(self, kind: ParaKind) -> bool {
        matches!(
            (self, kind),
            (
                Self::All,
                ParaKind::Project { .. } | ParaKind::Area | ParaKind::Resource
            ) | (Self::Projects, ParaKind::Project { .. })
                | (Self::Areas, ParaKind::Area)
                | (Self::Resources, ParaKind::Resource)
                | (Self::Archived, ParaKind::Archive)
        )
    }
}

/// Bir konu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Topic {
    pub id: TopicId,
    pub name: TopicName,
    pub kind: ParaKind,
    /// Simge anahtarı (ör. "drop", "flask", "house"); arayüz ikon setinden çözer.
    pub icon: Option<String>,
    pub pinned: bool,
}

impl Topic {
    #[must_use]
    pub fn new(name: TopicName, kind: ParaKind) -> Self {
        Self {
            id: TopicId::new(),
            name,
            kind,
            icon: None,
            pinned: false,
        }
    }

    /// Konuyu arşive taşır (PARA'nın "A"sı).
    pub fn archive(&mut self) {
        self.kind = ParaKind::Archive;
        self.pinned = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn due_date_only_on_projects() {
        let d = NaiveDate::from_ymd_opt(2026, 12, 31);
        assert_eq!(
            ParaKind::from_parts("project", d).unwrap(),
            ParaKind::Project { due: d }
        );
        assert_eq!(
            ParaKind::from_parts("area", d).unwrap_err(),
            CoreError::DueDateOnlyForProjects
        );
        assert!(ParaKind::from_parts("klasör", None).is_err());
    }

    #[test]
    fn parts_round_trip() {
        for k in [
            ParaKind::Project { due: None },
            ParaKind::Area,
            ParaKind::Resource,
            ParaKind::Archive,
        ] {
            assert_eq!(ParaKind::from_parts(k.name(), k.due()).unwrap(), k);
        }
    }

    #[test]
    fn filter_all_hides_archive() {
        assert!(KindFilter::All.matches(ParaKind::Area));
        assert!(!KindFilter::All.matches(ParaKind::Archive));
        assert!(KindFilter::Archived.matches(ParaKind::Archive));
        assert!(KindFilter::Projects.matches(ParaKind::Project { due: None }));
        assert!(!KindFilter::Areas.matches(ParaKind::Resource));
    }

    #[test]
    fn archiving_unpins() {
        let mut t = Topic::new(TopicName::new("Logo ERP geçişi").unwrap(), ParaKind::Area);
        t.pinned = true;
        t.archive();
        assert_eq!(t.kind, ParaKind::Archive);
        assert!(!t.pinned);
        assert_eq!(t.kind.label(), "Arşiv");
    }
}
