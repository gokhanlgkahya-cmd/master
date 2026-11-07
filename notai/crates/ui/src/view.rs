//! Saf görünüm yardımcıları (DOM'suz; yerel testlerle sınanır).

use chrono::{Datelike, Days, NaiveDate, NaiveTime};
use notai_api::{
    EntryView, ReminderGroupView, TaskEditRequest, TaskRowView, TasksView, TopicRowView,
};

const MONTHS: [&str; 12] = [
    "Oca", "Şub", "Mar", "Nis", "May", "Haz", "Tem", "Ağu", "Eyl", "Eki", "Kas", "Ara",
];

fn short_date(d: NaiveDate) -> String {
    let m = MONTHS.get(d.month0() as usize).copied().unwrap_or("");
    format!("{}\u{a0}{m}", d.day())
}

/// Konunun simgesi: kullanıcının seçtiği, yoksa türüne göre.
#[must_use]
pub fn topic_icon(icon: Option<&str>, kind: &str) -> String {
    icon.map_or_else(
        || match kind {
            "resource" => "book".to_owned(),
            "area" => "layers".to_owned(),
            _ => "folder".to_owned(),
        },
        str::to_owned,
    )
}

/// Konunun görünümü: simge, renk yuvası ve varsa profil fotoğrafı (sunucu/kabuk
/// `notai_core::look` ile hesaplar).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicLook {
    pub icon: String,
    pub color: u8,
    pub avatar: Option<String>,
}

impl TopicLook {
    #[must_use]
    pub fn of(t: &TopicRowView) -> Self {
        Self {
            icon: topic_icon(t.icon.as_deref(), &t.kind),
            color: t.color,
            avatar: t.avatar.clone(),
        }
    }

    /// Renksiz, yalnız simgeli görünüm (Ana Günlük, yükleniyor).
    #[must_use]
    pub fn plain(icon: &str) -> Self {
        Self {
            icon: icon.to_owned(),
            color: 0,
            avatar: None,
        }
    }

    /// Yuvarlak profilin sınıfı ("avatar avatar--c3", küçükse "+ avatar--sm").
    #[must_use]
    pub fn avatar_class(&self, small: bool) -> String {
        let mut c = String::from("avatar");
        if self.color > 0 {
            c.push_str(&format!(" avatar--c{}", self.color));
        }
        if small {
            c.push_str(" avatar--sm");
        }
        c
    }

    /// Kayıt üstündeki konu adının sınıfı (yalnız simge renk alır).
    #[must_use]
    pub fn meta_class(&self) -> String {
        if self.color == 0 {
            "meta__topic".to_owned()
        } else {
            format!("meta__topic meta__topic--c{}", self.color)
        }
    }
}

/// Bildirimin açacağı yer: konusu varsa o konu, yoksa Ana Günlük.
#[must_use]
pub fn notice_target(n: &notai_api::NoticeView) -> Option<notai_api::TopicRef> {
    match (&n.topic_id, &n.topic_name) {
        (Some(id), Some(name)) => Some(notai_api::TopicRef {
            id: id.clone(),
            name: name.clone(),
        }),
        _ => None,
    }
}

/// Android'de bildirime dokunma olayının içinden (konu kimliği, konu adı).
#[must_use]
pub fn tap_target(payload: &serde_json::Value) -> Option<notai_api::TopicRef> {
    // Eklenti olayı: { actionId, notification: { extra: { topic_id, topic_name, … } } }
    // (bazı sürümlerde Channel mesajı `{ message: … }` sarmalıdır).
    let p = payload.get("message").unwrap_or(payload);
    let extra = p.get("notification")?.get("extra")?;
    let id = extra.get("topic_id")?.as_str()?;
    let name = extra
        .get("topic_name")
        .and_then(|n| n.as_str())
        .unwrap_or("Konu");
    Some(notai_api::TopicRef {
        id: id.to_owned(),
        name: name.to_owned(),
    })
}

/// Ana ekran hap filtresi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeFilter {
    All,
    Unprocessed,
    Projects,
    Areas,
    Resources,
}

impl HomeFilter {
    pub const ALL: [Self; 5] = [
        Self::All,
        Self::Unprocessed,
        Self::Projects,
        Self::Areas,
        Self::Resources,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "Tümü",
            Self::Unprocessed => "İşlenmemiş",
            Self::Projects => "Proje",
            Self::Areas => "Alan",
            Self::Resources => "Kaynak",
        }
    }

    /// Sunucudaki konu türü filtresi.
    #[must_use]
    pub const fn kind_param(self) -> &'static str {
        match self {
            Self::All | Self::Unprocessed => "all",
            Self::Projects => "projects",
            Self::Areas => "areas",
            Self::Resources => "resources",
        }
    }

    /// Ana Günlük ve Haftalık gözden geçirme satırları bu filtrede görünür mü?
    #[must_use]
    pub const fn shows_journal(self, unprocessed: u64) -> bool {
        match self {
            Self::All => true,
            Self::Unprocessed => unprocessed > 0,
            _ => false,
        }
    }

    #[must_use]
    pub const fn shows_review(self) -> bool {
        matches!(self, Self::All)
    }

    /// Konu satırı bu filtrede görünür mü? (Tür filtresini sunucu uygular.)
    #[must_use]
    pub const fn keeps(self, t: &TopicRowView) -> bool {
        match self {
            Self::Unprocessed => t.pending > 0,
            _ => true,
        }
    }
}

