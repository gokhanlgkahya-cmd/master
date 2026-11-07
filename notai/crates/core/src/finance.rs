//! Harcamalar (Gökhan, 2026-11-07): hesap hareketleri, iş/ev/kişisel ayrımı, banka ekstresi,
//! mükerrersiz aylık rapor ve finansal yönlendirme.
//!
//! - Tutar kuruş cinsinden tamsayıdır (kayan nokta yok): eksi = gider, artı = gelir.
//! - Ekstre satırları kurallarla ayrıştırılır (tarih … açıklama … tutar); banka verisi bu yolda
//!   hiçbir AI'a gitmez. Kategori ve kapsam satıcı adından tahmin edilir; kullanıcının düzelttiği
//!   satıcı kuralları önce gelir (öğrenme).
//! - Mükerrer: aynı ekstre ikinci kez yüklenirse satırlar aynı `dedupe_key`'i üretir (eklenmez);
//!   elle girilen ödeme ekstrede görününce eşleştirilir ve raporda bir kez sayılır.

use std::collections::{BTreeMap, HashMap};

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::tr::tr_upper;

/// Kuruş cinsinden tutar (eksi = gider).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct Money(pub i64);

impl Money {
    /// "1.850,00 ₺" (işaretsiz; işaret ayrıca gösterilir).
    #[must_use]
    pub fn format_abs(self) -> String {
        let k = self.0.unsigned_abs();
        let (lira, kurus) = (k / 100, k % 100);
        let digits = lira.to_string();
        let mut grouped = String::new();
        for (i, ch) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i) % 3 == 0 {
                grouped.push('.');
            }
            grouped.push(ch);
        }
        format!("{grouped},{kurus:02}\u{a0}₺")
    }

    /// İşaretli: "−1.850,00 ₺" / "+85.000,00 ₺".
    #[must_use]
    pub fn format_signed(self) -> String {
        let sign = if self.0 < 0 { "−" } else { "+" };
        format!("{sign}{}", self.format_abs())
    }

    /// Türkçe ya da düz biçimli tutarı okur: "-1.245,60", "+85.000,00", "1850", "229.99".
    pub fn parse(raw: &str) -> Result<Self, CoreError> {
        let s: String = raw
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '₺')
            .collect();
        let s = s.trim_end_matches("TL").trim_end_matches("TRY");
        let (neg, body) = match s.chars().next() {
            Some('-' | '−') => (true, &s[s.char_indices().nth(1).map_or(s.len(), |x| x.0)..]),
            Some('+') => (false, &s[1..]),
            _ => (false, s),
        };
        let bad = || CoreError::UnknownVariant {
            field: "tutar",
            value: raw.to_owned(),
        };
        // Son ayırıcı virgülse ondalık virgüldür (TR); yalnız nokta varsa ve sonrasında 2 hane
        // varsa ondalık noktadır; diğer noktalar binlik ayırıcıdır.
        let (int_part, frac) = if let Some(i) = body.rfind(',') {
            (body[..i].replace('.', ""), body[i + 1..].to_owned())
        } else if let Some(i) = body.rfind('.') {
            if body.len() - i - 1 == 2 {
                (body[..i].replace('.', ""), body[i + 1..].to_owned())
            } else {
                (body.replace('.', ""), String::new())
            }
        } else {
            (body.to_owned(), String::new())
        };
        if int_part.is_empty() || !int_part.chars().all(|c| c.is_ascii_digit()) {
            return Err(bad());
        }
        if frac.len() > 2 || !frac.chars().all(|c| c.is_ascii_digit()) {
            return Err(bad());
        }
        let lira: i64 = int_part.parse().map_err(|_| bad())?;
        let kurus: i64 = format!("{frac:0<2}").parse().map_err(|_| bad())?;
        let v = lira
            .checked_mul(100)
            .and_then(|x| x.checked_add(kurus))
            .ok_or_else(bad)?;
        Ok(Self(if neg { -v } else { v }))
    }
}

/// Harcamanın kime ait olduğu (raporda ayrı tutulur).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Is,
    Ev,
    Kisisel,
}

