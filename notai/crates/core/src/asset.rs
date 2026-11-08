//! Varlıklar (Gökhan, 2026-11-07): menkul ve gayrimenkuller. Her varlık bir konudur ("Varlıklar"
//! ana konusunun altında): yazışması, hiyerarşisi, Obsidian notu ve senkronu hazırdır. Konu
//! `asset` bilgisi taşır: tür, miktar, değer, isteğe bağlı fiyat sembolü. Sembollü varlıkların
//! birim fiyatını ajan günde bir bildirir; değer güncellenir, büyük değişim konuya yazılır.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::finance::Money;

/// Varlık türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Gayrimenkul,
    Arac,
    Hisse,
    Fon,
    Altin,
    Doviz,
    Kripto,
    Mevduat,
    Diger,
}

impl AssetKind {
    pub const ALL: [Self; 9] = [
        Self::Gayrimenkul,
        Self::Arac,
        Self::Hisse,
        Self::Fon,
        Self::Altin,
        Self::Doviz,
        Self::Kripto,
        Self::Mevduat,
        Self::Diger,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Gayrimenkul => "gayrimenkul",
            Self::Arac => "arac",
            Self::Hisse => "hisse",
            Self::Fon => "fon",
            Self::Altin => "altin",
            Self::Doviz => "doviz",
            Self::Kripto => "kripto",
            Self::Mevduat => "mevduat",
            Self::Diger => "diger",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Gayrimenkul => "Gayrimenkul",
            Self::Arac => "Araç",
            Self::Hisse => "Hisse senedi",
            Self::Fon => "Fon",
            Self::Altin => "Altın / maden",
            Self::Doviz => "Döviz",
            Self::Kripto => "Kripto",
            Self::Mevduat => "Mevduat / nakit",
            Self::Diger => "Diğer",
        }
    }

    /// Konu simgesi.
    #[must_use]
    pub const fn icon(self) -> &'static str {
        match self {
            Self::Gayrimenkul => "house",
            Self::Arac => "car",
            Self::Hisse | Self::Fon | Self::Kripto => "chart",
            Self::Altin | Self::Doviz | Self::Mevduat | Self::Diger => "wallet",
        }
    }

    pub fn parse(s: &str) -> Result<Self, CoreError> {
        Self::ALL
            .into_iter()
            .find(|k| k.name() == s)
            .ok_or_else(|| CoreError::UnknownVariant {
                field: "varlık türü",
                value: s.to_owned(),
            })
    }
}

/// Konunun varlık bilgisi (konu anlık görüntüsüyle senkronlanır).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetInfo {
    pub kind: AssetKind,
    /// Miktar × 1000 (ör. 12,5 gram → 12 500); yoksa tek parça (daire, araç).
    #[serde(default)]
    pub quantity_milli: Option<i64>,
    /// Birim ("adet", "gram", "USD"…).
    #[serde(default)]
    pub unit: Option<String>,
    /// Otomatik fiyat için sembol ("THYAO", "gram altın", "USD/TRY"); yoksa elle güncellenir.
    #[serde(default)]
    pub symbol: Option<String>,
    /// Toplam güncel değer.
    pub value: Money,
    pub as_of: NaiveDate,
}

/// "12,5" / "120" / "1.250,75" → milli birim.
pub fn parse_quantity(s: &str) -> Result<i64, CoreError> {
    // Money ayrıştırıcısı TR ondalık kurallarını bilir; kuruş = 1/100 → milli için ×10.
    let m = Money::parse(s)?;
    if m.0 <= 0 {
        return Err(CoreError::UnknownVariant {
            field: "miktar",
            value: s.to_owned(),
        });
    }
    m.0.checked_mul(10)
        .ok_or_else(|| CoreError::UnknownVariant {
            field: "miktar",
            value: s.to_owned(),
        })
}

/// Milli miktarı Türkçe gösterir ("12,5", "120").
#[must_use]
pub fn format_quantity(milli: i64) -> String {
    let whole = milli / 1000;
    let frac = (milli % 1000).abs();
    if frac == 0 {
        whole.to_string()
    } else {
        format!("{whole},{}", format!("{frac:03}").trim_end_matches('0'))
    }
}

fn short(s: Option<&str>, field: &'static str, max: usize) -> Result<Option<String>, CoreError> {
    let Some(v) = s.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    if v.chars().count() > max {
        return Err(CoreError::TooLong {
            field,
            max,
            actual: v.chars().count(),
        });
    }
    Ok(Some(v.to_owned()))
}

