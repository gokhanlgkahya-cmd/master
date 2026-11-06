# NotAI — Üç Tasarım Yönünün Bağımsız Değerlendirmesi

> Yazan: design-critic · Tarih: 2026-11-06 · Ölçüt: `design/brief.md` (Revizyon 1 bağlayıcı) + `design/directions/plan.md`
> İncelenen: `design/directions/{A,B,C}/index.html` + açık/koyu PNG'ler. Ayrıca her telefon Playwright ile tek tek, 2 kat büyütülerek çekildi ve yakından bakıldı.
> Kontrast değerleri tasarımcı tablosundan alınmadı. HTML'deki CSS değişkenlerinden okunup WCAG formülüyle yeniden hesaplandı.
> Sınır: Yaylı (spring) animasyonlar durağan PNG'de görülemez. Hareket yalnız CSS değerlerinden (süre, aşma oranı) değerlendirildi.

---

## 1. Puan tablosu (1–10, yüksek = iyi)

| Kriter | A — Taş Defter | B — Çelik Pano | C — Saf Mürekkep |
|---|:-:|:-:|:-:|
| Minimalizm (gereksiz öğe yok, tek ana eylem belli) | **7** | 6 | 5 |
| İşlevsellik (2 sn yakalama, AI önerisini tek dokunuşla kabul/ret) | **7** | **7** | 6 |
| Okunabilirlik ve kontrast (WCAG AA) | **8** | 7 | **8** |
| Türkçe tipografi (ğ ş ı İ, uzun kelime) | **8** | 7 | 7 |
| Tutarlılık (boşluk ızgarası, köşe, renk, token kullanımı) | **7** | 6 | 6 |
| Marka kopyası riski (yüksek puan = düşük risk; WhatsApp dahil) | 6 | **9** | 8 |
| **Toplam / Ortalama** | **43 / 7,2** | 42 / 7,0 | 40 / 6,7 |

Kısaca: A ile B arasındaki fark küçük. A daha tanıdık ve sakin, B ise daha düzenli ve daha özgün. C görsel olarak en iddialı yön, ama "minimalist ve soluk AI" ilkesiyle en çok çatışan da o.

---

## 2. Kontrast denetimi (kendi hesabım)

Formül: WCAG 2.x göreli parlaklık, `(L1+0,05)/(L2+0,05)`. Metin için hedef 4,5:1, grafik öğe ya da şekil sınırı için 3:1.

