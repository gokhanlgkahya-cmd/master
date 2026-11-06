# NotAI — Üç Tasarım Yönü (Faz 1 · Adım 1)

> Yazan: design-director · Tarih: 2026-11-06 · Kaynak: `design/brief.md` (Revizyon 1 bağlayıcı) + `CLAUDE.md`
> Durum: **Seçim bekliyor.** `design/tokens.json` ve `design/guidelines.md` seçimden SONRA yazılacak.
> ui-designer için: `.claude/agents/ui-designer.md` içindeki eski ekran listesi (Bugün / Gelen Kutusu / Hızlı Yakalama…) **geçersizdir**; bu dosyadaki 5 ekran geçerlidir.

---

## 0. Bir bakışta

| | **A — Taş Defter** | **B — Çelik Pano** | **C — Saf Mürekkep** |
|---|---|---|---|
| Ruh hali | sakin · sıcak · samimi | kesin · çevik · düzenli | net · cesur · odaklı |
| Tek cümle | Kâğıt bir defterle sohbet eder gibi. | Bir mühendisin düzgün tutulmuş kayıt defteri. | Gazete sayfası kadar net, tek bakışta okunur. |
| Nötrün sıcaklığı | Sıcak taş/kâğıt grisi (hafif kahve) | Soğuk çelik grisi (hafif mavi) | Saf siyah-beyaz (renk tonu yok) |
| Kayıt görünümü | **Balon** (kuyruksuz, sağa yaslı) | **Çizgisiz satır** (solda saat sütunu) | **Kart** (tam genişlik, ince kenar) |
| Not detayı | **Bottom sheet** (alttan kayan sayfa, 2 durak) | **Yerinde genişleyen kart** | **Tam ekran** (kart büyüyerek ekrana dönüşür) |
| Yazı tipi | Manrope + IBM Plex Mono | Geist + Geist Mono | Schibsted Grotesk (başlık) + Inter (gövde) + Fragment Mono |
| Tip ölçeği | 1,2 oran · gövde 16px | 1,125 oran · gövde 16px (sıkı) | 1,25 oran · gövde 17px (iri başlıklar) |
| Köşe | 8 / 12 / 16 | 6 / 8 / 10 | 8 / 10 / 12 (tam ekranda 0) |
| Gölge | Yumuşak, sıcak, dağınık | Neredeyse yok; kenar çizgisiyle katman | Kartta yok; yalnız uçan öğelerde güçlü |
| Yay karakteri | "Yastık" — yavaşça oturur, hafif esner | "Çıt" — hızlı, kısa ve sıkı | "Lastik" — belirgin zıplama |
| Sekme çubuğu | İkon + etiket, etkin sekmede yumuşak hap | İkon + etiket, üstte kayan 2px çizgi | Pasifler yalnız ikon, etkin sekme siyah hapa açılır |
| Akış ekranında sekme çubuğu | Gizli (ekran üstüne açılır) | Hep görünür | Gizli |
| Koyu tema zemini | `#171512` (sıcak kömür) | `#0E1115` (gece mavisi-gri) | `#000000` (AMOLED siyahı, S24'te pil dostu) |

**Seçim için sorulacak asıl soru:** "Kayıtlarını balon mu, satır mı, kart mı görmek istersin; not detayını alttan mı, yerinde mi, tam ekran mı açalım?" Renk sıcaklığı ve yazı tipi ikincil — karışım mümkün (ör. "A'nın sıcak grisi + B'nin satırları").

---

## 1. Ortak zemin (üç yön için de aynı — farklılaşma bunun ÜSTÜNDE)

### 1.1 Sabit iskelet (Revizyon 1)
- **Ana ekran:** başlık "Günlük" · filtre (yöne göre değişir) · en üstte **sabitli Ana Günlük** satırı (işlenmemiş sayısı rozeti) · altında konu satırları (ikon · ad · son kayıt önizlemesi · saat) · **yazma çubuğu** (Ana Günlük'e yazar) · 5 sekmeli alt çubuk.
- **Akış ekranı:** üst çubuk (geri · konu ikonu + adı · ⋯ menü) · gün ayraçları (Çarşamba, 4 Kasım / Dün / Bugün) · en yeni en altta, açılışta en alta kaydırılmış · kayıt altında AI çipleri · işlenmemiş işareti · sabit yazma çubuğu (+ ataç · metin alanı · kamera · mikrofon; yazı varken mikrofon yerine Gönder).
- **Sekmeler (5):** Günlük · Görevler · Takvim · Hatırlatma · Ara/Sor.
  - Neden "Hatırlatmalar" değil "Hatırlatma": 412px genişlikte sekme başına ~82px var; "Hatırlatmalar" 12px'te ~75px tutuyor ve Android'de yazı büyütülünce taşıyor (ölçüldü: Manrope 74px, Geist 75px, Inter 73px). "Hatırlatma" ~59px → %35 büyütmeye kadar sığar. Ekran başlığı yine "Hatırlatmalar".
- **Dokunma hedefi:** her tıklanabilir öğe ≥ 44×44px (çip görseli 32px olabilir, dokunma alanı 44px'e genişletilir).
- **Boşluk ızgarası:** 4px. Değişkenler `--space-1`=4 · `-2`=8 · `-3`=12 · `-4`=16 · `-5`=20 · `-6`=24 · `-8`=32 · `-10`=40 · `-12`=48.
- **Ekran:** 412×915 telefon çerçevesi; durum çubuğu 32px, Android hareket çubuğu 24px bırak.

### 1.2 Renk kuralı
Tek "renk" = o yönün **mürekkebi** (en koyu nötr). Butonlar, odak halkası, etkin sekme mürekkeple çizilir. Renkli (semantik) tonlar YALNIZ şunlarda: **hata** = gecikmiş iş / silme uyarısı · **uyarı** = bugün/yakında son tarih · **başarı** = tamamlandı / öneri kabul edildi (kısa geri bildirim) · **bilgi** = senkron/çevrimdışı/sistem notu. **AI çipleri asla renkli değildir** (nötr, soluk).

### 1.3 Yazı tipi doğrulaması (font dosyaları indirilip glif tablosu kontrol edildi)
| Font | Lisans | Ç ç Ğ ğ İ ı Ö ö Ş ş Ü ü Â Î Û | ₺ (TL) | Tablo rakam (tnum) | Türkçe yerelleştirme (locl) |
|---|---|---|---|---|---|
| Manrope | OFL | ✓ | ✓ | ✓ | ✓ |
| IBM Plex Mono | OFL | ✓ | ✓ | (mono, gerek yok) | — |
| Geist | OFL | ✓ | **✗ yok** | ✓ | ✓ |
| Geist Mono | OFL | ✓ | **✗ yok** | (mono) | ✓ |
| Inter | OFL | ✓ | ✓ | ✓ | ✓ |
| Schibsted Grotesk | OFL | ✓ | ✓ | ✓ | ✓ |
| Fragment Mono | OFL | ✓ | ✓ | (mono) | ✓ |

- **₺ sorunu (yalnız B):** Geist ailesinde TL işareti yok. Çözüm: font yığınına Inter / Fragment Mono eklenir, tarayıcı yalnız ₺ karakterini oradan alır (bkz. B tipografi). Kullanıcı fiyat yazdığı için (tedarikçi teklifleri) bu zorunlu.
- **✦ işareti hiçbir fontta yok** → AI simgesi her yönde **inline SVG** (4 köşeli yıldız, tek renk, kendi çizimimiz) olarak çizilir; `✦` karakteri metin olarak KULLANILMAZ.
- `<html lang="tr">` zorunlu: büyük harfe çevirmede "i → İ", "ı → I" doğru olsun (ör. `İŞLENMEDİ`, `ÇARŞAMBA`). Saatlerde `font-variant-numeric: tabular-nums` (rakamlar aynı genişlikte, saatler alt alta hizalı).
- Türkçe test dizisi (her ekranda en az bir kez görünmeli): **"Ğ ğ Ş ş İ i I ı Ö ö Ü ü Ç ç"**, uzun kelime **"Değer akışı haritalandırması"**, büyük harf **"İŞLENMEDİ"**.

### 1.4 Ortak CSS değişken sözleşmesi (adlar üç yönde AYNI, değerler farklı)
Böylece eleştirmen yan yana karşılaştırabilir, seçilen yön doğrudan `tokens.json`'a dönüşür.

```
/* Ham nötr ölçek */            --neutral-50 … --neutral-950
/* Yüzeyler */                  --color-bg  --color-surface  --color-surface-raised  --color-surface-sunken
/* Metin */                     --color-text  --color-text-muted
/* Kenarlar */                  --color-border (dekoratif ayraç)  --color-border-strong (form/odak, ≥3:1)
/* Ana eylem */                 --color-ink  --color-on-ink  --color-focus
/* AI çipi */                   --color-chip-bg  --color-chip-text  --color-chip-border
/* Durum */                     --color-unprocessed  --color-scrim
/* Sekme */                     --color-tab-active  --color-tab-idle  --color-tab-indicator
/* Semantik */                  --color-success  --color-success-bg  --color-warning  --color-warning-bg
                                --color-error  --color-error-bg  --color-info  --color-info-bg
/* Tipografi */                 --font-heading  --font-body  --font-mono
                                --text-xs  --text-sm  --text-md  --text-lg  --text-xl  --text-2xl
                                --leading-tight  --leading-body  --weight-regular  --weight-medium  --weight-semibold  --weight-bold
                                --tracking-tight  --tracking-caps
/* Boşluk */                    --space-1 … --space-12
/* Köşe */                      --radius-sm  --radius-md  --radius-lg  --radius-pill
/* Gölge */                     --shadow-1  --shadow-2  --shadow-sheet
/* Hareket */                   --ease-spring-panel  --dur-spring-panel  --ease-spring-pop  --dur-spring-pop
                                --ease-out  --dur-fade
/* Ölçü */                      --size-touch  --size-tabbar  --size-composer  --size-topic-icon
```
- Tema uygulaması: değişkenler `.theme-light` ve `.theme-dark` sınıflarında tanımlanır (prototipte iki çerçeve yan yana); uygulamada `:root[data-theme="dark"]` + `@media (prefers-color-scheme: dark)`.
- Kodda sabit renk/ölçü YOK; her değer değişkenden.

### 1.5 Hareket (yay = spring) ortak kuralları
"Yay" eğrisi: bir öğe hedefine giderken hafifçe aşıp geri oturur. Değerler fizik parametreleriyle (sertlik k, sönüm c, kütle 1) hesaplandı ve CSS'in `linear()` zamanlama fonksiyonuna örneklendi — ui-designer bu dizgileri **olduğu gibi** kopyalar (Chromium 113+ destekler).
- **panel** eğrisi: sheet, panel, genişleyen kart, tam ekran geçişi (büyük yüzeyler).
- **pop** eğrisi: çip kabulü, gönderilen kaydın yerine oturması, sekme göstergesi, rozet (küçük öğeler).
- **Kapanış** her zaman açılıştan kısa: `--dur-spring-panel × 0,6`, `--ease-out` ile (aşma yok — kapanırken zıplama rahatsız eder).
- `@media (prefers-reduced-motion: reduce)`: tüm yaylar → `opacity` geçişi, `--dur-fade` (120ms), ölçek/kayma yok.
- **Kaydet/gönder hissi (üçünde de):** gönderilen kayıt yazma çubuğundan yukarı kayar (`translateY(16px)→0`, `scale(.96)→1`, pop eğrisi). AI çipleri 300ms sonra 40ms arayla tek tek belirir.

Yay parametreleri (kütle 1; sönüm oranı 1'e yaklaştıkça zıplama azalır):

| Yön · eğri | k (sertlik) | c (sönüm) | sönüm oranı | süre | aşma |
|---|---|---|---|---|---|
| A · panel | 170 | 20 | 0.77 | ~610ms | %2.2 |
| A · pop | 220 | 19 | 0.64 | ~602ms | %7.1 |
| B · panel | 520 | 36 | 0.79 | ~369ms | %1.6 |
| B · pop | 620 | 30 | 0.60 | ~424ms | %9.1 |
| C · panel | 300 | 28 | 0.81 | ~466ms | %1.2 |
| C · pop | 380 | 19 | 0.49 | ~657ms | %17.1 |

`linear()` dizgileri her yönün CSS bloğunda (`--ease-spring-panel`, `--ease-spring-pop`).

### 1.6 Referans sınırı (yalnız kalıp; görsel kopya yok)
| Kaynak | Alınan kalıp | Alınmayan |
|---|---|---|
| WhatsApp | Sohbet listesi → akış iskeleti, en yeni altta, gün ayracı, sabit yazma çubuğu, ataç paneli, ilet → "Konuya taşı" | Yeşil, duvar kâğıdı deseni, balon kuyruğu, ikonlar, çift tik, logo |
| Saner.AI | Kayıt altında otomatik etiket/konu önerisi, sabah öncelik planı | Arayüz renkleri, kart düzeni, adlandırma |
| Todoist (+Ramble) | Konuşurken canlı yapılandırma ve "aslında perşembe yap" düzeltmesi, doğal dilde tarih | Kırmızı marka, öncelik bayrakları görünümü, ikonlar |
| Google Keep (+Gemini) | Saniyede not, notlara sohbetle soru (Ara/Sor) | Renkli kart paleti, Gemini'nin renkli yıldızı, ızgara düzeni |

İkon setleri: açık lisanslı, yöne göre farklı (A: Phosphor · B: Lucide · C: Tabler). Çevrimdışıysan aynı çizgi kalınlığında basit inline SVG çiz; WhatsApp/Google/Todoist ikonlarını taklit etme.

---

## 2. YÖN A — "Taş Defter"

**Ruh hali:** sakin · sıcak · samimi
**Fikir:** Kâğıt bir defter ve onunla konuşan nazik bir asistan. Kendi yazdıkların sağda balonda (WhatsApp'ta "kendine mesaj" alışkanlığı), AI'ın sabah planı solda — iki taraflı bir sohbet hissi.
**Neden:** Gökhan'ın yakalama alışkanlığı WhatsApp; en tanıdık, en az öğrenme gerektiren yön. Sıcak gri, gece koyu temada gözü yormaz.
**Riski:** Balonlar uzun notlarda satır uzunluğunu kısaltır; arşivlenmiş eski günleri tararken satır görünümünden yavaş okunur.

### A.1 Renk
Nötr ölçek "taş" (sarımsı-kahve alt tonlu gri):

| 50 | 100 | 200 | 300 | 400 | 500 | 600 | 700 | 800 | 900 | 950 |
|---|---|---|---|---|---|---|---|---|---|---|
| `#FAF8F5` | `#F3F0EB` | `#E7E2DA` | `#D5CEC3` | `#B3AA9C` | `#8E8576` | `#6F675A` | `#554F45` | `#3A3530` | `#26231F` | `#171512` |

Semantik (toprak tonları, kısık doygunluk — nötr palete uyar):

| | Açık · metin | Açık · soluk zemin | Koyu · metin | Koyu · soluk zemin |
|---|---|---|---|---|
| Başarı | `#3F6B3A` | `#E8EFE3` | `#9CC48F` | `#1F2A1C` |
| Uyarı | `#855512` | `#F6EBD9` | `#E0B46A` | `#2E2414` |
| Hata | `#A33A2C` | `#F7E3DF` | `#F09A8B` | `#33191A` |
| Bilgi | `#2F5F7A` | `#E1ECF2` | `#8FC0DB` | `#172630` |

### A.2 Tipografi — Manrope (tümü) + IBM Plex Mono (yalnız kod/kimlik)
Manrope: yuvarlağa yakın, açık harf boşluklu modern grotesk; sıcak ama ciddi. Ölçek 1,2 (minör üçlü):

| Değişken | px / satır yüksekliği | Ağırlık | Kullanım |
|---|---|---|---|
| `--text-xs` | 12 / 16 | 500 | Sekme etiketi, saat |
| `--text-sm` | 14 / 20 | 500 | Çip, önizleme, meta |
| `--text-md` | 16 / 26 | 400 | Kayıt metni (gövde) |
| `--text-lg` | 19 / 26 | 600 | Konu adı, sheet başlığı |
| `--text-xl` | 23 / 30 | 700 | Ekran başlığı ("Günlük") |
| `--text-2xl` | 28 / 34 | 700 | Sabah planı sayısı |
`--tracking-tight: -0.01em` (lg ve üstü). Gün ayracı: `--text-sm`, 600, normal harf ("Dün", "Çarşamba, 4 Kasım").

### A.3 Biçim
- **Köşe:** `--radius-sm 8px` (çip dışı küçük öğeler, giriş alanı) · `--radius-md 12px` (balon, kart, ataç karoları) · `--radius-lg 16px` (yalnız sheet üst köşeleri) · `--radius-pill 999px` (çipler, sekme hapı).
- **Gölge:** açık temada yumuşak sıcak: `--shadow-1: 0 1px 2px rgba(38,35,31,.06)` (balon) · `--shadow-2: 0 4px 16px rgba(38,35,31,.08)` (ses paneli) · `--shadow-sheet: 0 -8px 32px rgba(38,35,31,.14)`. Koyu temada gölge yerine yüzey bir ton açılır + üst kenarda `inset 0 1px 0 rgba(243,240,235,.06)` ışık çizgisi; sheet `0 -8px 32px rgba(0,0,0,.5)`.
- **Hareket — "Yastık":** sakin, ağırca oturan yay. Panel `k170 c20` (~610ms, %2 aşma), pop `k220 c19` (~600ms, %7 aşma). Sheet açılırken arka ekran `scale(.96)` + karartma — "kâğıdın altına bir sayfa daha girdi" hissi.
- **İkon:** Phosphor "Regular" (yuvarlak uçlu, 1,5px eşdeğeri).

### A.4 Bileşen davranışı
- **Kayıt = balon:** kuyruksuz, `--radius-md`, **sağa yaslı**, en fazla %86 genişlik, `--color-surface` (beyaz) üzerinde `--color-border` ince çizgi + `--shadow-1`. Saat balon içinde sağ altta (`--text-xs`, muted). Balonlar arası 8px, farklı saatte gruplar arası 16px, gün ayracı üst-alt 24px.
- **Doğrudan konuya girilmiş kayıt** Ana Günlük'te: balonun üstünde küçük konu etiketi (ikon + "GemOlive", `--text-xs`).
- **AI çipi:** balonun **altında**, sağa yaslı sıra; hap biçimli, `--color-chip-bg` dolgu, kenar yok, solda SVG ✦ ikonu, sağda küçük × (ret). Dokununca: pop yayıyla `scale(1→1.06→1)`, dolgu mürekkebe döner, 1,2 sn sonra çip kaybolur ve balonun üst etiketine "GemOlive" olarak taşınır.
- **İşlenmemiş işareti:** balonun **solunda**, dikey ortada 8px dolu nokta (`--color-unprocessed`). Ekran okuyucu metni: "İşlenmedi". Ana ekranda Ana Günlük satırında yuvarlak rozet (mürekkep zemin, `--color-on-ink` sayı).
- **Sabah planı kartı:** solda (asistan tarafı), tam genişlik, `--color-surface-sunken` zemin + `--color-border` ince çizgi, `--radius-md`; başlık "Günaydın Gökhan", 3 maddelik onay kutulu liste, altta "Planı onayla" (mürekkep buton) + "Düzenle" (metin buton).
- **Ana ekran filtresi:** kaydırılabilir hap çipler: **Tümü · Sabitli · Projeler · Alanlar · Kaynaklar**; seçili hap mürekkep dolgu.
- **Haftalık gözden geçirme erişimi:** Pazar günleri Ana Günlük'te sabah planı yerine "Haftalık gözden geçirme" kartı + ana ekran başlığında takvim-onay ikonu.
- **Sekme çubuğu:** ikon + etiket hep görünür; etkin sekmede ikonun arkasında 56×32 yumuşak hap (`--color-tab-indicator`) pop yayıyla genişleyerek belirir. Akış ekranında sekme çubuğu **gizli** (akış ekranı ana ekranın üstüne sağdan kayarak açılır, panel yayı).
- **Bottom sheet (genel kural):** tutamak (36×4px) · üst köşe 16px · iki durak: **%55 "göz at"** ve **%92 "düzenle"** · aşağı çekince kapanır · arka ekran karartma `--color-scrim` + `scale(.96)`.
- **Not detayı = bottom sheet:** kayda dokununca sheet %55'te açılır (metin + AI önerileri + ana buton); yukarı çekince %92 (tüm alanlar, bağlantılar). Ana eylem altta sabit: "Görev olarak kaydet".
- **Ataç paneli = bottom sheet (tek durak, içerik yüksekliği):** 3×3 ızgara, her karo 56px ikon kutusu (`--color-surface-sunken`, `--radius-md`) + altında etiket; karolar 20ms arayla aşağıdan belirir.
- **Ses paneli:** yazma çubuğunun 8px üstünde yüzen kart (`--shadow-2`, `--radius-md`); solda 5 yumuşak çubuklu "nefes alan" dalga, canlı metin `--text-md`, altında yapılandırılmış önizleme çipi; altta "İptal" (metin) · "Gönder" (mürekkep). Mikrofon butonu durdur kare ikonuna döner.

#### A — CSS değişkenleri (kopyala-yapıştır)
```css
.theme-light, .theme-dark {
  --neutral-50: #FAF8F5;  --neutral-100: #F3F0EB;  --neutral-200: #E7E2DA;  --neutral-300: #D5CEC3;  --neutral-400: #B3AA9C;  --neutral-500: #8E8576;
  --neutral-600: #6F675A;  --neutral-700: #554F45;  --neutral-800: #3A3530;  --neutral-900: #26231F;  --neutral-950: #171512;
  --font-heading: "Manrope", system-ui, sans-serif;
  --font-body: "Manrope", system-ui, sans-serif;
  --font-mono: "IBM Plex Mono", ui-monospace, monospace;
  --text-xs: 12px; --text-sm: 14px; --text-md: 16px; --text-lg: 19px; --text-xl: 23px; --text-2xl: 28px;
  --leading-tight: 1.25; --leading-body: 1.6;
  --weight-regular: 400; --weight-medium: 500; --weight-semibold: 600; --weight-bold: 700;
  --tracking-tight: -0.01em; --tracking-caps: 0.02em;
  --space-1: 4px; --space-2: 8px; --space-3: 12px; --space-4: 16px; --space-5: 20px;
  --space-6: 24px; --space-8: 32px; --space-10: 40px; --space-12: 48px;
  --radius-sm: 8px; --radius-md: 12px; --radius-lg: 16px; --radius-pill: 999px;
  --ease-spring-panel: linear(0, 0.031, 0.111, 0.213, 0.325, 0.443, 0.549, 0.646, 0.734, 0.804, 0.863, 0.911, 0.947, 0.974, 0.994, 1.007, 1.015, 1.02, 1.022, 1.022, 1.021, 1.019, 1.016, 1.014, 1.011, 1.009, 1.007, 1.005, 1.004, 1.002, 1);
  --dur-spring-panel: 610ms;
  --ease-spring-pop: linear(0, 0.041, 0.139, 0.269, 0.411, 0.551, 0.679, 0.79, 0.885, 0.955, 1.005, 1.04, 1.06, 1.07, 1.071, 1.067, 1.059, 1.049, 1.039, 1.029, 1.021, 1.013, 1.007, 1.002, 0.999, 0.997, 0.995, 0.995, 0.995, 0.995, 1);
  --dur-spring-pop: 602ms;
  --ease-out: cubic-bezier(.2, .8, .2, 1); --dur-fade: 120ms;
  --size-touch: 44px; --size-tabbar: 64px; --size-composer: 64px; --size-topic-icon: 40px;
}
.theme-light {
  --color-bg: #FAF8F5;
  --color-surface: #FFFFFF;
  --color-surface-raised: #FFFFFF;
  --color-surface-sunken: #F3F0EB;
  --color-text: #26231F;
  --color-text-muted: #6F675A;
  --color-border: #E7E2DA;
  --color-border-strong: #8E8576;
  --color-ink: #26231F;
  --color-on-ink: #FAF8F5;
  --color-focus: #26231F;
  --color-chip-bg: #F3F0EB;
  --color-chip-text: #554F45;
  --color-chip-border: transparent;
  --color-unprocessed: #3A3530;
  --color-scrim: rgba(23,21,18,.32);
  --color-tab-active: #26231F;
  --color-tab-idle: #6F675A;
  --color-tab-indicator: #E7E2DA;
  --color-success: #3F6B3A; --color-success-bg: #E8EFE3;
  --color-warning: #855512; --color-warning-bg: #F6EBD9;
  --color-error: #A33A2C; --color-error-bg: #F7E3DF;
  --color-info: #2F5F7A; --color-info-bg: #E1ECF2;
  --shadow-1: 0 1px 2px rgba(38,35,31,.06); --shadow-2: 0 4px 16px rgba(38,35,31,.08);
  --shadow-sheet: 0 -8px 32px rgba(38,35,31,.14);
}
.theme-dark {
  --color-bg: #171512;
  --color-surface: #26231F;
  --color-surface-raised: #26231F;
  --color-surface-sunken: #3A3530;
  --color-text: #F3F0EB;
  --color-text-muted: #B3AA9C;
  --color-border: #3A3530;
  --color-border-strong: #8E8576;
  --color-ink: #F3F0EB;
  --color-on-ink: #171512;
  --color-focus: #F3F0EB;
  --color-chip-bg: #3A3530;
  --color-chip-text: #D5CEC3;
  --color-chip-border: transparent;
  --color-unprocessed: #E7E2DA;
  --color-scrim: rgba(0,0,0,.56);
  --color-tab-active: #FAF8F5;
  --color-tab-idle: #B3AA9C;
  --color-tab-indicator: #3A3530;
  --color-success: #9CC48F; --color-success-bg: #1F2A1C;
  --color-warning: #E0B46A; --color-warning-bg: #2E2414;
  --color-error: #F09A8B; --color-error-bg: #33191A;
  --color-info: #8FC0DB; --color-info-bg: #172630;
  --shadow-1: inset 0 1px 0 rgba(243,240,235,.06); --shadow-2: inset 0 1px 0 rgba(243,240,235,.06), 0 8px 24px rgba(0,0,0,.4);
  --shadow-sheet: 0 -8px 32px rgba(0,0,0,.5);
}
@media (prefers-reduced-motion: reduce) {
  .theme-light, .theme-dark { --ease-spring-panel: ease-out; --ease-spring-pop: ease-out;
    --dur-spring-panel: var(--dur-fade); --dur-spring-pop: var(--dur-fade); }
  /* ayrıca: transform animasyonlarını kapat, yalnız opacity */
}
```

### A.5 İş paketi (ui-designer — Yön A)
Hedef dosya: `design/directions/A/index.html` · PNG: `preview-light.png`, `preview-dark.png`. İçerik: §5 ortak içerik listesi.
1. **Ana ekran:** başlık "Günlük" + sağda yalnız ⋯ (arama ayrı sekmede: Ara/Sor) · filtre hapları · sabitli Ana Günlük satırı (ikon, önizleme "Pazar gözden geçirmede iş–ev dengesini konuş.", saat 12:05, rozet **4**) · 9 konu satırı (§5.1) · yazma çubuğu "Ana Günlük'e yaz…" · sekme çubuğu (Günlük etkin).
2. **Ana Günlük akışı:** 3 gün ayracı, §5.2'deki kayıtlar sağa yaslı balonlar, sabah planı kartı solda, 4 işlenmemiş nokta, 12:05 kaydı "yeni gönderildi" anında (çipler beliriyor), yazma çubuğu, sekme çubuğu yok.
3. **Ataç paneli:** akış ekranı üstünde bottom sheet, 3×3: Fotoğraf · Kamera · Belge · Konum · Kişi · Görev · Hatırlatma · Etkinlik · Liste.
4. **Sesli yakalama paneli:** §5.4 metni; düzeltilen kelime üstü çizili ("~~Yarın~~ Perşembe"), süre 0:07.
5. **Not detayı:** 09:12 kaydı, bottom sheet %55 durağında; ikinci çerçevede %92 durağı göster (bu yön için istisna olarak 6. çerçeve serbest).

---

## 3. YÖN B — "Çelik Pano"

**Ruh hali:** kesin · çevik · düzenli
**Fikir:** Bir endüstri mühendisinin temiz tuttuğu kayıt defteri / vardiya günlüğü. Kutular yok; saat sütunu, hizalama ve boşluk düzeni taşır. Her şey yerinde açılır, ekran değişmez.
**Neden:** En çok kayıt sığan ama yine ferah kalan yön (çerçeve yok → boşluk içeriğe gider). Gökhan'ın Bullet Journal günlük log'una en yakın görünüm: saat · madde. Detay yerinde açıldığı için bağlam kaybolmaz.
**Riski:** Balon alışkanlığından en uzak olan; ilk bakışta "soğuk" gelebilir. Yerinde genişleyen kart uzun notlarda listeyi çok iter.

### B.1 Renk
Nötr ölçek "çelik" (mavimsi alt tonlu gri):

| 50 | 100 | 200 | 300 | 400 | 500 | 600 | 700 | 800 | 900 | 950 |
|---|---|---|---|---|---|---|---|---|---|---|
| `#F7F8FA` | `#EEF1F4` | `#DDE2E8` | `#C3CAD3` | `#9AA4B1` | `#727D8B` | `#56606D` | `#424A55` | `#2C323A` | `#1B1F25` | `#0E1115` |

Semantik (berrak, net tonlar):

| | Açık · metin | Açık · soluk zemin | Koyu · metin | Koyu · soluk zemin |
|---|---|---|---|---|
| Başarı | `#1E7449` | `#E3F4EA` | `#6FD3A0` | `#0F2A1E` |
| Uyarı | `#8A5A08` | `#FBF0DA` | `#F2C060` | `#2B2110` |
| Hata | `#B9342A` | `#FCE6E4` | `#FF8F84` | `#33161A` |
| Bilgi | `#1F60B2` | `#E3EEFB` | `#8CB8F5` | `#11223A` |

### B.2 Tipografi — Geist + Geist Mono (₺ için Inter / Fragment Mono yedek)
Geist: dar açıklıklı, teknik, çok düzgün grotesk. Mono yazı bu yönün imzası: saatler, tarihler, sayılar, gün ayraçları.
- `--font-body/heading: "Geist", "Inter", system-ui, sans-serif` · `--font-mono: "Geist Mono", "Fragment Mono", ui-monospace, monospace`
- Google Fonts'tan Inter'i yalnız ₺ için yükle: `…css2?family=Inter:wght@400;500&text=₺` (birkaç KB). Fragment Mono'yu da aynı şekilde `&text=₺`.

Ölçek 1,125 (majör ikili, sıkı):

| Değişken | px / satır | Ağırlık | Kullanım |
|---|---|---|---|
| `--text-xs` | 12 / 16 | 500 mono | Saat sütunu, gün ayracı (BÜYÜK HARF), sekme etiketi (sans) |
| `--text-sm` | 14 / 20 | 400 | Çip, önizleme, meta |
| `--text-md` | 16 / 24 | 400 | Kayıt metni |
| `--text-lg` | 18 / 24 | 600 | Konu adı, genişleyen kart başlığı |
| `--text-xl` | 20 / 26 | 600 | Ekran başlığı |
| `--text-2xl` | 23 / 28 | 600 | Sabah planı başlığı |
`--tracking-tight: -0.02em` (lg+) · `--tracking-caps: 0.06em` (mono büyük harf). Gün ayracı: `BUGÜN · CUMA 6 KASIM`, solda kısa çizgi — `lang="tr"` testi burada.

### B.3 Biçim
- **Köşe:** `--radius-sm 6px` (çip, giriş, küçük buton) · `--radius-md 8px` (genişleyen kart, panel karoları) · `--radius-lg 10px` (ataç paneli, ses şeridi) · `--radius-pill 999px` (yalnız rozet). Brief'in 8–12'sinin alt ucu: teknik keskinlik.
- **Gölge:** neredeyse yok. Katman = 1px `--color-border` + yüzey tonu. `--shadow-1: none` · `--shadow-2: 0 1px 2px rgba(14,17,21,.06), 0 8px 24px rgba(14,17,21,.08)` (yalnız genişleyen kart ve panel) · `--shadow-sheet` = `--shadow-2`. Koyu tema: gölge yok, yalnız kenar + yüzey (`#1B1F25`).
- **Hareket — "Çıt":** hızlı ve sıkı. Panel `k520 c36` (~370ms, %1,6 aşma), pop `k620 c30` (~420ms, %9 aşma — kısa olduğu için "tık" diye hissedilir). Yükseklik değişimleri (kart genişlemesi) yay ile; içerik 60ms gecikmeyle `opacity` ile gelir.
- **İkon:** Lucide, çizgi kalınlığı 1,5px; etkin sekmede 2px.

### B.4 Bileşen davranışı
- **Kayıt = çizgisiz satır:** tam genişlik, kutu/çizgi yok. Solda 48px **saat sütunu** (mono, muted, `09:12`), sağda metin. Satır dikey dolgusu 12px, satırlar arası boşluk 4px; ayırıcı çizgi YOK (boşluk ayırır). Ses kaydıysa saatin altında küçük dalga ikonu + "0:12".
- **Doğrudan konuya girilmiş kayıt:** metnin önünde mono etiket `[GemOlive]` yerine ikon + ad, `--text-xs`, muted.
- **AI çipi:** metnin altında sola yaslı sıra; **kesikli 1px kenar** (`--color-chip-border`, "henüz öneri, kesin değil" anlamı), şeffaf zemin, `--radius-sm`, solda SVG ✦. Kabul edilince kenar düz çizgiye döner, ✓ belirir (pop), 1 sn sonra çip saat sütunu altındaki konu etiketine "yapışır".
- **İşlenmemiş işareti:** saat sütununda saatin solunda 8px **içi boş halka** (1,5px, `--color-unprocessed`); işlenince halka dolup kaybolur. Ana ekranda sayaç rozeti: mono "4" köşeli küçük etiket.
- **Sabah planı:** kutu değil, akışın içinde bir blok: mono başlık `08:00 · SABAH PLANI`, altında onay kutulu 3 satır; gecikmiş madde sağında `--color-error` metinle "1 gün gecikti", bugünkü madde `--color-warning` "bugün". Altta iki küçük buton.
- **Ana ekran filtresi:** **bölmeli kontrol** (segmented control — tek kutuda yan yana seçenekler): `Tümü | Proje | Alan | Kaynak`. Sabitli = Ana Günlük her zaman en üstte, ayrı filtre yok.
- **Haftalık gözden geçirme erişimi:** konu listesinde Ana Günlük'ün hemen altında **sabit özel satır** "Haftalık gözden geçirme · Pazar 20:00 · 6 işlenmemiş".
- **Sekme çubuğu:** ikon + etiket; etkin sekmenin **üstünde 2px mürekkep çizgi**, sekmeler arası pop yayıyla kayar (genişliği önce uzar sonra toparlanır). **Akış ekranında da görünür**; yazma çubuğu sekme çubuğunun hemen üstünde.
- **Bottom sheet (genel kural):** bu yönde sheet az kullanılır; yalnız ⋯ menüsü ve tarih seçici. Tutamaksız, `--radius-lg`, içerik yüksekliği kadar, karartma `--color-scrim` (hafif).
- **Not detayı = yerinde genişleyen kart:** satıra dokununca o satır yerinde büyür: zemin `--color-surface-raised`, 1px kenar, `--radius-md`, `--shadow-2`; içinde tam metin, AI önerileri (kabul/ret), görev alanları (tarih · saat · öncelik · hatırlatma) iki sütunlu mono etiketlerle, bağlantılar, alt satırda eylemler. Liste, kart görünür kalacak şekilde kayar. Aynı anda tek kart açık; dışarı dokununca veya "Bitti" ile kapanır. Diğer satırlar yerinde kalır (karartma yok).
- **Ataç paneli = yazma çubuğuna yapışık panel:** ataça basınca klavye yerine, yazma çubuğunun altından ~280px panel açılır (karartma yok, akış görünür kalır). 3×3 ızgara: düz çizgi ikon + etiket, karo kenarı yok, basılınca `--color-surface-sunken`. Panel açıkken ataç ikonu × olur.
- **Ses paneli = yazma çubuğu dönüşür:** mikrofona basınca çubuk yukarı doğru ~120px'lik şeride uzar: üstte canlı metin (tek/iki satır, son kelimeler mürekkep, öncekiler muted), altında yapılandırılmış önizleme **mono satır** `GÖREV · Perşembe 12 Kas 10:00 · Kostik tedarik`, solda mono sayaç `00:07` ve 1px ses seviyesi çizgisi, sağda Gönder.

#### B — CSS değişkenleri (kopyala-yapıştır)
```css
.theme-light, .theme-dark {
  --neutral-50: #F7F8FA;  --neutral-100: #EEF1F4;  --neutral-200: #DDE2E8;  --neutral-300: #C3CAD3;  --neutral-400: #9AA4B1;  --neutral-500: #727D8B;
  --neutral-600: #56606D;  --neutral-700: #424A55;  --neutral-800: #2C323A;  --neutral-900: #1B1F25;  --neutral-950: #0E1115;
  --font-heading: "Geist", "Inter", system-ui, sans-serif;
  --font-body: "Geist", "Inter", system-ui, sans-serif;
  --font-mono: "Geist Mono", "Fragment Mono", ui-monospace, monospace;
  --text-xs: 12px; --text-sm: 14px; --text-md: 16px; --text-lg: 18px; --text-xl: 20px; --text-2xl: 23px;
  --leading-tight: 1.25; --leading-body: 1.5;
  --weight-regular: 400; --weight-medium: 500; --weight-semibold: 600; --weight-bold: 700;
  --tracking-tight: -0.02em; --tracking-caps: 0.06em;
  --space-1: 4px; --space-2: 8px; --space-3: 12px; --space-4: 16px; --space-5: 20px;
  --space-6: 24px; --space-8: 32px; --space-10: 40px; --space-12: 48px;
  --radius-sm: 6px; --radius-md: 8px; --radius-lg: 10px; --radius-pill: 999px;
  --ease-spring-panel: linear(0, 0.035, 0.126, 0.234, 0.35, 0.465, 0.581, 0.675, 0.755, 0.827, 0.88, 0.921, 0.955, 0.978, 0.994, 1.004, 1.011, 1.015, 1.016, 1.016, 1.015, 1.013, 1.011, 1.009, 1.007, 1.006, 1.004, 1.003, 1.002, 1.001, 1);
  --dur-spring-panel: 369ms;
  --ease-spring-pop: linear(0, 0.057, 0.188, 0.354, 0.539, 0.697, 0.83, 0.935, 1.009, 1.057, 1.082, 1.091, 1.087, 1.075, 1.06, 1.045, 1.03, 1.017, 1.007, 1, 0.995, 0.993, 0.992, 0.992, 0.993, 0.994, 0.996, 0.997, 0.998, 0.999, 1);
  --dur-spring-pop: 424ms;
  --ease-out: cubic-bezier(.2, .8, .2, 1); --dur-fade: 120ms;
  --size-touch: 44px; --size-tabbar: 60px; --size-composer: 60px; --size-topic-icon: 40px;
}
.theme-light {
  --color-bg: #FFFFFF;
  --color-surface: #FFFFFF;
  --color-surface-raised: #F7F8FA;
  --color-surface-sunken: #F7F8FA;
  --color-text: #1B1F25;
  --color-text-muted: #56606D;
  --color-border: #DDE2E8;
  --color-border-strong: #727D8B;
  --color-ink: #1B1F25;
  --color-on-ink: #FFFFFF;
  --color-focus: #1B1F25;
  --color-chip-bg: transparent;
  --color-chip-text: #424A55;
  --color-chip-border: #727D8B;
  --color-unprocessed: #424A55;
  --color-scrim: rgba(14,17,21,.24);
  --color-tab-active: #1B1F25;
  --color-tab-idle: #56606D;
  --color-tab-indicator: #1B1F25;
  --color-success: #1E7449; --color-success-bg: #E3F4EA;
  --color-warning: #8A5A08; --color-warning-bg: #FBF0DA;
  --color-error: #B9342A; --color-error-bg: #FCE6E4;
  --color-info: #1F60B2; --color-info-bg: #E3EEFB;
  --shadow-1: none; --shadow-2: 0 1px 2px rgba(14,17,21,.06), 0 8px 24px rgba(14,17,21,.08);
  --shadow-sheet: 0 1px 2px rgba(14,17,21,.06), 0 8px 24px rgba(14,17,21,.08);
}
.theme-dark {
  --color-bg: #0E1115;
  --color-surface: #0E1115;
  --color-surface-raised: #1B1F25;
  --color-surface-sunken: #1B1F25;
  --color-text: #EEF1F4;
  --color-text-muted: #9AA4B1;
  --color-border: #2C323A;
  --color-border-strong: #727D8B;
  --color-ink: #EEF1F4;
  --color-on-ink: #0E1115;
  --color-focus: #EEF1F4;
  --color-chip-bg: transparent;
  --color-chip-text: #C3CAD3;
  --color-chip-border: #727D8B;
  --color-unprocessed: #C3CAD3;
  --color-scrim: rgba(0,0,0,.5);
  --color-tab-active: #EEF1F4;
  --color-tab-idle: #9AA4B1;
  --color-tab-indicator: #EEF1F4;
  --color-success: #6FD3A0; --color-success-bg: #0F2A1E;
  --color-warning: #F2C060; --color-warning-bg: #2B2110;
  --color-error: #FF8F84; --color-error-bg: #33161A;
  --color-info: #8CB8F5; --color-info-bg: #11223A;
  --shadow-1: none; --shadow-2: none;
  --shadow-sheet: none;
}
@media (prefers-reduced-motion: reduce) {
  .theme-light, .theme-dark { --ease-spring-panel: ease-out; --ease-spring-pop: ease-out;
    --dur-spring-panel: var(--dur-fade); --dur-spring-pop: var(--dur-fade); }
  /* ayrıca: transform animasyonlarını kapat, yalnız opacity */
}
```

### B.5 İş paketi (ui-designer — Yön B)
Hedef dosya: `design/directions/B/index.html` · PNG: `preview-light.png`, `preview-dark.png`. İçerik: §5.
1. **Ana ekran:** başlık "Günlük" · bölmeli kontrol · Ana Günlük (rozet 4) · "Haftalık gözden geçirme" özel satırı · diğer konu satırları (satır ayıracı yok, 64px yükseklik) · yazma çubuğu · sekme çubuğu.
2. **Ana Günlük akışı:** mono gün ayraçları (`ÇARŞAMBA · 4 KASIM`, `DÜN · PERŞEMBE 5 KASIM`, `BUGÜN · CUMA 6 KASIM`), saat sütunlu satırlar, sabah planı bloğu, kesikli çipler, içi boş halkalar, **sekme çubuğu görünür**.
3. **Ataç paneli:** yazma çubuğuna yapışık, klavye yüksekliğinde panel.
4. **Sesli yakalama:** yazma çubuğunun ses şeridine dönüşmüş hali.
5. **Not detayı:** 09:12 satırı yerinde genişlemiş; üstünde 08:00 planın son satırı, altında 10:47 satırı görünür kalsın (bağlamı göstermek için).

---

## 4. YÖN C — "Saf Mürekkep"

**Ruh hali:** net · cesur · odaklı
**Fikir:** Renk tonu sıfır, saf siyah-beyaz; kontrast ve iri tipografi hiyerarşiyi kurar. Gazete/afiş netliği. Her kayıt bir kart; dokununca kart ekranı kaplar ve yalnız o not kalır ("ekranda yalnız o anki iş" ilkesinin en uç hali).
**Neden:** En yüksek okunabilirlik (ana metin 18–20:1), güneşte telefon ekranında en iyi. Koyu temada saf siyah, Samsung AMOLED ekranda pil tasarrufu ve gece rahatlığı.
**Riski:** Siyah-beyaz sertliği uzun süre kullanımda "soğuk" gelebilir; iri başlıklar ekrana daha az kayıt sığdırır (~4 kayıt). Lastik yay abartılırsa oyuncak gibi durabilir — yalnız küçük öğelerde kullanılır.

### C.1 Renk
Nötr ölçek "mürekkep" (tam akromatik — R=G=B):

| 50 | 100 | 200 | 300 | 400 | 500 | 600 | 700 | 800 | 900 | 950 |
|---|---|---|---|---|---|---|---|---|---|---|
| `#FAFAFA` | `#F4F4F4` | `#E6E6E6` | `#D1D1D1` | `#A3A3A3` | `#767676` | `#5C5C5C` | `#474747` | `#2E2E2E` | `#171717` | `#0A0A0A` |
Ek: koyu tema zemini `#000000` (saf siyah).

Semantik (doygun ama az kullanılan — siyah-beyaz içinde tek renk lekesi olarak hemen görünür):

| | Açık · metin | Açık · soluk zemin | Koyu · metin | Koyu · soluk zemin |
|---|---|---|---|---|
| Başarı | `#157A3B` | `#E2F6E8` | `#5BD68A` | `#0B2414` |
| Uyarı | `#8F5400` | `#FFF1D6` | `#FFC14D` | `#2A1D00` |
| Hata | `#C42A20` | `#FFE5E2` | `#FF7A70` | `#330F0C` |
| Bilgi | `#1B5AC7` | `#E3ECFF` | `#7EA8FF` | `#0D1A38` |

### C.2 Tipografi — Schibsted Grotesk (başlık) + Inter (gövde) + Fragment Mono
Schibsted Grotesk: gazete için çizilmiş, kalın kesimleri güçlü bir grotesk → gün ayraçları ve başlıklar. Inter: ekranda en okunaklı gövde fontlarından; küçük boyutta net. İki grotesk birlikte "manşet + metin" ayrımı kurar.
- `--font-heading: "Schibsted Grotesk", "Inter", sans-serif` · `--font-body: "Inter", system-ui, sans-serif` · `--font-mono: "Fragment Mono", ui-monospace, monospace`
Ölçek 1,25 (majör üçlü, iri):

| Değişken | px / satır | Ağırlık | Kullanım |
|---|---|---|---|
| `--text-xs` | 13 / 18 | 500 Inter | Saat, sekme etiketi, meta |
| `--text-sm` | 15 / 22 | 500 Inter | Çip, önizleme |
| `--text-md` | 17 / 26 | 400 Inter | Kayıt metni |
| `--text-lg` | 21 / 28 | 700 Schibsted | Konu adı, kart başlığı |
| `--text-xl` | 27 / 32 | 800 Schibsted | Gün ayracı ("Bugün"), ekran başlığı |
| `--text-2xl` | 34 / 38 | 800 Schibsted | Tam ekran not başlığı, plan sayısı ("3 iş") |
`--tracking-tight: -0.025em` (lg+). Gün ayracı sola yaslı iri başlık: **Bugün** / küçük alt satır "Cuma, 6 Kasım".

### C.3 Biçim
- **Köşe:** `--radius-sm 8px` (çip, buton, giriş) · `--radius-md 10px` (kayıt kartı) · `--radius-lg 12px` (sheet, ses kartı) · tam ekran açıkken kart köşesi 10→0'a yay ile iner · `--radius-pill 999px` (etkin sekme hapı).
- **Gölge:** kartlarda YOK (beyaz kart, `#F4F4F4` zemin, kontrastla ayrışır). Yalnız uçan öğeler: `--shadow-2: 0 12px 32px rgba(0,0,0,.16)` (ses kartı, büyüme anında kart) · `--shadow-sheet: 0 -12px 40px rgba(0,0,0,.18)`. Koyu tema: gölge yok, kartlar `#171717` + 1px `#2E2E2E` kenar.
- **Hareket — "Lastik":** belirgin zıplama. Panel `k300 c28` (~470ms, %1 aşma — büyük yüzeyler sakin), pop `k380 c19` (~660ms, **%17 aşma** — çip kabulü, gönderme, sekme hapı). Tam ekran geçişi "paylaşılan öğe" (shared element: kart, kendi yerinden büyüyerek ekrana dönüşür; geri hareketinde yerine küçülür).
- **İkon:** Tabler (1,75px çizgi); etkin sekmede "filled" (dolu) varyantı.

### C.4 Bileşen davranışı
- **Kayıt = kart:** tam genişlik (16px kenar boşluğu), `--color-surface`, `--radius-md`, 16px iç dolgu; kartlar arası 12px. Üst satırda saat + (varsa) konu etiketi, altında metin, en altta çipler. Fotoğraflı kayıtta görsel kartın üstünde tam genişlik, 4:3.
- **AI çipi:** kartın alt kısmında, `--color-chip-border` 1px kenarlı küçük kutu (`--radius-sm`), muted metin, SVG ✦. Kabulde pop yayıyla zıplar, dolgu mürekkebe döner (beyaz metin), sonra kartın üst etiketine dönüşür.
- **İşlenmemiş işareti:** kartın **sağ üst köşesinde** 8px dolu mürekkep nokta + kartın sol kenarında 3px mürekkep şerit (en belirgin işaret — Gelen Kutusu yokluğunu telafi eder).
- **Sabah planı kartı:** **ters renkli** (açık temada siyah kart, beyaz metin); solda iri "3 iş" (`--text-2xl`), sağda maddeler; gecikmiş madde için açık-hata tonu (`--color-error` koyu tema değeri `#FF7A70`, siyah zeminde 8,27:1). Koyu temada ters çevirme yok: `#171717` kart + 1px `#FAFAFA` kenar (gece göz kamaştırmasın).
- **Ana ekran filtresi:** büyük başlık "Günlük" altında **metin sekmeleri**: `Tümü  Projeler  Alanlar  Kaynaklar` — seçili kalın + altı 2px çizgili.
- **Haftalık gözden geçirme erişimi:** üst çubuk ⋯ menüsünde + Pazar akşamı Ana Günlük'ün sonunda tam genişlik ters renkli kart.
- **Sekme çubuğu:** pasif sekmeler yalnız ikon (erişilebilir ad `aria-label` ile + uzun basınca etiket balonu); **etkin sekme siyah hapa açılır**: ikon + tam etiket ("Hatırlatmalar" bile sığar çünkü diğerleri yalnız ikon). Hap genişliği sekme değiştikçe pop yayıyla akar. Akış ekranında sekme çubuğu **gizli**.
- **Bottom sheet (genel kural):** ataç ve ⋯ menüsü için; tutamaklı, `--radius-lg`, güçlü karartma (`--color-scrim` %48), tek durak (içerik yüksekliği).
- **Not detayı = tam ekran (paylaşılan öğe geçişi):** karta dokununca kart büyüyerek tüm ekranı kaplar (panel yayı, köşe 10→0). Üstte geri oku + ⋯, iri başlık (`--text-2xl`, ilk satır), gövde metni, "Öneriler" bölümü (her öneri bir satır: açıklama + Kabul/Ret), görev alanları, bağlantılar. Alta sabit tek ana buton (tam genişlik siyah): "Görev olarak kaydet". Android geri hareketiyle kart yerine küçülür.
- **Ataç paneli = iki bölmeli bottom sheet:** üstte **"Ekle"**: Fotoğraf · Kamera · Belge · Konum · Kişi (yatay 5 iri ikon, 64px); altta **"Oluştur"**: Görev · Hatırlatma · Etkinlik · Liste — her biri tam genişlik satır (ikon + başlık + tek satır açıklama, ör. "Görev — tarih ve öncelikle"). Medya ile yapılandırılmış kaydı ayırır.
- **Ses paneli:** yazma çubuğunun üstünde yüzen **ters renkli kart** (açık temada siyah) — canlı metin iri (`--text-lg` 21px, Inter 500), düzeltilen kelime üstü çizili + yeni kelime vurgulu; altında önizleme kutusu; sayaç mono. Koyu temada `#171717` + `#FAFAFA` kenar.

#### C — CSS değişkenleri (kopyala-yapıştır)
```css
.theme-light, .theme-dark {
  --neutral-50: #FAFAFA;  --neutral-100: #F4F4F4;  --neutral-200: #E6E6E6;  --neutral-300: #D1D1D1;  --neutral-400: #A3A3A3;  --neutral-500: #767676;
  --neutral-600: #5C5C5C;  --neutral-700: #474747;  --neutral-800: #2E2E2E;  --neutral-900: #171717;  --neutral-950: #0A0A0A;
  --font-heading: "Schibsted Grotesk", "Inter", sans-serif;
  --font-body: "Inter", system-ui, sans-serif;
  --font-mono: "Fragment Mono", ui-monospace, monospace;
  --text-xs: 13px; --text-sm: 15px; --text-md: 17px; --text-lg: 21px; --text-xl: 27px; --text-2xl: 34px;
  --leading-tight: 1.15; --leading-body: 1.5;
  --weight-regular: 400; --weight-medium: 500; --weight-semibold: 600; --weight-bold: 800;
  --tracking-tight: -0.025em; --tracking-caps: 0.04em;
  --space-1: 4px; --space-2: 8px; --space-3: 12px; --space-4: 16px; --space-5: 20px;
  --space-6: 24px; --space-8: 32px; --space-10: 40px; --space-12: 48px;
  --radius-sm: 8px; --radius-md: 10px; --radius-lg: 12px; --radius-pill: 999px;
  --ease-spring-panel: linear(0, 0.035, 0.112, 0.218, 0.327, 0.442, 0.544, 0.641, 0.72, 0.791, 0.846, 0.894, 0.928, 0.956, 0.975, 0.99, 1, 1.006, 1.01, 1.012, 1.012, 1.012, 1.011, 1.01, 1.008, 1.007, 1.006, 1.004, 1.003, 1.003, 1);
  --dur-spring-panel: 466ms;
  --ease-spring-pop: linear(0, 0.083, 0.274, 0.508, 0.735, 0.915, 1.053, 1.135, 1.169, 1.166, 1.139, 1.101, 1.061, 1.025, 0.998, 0.982, 0.973, 0.971, 0.974, 0.98, 0.987, 0.993, 0.999, 1.002, 1.004, 1.005, 1.005, 1.004, 1.003, 1.002, 1);
  --dur-spring-pop: 657ms;
  --ease-out: cubic-bezier(.2, .8, .2, 1); --dur-fade: 120ms;
  --size-touch: 44px; --size-tabbar: 68px; --size-composer: 64px; --size-topic-icon: 44px;
}
.theme-light {
  --color-bg: #F4F4F4;
  --color-surface: #FFFFFF;
  --color-surface-raised: #FFFFFF;
  --color-surface-sunken: #F4F4F4;
  --color-text: #0A0A0A;
  --color-text-muted: #5C5C5C;
  --color-border: #E6E6E6;
  --color-border-strong: #767676;
  --color-ink: #0A0A0A;
  --color-on-ink: #FFFFFF;
  --color-focus: #0A0A0A;
  --color-chip-bg: #FFFFFF;
  --color-chip-text: #5C5C5C;
  --color-chip-border: #D1D1D1;
  --color-unprocessed: #0A0A0A;
  --color-scrim: rgba(0,0,0,.48);
  --color-tab-active: #FFFFFF;
  --color-tab-idle: #5C5C5C;
  --color-tab-indicator: #0A0A0A;
  --color-success: #157A3B; --color-success-bg: #E2F6E8;
  --color-warning: #8F5400; --color-warning-bg: #FFF1D6;
  --color-error: #C42A20; --color-error-bg: #FFE5E2;
  --color-info: #1B5AC7; --color-info-bg: #E3ECFF;
  --shadow-1: none; --shadow-2: 0 12px 32px rgba(0,0,0,.16);
  --shadow-sheet: 0 -12px 40px rgba(0,0,0,.18);
}
.theme-dark {
  --color-bg: #000000;
  --color-surface: #171717;
  --color-surface-raised: #171717;
  --color-surface-sunken: #171717;
  --color-text: #FAFAFA;
  --color-text-muted: #A3A3A3;
  --color-border: #2E2E2E;
  --color-border-strong: #767676;
  --color-ink: #FAFAFA;
  --color-on-ink: #0A0A0A;
  --color-focus: #FAFAFA;
  --color-chip-bg: #171717;
  --color-chip-text: #A3A3A3;
  --color-chip-border: #474747;
  --color-unprocessed: #FAFAFA;
  --color-scrim: rgba(0,0,0,.64);
  --color-tab-active: #0A0A0A;
  --color-tab-idle: #A3A3A3;
  --color-tab-indicator: #FAFAFA;
  --color-success: #5BD68A; --color-success-bg: #0B2414;
  --color-warning: #FFC14D; --color-warning-bg: #2A1D00;
  --color-error: #FF7A70; --color-error-bg: #330F0C;
  --color-info: #7EA8FF; --color-info-bg: #0D1A38;
  --shadow-1: none; --shadow-2: none;
  --shadow-sheet: none;
}
@media (prefers-reduced-motion: reduce) {
  .theme-light, .theme-dark { --ease-spring-panel: ease-out; --ease-spring-pop: ease-out;
    --dur-spring-panel: var(--dur-fade); --dur-spring-pop: var(--dur-fade); }
  /* ayrıca: transform animasyonlarını kapat, yalnız opacity */
}
```

### C.5 İş paketi (ui-designer — Yön C)
Hedef dosya: `design/directions/C/index.html` · PNG: `preview-light.png`, `preview-dark.png`. İçerik: §5.
1. **Ana ekran:** iri "Günlük" başlığı · metin sekmeleri · Ana Günlük satırı (rozet 4, en üstte ince ayırıcıyla ayrılmış) · konu satırları · yazma çubuğu · ikonlu sekme çubuğu, etkin hap "Günlük".
2. **Ana Günlük akışı:** iri gün ayraçları (**Çarşamba** 4 Kasım / **Dün** / **Bugün**), kartlar, ters renkli sabah planı kartı, işlenmemiş şerit + nokta, sekme çubuğu yok.
3. **Ataç paneli:** "Ekle" + "Oluştur" iki bölmeli sheet.
4. **Sesli yakalama:** ters renkli yüzen kart.
5. **Not detayı:** tam ekran; mümkünse yanında küçük bir "geçiş anı" çerçevesi (kart %60 büyümüş, köşe 6px) — paylaşılan öğe fikrini göstermek için.

---

## 5. Ortak örnek içerik (üç yön AYNI içeriği kullanır — karşılaştırma adil olsun)

Tarih bağlamı: **bugün Cuma, 6 Kasım 2026.** Dün = Perşembe 5 Kasım. "Perşembe" = 12 Kasım. Pazar = 8 Kasım. 14 Kasım = Cumartesi.

### 5.1 Ana ekran konu satırları (ikon · ad · tür · önizleme · saat)
| # | Ad | Tür | Son kayıt önizlemesi | Saat |
|---|---|---|---|---|
| 📌 | **Ana Günlük** (sabitli, rozet 4) | — | Pazar gözden geçirmede iş–ev dengesini konuş. | 12:05 |
| 1 | GemOlive | Alan · İş (damla ikonu) | Ambalaj faturası · 1.850 ₺ | 10:47 |
| 2 | Kostik tedarik | Proje · 20 Kasım | Ayhan Kimya teklifi: NaOH %99, 25 kg torba | 09:12 |
| 3 | Logo ERP geçişi | Proje · 15 Aralık | Stok kartı aktarımı: 1.240 kalem, 37 hatalı | Dün |
| 4 | Ev | Alan | Kombi bakımı – 14 Kasım Cumartesi | Dün |
| 5 | Sağlık | Alan | Tansiyon 12/8 · sabah yürüyüşü 35 dk | Dün |
| 6 | Ev sunucusu (S24) | Proje | n8n: WhatsApp → NotAI webhook akışı çalıştı | Çarşamba |
| 7 | Okuma notları | Kaynak | Ohno, Toyota Üretim Sistemi – s. 84, "muda" | Çarşamba |
| 8 | Değer akışı haritalandırması | Kaynak | Çevrim süresi 42 sn, takt süresi 55 sn | 2 Kasım |
| 9 | Haftalık gözden geçirme | Özel | Geçen hafta: 14 görev bitti, 6 kayıt işlenmedi | 1 Kasım |
("Değer akışı haritalandırması" uzun-kelime taşma testidir: tek satırda üç nokta ile kesilmeli, kelime ortasından bölünmemeli.)

### 5.2 Ana Günlük akışı (eski → yeni, en yeni en altta)
**Çarşamba, 4 Kasım**
- 18:42 — "Logo'da stok kartlarının birim alanı boş geliyor, Murat Bey'e sor." · *işlendi* → etiket: Görev · Logo ERP geçişi
- 21:05 — "Lavanta uçucu yağı 2 kg kaldı, yeni sipariş ver." · *işlendi* → GemOlive · Görev

**Dün**
- 08:15 — 🎙 ses kaydı (0:12): "Kombi bakımı için servisi ara, 14 Kasım cumartesi uygun." · *işlendi* → Ev · Hatırlatma 14 Kas 10:00
- 13:30 — *[GemOlive'e doğrudan girildi]* "Zeytinyağlı sabunda kür süresi 4–6 hafta. Raf etiketine üretim tarihi şart."
- 22:10 — *[Okuma notları]* "Ohno'nun 7 israf türünü haftalık gözden geçirmede özetle."

**Bugün**
- 08:00 — **Sabah planı (AI):** "Günaydın Gökhan. Bugün 3 iş öne çıkıyor:"
  1. Kostik tekliflerini karşılaştır — *1 gün gecikti* (hata)
  2. Logo: 37 hatalı stok kartını düzelt — *bugün* (uyarı)
  3. 19:00 akşam yürüyüşü — Sağlık
  → [Planı onayla] [Düzenle]
- 09:12 — ● *işlenmedi* "Kostik tedarikçisini ara – perşembe. 25 kg'lık torba fiyatını ve teslim süresini sor."
  çipler: [✦ Görev · Perşembe 12 Kas] [✦ → Kostik tedarik] [✦ Kişi: Ayhan Kimya] [×]
- 10:47 — ● *işlenmedi* 📷 fotoğraf (fatura görseli yer tutucu, gri dikdörtgen + "fatura.jpg") "Ambalaj faturası"
  çipler: [✦ → GemOlive] [✦ Gider · 1.850 ₺]
- 11:30 — ● *işlenmedi* "Şirket aracının muayenesi ne zamandı?"
  çipler: [✦ Ara/Sor'da sor] [✦ Hatırlatma kur]
- 12:05 — ● *işlenmedi, yeni gönderildi* "Pazar gözden geçirmede iş–ev dengesini konuş."
  çip: [✦ → Haftalık gözden geçirme] (beliriyor)

Yazma çubuğu yer tutucusu: **"Aklındakini yaz…"** (ana ekranda: **"Ana Günlük'e yaz…"**).

### 5.3 Ataç paneli
Fotoğraf · Kamera · Belge · Konum · Kişi · Görev · Hatırlatma · Etkinlik · Liste

### 5.4 Sesli yakalama
- Canlı metin: "Yarın sabah onda İzmir'deki kostik firmasını ara… yok, aslında perşembe yap."
- Yapılandırılmış önizleme: **Görev** · Kostik firmasını ara · ~~Yarın~~ **Perşembe 12 Kas** · 10:00 · → Kostik tedarik
- Süre: 0:07 · ipucu: "Düzeltmek için konuşmaya devam et" · butonlar: İptal · Gönder

### 5.5 Not detayı (09:12 kaydı)
- Başlık (ilk satır): **Kostik tedarikçisini ara – perşembe**
- Gövde: "25 kg'lık torba fiyatını ve teslim süresini sor. Geçen teklif 1.850 ₺ idi; İzmir deposundan teslim olursa nakliye düşer."
- Meta: Ana Günlük · Bugün 09:12 · Yazı · İşlenmedi
- Öneriler (her biri Kabul / Ret): Görev · Perşembe 12 Kasım 10:00 · Konu → Kostik tedarik · Kişi: Ayhan Kimya – Selin Hanım · Etiket #tedarik
- Görev alanları: Tarih: Perşembe, 12 Kasım · Saat: 10:00 · Öncelik: Yüksek · Tekrar: Yok · Hatırlatma: 1 saat önce
- Bağlantılı: "Ayhan Kimya teklifi: NaOH %99 – Salı, 3 Kasım"
- Ana eylem: **Görev olarak kaydet** · ikincil: Konuya taşı · Hatırlat · Paylaş · Arşivle (silme yok; arşiv)

---

## 6. ui-designer kontrol listesi (her yön için)
- [ ] Yalnız kendi yönünün değerleri; değişken adları §1.4 ile birebir.
- [ ] 5 ekran × açık + koyu = 10 çerçeve (412×915), yan yana; A için 6. çerçeve serbest (sheet %92), C için geçiş anı çerçevesi serbest.
- [ ] `<html lang="tr">`, fontlar Google Fonts'tan (B: Inter/Fragment Mono `text=₺` yedeği).
- [ ] AI ✦ = inline SVG; `✦` karakteri yok. İkon seti yönün setiyle uyumlu; marka ikonu taklidi yok.
- [ ] Yay eğrileri §1.5'teki `linear()` dizgileri; `prefers-reduced-motion` kuralı var.
- [ ] Dokunma hedefi ≥44px; metin kontrastı Ek'teki tabloyla uyumlu (yeni çift eklersen hesapla, 4,5:1 altını kullanma).
- [ ] Taşma yok: "Değer akışı haritalandırması" kesilir, "Hatırlatma" sekmesi sığar, `İŞLENMEDİ` doğru büyük harf.
- [ ] Lorem ipsum yok; §5 içeriği.

## 7. Seçimden sonra (bu turda YAPILMAZ)
Kullanıcı yönü (veya karışımı) seçince: `design/tokens.json` (W3C Design Tokens formatı: `$value`/`$type`; nötr 50–950, semantik açık/koyu, tipografi, 4px boşluk, radius, gölge, yay parametreleri `k/c` + `linear()` karşılığı) ve kısa `design/guidelines.md` yazılır. Semantik renkler o aşamada 4 tondan tam 50–950 ölçeğe genişletilir.

---

## Ek — Kontrast denetimi (WCAG 2.x göreli parlaklık formülüyle hesaplandı)
Gövde metni hedefi 4,5:1; grafik öğeler (nokta, form kenarı) 3:1. Tüm 66 çift geçiyor. En dar paylar: A "ikincil metin / gömük yüzey" 4,91 · C "başarı / soluk başarı zemini" 4,79 · C "hata / soluk hata zemini" 4,75 — bu çiftlerde metni küçültme, ince ağırlık (300) kullanma.
Not: Dekoratif kenarlar (`--color-border`: balon çizgisi, satır ayıracı) ve çip dolgusu bilgi taşımaz; çipin tıklanabilirliği ✦ ikonu + metinle anlaşılır. Bilgi taşıyan sınırlar (`--color-border-strong`: yazma alanı, odak) 3:1'i geçer.

### Yön A — Taş Defter
| Çift | Açık (ön/zemin) | Oran | Koyu (ön/zemin) | Oran | Hedef |
|---|---|---|---|---|---|
| Ana metin / zemin | `#26231F` / `#FAF8F5` | **14.76** ✓ | `#F3F0EB` / `#171512` | **16.03** ✓ | 4.5:1 |
| Ana metin / kayıt yüzeyi | `#26231F` / `#FFFFFF` | **15.64** ✓ | `#F3F0EB` / `#26231F` | **13.76** ✓ | 4.5:1 |
| İkincil metin (saat, önizleme) / zemin | `#6F675A` / `#FAF8F5` | **5.26** ✓ | `#B3AA9C` / `#171512` | **7.94** ✓ | 4.5:1 |
| İkincil metin / kayıt yüzeyi | `#6F675A` / `#FFFFFF` | **5.58** ✓ | `#B3AA9C` / `#26231F` | **6.81** ✓ | 4.5:1 |
| İkincil metin / sheet | `#6F675A` / `#FFFFFF` | **5.58** ✓ | `#B3AA9C` / `#26231F` | **6.81** ✓ | 4.5:1 |
| AI çip metni / çip zemini | `#554F45` / `#F3F0EB` | **7.13** ✓ | `#D5CEC3` / `#3A3530` | **7.76** ✓ | 4.5:1 |
| Ana buton metni / ana buton | `#FAF8F5` / `#26231F` | **14.76** ✓ | `#171512` / `#F3F0EB` | **16.03** ✓ | 4.5:1 |
| Pasif sekme etiketi / sekme çubuğu | `#6F675A` / `#FAF8F5` | **5.26** ✓ | `#B3AA9C` / `#171512` | **7.94** ✓ | 4.5:1 |
| Yer tutucu (placeholder) / yazma alanı | `#6F675A` / `#FFFFFF` | **5.58** ✓ | `#B3AA9C` / `#26231F` | **6.81** ✓ | 4.5:1 |
| İşlenmemiş noktası / kayıt (grafik, ≥3:1) | `#3A3530` / `#FFFFFF` | **12.13** ✓ | `#E7E2DA` / `#26231F` | **12.13** ✓ | 3.0:1 |
| Ana metin / gömük yüzey (plan kartı, alan) | `#26231F` / `#F3F0EB` | **13.76** ✓ | `#F3F0EB` / `#3A3530` | **10.67** ✓ | 4.5:1 |
| İkincil metin / gömük yüzey | `#6F675A` / `#F3F0EB` | **4.91** ✓ | `#B3AA9C` / `#3A3530` | **5.28** ✓ | 4.5:1 |
| Etkin sekme / etkin sekme göstergesi | `#26231F` / `#E7E2DA` | **12.13** ✓ | `#FAF8F5` / `#3A3530` | **11.44** ✓ | 4.5:1 |
| Form/çip kenarı / yüzey (grafik, ≥3:1) | `#8E8576` / `#FFFFFF` | **3.64** ✓ | `#8E8576` / `#26231F` | **4.3** ✓ | 3.0:1 |
| Başarı metni / zemin | `#3F6B3A` / `#FAF8F5` | **5.87** ✓ | `#9CC48F` / `#171512` | **9.29** ✓ | 4.5:1 |
| Başarı metni / Başarı soluk zemin | `#3F6B3A` / `#E8EFE3` | **5.3** ✓ | `#9CC48F` / `#1F2A1C` | **7.61** ✓ | 4.5:1 |
| Uyarı metni / zemin | `#855512` / `#FAF8F5` | **5.99** ✓ | `#E0B46A` / `#171512` | **9.46** ✓ | 4.5:1 |
| Uyarı metni / Uyarı soluk zemin | `#855512` / `#F6EBD9` | **5.39** ✓ | `#E0B46A` / `#2E2414` | **7.91** ✓ | 4.5:1 |
| Hata metni / zemin | `#A33A2C` / `#FAF8F5` | **6.19** ✓ | `#F09A8B` / `#171512` | **8.42** ✓ | 4.5:1 |
| Hata metni / Hata soluk zemin | `#A33A2C` / `#F7E3DF` | **5.32** ✓ | `#F09A8B` / `#33191A` | **7.49** ✓ | 4.5:1 |
| Bilgi metni / zemin | `#2F5F7A` / `#FAF8F5` | **6.52** ✓ | `#8FC0DB` / `#171512` | **9.31** ✓ | 4.5:1 |
| Bilgi metni / Bilgi soluk zemin | `#2F5F7A` / `#E1ECF2` | **5.75** ✓ | `#8FC0DB` / `#172630` | **7.91** ✓ | 4.5:1 |

### Yön B — Çelik Pano
| Çift | Açık (ön/zemin) | Oran | Koyu (ön/zemin) | Oran | Hedef |
|---|---|---|---|---|---|
| Ana metin / zemin | `#1B1F25` / `#FFFFFF` | **16.54** ✓ | `#EEF1F4` / `#0E1115` | **16.69** ✓ | 4.5:1 |
| Ana metin / kayıt yüzeyi | `#1B1F25` / `#FFFFFF` | **16.54** ✓ | `#EEF1F4` / `#0E1115` | **16.69** ✓ | 4.5:1 |
| İkincil metin (saat, önizleme) / zemin | `#56606D` / `#FFFFFF` | **6.38** ✓ | `#9AA4B1` / `#0E1115` | **7.5** ✓ | 4.5:1 |
| İkincil metin / kayıt yüzeyi | `#56606D` / `#FFFFFF` | **6.38** ✓ | `#9AA4B1` / `#0E1115` | **7.5** ✓ | 4.5:1 |
| İkincil metin / sheet | `#56606D` / `#F7F8FA` | **6.01** ✓ | `#9AA4B1` / `#1B1F25` | **6.55** ✓ | 4.5:1 |
| AI çip metni / çip zemini | `#424A55` / `#FFFFFF` | **8.97** ✓ | `#C3CAD3` / `#0E1115` | **11.45** ✓ | 4.5:1 |
| Ana buton metni / ana buton | `#FFFFFF` / `#1B1F25` | **16.54** ✓ | `#0E1115` / `#EEF1F4` | **16.69** ✓ | 4.5:1 |
| Pasif sekme etiketi / sekme çubuğu | `#56606D` / `#FFFFFF` | **6.38** ✓ | `#9AA4B1` / `#0E1115` | **7.5** ✓ | 4.5:1 |
| Yer tutucu (placeholder) / yazma alanı | `#56606D` / `#F7F8FA` | **6.01** ✓ | `#9AA4B1` / `#1B1F25` | **6.55** ✓ | 4.5:1 |
| İşlenmemiş noktası / kayıt (grafik, ≥3:1) | `#424A55` / `#FFFFFF` | **8.97** ✓ | `#C3CAD3` / `#0E1115` | **11.45** ✓ | 3.0:1 |
| Ana metin / gömük yüzey (plan kartı, alan) | `#1B1F25` / `#F7F8FA` | **15.57** ✓ | `#EEF1F4` / `#1B1F25` | **14.59** ✓ | 4.5:1 |
| İkincil metin / gömük yüzey | `#56606D` / `#F7F8FA` | **6.01** ✓ | `#9AA4B1` / `#1B1F25` | **6.55** ✓ | 4.5:1 |
| Etkin sekme / etkin sekme göstergesi | `#1B1F25` / `#FFFFFF` | **16.54** ✓ | `#EEF1F4` / `#0E1115` | **16.69** ✓ | 4.5:1 |
| Form/çip kenarı / yüzey (grafik, ≥3:1) | `#727D8B` / `#FFFFFF` | **4.18** ✓ | `#727D8B` / `#0E1115` | **4.53** ✓ | 3.0:1 |
| Başarı metni / zemin | `#1E7449` / `#FFFFFF` | **5.75** ✓ | `#6FD3A0` / `#0E1115` | **10.36** ✓ | 4.5:1 |
| Başarı metni / Başarı soluk zemin | `#1E7449` / `#E3F4EA` | **5.04** ✓ | `#6FD3A0` / `#0F2A1E` | **8.39** ✓ | 4.5:1 |
| Uyarı metni / zemin | `#8A5A08` / `#FFFFFF` | **5.92** ✓ | `#F2C060` / `#0E1115` | **11.25** ✓ | 4.5:1 |
| Uyarı metni / Uyarı soluk zemin | `#8A5A08` / `#FBF0DA` | **5.24** ✓ | `#F2C060` / `#2B2110` | **9.4** ✓ | 4.5:1 |
| Hata metni / zemin | `#B9342A` / `#FFFFFF` | **5.85** ✓ | `#FF8F84` / `#0E1115` | **8.57** ✓ | 4.5:1 |
| Hata metni / Hata soluk zemin | `#B9342A` / `#FCE6E4` | **4.9** ✓ | `#FF8F84` / `#33161A` | **7.49** ✓ | 4.5:1 |
| Bilgi metni / zemin | `#1F60B2` / `#FFFFFF` | **6.22** ✓ | `#8CB8F5` / `#0E1115` | **9.27** ✓ | 4.5:1 |
| Bilgi metni / Bilgi soluk zemin | `#1F60B2` / `#E3EEFB` | **5.3** ✓ | `#8CB8F5` / `#11223A` | **7.83** ✓ | 4.5:1 |

### Yön C — Saf Mürekkep
| Çift | Açık (ön/zemin) | Oran | Koyu (ön/zemin) | Oran | Hedef |
|---|---|---|---|---|---|
| Ana metin / zemin | `#0A0A0A` / `#F4F4F4` | **18.0** ✓ | `#FAFAFA` / `#000000` | **20.12** ✓ | 4.5:1 |
| Ana metin / kayıt yüzeyi | `#0A0A0A` / `#FFFFFF` | **19.8** ✓ | `#FAFAFA` / `#171717` | **17.18** ✓ | 4.5:1 |
| İkincil metin (saat, önizleme) / zemin | `#5C5C5C` / `#F4F4F4` | **6.08** ✓ | `#A3A3A3` / `#000000` | **8.33** ✓ | 4.5:1 |
| İkincil metin / kayıt yüzeyi | `#5C5C5C` / `#FFFFFF` | **6.69** ✓ | `#A3A3A3` / `#171717` | **7.11** ✓ | 4.5:1 |
| İkincil metin / sheet | `#5C5C5C` / `#FFFFFF` | **6.69** ✓ | `#A3A3A3` / `#171717` | **7.11** ✓ | 4.5:1 |
| AI çip metni / çip zemini | `#5C5C5C` / `#FFFFFF` | **6.69** ✓ | `#A3A3A3` / `#171717` | **7.11** ✓ | 4.5:1 |
| Ana buton metni / ana buton | `#FFFFFF` / `#0A0A0A` | **19.8** ✓ | `#0A0A0A` / `#FAFAFA` | **18.97** ✓ | 4.5:1 |
| Pasif sekme etiketi / sekme çubuğu | `#5C5C5C` / `#FFFFFF` | **6.69** ✓ | `#A3A3A3` / `#000000` | **8.33** ✓ | 4.5:1 |
| Yer tutucu (placeholder) / yazma alanı | `#5C5C5C` / `#F4F4F4` | **6.08** ✓ | `#A3A3A3` / `#171717` | **7.11** ✓ | 4.5:1 |
| İşlenmemiş noktası / kayıt (grafik, ≥3:1) | `#0A0A0A` / `#FFFFFF` | **19.8** ✓ | `#FAFAFA` / `#171717` | **17.18** ✓ | 3.0:1 |
| Ana metin / gömük yüzey (plan kartı, alan) | `#0A0A0A` / `#F4F4F4` | **18.0** ✓ | `#FAFAFA` / `#171717` | **17.18** ✓ | 4.5:1 |
| İkincil metin / gömük yüzey | `#5C5C5C` / `#F4F4F4` | **6.08** ✓ | `#A3A3A3` / `#171717` | **7.11** ✓ | 4.5:1 |
| Etkin sekme / etkin sekme göstergesi | `#FFFFFF` / `#0A0A0A` | **19.8** ✓ | `#0A0A0A` / `#FAFAFA` | **18.97** ✓ | 4.5:1 |
| Form/çip kenarı / yüzey (grafik, ≥3:1) | `#767676` / `#FFFFFF` | **4.54** ✓ | `#767676` / `#171717` | **3.95** ✓ | 3.0:1 |
| Başarı metni / zemin | `#157A3B` / `#F4F4F4` | **4.92** ✓ | `#5BD68A` / `#000000` | **11.43** ✓ | 4.5:1 |
| Başarı metni / Başarı soluk zemin | `#157A3B` / `#E2F6E8` | **4.79** ✓ | `#5BD68A` / `#0B2414` | **8.95** ✓ | 4.5:1 |
| Uyarı metni / zemin | `#8F5400` / `#F4F4F4` | **5.56** ✓ | `#FFC14D` / `#000000` | **12.99** ✓ | 4.5:1 |
| Uyarı metni / Uyarı soluk zemin | `#8F5400` / `#FFF1D6` | **5.47** ✓ | `#FFC14D` / `#2A1D00` | **10.19** ✓ | 4.5:1 |
| Hata metni / zemin | `#C42A20` / `#F4F4F4` | **5.16** ✓ | `#FF7A70` / `#000000` | **8.27** ✓ | 4.5:1 |
| Hata metni / Hata soluk zemin | `#C42A20` / `#FFE5E2` | **4.75** ✓ | `#FF7A70` / `#330F0C` | **6.81** ✓ | 4.5:1 |
| Bilgi metni / zemin | `#1B5AC7` / `#F4F4F4` | **5.73** ✓ | `#7EA8FF` / `#000000` | **8.93** ✓ | 4.5:1 |
| Bilgi metni / Bilgi soluk zemin | `#1B5AC7` / `#E3ECFF` | **5.31** ✓ | `#7EA8FF` / `#0D1A38` | **7.31** ✓ | 4.5:1 |