/// Konu kimliği → görünüm. Akış başlığında ve kayıtların üstündeki konu adında ana ekranla
/// aynı simge ve renk görünsün diye.
#[must_use]
pub fn topic_looks(topics: &[TopicRowView]) -> std::collections::HashMap<String, TopicLook> {
    topics
        .iter()
        .map(|t| (t.id.clone(), TopicLook::of(t)))
        .collect()
}

/// Konu satırındaki tür ("Proje · 20 Kas").
#[must_use]
pub fn kind_line(label: &str, due: Option<NaiveDate>) -> String {
    match due {
        Some(d) => format!("{label} · {}", short_date(d)),
        None => label.to_owned(),
    }
}

/// Akış başlığının alt satırı.
#[must_use]
pub fn unprocessed_line(n: u64) -> String {
    match n {
        0 => "Hepsi işlendi".to_owned(),
        n => format!("{n} işlenmemiş kayıt"),
    }
}

/// Görevler başlığının alt satırı ("2 geciken · 3 bugün").
#[must_use]
pub fn tasks_line(overdue: usize, today: usize) -> String {
    match (overdue, today) {
        (0, 0) => "Bugün için iş yok".to_owned(),
        (0, t) => format!("{t} bugün"),
        (o, 0) => format!("{o} geciken"),
        (o, t) => format!("{o} geciken · {t} bugün"),
    }
}

/// Sabah planı maddesinin alt satır sınıfı (renk yalnız anlam taşır: gecikme = hata).
#[must_use]
pub fn plan_reason_class(kind: &str) -> &'static str {
    match kind {
        "overdue" => "plan__sub plan__sub--error",
        "today" => "plan__sub plan__sub--warning",
        _ => "plan__sub",
    }
}

/// Görev satırının meta metni ("1 gün gecikti", "Cmt 14 Kas 10:00").
#[must_use]
pub fn task_meta(t: &TaskRowView) -> String {
    let mut parts = Vec::new();
    if t.overdue_days > 0 {
        parts.push(format!("{}\u{a0}gün gecikti", t.overdue_days));
    } else if let Some(d) = t.due_date {
        parts.push(short_date(d));
    }
    if let Some(time) = t.due_time {
        parts.push(time.format("%H:%M").to_string());
    }
    if t.recurring {
        parts.push("tekrarlı".to_owned());
    }
    parts.join(" · ")
}

/// Detay sayfasının meta satırı ("GemOlive · 09:12 · İşlenmedi").
#[must_use]
pub fn sheet_meta(e: &EntryView) -> String {
    let place = e.topic.as_ref().map_or("Ana Günlük", |t| t.name.as_str());
    let state = if e.unprocessed {
        "İşlenmedi"
    } else {
        "İşlendi"
    };
    format!("{place} · {} · {state}", e.time)
}

// ---------------- v0.2–v0.4 yardımcıları ----------------

const MONTHS_LONG: [&str; 12] = [
    "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim",
    "Kasım", "Aralık",
];
const WEEKDAYS_SHORT: [&str; 7] = ["Pzt", "Sal", "Çar", "Per", "Cum", "Cmt", "Paz"];
const WEEKDAYS_LONG: [&str; 7] = [
    "Pazartesi",
    "Salı",
    "Çarşamba",
    "Perşembe",
    "Cuma",
    "Cumartesi",
    "Pazar",
];

fn month_long(d: NaiveDate) -> &'static str {
    MONTHS_LONG.get(d.month0() as usize).copied().unwrap_or("")
}

/// Haftanın kısa gün adı ("Pzt").
#[must_use]
pub fn weekday_short(d: NaiveDate) -> &'static str {
    WEEKDAYS_SHORT
        .get(d.weekday().num_days_from_monday() as usize)
        .copied()
        .unwrap_or("")
}

/// Uzun tarih ("Cuma, 6 Kasım").
#[must_use]
pub fn long_date(d: NaiveDate) -> String {
    let wd = WEEKDAYS_LONG
        .get(d.weekday().num_days_from_monday() as usize)
        .copied()
        .unwrap_or("");
    format!("{wd}, {}\u{a0}{}", d.day(), month_long(d))
}