| Yön · tema | Çift | Ön / zemin | Oran | Hedef | Sonuç |
|---|---|---|---|---|---|
| A açık | Soluk meta (saat, önizleme) / zemin | `#6F675A` / `#FAF8F5` | 5,26 | 4,5 | ✓ |
| A açık | Soluk meta / sabah planı kartı | `#6F675A` / `#F3F0EB` | 4,91 | 4,5 | ✓ (payı dar) |
| A açık | AI çip metni / çip | `#554F45` / `#F3F0EB` | 7,13 | 4,5 | ✓ |
| A açık | **AI çip şekli / sayfa zemini** | `#F3F0EB` / `#FAF8F5` | **1,07** | 3 | ✗ (şekil görünmüyor) |
| A açık | İşlenmemiş noktası / zemin | `#3A3530` / `#FAF8F5` | 11,44 | 3 | ✓ |
| A açık | Pasif sekme etiketi / zemin | `#6F675A` / `#FAF8F5` | 5,26 | 4,5 | ✓ |
| A açık | "1 gün gecikti" / plan kartı | `#A33A2C` / `#F3F0EB` | 5,78 | 4,5 | ✓ |
| A koyu | Soluk meta / balon | `#B3AA9C` / `#26231F` | 6,81 | 4,5 | ✓ |
| A koyu | AI çip metni / çip | `#D5CEC3` / `#3A3530` | 7,76 | 4,5 | ✓ |
| A koyu | İşlenmemiş noktası / zemin | `#E7E2DA` / `#171512` | 14,13 | 3 | ✓ |
| B açık | Soluk saat sütunu / zemin | `#56606D` / `#FFFFFF` | 6,38 | 4,5 | ✓ |
| B açık | AI çip metni / zemin | `#424A55` / `#FFFFFF` | 8,97 | 4,5 | ✓ |
| B açık | Çipin kesikli kenarı / zemin | `#727D8B` / `#FFFFFF` | 4,18 | 3 | ✓ |
| B açık | İşlenmemiş halkası / zemin | `#424A55` / `#FFFFFF` | 8,97 | 3 | ✓ |
| B açık | "1 gün gecikti" / plan bloğu | `#B9342A` / `#F7F8FA` | 5,51 | 4,5 | ✓ |
| B koyu | Soluk meta / genişleyen kart | `#9AA4B1` / `#1B1F25` | 6,55 | 4,5 | ✓ |
| B koyu | Pasif sekme / zemin | `#9AA4B1` / `#0E1115` | 7,50 | 4,5 | ✓ |
| C açık | Soluk meta / zemin | `#5C5C5C` / `#F4F4F4` | 6,08 | 4,5 | ✓ |
| C açık | AI çip metni / çip | `#5C5C5C` / `#FFFFFF` | 6,69 | 4,5 | ✓ |
| C açık | **Çip kenarı / kart** | `#D1D1D1` / `#FFFFFF` | **1,53** | 3 | ✗ |
| C açık | Plan bloğu soluk metin / siyah | `#A3A3A3` / `#0A0A0A` | 7,85 | 4,5 | ✓ |
| C açık | Plan "1 gün gecikti" / siyah | `#FF7A70` / `#0A0A0A` | 7,80 | 4,5 | ✓ |
| C koyu | Soluk meta / kart | `#A3A3A3` / `#171717` | 7,11 | 4,5 | ✓ |
| C koyu | **Çip kenarı / kart** | `#474747` / `#171717` | **1,93** | 3 | ✗ |
| C koyu | **Kart / sayfa zemini** | `#171717` / `#000000` | **1,17** | 3 | ✗ |

**Sonuç:** Üç yönde de **tüm metin çiftleri AA'yı geçiyor**, tasarımcıların metin iddiaları doğru. Kalan sorunlar şekil sınırlarında:
- **A açık temada AI çipinin şekli fiilen görünmüyor (1,07:1).** Çip, gri yazılmış düz metin gibi duruyor. Dokunulabildiği yalnız ✦ ikonundan anlaşılıyor. Bu, WCAG'nin katı ihlali sayılmaz, çünkü metin kendini tanıtıyor. Ama "tek dokunuşla kabul" vaadini zayıflatıyor.
- **C'de çip kenarı (1,53 ve 1,93) ve koyu temada kart sınırı (1,17) zayıf.** Ayrıca plan.md'deki C kontrast tablosu "çip kenarı = `#767676` (4,54)" diyor, oysa prototipte kullanılan değer `#D1D1D1`. **Tablo ile uygulama birbirini tutmuyor.**
- B'nin 10–11px metinleri (fatura.jpg etiketi, "TARİH / ÖNCELİK" başlıkları) renk açısından geçiyor, ama boyut olarak ölçeğin (12px) altında kalıyor. Android'de yazı büyütme açıkken bile küçük kalırlar.

---

## 3. Yön yön gözlemler (ekranlara bakarak)

### A — Taş Defter (balon + bottom sheet)
**İyi:**
- En sakin ve en "hemen anlaşılır" yön. Sıcak taş grisi koyu temada gözü yormuyor.
- Mikrofon koyu, dolu ve yuvarlak. Ekranın tek ana eylemi her ekranda açıkça o.
- Sekme çubuğunda ikon ve etiket birlikte duruyor. "Hatırlatma" sığıyor, `İŞLENMEDİ` doğru büyük İ ile yazılmış.
- "Değer akışı haritalandırması" taşmadan tek satırda kalıyor.

