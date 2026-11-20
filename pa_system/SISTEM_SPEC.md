# PA Tabanlı Çoklu Zaman Dilimi İşlem Sistemi — Spesifikasyon v1.0

> Durum: **Üretim öncesi taslak.** `⚠ AÇIK` ile işaretli maddeler `OPEN_DECISIONS.md` içinde karar bekliyor; o zamana kadar `config/params.yaml` içindeki varsayılan geçerlidir.
> Tüm sayısal değerler `config/params.yaml` içindedir; bu dokümandaki sayılar başlangıç değerleridir ve kod içine sabit yazılmaz.

---

## 0. Özet (tek paragraf)

Yön ve risk **fiyat yapısından (PA)** gelir; göstergeler yalnız **filtre ve teyittir**. Üst zaman dilimi (HTF) rejimi ve yönü, PD (premium/discount) konumu belirler; alt zaman dilimi (LTF) tetik verir (trend çizgisi kırılımı / CHoCH / seviye kırılımı). Stop pivot − k×ATR, çıkış pivot trailing + Chandelier yedeği + yapısal/rejim/zaman çıkışlarıdır. Uygun koşul yoksa karar **İŞLEM YOK**'tur. Tarayıcı, BIST30 vb. evrende hangi enstrümanın hangi profilde daha uygun olduğunu **sıralar**; giriş yine PA tetiğiyle olur. Ekran görüntüsü yalnız **insan bilgisini aktarma ve kontrol** için kullanılır; sinyal, backtest, parametre ve fiyat veriden gelir.

### Temel ilkeler
1. Önce rejim → sonra yön → sonra konum (PD) → en son tetik → risk kontrolü.
2. HTF ile LTF çelişirse işlem yok.
3. Her boyut **tek ölçü** ile temsil edilir (aynı bilgi iki kez puanlanmaz).
4. Eşikler mümkünse **yüzdelik** (enstrümanın kendi geçmişine göre); kurallar enstrümanlar arası ortak, yalnız ATR çarpanları enstrümana özel.
5. **Lookahead yok:** HTF değerleri yalnız kapanmış mumdan; pivot yalnız **onaylandığı mumdan** itibaren kullanılır.
6. Kural seti **sürümlüdür**; hiçbir kod otomatik değişmez; her değişiklik test → onay → yeni sürüm.

### Orijinal taslaktan (önceki model) alınanlar / atılanlar
- Alındı: HTF yön / LTF zamanlama ayrımı, "işlem yok bir karardır", 4 rejim, kapanmış mum kuralı, walk-forward şartı.
- Atıldı: MACD Histogram, RSI MA5, Stochastic RSI, puanlamada ADX+ER birlikte, sayısal olmayan eşikler, boş karar matrisi, çıkışsız yapı.

---

## 1. Profiller ve enstrümanlar

| Profil | HTF | LTF | Trail TF (varsayılan ⚠ AÇIK) | Kapsam |
|---|---|---|---|---|
| P1 Aktif | 4H | 15m | 1H | BTC, ETH, XAU, XAG |
| P2 Swing | 1D | 1H | 4H (BIST: 1D ara pivot ⚠) | Hepsi |
| P3 Uzun swing | 1W | 4H (BIST: 1D) | 1D | Hepsi |

- Kripto günlük kapanış: UTC 00:00. BIST'te 4H mum kullanılmaz (seans 10:00–18:00).
- Enstrümana özel notlar §10'da.

---

## 2. Göstergeler (nihai set)

| Gösterge | Parametre | Görev | Not |
|---|---|---|---|
| **Pivot motoru** | §3 | Yapı, PD, stop, trail, trend çizgisi | Sistemin çekirdeği |
| ATR | 14 | Tampon, stop, salınım boyutu, normalizasyon | |
| ATR/Fiyat yüzdeliği | 100 bar | Volatilite rejimi | ≥80 yüksek, ≤20 düşük |
| EMA 50 / 200 | 50/200 | Makro filtre, aşırı uzama, eğim | Eğim = (EMA50 − EMA50[10]) / ATR |
| Efficiency Ratio | 20 | Trend kalitesi | Histerezis: giriş ≥0.30, çıkış <0.20, yatay <0.15 |
| ADX | 14 | Opsiyonel teyit | Varsayılan **kapalı**, puanlanmaz |
| RSI | 14 | HTF momentum bandı + LTF uyumsuzluk | Uptrend sağlıklı bant ≥40, downtrend ≤60 |
| Bollinger | 20 / 2σ | Sıkışma | |
| Keltner | EMA20 / 1.5×ATR | Sıkışma | Squeeze = BB tamamen KC içinde |
| Relative Volume | hacim / SMA20(hacim) | Kırılım katılımı | BIST intraday: aynı saat dilimi ortalaması. Gerçek hacmi olmayan enstrümanda **kapalı** |
| Çıkarılanlar | — | MACD, RSI MA5, Stoch RSI | Bilgi tekrarı / gürültü |

