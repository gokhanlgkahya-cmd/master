//! Konum hareketleri (Gökhan, 2026-11-08).
//!
//! "Evden çıkıyorum", "işe geldim", "metroyla Kadıköy'e gidiyorum" gibi kayıtlar hareket olarak
//! tanınır. Yerler birer konudur ("Hareketler" altında; Ev ve İş önceden tanımlı). Eksik ayrıntı
//! (nereye, neyle, iş mi özel mi) NotAI'nin tek sorusuyla tamamlanır. Konum açıksa hareket
//! konumla teyit edilir; yerin konumu ilk kez öğrenilir. Aylık rapor: kaç kez, nerede ne kadar,
//! ne kadar masraf; iş ve özel ayrı.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::finance::{Money, Scope};
use crate::ids::{EntryId, TopicId};
use crate::tr::tr_lower;

/// Hareketlerin toplandığı ana konu.
pub const MOVES_ROOT: &str = "Hareketler";
/// Yer yarıçapı varsayılanı (m).
pub const DEFAULT_RADIUS_M: u32 = 250;
/// Bu süreden uzun "kalış" belirsiz sayılır (çıkış yazılmamış olabilir).
pub const MAX_STAY_HOURS: i64 = 48;
/// Kalkıştan bu kadar sonra varış gelmezse yolculuk eşleşmez.
pub const MAX_TRIP_HOURS: i64 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveKind {
    Depart,
    Arrive,
}

/// Ulaşım türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Yuruyus,
    Araba,
    Metro,
    Metrobus,
    Otobus,
    Tren,
    Vapur,
    Taksi,
    Dolmus,
    Servis,
    Bisiklet,
    Motosiklet,
    Ucak,
}

impl Mode {
    pub const ALL: [Self; 13] = [
        Self::Yuruyus,
        Self::Araba,
        Self::Metro,
        Self::Metrobus,
        Self::Otobus,
        Self::Tren,
        Self::Vapur,
        Self::Taksi,
        Self::Dolmus,
        Self::Servis,
        Self::Bisiklet,
        Self::Motosiklet,
        Self::Ucak,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Yuruyus => "Yürüyerek",
            Self::Araba => "Araba",
            Self::Metro => "Metro",
            Self::Metrobus => "Metrobüs",
            Self::Otobus => "Otobüs",
            Self::Tren => "Tren / Marmaray",
            Self::Vapur => "Vapur",
            Self::Taksi => "Taksi",
            Self::Dolmus => "Dolmuş",
            Self::Servis => "Servis",
            Self::Bisiklet => "Bisiklet",
            Self::Motosiklet => "Motosiklet",
            Self::Ucak => "Uçak",
        }
    }

    /// Metinde bu türü gösteren kelime başları (küçük harf). Sıra önemli: "metrobüs" "metro"dan önce.
    const fn stems(self) -> &'static [&'static str] {
        match self {
            Self::Metrobus => &["metrobüs"],
            Self::Metro => &["metro"],
            Self::Yuruyus => &["yürüyerek", "yürüyorum", "yürüdüm", "yürüyoruz", "yaya"],
            Self::Araba => &[
                "araba",
                "araçla",
                "aracım",
                "aracımla",
                "otomobil",
                "kendi aracı",
            ],
            Self::Otobus => &["otobüs", "iett"],
            Self::Tren => &["marmaray", "tren", "yht", "banliyö"],
            Self::Vapur => &["vapur", "feribot"],
            Self::Taksi => &["taksi", "uber", "bitaksi"],
            Self::Dolmus => &["dolmuş", "minibüs"],
            Self::Servis => &["servis"],
            Self::Bisiklet => &["bisiklet"],
            Self::Motosiklet => &["motosiklet", "motorla"],
            Self::Ucak => &["uçak", "uçağ"],
        }
    }

    /// Metindeki ilk ulaşım türü.
    #[must_use]
    pub fn find(text_lower: &str) -> Option<Self> {
        let order = [
            Self::Metrobus,
            Self::Metro,
            Self::Otobus,
            Self::Tren,
            Self::Vapur,
            Self::Taksi,
            Self::Dolmus,
            Self::Servis,
            Self::Bisiklet,
            Self::Motosiklet,
            Self::Ucak,
            Self::Araba,
            Self::Yuruyus,
        ];
        let words: Vec<&str> = words(text_lower).collect();
        order.into_iter().find(|m| {
            m.stems().iter().any(|s| {
                if s.contains(' ') {
                    text_lower.contains(s)
                } else {
                    words.iter().any(|w| w.starts_with(s))
                }
            })
        })
    }
}

/// Konum (derece × 10⁶; tamsayı, karşılaştırılabilir).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fix {
    pub lat_e6: i64,
    pub lon_e6: i64,
    /// Doğruluk (m).
    pub accuracy_m: u32,
}

impl Fix {
    pub fn from_degrees(lat: f64, lon: f64, accuracy_m: f64) -> Result<Self, CoreError> {
        let bad = |v: f64| CoreError::UnknownVariant {
            field: "konum",
            value: v.to_string(),
        };
        if !lat.is_finite() || !(-90.0..=90.0).contains(&lat) {
            return Err(bad(lat));
        }
        if !lon.is_finite() || !(-180.0..=180.0).contains(&lon) {
            return Err(bad(lon));
        }
        let acc = if accuracy_m.is_finite() && accuracy_m >= 0.0 {
            accuracy_m.min(100_000.0) as u32
        } else {
            0
        };
        Ok(Self {
            lat_e6: (lat * 1e6).round() as i64,
            lon_e6: (lon * 1e6).round() as i64,
            accuracy_m: acc,
        })
    }