/// Ay ve yıl ("Kasım 2026").
#[must_use]
pub fn month_year(d: NaiveDate) -> String {
    format!("{} {}", month_long(d), d.year())
}

/// Tarihin bulunduğu haftanın pazartesisi.
#[must_use]
pub fn week_monday(d: NaiveDate) -> NaiveDate {
    let back = u64::from(d.weekday().num_days_from_monday());
    d.checked_sub_days(Days::new(back)).unwrap_or(d)
}

/// `monday`'den `weeks` hafta ileri/geri.
#[must_use]
pub fn shift_weeks(monday: NaiveDate, weeks: i32) -> NaiveDate {
    let days = Days::new(u64::from(weeks.unsigned_abs()) * 7);
    if weeks >= 0 {
        monday.checked_add_days(days).unwrap_or(monday)
    } else {
        monday.checked_sub_days(days).unwrap_or(monday)
    }
}

/// Haftanın yedi günü.
#[must_use]
pub fn week_days(monday: NaiveDate) -> Vec<NaiveDate> {
    (0..7)
        .filter_map(|i| monday.checked_add_days(Days::new(i)))
        .collect()
}

/// Hafta değişince seçilecek gün: bugün o haftadaysa bugün, değilse pazartesi.
#[must_use]
pub fn day_in_week(monday: NaiveDate, today: NaiveDate) -> NaiveDate {
    if week_monday(today) == monday {
        today
    } else {
        monday
    }
}

/// Ayın ilk günü.
#[must_use]
pub fn month_first(d: NaiveDate) -> NaiveDate {
    d.with_day(1).unwrap_or(d)
}

/// `d`'nin ayından `months` ay ileri/geri; ayın ilk günü.
#[must_use]
pub fn shift_months(d: NaiveDate, months: i32) -> NaiveDate {
    let first = month_first(d);
    let m = chrono::Months::new(months.unsigned_abs());
    if months >= 0 {
        first.checked_add_months(m)
    } else {
        first.checked_sub_months(m)
    }
    .unwrap_or(first)
}

/// Ay ızgarası: ayın ilk gününün haftasının pazartesisinden başlayan 6 hafta (42 gün;
/// sunucunun takvim ucu da en fazla 42 gün döndürür).
#[must_use]
pub fn month_grid(d: NaiveDate) -> Vec<NaiveDate> {
    let start = week_monday(month_first(d));
    (0..42)
        .filter_map(|i| start.checked_add_days(Days::new(i)))
        .collect()
}

/// Ay değişince seçilecek gün: bugün o aydaysa bugün, değilse ayın ilk günü.
#[must_use]
pub fn day_in_month(first: NaiveDate, today: NaiveDate) -> NaiveDate {
    if month_first(today) == month_first(first) {
        today
    } else {
        month_first(first)
    }
}

/// Hafta aralığı ("2–8 Kasım", "30 Kasım – 6 Aralık").
#[must_use]
pub fn week_range(start: NaiveDate) -> String {
    let end = start.checked_add_days(Days::new(6)).unwrap_or(start);
    if start.month() == end.month() {
        format!("{}–{}\u{a0}{}", start.day(), end.day(), month_long(end))
    } else {
        format!(
            "{}\u{a0}{}\u{a0}– {}\u{a0}{}",
            start.day(),
            month_long(start),
            end.day(),
            month_long(end)
        )
    }
}

/// Zaman çizelgesindeki saat sütunu ("14:00" ya da "Gün boyu").
#[must_use]
pub fn timeline_time(t: &TaskRowView) -> String {
    t.due_time
        .map_or_else(|| "Gün boyu".to_owned(), |x| x.format("%H:%M").to_string())
}

/// "Şimdi" çizgisinin konacağı sıra: saati şimdiden sonra olan ilk görevin önü.
/// Saatsiz (gün boyu) görevler en üstte durur; çizgi onların altına iner.
#[must_use]
pub fn nowline_index(times: &[Option<NaiveTime>], now: NaiveTime) -> usize {
    times
        .iter()
        .position(|t| t.is_some_and(|t| t > now))
        .unwrap_or(times.len())
}

/// Hatırlatmalar başlığının alt satırı ("1 kaçırılan · 3 bugün").
#[must_use]
pub fn reminders_line(groups: &[ReminderGroupView]) -> String {
    let count = |key: &str| {
        groups
            .iter()
            .filter(|g| g.key == key)
            .map(|g| g.items.len())
            .sum::<usize>()
    };
    match (count("missed"), count("today")) {
        (0, 0) if groups.iter().all(|g| g.items.is_empty()) => "Hatırlatma yok".to_owned(),
        (0, 0) => "Bugün hatırlatma yok".to_owned(),
        (0, t) => format!("{t} bugün"),
        (m, 0) => format!("{m} kaçırılan"),
        (m, t) => format!("{m} kaçırılan · {t} bugün"),
    }
}