---

## 3. Pivot motoru (hibrit)

1. **Aday:** fraktal — sol L / sağ R bar (HTF 5/5, LTF 3/3).
2. **Boyut filtresi:** son zıt pivottan uzaklık ≥ k×ATR(14). `ana` k=1.5, `ara` k=0.75.
3. **Alternasyon:** art arda iki tepe → yüksek olan kalır; iki dip → düşük olan kalır.
4. **İki seviye:** `ana` (yön, PD, CHoCH/BOS) · `ara` (stop, trail, LTF trend çizgisi, uyumsuzluk).
5. **Zaman damgası:** her pivotun `bar_index` (oluştuğu mum) ve `confirmed_index` (onaylandığı mum) alanı vardır. **Tüm kurallar yalnız `confirmed_index ≤ şimdiki mum` olan pivotları görür.**
6. Pivot fiyatı **fitil** (high/low); kırılımlar **kapanış** ile.
7. **Kalibrasyon:** kullanıcı etiket seti (enstrüman başına 50–100 pivot) ile eşleşme (±2 bar, ±0.25×ATR) maksimize edilir; backtest getirisine göre ayar yapılmaz. Parametreler 3–6 ayda bir gözden geçirilir.

---

## 4. Yapı, BOS/CHoCH, PD

### 4.1 Yapı (ana pivotlar, HTF)
- **UP:** son iki tepe HH **ve** son iki dip HL.
- **DOWN:** LH **ve** LL.
- **RANGE:** diğer durumlar.
- **BOS:** trend yönünde son ana pivotun kapanışla aşılması (devam).
- **CHoCH:** UP'ta son HL'nin kapanışla kırılması (DOWN'da son LH'nin) → yapı `TRANSITION`; yeni yönde HH+HL (veya LH+LL) oluşunca yeni trend onaylanır.

### 4.2 PD (premium/discount)
- **Aralık:** HTF'de son BOS'u üreten dürtü bacağı (bacağın başlangıç ana pivotu → BOS sonrası en uç nokta). Yeni HH/LL ile güncellenir.
- `pd = (close − range_low) / (range_high − range_low)` → 0 = dip, 1 = tepe.
- **Discount:** pd < 0.50 · **Premium:** pd > 0.50 · **OTE long:** pd 0.21–0.38 (= %62–79 geri çekilme) · OTE short: pd 0.62–0.79.
- Long yalnız discount, short yalnız premium. Sığ geri çekilme (long pd 0.50–0.62) yarım risk ile ⚠ AÇIK (varsayılan: kapalı).

---

## 5. Rejim sınıflandırması (HTF)

Öncelik sırası **D → C → B → A**; ilk sağlanan rejim geçerlidir. Yeni rejim **2 HTF mum** sürmeden kabul edilmez.

| Rejim | Koşul | Karar |
|---|---|---|
| **D** Yüksek vol / yönsüz | ATR/Fiyat yüzdeliği ≥80 **ve** ER <0.20 | İşlem yok |
| **C** Sıkışma | Squeeze ≥6 bar **ve** ATR/Fiyat yüzdeliği ≤40 | İzle; yalnız kırılım (S3) |
| **B** Yatay | Yapı RANGE **ve** ER <0.15 | Trend işlemi yok; yalnız aralık kırılımı (S3). Aralık içi işlem ⚠ AÇIK (varsayılan kapalı) |
| **A** Trend | Yapı UP/DOWN **ve** ER ≥0.30 (histerezis 0.20) | S1 |
| **A−** Erken trend | Yapı UP ama fiyat EMA200 altında (veya tersi) | S1/S2 yarım risk |
| Belirsiz | Hiçbiri | Bekle |

Ek bayrak — **Aşırı uzama:** |close − EMA50| > 2.5×ATR (HTF) → yeni giriş yok.

