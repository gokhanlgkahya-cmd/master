# RiskPlatform — Çarpan Tabloları v2.0 (Adaptif Sistem)
## Versiyon: 2.0 — Mart 2026
## Portföy: $100,000 | Max Aylık Risk: %6 | İşlem Risk Aralığı: %0.5 – %2.0

---

## GENEL BAKIŞ

### Sabit Kurallar → Adaptif Kurallar
v1 sabit BASE × 3 çarpan kullanıyordu.
v2 dinamik BASE × 7 çarpan kullanır — piyasa rejimi, portföy durumu,
likidite ve zaman faktörlerini otomatik adapte eder.

### Birleşik Formül

```
risk% = clamp(BASE × Ç1 × Ç2 × Ç3 × Ç5 × Ç6 × Ç7, 0.5, 2.0)

BASE  = Ç4 (Kelly Criterion — veri varsa dinamik, yoksa 1.0)
Ç1    = Korelasyon çarpanı
Ç2    = Makro yönlendirme çarpanı (VIX Term Structure dahil)
Ç3    = 1D teknik çarpan (ADX tükenme revizyonu dahil)
Ç5    = Drawdown protokolü çarpanı
Ç6    = Likidite faktörü çarpanı
Ç7    = Zaman çürümesi çarpanı

Final: risk% = min(risk%, kalan_risk)
       kalan_risk = %6 - açık_toplam_risk
```

---

## Ç4 — KELLY CRITERION (DİNAMİK BASE)

### Amaç
Sabit %1.0 BASE yerine, strateji performansına göre dinamik BASE.
Kazanan strateji daha fazla risk alır, kaybeden strateji otomatik küçülür.

### Formül

```python
def calc_kelly_base(win_rate, avg_rr):
    """
    Quarter Kelly — tam Kelly'nin 1/4'ü kullanılır.
    Tam Kelly aşırı agresif, quarter Kelly optimal risk-adjusted return verir.
    
    win_rate: Son 50 işlemin kazanma oranı (0.0 - 1.0)
    avg_rr:   Ortalama kazanç/kayıp oranı (R/R ratio)
    """
    if win_rate < 0.40 or avg_rr < 1.0:
        # Yetersiz performans — minimum BASE
        return 0.5
    
    # Kelly formülü: f* = (p × b - q) / b
    # p = win rate, q = 1-p, b = avg R/R
    p = win_rate
    q = 1 - p
    b = avg_rr
    
    full_kelly = (p * b - q) / b
    quarter_kelly = full_kelly * 0.25
    
    # BASE olarak clamp
    base = max(0.5, min(2.0, quarter_kelly * 100))
    # quarter_kelly × 100 çünkü Kelly oran veriyor, biz % olarak kullanıyoruz
    
    return round(base, 2)
```

### Kelly BASE Referans Tablosu

| Win Rate | Avg R/R | Full Kelly | Quarter Kelly | BASE   | Yorum                     |
|----------|---------|------------|---------------|--------|---------------------------|
| %40      | 1.0     | %0.0       | %0.0          | 0.50   | Yetersiz — minimum        |
| %45      | 1.2     | %7.5       | %1.9          | 0.50   | Zayıf — minimum           |
| %50      | 1.5     | %16.7      | %4.2          | 0.50   | Başabaş — minimum         |
| %52      | 1.5     | %18.7      | %4.7          | 0.50   | Hafif pozitif              |
| %55      | 1.5     | %21.7      | %5.4          | 0.54   | İyi — hafif agresif        |
| %55      | 2.0     | %27.5      | %6.9          | 0.69   | İyi — orta agresif         |
| %58      | 1.8     | %26.7      | %6.7          | 0.67   | Güçlü — agresif            |
| %60      | 2.0     | %30.0      | %7.5          | 0.75   | Çok güçlü                  |
| %60      | 2.5     | %34.0      | %8.5          | 0.85   | Mükemmel                   |
| %65      | 2.0     | %32.5      | %8.1          | 0.81   | Elite                      |
| %70      | 2.5     | %38.0      | %9.5          | 0.95   | Üst düzey                  |

### Uygulama Kuralları

**Kural 4a — Minimum Veri Gereksinimi:**
Son 50 işlem tamamlanmadan Kelly hesaplanmaz → BASE = 1.0 (sabit)
50-100 işlem arası → Kelly × 0.5 (yarı güven)
100+ işlem → Kelly tam uygulanır

**Kural 4b — Rolling Pencere:**
Son 50 işlem rolling — her yeni işlem girdiğinde en eski düşer.
Bu sayede performans bozulursa BASE otomatik düşer.

**Kural 4c — Sinyal Tipi Bazlı Kelly (İleri Seviye):**
Her sinyal tipi için ayrı Kelly tutulabilir:
- Squeeze sinyalleri: 30 işlem sonrası → ayrı win rate / R/R
- Trend sinyalleri: 30 işlem sonrası → ayrı win rate / R/R
Bu sayede squeeze %65 win rate ile çalışırken range %48 ise,
squeeze BASE yükselir, range BASE düşer.
→ Başlangıçta tek Kelly yeterli, sinyal bazlı ayrım 6 ay sonra.