/// Erteleme seçenekleri: (sunucu adı, etiket).
pub const SNOOZE_OPTIONS: [(&str, &str); 3] = [
    ("minutes10", "10\u{a0}dk"),
    ("hour1", "1\u{a0}sa"),
    ("tomorrow", "Yarın"),
];

/// Tekrar seçenekleri ("keep" = değiştirme).
pub const REPEAT_OPTIONS: [(&str, &str); 6] = [
    ("none", "Yok"),
    ("day", "Her gün"),
    ("weekday", "Hafta içi her gün"),
    ("week", "Her hafta"),
    ("month", "Her ay"),
    ("keep", "Değiştirme"),
];

/// Hatırlatma seçenekleri (dakika; negatif = yok, 0 = tam zamanında).
pub const REMIND_OPTIONS: [(i32, &str); 8] = [
    (-1, "Yok"),
    (0, "Tam zamanında"),
    (5, "5\u{a0}dk önce"),
    (10, "10\u{a0}dk önce"),
    (15, "15\u{a0}dk önce"),
    (30, "30\u{a0}dk önce"),
    (60, "1\u{a0}saat önce"),
    (1440, "1\u{a0}gün önce"),
];

/// Öncelik seçenekleri.
pub const PRIORITY_OPTIONS: [(&str, &str); 3] =
    [("high", "Yüksek"), ("normal", "Orta"), ("low", "Düşük")];

/// Konu türleri (yeni konu).
pub const TOPIC_KINDS: [(&str, &str); 3] = [
    ("project", "Proje"),
    ("area", "Alan"),
    ("resource", "Kaynak"),
];

/// Öncelik simgesi.
#[must_use]
pub fn priority_icon(p: &str) -> &'static str {
    match p {
        "high" => "prio-high",
        "low" => "prio-low",
        _ => "prio-mid",
    }
}

/// Görev düzenleme formunun durumu (yalnız metin; doğrulama sunucuda/depoda).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskForm {
    pub title: String,
    /// "YYYY-AA-GG" ya da boş.
    pub date: String,
    /// "SS:DD" ya da boş.
    pub time: String,
    pub priority: String,
    /// `REPEAT_OPTIONS` anahtarı.
    pub repeat: String,
    /// `None`: değiştirme.
    pub remind: Option<i32>,
}

impl TaskForm {
    /// Formu mevcut görevden doldurur.
    #[must_use]
    pub fn from_task(t: &TaskRowView) -> Self {
        Self {
            title: t.title.clone(),
            date: t
                .due_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default(),
            time: t
                .due_time
                .map(|x| x.format("%H:%M").to_string())
                .unwrap_or_default(),
            priority: t.priority.clone(),
            repeat: "keep".to_owned(),
            remind: None,
        }
    }
}

/// Yalnız değişen alanları taşıyan düzenleme isteği.
#[must_use]
pub fn task_edit(orig: &TaskRowView, form: &TaskForm) -> TaskEditRequest {
    let before = TaskForm::from_task(orig);
    let title = form.title.trim();
    let changed = |a: &str, b: &str| (a != b).then(|| a.to_owned());
    TaskEditRequest {
        title: (!title.is_empty() && title != before.title).then(|| title.to_owned()),
        due_date: changed(&form.date, &before.date),
        // Tarih kaldırılınca saat de gider; saat ayrıca gönderilmez.
        due_time: if form.date.is_empty() {
            None
        } else {
            changed(&form.time, &before.time)
        },
        priority: changed(&form.priority, &before.priority),
        repeat: (form.repeat != "keep").then(|| form.repeat.clone()),
        remind_before_minutes: form.remind,
        topic_id: None,
    }
}

/// Düzenleme isteği boş mu (gönderilecek bir şey yok)?
#[must_use]
pub fn edit_is_empty(r: &TaskEditRequest) -> bool {
    *r == TaskEditRequest::default()
}

/// Görev listesindeki tüm kimlikler.
#[must_use]
pub fn task_ids(v: &TasksView) -> Vec<String> {
    v.overdue
        .iter()
        .chain(&v.today)
        .chain(&v.upcoming)
        .chain(&v.undated)
        .map(|t| t.id.clone())
        .collect()
}

/// Hatırlatma/Etkinlik formundaki seçim → düzenleme isteğindeki dakika.
/// Hatırlatma formu her zaman hatırlatır (geçersiz seçim = tam zamanında); Etkinlik'te "Yok"
/// seçilirse hatırlatma kurulmaz.
#[must_use]
pub fn timed_remind(always: bool, choice: &str) -> Option<i32> {
    match choice.parse::<i32>() {
        Ok(m) if m >= 0 => Some(m),
        _ if always => Some(0),
        _ => None,
    }
}

