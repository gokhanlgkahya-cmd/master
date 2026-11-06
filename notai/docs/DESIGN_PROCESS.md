# Tasarım Süreci (Faz 0 + Faz 1)

## Faz 0 — Kullanıcıya sorulacak sorular
`AskUserQuestion` ile, her turda en fazla 4 soru, her soruda 2–4 seçenek. Mümkünse her seçeneğe
`preview` ile küçük bir görsel/ASCII örnek ekle. Cevapları `design/brief.md` dosyasına yaz.

**Tur 1 — Karakter**
1. Genel his: Sakin kâğıt/defter · Keskin teknik (terminal/Bloomberg esintili) · Yumuşak modern (Keep benzeri kartlar) · Lüks koyu
2. Varsayılan tema: Koyu · Açık · Sistemi takip et
3. Yoğunluk: Ferah (az öğe, büyük boşluk) · Dengeli · Yoğun (çok bilgi bir bakışta)
4. Vurgu rengi ailesi: Zeytin/yeşil · Lacivert/mavi · Kehribar/turuncu · Nötr (tek renk + gri tonlar)

**Tur 2 — Yapı**
1. Ana gezinme: Alt sekme çubuğu · Yan çekmece · Kaydırmalı sekmeler · Tek ekran + komut paleti
2. Sekmeler (çoklu seçim): Bugün · Gelen Kutusu · Görevler · Projeler/Alanlar · Takvim · Arama/Sohbet · Haftalık Gözden Geçirme
3. Not görünümü: Kart ızgarası (Keep) · Liste (Todoist) · Zaman akışı (günlük log) · Kullanıcı değiştirebilsin
4. Hızlı yakalama butonu: Ortada büyük FAB · Sağ alt FAB · Her ekranın altında sabit yazma çubuğu

**Tur 3 — Detay**
1. Açılır pencere şekli: Alttan kayan sayfa (bottom sheet) · Ortada modal kart · Tam ekran · Yerinde genişleyen kart
2. Köşe yapısı: Keskin (2–4px) · Orta (8–12px) · Yuvarlak (16–24px)
3. Yazı tipi karakteri: Grotesk sans (Inter / Geist) · İnsancıl sans (IBM Plex Sans / Source Sans) · Serif başlık + sans gövde · Mono vurgulu
4. Hareket: Neredeyse yok · İnce geçişler (150–200ms) · Belirgin yaylı animasyonlar

**Tur 4 — AI'ın görünürlüğü**
1. AI önerileri nasıl görünsün: Notun altında soluk "öneri çipi" · Ayrı "Önerilen" kutusu · Sadece gelen kutusunda
2. Sabah planı: Bildirim · Açılış ekranı kartı · İkisi de
3. Etiket/alan renkleri: Her alanın kendi rengi · Tek renk, ikonla ayrım
4. Sesli yakalama ekranı: Tam ekran dalga formu · Küçük kayan panel + canlı metin

## Faz 1 — Tasarımcı ajanlarla üretim
1. **design-director**, brief'i okur ve 3 farklı tasarım yönü tanımlar (isim, ruh hali, palet, tipografi, örnek ekran).
2. Üç **ui-designer** ajanı PARALEL çalışır, her biri bir yönü `design/directions/A|B|C/` altına
   tek dosyalık statik HTML prototip olarak çizer: Bugün ekranı, Gelen Kutusu, Hızlı Yakalama sheet'i, Not detayı.
   Telefon çerçevesinde (412×915) + koyu/açık.
3. **design-critic**, üçünü kontrast, okunabilirlik, Türkçe karakter, tutarlılık ve "minimalist ama işlevsel"
   kriterleriyle puanlar → `design/review.md`.
4. Kullanıcıya ekran görüntüleri + puan tablosu gösterilir, `AskUserQuestion` ile seçim yapılır
   (karışım da olabilir: "A'nın renkleri + C'nin yerleşimi").
5. Seçilen yönden `design/tokens.json` (renk tonları 50–950, semantik renkler, tipografi ölçeği,
   boşluk 4px ızgara, radius, gölge, motion) ve `design/components.html` (sekme çubuğu, butonlar, çipler,
   kartlar, bottom sheet, modal, toast, giriş alanları, boş durumlar) üretilir.
6. Tüm ana ekranlar `design/screens/` altına çizilir. Kullanıcı onayı → Faz 2.

Ekran görüntüsü için: `npx playwright screenshot` ile PNG al, kullanıcıya göster.
