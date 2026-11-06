# NotAI — Tasarım Brief'i (Faz 0 çıktısı)

> Tarih: 2026-11-06 · Kaynak: Gökhan ile 4 soru turu + 1 netleştirme turu (AskUserQuestion)

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

## Tasarımcılar için özet
- **Ruh:** Nötr renkli, ferah, grotesk yazılı, orta köşeli; ama hareketi canlı (yaylı).
- **Merkez kalıp:** WhatsApp benzeri günlük akış + altta sabit yazma çubuğu + kayıt altında AI çipleri.
- **Sekmeler:** Bugün · Gelen Kutusu · Görevler · Takvim (alt çubuk).
- **Serbest bırakılanlar:** Açılır pencere tipi, ikincil bölümlere (PARA, Arama/Sohbet, Haftalık GG) erişim yolu.
- **Zorunlular:** Açık + koyu tema, WCAG AA, ≥44px dokunma hedefi, gerçek Türkçe içerik, marka/ekran kopyası yok.
