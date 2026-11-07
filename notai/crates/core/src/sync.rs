//! Senkron birleştirme kuralları (saf mantık).
//!
//! Temel kural **son yazan kazanır** (LWW): her değişikliğin bir sürümü vardır
//! (zaman, kaynak cihaz); daha yeni sürüm kazanır, eşitlikte cihaz adı sırası belirler
//! (iki cihaz aynı sonuca varır).
//!
//! Alan bazlı istisnalar:
//! - Kayıt metni değişmez; yalnız konu/durum alanları birleştirilir.
//! - Karara bağlanmış AI önerisi asla "bekliyor"a dönmez (karar, zamandan bağımsız kazanır).
//! - Etiketler yalnız büyüyen kümedir (eklenen etiket kaybolmaz).
//! - Kalıcı silme (çöpte 30 günü dolmuş kayıt) her zaman kazanır.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::suggestion::Decision;

/// Bir varlığın sürümü.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
    pub at: DateTime<Utc>,
    pub origin: String,
}

impl Version {
    #[must_use]
    pub fn new(at: DateTime<Utc>, origin: impl Into<String>) -> Self {
        Self {
            at,
            origin: origin.into(),
        }
    }
}

/// Uzaktaki sürüm yereldekini geçer mi? Yerelde hiç sürüm yoksa geçer.
#[must_use]
pub fn remote_wins(local: Option<&Version>, remote: &Version) -> bool {
    local.is_none_or(|l| (remote.at, &remote.origin) > (l.at, &l.origin))
}

/// Öneri kararı birleştirmesi: kararlı > bekleyen; ikisi de kararlıysa LWW.
#[must_use]
pub fn suggestion_remote_wins(
    local: Decision,
    remote: Decision,
    local_version: Option<&Version>,
    remote_version: &Version,
) -> bool {
    match (local, remote) {
        (Decision::Pending, Decision::Pending) => false,
        (Decision::Pending, _) => true,
        (_, Decision::Pending) => false,
        _ => remote_wins(local_version, remote_version),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn v(sec: u32, origin: &str) -> Version {
        Version::new(
            Utc.with_ymd_and_hms(2026, 11, 6, 9, 0, sec).unwrap(),
            origin,
        )
    }

    #[test]
    fn newer_wins_and_ties_are_deterministic() {
        assert!(remote_wins(None, &v(0, "telefon")));
        assert!(remote_wins(Some(&v(1, "telefon")), &v(2, "masaustu")));
        assert!(!remote_wins(Some(&v(2, "telefon")), &v(1, "masaustu")));
        // Aynı an: cihaz adı sırası; iki taraf da aynı sonucu bulur.
        assert!(remote_wins(Some(&v(1, "masaustu")), &v(1, "telefon")));
        assert!(!remote_wins(Some(&v(1, "telefon")), &v(1, "masaustu")));
        // Aynı sürüm tekrar uygulanmaz (idempotent).
        assert!(!remote_wins(Some(&v(1, "telefon")), &v(1, "telefon")));
    }

    #[test]
    fn decided_suggestion_never_reverts_to_pending() {
        let at = Utc.with_ymd_and_hms(2026, 11, 6, 9, 0, 0).unwrap();
        let accepted = Decision::Accepted { at };
        let dismissed = Decision::Dismissed { at };
        // Eski bir "bekliyor" kopyası, yeni zaman damgasıyla gelse de kararı ezmez.
        assert!(!suggestion_remote_wins(
            accepted,
            Decision::Pending,
            Some(&v(0, "a")),
            &v(9, "b")
        ));
        // Yerel bekliyorken gelen karar, eski olsa bile uygulanır.
        assert!(suggestion_remote_wins(
            Decision::Pending,
            dismissed,
            Some(&v(9, "a")),
            &v(0, "b")
        ));
        // İki karar çatışırsa LWW.
        assert!(suggestion_remote_wins(
            accepted,
            dismissed,
            Some(&v(1, "a")),
            &v(2, "b")
        ));
        assert!(!suggestion_remote_wins(
            Decision::Pending,
            Decision::Pending,
            None,
            &v(2, "b")
        ));
    }
}