---

## 6. Seviyeler, trend çizgisi, kırılım, uyumsuzluk

### 6.1 Yatay seviye
- ≥2 pivot, birbirine ≤0.25×ATR → küme; seviye = küme ortalaması.
- Kalite: temas sayısı, yaş, HTF ile çakışma.

### 6.2 Trend çizgisi
- Alçalan çizgi: ≥2 onaylı alçalan tepe (fitil); 3. temas (±0.25×ATR) = geçerli çizgi. Pivotlar arası ≥5 bar.
- Eğim filtresi: |eğim| ≤ 0.5×ATR / bar (çok dik çizgi geçersiz).
- HTF için `ana`, LTF geri çekilme kanalı için `ara` pivotlar.

### 6.3 Kırılım olayı (yatay, çizgi, squeeze kutusu)
| Aşama | Kural |
|---|---|
| Kırılım | close > seviye + 0.2×ATR · RVOL ≥1.3 (squeeze kırılımı ≥1.5) · gövde/menzil ≥0.5 · kapanış mumun üst %30'unda |
| Teyit | Sonraki 3 mumda kapanış seviye içine dönmez **veya** retest tutar |
| Başarısız | Kapanış seviye içine döner → sahte kırılım (pozisyondaysa çık) |
| Etiket (backtest) | İçeri dönmeden +1R'ye ulaştı mı → başarılı/başarısız |

Çizgi kırılımı tek başına dönüş değildir: **çizgi kırılımı = ARMED**, son LH seviyesinin kırılması (CHoCH) = **onay**. Agresif mod (çizgi kırılımında yarım risk) ⚠ AÇIK.

### 6.4 Uyumsuzluk (RSI 14, LTF `ara` pivotlar)
- Pivotlar arası 5–60 bar; yalnız onaylı pivotlar.
- **Gizli (devam):** fiyat HL, RSI LL → trend yönünde teyit.
- **Klasik (dönüş):** fiyat LL, RSI HL; ilk pivotta RSI <40 (short için >60).
- Kullanım: gizli + HTF yönünde klasik → kurulum teyidi (ARMED kalitesini artırır). HTF'ye karşı klasik → yalnız **stop sıkılaştırma / kısmi kâr**, ters işlem yok.
- Uyumsuzluk tek başına tetik değildir.

---

## 7. Kurulumlar

| Kod | Kurulum | Koşullar | Risk |
|---|---|---|---|
| **S1** | Trend devamı (A+) | HTF A (UP) · pd <0.50 (tercihen OTE) · LTF: geri çekilme kanalı çizgi kırılımı **veya** LTF CHoCH up · (+ gizli/klasik uyumsuzluk = kalite artışı) | Tam |
| **S2** | Dönüş | HTF çizgi kırılımı + HTF CHoCH → yeni bacağın discount'una geri çekilme → LTF tetik | Yarım (HTF'de HH+HL oluşana kadar) |
| **S3** | Aralık / sıkışma kırılımı | Yatay sınır veya squeeze kutusu kırılımı (§6.3) · HTF yapı ters değil | Tam; stop aralık ortası veya son LTF `ara` dip |

