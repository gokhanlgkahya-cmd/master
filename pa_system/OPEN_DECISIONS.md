# Açık Kararlar

Karar verilene kadar `config/params.yaml` içindeki varsayılan geçerlidir. Karar verilince bu dosyada **Karar** sütunu doldurulur ve params güncellenir (PATCH/MINOR sürüm).

| # | Konu | Seçenekler | Varsayılan | Karar |
|---|---|---|---|---|
| 1 | Kullanım modu | Manuel karar desteği · Yarı otomatik (alarm) · Tam otomatik bot | Karar desteği + alarm | |
| 2 | Platform önceliği | Python çekirdek + Pine görsel · Yalnız Pine | Python çekirdek, Pine görsel/parite | |
| 3 | BIST short | Long-only · VİOP ile short | Long-only | |
| 4 | BIST evreni | BIST30 · BIST100 | BIST30 | |
| 5 | BIST veri kaynağı | yfinance (sınırlı) · Matriks · İş Yatırım · TV export · diğer | ? | |
| 6 | Altın/gümüş enstrümanı | Spot/CFD · VİOP · COMEX vadeli | ? (RVOL buna bağlı) | |
| 7 | Kripto | Spot · Perp; kaldıraç | Spot, kaldıraçsız | |
| 8 | Öncelikli profil | P1 4H/15m · P2 1D/1H · P3 1W/4H | P2 | |
| 9 | İşlem başı risk / açık risk | — | — | ✅ İşlem max %2 · açık toplam max %6 (güncel stopa göre) · grup max %3 · aylık zarar limiti %6 |
| 10 | Hedef profil | Düşük win% + yüksek R · Sık küçük kazanç | Düşük win% + yüksek R | |
| 11 | Pivot yöntemi | Hibrit (fraktal + ATR + alternasyon) · Saf ATR zigzag | Hibrit | |
| 12 | Trend çizgisi teyidi | 2 temas · 3 temas şart | 2 temas = ARMED, 3 temas = kalite | |
| 13 | Çizgi kırılımında giriş | CHoCH onayı · Agresif yarım risk | CHoCH onayı | |
| 14 | Trailing TF | Giriş TF · Ara TF · HTF | Ara TF | |
| 15 | BIST P2 trailing | 1H ara pivot · 1D ara pivot | 1D ara pivot | |
| 16 | Sığ geri çekilme (pd 0.50–0.62) | Kapalı · Yarım risk | Kapalı | |
| 17 | Yatay piyasada aralık içi işlem | Kapalı · PD ile mean-reversion | Kapalı | |
| 18 | Ç3 değişimi | Kalite notu ile tamamen değiştir · Yalnız ADX kısmını çıkar | Tamamen değiştir | |
| 19 | Tarama frekansı | Günlük kapanış · Seans içi saatlik | Günlük kapanış | |
| 20 | Paper trading süresi | 4 · 8 · 12 hafta | 8 hafta | |
| 21 | Bildirim kanalı | Telegram · e-posta · HTML rapor | HTML rapor + Telegram | |
| 22 | Orkestrasyon | Hermes Agent (Nous Research?) · Claude Code zamanlanmış görev · cron | Netleştirilecek | |
| 23 | Kısmi kâr ve başabaş | 1.5R %40 + BE · 1R + BE · kısmi yok | 1.5R %40 + BE (test edilecek) | |
| 24 | Kademeli giriş | Kapalı · İlk parça dar stop + yapı teyidinde ekleme (toplam ≤ %2) | Kapalı | |
| 25 | Pozisyon tavanları | — | — | ✅ BIST 25 · BTC 30 · ETH 20 · XAU 30 · XAG 15 (sermaye %); kaldıraç yok; gap kontrolü açık |
| 26 | Ç2 seviye/yön ayrımı | — | — | ✅ Seviye (VIX rejimi, vade yapısı) risk'te; yön (VVIX, DXY, US10Y, XU100/sektör, BTC.D) taramada makro rüzgar |
| 27 | Türkiye CDS | — | — | ✅ v1 kapsam dışı |
| 28 | HTF pivot ve stop | — | — | ✅ HTF ana pivot = trend geçersizlik seviyesi; stop giriş TF pivotundan |
| 29 | RSI 30 + RSI EMA20 sinyali | Ana tetik · Mevcut tetiklere ek teyit; hangi TF | Ek teyit (alternatif tetik), LTF; backtest karşılaştırması sonrası karar (KONUSMA_OZETI §4) | |
| 30 | Temel veri kaynağı | İş Yatırım (gayriresmî) · KAP XBRL · Fintables/Matriks (ücretli) | İş Yatırım (deneme), üretim için netleşecek | |
| 31 | Temel analiz evreni | BIST30 · BIST100 | BIST30 (banka + holding şablonları gerekli) | |
| 32 | Büyüme birimi | Reel TL (TÜFE) · USD | İkisi de hesaplanır, skorda reel TL (deneme USD ile yapıldı) | |
| 33 | Analist tahminleri | Var · Yok | v1'de yok | |
