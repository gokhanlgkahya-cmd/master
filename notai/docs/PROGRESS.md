# İlerleme ve devir notu (bağlam sıkıştırmasından sonra buradan devam edilir)

> Son güncelleme: 2026-11-07 (küçük eksikler tamamlandı) · Dal: `claude/new-session-cb4ul5` · Proje kökü: `notai/`
> Oturum Gökhan'ın `/goal tüm fazları işle` talimatıyla yürüdü. **Gökhan 2026-11-07'de Faz 1–2
> kararlarını (§4) onayladı.**

## 1. Faz durumu
| Faz | Durum | Çıktı |
|---|---|---|
| 0 · Soru turu | ✅ | `design/brief.md` (+ Revizyon 1: WhatsApp kurgusu) |
| 1 · Tasarım | ✅ | `design/` — 3 yön, `review.md`, `decision.md` (A tabanlı karışım), `tokens.json/.css`, `guidelines.md`, `components.html`, `screens/01–14`, `review-final.md` (ort. 8,1) + engelleyici düzeltmeler |
| 2 · İşlev kararı | ✅ | `docs/FEATURE_PLAN.md` |
| 3 · v0.1 | ✅ | 7 dilim (core, store, ai, server, sync, ui, app) |
| 3 · v0.2–v0.4 | ✅ (Android hariç) | bkz. §3 |
| Android APK | 👤 Gökhan derleyecek | Kod hazır (mobil giriş noktası); bulut oturumunda SDK/NDK kurulumuna izin verilmedi, Gökhan kendi bilgisayarında derleyecek: `docs/ANDROID.md` |

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

**v0.2:** hatırlatmalar (gruplar, erteleme 10 dk/1 sa/yarın, bir kez çalma, 24 saat sınırı), sunucuda ntfy bildirimi + kabukta yerel bildirim (tauri-plugin-notification; hatırlatma + 09:00 sabah planı), takvim (hafta), görev ekle/düzenle (tarih/saat/tekrar/önce hatırlat/öncelik/konu), konu ekle/düzenle (sabitle, ad, tür, bitiş), konuya taşı, Ayarlar (tema, ev sunucusu, otonom, seni tanıma, çöp).

**v0.3:** sesli yakalama (arayüzde MediaRecorder şeridi → sunucu `/api/voice` → Whisper uyumlu STT → kayıt + ses eki; kabuk sunucuya iletir), ekleme paneli (fotoğraf/kamera/belge/konum/kişi/liste/görev/hatırlatma), ekler depoda (≤10 MB, senkronlu, base64 günlük yükü).

**v0.4:** Ara/Sor (tam metin + embedding karma arama, cevap yalnız verilen notlara dayanır, uydurma kaynak atılır; AI yoksa ilgili kayıtlar), haftalık gözden geçirme (ilerlemeyen projeler ≥7 gün, yaklaşan 7 gün, geciken, geçen hafta biten), otonom mod (eşik ≥0,5, varsayılan 0,9; geri al = etki tersine + "yoksayıldı", senkron kuralıyla tutarlı), öğrenme sinyali (kullanıcının son 12 kararı sınıflandırma istemine örnek; otonom kararlar hariç).

**Küçük eksikler (2026-11-07):** sunucuda isteğe bağlı CORS (`NOTAI_CORS_ORIGINS`; varsayılan kapalı, belirteç yine zorunlu); takvimde Hafta/Ay anahtarı (6 haftalık ay ızgarası, ay ileri/geri, bugüne dön); ekleme panelinde **Etkinlik** (tarih + saatli görev olarak saklanır, takvimde görünür, varsayılan 15 dk önce hatırlatır); "tam zamanında" hatırlatma (`remind_before_minutes`: 0 = tam zamanında, negatif = kaldır; Hatırlatma formunun varsayılanı); `create_task` yeni görevin kimliğini döndürür (kabuk + sunucu); önizlemede (demo) ek denemesi artık kayıt açmadan uyarır, gerçek bağlantıda ek yüklenemezse yalnız ek için açılmış kayıt çöpe gider (yazılmış metin korunur); Manrope uygulamaya gömülü (Google Fonts kaldırıldı, CSP daraltıldı). Tarayıcı testi (gerçek sunucu, ayrı köken, CORS ile): 7/7; otonom karar dış ajan (`notai answer`) ile uygulanıp arayüzden geri alındı: 4/4.

