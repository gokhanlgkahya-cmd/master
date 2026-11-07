//! Fiyat / ürün takibi (Gökhan, 2026-11-07): bir konu "takip" ayarı taşır (ürün, bağlantılar,
//! hedef fiyat). Ajan (Hermes / Claude Code) günde bir bakar ve bulduğu teklifleri bildirir;
//! **yalnız değişince** (ilk sonuç, fiyat düşüşü, daha uygun yeni bağlantı, hedefe ulaşma) konuya
//! NotAI mesajı yazılır ve bildirim gider. Fiyat artışı ya da aynı sonuç sessizce kaydedilir.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::finance::Money;

/// Varsayılan kontrol aralığı (saat): günde bir.
pub const DEFAULT_EVERY_HOURS: u32 = 24;
/// En az / en çok aralık.
pub const MIN_EVERY_HOURS: u32 = 6;
pub const MAX_EVERY_HOURS: u32 = 24 * 7;
/// En fazla bağlantı ve teklif.
pub const MAX_URLS: usize = 10;
pub const MAX_OFFERS: usize = 20;
/// Bu oranın altındaki düşüş "değişim" sayılmaz (kuruş oynamaları bildirim yapmasın).
pub const MIN_DROP_PERMILLE: i64 = 5;

/// Konunun takip ayarı (konu anlık görüntüsüyle senkronlanır).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchConfig {
    /// Aranacak ürün ("Pixel 10 Pro 256 GB").
    pub query: String,
    #[serde(default)]
    pub urls: Vec<String>,
    #[serde(default)]
    pub target_price: Option<Money>,
    #[serde(default = "default_every")]
    pub every_hours: u32,
    #[serde(default = "default_true")]
    pub active: bool,
}

const fn default_every() -> u32 {
    DEFAULT_EVERY_HOURS
}
const fn default_true() -> bool {
    true
}

fn valid_url(u: &str) -> bool {
    (u.starts_with("https://") || u.starts_with("http://"))
        && u.len() <= 2000
        && !u.chars().any(char::is_whitespace)
}

impl WatchConfig {
    /// Doğrular ve normalleştirir (boş bağlantılar atılır).
    pub fn new(
        query: &str,
        urls: Vec<String>,
        target_price: Option<Money>,
        every_hours: u32,
        active: bool,
    ) -> Result<Self, CoreError> {
        let query = query.trim();
        if query.is_empty() {
            return Err(CoreError::Empty {
                field: "takip edilen ürün",
            });
        }
        if query.chars().count() > 200 {
            return Err(CoreError::TooLong {
                field: "takip edilen ürün",
                max: 200,
                actual: query.chars().count(),
            });
        }
        let urls: Vec<String> = urls
            .into_iter()
            .map(|u| u.trim().to_owned())
            .filter(|u| !u.is_empty())
            .collect();
        if urls.len() > MAX_URLS {
            return Err(CoreError::TooLong {
                field: "bağlantı sayısı",
                max: MAX_URLS,
                actual: urls.len(),
            });
        }
        if let Some(bad) = urls.iter().find(|u| !valid_url(u)) {
            return Err(CoreError::UnknownVariant {
                field: "bağlantı",
                value: bad.clone(),
            });
        }
        if target_price.is_some_and(|p| p.0 <= 0) {
            return Err(CoreError::UnknownVariant {
                field: "hedef fiyat",
                value: "0 ya da eksi".into(),
            });
        }
        Ok(Self {
            query: query.to_owned(),
            urls,
            target_price,
            every_hours: every_hours.clamp(MIN_EVERY_HOURS, MAX_EVERY_HOURS),
            active,
        })
    }
}

/// Ajanın bulduğu bir teklif.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub title: String,
    pub price: Money,
    pub url: String,
    #[serde(default)]
    pub store: Option<String>,
}

