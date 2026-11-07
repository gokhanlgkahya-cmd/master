# Faz 2 — İşlev Kararı

> Tarih: 2026-11-06 · Hazırlayan: Claude (Gökhan'ın `/goal tüm fazları işle` talimatıyla onay beklemeden).
> Dayanak: design/brief.md, design/decision.md, design/screens/, docs/ARCHITECTURE.md.
> Gökhan bu sırayı değiştirebilir; her dilim bağımsız test edildiği için yeniden sıralamak ucuzdur.

## 1. Ürün sürümleri (kullanıcının göreceği sıra)

| Sürüm | Kullanıcı ne yapabilir? | Ekranlar |
|---|---|---|
| **v0.1 — Yakala ve akıt** | Ana Günlük'e yazıyla kayıt; konu (Proje/Alan/Kaynak/Arşiv) açma; konuya doğrudan yazma (Ana Günlük'te de görünür); AI'ın kayıt altına çip önermesi, çip başına Kabul/Yoksay; işlenmemiş işareti; kaydı göreve çevirme (tarih, öncelik); not detayı sheet'i; açık/koyu/sistem tema; silme = Arşiv/Çöp | Ana ekran, Ana Günlük akışı, Konu akışı, Not detayı |
| **v0.2 — Hatırlat ve planla** | Görevler listesi (Bugün / Yaklaşan / Geciken), Hatırlatmalar + Android bildirimi, sabah planı (bildirim + akış kartı), Takvim (gün/hafta), WhatsApp → MacroDroid → webhook ile yakalama | Görevler, Hatırlatmalar, Takvim, sabah planı kartı |
| **v0.3 — Konuş ve ekle** | Sesli yakalama (canlı metin + "aslında perşembe yap" düzeltmesi), ekleme paneli (fotoğraf, kamera, belge, konum, kişi, liste, etkinlik), çok cihaz senkronu (telefon ↔ masaüstü ↔ ev sunucusu) | Ses şeridi, Ekleme paneli |
| **v0.4 — Sor ve gözden geçir** | Ara/Sor (FTS5 + anlamsal arama + LLM cevabı, kaynak kayıtlara bağlantı), Haftalık gözden geçirme, "otonom mod" (yüksek güvenli önerileri otomatik uygula, geri alınabilir), ProfileFact öğrenme ekranı | Ara/Sor, Haftalık gözden geçirme, Ayarlar |

**Bilinçli olarak sonraya bırakılanlar:** paylaşım/çok kullanıcı, iOS, uçtan uca şifreleme (Tailscale + ev ağı şimdilik yeterli), widget.

## 2. Teknik dilimler (Faz 3 — geliştirme sırası)
Her dilim: derlenir + `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings` + birim testleri + en az bir entegrasyon testi. Bir dilim bitmeden sonrakine geçilmez.

| # | Dilim | Crate | Kapsam | Bitti sayılması için |
|---|---|---|---|---|
| 0 | İskelet | workspace | Cargo workspace, ortak lint'ler (`unsafe` yasak; `unwrap/expect/panic` üretim kodunda clippy hatası), rustfmt, `scripts/check.sh` | `check.sh` yeşil |
| 1 | Alan modeli | `core` | Newtype kimlikler; Entry + kaynak + durum makinesi (gelen_kutusu → işlendi → arşiv → çöp); Topic + PARA türü; Task (tarih, öncelik, tekrar, hatırlatma); Suggestion (tür, güven skoru, karar); ProfileFact; günlük zaman çizelgesi kuralları (gün ayraçları, en yeni en altta, konu filtresi, işlenmemiş sayısı); öneri uygulama kuralları; sabah planı seçimi | I/O yok, kapsamlı birim testleri |
| 2 | Kalıcı depo | `store` | SQLite + sqlx (derleme zamanı kontrollü sorgular, `.sqlx` çevrimdışı önbellek), migration'lar, depo fonksiyonları, her yazma transaction içinde, değişiklik günlüğü (sync için), silme = arşiv/çöp | Geçici SQLite ile entegrasyon testleri |
| 3 | AI hattı | `ai` | Sağlayıcıdan bağımsız `LlmClient` trait'i; sınıflandırma istemi (ProfileFact + son kayıt bağlamı); **katı şema doğrulama** (bilinmeyen alan reddi + alan kuralları) → `Suggestion`; sağlayıcılar: Anthropic Messages API, OpenAI-uyumlu yerel uç (Ollama/Gemma), test için sahte istemci | Sahte HTTP sunucusuyla entegrasyon testi; bozuk JSON asla yazılmaz |
| 4 | Sunucu | `server` | axum REST (kayıt ekle/listele, Ana Günlük, konular, konu akışı, öneri kabul/yoksay, görevler, sabah planı), WhatsApp/MacroDroid webhook'u (paylaşılan gizli anahtarla), AI iş kuyruğu işçisi, zamanlayıcı (sabah planı) | HTTP entegrasyon testleri |
| 5 | Senkron | `sync` | Değişiklik günlüğü tabanlı push/pull protokolü; son-yazan-kazanır + alan bazlı birleştirme; sunucuda `/sync` uçları | İki istemci + sunucu senaryo testi |
| 6 | Arayüz | `ui` | Leptos bileşenleri: sekme çubuğu, kayıt balonu, AI çipi, gün ayracı, yazma çubuğu, sheet, konu satırı; ekranlar: Ana ekran, Ana Günlük akışı, Not detayı; CSS değişkenleri `design/tokens.json`'dan build betiğiyle üretilir | `wasm32` derlemesi + görünüm-model testleri |
| 7 | Uygulama kabuğu | `app` | Tauri 2 kabuğu; yerel SQLite (`store`) ile Tauri komutları; masaüstü (Linux/Windows) derlemesi; Android derleme talimatı | Masaüstü derlemesi (`cargo check`), komut testleri |