**Revizyon 2 — konu simgeleri (2026-11-07, Gökhan):** simge konu adından + son 20 kayıttan Türkçe anahtar kelimelerle seçilir (`crates/core/src/look.rs`; elle seçilen sabit; addan eşleşme yoksa ≥2 kayıt gerekir; yoksa türün varsayılanı), her konu oluşturulma sırasıyla 8 renkli paletten kalıcı renk alır (`tokens.json` `topic-1…8`, AA). 19 yeni simge (`design/screens/15-konu-simgeleri.html`). API `TopicRowView.color`; ana ekran, akış başlığı, kayıt üstü konu adı ve konu seçme listesi renkli. Testler: core (simge/renk kuralları + sprite'ta her simge var), store (kayıt geldikçe simge değişir), sunucu (`/api/topics` simge + renk).

**Revizyon 3 — WhatsApp yapısı (2026-11-07, Gökhan):** yuvarlak profil (renkli daire + simge ya da profil fotoğrafı; konu menüsünde "Profil fotoğrafı", "Fotoğrafı kaldır", "Simgeyi değiştir"), ana ekranda arama çubuğu + hap filtreler (Tümü · İşlenmemiş · Proje · Alan · Kaynak), tek liste (Ana Günlük + Haftalık iğneli), konu başına karar bekleyen kayıt rozeti, sağ altta "Yeni kayıt" düğmesi (ana ekranda yazma çubuğu yok), akışta WhatsApp yazma çubuğu. Şema: `0003_topic_avatar.sql` (topics.avatar, ≤120 000 karakter `data:image/…`). Tarayıcı testi (gerçek sunucu): 9/9.

**Bildirimler + AI sorusu (2026-11-07, Gökhan):** bildirim başlığında konu adı ("Hatırlatma · Ev"); AI'ın görev çipine varsayılan hatırlatma (saatliyse 15 dk önce, saatsizse o gün 09:00; geçmişteyse kurulmaz); bildirim konu/kayıt bilgisini taşır — Android'de dokununca o konu açılır (eklentinin `actionPerformed` olayı; cihazda sınanmadı), ayrıca ana ekranda "son bildirim" şeridi (2 saat; dokununca konu açılır). **AI sorusu:** kayıt net değilse AI `question` ile tek kısa soru sorar (yeni öneri türü `question`, migration 0004); Ana Günlük'te NotAI balonu + cevap alanı + "Geç"; cevap senkronla sunucuya gider, kayıt cevapla yeniden sınıflandırılır; cevaplar sonraki istemlere öğrenme örneği olur; kayıt başına en fazla 2 soru; otonom mod soruları uygulamaz; sorular ntfy + uygulama bildirimiyle gelir. Testler: core, ai, store, sync (telefonda cevap → sunucuda yeniden kuyruk), sunucu uçtan uca, uygulama bildirim metni; tarayıcı 7/7.

**Revizyon 4 — konu hiyerarşisi (2026-11-07, Gökhan):** `topics.parent_id` (migration 0005; ad benzersizliği üst konu içinde), çekirdekte `topic::Tree` (alt dal, üstler, döngü/derinlik ≤5 denetimi, yol adı), akış alt dalı kapsar (özyinelemeli sorgu; kayıt tek, mükerrer yok), liste alt dalı toplar (rozet, son kayıt, alt konu sayısı, yol adı); AI istemi yol adlarıyla, konuya yazılan kayıt yalnız o dalın alt konularına dağıtılır (`parent_id` ile yeni alt konu); arayüzde konu açılışı (hub), "Alt konu ekle", "Üst konuyu değiştir", alt konudan gelen kayıtta konu işareti. Testler: core, ai, store, sync; tarayıcı 8/8.

**Obsidian canlı klasörü (2026-11-07):** `NOTAI_VAULT_DIR` → sunucu her değişiklikte (30 sn'lik turda, değişiklik günlüğü ilerlediyse) `NotAI/` kasasını yazar: `Konular/<dal>/<Konu>.md` klasör notları (ön bilgi, üst bağlantı, alt konular, kendi kayıtları `^n…` blok kimliğiyle, görevler Tasks biçiminde), `Ana Günlük/YYYY-AA-GG.md` (konulu kayıtlar bağlantı + önizleme, konusuzlar burada), `Görevler.md`, `Profil.md`. Yalnız manifest'teki NotAI dosyaları güncellenir/silinir; içerik aynıysa yazılmaz; geçici dosya + taşıma ile yarım dosya kalmaz. `POST /api/vault/export` anında yazar. Saf üretici `core::vault` (test), sunucu testi (yazma, kendi notun korunması, yeniden adlandırma).

**Harcamalar + banka ekstresi (2026-11-07):** `core::finance` (kuruş tamsayı para, iş/ev/kişisel kapsam, 14 kategori, ekstre satırı ayrıştırıcı — PDF'teki kopuk harflere dayanıklı satıcı anahtarı, yerleşik satıcı kuralları + kullanıcının öğrettiği kurallar, mükerrer anahtarı, elle giriş ↔ ekstre eşleştirme, aylık rapor: gelir/gider/birikim, kapsam/kategori, düzenli ödemeler, öneriler — %20 birikim hedefi, kategori artışı, abonelikler, iş giderleri ayrı). Depo: `transactions`, `merchant_rules`, `statements` (migration 0006; senkronlanır), 0007 öneri türü CHECK kaldırıldı. AI: kayıttan `expense` önerisi (çip "Harcama · 1.850,00 ₺ · İş", kabulde hareket; geri alınabilir). PDF metni yerelde (`pdf-extract`) çıkarılır; biçim tanınmazsa ajan işi (`notai statements` / `statement-answer`). Uçlar: `/api/finance`, `/api/finance/statements`, `/api/finance/transactions/{id}/classify`, `/api/agent/statements…`; kabukta aynı komutlar. Arayüz: ana ekranda "Harcamalar" iğneli satırı → ay seçimi, kutucuklar, yorumlar, dağılım, kategoriler, düzenli ödemeler, hareketler (kategori/kapsam düzeltilir, öğrenilir), "Ekstre yükle (PDF)". Testler: core 5, store, sunucu (gerçek PDF), tarayıcı 5/5. **Açık karar:** banka verisini hangi AI işleyecek (şimdilik yerel kurallar + gerekirse ajan).

**Doğrulama:** Rust testleri (core ~70, store ~27, ai ~23, server ~19 HTTP, sync 6 senaryo, app 3, ui 14, tokens 4); gerçek ikili duman testi; Linux'ta gerçek Tauri penceresi (Xvfb); arayüz gerçek sunucuya karşı Playwright ile 19/19 adım (`docs/screenshots/ui-v0.4-screens.png`).

## 4. Kararlar (Gökhan onayladı, 2026-11-07)
1. Tasarım: A tabanlı karışım (`design/decision.md`); balon kayıtlar; not detayı bottom sheet.
2. Sabah planı ≤3 madde. 3. Bir kayıt ≤1 konu. 4. AI kendiliğinden uygulamaz (otonom mod kapalı varsayılan).
5. Senkron: LWW; kararlı öneri bekliyor'a dönmez; geri alma = yoksayıldı.
6. Öncelik adları Düşük/Orta/Yüksek. 7. **AI kurgusu (Gökhan):** S24'teki ajanlar (Hermes / Claude Code) `NOTAI_AI=external` + `notai` CLI ile sınıflandırır (`docs/AGENT.md`); dağıtım için AI kurgusu sonra kararlaştırılacak. Diğer yollar (yerel Gemma, Claude API `claude-opus-5-5`) kodda hazır.
9. **Sabah planı 09:00** (Gökhan).
10. **WhatsApp yakalama:** Gökhan ne yakalanacağından emin değil; webhook kodda hazır ama kapalı (gizli değer verilmedikçe çalışmaz). Karar ileride.
8. Takvim ucu tamamlanan görevleri de döndürür (üstü çizili gösterilir).

## 5. Sıradaki adımlar
1. **Android APK** — Gökhan kendi bilgisayarında derleyecek (karar: 2026-11-07). Adımlar `docs/ANDROID.md`. Derlemede hata çıkarsa çıktısı ile düzeltilecek.
2. ~~CORS~~, ~~ay görünümü~~, ~~Etkinlik~~, ~~create_task kimliği~~, ~~tam zamanında hatırlatma~~, ~~demo ek hatası~~, ~~otonom geri alma arayüz testi~~, ~~Manrope~~ → tamamlandı (yukarıda §3).
3. Etkinlik ayrı bir tür değil, saatli görevdir (Görevler listesinde de görünür). Ayrı tür (süre, bitiş saati) istenirse ileride.
4. Açık konu: WhatsApp'tan ne yakalanacağı (kendi sohbetine yazdığı notlar mı, belirli kişilerden gelenler mi) Gökhan'la netleştirilecek.
5. Bittiğinde: `docs/FEATURE_PLAN.md` §5 durum tablosunu güncelle; kullanıcıya özet.
