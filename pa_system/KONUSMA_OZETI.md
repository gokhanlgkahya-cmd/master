# Konuşma Özeti ve Karar Günlüğü

Kaynak: Claude Code bulut oturumu (Kasım 2026). Bu dosya, oturumun yerel Claude Code CLI'a taşınmasında bağlam kaybını önlemek için yazıldı. Kurallar için tek doğruluk kaynağı `SISTEM_SPEC.md` + `config/params.yaml`; burası **neden** ve **tartışma geçmişi**dir.

## 1. Başlangıç
- Başka bir modelin yazdığı "Çoklu zaman dilimli piyasa analiz mimarisi" taslağı incelendi. Alınan: HTF yön / LTF zamanlama, "işlem yok bir karardır", 4 rejim, kapanmış mum kuralı. Atılan: MACD, RSI MA5, Stoch RSI, sayısal olmayan eşikler, çıkışsız yapı, boş karar matrisi.
- Kapsam: BTC, ETH, altın, gümüş, BIST hisseleri.

## 2. Kullanıcının yönlendirmeleri (kararların kaynağı)
1. **PA ana katman:** pivot tabanlı trend, stop = son dip − ATR, pivot trailing; göstergeler yalnız destek.
2. **Trend çizgisi kırılımı** (alçalan tepeler çizgisi) değerli kırılım türü.
3. **Premium/discount:** HTF son dip–tepe aralığının %50'sine göre konum.
4. PD ve RSI farkı tartışıldı → PD konum, RSI güç; PD HTF'de, RSI LTF'de; aynı bilgi iki kez sayılmaz. **Stoch RSI kullanılmaz.**
5. **Uyumsuzluk:** gizli + trend yönündeki klasik = teyit; HTF'ye karşı klasik = yalnız stop sıkılaştırma. Tetik değildir.
6. Çıktı "AL/SAT/TUT %70" yerine aksiyon + seviye + geçersizlik + kurulum istatistiği (güven yüzdesi yok).
7. **Tarama çarpanı:** BIST30 vb. evrende hangi hisse hangi TF'de daha uygun → G × ΣwS × M.
8. **Pivot tespiti kodla** (hibrit: fraktal + ATR boyut + alternasyon); göz yalnız kalibrasyon için.
9. **Ekran görüntüsü:** otomatik kod revizyonu döngüsü reddedildi; görüntü yalnız U1–U8 (bilgi aktarma, kontrol, günlük, tarama özeti). Kırılım tespiti tamamen veriyle.
10. **Pozisyon boyutu = risk / stop mesafesi** (kullanıcı örneği: haftalık dip %50 uzakta → %4 pozisyon; 4H dip %10 → %20 pozisyon; ikisi de %2 risk). HTF dibi stop değil, trend geçersizlik seviyesi.
11. **Risk limitleri:** işlem max %2, açık toplam max %6 (güncel stopa göre), grup max %3, aylık zarar limiti %6, pozisyon tavanları (BIST 25, BTC 30, ETH 20, XAU 30, XAG 15).
12. **Makro yönelim:** VIX/VVIX/korele pariteler skorlamaya **yön** olarak girer (seviye Ç2'de kalır). Dinamik korelasyon ağırlıklı "makro rüzgar". Türkiye CDS v1'de yok.
13. **Temel analiz modülü** (BIST): temel metriklerin teknik analizi → watchlist sütunu + çarpan. Bkz. `TEMEL_ANALIZ_MIMARI.md`. SASA ile deneme yapıldı.

## 3. Ekran görüntüsü okumaları (U7 örnekleri — emir değil)
- **BTCUSDT 1G (Kasım 2026):** ana dip ~58k, ana tepe ~87k, 74.5k HL kırıldı (CHoCH aşağı), fiyat ~62k derin discount ama yapı aşağı → İŞLEM YOK / İZLE. Long: 58–59.5k tepki + LTF CHoCH (yarım risk); short: 70–75k tepkisinde LH.
- **XAUUSD 1H/1G/4s (26 Kasım 2026):** zirve ~5.600 → alçalan kanal; 2024'ten gelen trend çizgisi ~3.900–4.000'de kırıldı; fiyat ~3.575, 2025 aralık tepesi ~3.515 (kullanıcı çizgisi) üzerinde; 4H'de 3.515 altı süpürme ~3.450. Haftalık PD (3.126→5.600) ≈ 0.18. Karar: **İŞLEM YOK** (long: HTF aşağı + 0.7R; short: aşırı uzama).
  - Senaryo A short (öncelikli): 3.900–4.000 premium + kırılan çizgi retesti + 4H LH → A kalite %1.5 (XAU tavanı %30 ile fiilen ≈ %1).
  - Senaryo B long: günlük ~3.700–3.750 üzeri CHoCH → B kalite %1.
  - Senaryo C: 3.450–3.900 yatay → yalnız aralık kırılımı (S3).
  - Kullanıcının spot/CFD/vadeli tercihi (short mümkün mü) açık (#6).

## 4. Tartışmada kalan öneri: RSI 30 + RSI EMA20 sinyali
Kullanıcı önerisi: "RSI 30'u yukarı kestikten sonra RSI, EMA20'sini yukarı keserse gir."
- Çözdüğü: düşen bıçak yerine aşırı satımdan çıkışı bekler; iki aşamalı (ARMED → tetik); objektif, repaint yok.
- Eksik: trend bağlamı (güçlü düşüşte başarısız), fiyat/yapı teyidi, stop/hedef, gecikme (R/R bozulur), TTL/geçersizlik, TF tanımı, uyumsuzluk, rejim D filtresi.
- Önerilen uyarlama: bağlam (HTF UP + discount veya aralık dibi, rejim ≠ D) → ARMED RSI<30 (uptrend'de <40) → tetik: RSI 30 yukarı + 10 mum içinde RSI > EMA20(RSI) **ve** fiyat son `ara` tepe üstü kapanış → iptal: RSI tekrar <30 veya dip altı kapanış → stop dip − k×ATR, TP1 ≥ 2R; uyumsuzluk varsa A, yoksa B.
- Sisteme giriş biçimi: yeni puan değil, **alternatif tetik teyidi**; backtest'te mevcut tetiklerle rejim bazında karşılaştırılacak. Açık karar #29 (TF ve ana tetik mi / ek teyit mi — kullanıcı yanıtı bekleniyor).

## 5. Ortam notları
- Bulut ortamına `www.isyatirim.com.tr` ağ izni eklendi (kullanıcı onayıyla). Yerelde gerekmez.
- Repo kökündeki mevcut RiskPlatform kodu (`risk_multipliers.py` vb.) değiştirilmedi; entegrasyon `pa_engine/risk.py` üzerinden planlandı.
- Çalışma branch'i: `ccr-b14fadc4-722b1f`.

## 6. Sıradaki adımlar
1. Açık kararların netleştirilmesi (özellikle #5 BIST veri, #6 altın enstrümanı, #8 profil, #22 Hermes, #29 RSI sinyali, #30–33 temel analiz).
2. F0 (mevcut kod incelemesi) ve F2 (göstergeler + pivot motoru) — açık kararlardan bağımsız başlatılabilir.