    /// Büyük daire uzaklığı (m).
    #[must_use]
    pub fn distance_m(&self, other: &Self) -> u32 {
        let r = 6_371_000.0_f64;
        let (la1, lo1) = (self.lat_e6 as f64 / 1e6, self.lon_e6 as f64 / 1e6);
        let (la2, lo2) = (other.lat_e6 as f64 / 1e6, other.lon_e6 as f64 / 1e6);
        let (p1, p2) = (la1.to_radians(), la2.to_radians());
        let dp = (la2 - la1).to_radians();
        let dl = (lo2 - lo1).to_radians();
        let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
        let d = 2.0 * r * a.sqrt().min(1.0).asin();
        d.round().clamp(0.0, f64::from(u32::MAX)) as u32
    }
}

/// Bir yer konusunun ayarı (senkronlanır).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceInfo {
    /// Metinde yeri gösteren adlar (küçük harf, eksiz): ["iş", "ofis"].
    pub aliases: Vec<String>,
    #[serde(default)]
    pub fix: Option<Fix>,
    #[serde(default = "default_radius")]
    pub radius_m: u32,
    /// Bu yere gidişin varsayılan amacı.
    #[serde(default)]
    pub scope: Option<Scope>,
}

const fn default_radius() -> u32 {
    DEFAULT_RADIUS_M
}

impl PlaceInfo {
    #[must_use]
    pub fn new(aliases: &[&str], scope: Option<Scope>) -> Self {
        Self {
            aliases: aliases.iter().map(|a| tr_lower(a.trim())).collect(),
            fix: None,
            radius_m: DEFAULT_RADIUS_M,
            scope,
        }
    }
}

/// Önceden tanımlı yerler: (ad, adlar, amaç).
pub const DEFAULT_PLACES: [(&str, &[&str], Scope); 2] = [
    ("Ev", &["ev"], Scope::Ev),
    ("İş", &["iş", "ofis", "işyeri"], Scope::Is),
];

/// Ayrıştırıcıya verilen yer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub topic: TopicId,
    pub name: String,
    pub info: PlaceInfo,
}

/// Konum teyidi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum Check {
    /// Henüz konum sorulmadı / gelmedi.
    Waiting,
    /// Konum yerin yarıçapında.
    Confirmed { distance_m: u32 },
    /// Konum yerden uzak.
    Far { distance_m: u32 },
    /// Yerin konumu bu hareketle öğrenildi.
    Learned,
    /// Konumla yer bulundu (metinde yer yoktu).
    Matched { distance_m: u32 },
    /// Konum kapalı / alınamadı.
    NoLocation,
}

/// Bir hareket (kalkış ya da varış). Kayda bağlıdır; senkronlanır.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Movement {
    pub id: String,
    pub entry: EntryId,
    pub at: DateTime<Utc>,
    pub kind: MoveKind,
    /// Kalkışta nereden, varışta nereye.
    #[serde(default)]
    pub place: Option<TopicId>,
    /// Kalkışta söylenen hedef.
    #[serde(default)]
    pub to: Option<TopicId>,
    #[serde(default)]
    pub mode: Option<Mode>,
    #[serde(default)]
    pub scope: Option<Scope>,
    #[serde(default)]
    pub fix: Option<Fix>,
    pub check: Check,
    /// Kayıt yalnız hareketten ibaret (AI sınıflandırması gerekmez).
    #[serde(default)]
    pub only_movement: bool,
    /// Eksikler soruldu / konum beklemesi bitti.
    #[serde(default)]
    pub settled: bool,
}

/// Metinde geçen yer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaceRef {
    Known(TopicId),
    /// Tanınmayan yer adı ("Ayhan Kimya", "Market").
    New(String),
}

/// Ayrıştırma sonucu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detected {
    pub kind: MoveKind,
    pub place: Option<PlaceRef>,
    pub to: Option<PlaceRef>,
    pub mode: Option<Mode>,
    pub scope: Option<Scope>,
    pub only_movement: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Case {
    /// -den/-dan: nereden.
    Abl,
    /// -e/-a: nereye.
    Dat,
    /// -de/-da: nerede.
    Loc,
    /// -deyim/-dayım: buradayım (varış).
    Here,
    Bare,
}

const ABL: [&str; 12] = [
    "den", "dan", "ten", "tan", "nden", "ndan", "imden", "ımdan", "umdan", "ümden", "inden",
    "ından",
];
const DAT: [&str; 12] = [
    "e", "a", "ye", "ya", "ne", "na", "ime", "ıma", "uma", "üme", "ine", "ına",
];
const LOC: [&str; 12] = [
    "de", "da", "te", "ta", "nde", "nda", "imde", "ımda", "umda", "ümde", "inde", "ında",
];
const HERE: [&str; 6] = ["yim", "yım", "yum", "yüm", "yiz", "yız"];

