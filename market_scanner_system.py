"""
MARKET SCANNER SYSTEM - TÜM TIMEFRAME'LER, BORSALAR, SEKTÖRLER
Parallelize veri çekme + otomasyonlu analiz
"""

import ccxt
import pandas as pd
import numpy as np
from datetime import datetime, timedelta
import time
import json
from concurrent.futures import ThreadPoolExecutor, as_completed
from threading import Lock
import os

# ===== 1. TIMEFRAME VE SEKTÖR KONFİGÜRASYONU =====

TIMEFRAMES_AVAILABLE = {
    '1m': {'sn': 60, 'işlem': 'scalping', 'günü': 1440},
    '5m': {'sn': 300, 'işlem': 'scalping', 'günü': 288},
    '15m': {'sn': 900, 'işlem': 'intraday', 'günü': 96},
    '30m': {'sn': 1800, 'işlem': 'intraday', 'günü': 48},
    '1h': {'sn': 3600, 'işlem': 'day-trading', 'günü': 24},
    '4h': {'sn': 14400, 'işlem': 'swing', 'günü': 6},
    '1d': {'sn': 86400, 'işlem': 'swing', 'günü': 1},
    '1w': {'sn': 604800, 'işlem': 'position', 'günü': 0.14},
}

SECTORS = {
    'CRYPTO': {
        'borsa': 'binance',
        'çiftler': [
            'BTC/USDT', 'ETH/USDT', 'BNB/USDT', 'SOL/USDT',
            'XRP/USDT', 'ADA/USDT', 'DOGE/USDT', 'LINK/USDT',
            'MATIC/USDT', 'AVAX/USDT', 'ARB/USDT', 'OP/USDT'
        ]
    },
    
    'BIST_HISSE': {
        'borsa': 'yfinance',  # yfinance kütüphanesi gerekli
        'çiftler': [
            'ASELS.IS',  # Aselsan
            'GARAN.IS',  # Garanti
            'SASA.IS',   # Sasa Polyester
            'EREGL.IS',  # Ereğli Demir
            'TOASO.IS',  # Tofaş
            'THYAO.IS',  # Turkish Airlines
        ]
    },
    
    'US_STOCKS': {
        'borsa': 'yfinance',
        'çiftler': [
            'AAPL', 'MSFT', 'GOOGL', 'AMZN',
            'NVDA', 'TSLA', 'META', 'NFLX'
        ]
    },
    
    'EMTIA_FUTURES': {
        'borsa': 'binance',
        'çiftler': [
            'BTC/USDT',  # Kripto (bu yana da var)
        ]
        # Not: Gerçek emtia futures için brokerage API gerekli
    }
}

# ===== 2. TIMEFRAME SEÇİM STRATEJİSİ =====

SCANNING_STRATEGY = {
    "scalper": {
        "timeframes": ['1m', '5m', '15m'],
        "günceleme_sıklığı": 60,  # Her 1 dakika
        "işlem": "Dakikada 10+ işlem"
    },
    
    "day_trader": {
        "timeframes": ['15m', '30m', '1h'],
        "günceleme_sıklığı": 900,  # Her 15 dakika
        "işlem": "Günde 3-5 işlem"
    },
    
    "swing_trader": {
        "timeframes": ['1h', '4h', '1d'],
        "günceleme_sıklığı": 3600,  # Her 1 saat
        "işlem": "Haftada 3-5 işlem"
    },
    
    "position_trader": {
        "timeframes": ['1d', '1w'],
        "günceleme_sıklığı": 86400,  # Her 1 gün
        "işlem": "Ayda 2-3 işlem"
    },
    
    "all_in_one": {
        "timeframes": ['1h', '4h', '1d'],  # ← TAVSIYE EDİLEN
        "günceleme_sıklığı": 3600,
        "işlem": "Multi-timeframe analiz"
    }
}

# ===== 3. VERİ ÇEKME YAPISI =====

