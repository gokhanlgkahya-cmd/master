# NotAI — Tasarım Brief'i (Faz 0 çıktısı)

> Tarih: 2026-11-06 · Kaynak: Gökhan ile 4 soru turu + 1 netleştirme turu + Revizyon 1 (WhatsApp kurgusu)
>
> ⚠️ **Revizyon 1 (aşağıda) Tur 2'deki sekme ve Gelen Kutusu kararlarının yerine geçer.**

## Tur 1 — Karakter
| Soru | Cevap | Tasarıma yansıması |
|---|---|---|
| Genel his | "Temalar arası geçiş mümkün olsun" → netleştirme: **Tek temel karakter** | design-director 3 farklı his önerir, kullanıcı birini seçer. Ayarlarda yalnız görünüm modu (açık/koyu/sistem) değişir. |
| Varsayılan tema | **Seçenekli olsun** | Ayarlarda "Sistem / Açık / Koyu" seçimi. İlk açılışta varsayılan = Sistemi takip et. Açık + koyu tema eşit kalitede tasarlanır. |
| Yoğunluk | **Ferah** | Az öğe, büyük boşluk; ekranda ~4–6 kayıt. Meta bilgi (tarih/etiket) gerekince görünür. |
| Vurgu rengi | **Nötr** (tek renk + gri tonlar) | Palet mürekkep/gri ölçeği; renk yalnız semantik durumlar için (başarı/uyarı/hata/bilgi). |

## Tur 2 — Yapı
| Soru | Cevap | Tasarıma yansıması |
|---|---|---|
| Ana gezinme | **Alt sekme çubuğu** | Tek elle kullanım; altta sekmeler. |
| Sekmeler | **Bugün + Gelen Kutusu**, **Görevler + Takvim** | 4 ana bölüm: Bugün · Gelen Kutusu · Görevler · Takvim. (Projeler/Alanlar, Arama/Sohbet, Haftalık Gözden Geçirme ana sekme değil; ikincil erişim — ör. menü/ayarlar/arama ikonu — tasarımcıya bırakıldı.) |
| Not görünümü | **"Notlar günlük ana kayıt ise tarihe göre, en yeni en altta — WhatsApp mesaj mantığı"** | Ana akış = sohbet benzeri zaman akışı: kayıtlar balon/satır, en yeni en altta, gün ayraçlarıyla (Bugün, Dün, 4 Kasım…). Açılışta en alta kaydırılmış gelir. |
| Hızlı yakalama | **Sabit yazma çubuğu** | Her ekranın altında WhatsApp tarzı yazma alanı + mikrofon. Açılışta 1 dokunuşla yazmaya hazır. |

## Tur 3 — Detay
| Soru | Cevap | Tasarıma yansıması |
|---|---|---|
| Açılır pencere | "Fikrim yok" → netleştirme: **Tasarımcılar denesin** | 3 yön farklı pencere tipleri dener (bottom sheet / yerinde genişleyen kart / modal vb.); kullanıcı görerek seçer. |
| Köşe yapısı | **Orta (8–12px)** | radius ölçeği 8–12px merkezli. |
| Yazı tipi | **Grotesk sans** (Inter / Geist ailesi) | Ücretsiz, Türkçe tam destekli grotesk; ğ, ş, ı, İ test edilir. |
| Hareket | **Belirgin yaylı animasyonlar** | Spring (yay) eğrileri; sheet açılışı, kayıt gönderme, çip kabulünde hissedilir esneme. `prefers-reduced-motion` desteklenir. |

## Tur 4 — AI'ın görünürlüğü
| Soru | Cevap | Tasarıma yansıması |
|---|---|---|
| AI önerileri | **Notun altında soluk "öneri çipi"** | Her kaydın altında ✦ çipler (ör. "Görev · Perşembe", "#sabun · İş"); tek dokunuşla kabul/ret. |
| Sabah planı | **İkisi de** | Sabah bildirimi + Bugün ekranının üstünde plan kartı. |
| Alan/etiket ayrımı | **Tek renk, ikonla ayrım** | Nötr paletle uyumlu; alanlar (İş, Ev, Sağlık…) ikonla ayrılır. |
| Sesli yakalama | **Küçük kayan panel + canlı metin** | Yazma çubuğunun üstünde açılan panel; konuşma anında metne döner, yapılandırılmış önizleme (Görev · Perşembe) altında. |

