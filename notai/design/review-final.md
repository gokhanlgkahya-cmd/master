# NotAI — Son Ekranlar ve Bileşen Kütüphanesi: Bağımsız Denetim

> Yazan: design-critic · Tarih: 2026-11-06
> İncelenen: `design/screens/01–14` (HTML + PNG), `design/screens/screens.css`, `design/components.html` + `components.png`
> Ölçüt: `design/decision.md`, `design/guidelines.md`, `design/tokens.css`, önceki tur `design/review.md`
> Yöntem: Her PNG açılıp bakıldı. `components.png` 8 parçaya bölünüp tam çözünürlükte okundu. Başlık ve sekme çubuğu 2 kat büyütülüp yan yana kıyaslandı. Kontrast `tokens.css`'teki değerlerden WCAG formülüyle yeniden hesaplandı. Sabit değer araması grep ve küçük bir CSS ayrıştırıcıyla yapıldı.
> Sınır: Hareket ve yay animasyonları durağan PNG'de görülmez, değerlendirmeye girmedi.

---

## 1. Puan tablosu (1–10, yüksek = iyi)

Kriterler: **Min** = minimalizm ve tek ana eylem · **İşl** = 2 saniyede yakalama, öneriyi tek dokunuşla kabul/ret · **Kont** = okunabilirlik ve kontrast · **TR** = Türkçe tipografi · **Tut** = tutarlılık · **Kopya** = marka kopyası riski (yüksek puan = düşük risk)

| Ekran | Min | İşl | Kont | TR | Tut | Kopya | Ort. | Tek cümlelik gerekçe |
|---|:-:|:-:|:-:|:-:|:-:|:-:|:-:|---|
| 01 Ana ekran | 8 | 9 | 9 | 9 | 8 | 8 | **8,5** | Sade ve net. Bölmeli kontrol sığıyor, mikrofon tek mürekkep öğe. |
| 02 Ana Günlük | 7 | 9 | 8 | 9 | 8 | 7 | **8,0** | Çip başına × geldi. Ama ekranda 9 kesikli çip var ve plan kartıyla mikrofon iki mürekkep öğe. |
| 03 Konu akışı | 9 | 8 | 9 | 9 | 9 | 7 | **8,5** | En sakin ekran. "Ana Günlük’ten geldi" meta satırı çok iyi. |
| 04 Not detayı | 8 | 9 | 9 | 8 | 8 | 9 | **8,5** | ✓/× satırları ve tek ana düğme doğru. Öncelikte bayrak ikonu kullanılmış, kütüphanede çubuk ikonu var. |
| 05 Ekleme paneli | 9 | 9 | 9 | 9 | 8 | 8 | **8,7** | Karartma yok, Ekle/Oluştur net ayrılmış. Grup başlığı "EKLE" büyük harf, kütüphanede "Ekle". |
| 06 Sesli yakalama | 8 | 9 | 9 | 9 | 8 | 8 | **8,5** | Canlı düzeltme ("~~Yarın~~ Perşembe") ve önizleme kartı çok iyi. Tek mürekkep öğe "Gönder". |
| 07 İlk açılış | 9 | 8 | 9 | 9 | 7 | 8 | **8,3** | Klavye açık ve odak alanda, 1 dokunuş vaadi tutuyor. Geri oku ve gezinme yolu yok. |
| 08 Görevler | 6 | 7 | 9 | 6 | 5 | 5 | **6,3** | En zayıf ekran. Kırmızı bayraklı öncelik (Todoist kalıbı), kesilen başlıklar, düz `'`, farklı sekme ikonları. |
| 09 Takvim | 7 | 8 | 9 | 8 | 6 | 7 | **7,5** | Okunaklı. Ama renkler tür bildiriyor (etkinlik mavi, hatırlatma sarı) ve "şimdi" çizgisi hata renginde. "Renk = anlam" ilkesine aykırı. |
| 10 Hatırlatmalar | 8 | 8 | 9 | 7 | 6 | 9 | **7,8** | Satır içi "Ertele" iyi. "Hatırlatmalar" etiketi, düz `'`, "10 dk"da bölünmez boşluk yok. |
| 11 Ara/Sor | 8 | 8 | 9 | 7 | 7 | 8 | **7,8** | Yanıt kartı ve kaynak düğmeleri iyi. Vurgu için uyarı rengi kullanılmış, "25 kg" ve "1.850 ₺"da bölünmez boşluk yok. |
| 12 Haftalık gözden geçirme | 8 | 9 | 9 | 7 | 8 | 9 | **8,3** | Satır başına çip ve ×, tek ana düğme. Başlıklar ve çipler kesiliyor ("iş–e…", "→ Haftalık"). |
| 13 Ayarlar | 8 | 8 | 8 | 9 | 8 | 9 | **8,3** | Düzenli, "Seni tanıma" bölümü güçlü. Devre dışı satırdaki "v0.4’te gelecek" bilgisi 3,64:1 kontrastta. |
| 14 Sabah bildirimi | 8 | 8 | 9 | 9 | 6 | 8 | **8,0** | Temiz. Ama "5 madde" diyor, kural ve 02 ekranı "en çok 3" diyor. İçerik 02'deki planla aynı değil, uygulama ikonu kitap. |
| **Bileşen kütüphanesi** | 8 | 9 | 9 | 9 | 7 | 8 | **8,3** | Durumlar (basılı, odak, devre dışı) eksiksiz. Ama ekranlardan ayrışmış (aşağıda §5). |
| **Ortalama** | 7,9 | 8,4 | 8,9 | 8,3 | **7,3** | 7,9 | **8,1** | 01–07 ortalaması **8,4**, 08–14 ortalaması **7,7** |