impl AssetInfo {
    pub fn new(
        kind: AssetKind,
        quantity: Option<&str>,
        unit: Option<&str>,
        symbol: Option<&str>,
        value: Money,
        as_of: NaiveDate,
    ) -> Result<Self, CoreError> {
        if value.0 < 0 {
            return Err(CoreError::UnknownVariant {
                field: "varlık değeri",
                value: value.format_signed(),
            });
        }
        let quantity_milli = match quantity.map(str::trim).filter(|q| !q.is_empty()) {
            Some(q) => Some(parse_quantity(q)?),
            None => None,
        };
        Ok(Self {
            kind,
            quantity_milli,
            unit: short(unit, "birim", 20)?,
            symbol: short(symbol, "sembol", 40)?,
            value,
            as_of,
        })
    }

    /// Ajanın bildirdiği birim fiyatla yeniden değerler (miktar × birim fiyat). Miktar yoksa
    /// birim fiyat toplam değer sayılır. Değişim binde olarak döner.
    #[must_use]
    pub fn revalue(&self, unit_price: Money, date: NaiveDate) -> (Self, i64) {
        let value = match self.quantity_milli {
            Some(q) => Money(
                i64::try_from(i128::from(unit_price.0) * i128::from(q) / 1000).unwrap_or(i64::MAX),
            ),
            None => unit_price,
        };
        let change = if self.value.0 == 0 {
            0
        } else {
            (value.0 - self.value.0) * 1000 / self.value.0
        };
        (
            Self {
                value,
                as_of: date,
                ..self.clone()
            },
            change,
        )
    }
}

/// Bu kadar (binde) değişim konuya NotAI mesajı olarak yazılır.
pub const NOTIFY_CHANGE_PERMILLE: i64 = 20;

/// Toplam varlık özeti.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AssetSummary {
    pub total: Money,
    pub by_kind: Vec<(AssetKind, Money)>,
    /// Toplamın 30 gün önceki değere göre değişimi (geçmiş varsa).
    pub change_30d: Option<Money>,
}

/// Özet: `assets` güncel değerler, `past_total` 30 gün önceki toplam (varsa).
#[must_use]
pub fn summary(assets: &[AssetInfo], past_total: Option<Money>) -> AssetSummary {
    let total = Money(assets.iter().map(|a| a.value.0).sum());
    let mut by: std::collections::BTreeMap<AssetKind, i64> = std::collections::BTreeMap::new();
    for a in assets {
        *by.entry(a.kind).or_default() += a.value.0;
    }
    let mut by_kind: Vec<(AssetKind, Money)> = by.into_iter().map(|(k, v)| (k, Money(v))).collect();
    by_kind.sort_by_key(|x| std::cmp::Reverse(x.1));
    AssetSummary {
        total,
        by_kind,
        change_30d: past_total.map(|p| Money(total.0 - p.0)),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    #[test]
    fn quantity_and_revalue() {
        assert_eq!(parse_quantity("12,5").unwrap(), 12_500);
        assert_eq!(parse_quantity("120").unwrap(), 120_000);
        assert!(parse_quantity("-3").is_err());
        assert_eq!(format_quantity(12_500), "12,5");
        assert_eq!(format_quantity(120_000), "120");
        let gold = AssetInfo::new(
            AssetKind::Altin,
            Some("12,5"),
            Some("gram"),
            Some("gram altın"),
            Money(4_000_000),
            d(1),
        )
        .unwrap();
        let (v, ch) = gold.revalue(Money(330_000), d(8));
        assert_eq!(v.value, Money(4_125_000), "12,5 g × 3.300 ₺");
        assert_eq!(ch, 31, "%3,1 artış");
        assert_eq!(v.as_of, d(8));
        let flat = AssetInfo::new(
            AssetKind::Gayrimenkul,
            None,
            None,
            None,
            Money(1_000_000_000),
            d(1),
        )
        .unwrap();
        let (v, ch) = flat.revalue(Money(1_050_000_000), d(8));
        assert_eq!((v.value, ch), (Money(1_050_000_000), 50));
        assert!(AssetInfo::new(AssetKind::Diger, None, None, None, Money(-1), d(1)).is_err());
        assert!(
            AssetInfo::new(
                AssetKind::Diger,
                None,
                Some(&"x".repeat(21)),
                None,
                Money(1),
                d(1)
            )
            .is_err()
        );
    }

    #[test]
    fn summary_by_kind() {
        let a = |k, v| AssetInfo::new(k, None, None, None, Money(v), d(1)).unwrap();
        let s = summary(
            &[
                a(AssetKind::Gayrimenkul, 900),
                a(AssetKind::Altin, 100),
                a(AssetKind::Altin, 50),
            ],
            Some(Money(1000)),
        );
        assert_eq!(s.total, Money(1050));
        assert_eq!(s.by_kind[0], (AssetKind::Gayrimenkul, Money(900)));
        assert_eq!(s.by_kind[1], (AssetKind::Altin, Money(150)));
        assert_eq!(s.change_30d, Some(Money(50)));
        assert_eq!(AssetKind::parse("altin").unwrap(), AssetKind::Altin);
    }
}
