//! Türkçe büyük/küçük harf dönüşümü ve tarih adları.
//!
//! `str::to_uppercase` Türkçe için yanlıştır: "i" → "I" yapar, oysa doğrusu "İ"dir.

use chrono::{Datelike, NaiveDate, Weekday};

/// Türkçe kurallarıyla büyük harfe çevirir ("işlenmedi" → "İŞLENMEDİ").
#[must_use]
pub fn tr_upper(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            'i' => out.push('İ'),
            'ı' => out.push('I'),
            other => out.extend(other.to_uppercase()),
        }
    }
    out
}

/// Türkçe kurallarıyla küçük harfe çevirir ("İSTANBUL" → "istanbul", "IĞDIR" → "ığdır").
#[must_use]
pub fn tr_lower(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            'İ' => out.push('i'),
            'I' => out.push('ı'),
            other => out.extend(other.to_lowercase()),
        }
    }
    out
}

fn last_vowel_back(name: &str) -> bool {
    tr_lower(name)
        .chars()
        .rev()
        .find(|c| "aeıioöuü".contains(*c))
        .is_none_or(|c| "aıou".contains(c))
}

/// Özel ada "-e/-a" hal eki, kesme işaretiyle: "İş'e", "Ev'e", "Ayhan Kimya'ya", "Kadıköy'e".
#[must_use]
pub fn dative(name: &str) -> String {
    let ends_vowel = tr_lower(name)
        .chars()
        .last()
        .is_some_and(|c| "aeıioöuü".contains(c));
    let v = if last_vowel_back(name) { 'a' } else { 'e' };
    if ends_vowel {
        format!("{name}'y{v}")
    } else {
        format!("{name}'{v}")
    }
}

/// Özel ada "-den/-dan/-ten/-tan" eki: "İş'ten", "Ev'den", "Ayhan Kimya'dan".
#[must_use]
pub fn ablative(name: &str) -> String {
    let hard = tr_lower(name)
        .chars()
        .last()
        .is_some_and(|c| "çfhkpsşt".contains(c));
    let d = if hard { 't' } else { 'd' };
    let v = if last_vowel_back(name) { "an" } else { "en" };
    format!("{name}'{d}{v}")
}

/// Haftanın günü adı ("Çarşamba").
#[must_use]
pub const fn weekday_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "Pazartesi",
        Weekday::Tue => "Salı",
        Weekday::Wed => "Çarşamba",
        Weekday::Thu => "Perşembe",
        Weekday::Fri => "Cuma",
        Weekday::Sat => "Cumartesi",
        Weekday::Sun => "Pazar",
    }
}

/// Ay adı ("Kasım"). `month` 1–12 dışındaysa boş döner.
#[must_use]
pub const fn month_name(month: u32) -> &'static str {
    match month {
        1 => "Ocak",
        2 => "Şubat",
        3 => "Mart",
        4 => "Nisan",
        5 => "Mayıs",
        6 => "Haziran",
        7 => "Temmuz",
        8 => "Ağustos",
        9 => "Eylül",
        10 => "Ekim",
        11 => "Kasım",
        12 => "Aralık",
        _ => "",
    }
}

/// Kısa ay adı ("Kas").
#[must_use]
pub fn month_short(month: u32) -> String {
    month_name(month).chars().take(3).collect()
}

/// "6 Kasım" ya da yıl farklıysa "6 Kasım 2025".
#[must_use]
pub fn date_long(date: NaiveDate, current_year: i32) -> String {
    if date.year() == current_year {
        format!("{} {}", date.day(), month_name(date.month()))
    } else {
        format!(
            "{} {} {}",
            date.day(),
            month_name(date.month()),
            date.year()
        )
    }
}

#[cfg(test)]
mod suffix_tests {
    use super::{ablative, dative};

    #[test]
    fn case_suffixes_follow_harmony() {
        assert_eq!(dative("İş"), "İş'e");
        assert_eq!(dative("Ayhan Kimya"), "Ayhan Kimya'ya");
        assert_eq!(dative("Kadıköy"), "Kadıköy'e");
        assert_eq!(dative("Ofis"), "Ofis'e");
        assert_eq!(dative("Market"), "Market'e");
        assert_eq!(dative("Okul"), "Okul'a");
        assert_eq!(ablative("İş"), "İş'ten");
        assert_eq!(ablative("Ev"), "Ev'den");
        assert_eq!(ablative("Ayhan Kimya"), "Ayhan Kimya'dan");
        assert_eq!(ablative("Kadıköy"), "Kadıköy'den");
        assert_eq!(ablative("Depo"), "Depo'dan");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upper_handles_dotted_and_dotless_i() {
        assert_eq!(tr_upper("işlenmedi"), "İŞLENMEDİ");
        assert_eq!(tr_upper("ılık ığdır"), "ILIK IĞDIR");
        assert_eq!(tr_upper("görev · perşembe"), "GÖREV · PERŞEMBE");
    }

    #[test]
    fn lower_handles_dotted_and_dotless_i() {
        assert_eq!(tr_lower("İSTANBUL"), "istanbul");
        assert_eq!(tr_lower("IĞDIR"), "ığdır");
        assert_eq!(tr_lower("ŞÇÖÜĞ"), "şçöüğ");
    }

    #[test]
    fn names() {
        assert_eq!(weekday_name(Weekday::Wed), "Çarşamba");
        assert_eq!(month_name(11), "Kasım");
        assert_eq!(month_short(11), "Kas");
        assert_eq!(month_short(8), "Ağu");
        assert_eq!(month_name(13), "");
    }

    #[test]
    fn long_dates() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
        assert_eq!(date_long(d, 2026), "4 Kasım");
        assert_eq!(date_long(d, 2027), "4 Kasım 2026");
    }
}