impl Scope {
    pub const ALL: [Self; 3] = [Self::Is, Self::Ev, Self::Kisisel];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Is => "is",
            Self::Ev => "ev",
            Self::Kisisel => "kisisel",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Is => "İş",
            Self::Ev => "Ev",
            Self::Kisisel => "Kişisel",
        }
    }

    pub fn parse(s: &str) -> Result<Self, CoreError> {
        match s {
            "is" | "iş" | "İş" => Ok(Self::Is),
            "ev" | "Ev" => Ok(Self::Ev),
            "kisisel" | "kişisel" | "Kişisel" => Ok(Self::Kisisel),
            other => Err(CoreError::UnknownVariant {
                field: "kapsam",
                value: other.to_owned(),
            }),
        }
    }
}

/// Bilinen kategoriler (anahtar, Türkçe ad).
pub const CATEGORIES: [(&str, &str); 14] = [
    ("market", "Market"),
    ("yeme_icme", "Yeme-içme"),
    ("akaryakit", "Akaryakıt / ulaşım"),
    ("fatura", "Faturalar"),
    ("abonelik", "Abonelikler"),
    ("alisveris", "Alışveriş"),
    ("saglik", "Sağlık"),
    ("egitim", "Eğitim"),
    ("kira_aidat", "Kira / aidat"),
    ("tedarik", "Tedarik / malzeme"),
    ("gelir", "Gelir"),
    ("transfer", "Transfer"),
    ("vergi", "Vergi / resmi"),
    ("diger", "Diğer"),
];

#[must_use]
pub fn category_label(key: &str) -> &'static str {
    CATEGORIES
        .iter()
        .find(|(k, _)| *k == key)
        .map_or("Diğer", |(_, l)| l)
}

/// Satıcı anahtarı: büyük harf, yalnız harf/rakam (PDF'ten gelen kopuk harf aralıkları ve
/// şube ekleri eşleşmeyi bozmasın), en fazla 24 karakter.
#[must_use]
pub fn merchant_key(description: &str) -> String {
    tr_upper(description)
        .chars()
        .filter(|c| c.is_alphanumeric())
        .take(24)
        .collect()
}

/// (anahtar kelime, kategori, kapsam). Boşluksuz büyük harfle aranır.
const RULES: &[(&str, &str, Scope)] = &[
    ("MAAŞ", "gelir", Scope::Kisisel),
    ("MAAS", "gelir", Scope::Kisisel),
    ("MİGROS", "market", Scope::Ev),
    ("A101", "market", Scope::Ev),
    ("BİM", "market", Scope::Ev),
    ("ŞOKMARKET", "market", Scope::Ev),
    ("CARREFOUR", "market", Scope::Ev),
    ("MARKET", "market", Scope::Ev),
    ("AKARYAKIT", "akaryakit", Scope::Kisisel),
    ("SHELL", "akaryakit", Scope::Kisisel),
    ("OPET", "akaryakit", Scope::Kisisel),
    ("PETROLOFİSİ", "akaryakit", Scope::Kisisel),
    ("HGS", "akaryakit", Scope::Kisisel),
    ("FATURA", "fatura", Scope::Ev),
    ("İGDAŞ", "fatura", Scope::Ev),
    ("ENERJİSA", "fatura", Scope::Ev),
    ("DOĞALGAZ", "fatura", Scope::Ev),
    ("ELEKTRİK", "fatura", Scope::Ev),
    ("TURKCELL", "fatura", Scope::Kisisel),
    ("VODAFONE", "fatura", Scope::Kisisel),
    ("TÜRKTELEKOM", "fatura", Scope::Ev),
    ("NETFLIX", "abonelik", Scope::Kisisel),
    ("SPOTIFY", "abonelik", Scope::Kisisel),
    ("YOUTUBE", "abonelik", Scope::Kisisel),
    ("APPLECOM", "abonelik", Scope::Kisisel),
    ("DISNEY", "abonelik", Scope::Kisisel),
    ("TRENDYOL", "alisveris", Scope::Kisisel),
    ("HEPSİBURADA", "alisveris", Scope::Kisisel),
    ("AMAZON", "alisveris", Scope::Kisisel),
    ("ECZANE", "saglik", Scope::Kisisel),
    ("HASTANE", "saglik", Scope::Kisisel),
    ("KİRA", "kira_aidat", Scope::Ev),
    ("AİDAT", "kira_aidat", Scope::Ev),
    ("RESTORAN", "yeme_icme", Scope::Kisisel),
    ("CAFE", "yeme_icme", Scope::Kisisel),
    ("YEMEKSEPETİ", "yeme_icme", Scope::Kisisel),
    ("GETİR", "yeme_icme", Scope::Kisisel),
    ("VERGİ", "vergi", Scope::Kisisel),
    ("SGK", "vergi", Scope::Is),
    ("EFT", "transfer", Scope::Kisisel),
    ("HAVALE", "transfer", Scope::Kisisel),
    // Şirket ünvanları: tedarikçi ödemesi → iş.
    ("LTD", "tedarik", Scope::Is),
    ("ŞTİ", "tedarik", Scope::Is),
    ("SANTİC", "tedarik", Scope::Is),
];