/// Tema seçimi (Ayarlar → Görünüm).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// `<html data-theme>` değeri (`None`: öznitelik kaldırılır, sistem ayarı geçerli).
    #[must_use]
    pub const fn attr(self) -> Option<&'static str> {
        match self {
            Self::System => None,
            Self::Light => Some("light"),
            Self::Dark => Some("dark"),
        }
    }
}

/// Otonom eşik etiketi ("%90").
#[must_use]
pub fn threshold_label(t: f64) -> String {
    format!("%{}", (t * 100.0).round())
}

/// Kaydırıcı değeri (50–100) → eşik (0,5–1,0).
#[must_use]
pub fn threshold_from_input(v: &str) -> Option<f64> {
    let n: f64 = v.trim().parse().ok()?;
    (50.0..=100.0).contains(&n).then_some(n / 100.0)
}

/// Ek türüne göre yakalanacak kaydın metni ("Fotoğraf: ad.jpg", "Konum").
#[must_use]
pub fn attach_entry_text(kind: &str, name: &str) -> String {
    let label = match kind {
        "photo" => "Fotoğraf",
        "document" => "Belge",
        "location" => "Konum",
        "contact" => "Kişi",
        _ => "Ek",
    };
    if name.trim().is_empty() || kind == "location" {
        label.to_owned()
    } else {
        format!("{label}: {}", name.trim())
    }
}

/// Ek simgesi.
#[must_use]
pub fn attachment_icon(kind: &str) -> &'static str {
    match kind {
        "photo" => "image",
        "location" => "pin",
        "contact" => "user",
        "audio" => "mic",
        _ => "file",
    }
}

/// Konum değeri ("enlem,boylam", 6 ondalık).
#[must_use]
pub fn location_value(lat: f64, lon: f64) -> String {
    format!("{lat:.6},{lon:.6}")
}

/// Konum hatası (GeolocationPositionError.code).
#[must_use]
pub fn geo_error(code: u32) -> String {
    match code {
        1 => "Konum izni verilmedi.".to_owned(),
        2 => "Konum bulunamadı.".to_owned(),
        3 => "Konum zamanında alınamadı.".to_owned(),
        _ => "Konum alınamadı.".to_owned(),
    }
}

/// Mikrofon hatası (DOMException.name).
#[must_use]
pub fn mic_error(name: &str) -> String {
    match name {
        "NotAllowedError" | "SecurityError" => "Mikrofon izni verilmedi.".to_owned(),
        "NotFoundError" | "OverconstrainedError" => "Mikrofon bulunamadı.".to_owned(),
        "NotReadableError" | "AbortError" => "Mikrofon şu an kullanılamıyor.".to_owned(),
        "NotSupportedError" => "Bu cihazda ses kaydı desteklenmiyor.".to_owned(),
        _ => "Ses kaydı başlatılamadı.".to_owned(),
    }
}

/// Kayıt süresi ("0:07", "1:05").
#[must_use]
pub fn clock(secs: u32) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// Dosya boyutu ("640 B", "12 KB", "1,2 MB").
#[must_use]
pub fn size_label(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    if bytes < KB {
        format!("{bytes}\u{a0}B")
    } else if bytes < MB {
        format!("{}\u{a0}KB", bytes.div_ceil(KB))
    } else {
        let tenths = (bytes * 10).div_ceil(MB);
        format!("{},{}\u{a0}MB", tenths / 10, tenths % 10)
    }
}

/// Cevap kartının üst etiketi.
#[must_use]
pub fn answer_eyebrow(has_answer: bool, sources: usize) -> String {
    match (has_answer, sources) {
        (true, 0) => "Notlarından yanıt".to_owned(),
        (true, n) => format!("Notlarından yanıt · {n}\u{a0}kayıt"),
        (false, _) => "AI kapalı; ilgili kayıtlar:".to_owned(),
    }
}

/// Kaydın ilk satırı (başlık gibi gösterilir).
#[must_use]
pub fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_owned()
}

/// Liste oluştur: boş satırları atar; hiç madde yoksa `None`.
#[must_use]
pub fn list_text(raw: &str) -> Option<String> {
    let lines: Vec<&str> = raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    (!lines.is_empty()).then(|| lines.join("\n"))
}

/// Haftalık gözden geçirme: işlenmemiş kayıt cümlesi.
#[must_use]
pub fn unprocessed_lead(n: u64) -> String {
    match n {
        0 => "Ana Günlük temiz; karar bekleyen kayıt yok.".to_owned(),
        n => format!("{n}\u{a0}kayıt karar bekliyor. Çipi kabul et ya da yoksay."),
    }
}

/// Haftalık gözden geçirme: geciken görev cümlesi.
#[must_use]
pub fn overdue_lead(n: usize) -> String {
    match n {
        0 => "Geciken görev yok.".to_owned(),
        n => format!("{n}\u{a0}görev gecikti."),
    }
}

