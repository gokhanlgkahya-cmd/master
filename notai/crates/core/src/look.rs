//! Konunun görünümü (Revizyon 2, Gökhan 2026-11-07): simge ve renk.
//!
//! - **Simge:** elle seçilmişse o; değilse konu adından ve konuya düşen son kayıtlardan
//!   Türkçe anahtar kelimelerle seçilir (kayıt geldikçe değişebilir). Hiçbiri tutmazsa türün
//!   varsayılanı (Proje → klasör, Alan → katman, Kaynak → kitap, Arşiv → arşiv).
//! - **Renk:** oluşturulma sırasıyla paletten bir yuva (1–8); ad ya da simge değişse de renk
//!   değişmez, ilk 8 konu hep farklı renktedir.
//!   Renklerin kendisi `design/tokens.json`'da (`color.theme.*.topic-N-*`).
//!
//! Yerel ve deterministik: AI gerekmez, internetsiz çalışır, her cihazda aynı sonucu verir.

use crate::TopicId;
use crate::topic::{ParaKind, Topic};
use crate::tr::tr_lower;

/// Renk yuvası sayısı (tokens: topic-1 … topic-8).
pub const COLOR_SLOTS: u8 = 8;

/// Simge seçiminde bakılan en fazla son kayıt sayısı.
pub const RECENT_FOR_ICON: usize = 20;

/// Konu adındaki bir eşleşmenin ağırlığı (bir kayıttaki eşleşme 1 sayılır).
const NAME_WEIGHT: u32 = 3;
/// Addan eşleşme yoksa kayıtlardan simge seçmek için gereken en az kayıt sayısı.
const MIN_ENTRY_HITS: u32 = 2;

/// Simge → anahtar kelime kökleri (küçük harf, Türkçe). 4 harften kısa kökler yalnız tam
/// kelime olarak eşleşir ("ev", "iş", "su"); uzunlar kelime başı olarak ("kostik" → "kostikten").
const ICONS: &[(&str, &[&str])] = &[
    (
        "flask",
        &[
            "kimya",
            "kostik",
            "naoh",
            "laboratuvar",
            "deney",
            "formül",
            "asit",
            "reaktif",
            "solvent",
        ],
    ),
    (
        "drop",
        &[
            "sabun", "yağ", "su", "sıvı", "losyon", "şampuan", "krem", "parfüm",
        ],
    ),
    (
        "leaf",
        &[
            "bahçe", "bitki", "zeytin", "fidan", "tarla", "tarım", "çiçek", "sera", "gübre",
            "hasat",
        ],
    ),
    (
        "cube",
        &[
            "ürün", "ambalaj", "paket", "stok", "numune", "kutu", "depo", "envanter",
        ],
    ),
    (
        "truck",
        &[
            "kargo", "lojistik", "nakliye", "sevkiyat", "tedarik", "gönderi", "teslimat",
            "ithalat", "ihracat",
        ],
    ),
    (
        "factory",
        &["üretim", "atölye", "imalat", "fabrika", "makine"],
    ),
    (
        "chart",
        &[
            "satış",
            "rapor",
            "analiz",
            "borsa",
            "yatırım",
            "hisse",
            "trade",
            "trading",
            "kripto",
            "bist",
            "portföy",
            "grafik",
        ],
    ),
    (
        "wallet",
        &[
            "para", "bütçe", "fatura", "ödeme", "banka", "kredi", "borç", "harcama", "finans",
            "maaş", "vergi", "sigorta",
        ],
    ),
    (
        "cart",
        &[
            "alışveriş",
            "market",
            "sipariş",
            "satın",
            "trendyol",
            "amazon",
        ],
    ),
    (
        "house",
        &[
            "ev", "evin", "evde", "kira", "aidat", "tadilat", "mobilya", "apartman", "daire",
            "temizlik",
        ],
    ),
    (
        "car",
        &[
            "araba", "araç", "oto", "otomobil", "lastik", "kasko", "benzin", "plaka", "ehliyet",
        ],
    ),
    (
        "plane",
        &[
            "seyahat", "tatil", "uçuş", "uçak", "otel", "gezi", "yolculuk", "bilet", "vize",
            "pasaport",
        ],
    ),
    (
        "pill",
        &[
            "sağlık", "doktor", "ilaç", "hastane", "tansiyon", "diş", "tedavi", "tahlil", "aşı",
            "eczane",
        ],
    ),
    (
        "dumbbell",
        &[
            "spor",
            "antrenman",
            "koşu",
            "fitness",
            "gym",
            "yürüyüş",
            "diyet",
            "yoga",
            "pilates",
            "maraton",
            "bisiklet",
        ],
    ),
    (
        "heart",
        &["sevgi", "ilişki", "evlilik", "düğün", "nişan", "sevgili"],
    ),
    (
        "users",
        &[
            "aile", "çocuk", "anne", "baba", "kardeş", "eş", "arkadaş", "ekip",
        ],
    ),
    (
        "grad",
        &[
            "okul",
            "eğitim",
            "ders",
            "kurs",
            "sınav",
            "üniversite",
            "ödev",
            "öğren",
            "sertifika",
        ],
    ),
    (
        "book",
        &["kitap", "okuma", "makale", "roman", "yazar", "kütüphane"],
    ),
    (
        "code",
        &[
            "yazılım",
            "kod",
            "program",
            "uygulama",
            "rust",
            "github",
            "api",
            "web",
            "site",
        ],
    ),
    (
        "server",
        &[
            "sunucu",
            "n8n",
            "docker",
            "linux",
            "ubuntu",
            "termux",
            "tailscale",
        ],
    ),
    (
        "cpu",
        &["bilgisayar", "donanım", "elektronik", "laptop", "tablet"],
    ),
    (
        "wrench",
        &[
            "tamir", "bakım", "arıza", "usta", "tesisat", "kombi", "servis", "onarım",
        ],
    ),
    (
        "briefcase",
        &[
            "iş",
            "işi",
            "şirket",
            "müşteri",
            "toplantı",
            "ofis",
            "kariyer",
            "mesai",
            "ihale",
            "teklif",
        ],
    ),
    (
        "music",
        &["müzik", "şarkı", "gitar", "piyano", "konser", "albüm"],
    ),
    ("gift", &["hediye", "doğum", "yıldönümü", "sürpriz"]),
    ("paw", &["kedi", "köpek", "veteriner", "mama", "evcil"]),
    (
        "utensils",
        &["yemek", "tarif", "mutfak", "restoran", "kahvaltı"],
    ),
    ("camera", &["fotoğraf", "çekim", "video", "kamera"]),
    (
        "calendar",
        &["etkinlik", "randevu", "organizasyon", "davet"],
    ),
];

