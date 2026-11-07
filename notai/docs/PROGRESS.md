# İlerleme ve devir notu

> Son güncelleme: 2026-11-07. Bu oturum Gökhan'ın `/goal tüm fazları işle` talimatıyla tüm fazları
> **onay beklemeden** yürüttü. CLAUDE.md'deki onay kapılarında verilen kararlar aşağıda;
> Gökhan'ın gözden geçirmesi bekleniyor.

## Faz durumu

| Faz | Durum | Çıktı |
|---|---|---|
| 0 · Soru turu | ✅ | `design/brief.md` (4 tur + netleştirme + **Revizyon 1: WhatsApp kurgusu**) |
| 1 · Tasarım | ✅ | 3 yön (`design/directions/`), puanlama (`review.md`), seçim (`decision.md`, A tabanlı karışım), `tokens.json/.css`, `guidelines.md`, `components.html`, 14 ekran (`screens/`), son denetim (`review-final.md`, ort. 8,1) ve engelleyici düzeltmeler |
| 2 · İşlev kararı | ✅ | `docs/FEATURE_PLAN.md` (v0.1–v0.4 sürümleri, 7 teknik dilim, işlev kuralları) |
| 3 · Geliştirme | ✅ v0.1 çekirdeği | 9 crate, **131 test**, `scripts/check.sh` yeşil (fmt + clippy `-D warnings` + test) |

## Faz 3 dilimleri

| # | Dilim | Doğrulama |
|---|---|---|
| 0 | Workspace, lint'ler (unsafe yasak, üretimde unwrap/expect/panic yasak) | `check.sh` |
| 1 | `core` alan modeli | 60 birim testi |
| 2 | `store` SQLite + sqlx (derleme zamanı denetimli), değişiklik günlüğü, çöp kutusu | 19 entegrasyon + 2 birim |
| 3 | `ai` Claude API + yerel Gemma, katı şema doğrulama | 12 birim + 5 sahte HTTP testi |
| 4 | `server` REST + WhatsApp webhook + AI işçisi + sabah planı (ntfy) | 10 HTTP testi + gerçek ikiliyle duman testi |
| 5 | `sync` üç cihaz senkronu | 6 senaryo (çevrimdışı çakışma, karar koruma, aynı adlı konu, 450 kayıt) + HTTP üzerinden uçtan uca |
| 6 | `tokens`, `api`, `ui` (Leptos) | Üretici depodaki CSS ile bayt bayt aynı; arayüz tarayıcıda Playwright ile yaz-gönder / çip kabul / görev tamamla, konsol hatasız |
| 7 | `app` Tauri 2 kabuğu | Komut mantığı testleri; Linux'ta gerçek pencere açılıp yerel depoya bağlandı (ekran görüntüsü `docs/screenshots/`) |

## Onay beklemeden verilen kararlar (gözden geçir)
1. **Tasarım yönü:** A (Taş Defter) tabanlı karışım — `design/decision.md`. Kayıtlar balon; not detayı alttan kayan sayfa.
2. **Sabah planı en fazla 3 madde** (ferah yoğunluk); fazlası "+N görev daha".
3. **Bir kayıt en fazla bir konuya** bağlı; etiket ve kişi çoklu.
4. **AI hiçbir şeyi kendiliğinden uygulamaz** (otonom mod v0.4). AI'ın önerecek bir şeyi yoksa kayıt "işlendi" sayılır.
5. **Senkron:** son yazan kazanır; karar verilmiş öneri asla "bekliyor"a dönmez; aynı adla iki cihazda açılan konular silinmez, sonradan açılan " (2)" eki alır.
6. **Öncelik adları:** Düşük / Orta / Yüksek.
7. **Varsayılan Claude modeli:** `claude-opus-5-5` (yalnız `NOTAI_AI=anthropic` seçilirse; varsayılan AI kapalı).

## Açık sorular (FEATURE_PLAN §4)
1. Varsayılan AI: ev sunucusundaki Gemma mı, Claude API mi?
2. Sabah planı saati 08:00 uygun mu?
3. WhatsApp yakalama: MacroDroid mi, n8n mi?

## Bilinen eksikler / sonraki adımlar
- **Android derlemesi bu ortamda denenmedi** (Android SDK/NDK yok). Kod mobil giriş noktasıyla hazır; README'deki adımlar gerçek cihazda doğrulanmalı.
- v0.1'de arayüzde olmayanlar: Ayarlar ekranı (senkron adresi şimdilik `set_sync` komutuyla), konu oluşturma/düzenleme ekranı, "konuya taşı" seçicisi, kayıt düzenleme.
- v0.2: Takvim, Hatırlatmalar + Android bildirimleri (sunucu tarafı sabah planı hazır), görev düzenleme.
- v0.3: Sesli yakalama (STT), ekleme paneli (fotoğraf/belge/konum/kişi).
- v0.4: Ara/Sor (FTS5 arama depoda hazır; anlamsal arama + LLM cevabı yok), haftalık gözden geçirme, otonom mod, ProfileFact öğrenme.
- Manrope yazı tipi şimdilik Google Fonts'tan; çevrimdışı için uygulamaya gömülmeli (yoksa sistem yazı tipine düşer).
- Saat kayması: senkron cihaz saatine güvenir; saatleri çok kaymış cihazlarda "son yazan" yanlış seçilebilir.