/// Haftalık gözden geçirme: geçen hafta biten.
#[must_use]
pub fn done_lead(n: u64) -> String {
    match n {
        0 => "Geçen hafta tamamlanan görev yok.".to_owned(),
        n => format!("Geçen hafta {n}\u{a0}görev tamamlandı."),
    }
}

/// Senkron sonucu ("3 gönderildi · 2 alındı").
#[must_use]
pub fn sync_result(r: Option<(u32, u32)>) -> String {
    r.map_or_else(
        || "Sunucu ayarlı değil".to_owned(),
        |(p, g)| format!("{p} gönderildi · {g} alındı"),
    )
}

/// Kabul edilmiş (geri alınabilir) öneriler.
#[must_use]
pub fn accepted(e: &EntryView) -> Vec<notai_api::SuggestionView> {
    e.suggestions
        .iter()
        .filter(|s| s.decision == "accepted")
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        assert_eq!(unprocessed_line(0), "Hepsi işlendi");
        assert_eq!(unprocessed_line(4), "4 işlenmemiş kayıt");
        assert_eq!(tasks_line(2, 3), "2 geciken · 3 bugün");
        assert_eq!(tasks_line(0, 0), "Bugün için iş yok");
        assert_eq!(
            kind_line("Proje", NaiveDate::from_ymd_opt(2026, 11, 20)),
            "Proje · 20\u{a0}Kas"
        );
        assert_eq!(topic_icon(None, "resource"), "book");
        assert_eq!(topic_icon(Some("drop"), "area"), "drop");
        let row = |id: &str, icon: Option<&str>, kind: &str| TopicRowView {
            id: id.into(),
            name: id.into(),
            kind: kind.into(),
            kind_label: String::new(),
            due_date: None,
            icon: icon.map(Into::into),
            color: 3,
            avatar: None,
            pending: 0,
            pinned: false,
            last_label: None,
            preview: None,
        };
        let looks = topic_looks(&[
            row("k", Some("flask"), "project"),
            row("o", None, "resource"),
        ]);
        assert_eq!(
            looks["k"].icon, "flask",
            "akış başlığı ana ekranla aynı simge"
        );
        assert_eq!(looks["o"].icon, "book");
        assert_eq!(looks["k"].avatar_class(false), "avatar avatar--c3");
        assert_eq!(
            looks["k"].avatar_class(true),
            "avatar avatar--c3 avatar--sm"
        );
        assert_eq!(looks["k"].meta_class(), "meta__topic meta__topic--c3");
        assert_eq!(TopicLook::plain("notebook").avatar_class(false), "avatar");
        // Hap filtreleri
        let mut t = looks_row_pending(0);
        assert!(HomeFilter::All.keeps(&t));
        assert!(!HomeFilter::Unprocessed.keeps(&t));
        t.pending = 2;
        assert!(HomeFilter::Unprocessed.keeps(&t));
        assert!(HomeFilter::Unprocessed.shows_journal(1));
        assert!(!HomeFilter::Unprocessed.shows_journal(0));
        assert!(!HomeFilter::Projects.shows_journal(5));
        assert!(HomeFilter::All.shows_review() && !HomeFilter::Unprocessed.shows_review());
        assert_eq!(HomeFilter::Areas.kind_param(), "areas");
        assert_eq!(plan_reason_class("overdue"), "plan__sub plan__sub--error");
    }

    #[test]
    fn notification_targets() {
        let n = notai_api::NoticeView {
            kind: "reminder".into(),
            title: "Hatırlatma · Ev".into(),
            body: "Kombi".into(),
            topic_id: Some("t1".into()),
            topic_name: Some("Ev".into()),
            entry_id: None,
        };
        assert_eq!(notice_target(&n).unwrap().name, "Ev");
        let mut journal = n.clone();
        journal.topic_id = None;
        assert!(notice_target(&journal).is_none(), "konusuz → Ana Günlük");
        let tap = serde_json::json!({ "actionId": "tap",
            "notification": { "id": 1, "extra": { "topic_id": "t1", "topic_name": "Ev" } } });
        assert_eq!(tap_target(&tap).unwrap().id, "t1");
        let wrapped = serde_json::json!({ "message": tap, "index": 0 });
        assert_eq!(tap_target(&wrapped).unwrap().name, "Ev");
        assert!(tap_target(&serde_json::json!({ "notification": { "extra": {} } })).is_none());
    }

    fn looks_row_pending(pending: u64) -> TopicRowView {
        TopicRowView {
            id: "x".into(),
            name: "x".into(),
            kind: "area".into(),
            kind_label: "Alan".into(),
            due_date: None,
            icon: None,
            color: 1,
            avatar: None,
            pending,
            pinned: false,
            last_label: None,
            preview: None,
        }
    }

    #[test]
    fn task_meta_text() {
        let mut t = TaskRowView {
            id: "1".into(),
            title: "Kombi servisini ara".into(),
            topic_id: None,
            due_date: NaiveDate::from_ymd_opt(2026, 11, 14),
            due_time: chrono::NaiveTime::from_hms_opt(10, 0, 0),
            priority: "normal".into(),
            priority_label: "Orta".into(),
            overdue_days: 0,
            open: true,
            recurring: true,
        };
        assert_eq!(task_meta(&t), "14\u{a0}Kas · 10:00 · tekrarlı");
        t.overdue_days = 1;
        t.due_time = None;
        t.recurring = false;
        assert_eq!(task_meta(&t), "1\u{a0}gün gecikti");
    }
}

