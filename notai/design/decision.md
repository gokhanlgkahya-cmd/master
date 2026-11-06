# Faz 1 — Yön seçimi kararı

> Tarih: 2026-11-06 · Karar veren: Claude (Gökhan'ın `/goal tüm fazları işle` talimatıyla, onay beklemeden).
> Gökhan bu kararı istediği an değiştirebilir; token sistemi sayesinde değişiklik tek dosyada (`design/tokens.json`) yapılır.

## Seçim: **A (Taş Defter) tabanlı karışım** — design-critic'in önerisi
Puanlar (design/review.md): A 7,2 · B 7,0 · C 6,7.

Gerekçe (kısa):
- Gökhan'ın açık isteği "kurgu WhatsApp olsun". A, alışkanlığı en az öğrenmeyle taşıyor (balon akışı, net mikrofon).
- A'nın WhatsApp'a fazla benzeme riski, critic'in önerdiği düzeltmelerle azaltılıyor (saat balonun üstünde, sabit sol oluk, kendi ikonları, yeşil yok).
- B'nin hızı ve ✓/× satırları, C'nin net tipografi ayrıntıları ve Ekle/Oluştur gruplaması alınarak her yönün en güçlü parçası birleşiyor.

## Karışımın bağlayıcı maddeleri
| Konu | Karar |
|---|---|
| Palet | A'nın sıcak taş grisi (`#FAF8F5` … `#171512`) + toprak tonlu semantik renkler |
| Koyu zemin | `#171512` |
| Yazı tipi | Yalnız **Manrope** (gövde + başlık); saat/rakamlarda `font-variant-numeric: tabular-nums`. Mono font yok. |
| Tip ölçeği | 12 / 14 / 16 / 19 / 23 / 28 px; 12px altı yasak |
| Köşe | 8 / 12 / 16 px |
| Boşluk | 4px ızgara; tüm boşluklar `--space-*` değişkeninden |
| Kayıt bileşeni | Kuyruksuz balon (sağa yaslı). Saat + konu etiketi balonun **üstünde** tek satır; işlenmemiş noktası sabit **sol olukta** |
| AI çipi | Şeffaf hap, 1px kesikli kenar `#8E8576` (açık zeminde 3,44:1), metin `#554F45` (7,65:1); çip başına ayrı "Yoksay" (×) — toplu ret yok |
| Sabah planı | A'nın soluk kartı |
| Not detayı | Bottom sheet: %55 durağında B'nin ✓/× öneri satırları; %92 durağında C'nin tam ekran düzeni; tek ana düğme "Görev olarak kaydet" |
| Ekleme paneli | Klavyenin yerine açılır, karartma yok (B); "Ekle / Oluştur" gruplaması (C) |
| Ses paneli | Yazma çubuğu yerinde şeride dönüşür (B) + A'nın ikonlu yapılandırılmış önizleme kartı; arka plan karartılmaz |
| Sekme çubuğu | İkon + etiket hep görünür; akış ekranında gizli (A) |
| Ana ekran filtresi | Tümü \| Proje \| Alan \| Kaynak bölmeli kontrol (B); Haftalık gözden geçirme Ana Günlük'ün altında sabit satır |
| Hareket | B'nin hızı: panel yayı ~370ms, küçük öğe yayı %9 esneme; `prefers-reduced-motion` desteklenir |
| İkonlar | Phosphor tarzı (çizgi) kendi seti; Ana Günlük'e kitaptan farklı ikon |
| Tipografi ayrıntısı | ’ kesme işareti, tire öncesi `&nbsp;` (C) |

## Alternatif
Gökhan balon yerine satır görünümü (B) isterse token'lar aynen kalır, yalnız kayıt bileşeni değişir.