**Kısaca:** Önceki turdaki yön puanı 7,2'ydi, son tasarım **8,1**. En zayıf kriter **tutarlılık (7,3)**. Sorunların çoğu iki tasarımcının ekranları arasındaki farklardan ve kütüphane ile ekranların ayrışmasından geliyor. Tek başına kötü olan ekran **08 Görevler**.

---

## 2. Önceki turun "en büyük 3 sorunu" giderildi mi?

| # | Sorun (review.md) | Durum | Kanıt |
|---|---|---|---|
| 1a | AI çipinin şekli görünmüyordu (A'da 1,07:1) | ✅ **Giderildi** | Kenar artık `--color-chip-border #8E8576`, 1px kesikli çizgi. Hesap: açık temada zeminde 3,44, sheet'te 3,64; koyu temada 5,00. PNG'lerde çip şekli iki temada da seçiliyor. Not: 1px kesikli çizgi matematik olarak geçiyor ama ince. Gerçek Android WebView'de bir kez göz kontrolü yapılmalı. |
| 1b | Toplu ret vardı, çipler tek tek reddedilemiyordu | ✅ **Giderildi** | 02, 03, 05, 06 ve 12'de her çipin kendi × düğmesi var (`<button class="chip__x">`, 39 adet). `aria-label="Yoksay: …"` yazılmış. 04'te her öneri satırında 44px ✓/× var. Toplu ret düğmesi hiçbir ekranda yok. Dokunma alanı `::before` ile 44×44px'e genişletilmiş, satır arası 12px olduğu için hedefler çakışmıyor. |
| 1c | Çipler `<span>` olarak yazılmıştı (C) | ✅ **Giderildi** | Öneri çipleri `<button>`. Kabul edilmiş çipler bilinçli olarak `<span>`, yani etkileşimsiz. |
| 2 | Tek ana eylem bozuluyordu (C'de 5 siyah düğme, B'de sönük mikrofon, A'da karartmalı ses paneli) | 🟡 **Büyük ölçüde giderildi** | Mikrofon her yakalama ekranında 48px, dolu ve tek. 04'te tek mürekkep düğme "Görev olarak kaydet", ✓/× dolgusuz. 06'da karartma yok, tek dolu düğme "Gönder". 12'de "Sonraki" tek. **Kalan tek ihlal:** 02'de sabah planı görünürken hem "Planı onayla" hem mikrofon mürekkep dolgulu (bkz. §6). |
| 3 | Token disiplini zayıftı | ✅ **Giderildi** (küçük kalıntılarla) | §3'e bakın. Ekranlarda ve kütüphanede sabit renk ya da px yok. 13/15px yazı boyutları ve mono font kaldırılmış. Kalıntılar: `calc()` ile token'dan türetilmiş birkaç ızgara dışı değer ve iki ayrı CSS uygulaması. |

Önceki turun diğer maddeleri:
- **Giderilenler:** "Sabitli" filtresi kaldırıldı. "Kaynaklar" artık kesilmiyor. Saat balonun üstünde. İşlenmemiş noktası sabit sol olukta ve noktalar aynı hizada. `’` ve `&nbsp;–` 01–07'de doğru.
- **Kısmen giderilen:** Ana Günlük'e kitaptan farklı ikon verildi, ama yalnız 01–07'de ve kütüphanede. **08–11 sekme çubuğunda ve 14'teki bildirim ikonunda yine kitap kullanılmış.**

---

## 3. Token disiplini (sabit değer taraması)

Çalıştırılan komut:
```
grep -nE '#[0-9a-fA-F]{3,8}\b|[0-9]+px' design/screens/*.html design/screens/screens.css design/components.html
```
**Sonuç: 10 eşleşme, hepsi zararsız.**
- `screens.css:318`: yorum ("44px dokunma alanı…").
- `components.html:19`: `@media (max-width:900px)`. Kütüphane sayfasının iskeleti, uygulamaya girmez.
- `components.html:389, 417, 423, 435, 532, 543, 555, 615`: açıklama metinleri (`<p class="note">`).
- 14 ekran HTML'inde **0** eşleşme.

Genişletilmiş tarama: `rgb()`/`hsl()`, renk adları, `em`/`rem`/`ms`, `var()` dışındaki sayılar.
- `rgb`, `hsl` ya da renk adı yok. Bulunan `white` sözcükleri `white-space` özelliğine ait.
- Tüm `font-size`, `font-weight` ve `line-height` değerleri `var()` ile geliyor. Bir istisna var: aşağıdaki listede.
- Süreler token'dan geliyor. `20s`/`21s` eşleşmeleri SVG path verisi.

**Kalan küçük kalıntılar (engelleyici değil, Faz 3'te token'a çevrilsin):**
| Yer | Değer | Sorun |
|---|---|---|
| `screens.css:638, 640` (anahtar düğmesi), `components.html:252–253, 332` | `calc(var(--space-1) / 2)` = 2px | 4px ızgarasının dışında, token'la gizlenmiş sabit değer |
| `screens.css:672` (kilit ekranı saati) | `calc(var(--font-size-2xl) * 2.5)` = 70px, `line-height: 1` | Ölçek dışı boyut. `line-height: 1` kılavuzda yasak (§5). Sistem arayüzü çizimi olduğu için uygulamaya girmez, ama kural dışı. |
| `screens.css` (tutamak ve karo genişlikleri) | `calc(var(--size-handle-width) * 3)`, `calc(var(--size-tile) * 2.5)` | Çarpan sabit sayı. Gerekirse adlı token yapılmalı. |
| `components.html` | `transform: scale(1.06)`, `scale(.96)` | Kılavuz §3'te tanımlı değerler, ama token değiller. `--motion-scale-pop` gibi bir token önerilir. |

**Asıl token sorunu: iki ayrı CSS uygulaması var.** `components.html` `screens.css`'i hiç yüklemiyor, kendi `<style>` bloklarını kullanıyor. Sınıf adları da farklı: kütüphanede `btn-primary`, ekranlarda `btn--primary`. `screens.css`'in içinde de iki tasarımcının yazdığı aynı bileşenler iki kez tanımlı: `.pinned` ≈ `.card`, `.sectionlabel` ≡ `.grouphead`. Ayrıca `components.html` font ve `tokens.css` bağlantılarını iki kez yüklüyor.

---

## 4. Kontrast: kritik 5 çift (kendi hesabım, `tokens.css`'ten)

Formül: WCAG 2.x, `(L1 + 0,05) / (L2 + 0,05)`. Hedef metin için 4,5:1, grafik öğe için 3:1.

| # | Çift (nerede) | Hedef | Açık tema (ön / zemin) | Oran | Koyu tema (ön / zemin) | Oran |
|:-:|---|:-:|---|:-:|---|:-:|
| 1 | Meta satırı: saat · konu (02, 03, her liste) / zemin | 4,5 | `#6F675A` / `#FAF8F5` | **5,26** ✓ | `#B3AA9C` / `#171512` | **7,94** ✓ |
| 2 | AI çip metni / zemin (çip şeffaf) | 4,5 | `#554F45` / `#FAF8F5` | **7,65** ✓ | `#D5CEC3` / `#171512` | **11,67** ✓ |
| 3 | AI çipin kesikli kenarı / zemin | 3 | `#8E8576` / `#FAF8F5` | **3,44** ✓ | `#8E8576` / `#171512` | **5,00** ✓ |
| 4 | Bölmeli kontrolün pasif metni / ray (01, 09, 13) | 4,5 | `#6F675A` / `#F3F0EB` | **4,91** ✓ | `#B3AA9C` / `#3A3530` | **5,28** ✓ |
| 5 | "1 gün gecikti" (hata rengi) / sabah planı kartı (02) | 4,5 | `#A33A2C` / `#F3F0EB` | **5,78** ✓ | `#F09A8B` / `#3A3530` | **5,60** ✓ |

Kılavuz §9 tablosunda olmayan, yeni ekranlardan çıkan çiftler:

| Çift | Hedef | Açık | Oran | Koyu | Oran |
|---|:-:|---|:-:|---|:-:|
| "Bugün" (uyarı) / plan kartı (02) | 4,5 | `#855512` / `#F3F0EB` | 5,59 ✓ | `#E0B46A` / `#3A3530` | 6,30 ✓ |
| "Yüksek" etiketi / soluk hata (08) | 4,5 | `#A33A2C` / `#F7E3DF` | 5,32 ✓ | `#F09A8B` / `#33191A` | 7,49 ✓ |
| Etkinlik alt metni / soluk bilgi (09) | 4,5 | `#2F5F7A` / `#E1ECF2` | 5,75 ✓ | `#8FC0DB` / `#172630` | 7,91 ✓ |
| **Devre dışı satırın açıklaması (13 "Otonom mod")** | 4,5 | `#8E8576` / `#FFFFFF` | **3,64** ✗ | `#8E8576` / `#26231F` | **4,30** ✗ |
| Plan kartı kenarı (koyu) | — | — | — | `#3A3530` / `#3A3530` (kart zemini) | **1,00** (kenar görünmez) |

**Sonuç:** Kritik 5 çiftin hepsi iki temada da AA'yı geçiyor, en dar pay 4,91. Tek başarısızlık 13'te: "v0.4’te gelecek" devre dışı satırda ama okunması gereken bir bilgi. WCAG devre dışı öğeyi muaf tutar, ama bu cümle `--color-text-muted` olmalı. Koyu temada `--color-plan-border` ile `--color-plan-bg` aynı renk (`#3A3530`), yani kenar fiilen yok. Kenar dekoratif olduğu için kural ihlali değil, ama token boşa tanımlanmış.

---

## 5. Tutarlılık: 01–07 ile 08–14 arası farklar (iki tasarımcı)

| Konu | 01–07 (ve kütüphane) | 08–14 | Önem |
|---|---|---|---|
| **Sekme: Günlük ikonu** | Defter + kalem (`i-notebook`) | Açık kitap (`i-journal`). Kılavuz §8: "Ana Günlük'ün ikonu kitap değildir." | **Yüksek** |
| **Sekme: Görevler ikonu** | Kare içinde ✓ (`i-task`) | Daire içinde ✓ (`i-tasks`) | Orta |
| **Sekme: 4. etiket** | "Hatırlatma" | "Hatırlatmalar". 12px kalın yazıyla ~78px tutuyor, %130 yazı boyutunda ~100px olur. Bu, 82px'lik sekme hücresinden taşar (kılavuz §6). | **Yüksek** |
| Sekme `aria-label` | "Ana gezinme" | "Ana sekmeler" | Düşük |
| Üst başlık | Yalnız başlık (`<div class="topbar">`). Eylemler: ara + menü. | Başlık + alt satır (`topbar__text`). Başlık 10px yukarı kayıyor. 09'da anlamı belirsiz bir saat ikonu var. | Orta: alt satır ortak bir varyant olarak kütüphaneye eklenmeli |
| Liste düzeni | 01: konular düz zeminde, ayraç çizgisi girintili. Yalnız sabitliler kartta. | 08, 10, 11, 13: her grup beyaz kart içinde (`.card`), ayraç tam genişlik | Orta: iki farklı liste kalıbı var, biri seçilmeli |
| Bölüm başlığı | `.sectionlabel` | `.grouphead` (CSS'i birebir aynı, sağda sayaç var) | Düşük, birleştirilmeli |
| Kesme işareti | `’` her yerde | 08, 09, 10'da düz `'` ("Murat Bey'e", "Kimya'dan", "Kimya'yı") | Orta |
| Sayı + birim | `&nbsp;` var | 10 "10 dk", "1 sa"; 11 "25 kg", "1.850 ₺"; 13 "30 gün": düz boşluk | Düşük |
| Uzun başlık | Kayıt metni sarıyor (`overflow-wrap`) | 08 ve 12'de başlıklar tek satırda kesiliyor ("muhasebey…", "iş–e…"). Türkçe uzun kelimeler kelime ortasında bölünüyor. | **Orta-yüksek** (görev listesinde başlık okunamıyor) |
| Öncelik | 04'te bayrak ikonu. Kütüphanede çubuk ikonu, "Yüksek/Orta/Düşük". | 08'de kırmızı bayraklı hap; "Normal" ve "Düşük" ise çerçevesiz düz metin | **Yüksek** (bkz. aşağıdaki kopya notu) |
| Durum çubuğu | 12:07; wifi, sinyal, büyük pil | 12:10; sinyal, wifi, küçük pil | Düşük (prototipe özgü) |
| Sahne betiği ve SVG sprite | Ayrı betik, ayrı ikon seti | Ayrı betik, ayrı ikon seti | Faz 3'te tek ikon seti (`crates/ui`) |

**Kütüphane ile ekranlar arasındaki ayrışma:**
- Konu satırında kütüphane yuvarlak ikon kullanıyor, PARA türü yok, GemOlive ikonu çanta. 01 ve 03'te yuvarlatılmış kare ikon, "Alan/Proje" türü ve damla ikonu var.
- Ses şeridinde kütüphane dalga ve sayacı alta koyuyor. 06 bunları üste koyuyor ve "GÖREV" başlıklı önizleme kartı kullanıyor. 06 daha iyi.
- Sabah planı başlığında kütüphane "**3** iş bugün öne çıkıyor" diyor, 02 "Günaydın Gökhan. Bugün 3 iş öne çıkıyor:" diyor.
- Ana başlıktaki eylemler kütüphanede ara + katman, 01'de ara + menü.

**Renk = anlam ihlalleri (kılavuz §1.3):**
- 09: etkinlik mavi (bilgi rengi), hatırlatma sarı (uyarı rengi), "şimdi" çizgisi kırmızı (hata rengi).
- 08: "Yüksek" kırmızı. Gecikmiş satırda iki kırmızı öğe yarışıyor.
- 11: arama vurgusu uyarı zemininde (`--color-warning-bg`).

Bunlar durum değil, tür ya da önem bildiriyor.

**Kopya riski:** 08'deki kırmızı bayraklı öncelik hapı, kılavuz §7'de açıkça "alınmayan" diye yazılmış Todoist öncelik bayrağı görünümü. 04'teki bayrak ikonu da aynı riski taşıyor. Kütüphanedeki çubuk ikonu doğru seçim.

---

## 6. Çelişki: "en fazla bir mürekkep dolgulu öğe" ile plan kartı

**Durum:** 02'de sabah planı kartı görünürken "Planı onayla" ve mikrofon aynı anda mürekkep dolgulu. Kütüphanedeki örnek de aynı.

**Karar önerisi: mikrofon kazanır.**
1. Ürünün değişmez vaadi "1 dokunuş / 2 saniye yakalama". Yakalama ekranında en güçlü öğe hep mikrofon olmalı.
2. Plan kartı geçici: günde bir kez görünür ve onaylanınca kaybolur. Kalıcı bir eylemle yarışmamalı.
3. Kılavuz §1.2'deki "AI geride" ilkesine de uyar. Sabah planı bir AI önerisi.

**`guidelines.md` §1 madde 1'e eklenecek cümle:**
> Yazma çubuğunun göründüğü her ekranda mürekkep dolgu yalnız mikrofona (ya da onun yerini alan Gönder'e) aittir; aynı ekrandaki kart düğmeleri ("Planı onayla" dahil) ikincil, yani çerçeveli çizilir. Sayaç rozeti ile sistem arayüzü (bildirim, kilit ekranı) bu sayıma girmez.

Uygulama: 02'de ve `components.html`'deki "Sabah planı kartı"nda `btn--primary` yerine `btn--secondary` kullanılır. 09 (seçili gün) ve 12 ("Sonraki") yazma çubuğu olmayan ekranlar. Oralarda tek dolu öğe kuralı zaten sağlanıyor.

---

## 7. Sonuç: Faz 2'ye geçilebilir mi?

### **EVET**, koşullu

Gerekçe: Seçilen yönün ana kararları ekranlarda doğru uygulanmış. Önceki turun üç büyük sorunu çözülmüş, kontrast tamamen geçiyor, ekranlarda sabit değer kalmamış. Kalan sorunlar işlevi değil görünüşü ve tutarlılığı ilgilendiriyor. Faz 2'deki "hangi özellik, hangi sırayla" kararını engellemiyorlar. Bir madde (sabah planındaki madde sayısı) zaten Faz 2'nin konusu.
Not: CLAUDE.md'ye göre resmi "tasarım onaylandı" kararı Gökhan'ındır. Bu rapor, onay için engel kalmadığını söyler.

### Faz 3'te koda geçmeden önce düzeltilecekler

**🔴 Engelleyici.** İlgili dilim kodlanmadan çözülmeli, yoksa tutarsızlık koda taşınır:
1. **Tek bileşen kaynağı.** `components.html` ile `screens.css` iki ayrı uygulama. Leptos bileşenleri yazılmadan önce ekranlardaki sürüm esas alınmalı: konu satırı, ses şeridi, öncelik, başlık eylemleri. Kütüphane buna göre güncellenmeli. Yinelenen sınıflar birleştirilmeli (`.pinned`/`.card`, `.sectionlabel`/`.grouphead`, `btn-primary`/`btn--primary`). Liste kalıbı da tek olmalı: düz liste mi, kart içinde grup mu.
2. **Sekme çubuğu tekleştirilmeli.** Günlük ikonu defter + kalem olmalı (08–11'deki kitap kaldırılır), Görevler için tek ikon seçilmeli, etiket **"Hatırlatma"** olmalı (%130 yazı boyutunda sığması için). Tek SVG sprite kullanılmalı.
3. **Öncelik görünümü.** Kırmızı bayrak kaldırılmalı (Todoist kalıbı, kılavuz §7). Kütüphanedeki nötr çubuk ikonu + metin kullanılmalı. Kırmızı yalnız gecikme için. 04'teki bayrak da değişmeli. Adlar tek olmalı: "Yüksek / Orta / Düşük".
4. **Mürekkep kuralı** (§6'daki cümle) `guidelines.md`'ye eklenmeli. 02'deki ve kütüphanedeki "Planı onayla" ikincil düğme olmalı.
5. **Sabah planı madde sayısı.** Kılavuz ve 02 "en çok 3" diyor, 13 "en fazla 5", 14 "5 madde var". 02 ile 14'teki planın içeriği de birbirini tutmuyor. Faz 2'de karar verilmeli, dört yer eşitlenmeli.

**🟡 Engelleyici değil.** İlgili ekran kodlanırken düzeltilir:
6. 08, 09, 10'da düz `'` yerine `’` kullanılmalı. 10, 11 ve 13'te sayı ile birim arasına `&nbsp;` konmalı ("10&nbsp;dk", "25&nbsp;kg", "1.850&nbsp;₺").
7. Görev ve gözden geçirme başlıkları kesilmemeli: 2 satıra izin verilmeli (`line-clamp: 2`). Öncelik etiketi meta satırına inebilir.
8. 09'daki tür renkleri ve "şimdi" çizgisi, 11'deki vurgu: ya nötr tonlar kullanılmalı ya da ayrı token'lar eklenmeli (`--color-event-*`, `--color-now`, `--color-highlight`). Hangisi seçilirse `guidelines.md` §1.3'e yazılmalı.
9. 07 ilk açılış: Ana ekrana dönüş yolu eklenmeli (geri oku ya da açıklama).
10. 14: Bildirim ikonu kitap olmamalı, NotAI'nin kendi işareti kullanılmalı.
11. 13: "Otonom mod" açıklaması `--color-text-muted` olmalı (3,64 → 5,58).
12. `calc(var(--space-1) / 2)` (2px), `scale(1.06)` / `scale(.96)` ve kilit ekranındaki `line-height: 1` adlı token'a çevrilmeli ya da kaldırılmalı.
13. Koyu temada `--color-plan-border` ile `--color-plan-bg` aynı renk. Kenar isteniyorsa `neutral-700` gibi bir değer verilmeli.
14. Manrope Google Fonts'tan çekiliyor. Faz 3'te uygulamayla birlikte paketlenmeli (kılavuz §5, çevrimdışı). `components.html`'deki çift `<link>` temizlenmeli.
15. 04'teki "Kişi: Ayhan Kimya – Selin Hanım" ifadesinde tire yerine `·` kullanılmalı (diğer çiplerle aynı ayraç).
16. 1px kesikli çip kenarı gerçek bir Samsung cihazda, WebView'de, iki temada bir kez göz kontrolünden geçmeli.
