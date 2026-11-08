# DEVAM — PC'de buradan başla (son güncelleme: 2026-11-08)

> Bulut oturumundan PC'ye geçiş belgesi. Claude Code: bu dosyayı oku, sonra aşağıdaki sırayla ilerle.
> Bu oturumun tüm çıktıları git'te, **`claude/new-session-cb4ul5`** dalında. Konuşmanın kendisi taşınmaz;
> önemli olan her şey aşağıdaki dosyalarda.

## 1. Kodu al
```bash
git clone https://github.com/gokhanlgkahya-cmd/master.git
cd master
git checkout claude/new-session-cb4ul5
cd notai
```

## 2. Hangi belge ne işe yarar (okuma sırası)
| Belge | İçerik |
|---|---|
| `CLAUDE.md` | Değişmez proje kuralları (Rust, hata çıkarmayan kod, tasarım ilkeleri) |
| `docs/VIZYON.md` | Gökhan'ın amacı (kısa) — kararlarda pusula |
| `docs/PROGRESS.md` | Bugüne kadar yapılanlar (özellik özellik) |
| `docs/DEGERLENDIRME.md` | Genel durum, dış bağlantılar, raporlar, rakipler, WhatsApp kıyası, yeni mimari tanımlar |
| `docs/TALEPLER.md` | **Geliştirme talepleri** T-01…T-17 ve Gökhan'ın notları N-1…N-6 |
| `docs/TASARIM_NOTLARI.md` | Tasarım soruları, Hermes/ajan, ücretsiz bulut, dürüst değerlendirme |
| `docs/ACIK_SORULAR.md` | **49 açık soru** (önerilerle). Gökhan PC'de cevaplayacak: `> Cevap:` satırı |
| `docs/ANDROID.md` | APK'yı PC'de derleme adımları |
| `docs/SES.md` | S24'e Whisper kurulumu, ses → yazı akışı |
| `docs/AGENT.md` | Ajanın (Hermes / Claude Code) `notai` CLI ile çalışması |
| `docs/rehber/` | 26 adımlık kullanım rehberi (ekran görüntülü HTML) |
| `docs/raporlar/` | Raporların yerleşimi: bugünkü ekranlar + öneri taslakları |
| `docs/kod-haritasi/` | Hangi parça hangi dilde (grafik) + güvenlik katmanları ve eksikler (T-18) |
| `docs/DIS_MODEL_BRIFI.md` | Başka bir AI modeline tasarımı tanıtma brifi |
| `scripts/e2e/` | Tarayıcı denemeleri (Playwright) ve örnek veri betikleri |
| `scripts/commit-if-green.sh` | `check.sh` yeşilse commit + push |

## 3. Sıradaki iş (karar: yeni özellik eklemeyi durdur, önce gerçek kullanım)
1. **Açık soruları cevapla** (`docs/ACIK_SORULAR.md`), en azından A (altyapı) ve C (konular) bölümleri.
2. **İlk APK:** `docs/ANDROID.md` → derle → telefona kur. Beklenen riskler:
   - `crates/speech/android` (Kotlin) hiç derlenmedi; hata çıkarsa çıktının tamamıyla düzelt.
   - Bildirime dokunma, konum izni, konuşma tanıma, senkron cihazda ilk kez denenecek.
3. **T-01 güvenilir alarm** (uygulama kapalıyken). Bugün hatırlatmalar yalnız uygulama açıkken 30 sn yoklamayla geliyor.
4. **T-12 ajan iş gelince uyansın** (S24 pil sorunu).
5. **Sabun işi ilk gerçek paket:** T-14, T-15, N-1, N-3, N-6.
6. **2–3 hafta her gün kullanım.** Ölçüt: kendiliğinden açılıyor mu, WhatsApp'a kendine yazma bitti mi?

## 4. Çalışma kuralları (bu oturumda oturan alışkanlıklar)
- Her dilim: çalışır + test edilmiş + `./scripts/check.sh` yeşil (fmt, clippy -D warnings, tüm testler) → `scripts/commit-if-green.sh "mesaj"`.
- `.sqlx/` önbelleği: sorgu değişince `./scripts/prepare-sqlx.sh`.
- Arayüzde `use_context` beklemeden (`.await`) sonra okunamaz: `ctx()` / `backend()` önce alınır, `run_in(c, b, …)` kullanılır.
- Bellekteki test veritabanında tek bağlantı var: açık bir işlem içindeyken havuzdan sorgu yapılmaz (PoolTimedOut).
- Disk dolarsa `target/debug/incremental` ve `target/wasm32-unknown-unknown` silinir.
- Gökhan kod yazmıyor: Türkçe, kısa gerekçeyle anlat; karar gerektiren yerde sor (`CLAUDE.md` çalışma sırası).
