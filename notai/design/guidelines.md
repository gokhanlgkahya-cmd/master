# NotAI — Tasarım Kuralları

> Yazan: design-director · Tarih: 2026-11-06 · Dayanak: `design/decision.md` (bağlayıcı karışım), `design/review.md`, `design/directions/plan.md` (Yön A + ortak bölümler)
> Dosyalar: `design/tokens.json` (**tek kaynak**) → `design/tokens.css` (**üretilir, elle düzenlenmez**) → bu kurallar.

---

## 1. İlkeler
1. **Ekranda yalnız o anki iş.** Her ekranın tek bir ana eylemi var. Akışta bu eylem mikrofon ya da yazma, sheet'te "Görev olarak kaydet". Mürekkep dolgulu (koyu, dolu) öğe ekranda en fazla bir tane olur. Sayaç rozeti bunun dışındadır.
2. **Kullanıcının kaydı önde, AI geride.** AI çipleri soluk, şeffaf ve kesikli kenarlıdır. Hiçbir zaman renkli, dolgulu ya da kalın olmaz.
3. **Renk = anlam.** Palet sıcak taş grisidir (nötr). Renkli tonlar yalnız durum bildirir: hata (gecikmiş iş, silme uyarısı), uyarı (bugün veya yakın son tarih), başarı (tamamlandı, kısa kabul geri bildirimi), bilgi (senkron, çevrimdışı).
4. **Ferah.** Ekranda 4–6 kayıt görünür. Meta bilgi (saat, konu) küçük ve soluktur.
5. **Kodda sabit değer yok.** Renk, boşluk, köşe, gölge, süre ve katman hep `var(--…)` ile token'dan gelir. Izgara dışı bir değer gerekiyorsa önce `tokens.json`'a eklenir.
6. **Açık ve koyu tema eşit kalitede.** Her yeni bileşen iki temada da denetlenir.

## 2. Token'lar ve adlandırma

### 2.1 Akış
`design/tokens.json` W3C DTCG biçimindedir (her token `{"$value", "$type"}`; `$type` gruptan miras alınır). Değişiklik **yalnız** bu dosyada yapılır. Ardından `tokens.css` yeniden üretilir.
- Faz 1'de üretici, oturum çalışma alanındaki geçici bir Python betiğidir. **Faz 3'te `crates/ui/build.rs` aynı kuralları Rust'ta uygular** (`serde_json`). CI, üretilen dosya ile depodaki dosyanın aynı olduğunu denetler (`git diff --exit-code`).
- Tema seçimi: `<html lang="tr" data-theme="light|dark">`. Öznitelik yoksa sistem ayarı geçerlidir (Ayarlar → Sistem / Açık / Koyu).