Short için koşullar ters çevrilir (BIST'te short yalnız VİOP ile ⚠ AÇIK).

### Kalite notu
- **A+:** S1 + OTE + uyumsuzluk teyidi + RVOL desteği → risk çarpanı 1.0
- **A:** kurulum koşulları tam → 0.75
- **B:** S2 veya A− rejim veya sığ geri çekilme → 0.5

---

## 8. Durum makinesi ve giriş

```
IDLE ──(HTF uygun + kurulum oluşuyor)──► ARMED ──(tetik + tüm giriş kontrolleri)──► IN_POSITION
  ▲                                         │ TTL dolu / geçersizlik                     │
  │                                         ▼                                            ▼
  └──────────── COOLDOWN (stop sonrası) ◄── EXITED ◄──────────────────────────── çıkış kuralları
```

- **ARMED TTL:** 20 LTF mum; dolarsa iptal.
- **COOLDOWN:** stop sonrası aynı enstrümanda 10 LTF mum.
- **Olay filtresi:** FOMC / CPI / NFP / TCMB ±2 saat (kripto, metal); BIST bilanço / temettü / bedelsiz ±3 gün → ARMED→TRIGGERED engellenir. Mevcut `risk_multipliers.check_economic_calendar` genişletilerek kullanılır.

### Giriş kontrolleri (hepsi zorunlu)
1. Rejim ∈ {A, A−, C(S3), B(S3)} ve D değil.
2. HTF yön ile LTF tetik yönü aynı.
3. PD şartı (§4.2).
4. Aşırı uzama yok.
5. Stop mesafesi ≤ 2×ATR (giriş TF).
6. TP1 (karşı likidite = önceki HTF ana pivot) ≥ 2R; değilse işlem yok.
7. Korelasyon / toplam risk limiti uygun (§11).
8. Emir: kapanış teyidi sonrası sonraki mum açılışı (varsayılan) veya stop-limit.

### Karar matrisi

| HTF | LTF | Karar |
|---|---|---|
| UP trend | Pozitif + teyit | LONG |
| DOWN trend | Negatif + teyit | SHORT (BIST: bekle) |
| UP trend | Negatif momentum | ARMED — geri çekilme izle |
| DOWN trend | Pozitif momentum | ARMED — işlem yok |
| Yatay | Sıkışma | İzle; HTF aralık kırılımında S3 |
| Yatay | Yönsüz | İşlem yok |
| Sıkışma | Sıkışma | İzle |
| Sıkışma | Teyitli kırılım | HTF EMA yapısıyla uyumluysa yarım risk |
| D rejimi | Herhangi | İşlem yok |
| Güçlü trend | Teyitsiz ani hareket | Kovalama yok; retest bekle |

---

## 9. Çıkış yönetimi

| # | Kural | Varsayılan |
|---|---|---|
| 1 | İlk stop | Giriş TF son `ara` pivot − k×ATR (k: §10) |
| 2 | Kısmi kâr | 1.5R'de %40 kapat, stop başabaşa (1R'de BE test edilecek) |
| 3 | Trailing | Trail TF'de her yeni onaylı HL − k×ATR; **yedek** Chandelier (en yüksek − 3×ATR); long'da ikisinin **yükseği**; stop asla gevşemez |
| 4 | Yapısal çıkış | Trail TF'de ters CHoCH → tamamı kapat |
| 5 | Rejim çıkışı | HTF kapanış EMA50 karşı tarafında **veya** rejim → D → kapat |
| 6 | Zaman stopu | 30 LTF mum içinde +1R yoksa çık |
| 7 | Başarısız kırılım | Kapanış seviye içine dönerse çık (S3) |
| 8 | HTF karşı uyumsuzluk | Yalnız stop sıkılaştır (LTF `ara` pivota) |
| 9 | BIST gap | Pozisyon boyutunda gap payı (ortalama gece gap'i × 1.5) |

---

## 10. Enstrüman ayarları

| Enstrüman | Stop k | RVOL | Özel kurallar |
|---|---|---|---|
| BTC | 0.75 | Açık (spot/perp hacmi ⚠) | Hafta sonu kırılımlarında RVOL eşiği +0.3; perp ise funding maliyeti beklentiye eklenir |
| ETH | 0.75 | Açık | BTC ile tek korelasyon grubu; ETH/BTC trendi yön filtresi; boyut volatiliteye göre küçülür |
| XAU | 0.5 | Vadeli (GC) hacmi yoksa **kapalı** ⚠ | Makro olay filtresi; DXY / reel faiz bağlamı (mevcut Ç2) |
| XAG | 1.0 | Altınla aynı kaynak | Kırılımda XAU yön teyidi; XAU ile tek risk grubu |
| BIST | 0.5 + gap payı | Açık (seans normalize) | Long-only (VİOP yoksa) ⚠; likidite filtresi; XU100 rejimi = M çarpanı; USD bazlı trend kontrolü; temettü/bölünme düzeltmeli veri; tavan/taban mumları kırılım sayılmaz |

---

## 11. Risk ve pozisyon boyutu

- Mevcut çerçeve korunur: `risk% = clamp(BASE × Ç1 × Ç2 × Ç3 × Ç5 × Ç6 × Ç7, 0.5, 2.0)`, aylık max %6 (`carpan_tablosu_v2.md`, `risk_multipliers.py`).
- **Değişiklik:** Ç3 (1D teknik çarpan; eski sinyal tipleri + ADX) yerine **kalite notu çarpanı** (A+ 1.0 / A 0.75 / B 0.5). Aynı bilgi tarama skoru ve Ç3'te iki kez sayılmaz. ⚠ AÇIK: Ç3 tamamen mi değişecek, yoksa yalnız ADX kısmı mı çıkacak.
- Korelasyon grupları: {BTC, ETH}, {XAU, XAG}; BIST sektör başına max 2–3 pozisyon. Grup toplam riski tek pozisyon riskinin 1.5 katını aşmaz.
- Pozisyon boyutu = (sermaye × risk%) / (giriş − stop); BIST'te stop mesafesine gap payı eklenir.

---

## 12. Tarayıcı / watchlist skoru

**Skor = G × Σ(wᵢ·Sᵢ) × M** (+ uyum bonusu) — sıralama aracıdır, giriş sinyali değildir.

### G — eleme (0/1)
Likidite (20g ort. TL hacim > eşik) · HTF yapı DOWN değil · rejim ≠ D · aşırı uzama yok · olay penceresi dışında.

### S — bileşenler (0–1)
| Bileşen | Ölçü | w |
|---|---|---|
| Yapı | UP=1 · aralık kırılımı=0.7 · taze CHoCH=0.5 | 25 |
| PD | OTE (pd 0.21–0.38)=1 → pd 0.50'de 0 (lineer) | 20 |
| Göreceli güç | Fiyat/XU100 oran eğimi, evren içi yüzdelik | 15 |
| Trend kalitesi | ER, enstrümanın kendi geçmişine göre yüzdelik | 15 |
| Kuruluma yakınlık | Tetiğe ≤1×ATR · squeeze aktif · uyumsuzluk | 15 |
| R/R | Hedef / stop: 1.5→0, ≥3→1 | 10 |

### M — piyasa çarpanı
XU100 rejimi UP=1.0 · RANGE=0.8 · DOWN=0.5 (kripto/metal için mevcut Ç2 makro çarpanı).

### Çıktı
- Her **enstrüman × profil** için skor; enstrümanın en iyi profili = max skor; iki profil aynı yönde ve ≥60 → +5..10 uyum bonusu.
- ≥75 **HAZIR**, 60–75 **İZLE**, <60 dışarıda; histerezis: 75'te gir, 65 altında çık.
- Sektör başına max 2–3.
- **Doğrulama:** günlük skor dilimleri (desil) → sonraki N gün ortalama R monoton artmalı; artmıyorsa skor geçersiz. Ağırlık optimizasyonu bu testten sonra.

Normalizasyon ayrımı: rejim/volatilite = **zaman serisi** yüzdeliği; sıralama/RS = **kesit (evren içi)** yüzdeliği.

---

## 13. Çıktı formatı

Aksiyon pozisyon durumuna göre ayrılır:
- Pozisyon yok: `NO_TRADE` · `WATCH` (ARMED) · `OPEN_LONG` · `OPEN_SHORT`
- Pozisyon var: `HOLD` · `MOVE_STOP` · `PARTIAL_CLOSE` · `CLOSE`

Sinyal kartı alanları: enstrüman, profil, rejim, HTF yapı, pd, durum→aksiyon, kurulum+kalite, tetik koşulu, giriş+emir tipi, stop, TP1/TP2, R/R, risk%, geçersizlik + TTL, **kurulum geçmiş istatistiği** (win%, ort R, beklenti, n), uyarılar (olay, korelasyon), gerekçe (≤3 madde), `ruleset_version`.
"AL %70" tipi güven yüzdesi **kullanılmaz** (kalibre değil). Şema: `schemas/outputs.schema.json`.

---

## 14. Ekran görüntüsü mimarisi

### 14.1 İlke
Görüntü = **insan yorumunu sisteme aktarma ve kontrol**. Sayı ve karar veriden gelir.
**Yasak:** sinyal üretimi, emir, backtest, parametre optimizasyonu, stop/hedef fiyatı, kodun otomatik değiştirilmesi.

### 14.2 Bileşenler

```
[Veri (OHLC)] → [Render: Python, bar index'li şablon] → [Görüntü deposu + metadata]
                                                              │
     [TV Pine görsel doğrulama] ─(yalnız F8 / son kontrol)─────┤
                                                              ▼
                                   [Claude vision → yapılandırılmış JSON]
                                                              ▼
                                         [İnsan inceleme / onay]
                                                              ▼
       [Geri besleme: etiket seti · veto log · journal sınıfları · kural önerisi]
                                                              ▼
                       [Değişiklik kontrolü: öneri → test (etiket + OOS) → sürüm]
```

- **Render:** tercihen Python (mplfinance) ile veriden; her mumda indeks, pivot/çizgi/PD/stop overlay. TradingView ekran görüntüsü yalnız Pine görsel doğrulaması için (kullanım şartları ve kırılganlık nedeniyle).
- **Metadata (her görüntü):** sembol, TF, zaman aralığı, `ruleset_version`, `params_hash`, amaç (U1–U8), çekim zamanı.
- **Fiyat oturtma:** Claude çıktısı fiyat değil **bar index** verir; fiyat veriden alınır.

### 14.3 Kullanımlar

| # | Kullanım | Akış | Çıktı | Sıklık |
|---|---|---|---|---|
| U1 | Yorumu kurala çevirme | Kullanıcı çizer → Claude işaretli/işaretsiz farkını sorar | Kural taslağı + karşı örnek listesi | İhtiyaç halinde |
| U2 | Ön etiketleme | Render → Claude bar index önerir → kullanıcı düzeltir | `labels/*.json` (pivot) | Kalibrasyon dönemi |
| U3 | Repaint testi | Mum kapanışında + N mum sonra görüntü + değer logu karşılaştırma | Fark listesi (pivot, çizgi, sinyal) | Her sürümde + haftalık örnek |
| U4 | Çizim/gösterge kontrolü | Kural tanımı + render → tutarlılık | Hata listesi | Her sürümde |
| U5 | İşlem günlüğü | Giriş/çıkışta otomatik görüntü → haftalık sınıflama | a: normal kayıp · b: kural ihlali · c: kural boşluğu · d: veri/teknik | Her işlem / haftalık |
| U6 | Tarama özeti | İlk 5–10 enstrüman şablon grafik + 1–2 cümle yorum | Karar destek sayfası; **veto + neden log** | Her tarama |
| U7 | Veri yokken kaba okuma | Telefon görüntüsü → yapı, ana pivot, PD, senaryo | Yalnız NO_TRADE / WATCH; emir yok | İhtiyaç halinde |
| U8 | Çizgi + sahte kırılım incelemesi | Haftalık çizgi ve başarısız kırılım galerisi | Neden sınıfı (haber, hafta sonu, stop avı, HTF direnç) → kural önerisi | Haftalık |

Gruplar: U1–U2 bilgi aktarımı · U3–U4 kod doğruluğu · U5, U8 performans denetimi · U6–U7 karar anı insan kontrolü.

### 14.4 Değişiklik kontrolü
- Kural önerisi yalnız **tekrarlayan** desenden (U5 sınıf c, U8) ve yeterli örneklemle.
- Öneri → etiket seti + örneklem dışı test → mevcut sürümden iyiyse yeni sürüm (semver).
- Otomatik kod değişikliği **yoktur**.
- U6 vetolarının sonradan performansı ölçülür; değer katıyorsa kurala dönüştürülür.

---

## 15. Doğrulama ve test

1. **Lookahead testi (zorunlu birim testi):** seri `t` anında kesilip hesaplanan tüm değerler (pivot, yapı, rejim, sinyal), tam seri üzerinde hesaplanıp `t`'ye kadar alınan değerlerle **birebir aynı** olmalı.
2. Maliyet: komisyon + kayma + (perp) funding + BIST gap.
3. Enstrüman × profil ayrı rapor; rejim (A/B/C/D) ve kurulum (S1/S2/S3) kırılımı.
4. Metrikler: işlem sayısı, win%, ort kazanç/kayıp (R), beklenti (R), profit factor, max DD, ort işlem süresi.
5. Walk-forward + örneklem dışı; parametreler OOS doğrulanmadan sabitlenmez.
6. Minimum örneklem: kurulum başına ≥30 işlem raporlama, ≥100 işlem karar.
7. Tarama skoru desil monotonluk testi.
8. Canlıya geçiş öncesi paper trading (süre ⚠ AÇIK, öneri ≥8 hafta).
9. Python ↔ Pine parite testi (aynı veri, aynı sinyal zamanları).

---

## 16. Sürümleme
- `ruleset_version` (semver) `config/params.yaml` içinde; her sinyal, görüntü ve journal kaydı bu sürümü ve `params_hash`'i taşır.
- MAJOR: kural mantığı değişti · MINOR: yeni kurulum/filtre · PATCH: parametre veya hata düzeltmesi.