### Ç4 Örnek Hesaplama

```
Son 50 işlem: 28 kazanç, 22 kayıp
Win rate: 28/50 = %56
Ortalama kazanç: $1,200, Ortalama kayıp: $700
Avg R/R: 1200/700 = 1.71

Full Kelly: (0.56 × 1.71 - 0.44) / 1.71 = 0.302 → %30.2
Quarter Kelly: %30.2 / 4 = %7.55
BASE = max(0.5, min(2.0, 0.755)) = 0.76

→ BASE eskiden 1.0'dı, şimdi 0.76 — strateji iyi ama mükemmel değil,
  sistem hafif muhafazakar davranıyor.
```

---

## Ç5 — DRAWDOWN PROTOKOLÜ

### Amaç
Seri kayıp veya drawdown durumunda otomatik risk azaltma.
Psikolojik koruma — "tilt" durumunda sistem seni frenllesin.
%6 aylık limit ayrı, bu ek katman.

### Drawdown Hesaplama

```python
def calc_drawdown_multiplier(portfolio_value, peak_value, consecutive_losses):
    """
    İki girdi:
    1. Portföy drawdown yüzdesi (peak'ten düşüş)
    2. Art arda kayıp sayısı
    
    İkisinden baskın olan (daha düşük çarpan veren) uygulanır.
    """
    # Drawdown bazlı
    dd_pct = (peak_value - portfolio_value) / peak_value * 100
    
    if dd_pct < 2:
        dd_mult = 1.0    # Normal
    elif dd_pct < 5:
        dd_mult = 0.85   # Dikkatli ol
    elif dd_pct < 10:
        dd_mult = 0.65   # Risk azalt
    elif dd_pct < 15:
        dd_mult = 0.45   # Sadece yüksek konviksiyon
    else:
        dd_mult = 0.25   # Moratoryum eşiği
    
    # Seri kayıp bazlı
    if consecutive_losses <= 1:
        streak_mult = 1.0
    elif consecutive_losses == 2:
        streak_mult = 0.85
    elif consecutive_losses == 3:
        streak_mult = 0.65  # Mevcut H3 kuralıyla uyumlu
    elif consecutive_losses == 4:
        streak_mult = 0.45
    else:
        streak_mult = 0.25  # 5+ art arda kayıp — ciddi sorun
    
    # Baskın olan (düşük olan) uygulanır
    return min(dd_mult, streak_mult)
```

### Ç5 Detay Tablosu

#### Drawdown Bazlı

| Drawdown       | Ç5     | Aksiyon                               | Renk     |
|----------------|--------|---------------------------------------|----------|
| <%2            | ×1.00  | Normal operasyon                      | 🟢 Yeşil |
| %2 – %5       | ×0.85  | Dikkatli ol, yeni işlem seçici        | 🟡 Sarı  |
| %5 – %10      | ×0.65  | Risk azalt, sadece A/S sınıfı sinyal | 🟠 Turuncu|
| %10 – %15     | ×0.45  | Yüksek konviksiyon, max 1 yeni işlem | 🔴 Kırmızı|
| >%15           | ×0.25  | Moratoryum — 24-72 saat yeni işlem yok| ⛔ Siyah  |

#### Seri Kayıp Bazlı

| Art Arda Kayıp | Ç5     | Aksiyon                               | Psikoloji        |
|----------------|--------|---------------------------------------|------------------|
| 0-1            | ×1.00  | Normal                                | Sakin             |
| 2              | ×0.85  | Hafif fren                            | Dikkatli          |
| 3              | ×0.65  | Belirgin fren (H3 kuralı teyit)      | Gözden geçir      |
| 4              | ×0.45  | Ciddi fren                            | Strateji sorgula  |
| 5+             | ×0.25  | Neredeyse durdur                      | Mola ver          |

### Ç5 Ek Kuralları

**Kural 5a — Moratoryum Detayı (Ç5 ≤ 0.25):**
```
Drawdown >%15 VEYA 5+ art arda kayıp:
→ 24 saat: Yeni işlem hard block
→ 48 saat: Sadece altın/hedge pozisyon, max %0.5 risk
→ 72 saat: Normal operasyona dön, Ç5 = 0.45'ten başla (kademeli)
```
Platform moratoryum süresini countdown olarak gösterir.
Moratoryum boyunca açık pozisyonlar yönetilmeye devam eder (stop, scale).

**Kural 5b — Toparlanma Rampası:**
Drawdown'dan çıkış ani değil, kademeli olur:
```
%15'ten %10'a düştü → Ç5 hemen 0.45'e çıkmaz
→ İlk 3 gün: 0.35 (temkinli)
→ Sonraki 3 gün: 0.45 (normal)
→ Kazanan işlem gelirse: bir kademe yukarı (0.65)
```

**Kural 5c — Drawdown Peak Reset:**
Portföy yeni ATH yaptığında peak güncellenir.
Ayın 1'inde aylık risk limiti (%6) resetlenir.
Drawdown hesabı aylık değil, rolling peak'ten hesaplanır.

### Ç5 Örnek Hesaplama