impl Offer {
    pub fn new(
        title: &str,
        price: Money,
        url: &str,
        store: Option<&str>,
    ) -> Result<Self, CoreError> {
        if price.0 <= 0 {
            return Err(CoreError::UnknownVariant {
                field: "fiyat",
                value: price.format_abs(),
            });
        }
        if !valid_url(url) {
            return Err(CoreError::UnknownVariant {
                field: "bağlantı",
                value: url.to_owned(),
            });
        }
        let title: String = title.split_whitespace().collect::<Vec<_>>().join(" ");
        Ok(Self {
            title: title.chars().take(160).collect(),
            price,
            url: url.to_owned(),
            store: store
                .map(|s| s.trim().chars().take(60).collect::<String>())
                .filter(|s| !s.is_empty()),
        })
    }

    fn label(&self) -> String {
        self.store.clone().unwrap_or_else(|| {
            self.url
                .split('/')
                .nth(2)
                .unwrap_or("bağlantı")
                .trim_start_matches("www.")
                .to_owned()
        })
    }
}

/// Sunucuda tutulan son durum (senkronlanmaz).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WatchState {
    pub last_checked: Option<DateTime<Utc>>,
    pub best: Option<Offer>,
    #[serde(default)]
    pub seen_urls: Vec<String>,
}

/// Kontrol zamanı geldi mi?
#[must_use]
pub fn is_due(cfg: &WatchConfig, state: &WatchState, now: DateTime<Utc>) -> bool {
    cfg.active
        && state
            .last_checked
            .is_none_or(|t| now - t >= Duration::hours(i64::from(cfg.every_hours)))
}

/// Konuya yazılacak mesaj ve bildirim özeti.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Bildirimin kısa metni ("52.999,00 ₺ (−3,6%) · Hepsiburada").
    pub headline: String,
    /// Konuya yazılacak mesaj (teklif listesiyle).
    pub message: String,
}

fn pct_permille(from: Money, to: Money) -> i64 {
    if from.0 == 0 {
        0
    } else {
        (from.0 - to.0) * 1000 / from.0
    }
}

