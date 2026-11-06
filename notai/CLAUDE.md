# NotAI — Proje Kuralları (Claude Code bu dosyayı her oturumda okur)

## Ürün tek cümlede
Gün içinde hiç düşünmeden not/iş/fikir yakalanan, yapay zekânın bunları kullanıcıyı tanıyarak
kendiliğinden sınıflandırdığı, zamanı gelince hatırlattığı ve yapılacakları takip ettiği,
minimalist ama güçlü bir kişisel not + görev sistemi. İş ve özel hayat tek sistemde.

Referans alınan üç uygulama (yalnızca UX kalıpları; logo, marka, ikon seti, birebir ekran KOPYALANMAZ):
- **Saner.AI** → AI gelen kutusu, otomatik etiket önerisi, sabah öncelik planı, "beyin boşaltma → görev"
- **Todoist + Ramble** → konuşarak görev yakalama, canlı düzeltme ("onu perşembe yap"), güçlü görev/hatırlatma
- **Google Keep + Gemini** → saniyede not, renkli kartlar, etiketler, notlara sohbetle soru sorma

Kullanıcının yöntemi: **Bullet Journal günlük log + PARA (Projeler / Alanlar / Kaynaklar / Arşiv) + haftalık gözden geçirme.**

## Kullanıcı hakkında
- Gökhan, endüstri mühendisi, profesyonel yazılımcı değil. Türkçe konuş, teknik kararları kısa gerekçeyle açıkla.
- Ana cihaz Android (Samsung). İkinci bir S24 Ultra 7/24 ev sunucusu (n8n, yerel Gemma modelleri).
- Hızlı yakalama alışkanlığı: WhatsApp'a mesaj atmak.

## ÇALIŞMA SIRASI — DEĞİŞMEZ
1. **Faz 0 – Soru turu.** Kod yazmadan önce `AskUserQuestion` ile tasarım sorularını sor (bkz. docs/DESIGN_PROCESS.md).
2. **Faz 1 – Tasarım.** Tasarımcı ajanlar 3 farklı yön çıkarır → kullanıcı seçer → bileşenler → ekranlar.
   Kullanıcı "tasarım onaylandı" demeden Faz 2'ye GEÇME.
3. **Faz 2 – İşlev kararı.** Onaylı ekranlar üzerinden hangi özelliklerin hangi sırayla yapılacağı birlikte kararlaştırılır.
4. **Faz 3 – Geliştirme.** docs/ARCHITECTURE.md'ye göre, dilim dilim (her dilim çalışır + test edilmiş).

## Dil kuralı: Rust kullanılabilecek her yerde Rust
- Uygulama kabuğu: **Tauri 2** (Rust) → Android + Windows/Linux masaüstü.
- Arayüz: **Leptos** (Rust → WASM). İş mantığı JavaScript/TypeScript'te YAZILMAZ.
- Sunucu: **axum** (Rust). Veritabanı: **SQLite** (`sqlx`, derleme zamanı kontrollü sorgular).
- İstisnalar (yalnız bunlar): CSS, Faz 1'deki HTML tasarım prototipleri, Tauri'nin zorunlu kıldığı Android/Kotlin yapıştırıcı kodu.

## "Hata çıkarmayan" kod standartları
- `#![forbid(unsafe_code)]` her crate'te.
- Üretim kodunda `unwrap()`, `expect()`, `panic!` YOK → `thiserror` ile tipli hatalar, `?` ile yayılım.
- `cargo clippy --all-targets -- -D warnings` ve `cargo fmt --check` temiz olmadan iş bitmiş sayılmaz.
- Domain tipleri newtype (ör. `NoteId(Uuid)`), geçersiz durum tiple temsil edilemez olsun.
- Her özellik: birim testi + en az bir entegrasyon testi. AI çıktısı her zaman şema ile doğrulanır (serde + doğrulama), asla körü körüne yazılmaz.
- Veri kaybı sıfır tolerans: her yazma işlemi transaction içinde, silme = önce Arşiv/çöp kutusu.

## Tasarım ilkeleri
- Minimalist ama işlevsel: ekranda yalnız o anki iş. Her ekranın tek ana eylemi.
- Yakalama 1 dokunuş / 2 saniye: açılışta doğrudan yazma alanı veya mikrofon.
- Tüm renk, boşluk, köşe yarıçapı, gölge, animasyon süresi **design tokens** dosyasından gelir (`design/tokens.json`). Koda sabit renk yazılmaz.
- Açık + koyu tema baştan. WCAG AA kontrast. Türkçe karakterler (ğ, ş, ı, İ) tüm fontlarda test edilir.