class DataFetcher:
    """Parallelize veri çekme sistemi"""
    
    def __init__(self, max_workers=5):
        self.exchanges = {}
        self.lock = Lock()
        self.max_workers = max_workers
        self.cache = {}  # Veri cache'leme
        
    def setup_exchange(self, exchange_name):
        """Exchange'i başlat"""
        try:
            if exchange_name == 'binance':
                return ccxt.binance({'enableRateLimit': True})
            elif exchange_name == 'kraken':
                return ccxt.kraken({'enableRateLimit': True})
            elif exchange_name == 'coinbase':
                return ccxt.coinbase({'enableRateLimit': True})
            elif exchange_name == 'yfinance':
                import yfinance as yf
                return yf
            else:
                return ccxt.binance({'enableRateLimit': True})
        except Exception as e:
            print(f"❌ Exchange başlatma hatası ({exchange_name}): {e}")
            return None
    
    def fetch_crypto(self, symbol, timeframe, exchange, limit=200):
        """Kripto verisi çek"""
        try:
            ohlcv = exchange.fetch_ohlcv(symbol, timeframe, limit=limit)
            df = pd.DataFrame(
                ohlcv,
                columns=['timestamp', 'open', 'high', 'low', 'close', 'volume']
            )
            df['timestamp'] = pd.to_datetime(df['timestamp'], unit='ms')
            df['symbol'] = symbol
            df['timeframe'] = timeframe
            return df
        except Exception as e:
            print(f"❌ {symbol} {timeframe} veri hatası: {e}")
            return None
    
    def fetch_stock(self, symbol, timeframe='1h'):
        """Hisse senedi verisi çek (yfinance)"""
        try:
            import yfinance as yf
            
            # Timeframe'i yfinance formatına çevir
            tf_map = {
                '1m': '1m', '5m': '5m', '15m': '15m', '30m': '30m',
                '1h': '1h', '4h': '1h',  # 4h için 1h'lik veri çek
                '1d': '1d', '1w': '1wk'
            }
            
            tf = tf_map.get(timeframe, '1h')
            data = yf.download(
                symbol,
                period='3mo' if timeframe in ['1d', '1w'] else '1mo',
                interval=tf,
                progress=False
            )
            
            if data.empty:
                return None
            
            # DataFrame'i standardize et
            data.rename(columns={
                'Open': 'open',
                'High': 'high',
                'Low': 'low',
                'Close': 'close',
                'Volume': 'volume'
            }, inplace=True)
            
            data = data[['open', 'high', 'low', 'close', 'volume']].copy()
            data['symbol'] = symbol
            data['timeframe'] = timeframe
            data = data.reset_index()
            data.rename(columns={'Date': 'timestamp'}, inplace=True)
            
            return data.tail(200)  # Son 200'ü al
            
        except Exception as e:
            print(f"❌ {symbol} stok verisi hatası: {e}")
            return None
    
    def fetch_market_data(self, symbol, timeframe, sector):
        """Pazardan veri çek (otomatik borsa seçimi)"""
        sector_info = SECTORS.get(sector)
        if not sector_info:
            print(f"❌ Sektor bulunamadı: {sector}")
            return None
        
        exchange_name = sector_info['borsa']
        
        # Cache kontrol
        cache_key = f"{symbol}_{timeframe}_{sector}"
        if cache_key in self.cache:
            cached_time, cached_data = self.cache[cache_key]
            if datetime.now() - cached_time < timedelta(minutes=5):
                return cached_data
        
        # Veri çek
        if exchange_name == 'yfinance':
            data = self.fetch_stock(symbol, timeframe)
        else:
            exchange = self.setup_exchange(exchange_name)
            if exchange is None:
                return None
            data = self.fetch_crypto(symbol, timeframe, exchange)
        
        # Cache'le
        if data is not None:
            self.cache[cache_key] = (datetime.now(), data)
        
        return data
    
    def fetch_all_parallel(self, config, timeframes, sectors=None):
        """Tüm veriyi parallel olarak çek"""
        
        if sectors is None:
            sectors = ['CRYPTO']
        
        tasks = {}
        results = {}
        
        with ThreadPoolExecutor(max_workers=self.max_workers) as executor:
            for sector in sectors:
                sector_pairs = SECTORS[sector]['çiftler']
                
                for symbol in sector_pairs:
                    for timeframe in timeframes:
                        task = executor.submit(
                            self.fetch_market_data,
                            symbol, timeframe, sector
                        )
                        tasks[task] = (symbol, timeframe, sector)
            
            # Sonuçları topla
            completed = 0
            total = len(tasks)
            
            for task in as_completed(tasks):
                completed += 1
                symbol, timeframe, sector = tasks[task]
                
                try:
                    data = task.result()
                    if data is not None:
                        results[(symbol, timeframe, sector)] = data
                        print(f"✅ [{completed}/{total}] {symbol} {timeframe} ({sector})")
                    else:
                        print(f"⏭️  [{completed}/{total}] {symbol} {timeframe} - Veri yok")
                except Exception as e:
                    print(f"❌ [{completed}/{total}] {symbol} {timeframe} - Hata: {e}")
        
        return results


