# İlerleme ve devir notu (bağlam sıkıştırmasından sonra buradan devam edilir)

> Son güncelleme: 2026-11-07 · Dal: `claude/new-session-cb4ul5` · Proje kökü: `notai/`
> Oturum Gökhan'ın `/goal tüm fazları işle` talimatıyla yürüdü; onay kapılarındaki kararlar
> aşağıda (Gökhan'ın gözden geçirmesi bekleniyor).

## 1. Faz durumu
| Faz | Durum | Çıktı |
|---|---|---|
| 0 · Soru turu | ✅ | `design/brief.md` (+ Revizyon 1: WhatsApp kurgusu) |
| 1 · Tasarım | ✅ | `design/` — 3 yön, `review.md`, `decision.md` (A tabanlı karışım), `tokens.json/.css`, `guidelines.md`, `components.html`, `screens/01–14`, `review-final.md` (ort. 8,1) + engelleyici düzeltmeler |
| 2 · İşlev kararı | ✅ | `docs/FEATURE_PLAN.md` |
| 3 · v0.1 | ✅ | 7 dilim (core, store, ai, server, sync, ui, app) |
| 3 · v0.2–v0.4 | ✅ (Android hariç) | bkz. §3 |
| Android APK | ⛔ beklemede | bkz. §5 |

## 2. Kod düzeni (Cargo workspace, `notai/crates/`)
core · store (SQLite+sqlx, `.sqlx/` çevrimdışı önbellek, migrations 0001+0002) · ai · sync · server (axum) · api (ortak DTO + `convert` özelliği) · tokens (tokens.json→CSS) · ui (Leptos→WASM) · app (Tauri 2).

Çalışma kuralları / araçlar:
- `./scripts/check.sh` = fmt + clippy `-D warnings` + tüm testler ("iş bitti" ölçütü). Arayüz paketi yoksa önce `scripts/build-ui.sh` (trunk) çalıştırır.
- Şema/sorgu değişince `./scripts/prepare-sqlx.sh` (DB'yi `target/sqlx-dev.db`'de kurar, `.sqlx/` üretir).
- Commit yalnız check yeşilse: scratchpad'deki `commit-if-green.sh` kalıbı (`if check.sh; then git commit …`). `| grep` boru hattı çıkış kodunu yutar — kullanma.
- Testlerde yardımcı fonksiyonlarda unwrap için dosya başına `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`.
- Disk: dev profilinde bağımlılık debuginfo kapalı (target ~3,5 GB). Dolarsa `rm -rf target/wasm32-unknown-unknown target/debug/incremental`.
- Ortam: kişisel `master` ortamı (dl.google.com izinli) oluşturuldu ve bu oturum onu kullanıyor. Kurulu: sqlx-cli, trunk, wasm32 hedefi, webkit2gtk (Tauri Linux), Xvfb.

## 3. Yapılanlar (test sayıları yaklaşık; hepsi yeşil)
**v0.1:** yakalama, Ana Günlük/konu akışı (gün ayraçları, en yeni altta), AI çipleri (çip başına kabul/yoksay), işlenmemiş işareti, görevler, sabah planı (≤3), WhatsApp/MacroDroid webhook (yineleme penceresi), senkron (LWW + alan kuralları, aynı adlı konu → " (2)"), çöp/arşiv, Tauri kabuğu, Leptos arayüz.

**v0.2:** hatırlatmalar (gruplar, erteleme 10 dk/1 sa/yarın, bir kez çalma, 24 saat sınırı), sunucuda ntfy bildirimi + kabukta yerel bildirim (tauri-plugin-notification; hatırlatma + 08:00 sabah planı), takvim (hafta), görev ekle/düzenle (tarih/saat/tekrar/önce hatırlat/öncelik/konu), konu ekle/düzenle (sabitle, ad, tür, bitiş), konuya taşı, Ayarlar (tema, ev sunucusu, otonom, seni tanıma, çöp).

**v0.3:** sesli yakalama (arayüzde MediaRecorder şeridi → sunucu `/api/voice` → Whisper uyumlu STT → kayıt + ses eki; kabuk sunucuya iletir), ekleme paneli (fotoğraf/kamera/belge/konum/kişi/liste/görev/hatırlatma), ekler depoda (≤10 MB, senkronlu, base64 günlük yükü).

**v0.4:** Ara/Sor (tam metin + embedding karma arama, cevap yalnız verilen notlara dayanır, uydurma kaynak atılır; AI yoksa ilgili kayıtlar), haftalık gözden geçirme (ilerlemeyen projeler ≥7 gün, yaklaşan 7 gün, geciken, geçen hafta biten), otonom mod (eşik ≥0,5, varsayılan 0,9; geri al = etki tersine + "yoksayıldı", senkron kuralıyla tutarlı), öğrenme sinyali (kullanıcının son 12 kararı sınıflandırma istemine örnek; otonom kararlar hariç).

**Doğrulama:** Rust testleri (core ~70, store ~27, ai ~23, server ~19 HTTP, sync 6 senaryo, app 3, ui 14, tokens 4); gerçek ikili duman testi; Linux'ta gerçek Tauri penceresi (Xvfb); arayüz gerçek sunucuya karşı Playwright ile 19/19 adım (`docs/screenshots/ui-v0.4-screens.png`).

## 4. Onay beklemeden verilen kararlar (gözden geçir)
1. Tasarım: A tabanlı karışım (`design/decision.md`); balon kayıtlar; not detayı bottom sheet.
2. Sabah planı ≤3 madde. 3. Bir kayıt ≤1 konu. 4. AI kendiliğinden uygulamaz (otonom mod kapalı varsayılan).
5. Senkron: LWW; kararlı öneri bekliyor'a dönmez; geri alma = yoksayıldı.
6. Öncelik adları Düşük/Orta/Yüksek. 7. Claude varsayılan modeli `claude-opus-5-5` (yalnız `NOTAI_AI=anthropic`).
8. Takvim ucu tamamlanan görevleri de döndürür (üstü çizili gösterilir).

## 5. Sıradaki adımlar
1. **Android APK** — otomatik güvenlik denetimi Android SDK/NDK indirip çalıştırmayı ("dışarıdan kod") engelledi; Gökhan'a soruldu, cevap: önce bağlam sıkıştırması. Sıkıştırmadan sonra Gökhan'a tekrar sor: izin verirse `scripts/android-setup.sh` (cmdline-tools 11076708 → platform-tools, platforms;android-34, build-tools;34.0.0, ndk;27.2.12479018; rust android hedefleri; `cargo install tauri-cli --version "^2"`) → `cd crates/app && cargo tauri android init && cargo tauri android build --apk`. Kabukta `#[cfg_attr(mobile, tauri::mobile_entry_point)]` hazır.
2. **CORS**: tarayıcıdan sunucuya doğrudan bağlanmak (`?api=http://sunucu:8787`) için sunucuya tower-http CorsLayer gerekebilir (şimdilik trunk proxy ile test edildi).
3. Küçük eksikler: takvimde Ay görünümü yok; ekleme panelinde "Etkinlik" yok; `create_task` kimlik döndürmüyor (arayüz önce/sonra farkıyla buluyor); "tam zamanında" hatırlatma yok (en az 5 dk önce); demo'da ek yükleme hatası eksiz kayıt bırakır; otonom kararın geri alınması arayüzde gerçek AI ile sınanmadı (sunucu testinde sınandı); Manrope uygulamaya gömülmedi.
4. Gökhan'a açık sorular: varsayılan AI (Gemma mı Claude mu), sabah 08:00 uygun mu, WhatsApp için MacroDroid mi n8n mi.
5. Bittiğinde: `docs/FEATURE_PLAN.md` §5 durum tablosunu güncelle; kullanıcıya özet.
