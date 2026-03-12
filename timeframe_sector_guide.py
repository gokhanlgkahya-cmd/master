"""
TİMEFRAME, SEKTÖR VE BORSA SEÇIMI - KAPSAMLI REHBER
Hangi TF'leri seçmeliyim? Kaç taneyi aynı anda çalıştırabilirim?
"""

# ===== MEVCUT TİMEFRAMELER =====

TIMEFRAME_DETAILS = {
    "1 Dakika (1m)": {
        "avantajlar": [
            "✅ Gerçek zamanlı sinyaller",
            "✅ Çok sık trade edebilme",
            "✅ Stop loss'u hızlı koruma"
        ],
        "dezavantajları": [
            "❌ Çok volatil, false signal fazla",
            "❌ Slippage riski yüksek",
            "❌ 24/7 monitoring gerekli",
            "❌ Network/API hataları riski"
        ],
        "kullanıcı": "Scalper (Profesyonel)",
        "günde_işlem": "50-100",
        "risk": "Çok Yüksek",
        "tavsiye": "❌ Başlangıç için UYGUN DEĞİL"
    },
    
    "5 Dakika (5m)": {
        "avantajları": [
            "✅ Hızlı sinyaller",
            "✅ Günde 10+ işlem mümkün",
            "✅ Düşük slippage (1m'den)"
        ],
        "dezavantajları": [
            "❌ Hala volatil",
            "❌ Çok monitor etmek gerekli",
            "❌ Gece trade etmek zor"
        ],
        "kullanıcı": "Scalper / Intraday",
        "günde_işlem": "20-50",
        "risk": "Yüksek",
        "tavsiye": "⚠️  Deneyimli olanlar için"
    },
    
    "15 Dakika (15m)": {
        "avantajları": [
            "✅ Makul sinyal sayısı",
            "✅ Günde 5-10 işlem",
            "✅ False signals az",
            "✅ Part-time için uygun"
        ],
        "dezavantajları": [
            "❌ Gün içinde kontrol gerekli",
            "❌ Ani patlama'ları kaçırabilir"
        ],
        "kullanıcı": "Intraday Trader",
        "günde_işlem": "5-10",
        "risk": "Orta",
        "tavsiye": "✅ İYİ SEÇIM (Başlangıç için ideal)"
    },
    
    "30 Dakika (30m)": {
        "avantajları": [
            "✅ Daha güvenilir sinyaller",
            "✅ 2-3 işlem / gün",
            "✅ Gün içinde kontrol etmek kolay"
        ],
        "dezavantajları": [
            "❌ Daha az işlem",
            "❌ Trend shift'leri biraz geç görür"
        ],
        "kullanıcı": "Intraday / Swing",
        "günde_işlem": "2-5",
        "risk": "Orta",
        "tavsiye": "✅ TAVSIYE EDİLEN"
    },
    
    "1 Saat (1h)": {
        "avantajları": [
            "✅ Çok güvenilir sinyaller",
            "✅ Part-time mükemmel",
            "✅ Düşük false positive",
            "✅ Gece trade'i mümkün"
        ],
        "dezavantajları": [
            "❌ Daha az işlem (haftada 10-15)",
            "❌ Hızlı trend shift'ini kaçırabilir"
        ],
        "kullanıcı": "Swing / Day Trader",
        "günde_işlem": "2-3",
        "risk": "Düşük-Orta",
        "tavsiye": "✅ MÜKEMMEL (Başlangıç için)"
    },
    
    "4 Saat (4h)": {
        "avantajları": [
            "✅ Çok stabil sinyaller",
            "✅ Haftada 5-8 işlem",
            "✅ Risk yönetimi kolay",
            "✅ Gece rahat uyku"
        ],
        "dezavantajları": [
            "❌ Az işlem = az kar",
            "❌ Trendin çok sonunda girebilirsin"
        ],
        "kullanıcı": "Swing Trader",
        "günde_işlem": "1-2",
        "risk": "Düşük",
        "tavsiye": "✅ ÇOK İYİ (Long-term için)"
    },
    
    "1 Gün (1d)": {
        "avantajları": [
            "✅ En güvenilir sinyaller (%70+)",
            "✅ Ayda 2-3 işlem",
            "✅ Haftalarca uyku = stresten arınma",
            "✅ Manual trade yoksa automated bot ile kolay"
        ],
        "dezavantajları": [
            "❌ Çok az işlem",
            "❌ Gün ortasında entry/exit'i kaçırabilirsin"
        ],
        "kullanıcı": "Position Trader",
        "günde_işlem": "0.1-0.2",
        "risk": "Düşük",
        "tavsiye": "✅ PASSIVE INCOME İÇİN İDEAL"
    },
    
    "1 Hafta (1w)": {
        "avantajları": [
            "✅ Ayda 1-2 sinyal (çok seçici)",
            "✅ Günler monitör etmek gerekmez",
            "✅ Gücü sinyaller",
            "✅ Duygusal kararlar az"
        ],
        "dezavantajları": [
            "❌ Çok az işlem",
            "❌ Trend'in çok geç çıkabilirsin"
        ],
        "kullanıcı": "Position / Long-term",
        "günde_işlem": "0.05",
        "risk": "Çok Düşük",
        "tavsiye": "✅ Set-and-Forget sistem"
    }
}