const DEPART: [&str; 18] = [
    "çıkıyorum",
    "çıktım",
    "çıkıyoruz",
    "çıktık",
    "çıkıyom",
    "ayrıldım",
    "ayrılıyorum",
    "ayrıldık",
    "ayrılıyoruz",
    "gidiyorum",
    "gidiyoruz",
    "gidiyom",
    "dönüyorum",
    "dönüyoruz",
    "yoldayım",
    "yoldayız",
    "geçiyorum",
    "hareket",
];
const ARRIVE: [&str; 12] = [
    "geldim",
    "geldik",
    "vardım",
    "vardık",
    "ulaştım",
    "ulaştık",
    "döndüm",
    "döndük",
    "gittim",
    "gittik",
    "girdim",
    "geçtim",
];
/// Fiilden önce gelip yer sanılmaması gerekenler.
const NOT_PLACES: [&str; 16] = [
    "yol",
    "dışarı",
    "ora",
    "bura",
    "şura",
    "geri",
    "erken",
    "geç",
    "biraz",
    "şimdi",
    "sabah",
    "akşam",
    "öğlen",
    "toplantı",
    "iş",
    "hemen",
];
/// Cümle başında büyük harfle yazılsa da yer adına katılmayanlar.
const STARTERS: [&str; 12] = [
    "bugün", "şimdi", "sabah", "akşam", "yarın", "dün", "ben", "biz", "az", "biraz", "saat",
    "tamam",
];

fn words(s: &str) -> impl Iterator<Item = &str> {
    s.split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '’'))
        .filter(|w| !w.is_empty())
}

/// Metinde hareket fiili var mı? (Veritabanına dokunmadan hızlı ön süzgeç.)
#[must_use]
pub fn has_cue(text: &str) -> bool {
    let low = tr_lower(text);
    words(&low).any(|w| {
        DEPART.contains(&w)
            || ARRIVE.contains(&w)
            || HERE.iter().any(|h| {
                LOC.iter()
                    .any(|l| w.ends_with(&format!("{l}{h}")) && w.chars().count() > 5)
            })
    })
}

fn classify_suffix(rest: &str) -> Option<Case> {
    let rest = rest.trim_start_matches(['\'', '’']);
    if rest.is_empty() {
        return Some(Case::Bare);
    }
    for l in LOC {
        if let Some(h) = rest.strip_prefix(l)
            && HERE.contains(&h)
        {
            return Some(Case::Here);
        }
    }
    if ABL.contains(&rest) {
        return Some(Case::Abl);
    }
    if DAT.contains(&rest) {
        return Some(Case::Dat);
    }
    if LOC.contains(&rest) {
        return Some(Case::Loc);
    }
    None
}

/// Bilinen yer adlarını metinde arar: (yer, durum, konum sırası).
fn known_mentions(low: &str, places: &[Place]) -> Vec<(TopicId, Case, usize)> {
    let mut out = Vec::new();
    for p in places {
        let mut names: Vec<String> = p.info.aliases.clone();
        names.push(tr_lower(&p.name));
        for a in names.iter().filter(|a| !a.is_empty()) {
            let mut from = 0;
            while let Some(i) = low[from..].find(a.as_str()) {
                let start = from + i;
                let end = start + a.len();
                from = end;
                let before_ok = low[..start]
                    .chars()
                    .last()
                    .is_none_or(|c| !c.is_alphanumeric());
                if !before_ok {
                    continue;
                }
                let rest: String = low[end..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '\'' || *c == '’')
                    .collect();
                if let Some(case) = classify_suffix(&rest) {
                    out.push((p.topic, case, start));
                }
            }
        }
    }
    out.sort_by_key(|x| x.2);
    out
}

fn title_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => crate::tr::tr_upper(&f.to_string()) + c.as_str(),
        None => String::new(),
    }
}

/// Tanınmayan yer adları: kesme işaretli özel adlar ("Ayhan Kimya'ya") ve fiilden hemen önce
/// ekli tek kelime ("markete gidiyorum").
fn new_mentions(text: &str, places: &[Place]) -> Vec<(String, Case, usize)> {
    let mut out = Vec::new();
    let toks: Vec<(usize, &str)> = {
        let mut v = Vec::new();
        let mut idx = 0;
        for w in text.split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '’')) {
            if !w.is_empty() {
                // Kelimenin metindeki başlangıcı (bayt).
                if let Some(pos) = text[idx..].find(w) {
                    v.push((idx + pos, w));
                    idx += pos + w.len();
                }
            }
        }
        v
    };
    let known = |name: &str| {
        let l = tr_lower(name);
        places
            .iter()
            .any(|p| tr_lower(&p.name) == l || p.info.aliases.contains(&l))
    };
    for (i, (pos, w)) in toks.iter().enumerate() {
        if let Some((stem, suf)) = w.split_once(['\'', '’']) {
            let Some(case) = classify_suffix(suf).filter(|c| *c != Case::Bare) else {
                continue;
            };
            let mut parts = vec![stem.to_owned()];
            let mut j = i;
            while j > 0 && parts.len() < 4 {
                let prev = toks[j - 1].1;
                let upper = prev.chars().next().is_some_and(char::is_uppercase);
                if !upper || STARTERS.contains(&tr_lower(prev).as_str()) || prev.contains('\'') {
                    break;
                }
                parts.insert(0, prev.to_owned());
                j -= 1;
            }
            let name = parts.join(" ");
            if !known(&name) && stem.chars().count() >= 2 {
                out.push((name, case, toks[j].0));
            }
            continue;
        }
        // Fiilden hemen önceki ekli kelime.
        let next = toks.get(i + 1).map(|t| tr_lower(t.1));
        let is_verb = next
            .as_deref()
            .is_some_and(|n| DEPART.contains(&n) || ARRIVE.contains(&n));
        if !is_verb {
            continue;
        }
        let low = tr_lower(w);
        let mut best: Option<(String, Case)> = None;
        for (list, case) in [(&DAT[..], Case::Dat), (&ABL[..], Case::Abl)] {
            for s in list {
                if let Some(stem) = low.strip_suffix(s)
                    && stem.chars().count() >= 3
                    && best.as_ref().is_none_or(|(b, _)| stem.len() < b.len())
                {
                    best = Some((stem.to_owned(), case));
                }
            }
        }
        if let Some((stem, case)) = best
            && !NOT_PLACES.contains(&stem.as_str())
            && !known(&stem)
            && Mode::find(&low).is_none()
        {
            out.push((title_case(&stem), case, *pos));
        }
    }
    out
}