## 3. Kararlaştırılan işlev ayrıntıları

### 3.1 Yakalama
- Yakalama hiçbir zaman AI'ı beklemez: kayıt önce `gelen_kutusu` durumunda kaydedilir, AI işi kuyruğa atılır.
- Boş veya yalnız boşluk içeren kayıt reddedilir; 20 000 karakter üst sınır.
- Kaynak: `uygulama`, `ses`, `whatsapp`, `paylaşım`. WhatsApp webhook'u aynı metni 5 dk içinde ikinci kez alırsa yineleme sayılır (tekil anahtar).

### 3.2 Konular (PARA)
- Konu türleri: Proje (isteğe bağlı bitiş tarihi), Alan, Kaynak, Arşiv. Ana Günlük bir konu değil, tüm kayıtların görünümüdür.
- Bir kayıt en fazla **bir** konuya bağlıdır (basitlik; etiketler çoklu olabilir).
- Konuya doğrudan yazılan kayıt `işlendi` sayılır (kullanıcı zaten sınıflandırdı); AI yine görev/tarih önerebilir.

### 3.3 AI önerileri
- Öneri türleri: `konu`, `görev` (başlık, tarih, öncelik), `etiket`, `kişi`, `hatırlatma`.
- Her öneri ayrı kabul/yoksay edilir (critic'in "toplu ret" bulgusu).
- Bir kaydın tüm önerileri karara bağlanınca kayıt `işlendi` olur.
- Güven skoru 0–1; "otonom mod" v0.4'te. O zamana kadar hiçbir öneri kendiliğinden uygulanmaz.
- AI çıktısı şemaya uymazsa: öneri yazılmaz, iş 3 kez üstel beklemeyle yeniden denenir, sonra "AI işleyemedi" olarak işaretlenir (kayıt kaybolmaz).

### 3.4 Görevler ve sabah planı
- Öncelik: Düşük / Normal / Yüksek. Tarih isteğe bağlı; saat isteğe bağlı.
- Sabah planı (varsayılan 08:00, Europe/Istanbul): geciken görevler → bugünkü görevler → yüksek öncelikli tarihsizler; **en fazla 3 madde** (ferah yoğunluk; design-critic son tur kararı ile eşitlendi). Kalanlar "+N görev daha" bağlantısıyla Görevler sekmesine gider.

### 3.5 Silme ve veri güvenliği
- "Sil" = Çöp kutusu (30 gün), "Arşivle" = Arşiv. Kalıcı silme yalnız çöp kutusundan, ayrı onayla.
- Her yazma transaction içinde; her değişiklik değişiklik günlüğüne yazılır.

## 4. Gökhan'a açık sorular (sonraki oturumda)
1. AI sağlayıcısı varsayılanı: Claude API mi, ev sunucusundaki Gemma mı? (Kod ikisini de destekliyor; varsayılan: yerel Gemma, yedek Claude.)
2. Sabah planı saati 08:00 uygun mu?
3. WhatsApp yakalaması için MacroDroid akışı mı, n8n üzerinden mi?

## 5. Faz durumu
| Faz | Durum | Not |
|---|---|---|
| Faz 0 | ✅ | design/brief.md (+ Revizyon 1: WhatsApp kurgusu) |
| Faz 1 | ✅ (otonom onay) | design/decision.md (A tabanlı karışım), tokens.json/css, guidelines.md, components.html, screens/01–14, review-final.md (ort. 8,1). "Tasarım onaylandı" kararı `/goal tüm fazları işle` talimatıyla Claude tarafından verildi; Gökhan'ın sonradan gözden geçirmesi beklenir. |
| Faz 2 | ✅ (otonom onay) | Bu belge. Açık sorular §4'te. |
| Faz 3 | ✅ v0.1–v0.4 (Android APK hariç) | Ayrıntı ve kalanlar `docs/PROGRESS.md`'de. |