# ===== EN UYGUN KOMBİNASYONLAR =====

RECOMMENDED_STRATEGIES = {
    "SCALPER (Günde 100 işlem)": {
        "timeframes": ['1m', '5m'],
        "sektör": 'CRYPTO',
        "işlem": "Volatilite ticareti",
        "veri_güncelleme": "Her 1 dakika",
        "başlamak": "❌ Para yatırmadan BAŞLAMAYINIZ (çok riskli)",
        "required_capital": "$1000+ min",
        "win_rate_target": "55%",
        "profit_per_trade": "0.1%"
    },
    
    "INTRADAY (5-10 işlem/gün)": {
        "timeframes": ['15m', '30m'],
        "sektör": 'CRYPTO',
        "işlem": "Trend takibi + pullback",
        "veri_güncelleme": "Her 15 dakika",
        "başlamak": "⚠️  Deneyim gerekli",
        "required_capital": "$500+ min",
        "win_rate_target": "55-60%",
        "profit_per_trade": "0.3-0.5%"
    },
    
    "SWING / DAY TRADER ⭐⭐⭐": {
        "timeframes": ['1h', '4h'],
        "sektör": 'CRYPTO',
        "işlem": "Trend başlangıcı + reversal",
        "veri_güncelleme": "Her 1 saat",
        "başlamak": "✅ BAŞLANGICIN BAŞLAMAK İÇİN İDEAL",
        "required_capital": "$100+ yeterli",
        "win_rate_target": "55-65%",
        "profit_per_trade": "0.5-1%",
        "notes": "Part-time'a mükemmel, gece rahat uyku"
    },
    
    "POSITION TRADER ⭐⭐⭐⭐": {
        "timeframes": ['4h', '1d'],
        "sektör": 'CRYPTO + STOCKS',
        "işlem": "Major trend başlangıcı",
        "veri_güncelleme": "Her 4 saat / 1 gün",
        "başlamak": "✅ BAŞLANGIÇ İÇİN HARIKA",
        "required_capital": "$100+ yeterli",
        "win_rate_target": "60-70%",
        "profit_per_trade": "1-3%",
        "notes": "Set-and-Forget, hafta 1-2 işlem"
    },
    
    "PASSIVE INCOME (Haftada 2 işlem)": {
        "timeframes": ['1d', '1w'],
        "sektör": 'CRYPTO + STOCKS + EMTIA',
        "işlem": "Major trend'e girebilme",
        "veri_güncelleme": "Her 1 gün",
        "başlamak": "✅ LAZY INVESTOR'ün RÜYASI",
        "required_capital": "$100+ yeterli",
        "win_rate_target": "65-75%",
        "profit_per_trade": "2-5%",
        "notes": "Hafta 1-2 işlem, çok stabil"
    }
}

# ===== SEKTÖRLER VE BORSALAR =====

