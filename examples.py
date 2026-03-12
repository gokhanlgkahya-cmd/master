"""
TRADING BOT KURULUM VE KULLANIŞLI ÖRNEKLERİ
"""

# ===== KURULUM =====
"""
1. Gerekli kütüphaneleri yükle:
   pip install ccxt pandas numpy ta-lib ta --break-system-packages

2. Python versiyonunu kontrol et:
   python3 --version (3.8+ gerekli)

3. API Key (isteğe bağlı):
   - Binance, Kraken, Coinbase'den key almak işlem yapacaksanız gerekli
   - Sadece veri çekmek için API KEY GEREKMEZ!
"""

# ===== ÖRNEK 1: BASIN BİR KRİPTO ANALİZİ =====
from trading_bot_analysis import analyze_crypto

# Bitcoin'i 1 saatlik mum'larla analiz et
btc = analyze_crypto('BTC/USDT', '1h')

# Ethereum'u 4 saatlik mum'larla analiz et
eth = analyze_crypto('ETH/USDT', '4h')


# ===== ÖRNEK 2: YAŞAYAN SINYAL TAKİBİ =====
import time
from datetime import datetime

def live_signal_monitoring(symbol='BTC/USDT', timeframe='1h', check_interval=3600):
    """
    Canlı sinyal takibi yapıyor
    check_interval: kontrol sıklığı (saniye cinsinden, default=1 saat)
    """
    from trading_bot_analysis import TradingSignalGenerator
    
    bot = TradingSignalGenerator(symbol=symbol, timeframe=timeframe)
    
    print(f"🚀 {symbol} canlı takibi başlıyor...")
    print(f"⏰ Kontrol sıklığı: Her {check_interval//60} dakika")
    
    iteration = 0
    while True:
        iteration += 1
        try:
            print(f"\n\n{'='*70}")
            print(f"🔄 Kontrol #{iteration} - {datetime.now()}")
            print(f"{'='*70}")
            
            # Veri çek
            bot.fetch_ohlcv_data(limit=200)
            
            # İndikatörler
            bot.calculate_indicators()
            
            # Sinyaller
            bot.generate_signals()
            
            # Göster
            signals = bot.get_latest_signals()
            
            # CSV kaydet
            bot.export_to_csv(f'live_{symbol.replace("/", "_")}_signals.csv')
            
            # Uyarı sistemi
            latest = bot.df.iloc[-1]
            
            # STRONG BUY
            strong_buy = sum([
                latest['trend_buy'],
                latest['squeeze_buy'],
                latest['bullish_engulfing']
            ]) >= 2
            
            # STRONG SELL
            strong_sell = sum([
                latest['trend_sell'],
                latest['squeeze_sell'],
                latest['bearish_engulfing']
            ]) >= 2
            
            if strong_buy:
                print("\n🎯🎯🎯 STRONG BUY SİNYALİ! 🎯🎯🎯\n")
            elif strong_sell:
                print("\n⚠️ ⚠️ ⚠️ STRONG SELL SİNYALİ! ⚠️ ⚠️ ⚠️\n")
            
            # Sonraki kontrol zamanını bekle
            print(f"\n⏳ Sonraki kontrol: {check_interval//60} dakika sonra...")
            time.sleep(check_interval)
            
        except KeyboardInterrupt:
            print("\n\n⛔ Takip durduruldu.")
            break
        except Exception as e:
            print(f"❌ Hata: {e}")
            print("🔄 5 saniye sonra tekrar deneyelim...")
            time.sleep(5)


# ===== ÖRNEK 3: BACKTESTING =====
from trading_bot_analysis import TradingSignalGenerator

def simple_backtest(symbol='BTC/USDT', timeframe='1h', initial_balance=1000):
    """
    Basit backtesting
    """
    from trading_bot_analysis import TradingSignalGenerator
    import pandas as pd
    
    print(f"\n📊 {symbol} Backtest başlıyor...")
    
    # Sinyal üret
    bot = TradingSignalGenerator(symbol=symbol, timeframe=timeframe)
    bot.fetch_ohlcv_data(limit=200)
    bot.calculate_indicators()
    bot.generate_signals()
    
    df = bot.df.copy()
    
    # Backtest parametreleri
    balance = initial_balance
    position = 0  # Kaç adet kripto holding
    entry_price = 0
    trades = []
    
    for idx, row in df.iterrows():
        # BUY SİNYALİ
        if (row['trend_buy'] or row['squeeze_buy']) and position == 0:
            position = balance / row['close']  # Tüm balansla al
            entry_price = row['close']
            balance = 0
            trades.append({
                'type': 'BUY',
                'timestamp': row['timestamp'],
                'price': row['close'],
                'amount': position
            })
            print(f"✅ BUY @ {row['close']:.2f} ({row['timestamp']})")
        
        # SELL SİNYALİ
        elif (row['trend_sell'] or row['squeeze_sell']) and position > 0:
            balance = position * row['close']
            profit_loss = balance - initial_balance
            pnl_percent = (profit_loss / initial_balance) * 100
            
            trades.append({
                'type': 'SELL',
                'timestamp': row['timestamp'],
                'price': row['close'],
                'amount': position,
                'profit_loss': profit_loss,
                'pnl_percent': pnl_percent
            })
            print(f"❌ SELL @ {row['close']:.2f} (P/L: {profit_loss:.2f}, {pnl_percent:.2f}%)")
            
            position = 0
    
    # Açık pozisyon varsa kapat
    if position > 0:
        final_price = df.iloc[-1]['close']
        balance = position * final_price
        profit_loss = balance - initial_balance
        pnl_percent = (profit_loss / initial_balance) * 100
    else:
        profit_loss = balance - initial_balance
        pnl_percent = (profit_loss / initial_balance) * 100
    
    # Sonuçlar
    print(f"\n{'='*70}")
    print("📊 BACKTEST SONUÇLARI")
    print(f"{'='*70}")
    print(f"Başlangıç: ${initial_balance:.2f}")
    print(f"Final: ${balance:.2f}")
    print(f"Kâr/Zarar: ${profit_loss:.2f} ({pnl_percent:.2f}%)")
    print(f"Toplam İşlem: {len(trades)}")
    print(f"{'='*70}\n")
    
    return trades, balance, profit_loss