/// Sonuçları değerlendirir: yeni durum ve (yalnız değişince) mesaj.
#[must_use]
pub fn evaluate(
    cfg: &WatchConfig,
    prev: &WatchState,
    offers: &[Offer],
    now: DateTime<Utc>,
) -> (WatchState, Option<Report>) {
    let mut sorted: Vec<&Offer> = offers.iter().collect();
    sorted.sort_by(|a, b| a.price.cmp(&b.price).then(a.url.cmp(&b.url)));
    let best = sorted.first().map(|o| (*o).clone());
    let mut seen = prev.seen_urls.clone();
    for o in offers {
        if !seen.contains(&o.url) {
            seen.push(o.url.clone());
        }
    }
    let state = WatchState {
        last_checked: Some(now),
        best: best.clone().or_else(|| prev.best.clone()),
        seen_urls: seen,
    };
    let Some(best) = best else {
        return (state, None);
    };
    let reason = match &prev.best {
        None => Some(format!(
            "Takip başladı: en düşük {}",
            best.price.format_abs()
        )),
        Some(old) => {
            let drop = pct_permille(old.price, best.price);
            let target_hit = cfg
                .target_price
                .is_some_and(|t| best.price <= t && old.price > t);
            let new_cheaper_link = !prev.seen_urls.contains(&best.url) && best.price <= old.price;
            if target_hit {
                Some(format!(
                    "Hedef fiyata ulaşıldı (≤ {}): {}",
                    cfg.target_price.map_or_else(String::new, Money::format_abs),
                    best.price.format_abs()
                ))
            } else if drop >= MIN_DROP_PERMILLE {
                Some(format!(
                    "Fiyat düştü: {} (önceki {}, −%{},{})",
                    best.price.format_abs(),
                    old.price.format_abs(),
                    drop / 10,
                    drop % 10
                ))
            } else if new_cheaper_link {
                Some(format!("Yeni uygun bağlantı: {}", best.price.format_abs()))
            } else {
                None
            }
        }
    };
    let report = reason.map(|r| {
        let mut message = format!("{r} — {}\n", best.label());
        for o in sorted.iter().take(3) {
            message.push_str(&format!(
                "• {} · {} · {}\n",
                o.label(),
                o.price.format_abs(),
                o.url
            ));
        }
        Report {
            headline: format!("{} · {}", best.price.format_abs(), best.label()),
            message: message.trim_end().to_owned(),
        }
    });
    (state, report)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use chrono::TimeZone;

    fn t(h: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 8, 6, 0, 0).unwrap() + Duration::hours(h)
    }

    fn offer(store: &str, lira: i64, url: &str) -> Offer {
        Offer::new("Pixel 10 Pro 256 GB", Money(lira * 100), url, Some(store)).unwrap()
    }

    fn cfg() -> WatchConfig {
        WatchConfig::new(
            "Pixel 10 Pro",
            vec!["https://example.com/pixel".into(), "  ".into()],
            Some(Money(5_000_000)),
            1,
            true,
        )
        .unwrap()
    }

    #[test]
    fn config_validation() {
        assert_eq!(cfg().urls.len(), 1, "boş bağlantı atılır");
        assert_eq!(cfg().every_hours, MIN_EVERY_HOURS, "aralık sınırlanır");
        assert!(WatchConfig::new(" ", vec![], None, 24, true).is_err());
        assert!(WatchConfig::new("x", vec!["ftp://a".into()], None, 24, true).is_err());
        assert!(Offer::new("x", Money(0), "https://a.b", None).is_err());
        assert!(Offer::new("x", Money(100), "javascript:alert(1)", None).is_err());
    }

    #[test]
    fn due_once_a_day() {
        let mut c = cfg();
        c.every_hours = 24;
        let mut s = WatchState::default();
        assert!(is_due(&c, &s, t(0)));
        s.last_checked = Some(t(0));
        assert!(!is_due(&c, &s, t(23)));
        assert!(is_due(&c, &s, t(24)));
        c.active = false;
        assert!(!is_due(&c, &s, t(48)));
    }

    #[test]
    fn notifies_only_on_change() {
        let c = cfg();
        // İlk sonuç.
        let (s1, r) = evaluate(
            &c,
            &WatchState::default(),
            &[
                offer("Trendyol", 54_999, "https://t.com/1"),
                offer("Hepsiburada", 55_499, "https://h.com/1"),
            ],
            t(0),
        );
        let r = r.unwrap();
        assert!(
            r.message
                .starts_with("Takip başladı: en düşük 54.999,00\u{a0}₺ — Trendyol")
        );
        assert!(
            r.message
                .contains("• Hepsiburada · 55.499,00\u{a0}₺ · https://h.com/1")
        );
        // Aynı sonuç: sessiz.
        let (s2, r) = evaluate(
            &c,
            &s1,
            &[offer("Trendyol", 54_999, "https://t.com/1")],
            t(24),
        );
        assert!(r.is_none());
        assert_eq!(s2.last_checked, Some(t(24)));
        // Artış: sessiz.
        let (s3, r) = evaluate(
            &c,
            &s2,
            &[offer("Trendyol", 56_000, "https://t.com/1")],
            t(48),
        );
        assert!(r.is_none());
        // Düşüş (%5,4).
        let (s4, r) = evaluate(
            &c,
            &s3,
            &[offer("Trendyol", 52_999, "https://t.com/1")],
            t(72),
        );
        let r = r.unwrap();
        assert!(
            r.message
                .starts_with("Fiyat düştü: 52.999,00\u{a0}₺ (önceki 56.000,00\u{a0}₺, −%5,3)"),
            "{}",
            r.message
        );
        assert_eq!(r.headline, "52.999,00\u{a0}₺ · Trendyol");
        // Yeni ve aynı fiyattan bağlantı.
        let (s5, r) = evaluate(
            &c,
            &s4,
            &[offer("Amazon", 52_999, "https://a.com/1")],
            t(96),
        );
        assert!(r.unwrap().message.starts_with("Yeni uygun bağlantı"));
        // Hedef fiyat (≤ 50.000).
        let (_, r) = evaluate(
            &c,
            &s5,
            &[offer("Amazon", 49_900, "https://a.com/1")],
            t(120),
        );
        assert!(
            r.unwrap()
                .message
                .starts_with("Hedef fiyata ulaşıldı (≤ 50.000,00\u{a0}₺): 49.900,00\u{a0}₺")
        );
        // Sonuç yoksa: sessiz, son en iyi korunur.
        let (s6, r) = evaluate(&c, &s4, &[], t(130));
        assert!(r.is_none());
        assert_eq!(s6.best, s4.best);
    }
}
