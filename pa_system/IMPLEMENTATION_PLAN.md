# Uygulama Planı (Claude Code için)

Kaynak: `SISTEM_SPEC.md` (kurallar) · `config/params.yaml` (sayılar) · `schemas/outputs.schema.json` (çıktılar) · `OPEN_DECISIONS.md` (açık noktalar).
Her faz ayrı branch/PR; bir faz kabul kriterlerini geçmeden sonrakine geçilmez.

## Önerilen modül yapısı

```
pa_system/
  pa_engine/
    data/          # veri çekme, temizlik, düzeltme, kalite kontrol
    indicators.py  # ATR, EMA, ER, RSI, BB, KC, RVOL, yüzdelikler
    pivots.py      # hibrit pivot motoru (ana/ara, confirmed_index)
    structure.py   # UP/DOWN/RANGE, BOS, CHoCH, PD aralığı
    levels.py      # yatay seviye kümeleri, trend çizgileri, squeeze kutusu
    breakout.py    # kırılım olayı, teyit, başarısız kırılım, etiket
    divergence.py  # RSI gizli/klasik uyumsuzluk
    regime.py      # D→C→B→A, histerezis, confirm_bars
    setups.py      # S1/S2/S3, kalite notu
    state.py       # IDLE/ARMED/.../COOLDOWN durum makinesi
    exits.py       # stop, kısmi, trailing, yapısal/rejim/zaman çıkışı
    risk.py        # risk_multipliers.py entegrasyonu (Ç3 → kalite)
    scanner.py     # G × ΣwS × M, profil matrisi, tier, histerezis
    backtest/      # olay tabanlı motor, maliyetler, walk-forward, rapor
    render/        # mplfinance şablonları (bar index'li)
    review/        # U1–U8 akışları, Claude vision istem şablonları
  config/params.yaml
  schemas/outputs.schema.json
  labels/          # pivot etiket setleri (JSON)
  journal/         # işlem günlüğü + görüntüler
  reports/
  tests/
```

## Fazlar

| Faz | İş | Kabul kriteri |
|---|---|---|
| **F0** Mevcut kod incelemesi | `risk_multipliers.py`, `market_scanner_system.py`, `trading_bot_analysis.py`, `macro_data.py` incelenir; yeniden kullanılacaklar (risk çarpanları, ekonomik takvim, veri çekme) belirlenir | Kısa entegrasyon notu; mevcut uygulama bozulmaz |
| **F1** Veri katmanı | Kripto (ccxt/Binance), BIST ve metal kaynakları (Açık #5, #6); makro sürücüler (VIX, VVIX, DXY, US10Y, NDX, EEM, USDTRY, HG, Brent, EURUSD, BTC.D — `macro_data.py` genişletilir); yeterli geçmiş (≥ 3 yıl HTF); düzeltilmiş veri; boşluk/eksik mum kontrolü; gap istatistiği; parquet önbellek | Veri kalite raporu; mevcut `fetch_stock` 200 bar sınırı aşılmış; sürücü zaman hizalama testi (kapanmamış mum kullanılmıyor) |
| **F2** Göstergeler + pivot motoru | `indicators.py`, `pivots.py` | Birim testleri; **lookahead testi** (kesilmiş seri = tam seri) geçer; `confirmed_index` her pivotta |
| **F3** Etiketleme + kalibrasyon | Render (bar index'li), U2 ön etiketleme akışı, kullanıcı düzeltme, eşleşme metriği | Enstrüman başına 50–100 etiket; seçilen parametrelerin eşleşme oranı raporlanır; parametreler sabitlenir |
| **F4** Yapı / PD / rejim / seviye / çizgi / kırılım / uyumsuzluk | `structure`, `regime`, `levels`, `breakout`, `divergence` | Lookahead testi; örnek grafiklerde U4 görsel kontrol |
| **F5** Kurulumlar + durum makinesi + çıkışlar | `setups`, `state`, `exits`, `risk` (SPEC §11: %2/%6/grup %3/aylık %6, güncel stopa göre açık risk, pozisyon tavanı, gap kontrolü, Ç2 `level_only` = mevcut `calc_macro_multiplier` nötr yön girdileriyle) | Her kural için birim testi; limit testleri (ör. 4. pozisyon %6'yı aşarsa reddedilir; BE'deki pozisyon riski 0); sinyal kartı şemaya uygun |
| **F6** Backtest + doğrulama | Olay tabanlı motor, maliyetler, walk-forward, OOS, rejim/kurulum kırılımı | SPEC §15 metrik raporu; enstrüman × profil |
| **F7** Tarayıcı | Skor, profil matrisi, tier, sektör sınırı, histerezis, makro rüzgar (dinamik korelasyon ağırlıklı yön) ve VIX şoku elemesi | Desil monotonluk testi raporu (rüzgarlı / rüzgarsız karşılaştırma); `scan_row` şemaya uygun |
| **F8** Ekran görüntüsü katmanı | U3 repaint, U5 journal, U6 tarama özeti, U8 haftalık rapor, veto log | Görüntü metadata şemaya uygun; haftalık rapor üretiliyor |
| **F9** Pine v6 gösterge | Pivot/yapı/PD/çizgi görsel göstergesi | Python ↔ Pine parite testi (aynı sinyal zamanları) |
| **F10** Canlı (paper) | Alarm/bildirim, zamanlanmış tarama | Paper trading süresi (Açık #20) tamamlanır, rapor onaylanır |

## Kalıcı kurallar (her fazda)
- Sayılar `params.yaml`'dan; her çıktı `ruleset_version` + `params_hash` taşır.
- Lookahead testi CI'da zorunlu.
- Görüntüden fiyat/sinyal/parametre üretilmez (SPEC §14.1).
- Kural mantığını değiştiren her PR: SPEC güncellemesi + sürüm artışı + backtest karşılaştırması.
- Açık karar gerektiren yerde varsayılanı kullan, PR açıklamasında belirt; kendi başına karar verme.