**Sorunlar:**
1. **WhatsApp'a en çok benzeyen yön bu.** Sağa yaslı beyaz balonlar, saatin balonun içinde sağ altta durması (WhatsApp'ın en tanınır ayrıntısı), kameranın yazma alanının içinde olması ve koyu yuvarlak mikrofon, hep birlikte WhatsApp'ın sohbet ekranını çağrıştırıyor. Yeşil renk, balon kuyruğu ve duvar kâğıdı yok, yani sınır aşılmamış. Ama sınıra en yakın yön A.
2. **AI çip şekli açık temada görünmüyor** (1,07:1, yukarıdaki tabloya bakın).
3. Çip satırının sonundaki tek **×** aslında "tüm önerileri reddet" anlamına geliyor (`aria-label="Önerileri reddet"`). Ama satır kaydığında son çipin yanına düşüyor ve sanki yalnız o çipi siliyormuş gibi okunuyor (09:12 kaydında "Kişi: Ayhan Kimya ×").
4. İşlenmemiş noktası balonun sol kenarına göre konumlanıyor. Fotoğraflı balon dar olduğu için nokta ortaya kayıyor. Bu yüzden noktalar aynı dikey hizada durmuyor ve taranması zorlaşıyor.
5. Ana ekranda 5 filtre hapı var. "Sabitli" gereksiz, çünkü sabitli olan zaten yalnız Ana Günlük. "Kaynaklar" ise ekranın sağından kesiliyor.
6. Ana Günlük ve Okuma notları **aynı kitap ikonunu** kullanıyor. Alanlar ikonla ayrıldığı için bu kafa karıştırır.
7. Sesli yakalama paneli arka ekranı karartıyor (scrim, yani karartma perdesi). Brief'teki "küçük kayan panel" yerine modal (önündeki iş bitmeden arkaya dokunulamayan pencere) gibi davranıyor.
8. Kodda 13px ve 15px yazı boyutları ölçek dışı kalmış. Ölçek 12/14/16/19 olmalı.

### B — Çelik Pano (satır + yerinde genişleyen kart)
**İyi:**
- Saat sütunu ve hizalama gerçekten düzenli. Bullet Journal'daki "saat · madde" düzenine en yakın yön bu.
- **WhatsApp'a en az benzeyen yön.** Balon yok, kart yok, kendine özgü bir kimliği var.
- Detayda **her öneri için ayrı ✓ / × düğmesi** var, 44px ve satır içinde. Kabul/ret için en net çözüm bu.
- Haftalık gözden geçirme Ana Günlük'ün hemen altında sabit bir satır. Bölmeli kontrol (segmented control: tek kutuda yan yana seçenekler) ekrana kesilmeden sığıyor.
- Ekleme paneli klavyenin yerine açılıyor. Karartma yok, akış görünür kalıyor ve ataç ikonu × oluyor. Üç yön arasında en akıcı panel bu.
- Ses kaydında yazma çubuğu yerinde şeride dönüşüyor. Karartma yok.

**Sorunlar:**
1. **Kesikli çerçeveli çipler** ekranda çok fazla. Her kayıtta 2–4 kesikli kutu var ve görsel gürültü oluşturuyor. Yön, "kutu yok" vaadinin tersine kutularla dolmuş.
2. **Mono (eş genişlikli) yazı çok yerde kullanılmış.** "Sağlık", "Çarşamba", "1 gün gecikti" ve sesli kayıt önizlemesi mono. Türkçe kelimeler mono yazıda teknik ve soğuk görünüyor, ayrıca daha yavaş okunuyor.
3. Mikrofon, kamera ile aynı ağırlıkta düz bir ikon. **Yakalamanın ana eylemi göze çarpmıyor.**
4. Akış ekranında hem yazma çubuğu hem sekme çubuğu açık. Altta yaklaşık 130px sabit alan gidiyor.
5. Fotoğraflı kayıtta metin, saat sütunu hizasından kayıyor ("Ambalaj faturası" küçük resmin sağına itilmiş).
6. **Geist fontunda ₺ işareti yok.** Inter yedeği çalışıyor, ama aynı satırda iki font karışıyor ("1.850 ₺").
7. Kodda boşluk değişkenleri (`--space-*`) tanımlanmış ama **bir kez bile kullanılmamış**. Tüm boşluklar elle px yazılmış. Bu, "koda sabit ölçü yazılmaz" kuralının ihlali.
8. Ana Günlük ve Okuma notları burada da aynı kitap ikonunu kullanıyor.