#[cfg(test)]
mod tests_v2 {
    use super::*;
    use notai_api::ReminderView;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn task(id: &str, title: &str) -> TaskRowView {
        TaskRowView {
            id: id.into(),
            title: title.into(),
            topic_id: None,
            due_date: Some(d(2026, 11, 14)),
            due_time: NaiveTime::from_hms_opt(10, 0, 0),
            priority: "normal".into(),
            priority_label: "Orta".into(),
            overdue_days: 0,
            open: true,
            recurring: false,
        }
    }

    #[test]
    fn dates_and_weeks() {
        let fri = d(2026, 11, 6);
        assert_eq!(long_date(fri), "Cuma, 6\u{a0}Kasım");
        assert_eq!(month_year(fri), "Kasım 2026");
        assert_eq!(weekday_short(fri), "Cum");
        let mon = week_monday(fri);
        assert_eq!(mon, d(2026, 11, 2));
        assert_eq!(week_monday(mon), mon);
        assert_eq!(week_monday(d(2026, 11, 8)), mon);
        assert_eq!(shift_weeks(mon, 1), d(2026, 11, 9));
        assert_eq!(shift_weeks(mon, -1), d(2026, 10, 26));
        let days = week_days(mon);
        assert_eq!(days.len(), 7);
        assert_eq!(days[6], d(2026, 11, 8));
        assert_eq!(day_in_week(mon, fri), fri);
        assert_eq!(day_in_week(shift_weeks(mon, 1), fri), d(2026, 11, 9));
        assert_eq!(week_range(mon), "2–8\u{a0}Kasım");
        // Ay görünümü
        assert_eq!(month_first(fri), d(2026, 11, 1));
        assert_eq!(shift_months(fri, 1), d(2026, 12, 1));
        assert_eq!(shift_months(fri, -11), d(2025, 12, 1));
        assert_eq!(shift_months(d(2027, 1, 31), -1), d(2026, 12, 1));
        let grid = month_grid(fri);
        assert_eq!(grid.len(), 42);
        assert_eq!(
            grid[0],
            d(2026, 10, 26),
            "1 Kasım pazar: ızgara 26 Ekim pazartesi başlar"
        );
        assert!(grid.contains(&d(2026, 11, 30)));
        assert_eq!(day_in_month(d(2026, 11, 1), fri), fri);
        assert_eq!(day_in_month(d(2026, 12, 1), fri), d(2026, 12, 1));
        assert_eq!(
            week_range(d(2026, 11, 30)),
            "30\u{a0}Kasım\u{a0}– 6\u{a0}Aralık"
        );
    }

    #[test]
    fn timeline_helpers() {
        let mut t = task("1", "Kombi");
        assert_eq!(timeline_time(&t), "10:00");
        t.due_time = None;
        assert_eq!(timeline_time(&t), "Gün boyu");
        let h = |x| NaiveTime::from_hms_opt(x, 0, 0);
        let noon = NaiveTime::from_hms_opt(12, 10, 0).unwrap();
        assert_eq!(nowline_index(&[None, h(9), h(14)], noon), 2);
        assert_eq!(nowline_index(&[h(13)], noon), 0);
        assert_eq!(nowline_index(&[h(9)], noon), 1);
        assert_eq!(nowline_index(&[], noon), 0);
    }

    #[test]
    fn reminders_subline() {
        let item = ReminderView {
            task_id: "1".into(),
            title: "Ara".into(),
            topic_id: None,
            fire_at: chrono::DateTime::UNIX_EPOCH,
            when: "11:30".into(),
            recurring: false,
        };
        let g = |key: &str, n: usize| ReminderGroupView {
            key: key.into(),
            label: key.into(),
            items: vec![item.clone(); n],
        };
        assert_eq!(reminders_line(&[]), "Hatırlatma yok");
        assert_eq!(reminders_line(&[g("today", 3)]), "3 bugün");
        assert_eq!(
            reminders_line(&[g("missed", 1), g("today", 2)]),
            "1 kaçırılan · 2 bugün"
        );
        assert_eq!(reminders_line(&[g("later", 2)]), "Bugün hatırlatma yok");
    }