/// Tüm otomatik seçilebilen simge adları (tasarımdaki sprite'ta bulunmaları sınanır).
#[must_use]
pub fn auto_icons() -> Vec<&'static str> {
    ICONS.iter().map(|(i, _)| *i).collect()
}

fn words(text: &str) -> impl Iterator<Item = String> {
    tr_lower(text)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>()
        .into_iter()
}

fn word_matches(word: &str, stem: &str) -> bool {
    if stem.chars().count() < 4 {
        word == stem
    } else {
        word.starts_with(stem)
    }
}

/// Metindeki kelimelerden biri simgenin anahtarlarından birine uyuyor mu?
fn hits(words: &[String], stems: &[&str]) -> bool {
    words
        .iter()
        .any(|w| stems.iter().any(|s| word_matches(w, s)))
}

/// Addan ve son kayıtlardan simge önerir. Liste sırası: nesne/alan simgeleri (araba, ev) genel
/// eylemlerden (tamir, iş) önce gelir; "Araba bakımı" → araba. Ad eşleşmesi 3, her kayıt
/// 1 puan; addan eşleşme yoksa en az 2 kaydın aynı simgeyi göstermesi gerekir (tek kayıt
/// simgeyi değiştirmez). Eşitlikte listedeki sıra kazanır.
#[must_use]
pub fn suggest_icon<S: AsRef<str>>(name: &str, recent: &[S]) -> Option<&'static str> {
    let name_words: Vec<String> = words(name).collect();
    let entry_words: Vec<Vec<String>> = recent
        .iter()
        .take(RECENT_FOR_ICON)
        .map(|t| words(t.as_ref()).collect())
        .collect();
    let mut best: Option<(&'static str, u32)> = None;
    for (icon, stems) in ICONS {
        let from_name = if hits(&name_words, stems) {
            NAME_WEIGHT
        } else {
            0
        };
        let from_entries = u32::try_from(entry_words.iter().filter(|w| hits(w, stems)).count())
            .unwrap_or(u32::MAX);
        if from_name == 0 && from_entries < MIN_ENTRY_HITS {
            continue;
        }
        let score = from_name.saturating_add(from_entries);
        if best.is_none_or(|(_, s)| score > s) {
            best = Some((icon, score));
        }
    }
    best.map(|(i, _)| i)
}

/// Türün varsayılan simgesi.
#[must_use]
pub const fn kind_icon(kind: &ParaKind) -> &'static str {
    match kind {
        ParaKind::Project { .. } => "folder",
        ParaKind::Area => "layers",
        ParaKind::Resource => "book",
        ParaKind::Archive => "archive",
    }
}

/// Konunun gösterilecek simgesi: elle seçilen > addan/kayıtlardan > türün varsayılanı.
#[must_use]
pub fn icon_for<S: AsRef<str>>(topic: &Topic, recent: &[S]) -> String {
    topic.icon.clone().unwrap_or_else(|| {
        if let Some(a) = &topic.asset {
            return a.kind.icon().to_owned();
        }
        suggest_icon(topic.name.as_str(), recent)
            .unwrap_or_else(|| kind_icon(&topic.kind))
            .to_owned()
    })
}