# ===== 4. ÇOKLU TİMEFRAME ANALİZ =====

class MultiTimeframeAnalyzer:
    """Aynı coin'i birden çok timeframe'de analiz"""
    
    def __init__(self):
        self.results = {}
    
    def analyze_symbol(self, symbol, timeframes, sector='CRYPTO'):
        """Bir sembolü tüm timeframe'lerde analiz et"""
        
        from trading_bot_analysis import TradingSignalGenerator
        
        fetcher = DataFetcher()
        symbol_results = {}
        
        print(f"\n{'='*70}")
        print(f"📊 MULTI-TIMEFRAME ANALİZİ: {symbol}")
        print(f"{'='*70}\n")
        
        for tf in timeframes:
            print(f"⏳ {tf.upper()} analiz ediliyor...")
            
            data = fetcher.fetch_market_data(symbol, tf, sector)
            
            if data is None:
                print(f"   ⏭️  Veri alınamadı\n")
                continue
            
            try:
                # Sinyal üret
                bot = TradingSignalGenerator(symbol=symbol, timeframe=tf)
                bot.df = data.copy()
                bot.calculate_indicators()
                bot.generate_signals()
                
                latest = bot.df.iloc[-1]
                
                # Özet
                print(f"   Price: {latest['close']:.2f}")
                print(f"   RSI: {latest['rsi']:.0f} | ADX: {latest['adx']:.0f}")
                
                buy_count = sum([
                    latest['trend_buy'],
                    latest['squeeze_buy'],
                    latest['hype_buy']
                ])
                
                print(f"   Signals: {buy_count} BUY")
                print()
                
                symbol_results[tf] = {
                    'price': latest['close'],
                    'rsi': latest['rsi'],
                    'adx': latest['adx'],
                    'buy_signals': buy_count,
                    'bot': bot
                }
                
            except Exception as e:
                print(f"   ❌ Analiz hatası: {e}\n")
        
        self.results[symbol] = symbol_results
        return symbol_results
    
    def get_trend_summary(self, symbol):
        """Sembol için trend özeti"""
        
        if symbol not in self.results:
            return None
        
        results = self.results[symbol]
        
        # Trend uyumu kontrol
        uptrend_count = 0
        downtrend_count = 0
        
        for tf, data in results.items():
            if data['buy_signals'] > 0:
                uptrend_count += 1
            else:
                downtrend_count += 1
        
        if uptrend_count >= len(results) * 0.66:
            trend = "🔺 STRONG UPTREND"
        elif downtrend_count >= len(results) * 0.66:
            trend = "🔻 STRONG DOWNTREND"
        else:
            trend = "↔️  MIXED / CONSOLIDATING"
        
        return {
            'symbol': symbol,
            'trend': trend,
            'uptrend_timeframes': uptrend_count,
            'downtrend_timeframes': downtrend_count,
            'total_timeframes': len(results)
        }


# ===== 5. OTOMASYONLU MARKET SCANNER =====