AVAILABLE_MARKETS = {
    "🔹 KRİPTO (Binance)": {
        "borsa": "binance",
        "kostless": "✅ Tamamen ÜCRETSİZ",
        "api_key": "❌ Gerek YOK (veri çekmek için)",
        "24_7": "✅ EVET (hafta sonu dahil)",
        "commission": "0.1% (trade'de)",
        "mümkün_ciftler": 100+,
        "önemliler": [
            "BTC/USDT", "ETH/USDT", "SOL/USDT",
            "XRP/USDT", "ADA/USDT", "BNB/USDT",
            "DOGE/USDT", "LINK/USDT", "MATIC/USDT"
        ],
        "tavsiye": "✅ EN İYİ BAŞLANGIÇ (24/7, ÜCRETSİZ)",
        "volatility": "YÜKSEK (hızlı karlı/zararı)"
    },
    
    "🔹 BIST HISSE (Türkiye)": {
        "borsa": "yfinance",
        "kostless": "✅ Tamamen ÜCRETSİZ",
        "api_key": "❌ Gerek YOK",
        "24_7": "❌ 09:30 - 16:00 (iş günleri)",
        "commission": "0.001-0.005% (gerçek ticarette)",
        "mümkün_ciftler": "~500",
        "önemliler": [
            "ASELS.IS", "GARAN.IS", "EREGL.IS",
            "SASA.IS", "TOASO.IS", "THYAO.IS"
        ],
        "tavsiye": "✅ TÜRKİYE İÇİ, LOCAL",
        "volatility": "DÜŞÜK (istikrarlı, yavaş)"
    },
    
    "🔹 US STOCKS (NYSE, NASDAQ)": {
        "borsa": "yfinance",
        "kostless": "✅ Tamamen ÜCRETSİZ",
        "api_key": "❌ Gerek YOK",
        "24_7": "❌ 09:30 - 16:00 EST (hafta iş günü)",
        "commission": "0.001-0.005% (gerçek ticarette)",
        "mümkün_ciftler": "~5000+",
        "önemliler": [
            "AAPL", "MSFT", "GOOGL", "AMZN",
            "NVDA", "TSLA", "META", "NFLX"
        ],
        "tavsiye": "✅ GLOBAL, İSTİKRARLI",
        "volatility": "ORTA"
    },
    
    "🔹 EMTIA (Altın, Petrol)": {
        "borsa": "Özel broker gerekli",
        "kostless": "⚠️  API Key gerekli",
        "api_key": "✅ GEREK VAR (tic platform hesabı)",
        "24_7": "✅ Session dependent",
        "commission": "0.001-0.01%",
        "mümkün_ciftler": "~50",
        "önemliler": [
            "XAUUSD (Altın)",
            "XBRUSD (Brent Petrol)",
            "XAGUSD (Gümüş)"
        ],
        "tavsiye": "⚠️  İleri seviye",
        "volatility": "ORTA-YÜKSEK"
    }
}

# ===== ÇALIŞMA YÖNTEMI =====

EXECUTION_METHODS = {
    "1. Backtest (Geçmiş Veri)": {
        "nedir": "Geçmişteki verilerle stratejiyi test et",
        "yapı": """
        ├─ Veri Çek (2022-2024)
        ├─ Sinyalleri Üret
        ├─ Sahte Trade'ler Yap
        └─ Win Rate Ölç
        """,
        "zaman": "10 dakika (1000 bar)",
        "cost": "✅ ÜCRETSİZ",
        "güvenilirlik": "⚠️  70% (repainting risk)",
        "tavsiye": "✅ HERKESE TAVSIYE"
    },
    
    "2. Paper Trading (Sahte Para)": {
        "nedir": "Simulatör'de gerçek kurallara göre trade",
        "yapı": """
        ├─ Demo Hesap Aç
        ├─ Sinyal Al
        ├─ Sahte Trade Yap
        └─ Sonuçları İzle (Gerçek gibi)
        """,
        "zaman": "1 hafta - 1 ay",
        "cost": "✅ ÜCRETSİZ",
        "güvenilirlik": "✅ 90%",
        "tavsiye": "✅ BAŞLAMADAN YAPMANIZ LAZIM"
    },
    
    "3. Live Trading Micro": {
        "nedir": "Gerçek para ama çok küçük rakamlar",
        "yapı": """
        ├─ $10-50 ile başla
        ├─ 0.001 BTC (mikro miktarlar)
        ├─ Her işlem $1-5
        └─ Win Rate gözlemle
        """,
        "zaman": "2-3 hafta",
        "cost": "💰 Minimal ($10-50)",
        "güvenilirlik": "✅ 100%",
        "tavsiye": "✅ İLK TRADE'DEN SONRA"
    },
    
    "4. Live Trading Full": {
        "nedir": "Hesapla bütün risk yönetimi protokolü",
        "yapı": """
        ├─ 50%+ Win Rate kanıtla (backtest+paper)
        ├─ Risk/Reward 1:2 minimum
        ├─ 0.5% per trade risk
        └─ Otomatik stop loss / take profit
        """,
        "zaman": "Hiç bitmez (continuous)",
        "cost": "💰 Değişken (risk bağlı)",
        "güvenilirlik": "✅ 100%",
        "tavsiye": "✅ 3 ay paper'dan sonra"
    }
}