### 2.2 JSON → CSS dönüşüm kuralları (üretici ve `build.rs` için sözleşme)
| # | Kural | Örnek |
|---|---|---|
| 1 | Yol parçaları `-` ile birleşir, başa `--` eklenir. camelCase parça kebab-case'e çevrilir. | `space.4` → `--space-4` · `font.lineHeight.md` → `--font-line-height-md` |
| 2 | `color.theme.<mod>.<rol>` → `--color-<rol>`. `light` değeri `:root`'a, `dark` değeri koyu seçicilere yazılır. | `color.theme.light.text` → `--color-text` |
| 3 | `shadow.<mod>.<ad>` → `--shadow-<ad>` (kural 2'deki gibi moda göre). | `shadow.dark.sheet` → `--shadow-sheet` |
| 4 | `$extensions."app.notai".emit: false` taşıyan grup CSS'e çıkmaz. Takma ad (alias) için yine çözülür. | `color.semantic.*` (yalnız `color.theme` üzerinden kullanılır) |
| 5 | Takma ad `{yol}` sona kadar çözülür, CSS'e son değer yazılır. Kaynak yolu yorum olarak eklenir. Döngü veya çözülemeyen ad varsa üretim durur. | `--color-bubble: #FFFFFF; /* {color.theme.light.surface} */` |
| 6 | Biçimler: `color` → `#RRGGBB(AA)` · `dimension`/`duration` `{value,unit}` → `12px`/`370ms` · `cubicBezier` → `cubic-bezier(…)` · `fontFamily` dizisi → virgüllü liste (genel aileler tırnaksız) · `shadow` (tek veya dizi) → `[inset] x y blur spread renk` · `number` → sayı, `cssUnit` uzantısı varsa birimle (`-0.01em`) · `string`/`strokeStyle` → olduğu gibi. | |
| 7 | Açık ve koyu rol kümeleri birebir aynı olmalı. Biri eksikse üretim durur. | |
| 8 | Çıktı sırası: `:root { color-scheme: light; ortak + açık }` → `@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) { koyu } }` → `:root[data-theme="dark"] { koyu }` → `@media (prefers-reduced-motion: reduce) { :root { reducedMotion değerleri } }`. | |
| 9 | `$extensions."app.notai".reducedMotion` (takma ad veya değer) azaltılmış hareket bloğunda o değişkenin yerine geçer. | `--motion-spring-panel-duration: 120ms` |

DTCG notları: Renkler hex dizgisi olarak tutulur. `linear()` dizgisi ve `tabular-nums` gibi CSS ifadelerinin DTCG'de karşılığı olmadığı için `$type: "string"` kullanılır. `em` birimi DTCG `dimension` tipinde tanımlı değil. Bu yüzden harf aralığı `number` + `cssUnit: "em"` ile tutulur.

### 2.3 Prototip adlarından geçiş (ui-designer için)
`--text-md` → `--font-size-md` · `--leading-body` → `--font-line-height-body` · `--weight-*` → `--font-weight-*` · `--tracking-*` → `--font-letter-spacing-*` · `--color-ink` → `--color-accent` · `--color-on-ink` → `--color-on-accent` · `--color-focus` → `--color-focus-ring` · `--color-unprocessed` → `--color-unprocessed-dot` · `--radius-pill` → `--radius-full` · `--shadow-1/2/sheet` → `--shadow-raised/floating/sheet` · `--ease-spring-panel`/`--dur-spring-panel` → `--motion-spring-panel-easing`/`-duration` · `--ease-spring-pop` → `--motion-spring-small-easing` · `--size-touch` → `--size-touch-min` · `--font-mono` **kaldırıldı**.

### 2.4 Tipografi rolleri (Manrope, tek aile)
| Rol | Boyut / satır | Ağırlık | Kullanım |
|---|---|---|---|
| `xs` | 12 / 16 | 500 | Sekme etiketi, meta satırı (saat · konu), rozet |
| `sm` | 14 / 20 | 500 | AI çipi, konu önizlemesi, gün ayracı (600) |
| `md` | 16 / 24 | 400 | Kayıt metni, yazma alanı, liste satırı |
| `lg` | 19 / 28 | 600 | Konu adı, sheet başlığı, plan kartı başlığı (`letter-spacing: tight`) |
| `xl` | 23 / 32 | 700 | Ekran başlığı ("Günlük") |
| `2xl` | 28 / 36 | 700 | Sabah planındaki büyük sayı |

Ölçek dışı boyut (13, 15, 17px…) ve 12px altı yasak. Saat, sayaç, rozet ve tutarlarda `font-variant-numeric: var(--font-feature-tabular)` kullanılır (rakamlar eşit genişlikte olur, alt alta hizalanır). Mono font yoktur.

## 3. Bileşen kuralları

**Kayıt balonu** (akıştaki her kayıt)
- Sağa yaslıdır, kuyruğu yoktur. Genişliği akış genişliğinin en fazla `--size-bubble-max` katıdır (%86). Köşe `--radius-md`. Zemin `--color-bubble`, kenar 1px `--color-bubble-border`, gölge `--shadow-raised`. İç dolgu `--space-3` (yatay) ve `--space-2` (dikey). Metin `md`.
- **Meta satırı balonun üstündedir**, tek satır ve sağa hizalıdır. Saat (tabular-nums) gelir, konuya doğrudan girilmiş kayıtta ardından `·` + 16px konu ikonu + konu adı. Boyut `xs`, renk `--color-meta`. Saat balonun içine konmaz (WhatsApp'tan ayrışma).
- **İşlenmemiş noktası sabit sol oluktadır.** Oluk akışın sol kenarında, `--size-gutter` (16px) genişliğindedir. 8px nokta `--color-unprocessed-dot` renktedir ve balonun ilk satırıyla aynı yüksekliğe hizalanır. Böylece tüm noktalar aynı dikey çizgide durur. Ekran okuyucu metni "İşlenmedi" olur.
- Aralıklar: aynı dakika grubundaki balonlar arası `--space-2`, gruplar arası `--space-4`. Gün ayracının ("Dün", "Çarşamba, 4 Kasım") üstünde ve altında `--space-6` boşluk bırakılır.
- Balona dokunmak not detayı sheet'ini açar. Uzun basınca menü açılır: Konuya taşı ↪ · Kopyala · Arşivle. Silme yok, önce Arşiv.

**AI öneri çipi**
- Çip balonun altında, sağa yaslı ve satır kaydırmalı bir sırada durur. Çipler arası yatay boşluk `--space-2`, satırlar arası `--space-3`, böylece 44px dokunma alanları çakışmaz.
- Hap biçimlidir (`--radius-full`). Görünür yüksekliği `--size-chip` (32px), dokunma alanı 44px'tir. Zemin `--color-chip-bg` (şeffaf), kenar 1px `--border-style-suggested` (kesikli) `--color-chip-border`. Metin `sm` ve `--color-chip-text`. Solda 16px ✦ (inline SVG) vardır.
- Her çip iki parçalıdır: **gövde** önerinin kabulüdür, **sağdaki ×** yalnız o çipi yoksayar (`aria-label="Yoksay: Görev · Perşembe"`, en az 44px genişlik). Toplu ret düğmesi yok.
- Kabul edilince kenar `--border-style-accepted` (düz çizgi) olur, `--motion-spring-small` ile `scale(1 → 1.06 → 1)` çalışır, ✦ yerini `--color-accept` ✓ işaretine bırakır. `--motion-delay-chip-merge` (1 sn) sonra çip meta satırındaki konu etiketine taşınır. Yoksayılan çip `--motion-duration-close` sürede solar.
- Çipler `<button>` olarak yazılır. Hiçbir durumda semantik renk ya da mürekkep dolgu almaz.

**Bottom sheet = not detayı** (alttan kayan sayfa)
- Sheet'in iki durağı var:
  - `--size-sheet-peek` (%55), **göz at**: tam metin, ardından her öneri için bir satır. Satırın solunda ✦ + öneri, sağında 44px **✓** (`--color-accept`) ve **×** (`--color-dismiss`) düğmeleri. Altta sabit tek ana düğme: **"Görev olarak kaydet"** (mürekkep).
  - `--size-sheet-full` (%92), **düzenle**: tam ekran düzeni. Görev alanları (tarih · saat · öncelik · hatırlatma) iki sütunda, altında bağlantılı kayıtlar.
- Görünüm: zemin `--color-sheet-bg`, üst köşeler `--radius-lg`, gölge `--shadow-sheet`, tutamak `--size-handle-*` boyutunda ve `--color-sheet-handle` renkte.
- **Karartma (`--color-scrim`) yalnız sheet'te kullanılır.** Sheet aşağı çekilince, karartmaya dokununca ya da Android geri hareketiyle kapanır. Kapanış `--motion-duration-close` + `--motion-easing-out` ile, zıplamasız olur.
- Duraklar arası geçiş yalnız `transform: translateY` ile yapılır, yükseklik animasyonu yoktur. Sürüklerken parmak bırakılınca yay, `stiffness/damping/mass` token'larıyla ve parmağın hızı korunarak devam eder.
- Sheet modaldır (önündeki iş bitmeden arkaya dokunulmaz): `role="dialog"`, `aria-modal="true"`, odak sheet içinde kalır.

**Ekleme (ataç) paneli**
- Ataca basınca panel **klavyenin yerine** açılır: yükseklik son klavye yüksekliğidir, bilinmiyorsa `--size-attach-panel-min`. **Karartma yoktur**, akış görünür kalır. Ataç ikonu × olur. Zemin `--color-panel-bg`, katman `--z-panel`.
- İki grup halinde ızgara düzeni vardır. Her grubun küçük başlığı `xs`, 600 ağırlık ve soluk renktedir:
  - **Ekle**: Fotoğraf · Kamera · Belge · Konum · Kişi
  - **Oluştur**: Görev · Hatırlatma · Etkinlik · Liste
- Karo: 56px (`--size-tile`) ikon kutusu, `--color-tile-bg` zemin, `--radius-md` köşe. Etiket altında `xs`/`sm`. Karolar `--motion-stagger-tile` arayla aşağıdan belirir.
- Panel modal değildir: odak tuzağı yoktur, yazma alanı odaklanırsa panel kapanır ve klavye açılır.

**Ses şeridi**
- Mikrofona basınca yazma çubuğu **yerinde** yukarı doğru şeride dönüşür (en az `--size-voice-strip-min`, `--motion-spring-panel`). **Arka plan karartılmaz.** Mikrofon "durdur" kare ikonuna döner.
- İçerik:
  - Canlı metin `md` boyutundadır. Son kelimeler `--color-voice-live`, öncekiler `--color-voice-settled` renktedir. Düzeltilen kelimenin üstü çizilir ("~~Yarın~~ Perşembe").
  - Altında **yapılandırılmış önizleme kartı** yer alır (`--color-tile-bg`, `--radius-md`). İkonlu satırlardan oluşur: takvim "Perşembe 12 Kas" · saat "10:00" · klasör "Kostik tedarik". Mono kullanılmaz.
  - Sayaç `0:07` (tabular-nums) ile "İptal" (metin düğme) ve "Gönder" (mürekkep) düğmeleri.
- Canlı metin `aria-live="polite"` olur. Kayıt sürerken ekran kilitlenmez, başka hiçbir öğe gizlenmez.

**Sekme çubuğu**
- 5 sekme vardır: **Günlük · Görevler · Takvim · Hatırlatma · Ara/Sor**. İkon ve etiket her zaman görünür. Yükseklik `--size-tabbar`, zemin `--color-bg`, üstte 1px `--color-border`.
- Etkin sekmede ikonun arkasında 56×32 hap (`--color-tab-indicator`) durur. Hap sekmeler arasında `--motion-spring-small` ile kayar. Etkin etiket `--color-tab-active` renkte ve 700 ağırlıkta, pasif etiket `--color-tab-idle` renkte ve 500 ağırlıktadır.
- **Akış ekranında sekme çubuğu gizlidir.** Akış ana ekranın üstüne sağdan kayarak açılır (`--motion-spring-panel`).

**Yazma çubuğu ve mikrofon**
- Yükseklik `--size-composer`. Sıralama: ataç (44px) · alan · kamera (44px, alanın dışında) · mikrofon.
- Alan 48px yüksekliğindedir: zemin `--color-input-bg`, kenar 1px `--color-input-border`, köşe `--radius-md`, yer tutucu "Ana Günlük’e yaz…".
- Mikrofon 48px, dolu ve yuvarlaktır (`--color-mic-bg`). Alanda yazı varsa aynı yerde aynı boyutta **Gönder** görünür. Gönderilen kayıt `translateY(16px) → 0` ve `scale(.96) → 1` hareketiyle (`--motion-spring-small`) yerine oturur. Çipler `--motion-delay-chips` sonra `--motion-stagger-chip` arayla belirir.

**Diğer**
- **Sabah planı kartı:** sol tarafta, tam genişlikte durur (`--color-plan-bg`, 1px `--color-plan-border`, `--radius-md`, iç dolgu `--space-4`). İçerik: ✦ + "Sabah planı" üst etiketi (`xs`, soluk), başlık `lg`, en çok 3 onay kutulu madde, "Planı onayla" (mürekkep) + "Düzenle" (metin). Gecikme yalnız metinle ve `--color-error` ile belirtilir ("1 gün gecikti").
- **Ana ekran filtresi:** bölmeli kontrol (tek kutuda yan yana seçenekler) `Tümü | Proje | Alan | Kaynak`. Ray `--color-segment-track`, seçili parça `--color-segment-thumb` zemin ve 1px `--color-segment-thumb-border` kenar ile belirtilir. Ana Günlük her zaman en üstte sabittir ve işlenmemiş sayısı rozeti taşır. **Haftalık gözden geçirme** Ana Günlük'ün hemen altında sabit bir satırdır.
- **Alternatif:** Gökhan balon yerine satır görünümü isterse token'ların hiçbiri değişmez, yalnız kayıt bileşeni değişir.

## 4. Hareket kuralları
- **Yalnız `transform` ve `opacity` animasyonu yapılır.** `height`, `width`, `top` ve `margin` animasyonu yasak (Android WebView'de takılır).
- `--motion-spring-panel` (~370ms, %1,6 aşma) büyük yüzeyler içindir: sheet, ekleme paneli, ses şeridi, akış ekranı geçişi.
- `--motion-spring-small` (~424ms, %9 esneme) küçük öğeler içindir: çip kabulü, gönderilen kaydın oturması, sekme hapı, rozet. "Belirgin yay" hissi buradan gelir.
- **Kapanış hep açılıştan kısadır ve zıplamaz:** `--motion-duration-close` (220ms) + `--motion-easing-out`.
- Panel açılınca içerik `--motion-delay-content` gecikmeyle opacity ile gelir. Aynı anda tek büyük yüzey hareket eder.
- JS/WASM ile sürülen yaylar (sürükleme bırakma) `--motion-spring-*-stiffness/damping/mass` değerleriyle hesaplanır. CSS geçişleri `linear()` dizgisini kullanır.
- **Azaltılmış hareket** (`prefers-reduced-motion: reduce`) iki parçalıdır:
  - `tokens.css` yayları otomatik olarak `--motion-reduced-duration` (120ms) + `--motion-reduced-easing` değerlerine düşürür ve kademeleri 0 yapar.
  - Bileşen CSS'i aynı medya sorgusunda keyframe'lerdeki `transform` değerlerini kaldırır. Yalnız opacity değişir.

## 5. Türkçe tipografi
- `<html lang="tr">` zorunlu. CSS `text-transform: uppercase` ancak bu sayede `i → İ`, `ı → I` doğru çevrilir (`İŞLENMEDİ`, `ÇARŞAMBA`).
- **Rust'ta dikkat:** `str::to_uppercase()`/`to_lowercase()` dile duyarsız çalışır: `"i".to_uppercase()` = `"I"` (yanlış). Türkçe metin için `tr_upper`/`tr_lower` yardımcısı yazılır (önce `i↔İ`, `ı↔I` eşlemesi, sonra std) ya da tr yerel ayarıyla `icu_casemap` kullanılır. Arama ve büyük/küçük harf duyarsız karşılaştırma da aynı katlamayla yapılır: "İstanbul" = "istanbul", "IŞIK" = "ışık".
- **Kesme işareti `’` (U+2019)**, düz `'` değil: "Ana Günlük’e", "kg’lık", "Murat Bey’e". Üç nokta `…` (U+2026). Tırnak “ ”.
- **Tire:** aralıkta boşluksuz en tire kullanılır: "4–6 hafta". Cümle içindeki ayraçta tirenin önüne `&nbsp;` konur, tire satır başına düşmez: `ara&nbsp;– perşembe`.
- **Sayı ile birim arasına `&nbsp;` konur:** `25&nbsp;kg`, `1.850&nbsp;₺`. Binlik ayırıcı nokta, ondalık ayırıcı virgüldür. Saat 24 saatlik yazılır (`09:12`). Tarih biçimleri "12 Kas" ve "Perşembe, 12 Kasım". Gün adı cümle içinde küçük, ayraç başlığında büyük harfle başlar.
- Satır yüksekliği en az 1,25 olur. `line-height: 1` yasak, yoksa İ, Ğ ve Ş/Ç kancaları kırpılır.
- Uzun kelimeler `overflow-wrap: break-word` ile kırılır. `hyphens: auto` kullanılmaz (WebView'de Türkçe heceleme sözlüğü güvenilir değil). Her ekranda test dizisi kullanılır: "Ğ ğ Ş ş İ i I ı Ö ö Ü ü Ç ç", "Değer akışı haritalandırması", "İŞLENMEDİ".
- Manrope (OFL) uygulamayla birlikte paketlenir (çevrimdışı). latin + latin-ext alt kümesi, `font-display: swap`. ₺ ve Türkçe harfler Manrope'ta var.

## 6. Erişilebilirlik
- **Dokunma hedefi ≥ 44×44px** (`--size-touch-min`). Görseli küçük olan öğenin (çip 32px, ikon 24px) dokunma alanı dolgu veya `::before` ile genişletilir. Komşu hedefler çakışmaz.
- **Kontrast:** metin ≥ 4,5:1, grafik öğe ve bilgi taşıyan kenar ≥ 3:1 (WCAG 2.x AA). Tüm çiftler §9'da hesaplandı. `--color-text-subtle` yalnız devre dışı öğe ve dekoratif ikonda kullanılır, okunması gereken metinde kullanılmaz. `--color-border` dekoratiftir ve bilgi taşıyamaz.
- **Renk tek başına anlam taşımaz:** gecikme metinle ("1 gün gecikti"), işlenmemiş durum ekran okuyucu etiketiyle, kabul ✓ şekliyle belirtilir.
- **Odak:** `:focus-visible` durumunda 2px `--color-focus-ring` halka çizilir, aradaki boşluk `--border-width-focus-offset`. `outline: none` tek başına yasak.
- **Hareket:** `prefers-reduced-motion` desteklenir (§4).
- **Ekran okuyucu:** yalnız ikondan oluşan düğmede Türkçe `aria-label` bulunur ("Mikrofon", "Ekle", "Gönder"). Çipler `<button>` olur. Sheet `role="dialog"` olur. Ses metni `aria-live` ile okunur. Sekme çubuğunda `aria-current="page"` kullanılır.
- **Yazı büyütme:** Android'de %130 yazı boyutuyla test edilir. Metin kutularına sabit `height` verilmez, `min-height` kullanılır. "Hatırlatma" sekme etiketi bu boyutta sığar.

## 7. Kopya sınırı (yalnız kalıp alınır)
| Kaynak | Alınan kalıp | **Alınmayan** |
|---|---|---|
| WhatsApp | Liste → akış iskeleti, en yeni altta, gün ayracı, sabit yazma çubuğu, ataç paneli, ilet → "Konuya taşı" | Yeşil renk, duvar kâğıdı deseni, balon kuyruğu, balon içinde sağ altta saat, çift tik, ikon seti, logo/ad |
| Saner.AI | Kayıt altında otomatik konu/etiket önerisi, sabah öncelik planı | Renkleri, kart düzeni, adlandırma |
| Todoist (+Ramble) | Konuşurken canlı yapılandırma, "aslında perşembe yap" düzeltmesi, doğal dilde tarih | Kırmızı marka rengi, öncelik bayrağı görünümü, ikonlar |
| Google Keep (+Gemini) | Saniyede not, notlara soru sorma (Ara/Sor) | Renkli kart paleti, Gemini'nin renkli yıldızı, ızgara düzeni |

Ekran görüntüsü, logo ya da marka rengi hiçbir dosyaya girmez. Şüphede kalınan öğe kullanılmaz.

## 8. İkon kuralları
- İkonlar Phosphor "Regular" tarzında (çizgi, yuvarlak uçlu) çizilmiş **kendi setimizdir**. Kaynak olarak açık lisanslı (MIT) Phosphor çizimleri ya da aynı ölçülerde kendi çizimlerimiz kullanılır. Başka setle karıştırılmaz.
- Boyutlar `--icon-size-md` (24px, varsayılan), `-sm` (20px), `-xs` (16px; meta satırı, çip). Çizgi kalınlığı `--icon-stroke` (24px kutuda 1,5). Renk `currentColor` olur, ikon ayrı renk almaz.
- **✦ AI işareti** 4 köşeli, tek renkli, kendi çizimimiz olan bir inline SVG'dir. `✦` karakteri metin olarak kullanılmaz.
- **Ana Günlük'ün ikonu kitap değildir** (defter + kalem). Kitap ikonu Okuma notları'na aittir. Her konu farklı bir ikon taşır, iki konu aynı ikonu paylaşmaz.
- Dekoratif ikon `aria-hidden="true"` olur. Anlam taşıyan tek ikonun düğmesinde etiket bulunur. WhatsApp, Google, Todoist ve Saner ikonları taklit edilmez.

## 9. Kontrast tablosu (tokens.json'dan hesaplandı)
Formül: WCAG 2.x göreli parlaklık, `(L1 + 0,05) / (L2 + 0,05)`. Değerler doğrudan `tokens.json`'daki takma adlar çözülerek hesaplandı. Şeffaf çip zemini için altındaki yüzey kullanıldı.

| Çift | Hedef | Açık (ön / zemin) | Oran | Koyu (ön / zemin) | Oran |
|---|:-:|---|:-:|---|:-:|
| Ana metin / zemin | 4.5:1 | `#26231F` / `#FAF8F5` | **14.76** ✓ | `#F3F0EB` / `#171512` | **16.03** ✓ |
| Ana metin / balon | 4.5:1 | `#26231F` / `#FFFFFF` | **15.64** ✓ | `#F3F0EB` / `#26231F` | **13.76** ✓ |
| Ana metin / gömük yüzey (plan kartı) | 4.5:1 | `#26231F` / `#F3F0EB` | **13.76** ✓ | `#F3F0EB` / `#3A3530` | **10.67** ✓ |
| Ana metin / sheet | 4.5:1 | `#26231F` / `#FFFFFF` | **15.64** ✓ | `#F3F0EB` / `#26231F` | **13.76** ✓ |
| Soluk metin / zemin (saat, gün ayracı) | 4.5:1 | `#6F675A` / `#FAF8F5` | **5.26** ✓ | `#B3AA9C` / `#171512` | **7.94** ✓ |
| Soluk metin / balon · panel | 4.5:1 | `#6F675A` / `#FFFFFF` | **5.58** ✓ | `#B3AA9C` / `#26231F` | **6.81** ✓ |
| Soluk metin / gömük yüzey | 4.5:1 | `#6F675A` / `#F3F0EB` | **4.91** ✓ | `#B3AA9C` / `#3A3530` | **5.28** ✓ |
| Meta satırı (saat · konu) / zemin | 4.5:1 | `#6F675A` / `#FAF8F5` | **5.26** ✓ | `#B3AA9C` / `#171512` | **7.94** ✓ |
| Yer tutucu / yazma alanı | 4.5:1 | `#6F675A` / `#FFFFFF` | **5.58** ✓ | `#B3AA9C` / `#26231F` | **6.81** ✓ |
| AI çip metni / zemin (çip şeffaf) | 4.5:1 | `#554F45` / `#FAF8F5` | **7.65** ✓ | `#D5CEC3` / `#171512` | **11.67** ✓ |
| AI çip metni / sheet (çip şeffaf) | 4.5:1 | `#554F45` / `#FFFFFF` | **8.11** ✓ | `#D5CEC3` / `#26231F` | **10.01** ✓ |
| AI çip metni / çip basılı | 4.5:1 | `#554F45` / `#F3F0EB` | **7.13** ✓ | `#D5CEC3` / `#3A3530` | **7.76** ✓ |
| Mürekkep düğme metni / mürekkep | 4.5:1 | `#FAF8F5` / `#26231F` | **14.76** ✓ | `#171512` / `#F3F0EB` | **16.03** ✓ |
| Mürekkep düğme metni / basılı | 4.5:1 | `#FAF8F5` / `#3A3530` | **11.44** ✓ | `#171512` / `#E7E2DA` | **14.13** ✓ |
| Rozet sayısı / rozet | 4.5:1 | `#FAF8F5` / `#26231F` | **14.76** ✓ | `#171512` / `#F3F0EB` | **16.03** ✓ |
| Pasif sekme / zemin | 4.5:1 | `#6F675A` / `#FAF8F5` | **5.26** ✓ | `#B3AA9C` / `#171512` | **7.94** ✓ |
| Etkin sekme / sekme hapı | 4.5:1 | `#26231F` / `#E7E2DA` | **12.13** ✓ | `#FAF8F5` / `#3A3530` | **11.44** ✓ |
| Bölmeli kontrol: pasif / ray | 4.5:1 | `#6F675A` / `#F3F0EB` | **4.91** ✓ | `#B3AA9C` / `#3A3530` | **5.28** ✓ |
| Bölmeli kontrol: seçili / başparmak | 4.5:1 | `#26231F` / `#FFFFFF` | **15.64** ✓ | `#F3F0EB` / `#26231F` | **13.76** ✓ |
| Ses şeridi: önceki kelimeler / panel | 4.5:1 | `#6F675A` / `#FFFFFF` | **5.58** ✓ | `#B3AA9C` / `#26231F` | **6.81** ✓ |
| Ses şeridi: canlı kelimeler / panel | 4.5:1 | `#26231F` / `#FFFFFF` | **15.64** ✓ | `#F3F0EB` / `#26231F` | **13.76** ✓ |
| Ekleme karosu ikonu / karo | 3:1 | `#26231F` / `#F3F0EB` | **13.76** ✓ | `#F3F0EB` / `#3A3530` | **10.67** ✓ |
| Başarı / zemin | 4.5:1 | `#3F6B3A` / `#FAF8F5` | **5.87** ✓ | `#9CC48F` / `#171512` | **9.29** ✓ |
| Başarı / yüzey | 4.5:1 | `#3F6B3A` / `#FFFFFF` | **6.22** ✓ | `#9CC48F` / `#26231F` | **7.97** ✓ |
| Başarı / soluk başarı | 4.5:1 | `#3F6B3A` / `#E8EFE3` | **5.30** ✓ | `#9CC48F` / `#1F2A1C` | **7.61** ✓ |
| Uyarı / zemin | 4.5:1 | `#855512` / `#FAF8F5` | **5.99** ✓ | `#E0B46A` / `#171512` | **9.46** ✓ |
| Uyarı / yüzey | 4.5:1 | `#855512` / `#FFFFFF` | **6.35** ✓ | `#E0B46A` / `#26231F` | **8.12** ✓ |
| Uyarı / soluk uyarı | 4.5:1 | `#855512` / `#F6EBD9` | **5.39** ✓ | `#E0B46A` / `#2E2414` | **7.91** ✓ |
| Hata / zemin | 4.5:1 | `#A33A2C` / `#FAF8F5` | **6.19** ✓ | `#F09A8B` / `#171512` | **8.42** ✓ |
| Hata / yüzey | 4.5:1 | `#A33A2C` / `#FFFFFF` | **6.56** ✓ | `#F09A8B` / `#26231F` | **7.22** ✓ |
| Hata / soluk hata | 4.5:1 | `#A33A2C` / `#F7E3DF` | **5.32** ✓ | `#F09A8B` / `#33191A` | **7.49** ✓ |
| Bilgi / zemin | 4.5:1 | `#2F5F7A` / `#FAF8F5` | **6.52** ✓ | `#8FC0DB` / `#171512` | **9.31** ✓ |
| Bilgi / yüzey | 4.5:1 | `#2F5F7A` / `#FFFFFF` | **6.91** ✓ | `#8FC0DB` / `#26231F` | **7.99** ✓ |
| Bilgi / soluk bilgi | 4.5:1 | `#2F5F7A` / `#E1ECF2` | **5.75** ✓ | `#8FC0DB` / `#172630` | **7.91** ✓ |
| Ana metin / soluk hata (banner gövdesi) | 4.5:1 | `#26231F` / `#F7E3DF` | **12.67** ✓ | `#F3F0EB` / `#33191A` | **14.27** ✓ |
| Kabul ✓ / sheet | 3:1 | `#3F6B3A` / `#FFFFFF` | **6.22** ✓ | `#9CC48F` / `#26231F` | **7.97** ✓ |
| Yoksay × / sheet | 3:1 | `#6F675A` / `#FFFFFF` | **5.58** ✓ | `#B3AA9C` / `#26231F` | **6.81** ✓ |
| AI çip kenarı / zemin | 3:1 | `#8E8576` / `#FAF8F5` | **3.44** ✓ | `#8E8576` / `#171512` | **5.00** ✓ |
| AI çip kenarı / sheet | 3:1 | `#8E8576` / `#FFFFFF` | **3.64** ✓ | `#8E8576` / `#26231F` | **4.30** ✓ |
| İşlenmemiş noktası / zemin (sol oluk) | 3:1 | `#3A3530` / `#FAF8F5` | **11.44** ✓ | `#E7E2DA` / `#171512` | **14.13** ✓ |
| Mikrofon / zemin | 3:1 | `#26231F` / `#FAF8F5` | **14.76** ✓ | `#F3F0EB` / `#171512` | **16.03** ✓ |
| Yazma alanı kenarı / zemin | 3:1 | `#8E8576` / `#FAF8F5` | **3.44** ✓ | `#8E8576` / `#171512` | **5.00** ✓ |
| Form kenarı / sheet | 3:1 | `#8E8576` / `#FFFFFF` | **3.64** ✓ | `#8E8576` / `#26231F` | **4.30** ✓ |
| Bölmeli kontrol başparmak kenarı / ray | 3:1 | `#8E8576` / `#F3F0EB` | **3.20** ✓ | `#8E8576` / `#3A3530` | **3.33** ✓ |
| Sheet tutamağı / sheet | 3:1 | `#8E8576` / `#FFFFFF` | **3.64** ✓ | `#8E8576` / `#26231F` | **4.30** ✓ |
| Odak halkası / zemin | 3:1 | `#26231F` / `#FAF8F5` | **14.76** ✓ | `#F3F0EB` / `#171512` | **16.03** ✓ |
| Odak halkası / sheet | 3:1 | `#26231F` / `#FFFFFF` | **15.64** ✓ | `#F3F0EB` / `#26231F` | **13.76** ✓ |
| Başarı kenarı / yüzey | 3:1 | `#658961` / `#FFFFFF` | **3.96** ✓ | `#6D8964` / `#26231F` | **4.03** ✓ |
| Uyarı kenarı / yüzey | 3:1 | `#9D7741` / `#FFFFFF` | **4.08** ✓ | `#9D7E4A` / `#26231F` | **4.11** ✓ |
| Hata kenarı / yüzey | 3:1 | `#B56156` / `#FFFFFF` | **4.37** ✓ | `#A86C61` / `#26231F` | **3.71** ✓ |
| Bilgi kenarı / yüzey | 3:1 | `#597F95` / `#FFFFFF` | **4.29** ✓ | `#648699` / `#26231F` | **4.03** ✓ |
| Silik (yalnız devre dışı/dekoratif) / zemin | 3:1 | `#8E8576` / `#FAF8F5` | **3.44** ✓ | `#8E8576` / `#171512` | **5.00** ✓ |
| Silik / yüzey | 3:1 | `#8E8576` / `#FFFFFF` | **3.64** ✓ | `#8E8576` / `#26231F` | **4.30** ✓ |

**Sonuç:** 106 denetim (53 çift × 2 tema), 0 başarısız. En dar pay: Bölmeli kontrol başparmak kenarı / ray (açık tema) 3,20 / hedef 3:1

Yeni bir renk çifti eklendiğinde kontrast betiğine eklenir. 4,5:1 (metin) ya da 3:1 (grafik) altında kalan çift kullanılmaz.