```
Portföy: $100,000 → peak $102,000 → şu an $95,500
Drawdown: ($102,000 - $95,500) / $102,000 = %6.37
Son işlemler: K K K (3 art arda kayıp)

DD bazlı: %6.37 → %5-10 bandı → ×0.65
Streak bazlı: 3 kayıp → ×0.65
Ç5 = min(0.65, 0.65) = ×0.65

Etki: BASE 1.0 olan bir işlem artık max 0.65 ile çarpılır.
Tüm diğer çarpanlar da bunun üstüne gelir.
risk% = 1.0 × Ç1 × Ç2 × Ç3 × 0.65 × Ç6 × Ç7
→ Sistem kendini otomatik koruma altına aldı.
```

---

## Ç6 — LİKİDİTE FAKTÖRÜ

### Amaç
Likit olmayan enstrümana büyük pozisyon açmayı engelle.
BIST'te özellikle kritik — bazı hisseler günlük 50K TL hacim görüyor.

### Hesaplama

```python
def calc_liquidity_multiplier(avg_daily_volume_usd, spread_pct, position_size_usd):
    """
    Üç faktör:
    1. Ortalama günlük hacim (USD)
    2. Spread yüzdesi (bid-ask farkı / mid price)
    3. Pozisyon boyutunun hacme oranı
    """
    # Hacim skoru
    if avg_daily_volume_usd >= 10_000_000:    # $10M+
        vol_score = 1.0
    elif avg_daily_volume_usd >= 5_000_000:   # $5M+
        vol_score = 0.95
    elif avg_daily_volume_usd >= 1_000_000:   # $1M+
        vol_score = 0.90
    elif avg_daily_volume_usd >= 500_000:     # $500K+
        vol_score = 0.80
    elif avg_daily_volume_usd >= 100_000:     # $100K+
        vol_score = 0.65
    elif avg_daily_volume_usd >= 50_000:      # $50K+
        vol_score = 0.50
    else:
        vol_score = 0.0  # Hard block — çok düşük hacim
    
    # Spread skoru
    if spread_pct <= 0.05:      # <%0.05
        spread_score = 1.0
    elif spread_pct <= 0.10:    # <%0.10
        spread_score = 0.95
    elif spread_pct <= 0.20:    # <%0.20
        spread_score = 0.90
    elif spread_pct <= 0.30:    # <%0.30
        spread_score = 0.80
    elif spread_pct <= 0.50:    # <%0.50
        spread_score = 0.65
    else:
        spread_score = 0.50     # >%0.50 çok geniş spread
    
    # Pozisyon/Hacim oranı skoru
    # Pozisyon günlük hacmin %2'sinden fazla olmamalı
    pv_ratio = position_size_usd / avg_daily_volume_usd if avg_daily_volume_usd > 0 else 999
    if pv_ratio <= 0.01:        # <%1 — sorun yok
        pv_score = 1.0
    elif pv_ratio <= 0.02:      # <%2 — kabul edilebilir
        pv_score = 0.90
    elif pv_ratio <= 0.05:      # <%5 — dikkatli ol
        pv_score = 0.70
    elif pv_ratio <= 0.10:      # <%10 — riskli
        pv_score = 0.50
    else:
        pv_score = 0.0          # >%10 — hard block, piyasa etkisi çok büyük
    
    # En düşük skor baskın
    return min(vol_score, spread_score, pv_score)
```

### Ç6 Referans Tablosu — Varlık Sınıfı Bazlı

#### Kripto (Binance)

| Enstrüman  | Ort. Günlük Hacim | Spread  | Ç6 Beklenti | Not                    |
|------------|-------------------|---------|-------------|------------------------|
| BTC/USDT   | $15B+             | %0.01   | ×1.00       | Sıfır likidite sorunu  |
| ETH/USDT   | $5B+              | %0.01   | ×1.00       | Sıfır likidite sorunu  |
| SOL/USDT   | $1B+              | %0.02   | ×1.00       | Sorun yok              |
| Orta kap   | $50M-500M         | %0.03   | ×0.95       | Genelde sorun yok      |
| Küçük kap  | $5M-50M           | %0.10   | ×0.80-0.90  | Dikkatli boyutlandır   |

→ Kripto'da Ç6 genelde 0.95-1.00 arası, büyük sorun çıkmaz.

#### BIST Hisse

| Enstrüman  | Ort. Günlük Hacim | Spread  | Ç6 Beklenti | Not                    |
|------------|-------------------|---------|-------------|------------------------|
| THYAO      | 3-5B TL           | %0.05   | ×1.00       | En likit BIST hisse    |
| GARAN      | 2-4B TL           | %0.05   | ×1.00       | Çok likit              |
| AKBNK      | 1-3B TL           | %0.08   | ×0.95       | Likit                  |
| EREGL      | 500M-1B TL        | %0.10   | ×0.90       | Yeterli                |
| SODA       | 200-500M TL       | %0.15   | ×0.85       | Orta                   |
| TATGD      | 50-200M TL        | %0.25   | ×0.70       | Dikkatli               |
| BANVT      | 20-50M TL         | %0.40   | ×0.50       | Riskli — küçük poz     |

→ BIST'te Ç6 çok kritik. BANVT gibi düşük hacimli hisselerde pozisyon
  boyutu otomatik küçülür.

