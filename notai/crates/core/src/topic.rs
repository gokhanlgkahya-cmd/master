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

/// Profil fotoğrafının en büyük boyutu (data URL karakteri). Arayüz görseli ≈96 px'e küçültür
/// (tipik 4–8 KB); bu sınır yalnız kötüye kullanıma karşıdır.
pub const AVATAR_MAX: usize = 120_000;

/// Bir konu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Topic {
    pub id: TopicId,
    pub name: TopicName,
    pub kind: ParaKind,
    /// Simge anahtarı (ör. "drop", "flask", "house"); arayüz ikon setinden çözer.
    pub icon: Option<String>,
    /// Profil fotoğrafı (küçültülmüş görsel, `data:image/…;base64,…`). Varsa simgenin yerine
    /// yuvarlak içinde gösterilir.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// Üst konu (hiyerarşi: "Sabun işi" → "Malzeme hareketleri"). `None` = en üst.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<TopicId>,
    /// Fiyat / ürün takibi ayarı (varsa).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watch: Option<crate::watch::WatchConfig>,
    /// Varlık bilgisi (menkul / gayrimenkul).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset: Option<crate::asset::AssetInfo>,
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
            avatar: None,
            parent: None,
            watch: None,
            asset: None,
            pinned: false,
        }
    }

    /// Profil fotoğrafını doğrular: yalnız küçük `data:image/(jpeg|png|webp);base64,` görseli.
    pub fn check_avatar(s: &str) -> Result<(), crate::CoreError> {
        let ok_prefix = [
            "data:image/jpeg;base64,",
            "data:image/png;base64,",
            "data:image/webp;base64,",
        ]
        .iter()
        .any(|p| s.starts_with(p));
        if !ok_prefix {
            return Err(crate::CoreError::UnknownVariant {
                field: "profil fotoğrafı",
                value: s.chars().take(24).collect(),
            });
        }
        if s.len() > AVATAR_MAX {
            return Err(crate::CoreError::TooLong {
                field: "profil fotoğrafı",
                max: AVATAR_MAX,
                actual: s.len(),
            });
        }
        Ok(())
    }

    /// Konuyu arşive taşır (PARA'nın "A"sı).
    pub fn archive(&mut self) {
        self.kind = ParaKind::Archive;
        self.pinned = false;
    }
}

// ---------------- Hiyerarşi (Gökhan, 2026-11-07) ----------------
//
// Konular ağaç oluşturur (Obsidian klasörleri gibi). Bir kayıt tek bir konuya bağlıdır; üst
// konuların ve Ana Günlük'ün akışında da *görünür* ama tek kayıttır (raporlarda mükerrer olmaz).

/// En fazla iç içe derinlik (en üst = 1).
pub const MAX_DEPTH: usize = 5;

/// Ağaç işlemleri için konu → üst konu eşlemesi.
#[derive(Debug, Clone, Default)]
pub struct Tree {
    parent: std::collections::HashMap<TopicId, Option<TopicId>>,
}