/// Kullanıcının öğrettiği satıcı kuralı (satıcı anahtarı → kategori, kapsam).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerchantRule {
    pub merchant: String,
    pub category: String,
    pub scope: Scope,
}

/// Kategori ve kapsam tahmini: önce kullanıcı kuralları, sonra yerleşik kurallar.
#[must_use]
pub fn categorize(description: &str, amount: Money, learned: &[MerchantRule]) -> (String, Scope) {
    let key = merchant_key(description);
    if let Some(r) = learned
        .iter()
        .find(|r| !r.merchant.is_empty() && key.starts_with(&r.merchant))
    {
        return (r.category.clone(), r.scope);
    }
    let flat: String = tr_upper(description)
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect();
    for (kw, cat, scope) in RULES {
        if flat.contains(kw) {
            return ((*cat).to_owned(), *scope);
        }
    }
    // "A.Ş." ünvanı (noktalarıyla; "TAŞIMA", "ATAŞEHİR" gibi kelimeler şirket sayılmasın).
    let dotted: String = tr_upper(description)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if dotted.contains("A.Ş") {
        return ("tedarik".to_owned(), Scope::Is);
    }
    if amount.0 > 0 {
        ("gelir".to_owned(), Scope::Kisisel)
    } else {
        ("diger".to_owned(), Scope::Kisisel)
    }
}

/// Ekstreden okunmuş bir satır.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementLine {
    pub date: NaiveDate,
    pub description: String,
    pub amount: Money,
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    ["%d.%m.%Y", "%d/%m/%Y", "%Y-%m-%d", "%d.%m.%y"]
        .iter()
        .find_map(|f| NaiveDate::parse_from_str(s, f).ok())
}

/// Ekstre metnini kurallarla ayrıştırır: satır başında tarih, sonda tutar. Bakiye sütunu varsa
/// (satırda iki tutar) ilk tutar işlem tutarıdır. Tanınmayan satırlar atlanır.
#[must_use]
pub fn parse_statement(text: &str) -> Vec<StatementLine> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        let Some((first, rest)) = line.split_once(' ') else {
            continue;
        };
        let Some(date) = parse_date(first) else {
            continue;
        };
        // Sondan tutar biçimli parçaları topla (bakiye + tutar olabilir).
        let words: Vec<&str> = rest.split(' ').collect();
        let mut amounts: Vec<(usize, Money)> = Vec::new();
        for (i, w) in words.iter().enumerate().rev() {
            let looks = w.chars().any(|c| c.is_ascii_digit())
                && w.chars()
                    .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+' | '−'))
                && (w.contains(',') || w.starts_with(['-', '+', '−']));
            match (looks, Money::parse(w)) {
                (true, Ok(m)) => amounts.push((i, m)),
                _ if *w == "TL" || *w == "₺" => {}
                _ => break,
            }
        }
        let Some(&(idx, amount)) = amounts.last() else {
            continue;
        };
        let description = words[..idx].join(" ").trim().to_owned();
        if description.is_empty() || amount.0 == 0 {
            continue;
        }
        out.push(StatementLine {
            date,
            description,
            amount,
        });
    }
    out
}