/// Metinden amaç: iş / ev / kişisel.
#[must_use]
pub fn scope_in(low: &str) -> Option<Scope> {
    let has = |xs: &[&str]| xs.iter().any(|x| low.contains(x));
    if has(&[
        "iş için",
        "işle ilgili",
        "iş gereği",
        "müşteri",
        "tedarikçi",
        "toplantı",
        "iş ziyaret",
    ]) {
        Some(Scope::Is)
    } else if has(&["ev için", "ev işi", "evin "]) {
        Some(Scope::Ev)
    } else if has(&[
        "özel", "kişisel", "gezi", "tatil", "eğlence", "arkadaş", "spor",
    ]) {
        Some(Scope::Kisisel)
    } else {
        None
    }
}

/// Metni ayrıştırır. Hareket fiili yoksa `None`.
#[must_use]
pub fn detect(text: &str, places: &[Place]) -> Option<Detected> {
    let low = tr_lower(text);
    let ws: Vec<&str> = words(&low).collect();
    let arrive_verb = ws.iter().any(|w| ARRIVE.contains(w));
    let depart_verb = ws.iter().any(|w| DEPART.contains(w));
    let mut mentions: Vec<(PlaceRef, Case, usize)> = known_mentions(&low, places)
        .into_iter()
        .map(|(t, c, p)| (PlaceRef::Known(t), c, p))
        .collect();
    // Bilinmeyenlerin konumu özgün metinde; küçük harfe çevirme bayt uzunluğunu nadiren değiştirir,
    // sıralama için yeterli.
    mentions.extend(
        new_mentions(text, places)
            .into_iter()
            .map(|(n, c, p)| (PlaceRef::New(n), c, p)),
    );
    mentions.sort_by_key(|m| m.2);
    let here = mentions.iter().find(|m| m.1 == Case::Here);
    if !arrive_verb && !depart_verb && here.is_none() {
        return None;
    }
    let first = |cases: &[Case]| {
        mentions
            .iter()
            .find(|m| cases.contains(&m.1))
            .map(|m| m.0.clone())
    };
    let (kind, place, to) = if arrive_verb || (here.is_some() && !depart_verb) {
        (
            MoveKind::Arrive,
            first(&[Case::Here, Case::Dat, Case::Loc]),
            None,
        )
    } else {
        (MoveKind::Depart, first(&[Case::Abl]), first(&[Case::Dat]))
    };
    let only_movement = ws.len() <= 8
        && !low.contains("₺")
        && !low.contains(" tl")
        && !ws
            .iter()
            .any(|w| w.chars().all(|c| c.is_ascii_digit()) && w.len() >= 3);
    Some(Detected {
        kind,
        place,
        to,
        mode: Mode::find(&low),
        scope: scope_in(&low),
        only_movement,
    })
}

/// Soruya verilen cevaptan çıkanlar.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Refine {
    pub place: Option<PlaceRef>,
    pub mode: Option<Mode>,
    pub scope: Option<Scope>,
}

/// Cevabı ayrıştırır: "metroyla, iş için", "Ayhan Kimya", "özel", "işe".
#[must_use]
pub fn refine(answer: &str, places: &[Place]) -> Refine {
    let low = tr_lower(answer.trim());
    let mode = Mode::find(&low);
    let mut scope = scope_in(&low);
    let ws: Vec<&str> = words(&low).collect();
    if scope.is_none() {
        scope = match ws.as_slice() {
            ["iş"] | ["is"] | ["iş", "için"] => Some(Scope::Is),
            ["ev"] | ["ev", "için"] => Some(Scope::Ev),
            _ => None,
        };
    }
    let known = known_mentions(&low, places);
    let mut place = known.first().map(|k| PlaceRef::Known(k.0));
    // "iş" tek başına amaç cevabıdır; yer sorusu varsa yer olarak da sayılır (çağıran karar verir).
    if place.is_none() {
        place = new_mentions(answer, places)
            .into_iter()
            .next()
            .map(|n| PlaceRef::New(n.0));
    }
    if place.is_none() && mode.is_none() && scope.is_none() && (1..=4).contains(&ws.len()) {
        let name = answer
            .trim()
            .trim_end_matches(['.', '!', '?'])
            .trim()
            .to_owned();
        if !name.is_empty() && !matches!(ws.as_slice(), ["evet"] | ["hayır"] | ["yok"]) {
            place = Some(PlaceRef::New(title_case(&name)));
        }
    }
    Refine { place, mode, scope }
}

