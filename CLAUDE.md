# CLAUDE.md

## Proje
- Kök dizin: mevcut RiskPlatform (React arayüz + Python: `risk_multipliers.py`, `market_scanner_system.py`, `trading_bot_analysis.py`, `macro_data.py`, `carpan_tablosu_v2.md`).
- Yeni PA işlem sistemi: `pa_system/`. Önce şunları oku:
  1. `pa_system/SISTEM_SPEC.md` — kurallar (tek doğruluk kaynağı)
  2. `pa_system/config/params.yaml` — tüm sayısal değerler
  3. `pa_system/IMPLEMENTATION_PLAN.md` — fazlar ve kabul kriterleri
  4. `pa_system/OPEN_DECISIONS.md` — açık kararlar
  5. `pa_system/schemas/outputs.schema.json` — çıktı şemaları

## Kesin kurallar
- **Lookahead yok:** HTF değerleri yalnız kapanmış mumdan; pivotlar yalnız `confirmed_index`'ten itibaren kullanılır. Her motor değişikliğinde "kesilmiş seri = tam seri" testi geçmeli.
- Sayıları koda sabit yazma; `params.yaml`'dan oku. Her çıktıya `ruleset_version` ve `params_hash` ekle.
- Ekran görüntüsünden sinyal, fiyat, stop/hedef, backtest veya parametre üretme. Görüntü yalnız SPEC §14'teki U1–U8 için.
- Kural mantığını kendi başına değiştirme. Öneri → test (etiket seti + OOS) → kullanıcı onayı → sürüm artışı.
- `OPEN_DECISIONS.md`'deki konularda varsayılanı kullan ve bunu açıkça belirt; karar verme.
- Mevcut RiskPlatform kodunu bozma; entegrasyon `pa_system/pa_engine/risk.py` üzerinden.
- Kaldırılan göstergeleri (MACD, RSI MA5, Stoch RSI) geri ekleme.

## Dil
- Kullanıcı ile Türkçe, kısa ve net. Kod tanımlayıcıları İngilizce, açıklamalar Türkçe olabilir.