class MarketScanner:
    """Tüm pazarı otomatik tarayıp sinyalleri bulur"""
    
    def __init__(self, strategy='all_in_one', sectors=None):
        """
        strategy: 'scalper', 'day_trader', 'swing_trader', 'position_trader', 'all_in_one'
        sectors: ['CRYPTO', 'BIST_HISSE', 'US_STOCKS']
        """
        self.strategy = SCANNING_STRATEGY[strategy]
        self.sectors = sectors or ['CRYPTO']
        self.fetcher = DataFetcher(max_workers=10)
        self.analyzer = MultiTimeframeAnalyzer()
        self.update_interval = self.strategy['günceleme_sıklığı']
        
    def run_scan(self, output_file='market_scan.json'):
        """Tek scan çalıştır"""
        
        print(f"\n{'='*70}")
        print(f"🔍 MARKET SCAN BAŞLIYOR - {datetime.now()}")
        print(f"Strategy: {self.strategy}")
        print(f"{'='*70}\n")
        
        timeframes = self.strategy['timeframes']
        
        # Tüm veriyi parallel çek
        all_data = self.fetcher.fetch_all_parallel(
            self.strategy,
            timeframes,
            self.sectors
        )
        
        # Sonuçları analiz et
        results = {
            'timestamp': datetime.now().isoformat(),
            'strategy': self.strategy,
            'sectors': self.sectors,
            'timeframes': timeframes,
            'strong_signals': [],
            'all_symbols': {}
        }
        
        print(f"\n{'='*70}")
        print("📊 SINYAL ANALİZİ YAPILIYOR...")
        print(f"{'='*70}\n")
        
        for (symbol, timeframe, sector), data in all_data.items():
            print(f"Analiz: {symbol} {timeframe}")
            
            try:
                from trading_bot_analysis import TradingSignalGenerator
                
                bot = TradingSignalGenerator(symbol=symbol, timeframe=timeframe)
                bot.df = data.copy()
                bot.calculate_indicators()
                bot.generate_signals()
                
                latest = bot.df.iloc[-1]
                
                buy_signals = sum([
                    latest['trend_buy'],
                    latest['squeeze_buy'],
                    latest['hype_buy'],
                    latest['bullish_engulfing']
                ])
                
                sell_signals = sum([
                    latest['trend_sell'],
                    latest['squeeze_sell'],
                    latest['hype_sell'],
                    latest['bearish_engulfing']
                ])
                
                symbol_key = f"{symbol}_{timeframe}_{sector}"
                
                results['all_symbols'][symbol_key] = {
                    'symbol': symbol,
                    'timeframe': timeframe,
                    'sector': sector,
                    'price': float(latest['close']),
                    'rsi': float(latest['rsi']),
                    'adx': float(latest['adx']),
                    'buy_signals': int(buy_signals),
                    'sell_signals': int(sell_signals)
                }
                
                # Güçlü sinyaleri kaydet
                if buy_signals >= 2:
                    results['strong_signals'].append({
                        'type': 'BUY',
                        'symbol': symbol,
                        'timeframe': timeframe,
                        'sector': sector,
                        'strength': buy_signals,
                        'price': float(latest['close']),
                        'rsi': float(latest['rsi']),
                        'adx': float(latest['adx'])
                    })
                
                if sell_signals >= 2:
                    results['strong_signals'].append({
                        'type': 'SELL',
                        'symbol': symbol,
                        'timeframe': timeframe,
                        'sector': sector,
                        'strength': sell_signals,
                        'price': float(latest['close']),
                        'rsi': float(latest['rsi']),
                        'adx': float(latest['adx'])
                    })
                
            except Exception as e:
                print(f"   ⚠️  Hata: {e}")
        
        # Sonuçları göster
        print(f"\n{'='*70}")
        print("🎯 GÜÇLÜ SİNYALLER")
        print(f"{'='*70}\n")
        
        if results['strong_signals']:
            # Sıralayıp göster
            buy_signals = [s for s in results['strong_signals'] if s['type'] == 'BUY']
            sell_signals = [s for s in results['strong_signals'] if s['type'] == 'SELL']
            
            if buy_signals:
                print("✅ BUY SİNYALLERİ:")
                for sig in sorted(buy_signals, key=lambda x: x['strength'], reverse=True):
                    print(f"   {sig['symbol']:12} {sig['timeframe']:4} - Strength: {sig['strength']} (ADX: {sig['adx']:.0f})")
            
            if sell_signals:
                print("\n❌ SELL SİNYALLERİ:")
                for sig in sorted(sell_signals, key=lambda x: x['strength'], reverse=True):
                    print(f"   {sig['symbol']:12} {sig['timeframe']:4} - Strength: {sig['strength']} (ADX: {sig['adx']:.0f})")
        else:
            print("   ➖ Güçlü sinyal yok")
        
        # JSON'a kaydet
        with open(output_file, 'w') as f:
            json.dump(results, f, indent=2)
        
        print(f"\n✅ Sonuçlar '{output_file}' olarak kaydedildi!")
        
        return results
    
    def run_continuous(self, duration_hours=24):
        """Sürekli tarama çalıştır"""
        
        start_time = datetime.now()
        end_time = start_time + timedelta(hours=duration_hours)
        iteration = 0
        
        while datetime.now() < end_time:
            iteration += 1
            
            print(f"\n\n{'#'*70}")
            print(f"# SCAN #{iteration} - {datetime.now()}")
            print(f"# Sonraki scan: {(datetime.now() + timedelta(seconds=self.update_interval)).strftime('%H:%M:%S')}")
            print(f"{'#'*70}")
            
            self.run_scan(output_file=f'scan_{iteration}.json')
            
            # Sonraki update'e kadar bekle
            time.sleep(self.update_interval)


# ===== 6. RAPOR OLUŞTURUCU =====

