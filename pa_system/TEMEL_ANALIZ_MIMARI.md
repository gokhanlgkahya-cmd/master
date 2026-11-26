# Temel Analiz Modülü — Mimari Taslak v0.1 (BIST)

> Durum: **taslak**, kod yazılmadı (yalnız `research/sasa/` denemesi var). Açık kararlar `OPEN_DECISIONS.md` #30–#33.
> Amaç: F/K vb. temel metrikleri **zaman serisi** olarak işleyip yön/güç/tutarlılık hesaplamak; watchlist'e **temel skor sütunu** ve (doğrulama sonrası) tarama skoruna hafif bir **çarpan** üretmek.

## 1. Akış
```
[KAP / finansal tablo kaynağı] → [Yayın tarihli depo (point-in-time)] → [Normalizasyon: TTM, TMS29, USD/reel, sektör]
 → [Metrik serileri] → [Seviye + Yön + Tutarlılık] → [Sütun skorları → Temel skor + çarpan] → [Watchlist sütunu + grafikler]
```

## 2. Zorunlu veri kuralları
| Kural | Açıklama |
|---|---|
| Yayın tarihi | Her kayıt **KAP yayın zamanı** ile; veri o andan sonra görünür (lookahead yok) |
| TMS 29 kırılması | 2022/12 ve 2023/12 yıl sonu tabloları düzeltilmiş, ara dönemler nominal (SASA denemesinde kanıtlandı). Bu çeyreklere değen TTM pencereleri **geçersiz**. Otomatik tespit: YTD satış TRY/USD ima edilen kurda sıçrama |
| Dönemler arası seviye | TMS 29 öncesi ve sonrası **karıştırılmaz**; seviye yüzdeliği tutarlı dönem içinde |
| Büyüme birimi | USD bazlı ve/veya reel TL (TÜFE). İş Yatırım USD: gelir tablosu ≈ ortalama kur, bilanço = dönem sonu kur |
| İlk yayın | Sonradan düzeltilen tablolar için ilk yayımlanan hâli saklanır |
| Sulanma | Hisse sayısı serisi tutulur; **hisse başı** metrikler (FAVÖK/hisse) ayrıca hesaplanır |
| Net kâr | TMS 29 + ertelenmiş vergi nedeniyle oynak → ağırlığı düşük; FAVÖK ve nakit akımı öncelikli |

## 3. Sektör şablonları
| Şablon | Örnek | Metrikler |
|---|---|---|
| Sanayi / hizmet | THYAO, TUPRS, ASELS, SASA | K/F (F/K yerine), FD/FAVÖK, FAVÖK marjı, ROE, net borç/FAVÖK, net döviz poz./özkaynak, faaliyet nakdi/FAVÖK, serbest nakit/satış |
| Banka | AKBNK, GARAN, YKBNK | PD/DD, ROE, net faiz marjı, takipteki kredi oranı, sermaye yeterliliği, reel kredi büyümesi |
| Holding | KCHOL, SAHOL | NAV iskontosu, iştirak değeri yönü |
| GYO | EKGYO | NAV iskontosu, satış/ön satış hacmi |

## 4. Metrik başına ölçüler ("temelin teknik analizi")
| Ölçü | Tanım | Teknik karşılık |
|---|---|---|
| Seviye | %60 sektör içi yüzdelik + %40 kendi geçmiş yüzdeliği (tutarlı dönem) | PD konumu |
| Yön | Son 4 TTM noktasının normalize regresyon eğimi + ivme | EMA eğimi |
| Tutarlılık | Son 8 çeyrekte iyileşen çeyrek oranı | ER |

- Değerleme yönü ayrıştırılır: Δ(F/K) = Δfiyat − Δkazanç (kazanç artışıyla düşen F/K pozitif).
- **Temel uyumsuzluk:** fiyat ↑ & TTM FAVÖK/marj ↓ → negatif bayrak; fiyat ↓ & FAVÖK/marj ↑ → pozitif bayrak (net zarar, negatif nakit veya sulanma varsa "teyitsiz").

## 5. Skor ve çarpan (sanayi şablonu, başlangıç)
| Sütun | Metrikler | Ağırlık |
|---|---|---|
| Değerleme | K/F, FD/FAVÖK | 20 |
| Kârlılık | ROE, FAVÖK marjı | 20 |
| Büyüme | USD/reel satış ve FAVÖK büyümesi | 25 |
| Bilanço | Net borç/FAVÖK, net döviz poz./özkaynak | 20 |
| Kalite | Faaliyet nakdi/FAVÖK, serbest nakit/satış | 15 |

- Sütun = **%60 (yön + tutarlılık) + %40 seviye**.
- Temel skor 0–100 · Yön ↑/→/↓ + güç · **Çarpan F = 1 + 0.15 × (skor − 50)/50** (0.85–1.15).
- Profil ağırlığı: P3 tam, P2 yarım.
- **Kırmızı bayrak (eleme):** negatif özkaynak · 4 çeyrek üst üste zarar + yüksek borç · olumsuz/şartlı denetim görüşü · **serbest nakit < 0 ve net borç/FAVÖK > 6** → en fazla İZLE.
- Watchlist sütunları: Temel skor · Yön oku + güç · Çarpan · Son bilançodan gün · Bayrak.

## 6. Grafikler
1. Metrik serisi (TTM) + trend + sektör bandı · 2. F/K bant grafiği · 3. Fiyat ve TTM FAVÖK üst üste (uyumsuzluk) · 4. Sütun radarı (çeyrek karşılaştırma).

## 7. Güncelleme ve doğrulama
- Fiyat bazlı metrikler günlük; tablo bazlı metrikler yeni KAP bildiriminde.
- Desil testi 1/3/6 ay ileri getiri; **etkileşim testi**: S1 başarısı yüksek temel skorlu hisselerde daha mı iyi?
- Testler geçilene kadar çarpan **yalnız bilgi sütunu**; tarama skoruna girmez.

## 8. SASA denemesi (Kasım 2026) — özet
Kaynak: İş Yatırım (MaliTablo + HisseTekil). Kod/çıktı: `research/sasa/`.
| Alan | 2026Q3 | Yön |
|---|---|---|
| Satış TTM | 1.821 mn $ (YoY +%3) | ↑ |
| Çekirdek FAVÖK TTM | 327 mn $ (YoY +%65), marj %11.2 → %18.0 | ↑↑ |
| Net kâr TTM | −381 mn $ | → |
| Net borç | 2.698 mn $; ND/FAVÖK 17.2x → 8.3x | ↓ (iyi) |
| Net döviz poz./özkaynak | −%66 (tarihinin en iyisi) | ↑ |
| Faaliyet nakdi / FCF TTM | ≈ −190 / −217 mn $ | ↓ (zayıf) |
| Hisse sayısı | 43.8 → 52.5 mlr (+%20, May–Tem 2026; muhtemel bedelli) | sulanma |
| Değerleme | PD 1.50 mlr $; FD/FAVÖK 33x → 13.3x; PD/DD 0.38 | — |

Sonuç: **Temel skor 58/100, Yön ↑ (63), Çarpan 1.02.** Pozitif temel uyumsuzluk var ama teyitsiz (net zarar, negatif nakit, sulanma).
Mimariye geri bildirim: TMS 29 otomatik tespiti, dönem karıştırmama, net kâr ağırlığını düşürme, hisse başı metrik, sektör kıyası şartı, KAP zaman damgası, nakit/kaldıraç kırmızı bayrağı (yukarıya işlendi).