#### Kıymetli Maden

| Enstrüman  | Ort. Günlük Hacim | Spread  | Ç6 Beklenti | Not                    |
|------------|-------------------|---------|-------------|------------------------|
| XAU/USD    | $100B+ (futures)  | %0.02   | ×1.00       | Dünyanın en likit      |
| XAG/USD    | $20B+             | %0.03   | ×1.00       | Çok likit              |

### Ç6 Ek Kuralları

**Kural 6a — Piyasa Saati Düzeltmesi (BIST):**
```
BIST açılış ilk 15 dk (09:30-09:45): Spread 2-3x genişler → Ç6 × 0.7
BIST kapanış son 15 dk (17:45-18:00): Spread genişler → Ç6 × 0.8
BIST öğle 12:30-13:30: Hacim düşer → Ç6 × 0.9
Diğer saatler: Ç6 normal
```
Kripto 24/7 olduğu için saat düzeltmesi gerekmez.

**Kural 6b — Hacim Anomalisi Uyarısı:**
Günlük hacim 20-günlük ortalamının 3x üzerindeyse:
→ Olağandışı aktivite — habersel kontrol gerekli
→ Ç6'yı düşürme ama "⚠ Hacim Anomalisi" uyarısı göster

**Kural 6c — Dinamik Güncelleme:**
Hacim ve spread verisi her refresh'te güncellenir (saatlik).
20 günlük ortalama hacim haftalık taramada hesaplanır.

### Ç6 Örnek Hesaplama

```
Aday: SODA.IS
Ort. günlük hacim: 350M TL ≈ $10M (USD/TRY 35)
Spread: %0.15
Hesaplanan pozisyon boyutu: $1,500 (risk% × portföy hesabından)

vol_score: $10M → ×1.0 (güçlü)
spread_score: %0.15 → ×0.90
pv_ratio: $1,500 / $10M = 0.00015 → ×1.0 (sorun yok)

Ç6 = min(1.0, 0.90, 1.0) = ×0.90

→ Spread biraz geniş, %10 ceza.
  $100K portföy için pozisyon boyutları BIST'te genelde sorun çıkarmaz.
  Ama portföy büyürse ($1M+) Ç6 daha önemli hale gelir.
```

---

## Ç7 — ZAMAN ÇÜRÜMESİ

### Amaç
Pozisyon uzun süre açık kalırsa risk azalt. Arkasındaki mantık:
Sinyal "taze" iken en geçerli. Zaman geçtikçe piyasa koşulları değişir,
orijinal giriş tezi zayıflar. Ayrıca sermaye bağlayan pozisyon fırsat maliyeti yaratır.

### Hesaplama

```python
def calc_time_decay_multiplier(days_open, trade_type='swing'):
    """
    trade_type:
    - 'swing': 5-10 gün beklenti (4H sinyalleri)
    - 'position': 20-30 gün beklenti (1D sinyalleri)
    
    Formül: Ç7 = max(0.60, 1 - (days / threshold × decay_rate))
    """
    if trade_type == 'swing':
        threshold = 10   # 10 gün sonra çürüme hızlanır
        decay_rate = 0.04  # Günde %4 çürüme
    else:  # position
        threshold = 25   # 25 gün sonra çürüme hızlanır
        decay_rate = 0.02  # Günde %2 çürüme
    
    if days_open <= threshold * 0.5:
        # İlk yarı — çürüme yok
        return 1.0
    elif days_open <= threshold:
        # İkinci yarı — yavaş çürüme
        excess = days_open - threshold * 0.5
        return max(0.60, 1.0 - excess * decay_rate * 0.5)
    else:
        # Threshold aşıldı — hızlı çürüme
        excess = days_open - threshold
        base_decay = threshold * 0.5 * decay_rate * 0.5
        return max(0.60, 1.0 - base_decay - excess * decay_rate)
```

### Ç7 Referans Tablosu

#### Swing Trade (4H sinyalleri — beklenti 5-10 gün)

| Gün   | Ç7     | Durum                                  |
|-------|--------|----------------------------------------|
| 1-5   | ×1.00  | Taze — tam güç                         |
| 6     | ×0.98  | Hafif yaşlanma başlıyor                |
| 7     | ×0.96  | Normal swing süresi dolmak üzere       |
| 8     | ×0.94  | Uzuyor — çıkış düşün                   |
| 9     | ×0.92  | Threshold'a yaklaşıyor                 |
| 10    | ×0.90  | Threshold — pozisyon gözden geçir      |
| 12    | ×0.82  | Hızlı çürüme — kapat veya stop sıkıla |
| 15    | ×0.70  | Ciddi çürüme                           |
| 20+   | ×0.60  | Minimum — kapat önerisi                |

#### Position Trade (1D sinyalleri — beklenti 20-30 gün)

| Gün   | Ç7     | Durum                                  |
|-------|--------|----------------------------------------|
| 1-12  | ×1.00  | Taze — tam güç                         |
| 15    | ×0.97  | Hafif yaşlanma                         |
| 20    | ×0.94  | Normal position süresi                 |
| 25    | ×0.90  | Threshold — gözden geçir               |
| 30    | ×0.80  | Uzuyor                                 |
| 35    | ×0.70  | Ciddi çürüme                           |
| 40+   | ×0.60  | Minimum — kapat önerisi                |