/// Mükerrer anahtarı: tarih + tutar + satıcı + aynı gün aynı satırın sırası. Aynı ekstre (ya da
/// çakışan dönemler) ikinci kez yüklenirse aynı anahtarlar üretilir.
#[must_use]
pub fn dedupe_keys(account: &str, lines: &[StatementLine]) -> Vec<String> {
    let mut seen: HashMap<(NaiveDate, i64, String), u32> = HashMap::new();
    lines
        .iter()
        .map(|l| {
            let m = merchant_key(&l.description);
            let n = seen.entry((l.date, l.amount.0, m.clone())).or_insert(0);
            *n += 1;
            format!(
                "{}|{}|{}|{m}|{n}",
                merchant_key(account),
                l.date.format("%Y-%m-%d"),
                l.amount.0
            )
        })
        .collect()
}

/// Elle girilmiş hareketle ekstre satırı aynı ödeme mi? (Tutar aynı, tarih ±3 gün.)
#[must_use]
pub fn same_payment(
    manual_date: NaiveDate,
    manual: Money,
    line_date: NaiveDate,
    line: Money,
) -> bool {
    manual.0 == line.0 && (manual_date - line_date).num_days().abs() <= 3
}

/// Hareketin kaynağı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TxSource {
    /// Kayıttan (AI önerisiyle) ya da elle.
    Manual,
    /// Banka ekstresinden.
    Statement,
}

impl TxSource {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Statement => "statement",
        }
    }
}

/// Bir hesap hareketi (senkronda anlık görüntü olarak taşınır).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub id: crate::TransactionId,
    pub date: NaiveDate,
    pub amount: Money,
    pub description: String,
    pub category: String,
    pub scope: Scope,
    pub source: TxSource,
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub topic: Option<crate::TopicId>,
    #[serde(default)]
    pub entry: Option<crate::EntryId>,
    /// Ekstre satırının mükerrer anahtarı (elle girişte yok).
    #[serde(default)]
    pub dedupe_key: Option<String>,
    /// Elle giriş ekstredeki karşılığıyla eşleştiyse o satır: raporda sayılmaz.
    #[serde(default)]
    pub matched_to: Option<crate::TransactionId>,
}

impl Transaction {
    #[must_use]
    pub fn movement(&self) -> Movement {
        Movement {
            date: self.date,
            description: self.description.clone(),
            amount: self.amount,
            category: self.category.clone(),
            scope: self.scope,
            matched: self.matched_to.is_some(),
        }
    }
}

/// Rapor için hareket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movement {
    pub date: NaiveDate,
    pub description: String,
    pub amount: Money,
    pub category: String,
    pub scope: Scope,
    /// Ekstredeki karşılığıyla eşleşmiş elle giriş: raporda sayılmaz (mükerrer).
    pub matched: bool,
}

/// Aylık rapor.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MonthReport {
    pub year: i32,
    pub month: u32,
    pub income: Money,
    /// Pozitif sayı olarak toplam gider.
    pub expense: Money,
    pub by_scope: Vec<(Scope, Money)>,
    pub by_category: Vec<(String, Money)>,
    pub recurring: Vec<(String, Money)>,
    pub insights: Vec<String>,
}

fn month_of(d: NaiveDate) -> (i32, u32) {
    (d.year(), d.month())
}

fn prev_month(y: i32, m: u32) -> (i32, u32) {
    if m == 1 { (y - 1, 12) } else { (y, m - 1) }
}

/// Hedef birikim oranı (%).
pub const SAVINGS_TARGET_PCT: i64 = 20;