# ===== KAÇ TIMEFRAME AYNANDA ÇALIŞTIRABILIRIM? =====

CONCURRENT_CAPABILITY = {
    "1 PC, Python Script": {
        "cpu": "Normal (8GB RAM, i7)",
        "network": "ADSL / Fiber",
        
        "1 timeframe": {
            "örnek": "BTC/USDT 1h",
            "veri_çekme": "2 saniye",
            "analiz": "1 saniye",
            "toplam": "3 saniye",
            "mümkün": "✅ KOLAYCA"
        },
        
        "5 timeframe": {
            "örnek": "BTC/USDT (1m, 5m, 15m, 1h, 4h)",
            "veri_çekme": "10 saniye (parallel)",
            "analiz": "5 saniye",
            "toplam": "15 saniye",
            "mümkün": "✅ KOLAYCA"
        },
        
        "10 coin x 3 timeframe": {
            "örnek": "BTC, ETH, SOL, ... (1h, 4h, 1d)",
            "veri_çekme": "30 saniye (parallel)",
            "analiz": "15 saniye",
            "toplam": "45 saniye",
            "mümkün": "✅ KOLAYCA",
            "loop": "Her saatte 1 kez"
        },
        
        "100 coin x 5 timeframe": {
            "örnek": "Tüm BIST + Kripto + US Stocks",
            "veri_çekme": "3 dakika (parallel, rate limit)",
            "analiz": "2 dakika",
            "toplam": "5 dakika",
            "mümkün": "✅ YAVAŞ AMA MÜMKÜN",
            "loop": "Her 5 dakikada 1 kez"
        },
        
        "1000 coin x 5 timeframe": {
            "örnek": "TÜM PAZAR",
            "veri_çekme": "30 dakika",
            "analiz": "15 dakika",
            "toplam": "45 dakika",
            "mümkün": "❌ ÇOK YAVAŞ, rate limit'ten kırılır",
            "çözüm": "Bulut sunucu gerekli"
        }
    },
    
    "AWS / Google Cloud": {
        "cpu": "16-32 cores",
        "network": "Unlimited",
        "mümkün": "✅ Tüm pazarı aynanda (saat başı)",
        "cost": "$20-50/ay",
        "tavsiye": "Ileride gerek olursa"
    }
}

# ===== UYGULAMALI ÖRNEK: KAÇ TIMEFRAME SEÇMELI? =====