### Ç7 Ek Kuralları

**Kural 7a — Karda İse Çürüme Yavaşlar:**
```
P&L > +%5 → Ç7 çürüme hızı × 0.5 (yarıya düşür)
Mantık: Kazançlı pozisyonu erken kapatmak da hata.
Trend devam ediyorsa (Ç3 hâlâ güçlü) zaman baskısı azalır.
```

**Kural 7b — Zararda İse Çürüme Hızlanır:**
```
P&L < -%3 → Ç7 çürüme hızı × 1.5 (artır)
Mantık: Hem zaman hem para kaybediyorsun.
Zarardaki eski pozisyon = sermaye ve psikoloji tuzağı.
```

**Kural 7c — Mature Trend İstisnası:**
```
Pozisyon 12+ gün açık AMA:
  - 1D ADX > 25 ve yükseliyor
  - Fiyat EMA50 üzerinde (long için)
  - AlphaTrend hâlâ aynı yönde
→ "Mature Trend" protokolü: Ç7 minimum 0.85 (normal 0.70 olacakken)
Mantık: Trend hâlâ sağlıklıysa sadece yaşlandı diye kapatma.
```

**Kural 7d — Sinyal Tipi → Trade Tipi Eşleme:**
```
Squeeze, Range, Hype, Pullback → swing (5-10 gün beklenti)
Trend, AlphaTrend, NewTrend → position (20-30 gün beklenti)
Giriş sinyaline göre trade_type otomatik atanır.
```

### Ç7 Örnek Hesaplama

```
Pozisyon: XAU/USD LONG, giriş 12 gün önce
Sinyal tipi: Trend Buy → trade_type = 'position'
P&L: +%3.2 (karda)
1D ADX: 28, yükseliyor, AlphaTrend hâlâ bullish

Normal Ç7 (12 gün, position): ×1.00 (ilk 12 gün tam güç)
P&L pozitif ama <%5 → çürüme hızı normal
Mature trend koşulları sağlanıyor → minimum 0.85 garantisi

Ç7 = ×1.00

→ Position trade henüz genç, trend sağlıklı, çürüme yok.
  25. güne kadar rahat.
```

---

## Ç2 REVİZYON — VIX TERM STRUCTURE EKLEMESİ

### Mevcut Ç2'ye Eklenen Alt Parametre

VIX Term Structure = VIX Spot vs VIX Futures (Front Month) ilişkisi.

```python
def get_vix_term_structure(vix_spot, vix_futures_m1):
    """
    Contango: M1 > Spot → piyasa ileride daha fazla vol bekliyor (normal)
    Backwardation: Spot > M1 → piyasa şu anki paniğin geçici olduğunu düşünüyor
    """
    spread = vix_futures_m1 - vix_spot
    spread_pct = spread / vix_spot * 100
    
    if spread_pct > 5:
        return 'STEEP_CONTANGO'    # Piyasa çok tedirgin
    elif spread_pct > 0:
        return 'CONTANGO'          # Normal yapı
    elif spread_pct > -5:
        return 'FLAT'              # Geçiş bölgesi
    else:
        return 'BACKWARDATION'     # Kriz sonu sinyali
```

### Term Structure Düzeltmeleri (Ç2'ye ek çarpan)

#### CRISIS Rejiminde (VIX > 30)

| Term Structure    | VVIX Yönü | Ek Çarpan | Uygulanacak Yer         | Mantık                         |
|-------------------|-----------|-----------|-------------------------|--------------------------------|
| CONTANGO          | RISING    | ×0.8      | Tüm varlıklar          | Kriz derinleşiyor              |
| CONTANGO          | STABLE    | ×0.9      | Tüm varlıklar          | Kriz devam                     |
| CONTANGO          | FALLING   | ×1.0      | Tüm varlıklar          | Karışık sinyal                 |
| BACKWARDATION     | RISING    | ×1.0      | Tüm varlıklar          | Panik ama geçici olabilir      |
| BACKWARDATION     | FALLING   | ×1.2      | Kripto+Hisse çarpanına  | Kriz sonu — contrarian fırsat  |
| BACKWARDATION     | STABLE    | ×1.1      | Kripto+Hisse çarpanına  | Normalleşme başlıyor           |

#### Kritik Değişiklik — CRISIS Kripto Soft Allow:
```
Eski: VIX >30 → Kripto ×0.0 (hard block)
Yeni: VIX >30 + BACKWARDATION + VVIX FALLING → Kripto ×0.3 (soft allow)
Mantık: Kriz sonunu erken yakalamak. Ama max risk %0.5 (minimum),
küçük deneme pozisyonu. Hata payı kabul edilebilir.
```

#### ELEVATED Rejiminde (VIX 20-30)

| Term Structure    | Ek Çarpan | Etki                              |
|-------------------|-----------|-----------------------------------|
| STEEP_CONTANGO    | ×0.85     | Piyasa daha fazla düşüş bekliyor  |
| CONTANGO          | ×1.0      | Normal, değişiklik yok            |
| BACKWARDATION     | ×1.15     | Elevated ama geçici görünüyor     |