## Revizyon 1 — Kurgu: WhatsApp iskeleti, NotAI görsel dili
Gökhan 3 WhatsApp ekran görüntüsü paylaştı (sohbet listesi, sohbet akışı, ataç menüsü) ve şunu istedi:
**"Kurgu olarak WhatsApp'ı kullanalım, tasarım dili sorulardaki gibi olsun."**

### Yapı eşlemesi
| WhatsApp | NotAI |
|---|---|
| Sohbet listesi (ana ekran) | **Ana ekran:** en üstte sabitli **Ana Günlük**, altında **konular** (PARA proje/alanları: GemOlive, Ev, Sağlık…). Her satır: ikon · ad · son kayıt önizlemesi · saat. |
| Sohbet ekranı | **Akış ekranı** (Ana Günlük veya bir konu): gün ayraçları (Perşembe / Dün / Bugün), en yeni en altta, açılışta en alta kaydırılmış, altta sabit yazma çubuğu + ataç + kamera + mikrofon. |
| Ataç menüsü (alttan panel) | **Ekleme paneli:** Fotoğraf · Kamera · Belge · Konum · Kişi · Görev · Hatırlatma · Etkinlik · Liste. |
| İlet oku ↪ | **Konuya taşı/gönder** eylemi. |
| Tümü/Okunmamış/Favoriler çipleri | Tümü · Sabitlenenler · (konu türüne göre: Projeler / Alanlar / Kaynaklar) — tasarımcıya bırakıldı. |
| Alt sekmeler | **5 sekme:** Günlük/Konular · Görevler · Takvim · Hatırlatmalar · Ara/Sor (notlara soru sorma). |

### Akış kuralları
- **Ana Günlük = günlük log.** Her kayıt (nereye girilirse girilsin) burada kronolojik görünür.
- **Ana Günlük'e girilen kayıt** → AI kaydın altına soluk çip önerir (ör. "→ GemOlive", "Görev · Perşembe"); kabul edilince kayıt o konuda da görünür.
- **Doğrudan konuya girilen kayıt** → Ana Günlük'te de kendi saatinde, küçük konu etiketiyle görünür.
- Veri modeli açısından: tek Entry akışı; konu = Area/Proje filtresi (docs/ARCHITECTURE.md ile uyumlu).