    #[test]
    fn task_edit_sends_only_changes() {
        let t = task("1", "Kombi servisini ara");
        let mut f = TaskForm::from_task(&t);
        assert_eq!(f.date, "2026-11-14");
        assert_eq!(f.time, "10:00");
        assert!(edit_is_empty(&task_edit(&t, &f)));
        f.title = "  Kombi bakımı ".into();
        f.time = "11:30".into();
        f.repeat = "week".into();
        f.remind = Some(60);
        let r = task_edit(&t, &f);
        assert_eq!(r.title.as_deref(), Some("Kombi bakımı"));
        assert_eq!(r.due_date, None);
        assert_eq!(r.due_time.as_deref(), Some("11:30"));
        assert_eq!(r.repeat.as_deref(), Some("week"));
        assert_eq!(r.remind_before_minutes, Some(60));
        assert_eq!(r.priority, None);
        // Tarih kaldırılınca saat gönderilmez.
        let mut g = TaskForm::from_task(&t);
        g.date = String::new();
        g.time = String::new();
        let r = task_edit(&t, &g);
        assert_eq!(r.due_date.as_deref(), Some(""));
        assert_eq!(r.due_time, None);
        // Boş başlık değişiklik sayılmaz.
        let mut h = TaskForm::from_task(&t);
        h.title = "   ".into();
        assert!(edit_is_empty(&task_edit(&t, &h)));
    }

    #[test]
    fn timed_form_reminder_choice() {
        assert_eq!(timed_remind(true, "0"), Some(0));
        assert_eq!(
            timed_remind(true, "-1"),
            Some(0),
            "hatırlatma formu her zaman kurar"
        );
        assert_eq!(timed_remind(true, "x"), Some(0));
        assert_eq!(timed_remind(false, "15"), Some(15));
        assert_eq!(timed_remind(false, "-1"), None, "etkinlikte Yok");
    }

    #[test]
    fn theme_and_threshold() {
        assert_eq!(Theme::parse("dark"), Theme::Dark);
        assert_eq!(Theme::parse("x"), Theme::System);
        assert_eq!(Theme::System.attr(), None);
        assert_eq!(Theme::Light.attr(), Some("light"));
        assert_eq!(Theme::parse(Theme::Light.as_str()), Theme::Light);
        assert_eq!(threshold_label(0.9), "%90");
        assert_eq!(threshold_from_input("85"), Some(0.85));
        assert_eq!(threshold_from_input("40"), None);
        assert_eq!(threshold_from_input("abc"), None);
    }

    #[test]
    fn attachments_and_media() {
        assert_eq!(attach_entry_text("photo", "ad.jpg"), "Fotoğraf: ad.jpg");
        assert_eq!(attach_entry_text("location", "x"), "Konum");
        assert_eq!(attach_entry_text("contact", "Ayhan Bey"), "Kişi: Ayhan Bey");
        assert_eq!(attach_entry_text("document", ""), "Belge");
        assert_eq!(attachment_icon("photo"), "image");
        assert_eq!(attachment_icon("x"), "file");
        assert_eq!(location_value(41.0082, 28.9784), "41.008200,28.978400");
        assert_eq!(geo_error(1), "Konum izni verilmedi.");
        assert_eq!(mic_error("NotAllowedError"), "Mikrofon izni verilmedi.");
        assert_eq!(mic_error("?"), "Ses kaydı başlatılamadı.");
        assert_eq!(clock(7), "0:07");
        assert_eq!(clock(65), "1:05");
        assert_eq!(size_label(640), "640\u{a0}B");
        assert_eq!(size_label(12 * 1024), "12\u{a0}KB");
        assert_eq!(size_label(1_250_000), "1,2\u{a0}MB");
    }

    #[test]
    fn texts() {
        assert_eq!(answer_eyebrow(true, 2), "Notlarından yanıt · 2\u{a0}kayıt");
        assert_eq!(answer_eyebrow(false, 2), "AI kapalı; ilgili kayıtlar:");
        assert_eq!(first_line("\n  Kostik teklifi \nikinci"), "Kostik teklifi");
        assert_eq!(
            list_text(" süt\n\n yumurta \n"),
            Some("süt\nyumurta".into())
        );
        assert_eq!(list_text("  \n"), None);
        assert_eq!(
            unprocessed_lead(0),
            "Ana Günlük temiz; karar bekleyen kayıt yok."
        );
        assert_eq!(overdue_lead(2), "2\u{a0}görev gecikti.");
        assert_eq!(done_lead(4), "Geçen hafta 4\u{a0}görev tamamlandı.");
        assert_eq!(sync_result(Some((3, 2))), "3 gönderildi · 2 alındı");
        assert_eq!(sync_result(None), "Sunucu ayarlı değil");
        assert_eq!(priority_icon("high"), "prio-high");
    }
}