#### LOW/NORMAL Rejiminde
Term Structure etkisi minimal — ×1.0 (nötr). Düşük VIX'te term structure
zaten normal contango'da olur, bilgi değeri düşük.

### Veri Kaynağı
```
VIX Spot: yfinance → ^VIX
VIX Futures M1: yfinance → VX=F (CBOE VIX futures front month)
Alternatif: VXX ETN fiyatı yaklaşık proxy olarak kullanılabilir
```

---

## Ç3 REVİZYON — ADX TÜKENME SKALASI

### Eski ADX Bonusu
```
ADX > 25 → ×1.1
ADX > 40 → ×1.15
ADX < 15 → ×0.9
```

### Yeni ADX Skalası (Tükenme Dahil)

| ADX Aralığı | Ç3 Ek Çarpan | Yorum                                    |
|-------------|-------------|------------------------------------------|
| ADX < 15    | ×0.85       | Trend yok — range piyasa, dikkatli       |
| ADX 15-20   | ×0.95       | Zayıf trend — gelişmekte                 |
| ADX 20-25   | ×1.00       | Trend oluşuyor — nötr                    |
| ADX 25-30   | ×1.10       | Sağlıklı trend — optimal bölge ✓        |
| ADX 30-35   | ×1.10       | Güçlü trend — optimal bölge ✓           |
| ADX 35-40   | ×1.05       | Çok güçlü ama dikkat — olgunlaşıyor     |
| ADX 40-45   | ×1.00       | Nötr — güç ile tükenme dengede          |
| ADX 45-50   | ×0.90       | Tükenme olasılığı — kar realizasyonu düşün|
| ADX > 50    | ×0.80       | Yüksek tükenme riski — yeni giriş riskli|

### ADX Yön Bonusu (Ek)
```
ADX yükseliyor (ADX > ADX[5]) → mevcut çarpana ×1.05 bonus
ADX düşüyor (ADX < ADX[5]) → mevcut çarpana ×0.95 ceza
ADX'in seviyesi kadar yönü de önemli — düşen ADX 35'te bile
trend zayıflıyor demektir.
```

---

## Ç1 REVİZYON — DİNAMİK KORELASYON

### Aşamalı Uygulama

**Faz 1 (Çarşamba — başlangıç):** Statik matris kullanılır.
Mevcut 9 grup ve sabit korelasyon değerleri.

**Faz 2 (1 ay sonra — veri biriktikten sonra):**
60 günlük rolling korelasyon hesaplanır.

```python
def calc_dynamic_correlation(returns_a, returns_b, window=60):
    """
    Son 60 günlük günlük getirilerin Pearson korelasyonu.
    Her haftalık taramada güncellenir.
    """
    if len(returns_a) < window or len(returns_b) < window:
        return get_static_correlation(group_a, group_b)  # Fallback
    
    corr = np.corrcoef(returns_a[-window:], returns_b[-window:])[0][1]
    return round(corr, 2)
```

### Kriz Korelasyonu Alarmı

```python
def check_correlation_breakdown(current_corr_matrix, baseline_corr_matrix, threshold=0.40):
    """
    Krizde korelasyonlar yükselir — normalde düşük korele varlıklar
    (altın-kripto) aniden +0.70'e çıkabilir.
    
    Sapma >%40 ise → Correlation Breakdown Alarmı
    """
    deviations = []
    for pair in current_corr_matrix:
        baseline = baseline_corr_matrix[pair]
        current = current_corr_matrix[pair]
        deviation = abs(current - baseline) / max(abs(baseline), 0.01)
        if deviation > threshold:
            deviations.append({
                'pair': pair,
                'baseline': baseline,
                'current': current,
                'deviation_pct': round(deviation * 100, 1)
            })
    
    if len(deviations) > 0:
        # Tüm Ç1 değerlerini ×0.9'a çek (güvenlik)
        safety_override = 0.90
        return True, deviations, safety_override
    
    return False, [], 1.0
```

### Kriz Korelasyon Düzeltme Tablosu

| Normal Korelasyon | Kriz Korelasyonu | Fark  | Aksiyon                      |
|-------------------|------------------|-------|------------------------------|
| 0.10              | 0.55             | +0.45 | ⚠ ALARM — Ç1 override ×0.9 |
| 0.25              | 0.50             | +0.25 | Normal sapma, dikkat         |
| 0.60              | 0.80             | +0.20 | Beklenen, sorun yok          |
| 0.10              | 0.70             | +0.60 | ⛔ CİDDİ ALARM — tüm Ç1 ×0.8|

---

## BİRLEŞİK ÖZEL SENARYOLAR

### Senaryo A — Kriz Sonu Contrarian