### C — Saf Mürekkep (kart + tam ekran)
**İyi:**
- 17px gövde yazısı ve en yüksek metin kontrastı (açık temada 18–20:1) sayesinde güneşte en okunaklı yön.
- Tam ekran detay geniş ve sakin. Görev alanları iki sütunda rahat duruyor.
- Ekleme panelinde "Ekle" (fotoğraf, belge…) ile "Oluştur" (görev, hatırlatma…) ayrılmış. İşlev açısından akıllıca bir gruplama.
- `kg’lık` kesme işareti doğru (’) kullanılmış. A ve B düz tırnak (') kullanıyor.

**Sorunlar:**
1. **Sabah planı kalın siyah bir blok** ve ekranın en baskın öğesi. Bu, brief'teki "AI soluk kalır, kullanıcının kaydı öndedir" ilkesinin tersi. Koyu temada ise blok kenarlı griye dönüyor. Yani vurgu iki temada farklı ve tutarsız.
2. **Detayda 4 siyah "Kabul" düğmesi ve 1 siyah "Görev olarak kaydet" düğmesi var.** Beş düğme eşit ağırlıkta, tek ana eylem kayboluyor.
3. **Pasif sekmeler yalnız ikon olarak duruyor.** Zil ikonu "Hatırlatma" mı, "bildirim" mi? Büyüteç ikonu "Ara/Sor"daki *Sor* (notlara soru sorma) özelliğini hiç anlatmıyor. Ayrıca "Hatırlatma" etiketinin sığıp sığmadığı denenmemiş oluyor.
4. Kartın sağ üstündeki **×** ("önerileri reddet") işlenmemiş noktasının hemen yanında duruyor ve **kaydı silme düğmesi gibi okunuyor**. Veri kaybı korkusu yaratır.
5. İşlenmemiş durum iki kez kodlanmış: sol kenarda 3px siyah çizgi ve sağ üstte nokta. İkisi birden gereksiz.
6. Başlıkta satır sonu hatası var: "Kostik tedarikçisini ara / **– perşembe**". Tire yeni satırın başına düşmüş. Tirenin önünde bölünmez boşluk (`&nbsp;`) olmalı.
7. Bir satırda üç font var: Schibsted başlık, Inter önizleme, Fragment Mono tarih. Ana ekran kalabalık görünüyor.
8. Kodda: çipler `<span>` olarak yazılmış, `<button>` değil. Klavye ve ekran okuyucu ile dokunulamazlar. Kart iç dolgusu 10px, yani 4px ızgarasının dışında.

---

## 4. Not detayı: hangi açılır pencere tipi işlevsel olarak daha iyi?

| | A — Bottom sheet (alttan kayan sayfa, %55 ve %92 iki durak) | B — Yerinde genişleyen kart | C — Tam ekran |
|---|---|---|---|
| Bağlamı korur mu? | Kısmen. Akışın üstü karartılmış hâlde görünür. | Evet. Liste yerinde kalır. | Hayır, akış tamamen kaybolur. |
| Tek elle kullanım | **En iyisi.** İçerik başparmak bölgesinde, aşağı çekince kapanır. | Orta. "Bitti" sağ üstte, başparmağa uzak. | Orta. Geri oku sol üstte, ama Android geri hareketi çalışır. |
| Öneriyi kabul/ret | %55'te yalnız çip var, tek tek ret yok. %92'de Kabul/Ret yazılı düğmeler var. | **En iyisi.** Her satırda 44px ✓/× var. | Var, ama 4 siyah düğme ana eylemle yarışıyor. |
| Uzun not | %92'de kaydırılır. | **Zayıf.** Kart ekranın yaklaşık %63'ünü kaplıyor, listeyi itiyor ve kaydırma çakışıyor. | **En iyisi.** Tüm ekran nota ayrılmış. |
| Belirsizlik | Az. | Yazma çubuğu açık kalıyor: yazılan şey bu nota mı, akışa mı gidecek? | Yok. |
| Uygulama zorluğu (Tauri + Leptos, Android WebView) | Orta. Sürükleme hareketi ve iki durak gerekiyor. | **En zor.** Yükseklik animasyonu WebView'de takılır (her karede sayfa yeniden dizilir). | Kolay, sadece yeni sayfaya geçiş. Paylaşılan öğe geçişi (kartın büyüyerek ekrana dönüşmesi) ise zor, ama atlanabilir. |
| **Puan** | **8** | 6 | 7 |

**Öneri:** Bottom sheet kalsın, ama iki durağın işi ayrılsın:
- **%55 durağı (göz at):** B'nin ✓/× öneri satırları ve ana düğme "Görev olarak kaydet".
- **%92 durağı (düzenle):** pratikte C'nin tam ekran düzeni (görev alanları iki sütun, bağlantılı kayıtlar). Android geri hareketi sheet'i kapatır.

Böylece A'nın tek el rahatlığı, B'nin kabul/ret netliği ve C'nin uzun not alanı tek pencerede birleşir.

---

## 5. Özet

### En güçlü yön
**A — Taş Defter**, kıl payı önde. Kullanıcının WhatsApp alışkanlığını en az öğrenmeyle taşıyor. Tek ana eylemi (mikrofon/yazma) en net gösteren yön bu. Sıcak gri iki temada da en dengeli sonucu veriyor. Ama tek başına seçilirse marka benzerliği ve görünmeyen çip sorunları düzeltilmeli. B ise "düzen ve özgünlük" tarafında açıkça daha iyi.

### En büyük 3 sorun (üç yönün toplamında)
1. **AI çiplerinin "dokunulabilir öneri" olduğu belli değil ya da çok gürültülü.** A'da şekil görünmüyor (1,07:1), B'de kesikli kutular kalabalık, C'de kenar 1,53:1. Ayrıca üç yönde de akış ekranında **öneriyi tek tek reddetmek mümkün değil**: satır sonundaki × hepsini birden reddediyor. Brief'teki "tek dokunuşla kabul/ret" yalnız kabul için sağlanmış.
2. **Tek ana eylem ilkesi bazı ekranlarda bozuluyor.** C'de detayda 5 siyah düğme ve baskın siyah plan bloğu var. B'de mikrofon sönük. A'da ses paneli gereksiz yere karartma açıyor.
3. **Token disiplini zayıf.** B boşluk değişkenlerini hiç kullanmıyor. A'da ölçek dışı 13/15px yazı boyutları, C'de 10px ızgara dışı dolgular var. A ve B'de aynı ikon iki farklı konuda kullanılmış. plan.md'deki C kontrast tablosu da uygulamayla uyuşmuyor. tokens.json yazılırken bunlar temizlenmezse uygulamaya taşınır.

### Önerilen karışım (token düzeyinde)

| Konu | Seçim | Somut değer / kural |
|---|---|---|
| **Palet** | A'nın sıcak taş grisi | `--neutral-50…950` = A (`#FAF8F5` … `#171512`); semantik renkler A'nın toprak tonları |
| **Koyu tema zemini** | A | `--color-bg: #171512`. AMOLED pil tasarrufu istenirse ileride "saf siyah" seçeneği eklenebilir. |
| **Yazı tipi** | A'nın Manrope'u, tek aile | `--font-heading` ve `--font-body` = Manrope. ₺ işareti var, Türkçe tam. Saatlerde mono yerine `font-variant-numeric: tabular-nums`. `--font-mono` (IBM Plex Mono) yalnız kimlik/kod için. Geist alınmaz (₺ yok). |
| **Tip ölçeği** | A, 1,2 oran | 12/14/16/19/23/28. 13 ve 15px kaldırılır. 12px altı yasak (B'nin 10–11px etiketleri dahil). |
| **Köşe** | A | `--radius-sm 8` · `--radius-md 12` · `--radius-lg 16` · `--radius-pill 999` |
| **Boşluk** | Ortak 4px ızgarası, **zorunlu değişken kullanımı** | `--space-1…12`. 6/10/14px gibi ızgara dışı değerler 4 veya 8'e yuvarlanır. |
| **Kayıt görünümü** | A'nın kuyruksuz balonu, iki düzeltmeyle | Balon en fazla %86 genişlikte. **İşlenmemiş noktası sabit bir sol oluğa** taşınır (B'nin saat sütunu disiplini), böylece tüm noktalar aynı hizada durur. WhatsApp benzerliğini azaltmak için saat balon içinde kalabilir ama sağ alt yerine **balonun üstüne, konu etiketiyle aynı satıra** (soluk, 12px) konur. |
| **AI çipi** | A'nın hap biçimi + B'nin kesikli kenar fikri | `--color-chip-bg: transparent` · `--color-chip-border: #8E8576` (1px kesikli; açık zeminde 3,44, koyuda 5,0) · `--color-chip-text: #554F45` (açık zeminde 7,65) / koyu `#D5CEC3` (11,67). Dokununca kabul edilir ve kenar düz çizgiye döner. Satır sonundaki × yerine **"Yoksay"** yazılı küçük metin düğmesi konur (tümünü yoksayar). Tek tek ret detay sheet'inde yapılır. |
| **İşlenmemiş işareti** | A | 8px dolu nokta, `--color-unprocessed` (açık `#3A3530`, koyu `#E7E2DA`), ekran okuyucu metni "İşlenmedi". |
| **Sabah planı** | A | `--color-surface-sunken` zemin + ince kenar. C'nin siyah bloğu alınmaz. |
| **Not detayı** | A sheet + B satırları + C tam ekran | Bölüm 4'teki birleşik öneri: %55'te ✓/× satırları, %92'de tam ekran alan düzeni. Tek ana düğme "Görev olarak kaydet". "Kabul" düğmeleri dolgusuz ikon olur (✓ başarı rengi, × soluk). |
| **Ekleme (ataç) paneli** | B'nin davranışı + C'nin gruplaması | Klavyenin yerine açılır, karartma yok, ataç × olur. İki grup: **Ekle** (Fotoğraf · Kamera · Belge · Konum · Kişi) ve **Oluştur** (Görev · Hatırlatma · Etkinlik · Liste), ızgara düzeninde. |
| **Ses paneli** | B'nin yerinde şeridi + A'nın önizleme kartı | Karartma yok. Canlı metinde eski kelimeler soluk, son kelimeler koyu (B). Yapılandırılmış önizleme ikonlu satırlarla gösterilir (📅 ~~Yarın~~ Perşembe 12 Kas · 🕙 10:00 · 📁 Kostik tedarik), mono kullanılmaz. |
| **Mikrofon** | A ve C | Dolu, koyu, yuvarlak 48px. Her ekranın ana eylemi. |
| **Sekme çubuğu** | A | İkon + etiket her zaman görünür. Etkin sekmede yumuşak hap (`--color-tab-indicator`). Akış ekranında gizli. |
| **Ana ekran filtresi** | B | Bölmeli kontrol `Tümü \| Proje \| Alan \| Kaynak`. "Sabitli" kaldırılır. |
| **Haftalık gözden geçirme erişimi** | B | Ana Günlük'ün hemen altında sabit satır. |
| **Hareket** | B'nin hızı | Panel: B `k520 c36` (~370ms, %1,6 aşma). A'nın 610ms'si sık açılan sheet için ağır. Pop: B `k620 c30` (~424ms, %9 aşma), "belirgin yay" hissini korur. C'nin %17 aşması oyuncak gibi durur. |
| **Gölge** | A | `--shadow-1/2/sheet` A değerleri. Koyu temada gölge yerine yüzey bir ton açılır. |
| **İkon** | A'nın Phosphor seti | Ana Günlük'e kitap dışında ayrı bir ikon verilir (ör. defter-kalem). Kitap ikonu Okuma notları'nda kalır. |
| **Tipografi ayrıntısı** | C | Kesme işareti olarak ’ kullanılır (`kg’lık`). Başlıkta tirenin önüne `&nbsp;` konur. `lang="tr"` korunur. |

> Not: Gökhan balon yerine satır görünümünü (B) tercih ederse, yukarıdaki tüm token'lar değişmeden çalışır. Değişen tek şey kayıt bileşeni olur: solda tabular-nums saat sütunu, kutusuz satır.