PRACTICAL_EXAMPLES = {
    "Senaryo 1: BAŞLANGIÇ (İlk hafta)": {
        "yaşanabilecek_durumlar": "Sistem ve sinyalleri test etmek",
        "seçim": {
            "timeframe": "4h + 1d",
            "coin": "BTC/USDT, ETH/USDT (sadece 2)",
            "veri_çekme": "Her 4 saat",
            "toplam_time": "30 saniye"
        },
        "kod": """
from market_scanner_system import MarketScanner
scanner = MarketScanner(strategy='swing_trader', sectors=['CRYPTO'])
scanner.run_scan()
""",
        "output": "JSON + HTML rapor",
        "beklentiler": "Sinyalleri görmek, sistemin çalışmasını anlamak"
    },
    
    "Senaryo 2: PART-TIME TRADER (Hafta)": {
        "yaşanabilecek_durumlar": "Gün içinde 2-3 işlem",
        "seçim": {
            "timeframe": "1h + 4h",
            "coin": "BTC, ETH, SOL, XRP (4 coin)",
            "veri_çekme": "Her 1 saat",
            "toplam_time": "2 dakika"
        },
        "kod": """
scanner = MarketScanner(strategy='swing_trader', sectors=['CRYPTO'])
# Saatlik task scheduler'a koy (cron)
scanner.run_scan()
""",
        "output": "Email + SMS alert sistem",
        "beklentiler": "Güçlü sinyalleri mail'de görmek"
    },
    
    "Senaryo 3: AKTIF TRADER (Gün)": {
        "yaşanabilecek_durumlar": "5-10 işlem / gün",
        "seçim": {
            "timeframe": "15m + 1h + 4h",
            "coin": "Top 10 coins",
            "veri_çekme": "Her 15 dakika",
            "toplam_time": "5 dakika"
        },
        "kod": """
scanner = MarketScanner(strategy='day_trader', sectors=['CRYPTO'])
scanner.run_continuous(duration_hours=8)
""",
        "output": "Real-time Dashboard",
        "beklentiler": "Sanal ekran'da canlı sinyalleri görmek"
    },
    
    "Senaryo 4: FULL-TIME BOT": {
        "yaşanabilecek_durumlar": "24/7 otomatik trading",
        "seçim": {
            "timeframe": "1h + 4h + 1d",
            "coin": "Top 20 coins + BIST + US Stocks",
            "veri_çekme": "Her saat",
            "toplam_time": "3 dakika"
        },
        "kod": """
scanner = MarketScanner(strategy='all_in_one', sectors=['CRYPTO', 'BIST_HISSE', 'US_STOCKS'])
scanner.run_continuous(duration_hours=999)  # Sonsuz
""",
        "output": "Telegram bot alerts",
        "beklentiler": "Sinyal olunca otomatik telegram mesajı"
    }
}

# ===== TARAF TARAF TAVSIYE =====

FINAL_RECOMMENDATIONS = """
🎯 HANGI KOMBINASYONU SEÇMELİYİM?

1️⃣  İLK GÜN (Demo / Backtest):
   ├─ Timeframe: 1h, 4h
   ├─ Coin: BTC/USDT (1 tane)
   ├─ Veri: Son 1 yıl (backtest)
   └─ Zaman: 30 dakika

2️⃣  İLK HAFTA (Paper Trading):
   ├─ Timeframe: 1h, 4h
   ├─ Coin: BTC, ETH (2 tane)
   ├─ Update: Her saat
   └─ Zaman: 5 dakika / saat

3️⃣  İLK AY (Micro Real):
   ├─ Timeframe: 1h
   ├─ Coin: BTC, ETH, SOL (3 tane)
   ├─ Risk: 0.1% per trade
   └─ Capital: $10-50

4️⃣  2-3 AY (Real Trading):
   ├─ Timeframe: 1h, 4h
   ├─ Coin: Top 5-10
   ├─ Risk: 0.5% per trade
   └─ Capital: $100+

🚀 BAŞLAMAK İÇİN EN İDEAL:
   ✅ Timeframe: 1h + 4h (2 tane)
   ✅ Coin: BTC/USDT + ETH/USDT (2 tane)
   ✅ Update: Her saat otomatik
   ✅ Ilk 2 hafta: Paper trading
   ✅ Sonra: Micro real ($10)

BUNU ÇALIŞTIRMANIZ YETERLI:
---
from market_scanner_system import MarketScanner

# Haftada 1-2 işlem (çok güvenli)
scanner = MarketScanner(strategy='swing_trader', sectors=['CRYPTO'])
scanner.run_scan()  # Her saat çalıştır
---

ZAMAN: 30 saniye / saat
HARCAMA: 0 ₺ (paper trade'ye kadar)
BAŞARI ORANSAL: 60-65% (açıklanmış)
"""

if __name__ == "__main__":
    import json
    
    print("="*80)
    print("TİMEFRAME VE SEKTÖR SEÇIMI - REHBER")
    print("="*80)
    
    print("\n📊 EN UYGUN KOMBİNASYONLAR:\n")
    for strategy, details in RECOMMENDED_STRATEGIES.items():
        print(f"{strategy}")
        print(f"  Timeframes: {details['timeframes']}")
        print(f"  Update: {details['veri_güncelleme']}")
        print(f"  {details['başlamak']}")
        print()
    
    print(FINAL_RECOMMENDATIONS)