```
Durum:
  VIX: 33 (CRISIS)
  VVIX: 5 günde -%18 (FALLING)
  VIX Term Structure: BACKWARDATION (spot > futures)
  DXY: Düşüyor (WEAK)
  Açık pozisyon: Yok (kriz öncesi hepsi kapatıldı)
  Drawdown: %8 (son krizden)
  Aday: BTC/USDT

Hesaplama:
  Ç4 (Kelly): Veri yok (son 50 işlem < 50) → BASE = 1.0
  Ç1: Açık pozisyon yok → ×1.0
  Ç2: CRISIS + FALLING + WEAK → Kripto ×0.0
       ANCAK BACKWARDATION + FALLING → soft allow ×0.3
       Term Structure ek: BACKWARDATION + FALLING → ×1.2
       Ç2 final: 0.3 × 1.2 = ×0.36
  Ç3: 1D Trend Buy + AlphaTrend Buy → ×1.4
       ADX 28, yükseliyor → ×1.10 × ×1.05 = ×1.155
       Ç3 final: 1.4 × 1.155 = ×1.617
  Ç5: Drawdown %8 → ×0.65
  Ç6: BTC Binance → ×1.0
  Ç7: Yeni giriş → ×1.0

  risk% = 1.0 × 1.0 × 0.36 × 1.617 × 0.65 × 1.0 × 1.0
        = %0.378 → clamp → %0.5 (minimum)
  risk$ = $500

Sonuç: Kriz ortamında küçük deneme pozisyonu.
Sistem izin veriyor ama minimum risk ile — akıllı contrarian.
Drawdown cezası da var, agresif olmanı engelliyor.
```

### Senaryo B — Sektör Yığılması + Düşük Likidite

```
Durum:
  Açık: GARAN (G2) %1.0, AKBNK (G2) %0.8
  Aday: YKBNK (G2) — 3. bankacılık hissesi
  VIX: 18 (NORMAL), VVIX stabil
  YKBNK günlük hacim: 800M TL

Hesaplama:
  Ç1: G2'de zaten 2 pozisyon → HARD BLOCK → Ç1 = ×0.0
  
  → risk% = ... × 0.0 × ... = %0.0 → ⛔ İŞLEM YAPILAMAZ

Ek uyarı: "Bankacılık sektörü toplam risk %1.8 (%25 portföy üst sınırına
yaklaşıyor). G2 grubunda 3. pozisyon açılamaz."

Sistem önerisi: "Düşük korele alternatif: XAU/USD (Ç1=×1.3),
BIMAS (Ç1=×1.1), BTC (Ç1=×1.1)"
```

### Senaryo C — Uzun Süreli Olgun Trend

```
Durum:
  Pozisyon: XAU/USD LONG, 18 gün açık
  Sinyal tipi: Trend Buy → position trade
  P&L: +%7.2 (karda)
  1D ADX: 31 (optimal bölge), yükseliyor
  AlphaTrend: Hâlâ bullish
  Fiyat: EMA50 üzerinde

Ç7 hesaplama:
  18 gün, position trade → threshold 25
  İlk 12 gün tam → gün 13-18 yavaş çürüme
  Normal: excess = 18 - 12.5 = 5.5 × 0.02 × 0.5 = 0.055
  Ç7 = 1.0 - 0.055 = ×0.945

  P&L > +%5 → çürüme hızı × 0.5 (Kural 7a)
  Düzeltilmiş: excess = 5.5 × 0.02 × 0.5 × 0.5 = 0.028
  Ç7 = 1.0 - 0.028 = ×0.972

  Mature trend koşulları? ADX 31 ✓, EMA50 üzeri ✓, AT bullish ✓
  → Minimum 0.85 garantisi (zaten 0.972 > 0.85, etkisiz)

  Ç7 final = ×0.972

Sonuç: 18 günlük pozisyon ama trend sağlıklı ve karda.
Çürüme neredeyse yok — doğru karar, pozisyonu koru.
```

### Senaryo D — Tüm Çarpanlar Aktif (Tam Hesaplama)