/// Renk yuvaları (1–8): konular oluşturulma sırasıyla (UUID v7 kimlik = zaman sırası) paletten
/// sırayla renk alır. İlk 8 konu birbirinden farklı renktedir; yeni konu eskilerin rengini
/// değiştirmez, arşivlenen konu da sırada kalır. (Tek istisna: iki cihazda çevrimdışı açılan
/// konular senkronda araya girerse sonrakilerin rengi bir kez kayabilir.)
#[must_use]
pub fn color_slots(
    ids: impl IntoIterator<Item = TopicId>,
) -> std::collections::HashMap<TopicId, u8> {
    let mut ids: Vec<TopicId> = ids.into_iter().collect();
    ids.sort_unstable();
    ids.dedup();
    ids.into_iter()
        .enumerate()
        .map(|(i, id)| {
            let slot = u8::try_from(i % usize::from(COLOR_SLOTS)).unwrap_or(0) + 1;
            (id, slot)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::text::TopicName;

    const NONE: [&str; 0] = [];

    #[test]
    fn name_decides_icon() {
        assert_eq!(suggest_icon("Kostik tedarik", &NONE), Some("flask"));
        assert_eq!(suggest_icon("Araba bakımı", &NONE), Some("car"));
        assert_eq!(suggest_icon("Ev", &NONE), Some("house"));
        assert_eq!(suggest_icon("Okuma notları", &NONE), Some("book"));
        assert_eq!(
            suggest_icon("SAĞLIK", &NONE),
            Some("pill"),
            "Türkçe büyük harf"
        );
        assert_eq!(suggest_icon("İzmir tatili", &NONE), Some("plane"));
        assert_eq!(suggest_icon("GemOlive", &NONE), None, "marka adı bilinmez");
    }

    #[test]
    fn short_stems_match_whole_words_only() {
        assert_eq!(suggest_icon("Evrak", &NONE), None, "\"ev\" evrak'a uymaz");
        assert_eq!(
            suggest_icon("İşlemler", &NONE),
            None,
            "\"iş\" işlem'e uymaz"
        );
        assert_eq!(suggest_icon("Yeni iş", &NONE), Some("briefcase"));
    }

    #[test]
    fn entries_decide_when_name_is_silent_and_need_two() {
        let one = ["Zeytinyağlı sabunda kür süresi 4–6 hafta."];
        assert_eq!(suggest_icon("GemOlive", &one), None, "tek kayıt yetmez");
        let two = [
            "Zeytinyağlı sabunda kür süresi 4–6 hafta.",
            "Sabun kalıbı ölçüsünü öğren",
        ];
        assert_eq!(suggest_icon("GemOlive", &two), Some("drop"));
    }

    #[test]
    fn name_outweighs_a_couple_of_entries() {
        let recent = ["Kargo yarın çıkıyor", "Kargo firmasını ara"];
        assert_eq!(
            suggest_icon("Kostik tedarik", &recent),
            Some("truck"),
            "3+2: tedarik+kargo"
        );
        assert_eq!(
            suggest_icon("Kostik", &["Kargo yarın", "Kargo geldi"]),
            Some("flask"),
            "ad 3, kayıtlar 2"
        );
    }

    #[test]
    fn manual_icon_wins_then_auto_then_kind() {
        let mut t = Topic::new(TopicName::new("Araba").unwrap(), ParaKind::Area);
        assert_eq!(icon_for(&t, &NONE), "car");
        t.icon = Some("heart".into());
        assert_eq!(icon_for(&t, &NONE), "heart");
        let r = Topic::new(TopicName::new("Diğer").unwrap(), ParaKind::Resource);
        assert_eq!(icon_for(&r, &NONE), "book");
        let p = Topic::new(
            TopicName::new("Diğer").unwrap(),
            ParaKind::Project { due: None },
        );
        assert_eq!(icon_for(&p, &NONE), "folder");
    }

    #[test]
    fn colors_follow_creation_order_and_do_not_shift() {
        let ids: Vec<TopicId> = (0..10).map(|_| TopicId::new()).collect();
        let first = color_slots(ids[..8].iter().copied());
        let mut seen: Vec<u8> = ids[..8].iter().map(|id| first[id]).collect();
        seen.sort_unstable();
        assert_eq!(seen, [1, 2, 3, 4, 5, 6, 7, 8], "ilk 8 konu farklı renk");
        // Yeni konular eskilerin rengini değiştirmez; 9. konu başa döner.
        let all = color_slots(ids.iter().copied());
        for id in &ids[..8] {
            assert_eq!(all[id], first[id]);
        }
        assert_eq!(all[&ids[8]], 1);
        assert_eq!(all[&ids[9]], 2);
    }
}