/// Aylık rapor ve öneriler. Transfer kategorisi gelir/gidere katılmaz; eşleşmiş elle girişler
/// sayılmaz (ekstredeki karşılığı sayılır).
#[must_use]
pub fn month_report(all: &[Movement], year: i32, month: u32) -> MonthReport {
    let counted = |m: &&Movement| !m.matched && m.category != "transfer";
    let this: Vec<&Movement> = all
        .iter()
        .filter(counted)
        .filter(|m| month_of(m.date) == (year, month))
        .collect();
    let (py, pm) = prev_month(year, month);
    let prev: Vec<&Movement> = all
        .iter()
        .filter(counted)
        .filter(|m| month_of(m.date) == (py, pm))
        .collect();
    let income = Money(
        this.iter()
            .filter(|m| m.amount.0 > 0)
            .map(|m| m.amount.0)
            .sum(),
    );
    let expense = Money(
        -this
            .iter()
            .filter(|m| m.amount.0 < 0)
            .map(|m| m.amount.0)
            .sum::<i64>(),
    );
    let mut by_scope: BTreeMap<Scope, i64> = BTreeMap::new();
    let mut by_cat: BTreeMap<String, i64> = BTreeMap::new();
    for m in this.iter().filter(|m| m.amount.0 < 0) {
        *by_scope.entry(m.scope).or_default() -= m.amount.0;
        *by_cat.entry(m.category.clone()).or_default() -= m.amount.0;
    }
    let mut prev_cat: BTreeMap<String, i64> = BTreeMap::new();
    for m in prev.iter().filter(|m| m.amount.0 < 0) {
        *prev_cat.entry(m.category.clone()).or_default() -= m.amount.0;
    }
    let mut by_category: Vec<(String, Money)> =
        by_cat.iter().map(|(k, v)| (k.clone(), Money(*v))).collect();
    by_category.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    // Düzenli ödemeler: son iki ayda da görülen aynı satıcı (gider).
    let mut recurring: Vec<(String, Money)> = Vec::new();
    for m in this.iter().filter(|m| m.amount.0 < 0) {
        let key = merchant_key(&m.description);
        if prev
            .iter()
            .any(|p| p.amount.0 < 0 && merchant_key(&p.description) == key)
            && !recurring.iter().any(|(d, _)| merchant_key(d) == key)
        {
            recurring.push((m.description.clone(), Money(-m.amount.0)));
        }
    }

    let mut insights = Vec::new();
    if income.0 > 0 {
        let net = income.0 - expense.0;
        let pct = net * 100 / income.0;
        if net >= 0 {
            insights.push(format!(
                "Bu ay gelirinin %{pct}'ini biriktirdin ({}).",
                Money(net).format_abs()
            ));
        } else {
            insights.push(format!(
                "Bu ay harcaman gelirini {} aştı.",
                Money(-net).format_abs()
            ));
        }
        if pct < SAVINGS_TARGET_PCT {
            let need = income.0 * SAVINGS_TARGET_PCT / 100 - net;
            if let Some((cat, amt)) = by_category.iter().find(|(c, _)| c != "tedarik") {
                insights.push(format!(
                    "%{SAVINGS_TARGET_PCT} birikim için bu ay {} daha az harcaman gerekirdi; en büyük kişisel kalem: {} ({}).",
                    Money(need).format_abs(),
                    category_label(cat),
                    amt.format_abs()
                ));
            }
        }
    }
    for (cat, amt) in &by_category {
        if let Some(p) = prev_cat.get(cat).copied().filter(|p| *p > 0) {
            let diff = amt.0 - p;
            if diff >= 50_000 && diff * 100 / p >= 20 {
                insights.push(format!(
                    "{} harcaman geçen aya göre %{} arttı (+{}).",
                    category_label(cat),
                    diff * 100 / p,
                    Money(diff).format_abs()
                ));
            }
        }
    }
    let subs: i64 = by_cat.get("abonelik").copied().unwrap_or(0);
    if subs > 0 {
        insights.push(format!(
            "Abonelikler bu ay {}; kullanmadıklarını iptal etmeyi düşün.",
            Money(subs).format_abs()
        ));
    }
    if let Some(is) = by_scope.get(&Scope::Is).copied().filter(|v| *v > 0) {
        insights.push(format!(
            "İş giderleri {} — kişisel bütçeden ayrı tutuldu.",
            Money(is).format_abs()
        ));
    }

    MonthReport {
        year,
        month,
        income,
        expense,
        by_scope: Scope::ALL
            .iter()
            .map(|s| (*s, Money(by_scope.get(s).copied().unwrap_or(0))))
            .collect(),
        by_category,
        recurring,
        insights,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const PDF_TEXT: &str = "ÖRNEK BANKASI A.Ş. — Hesap Ekstresi\n\nHesap: V adesiz  TL ****1234 · Dönem: 01.10.2026 - 31.10.2026\n\n\
Tarih Açıklama Tutar (TL)\n\n01.10.2026 MİGROS ŞİŞLİ İSTANBUL -1.245,60\n\n03.10.2026 AYHAN KİMY A  SAN.  TİC. L TD -1.850,00\n\n\
05.10.2026 MAAŞ ÖDEMESİ +85.000,00\n\n07.10.2026 SHELL AKAR YAKIT  KADIKÖY -2.100,00\n\n10.10.2026 İGDAŞ DOĞALGAZ F ATURA -1.320,45\n\n\
12.10.2026 NETFLIX.COM -229,99\n\n15.10.2026 TRENDYOL  ÖDEME -640,00\n\n18.10.2026 MİGROS ŞİŞLİ İSTANBUL -980,30\n\n\
20.10.2026 ENERJİSA  ELEKTRİK FATURA -1.150,00\n\n25.10.2026 AMBALAJ DÜNY ASI  A.Ş. -3.400,00\n\nDönem sonu bakiye: 72.084,66  TL";

    fn d(m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, m, day).unwrap()
    }

    #[test]
    fn money_parse_and_format() {
        assert_eq!(Money::parse("-1.245,60").unwrap(), Money(-124_560));
        assert_eq!(Money::parse("+85.000,00").unwrap(), Money(8_500_000));
        assert_eq!(Money::parse("229.99").unwrap(), Money(22_999));
        assert_eq!(Money::parse("1850").unwrap(), Money(185_000));
        assert_eq!(Money::parse("1.850 TL").unwrap(), Money(185_000));
        assert!(Money::parse("abc").is_err());
        assert!(Money::parse("1,2345").is_err());
        assert_eq!(Money(-185_000).format_abs(), "1.850,00\u{a0}₺");
        assert_eq!(Money(8_500_000).format_signed(), "+85.000,00\u{a0}₺");
        assert_eq!(Money(-99).format_signed(), "−0,99\u{a0}₺");
    }

    #[test]
    fn statement_text_is_parsed_and_categorized() {
        let lines = parse_statement(PDF_TEXT);
        assert_eq!(lines.len(), 10, "bakiye/başlık satırları atlanır");
        assert_eq!(lines[1].amount, Money(-185_000));
        assert_eq!(lines[2].amount, Money(8_500_000));
        let cat = |i: usize| categorize(&lines[i].description, lines[i].amount, &[]);
        assert_eq!(cat(0), ("market".into(), Scope::Ev));
        assert_eq!(cat(1), ("tedarik".into(), Scope::Is), "LTD şirketi: iş");
        assert_eq!(cat(2), ("gelir".into(), Scope::Kisisel));
        assert_eq!(
            cat(3),
            ("akaryakit".into(), Scope::Kisisel),
            "PDF'teki kopuk harfler"
        );
        assert_eq!(cat(4), ("fatura".into(), Scope::Ev));
        assert_eq!(cat(5), ("abonelik".into(), Scope::Kisisel));
        assert_eq!(cat(9), ("tedarik".into(), Scope::Is), "A.Ş. ünvanı");
        assert_eq!(
            categorize("ATAŞEHİR TAŞIMACILIK", Money(-100), &[]).1,
            Scope::Kisisel,
            "AŞ geçen kelime şirket değil"
        );
        // Öğrenilmiş kural önce gelir.
        let learned = [MerchantRule {
            merchant: merchant_key("TRENDYOL"),
            category: "tedarik".into(),
            scope: Scope::Is,
        }];
        assert_eq!(
            categorize(&lines[6].description, lines[6].amount, &learned),
            ("tedarik".into(), Scope::Is)
        );
    }

    #[test]
    fn balance_column_and_noise() {
        let t =
            "02.11.2026 A101 KADIKÖY -150,00 12.345,67\nToplam 1.000,00\n05.11.2026 sadece metin";
        let l = parse_statement(t);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].amount, Money(-15_000), "işlem tutarı, bakiye değil");
        assert_eq!(l[0].description, "A101 KADIKÖY");
    }

    #[test]
    fn dedupe_keys_are_stable_and_count_same_day_repeats() {
        let lines = vec![
            StatementLine {
                date: d(10, 1),
                description: "MİGROS".into(),
                amount: Money(-1000),
            },
            StatementLine {
                date: d(10, 1),
                description: "MİGROS".into(),
                amount: Money(-1000),
            },
        ];
        let a = dedupe_keys("****1234", &lines);
        assert_ne!(a[0], a[1], "aynı gün iki eş alışveriş ayrı sayılır");
        assert_eq!(
            a,
            dedupe_keys("****1234", &lines),
            "yeniden yükleme aynı anahtarlar"
        );
        assert!(same_payment(
            d(10, 3),
            Money(-185_000),
            d(10, 5),
            Money(-185_000)
        ));
        assert!(!same_payment(
            d(10, 3),
            Money(-185_000),
            d(10, 9),
            Money(-185_000)
        ));
    }

    #[test]
    fn month_report_counts_once_and_gives_advice() {
        let mv =
            |m: u32, day: u32, desc: &str, amt: i64, cat: &str, scope: Scope, matched: bool| {
                Movement {
                    date: d(m, day),
                    description: desc.into(),
                    amount: Money(amt),
                    category: cat.into(),
                    scope,
                    matched,
                }
            };
        let all = vec![
            mv(9, 5, "MAAŞ", 8_500_000, "gelir", Scope::Kisisel, false),
            mv(
                9,
                12,
                "NETFLIX.COM",
                -22_999,
                "abonelik",
                Scope::Kisisel,
                false,
            ),
            mv(9, 15, "MİGROS", -1_000_000, "market", Scope::Ev, false),
            mv(10, 5, "MAAŞ", 8_500_000, "gelir", Scope::Kisisel, false),
            mv(
                10,
                12,
                "NETFLIX.COM",
                -22_999,
                "abonelik",
                Scope::Kisisel,
                false,
            ),
            mv(10, 15, "MİGROS", -1_500_000, "market", Scope::Ev, false),
            mv(10, 3, "Ayhan Kimya", -185_000, "tedarik", Scope::Is, false),
            // Aynı ödemenin elle girilmiş hali (eşleşmiş): sayılmaz.
            mv(10, 3, "Ayhan'a ödeme", -185_000, "tedarik", Scope::Is, true),
            mv(
                10,
                20,
                "Kendi hesabıma EFT",
                -5_000_000,
                "transfer",
                Scope::Kisisel,
                false,
            ),
            mv(10, 25, "Tatil", -6_000_000, "diger", Scope::Kisisel, false),
        ];
        let r = month_report(&all, 2026, 10);
        assert_eq!(r.income, Money(8_500_000));
        assert_eq!(
            r.expense,
            Money(22_999 + 1_500_000 + 185_000 + 6_000_000),
            "transfer ve eşleşmiş elle giriş yok"
        );
        assert_eq!(r.by_scope[0], (Scope::Is, Money(185_000)));
        assert_eq!(r.by_category[0].0, "diger");
        assert_eq!(r.recurring.len(), 2, "Netflix ve Migros iki aydır");
        let all_text = r.insights.join("\n");
        assert!(all_text.contains("biriktirdin"), "{all_text}");
        assert!(
            all_text.contains("Market harcaman geçen aya göre %50 arttı"),
            "{all_text}"
        );
        assert!(all_text.contains("Abonelikler bu ay 229,99"), "{all_text}");
        assert!(all_text.contains("İş giderleri 1.850,00"), "{all_text}");
        assert!(all_text.contains("%20 birikim için"), "{all_text}");
    }
}