### Gelen Kutusu
- Ayrı sekme **yok**. İşlenmemiş (AI'ın henüz sınıflandırmadığı / kullanıcının karar vermediği) kayıtlar **Ana Günlük'te işaretle** (nokta/rozet) görünür. Ana ekranda Ana Günlük satırında işlenmemiş sayısı rozeti olabilir.

### Değişmeyenler (görsel dil)
Nötr palet, ferah yoğunluk, grotesk sans, orta köşe (8–12px), belirgin yaylı hareket, açık/koyu/sistem seçimi, kayıt altında AI çipi, alanlar ikonla ayrılır, sesli kayıt = yazma çubuğu üstünde küçük panel + canlı metin, sabah planı = bildirim + Ana Günlük/Bugün üstünde kart.

### Kopya sınırı
WhatsApp'ın **düzen ve etkileşim kalıbı** alınır; yeşil marka rengi, duvar kâğıdı deseni, ikon seti, balon kuyruğu şekli, logo/ad **alınmaz**.

### Faz 1 prototip ekranları (DESIGN_PROCESS listesinin bu kurguya uyarlanmış hali)
1. Ana ekran (Ana Günlük + konular listesi, 5 sekmeli alt çubuk)
2. Ana Günlük akışı (gün ayraçları, AI çipleri, işlenmemiş işareti, sabah planı kartı, sabit yazma çubuğu)
3. Ekleme (ataç) paneli
4. Sesli yakalama paneli
5. Not/kayıt detayı (açılır pencere tipi her yönde farklı denenir)

## Tasarımcılar için özet
- **Ruh:** Nötr renkli, ferah, grotesk yazılı, orta köşeli; ama hareketi canlı (yaylı).
- **Merkez kalıp:** WhatsApp iskeleti — ana ekranda sabitli Ana Günlük + konu listesi; akışta en yeni en altta, altta sabit yazma çubuğu, kayıt altında AI çipleri.
- **Sekmeler (alt çubuk, 5):** Günlük/Konular · Görevler · Takvim · Hatırlatmalar · Ara/Sor.
- **Serbest bırakılanlar:** Not detayı açılır pencere tipi, ana ekrandaki filtre çipleri, Haftalık Gözden Geçirme'ye erişim yolu.
- **Zorunlular:** Açık + koyu tema, WCAG AA, ≥44px dokunma hedefi, gerçek Türkçe içerik, marka/ekran kopyası yok.

## Revizyon 2 — Konu simgeleri otomatik ve renkli (Gökhan, 2026-11-07)
- Tur 3'teki "tek renk ikon" kararı **konu simgeleri için** değişti: her konu paletten kalıcı bir renk alır (8 yumuşak renk, açık/koyu temada simge ≥4,5:1). Diğer ikonlar, metin ve AI çipleri nötr kalır.
- Konu simgesi elle seçilmediyse **konu adından ve konuya düşen kayıtlardan** anlamlı bir simge seçilir; kayıt geldikçe değişebilir. Seçenekler: "Addan anlamlı simge" (seçildi) / "Baş harfli rozet" / "İkisi birden"; renk için "Her konuya kendi rengi" (seçildi) / "Türe göre renk".
- Ayrıntı: `guidelines.md` §ikonlar, `screens/15-konu-simgeleri.html`, `tokens.json` (`color.theme.*.topic-N-*`).

## Revizyon 3 — WhatsApp yapısı (Gökhan, 2026-11-07)
Gökhan: "Kişilerin profilleri gibi yuvarlak içinde görmeliyiz; genel tasarım neden WhatsApp'a
benzemedi (yazı tipi, ikon, renk değil)." Revizyon 1'de kurgu WhatsApp alınmış, ama ekran
düzeni bilinçli olarak WhatsApp'tan ayrıştırılmıştı; bu revizyon **yapıyı** WhatsApp'a yaklaştırır.
Yazı tipi, ikon seti ve renkler NotAI'nin kendi token'ları olarak kalır.
- **Yuvarlak profil:** konu = renkli daire + simge; istenirse profil fotoğrafı (konu menüsü →
  Profil fotoğrafı; ≈96 px'e küçültülür, senkronlanır). Simge elle de değiştirilebilir ("Otomatik"e dönülebilir).
- **Ana ekran:** başlık + tam genişlik arama çubuğu + kaydırılabilir hap filtreler (Tümü · İşlenmemiş ·
  Proje · Alan · Kaynak); tek liste: Ana Günlük ve Haftalık gözden geçirme iğneli satır, sonra konular;
  satır ayracı, "KONULAR" başlığı ve tür etiketi yok; her konuda karar bekleyen kayıt rozeti;
  sağ altta "Yeni kayıt" düğmesi (ana ekranda yazma çubuğu yok — yakalama 2 dokunuş, Gökhan onayladı).
- **Akış:** başlıkta yuvarlak profil; yazma çubuğu hap alan, ataç + kamera alanın içinde, yanında
  ayrı yuvarlak mikrofon/Gönder.
- Ekran görüntüleri: `screens/r3-ana-ekran.png`, `r3-ana-ekran-koyu.png`, `r3-konu-akisi.png`.
  01/02/03 HTML prototipleri bu revizyondan öncedir; bileşenlerin tek kaynağı `screens.css`'tir.

## Revizyon 4 — Konu hiyerarşisi (Gökhan, 2026-11-07)
"Ana başlıklar olmalı: Sabun işi; altında Malzeme hareketleri, Yapılacaklar, Para alışverişi… Her iş ayrı
yazışma olmaz. Ana iş ana ekranda bir kişi gibi; girince en üstte ana konunun yazışması, altında alt
konular (ana ekran gibi). Ana konuya yazılan alta dağıtılır; alt konuya yazılan üst konuda ve Ana
Günlük'te de görünür — aslında tek kayıt. Zamanla Obsidian gibi dallanır."
- Ana ekranda yalnız en üst konular; satırda alt konu sayısı (katman simgesi). Rozet ve son kayıt alt dalları kapsar.
- Alt konusu olan konu açılınca: "<Konu> · genel" iğneli satırı (dalın tüm kayıtları), alt konular, "Yeni kayıt".
- Ana konuya yazılan kayıt için AI yalnız o dalın alt konularını önerir (ya da dalın içinde yeni alt konu).
- Konu menüsü: "Alt konu ekle", "Üst konuyu değiştir". En fazla 5 kat; döngü yok.
- Görüntü: `screens/r4-alt-konular.png`.