class ReportGenerator:
    """Tarama sonuçlarından rapor oluştur"""
    
    @staticmethod
    def generate_html_report(scan_results, output_file='report.html'):
        """HTML rapor oluştur"""
        
        html = f"""
        <!DOCTYPE html>
        <html>
        <head>
            <title>Market Scan Report</title>
            <style>
                body {{ font-family: Arial; margin: 20px; }}
                .header {{ background: #1a1a1a; color: white; padding: 20px; border-radius: 5px; }}
                .signal {{ padding: 10px; margin: 5px 0; border-radius: 5px; }}
                .buy {{ background: #90EE90; }}
                .sell {{ background: #FFB6C6; }}
                table {{ border-collapse: collapse; width: 100%; margin: 20px 0; }}
                th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
                th {{ background: #4CAF50; color: white; }}
            </style>
        </head>
        <body>
            <div class="header">
                <h1>📊 Market Scan Report</h1>
                <p>Timestamp: {scan_results['timestamp']}</p>
                <p>Strategy: {scan_results['strategy']}</p>
            </div>
            
            <h2>🎯 Güçlü Sinyaller ({len(scan_results['strong_signals'])})</h2>
        """
        
        for sig in scan_results['strong_signals']:
            css_class = 'buy' if sig['type'] == 'BUY' else 'sell'
            html += f"""
            <div class="signal {css_class}">
                <strong>{sig['type']}</strong> - {sig['symbol']} {sig['timeframe']} 
                (Gücü: {sig['strength']}/4) | Price: ${sig['price']:.2f} | ADX: {sig['adx']:.0f}
            </div>
            """
        
        # Tüm semboller tablosu
        html += """
        <h2>📋 Tüm Semboller</h2>
        <table>
            <tr>
                <th>Symbol</th>
                <th>TF</th>
                <th>Price</th>
                <th>RSI</th>
                <th>ADX</th>
                <th>Buy</th>
                <th>Sell</th>
            </tr>
        """
        
        for key, data in scan_results['all_symbols'].items():
            html += f"""
            <tr>
                <td>{data['symbol']}</td>
                <td>{data['timeframe']}</td>
                <td>${data['price']:.2f}</td>
                <td>{data['rsi']:.0f}</td>
                <td>{data['adx']:.0f}</td>
                <td>{data['buy_signals']}</td>
                <td>{data['sell_signals']}</td>
            </tr>
            """
        
        html += """
        </table>
        </body>
        </html>
        """
        
        with open(output_file, 'w') as f:
            f.write(html)
        
        print(f"✅ HTML rapor: {output_file}")


# ===== 7. KULLANIM ÖRNEKLERI =====

if __name__ == "__main__":
    
    print("="*70)
    print("MARKET SCANNER SYSTEM - DEMO")
    print("="*70)
    
    # ÖRNEK 1: Swing Trader için tarama (1h, 4h, 1d)
    # ========================================
    print("\n\n🔍 ÖRNEK 1: SWING TRADER SCANI")
    print("-" * 70)
    
    scanner = MarketScanner(
        strategy='swing_trader',  # 1h, 4h, 1d
        sectors=['CRYPTO']  # Sadece kripto
    )
    
    results = scanner.run_scan(output_file='swing_trader_scan.json')
    
    # Rapor oluştur
    ReportGenerator.generate_html_report(results, 'swing_trader_report.html')
    
    
    # ÖRNEK 2: Multi-timeframe analiz (tek coin)
    # ==========================================
    print("\n\n📊 ÖRNEK 2: MULTI-TIMEFRAME ANALİZİ (BTC)")
    print("-" * 70)
    
    analyzer = MultiTimeframeAnalyzer()
    btc_analysis = analyzer.analyze_symbol(
        'BTC/USDT',
        timeframes=['1h', '4h', '1d'],
        sector='CRYPTO'
    )
    
    trend_summary = analyzer.get_trend_summary('BTC/USDT')
    print(f"\n{trend_summary}")
    
    
    # ÖRNEK 3: Sürekli tarama (24 saat)
    # ================================
    print("\n\n⏱️  ÖRNEK 3: SÜREKLI TARAMA (Yorum yapıldı)")
    print("-" * 70)
    print("scanner.run_continuous(duration_hours=24)  # Etkinleştirmek için aç")
    
    
    # ÖRNEK 4: Day Trader için tarama
    # ==============================
    print("\n\n📈 ÖRNEK 4: DAY TRADER SCANI (15m, 30m, 1h)")
    print("-" * 70)
    
    day_scanner = MarketScanner(
        strategy='day_trader',
        sectors=['CRYPTO']
    )
    
    day_results = day_scanner.run_scan(output_file='day_trader_scan.json')