/// Eksik ayrıntılar için tek soru. `prev_depart` aynı yolculuğun kalkışı (varsa).
#[must_use]
pub fn question(m: &Movement, places: &[Place], prev_depart: Option<&Movement>) -> Option<String> {
    let place_of = |t: Option<TopicId>| t.and_then(|t| places.iter().find(|p| p.topic == t));
    let mut parts: Vec<String> = Vec::new();
    match m.kind {
        MoveKind::Arrive => {
            let p = place_of(m.place);
            if p.is_none() {
                parts.push("Nereye geldin?".into());
            }
            let trip_mode = m.mode.or(prev_depart.and_then(|d| d.mode));
            if trip_mode.is_none() && prev_depart.is_some() {
                parts.push("Neyle geldin (metro, araba, otobüs…)?".into());
            }
            if m.scope.is_none() && p.is_some_and(|p| p.info.scope.is_none()) {
                parts.push(format!(
                    "{} iş için mi, özel mi?",
                    p.map_or("", |p| p.name.as_str())
                ));
            } else if m.scope.is_none() && p.is_none() {
                parts.push("İş için mi, özel mi?".into());
            }
        }
        MoveKind::Depart => {
            let to = place_of(m.to);
            if m.mode.is_none() {
                parts.push(match to {
                    Some(t) => format!(
                        "{} neyle gidiyorsun (metro, araba, otobüs…)?",
                        crate::tr::dative(&t.name)
                    ),
                    None => "Nereye, neyle gidiyorsun (metro, araba, otobüs…)?".into(),
                });
            }
            if m.scope.is_none()
                && let Some(t) = to
                && t.info.scope.is_none()
            {
                parts.push(format!("{} iş için mi, özel mi?", t.name));
            }
        }
    }
    let q = parts.join(" ");
    (!q.is_empty()).then(|| q.chars().take(200).collect())
}

/// Konumla teyit: hareketin yeriyle karşılaştırır; yer bilinmiyorsa en yakın yeri bulur.
/// Dönüş: (yeni teyit durumu, eşleşen yer — metinde yer yoksa, öğrenilecek yer).
#[must_use]
pub fn verify(
    m: &Movement,
    fix: &Fix,
    places: &[Place],
) -> (Check, Option<TopicId>, Option<TopicId>) {
    let target = match m.kind {
        MoveKind::Arrive | MoveKind::Depart => m.place,
    };
    if let Some(t) = target
        && let Some(p) = places.iter().find(|p| p.topic == t)
    {
        return match &p.info.fix {
            Some(pf) => {
                let d = pf.distance_m(fix);
                let tol = p.info.radius_m + fix.accuracy_m.min(500);
                if d <= tol {
                    (Check::Confirmed { distance_m: d }, None, None)
                } else {
                    (Check::Far { distance_m: d }, None, None)
                }
            }
            // İlk kez: yerin konumu öğrenilir (doğruluk yeterliyse).
            None if fix.accuracy_m <= 200 => (Check::Learned, None, Some(t)),
            None => (Check::NoLocation, None, None),
        };
    }
    let nearest = places
        .iter()
        .filter_map(|p| p.info.fix.map(|pf| (p, pf.distance_m(fix))))
        .filter(|(p, d)| *d <= p.info.radius_m + fix.accuracy_m.min(500))
        .min_by_key(|(_, d)| *d);
    match nearest {
        Some((p, d)) => (Check::Matched { distance_m: d }, Some(p.topic), None),
        None => (Check::NoLocation, None, None),
    }
}

/// Konum bu hareket için elzem mi? (Yer bilinmiyor ya da yerin konumu henüz öğrenilmedi.)
#[must_use]
pub fn location_essential(m: &Movement, places: &[Place]) -> bool {
    match m.place.and_then(|t| places.iter().find(|p| p.topic == t)) {
        None => m.kind == MoveKind::Arrive,
        Some(p) => p.info.fix.is_none(),
    }
}

/// Bir yerde kalış.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stay {
    pub place: TopicId,
    pub from: DateTime<Utc>,
    /// Çıkış bilinmiyorsa `None`.
    pub to: Option<DateTime<Utc>>,
}

/// Bir yolculuk (kalkış → varış).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trip {
    pub from: Option<TopicId>,
    pub to: Option<TopicId>,
    pub depart: Option<DateTime<Utc>>,
    pub arrive: Option<DateTime<Utc>>,
    pub mode: Option<Mode>,
    pub scope: Option<Scope>,
}

/// Hareketlerden kalışlar ve yolculuklar.
#[must_use]
pub fn timeline(moves: &[Movement]) -> (Vec<Stay>, Vec<Trip>) {
    let mut ms: Vec<&Movement> = moves.iter().collect();
    ms.sort_by_key(|m| m.at);
    let mut stays = Vec::new();
    let mut trips = Vec::new();
    let mut here: Option<(TopicId, DateTime<Utc>)> = None;
    let mut open: Option<Trip> = None;
    for m in ms {
        match m.kind {
            MoveKind::Depart => {
                let origin = m.place.or(here.map(|h| h.0));
                if let Some((p, since)) = here.take() {
                    let ok = m.at - since <= Duration::hours(MAX_STAY_HOURS);
                    stays.push(Stay {
                        place: p,
                        from: since,
                        to: ok.then_some(m.at),
                    });
                }
                if let Some(t) = open.take() {
                    trips.push(t);
                }
                open = Some(Trip {
                    from: origin,
                    to: m.to,
                    depart: Some(m.at),
                    arrive: None,
                    mode: m.mode,
                    scope: m.scope,
                });
            }
            MoveKind::Arrive => {
                if let Some((p, since)) = here.take() {
                    stays.push(Stay {
                        place: p,
                        from: since,
                        to: None,
                    });
                }
                match open.take() {
                    Some(mut t)
                        if t.depart
                            .is_some_and(|d| m.at - d <= Duration::hours(MAX_TRIP_HOURS)) =>
                    {
                        t.arrive = Some(m.at);
                        t.to = m.place.or(t.to);
                        t.mode = m.mode.or(t.mode);
                        t.scope = m.scope.or(t.scope);
                        trips.push(t);
                    }
                    other => {
                        if let Some(t) = other {
                            trips.push(t);
                        }
                        trips.push(Trip {
                            from: None,
                            to: m.place,
                            depart: None,
                            arrive: Some(m.at),
                            mode: m.mode,
                            scope: m.scope,
                        });
                    }
                }
                if let Some(p) = m.place {
                    here = Some((p, m.at));
                }
            }
        }
    }
    if let Some(t) = open {
        trips.push(t);
    }
    if let Some((p, since)) = here {
        stays.push(Stay {
            place: p,
            from: since,
            to: None,
        });
    }
    (stays, trips)
}