# ===== ÖRNEK 4: BIST HISSE SENEDİ ANALİZİ (Finansal Verilerle) =====
def analyze_bist_stock():
    """
    BIST hisse senedi için (Özel veri kaynağı gerekli)
    Örnek: Borsa Istanbul verileri yandex.Alisa vb.'den çekilebilir
    """
    print("⚠️ BIST analizi için özel veri kaynağı gerekli!")
    print("Alternatif: yfinance kütüphanesini kullan")
    print("pip install yfinance")
    
    # Örnek:
    # import yfinance as yf
    # data = yf.download('ASELS.IS', progress=False)


# ===== ÖRNEK 5: EMTİA ANALİZİ (Futures) =====
def analyze_commodity(symbol='XAUUSD', exchange='kraken'):
    """
    Emtia analizi
    Altın (XAUUSD), Petrol (XBRUSD), vb.
    """
    from trading_bot_analysis import TradingSignalGenerator
    
    print(f"📊 {symbol} analiz başlıyor...")
    
    try:
        bot = TradingSignalGenerator(symbol=symbol, timeframe='1h', exchange_name=exchange)
        bot.fetch_ohlcv_data(limit=200)
        bot.calculate_indicators()
        bot.generate_signals()
        bot.get_latest_signals()
        return bot
    except:
        print(f"⚠️ {symbol} bulunamadı. Mevcut semboller kontrol edin.")


# ===== ÖRNEK 6: MULTI-TIMEFRAME ANALİZİ =====
def multi_timeframe_analysis(symbol='BTC/USDT'):
    """
    Aynı coin'i farklı timeframe'lerde analiz et
    """
    print(f"\n{'='*70}")
    print(f"MULTI-TIMEFRAME ANALİZİ: {symbol}")
    print(f"{'='*70}\n")
    
    from trading_bot_analysis import TradingSignalGenerator
    
    timeframes = ['1h', '4h', '1d']
    results = {}
    
    for tf in timeframes:
        print(f"\n📊 {tf.upper()} ANALYSIS")
        print("-" * 50)
        
        bot = TradingSignalGenerator(symbol=symbol, timeframe=tf)
        bot.fetch_ohlcv_data(limit=200)
        bot.calculate_indicators()
        bot.generate_signals()
        
        latest = bot.df.iloc[-1]
        
        # Özet göster
        print(f"Price: {latest['close']:.2f}")
        print(f"RSI: {latest['rsi']:.2f}")
        print(f"ADX: {latest['adx']:.2f}")
        print(f"Trend: {'UP' if latest['close'] > latest['ema_50'] else 'DOWN'}")
        
        buy_signals = sum([
            latest['trend_buy'],
            latest['squeeze_buy'],
            latest['hype_buy']
        ])
        sell_signals = sum([
            latest['trend_sell'],
            latest['squeeze_sell'],
            latest['hype_sell']
        ])
        
        print(f"Buy Signals: {buy_signals}")
        print(f"Sell Signals: {sell_signals}")
        
        results[tf] = bot
    
    return results


# ===== ÖRNEK 7: MÜLTİ PAIR ANALİZİ =====
def scan_crypto_market():
    """
    Pazar taraması - En aktif coin'leri kontrol et
    """
    from trading_bot_analysis import analyze_multiple
    
    coins = [
        'BTC/USDT',
        'ETH/USDT',
        'BNB/USDT',
        'XRP/USDT',
        'SOL/USDT',
    ]
    
    print("\n🔍 PİYASA TARAMAS BAŞLANDI\n")
    
    results = analyze_multiple(coins, timeframe='1h')
    
    # En iyi sinyalleri göster
    print("\n" + "="*70)
    print("🎯 EN İYİ SINYALLER")
    print("="*70 + "\n")
    
    for pair, bot in results.items():
        latest = bot.df.iloc[-1]
        buy_count = sum([
            latest['trend_buy'],
            latest['squeeze_buy'],
            latest['hype_buy'],
            latest['pullback_buy']
        ])
        
        if buy_count >= 2:
            print(f"✅ {pair}: {buy_count} BUY sinyali!")


if __name__ == "__main__":
    # Eğer bu dosyayı direkt çalıştırırsan:
    
    print("\n" + "="*70)
    print("TRADING BOT - KULLANIŞLI ÖRNEKLERİ")
    print("="*70)
    
    # Aşağıdakilerden birini çalıştır:
    
    # 1. Basit analiz
    # analyze_crypto('BTC/USDT', '1h')
    
    # 2. Backtest
    # simple_backtest('BTC/USDT', '1h')
    
    # 3. Multi-Timeframe
    # multi_timeframe_analysis('BTC/USDT')
    
    # 4. Pazar taraması
    # scan_crypto_market()
    
    # 5. Canlı takip (ÇALIŞIR!)
    # live_signal_monitoring('BTC/USDT', '1h', check_interval=3600)
    
    print("✅ Kodlar hazır!")
    print("📚 Örnek kullanım için yukarıdaki fonksiyonları çağır.")