impl Tree {
    #[must_use]
    pub fn new(pairs: impl IntoIterator<Item = (TopicId, Option<TopicId>)>) -> Self {
        Self {
            parent: pairs.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn parent_of(&self, id: TopicId) -> Option<TopicId> {
        self.parent.get(&id).copied().flatten()
    }

    /// Üst konular (yakından uzağa). Bozuk veride döngü olsa bile sonlanır.
    #[must_use]
    pub fn ancestors(&self, id: TopicId) -> Vec<TopicId> {
        let mut out = Vec::new();
        let mut cur = self.parent_of(id);
        while let Some(p) = cur {
            if p == id || out.contains(&p) || out.len() > self.parent.len() {
                break;
            }
            out.push(p);
            cur = self.parent_of(p);
        }
        out
    }

    /// Doğrudan alt konular.
    #[must_use]
    pub fn children(&self, id: TopicId) -> Vec<TopicId> {
        let mut c: Vec<TopicId> = self
            .parent
            .iter()
            .filter(|(_, p)| **p == Some(id))
            .map(|(c, _)| *c)
            .collect();
        c.sort_unstable();
        c
    }

    /// Konu ve tüm alt konuları.
    #[must_use]
    pub fn subtree(&self, id: TopicId) -> Vec<TopicId> {
        let mut out = vec![id];
        let mut i = 0;
        while i < out.len() {
            for c in self.children(out[i]) {
                if !out.contains(&c) {
                    out.push(c);
                }
            }
            i += 1;
        }
        out
    }

    /// `id` konusunu `new_parent` altına taşımak geçerli mi? (Kendi altına/torununa taşınamaz;
    /// derinlik sınırı aşılamaz.)
    pub fn check_move(&self, id: TopicId, new_parent: Option<TopicId>) -> Result<(), CoreError> {
        let Some(p) = new_parent else {
            return Ok(());
        };
        let sub = self.subtree(id);
        if sub.contains(&p) {
            return Err(CoreError::InvalidTransition {
                from: "konu",
                to: "kendi alt konusu",
            });
        }
        // Yeni yerdeki derinlik + taşınan dalın yüksekliği.
        let parent_depth = self.ancestors(p).len() + 1;
        let height = sub
            .iter()
            .map(|d| self.ancestors(*d).iter().take_while(|a| **a != id).count() + 1)
            .max()
            .unwrap_or(1);
        if parent_depth + height > MAX_DEPTH {
            return Err(CoreError::TooLong {
                field: "konu derinliği",
                max: MAX_DEPTH,
                actual: parent_depth + height,
            });
        }
        Ok(())
    }
}

/// Yol adı: "Sabun işi / Malzeme hareketleri". `names`: konu → ad.
#[must_use]
pub fn path_name(
    tree: &Tree,
    names: &std::collections::HashMap<TopicId, String>,
    id: TopicId,
) -> String {
    let mut parts: Vec<&str> = tree
        .ancestors(id)
        .iter()
        .rev()
        .filter_map(|a| names.get(a).map(String::as_str))
        .collect();
    if let Some(n) = names.get(&id) {
        parts.push(n);
    }
    parts.join(" / ")
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

#[cfg(test)]
mod tree_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn tree_paths_subtree_and_moves() {
        let (sabun, malzeme, para, kostik, ev) = (
            TopicId::new(),
            TopicId::new(),
            TopicId::new(),
            TopicId::new(),
            TopicId::new(),
        );
        let tree = Tree::new([
            (sabun, None),
            (malzeme, Some(sabun)),
            (para, Some(sabun)),
            (kostik, Some(malzeme)),
            (ev, None),
        ]);
        let names: HashMap<TopicId, String> = [
            (sabun, "Sabun işi"),
            (malzeme, "Malzeme hareketleri"),
            (para, "Para alışverişi"),
            (kostik, "Kostik"),
            (ev, "Ev"),
        ]
        .into_iter()
        .map(|(i, n)| (i, n.to_owned()))
        .collect();
        assert_eq!(
            path_name(&tree, &names, kostik),
            "Sabun işi / Malzeme hareketleri / Kostik"
        );
        assert_eq!(tree.ancestors(kostik), [malzeme, sabun]);
        let mut sub = tree.subtree(sabun);
        sub.sort_unstable();
        let mut want = vec![sabun, malzeme, para, kostik];
        want.sort_unstable();
        assert_eq!(sub, want);
        assert_eq!(tree.subtree(ev), [ev]);

        // Döngü yasak: üst konu kendi torununun altına taşınamaz.
        assert!(tree.check_move(sabun, Some(kostik)).is_err());
        assert!(tree.check_move(sabun, Some(sabun)).is_err());
        assert!(tree.check_move(ev, Some(kostik)).is_ok());
        assert!(tree.check_move(kostik, None).is_ok());
        // Derinlik: Sabun dalı (3 kat) Ev'in altına girerse 4; 5'i aşmaz.
        assert!(tree.check_move(sabun, Some(ev)).is_ok());
        let deep = Tree::new([
            (sabun, None),
            (malzeme, Some(sabun)),
            (para, Some(malzeme)),
            (kostik, Some(para)),
            (ev, Some(kostik)),
        ]);
        let extra = TopicId::new();
        let mut t2 = deep.clone();
        t2.parent.insert(extra, None);
        assert!(t2.check_move(extra, Some(ev)).is_err(), "6. kat olmaz");
    }
}