/// Rapor için bir harcama.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpendAt {
    /// Kayıt zamanı (saatli) ya da ekstre günü (gün başı).
    pub at: DateTime<Utc>,
    /// Saat biliniyor mu? (Ekstre satırının saati yok.)
    pub timed: bool,
    /// Gider (pozitif).
    pub amount: Money,
    pub scope: Scope,
    pub category: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceStat {
    pub place: TopicId,
    pub visits: u32,
    /// Çıkışı bilinen kalışların toplamı (dk).
    pub stay_minutes: i64,
    pub timed_stays: u32,
    pub cost: Money,
}

/// Aylık hareket raporu.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MoveReport {
    pub trips: u32,
    pub trips_is: u32,
    pub trips_other: u32,
    pub trips_unknown: u32,
    pub travel_minutes: i64,
    pub by_mode: Vec<(Mode, u32)>,
    pub places: Vec<PlaceStat>,
    /// Yol masrafı (yakıt/ulaşım + yolculuk sırasında girilen harcamalar).
    pub transport_is: Money,
    pub transport_other: Money,
    /// Yerlerde yapılan harcamalar toplamı.
    pub at_places: Money,
    /// Konumla teyit edilen / uzak çıkan hareket sayısı.
    pub confirmed: u32,
    pub far: u32,
}

/// Yolculuğun amacı: söylenen; yoksa uçlardan biri iş yeriyse iş; yoksa hedefin amacı.
#[must_use]
pub fn trip_scope(t: &Trip, places: &[Place]) -> Option<Scope> {
    let sc = |p: Option<TopicId>| {
        p.and_then(|p| places.iter().find(|x| x.topic == p))
            .and_then(|x| x.info.scope)
    };
    t.scope.or_else(|| {
        let (a, b) = (sc(t.from), sc(t.to));
        if a == Some(Scope::Is) || b == Some(Scope::Is) {
            Some(Scope::Is)
        } else {
            b.or(a)
        }
    })
}