```
Durum:
  Portföy: $100,000, peak: $103,000, şu an: $98,500
  Açık: BTC (G1) %1.2, SODA (G6) %0.6
  Toplam açık risk: %1.8
  Kalan risk: %6 - %1.8 = %4.2
  Son 60 işlem: 34K/26L, avg win $980, avg loss $620
  Drawdown: ($103K - $98.5K)/$103K = %4.37
  Art arda kayıp: 1

  Aday: THYAO (G5)
  VIX: 22 (ELEVATED), VVIX: stabil, DXY: düşüyor
  VIX Term Structure: Contango (normal)
  1D: AlphaTrend Buy aktif, Trend sinyali yok
  ADX: 27, yükseliyor
  Sinyal yaşı: 3 bar (taze)
  THYAO hacim: 3B TL, spread: %0.06
  4H: Squeeze Buy tetiklendi → 🟢 AL

Hesaplama:
  Ç4 (Kelly):
    Win rate: 34/60 = %56.7
    Avg R/R: 980/620 = 1.58
    Full Kelly: (0.567 × 1.58 - 0.433) / 1.58 = %0.293
    Quarter: %7.33
    60 işlem (50-100 arası) → Kelly × 0.5 = %3.67
    BASE = max(0.5, min(2.0, 0.367)) = 0.50
    → Henüz 100 işlem olmadı, Kelly tam uygulanmıyor

  Ç1 (Korelasyon):
    BTC (G1) ve SODA (G6) açık
    THYAO = G5
    G1-G5: 0.15, G6-G5: 0.35
    Max korelasyon: 0.35 → Orta → ×0.9

  Ç2 (Makro):
    ELEVATED + STABLE + WEAK → BIST Hisse ×0.7
    XU100 UP + XULAS UP → ek ×1.1 (Kural 2c)
    Term Structure: Contango + ELEVATED → ×1.0
    Ç2 final: 0.7 × 1.1 × 1.0 = ×0.77

  Ç3 (1D Teknik):
    AlphaTrend Buy aktif, Trend yok → ×1.1
    ADX 27, yükseliyor → ×1.10 (optimal) × ×1.05 (yön) = ×1.155
    Sinyal yaşı: 3 bar → taze, tam değer
    Ç3 final: 1.1 × 1.155 = ×1.27

  Ç5 (Drawdown):
    DD: %4.37 → %2-5 bandı → ×0.85
    Streak: 1 → ×1.0
    Ç5 = min(0.85, 1.0) = ×0.85

  Ç6 (Likidite):
    Hacim: 3B TL ≈ $86M → ×1.0
    Spread: %0.06 → ×0.95
    PV ratio: minimal → ×1.0
    Ç6 = min(1.0, 0.95, 1.0) = ×0.95

  Ç7 (Zaman):
    Yeni giriş → ×1.0

  SONUÇ:
  risk% = 0.50 × 0.9 × 0.77 × 1.27 × 0.85 × 0.95 × 1.0
        = 0.50 × 0.9 × 0.77 × 1.27 × 0.85 × 0.95
        = 0.50 × 0.709
        = %0.355 → clamp → %0.50 (minimum)
  
  Kalan risk: %4.2 → %0.5 sığar ✓
  risk$ = $500
  4H: 🟢 AL

  Sonuç: THYAO LONG, $500 risk.
  Neden minimum? Kelly henüz düşük (veri yetersiz), ELEVATED VIX
  çarpanı düşürüyor, hafif drawdown freni var. Sistem muhafazakar
  davranıyor — doğru karar.

  6 ay sonra 100+ işlem, Kelly BASE 0.75, drawdown %0:
  risk% = 0.75 × 0.9 × 0.77 × 1.27 × 1.0 × 0.95 × 1.0
        = %0.63 → $630 risk
  → Aynı senaryo, daha yüksek risk — performans kanıtlandıkça
    sistem sana daha fazla güveniyor.
```

---

## UYGULAMA ÖNCELİK SIRASI

| Öncelik | Çarpan | Neden                                          | Çarşamba? |
|---------|--------|-------------------------------------------------|-----------|
| 1       | Ç5     | Psikolojik koruma — en kritik savunma hattı    | ✅ Evet   |
| 2       | Ç6     | BIST likiditesi — yanlış boyutlanma önlenir    | ✅ Evet   |
| 3       | Ç2+    | VIX Term Structure — kriz tradingi fırsatı     | ✅ Evet   |
| 4       | Ç3+    | ADX tükenme — agresif girişi frenleme          | ✅ Evet   |
| 5       | Ç7     | Zaman çürümesi — pozisyon trade'ler için       | ✅ Evet   |
| 6       | Ç4     | Kelly — veri biriktikten sonra devreye girer   | ✅ Formül |
| 7       | Ç1+    | Dinamik korelasyon — 60 gün veri lazım         | ⏳ Faz 2  |

Çarşamba'da Ç1-Ç7 hepsi kodlanacak. Ç4 (Kelly) formül hazır olacak
ama BASE = 1.0 ile başlayacak (veri yetersiz). Ç1 statik matris ile
başlayacak, 60 gün sonra dinamiğe geçecek.

---

## DASHBOARD GÖRÜNÜM (GÜNCELLENMİŞ)

```
KALAN RİSK İŞTAHI: %4.2 ($4,200)     DD: %4.4 (⚠)     Streak: 1
Kelly BASE: 0.50 (veri: 60/100)        Moratoryum: YOK
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Enstr.  │ Fiyat │ ATR% │ Ç1  │ Ç2  │ Ç3  │ Ç5  │ Ç6  │ Ç7  │Risk%│ 4H
────────┼───────┼──────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┼────
THYAO   │₺312  │ 2.1% │ 0.9 │ 0.77│ 1.27│ 0.85│ 0.95│ 1.0 │ 0.5%│ 🟢
XAU     │$2680 │ 1.4% │ 1.1 │ 1.40│ 1.40│ 0.85│ 1.00│ 1.0 │ 1.3%│ ⚪
EREGL   │₺43   │ 2.8% │ 0.9 │ 0.70│ 0.80│ 0.85│ 0.90│ 1.0 │ 0.5%│ ⚪
ETH     │$3150 │ 3.5% │ 0.5 │ 0.50│ 1.10│ 0.85│ 1.00│ 1.0 │ 0.5%│ 🟢
YKBNK   │₺35   │ 2.4% │ 0.0 │  —  │  —  │  —  │  —  │  —  │ ⛔  │ ⛔

AÇIK POZİSYONLAR:
BTC   │ LONG │ 8 gün │ +$1,200 (+%4.1) │ Ç7: 0.98 │ 🟢 SAĞLIKLI
SODA  │ LONG │ 3 gün │ -$180 (-%1.2)   │ Ç7: 1.00 │ 🟡 İZLE
```