/// `[from, to)` aralığının raporu.
#[must_use]
pub fn report(
    moves: &[Movement],
    places: &[Place],
    spends: &[SpendAt],
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> MoveReport {
    let (stays, trips) = timeline(moves);
    let in_range = |t: DateTime<Utc>| t >= from && t < to;
    let trips: Vec<&Trip> = trips
        .iter()
        .filter(|t| t.depart.or(t.arrive).is_some_and(in_range))
        .collect();
    let mut r = MoveReport::default();
    let mut modes: BTreeMap<Mode, u32> = BTreeMap::new();
    for t in &trips {
        r.trips += 1;
        match trip_scope(t, places) {
            Some(Scope::Is) => r.trips_is += 1,
            Some(_) => r.trips_other += 1,
            None => r.trips_unknown += 1,
        }
        if let Some(m) = t.mode {
            *modes.entry(m).or_default() += 1;
        }
        if let (Some(d), Some(a)) = (t.depart, t.arrive) {
            r.travel_minutes += (a - d).num_minutes();
        }
    }
    let mut by_mode: Vec<(Mode, u32)> = modes.into_iter().collect();
    by_mode.sort_by_key(|x| std::cmp::Reverse(x.1));
    r.by_mode = by_mode;

    let mut stats: BTreeMap<TopicId, PlaceStat> = BTreeMap::new();
    let stat = |stats: &mut BTreeMap<TopicId, PlaceStat>, p: TopicId| {
        stats.entry(p).or_insert(PlaceStat {
            place: p,
            visits: 0,
            stay_minutes: 0,
            timed_stays: 0,
            cost: Money(0),
        });
    };
    for m in moves
        .iter()
        .filter(|m| m.kind == MoveKind::Arrive && in_range(m.at))
    {
        if let Some(p) = m.place {
            stat(&mut stats, p);
            if let Some(s) = stats.get_mut(&p) {
                s.visits += 1;
            }
        }
    }
    for s in stays.iter().filter(|s| in_range(s.from)) {
        if let Some(end) = s.to {
            stat(&mut stats, s.place);
            if let Some(x) = stats.get_mut(&s.place) {
                x.stay_minutes += (end - s.from).num_minutes();
                x.timed_stays += 1;
            }
        }
    }
    // Harcamalar: yolculuk sırasında → yol; kalış sırasında → o yer; yakıt/ulaşım → yol.
    for sp in spends.iter().filter(|s| in_range(s.at)) {
        let during_trip = sp
            .timed
            .then(|| {
                trips.iter().find(|t| match (t.depart, t.arrive) {
                    (Some(d), Some(a)) => sp.at >= d && sp.at <= a,
                    _ => false,
                })
            })
            .flatten();
        if let Some(t) = during_trip {
            if trip_scope(t, places) == Some(Scope::Is) {
                r.transport_is.0 += sp.amount.0;
            } else {
                r.transport_other.0 += sp.amount.0;
            }
            continue;
        }
        let stay = sp
            .timed
            .then(|| {
                stays
                    .iter()
                    .find(|s| sp.at >= s.from && s.to.is_some_and(|e| sp.at <= e))
            })
            .flatten();
        if let Some(s) = stay {
            stat(&mut stats, s.place);
            if let Some(x) = stats.get_mut(&s.place) {
                x.cost.0 += sp.amount.0;
            }
            r.at_places.0 += sp.amount.0;
            continue;
        }
        if sp.category == "akaryakit" {
            if sp.scope == Scope::Is {
                r.transport_is.0 += sp.amount.0;
            } else {
                r.transport_other.0 += sp.amount.0;
            }
        }
    }
    let mut places_v: Vec<PlaceStat> = stats.into_values().collect();
    places_v.sort_by_key(|p| {
        (
            std::cmp::Reverse(p.visits),
            std::cmp::Reverse(p.stay_minutes),
        )
    });
    r.places = places_v;
    for m in moves.iter().filter(|m| in_range(m.at)) {
        match m.check {
            Check::Confirmed { .. } | Check::Learned | Check::Matched { .. } => r.confirmed += 1,
            Check::Far { .. } => r.far += 1,
            Check::Waiting | Check::NoLocation => {}
        }
    }
    r
}

/// "9 sa 10 dk" / "45 dk".
#[must_use]
pub fn duration_label(minutes: i64) -> String {
    let (h, m) = (minutes / 60, minutes % 60);
    match (h, m) {
        (0, m) => format!("{m} dk"),
        (h, 0) => format!("{h} sa"),
        (h, m) => format!("{h} sa {m} dk"),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]
    use super::*;
    use chrono::TimeZone;

    fn places() -> Vec<Place> {
        DEFAULT_PLACES
            .iter()
            .map(|(n, a, s)| Place {
                topic: TopicId::new(),
                name: (*n).to_owned(),
                info: PlaceInfo::new(a, Some(*s)),
            })
            .collect()
    }

    fn t(h: i64, m: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 9, 0, 0, 0).unwrap()
            + Duration::hours(h)
            + Duration::minutes(m)
    }

    #[test]
    fn detects_turkish_movements() {
        let ps = places();
        let (ev, is) = (ps[0].topic, ps[1].topic);
        let d = detect("Evden çıkıyorum", &ps).unwrap();
        assert_eq!(d.kind, MoveKind::Depart);
        assert_eq!(d.place, Some(PlaceRef::Known(ev)));
        assert!(d.only_movement);
        let d = detect("işe geldim", &ps).unwrap();
        assert_eq!(
            (d.kind, d.place),
            (MoveKind::Arrive, Some(PlaceRef::Known(is)))
        );
        let d = detect("Metroyla işe gidiyorum", &ps).unwrap();
        assert_eq!(d.kind, MoveKind::Depart);
        assert_eq!(d.to, Some(PlaceRef::Known(is)));
        assert_eq!(d.mode, Some(Mode::Metro));
        let d = detect("ofisteyim", &ps).unwrap();
        assert_eq!(
            (d.kind, d.place),
            (MoveKind::Arrive, Some(PlaceRef::Known(is)))
        );
        let d = detect("eve döndüm", &ps).unwrap();
        assert_eq!(d.place, Some(PlaceRef::Known(ev)));
        let d = detect("Ayhan Kimya'ya geldim, arabayla", &ps).unwrap();
        assert_eq!(d.place, Some(PlaceRef::New("Ayhan Kimya".into())));
        assert_eq!(d.mode, Some(Mode::Araba));
        let d = detect("Bugün Kadıköy'den ayrıldım", &ps).unwrap();
        assert_eq!(d.place, Some(PlaceRef::New("Kadıköy".into())));
        let d = detect("markete gidiyorum", &ps).unwrap();
        assert_eq!(d.to, Some(PlaceRef::New("Market".into())));
        let d = detect("yola çıktım metrobüsle", &ps).unwrap();
        assert_eq!((d.place, d.to, d.mode), (None, None, Some(Mode::Metrobus)));
        let d = detect("geldim", &ps).unwrap();
        assert_eq!((d.kind, d.place), (MoveKind::Arrive, None));
        assert!(detect("Kostik siparişini ver", &ps).is_none());
        assert!(detect("eve süt al", &ps).is_none(), "fiil yok");
        assert!(!has_cue("toplantı notları"));
        assert!(has_cue("İşe geldim"));
        let d = detect("Taksiyle işe geldim, 250 tl verdim", &ps).unwrap();
        assert!(!d.only_movement, "harcama da var → AI de bakar");
        assert_eq!(d.mode, Some(Mode::Taksi));
        assert_eq!(
            detect("müşteri ziyareti için Ayhan Kimya'ya gidiyorum", &ps)
                .unwrap()
                .scope,
            Some(Scope::Is)
        );
    }

    #[test]
    fn refines_answers_and_asks_one_question() {
        let ps = places();
        let r = refine("metroyla, iş için", &ps);
        assert_eq!((r.mode, r.scope), (Some(Mode::Metro), Some(Scope::Is)));
        assert_eq!(
            refine("Ayhan Kimya", &ps).place,
            Some(PlaceRef::New("Ayhan Kimya".into()))
        );
        assert_eq!(refine("özel", &ps).scope, Some(Scope::Kisisel));
        assert_eq!(refine("iş", &ps).scope, Some(Scope::Is));
        assert_eq!(
            refine("ofise", &ps).place,
            Some(PlaceRef::Known(ps[1].topic))
        );
        let m = Movement {
            id: "m".into(),
            entry: EntryId::new(),
            at: t(8, 0),
            kind: MoveKind::Arrive,
            place: None,
            to: None,
            mode: None,
            scope: None,
            fix: None,
            check: Check::Waiting,
            only_movement: true,
            settled: false,
        };
        assert_eq!(
            question(&m, &ps, None).unwrap(),
            "Nereye geldin? İş için mi, özel mi?"
        );
        let dep = Movement {
            kind: MoveKind::Depart,
            place: Some(ps[0].topic),
            to: Some(ps[1].topic),
            ..m.clone()
        };
        assert_eq!(
            question(&dep, &ps, None).unwrap(),
            "İş'e neyle gidiyorsun (metro, araba, otobüs…)?"
        );
        let ok = Movement {
            mode: Some(Mode::Metro),
            ..dep
        };
        assert!(question(&ok, &ps, None).is_none(), "iş yerinin amacı belli");
    }

    #[test]
    fn location_verifies_learns_and_matches() {
        let mut ps = places();
        let home = Fix::from_degrees(40.990_000, 29.030_000, 20.0).unwrap();
        let near = Fix::from_degrees(40.990_900, 29.030_000, 30.0).unwrap();
        let far = Fix::from_degrees(41.020_000, 29.030_000, 20.0).unwrap();
        assert!((95..=105).contains(&home.distance_m(&near)));
        assert!(Fix::from_degrees(91.0, 0.0, 1.0).is_err());
        let m = Movement {
            id: "m".into(),
            entry: EntryId::new(),
            at: t(8, 0),
            kind: MoveKind::Depart,
            place: Some(ps[0].topic),
            to: None,
            mode: None,
            scope: None,
            fix: None,
            check: Check::Waiting,
            only_movement: true,
            settled: false,
        };
        assert!(location_essential(&m, &ps), "evin konumu henüz yok");
        assert_eq!(
            verify(&m, &home, &ps),
            (Check::Learned, None, Some(ps[0].topic))
        );
        ps[0].info.fix = Some(home);
        assert!(!location_essential(&m, &ps));
        assert!(matches!(verify(&m, &near, &ps).0, Check::Confirmed { .. }));
        assert!(matches!(verify(&m, &far, &ps).0, Check::Far { distance_m } if distance_m > 3000));
        let unknown = Movement {
            kind: MoveKind::Arrive,
            place: None,
            ..m
        };
        let (c, matched, _) = verify(&unknown, &near, &ps);
        assert!(matches!(c, Check::Matched { .. }));
        assert_eq!(matched, Some(ps[0].topic));
    }

    #[test]
    fn monthly_report_counts_stays_trips_and_costs() {
        let ps = places();
        let (ev, is) = (ps[0].topic, ps[1].topic);
        let supplier = TopicId::new();
        let mut all = ps.clone();
        all.push(Place {
            topic: supplier,
            name: "Ayhan Kimya".into(),
            info: PlaceInfo::new(&["ayhan kimya"], Some(Scope::Is)),
        });
        let mv = |at, kind, place, mode| Movement {
            id: format!("{at}"),
            entry: EntryId::new(),
            at,
            kind,
            place,
            to: None,
            mode,
            scope: None,
            fix: None,
            check: Check::Waiting,
            only_movement: true,
            settled: true,
        };
        let moves = vec![
            mv(t(8, 0), MoveKind::Depart, Some(ev), Some(Mode::Metro)),
            mv(t(8, 50), MoveKind::Arrive, Some(is), None),
            mv(t(13, 0), MoveKind::Depart, Some(is), Some(Mode::Taksi)),
            mv(t(13, 30), MoveKind::Arrive, Some(supplier), None),
            mv(t(15, 0), MoveKind::Depart, None, Some(Mode::Taksi)),
            mv(t(15, 30), MoveKind::Arrive, Some(is), None),
            mv(t(18, 0), MoveKind::Depart, None, Some(Mode::Araba)),
            mv(t(19, 0), MoveKind::Arrive, Some(ev), None),
            mv(t(20, 0), MoveKind::Depart, None, Some(Mode::Yuruyus)),
            mv(t(20, 10), MoveKind::Arrive, None, None),
        ];
        let spends = vec![
            SpendAt {
                at: t(13, 10),
                timed: true,
                amount: Money(25_000),
                scope: Scope::Is,
                category: "diger".into(),
            },
            SpendAt {
                at: t(14, 0),
                timed: true,
                amount: Money(185_000),
                scope: Scope::Is,
                category: "tedarik".into(),
            },
            SpendAt {
                at: t(0, 0),
                timed: false,
                amount: Money(150_000),
                scope: Scope::Kisisel,
                category: "akaryakit".into(),
            },
        ];
        let r = report(&moves, &all, &spends, t(0, 0), t(24, 0));
        assert_eq!(r.trips, 5);
        assert_eq!(r.trips_is, 4, "ev→iş, iş→tedarikçi, geri, iş→ev");
        assert_eq!(
            (r.trips_other, r.trips_unknown),
            (1, 0),
            "akşam yürüyüşü: hedef yok, evden çıkıldı → ev"
        );
        assert_eq!(r.by_mode[0], (Mode::Taksi, 2));
        assert_eq!(r.travel_minutes, 50 + 30 + 30 + 60 + 10);
        let work = r.places.iter().find(|p| p.place == is).unwrap();
        assert_eq!(work.visits, 2);
        assert_eq!(work.stay_minutes, 250 + 150);
        let sup = r.places.iter().find(|p| p.place == supplier).unwrap();
        assert_eq!(
            (sup.visits, sup.stay_minutes, sup.cost),
            (1, 90, Money(185_000))
        );
        assert_eq!(r.transport_is, Money(25_000), "taksi yolda girildi");
        assert_eq!(r.transport_other, Money(150_000), "yakıt (ekstre)");
        assert_eq!(r.at_places, Money(185_000));
        assert_eq!(duration_label(550), "9 sa 10 dk");
        assert_eq!(duration_label(45), "45 dk");
    }
}
