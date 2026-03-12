import ccxt
import pandas as pd
import numpy as np
from ta.momentum import RSIIndicator, StochRSIIndicator
from ta.trend import MACD, EMAIndicator
from ta.volatility import BollingerBands, AverageTrueRange, KeltnerChannel
from datetime import datetime

class MultiTimeframeAnalyzer:
    def __init__(self, symbols):
        self.symbols  = symbols
        self.exchange = ccxt.binance({'enableRateLimit': True})

    def fetch_ohlcv(self, symbol, timeframe, limit=200):
        try:
            print(f"  [{datetime.now().strftime('%H:%M:%S')}] Veri cekiliyor: {symbol} ({timeframe})...")
            raw  = self.exchange.fetch_ohlcv(symbol, timeframe, limit=limit)
            df   = pd.DataFrame(raw, columns=['timestamp','open','high','low','close','volume'])
            df['timestamp'] = pd.to_datetime(df['timestamp'], unit='ms')
            df = df.astype({'open':float,'high':float,'low':float,'close':float,'volume':float})
            return df
        except Exception as e:
            print(f"  Veri hatasi: {e}")
            return None

    def calc_1d_trend(self, df):
        if df is None or len(df) < 200:
            return {'trend': 'UNKNOWN', 'ema50': 0, 'ema200': 0, 'direction': 'neutral'}
        ema50  = EMAIndicator(df['close'], window=50).ema_indicator().iloc[-1]
        ema200 = EMAIndicator(df['close'], window=200).ema_indicator().iloc[-1]
        price  = df['close'].iloc[-1]
        gap_pct = abs(ema50 - ema200) / ema200 * 100

        if gap_pct < 1.0:
            trend, direction = 'YATAY', 'neutral'
        elif ema50 > ema200 and price > ema50:
            trend, direction = 'GUCLU_YUKARI', 'bullish'
        elif ema50 > ema200:
            trend, direction = 'ZAYIF_YUKARI', 'bullish'
        elif ema50 < ema200 and price < ema50:
            trend, direction = 'GUCLU_ASAGI', 'bearish'
        else:
            trend, direction = 'ZAYIF_ASAGI', 'bearish'

        return {'trend': trend, 'ema50': round(ema50,2), 'ema200': round(ema200,2),
                'price': round(price,2), 'direction': direction, 'gap_pct': round(gap_pct,2)}

    def calc_4h_score(self, df):
        if df is None or len(df) < 50:
            return {'buy_score': 0, 'sell_score': 0, 'signals': {}}

        close = df['close']
        high  = df['high']
        low   = df['low']
        vol   = df['volume']

        signals = {}
        buy_score = sell_score = 0

        # RSI
        try:
            rsi = RSIIndicator(close, window=14).rsi().iloc[-1]
            signals['rsi'] = round(rsi, 1)
            if rsi < 40:   buy_score  += 2
            elif rsi > 60: sell_score += 2
        except: pass

        # MACD
        try:
            macd_obj = MACD(close)
            macd_line   = macd_obj.macd().iloc[-1]
            signal_line = macd_obj.macd_signal().iloc[-1]
            hist_curr   = macd_obj.macd_diff().iloc[-1]
            hist_prev   = macd_obj.macd_diff().iloc[-2]
            signals['macd_hist'] = round(hist_curr, 4)
            if macd_line > signal_line and hist_curr > hist_prev: buy_score  += 2
            if macd_line < signal_line and hist_curr < hist_prev: sell_score += 2
        except: pass

        # Volume
        try:
            vol_ma = vol.rolling(20).mean().iloc[-1]
            vol_ratio = vol.iloc[-1] / vol_ma if vol_ma > 0 else 1
            signals['vol_ratio'] = round(vol_ratio, 2)
            if vol_ratio > 1.5: buy_score += 2; sell_score += 2  # hacim teyit her iki yone
        except: pass

        # EMA alignment
        try:
            ema9  = EMAIndicator(close, window=9).ema_indicator().iloc[-1]
            ema21 = EMAIndicator(close, window=21).ema_indicator().iloc[-1]
            ema50 = EMAIndicator(close, window=50).ema_indicator().iloc[-1]
            signals['ema_align'] = f"{round(ema9,1)}/{round(ema21,1)}/{round(ema50,1)}"
            if ema9 > ema21 > ema50: buy_score  += 2
            if ema9 < ema21 < ema50: sell_score += 2
        except: pass

        # Squeeze momentum (BB < KC)
        try:
            bb   = BollingerBands(close, window=20, window_dev=2)
            bb_w = (bb.bollinger_hband().iloc[-1] - bb.bollinger_lband().iloc[-1])
            kc   = KeltnerChannel(high, low, close, window=20)
            kc_w = (kc.keltner_channel_hband().iloc[-1] - kc.keltner_channel_lband().iloc[-1])
            squeeze = bb_w < kc_w
            signals['squeeze'] = bool(squeeze)
            price_now  = close.iloc[-1]
            price_prev = close.iloc[-5]
            if squeeze and price_now > price_prev: buy_score  += 2
            if squeeze and price_now < price_prev: sell_score += 2
        except: pass

        # StochRSI
        try:
            srsi = StochRSIIndicator(close, window=14, smooth1=3, smooth2=3)
            srsi_k = srsi.stochrsi_k().iloc[-1]
            srsi_k_prev = srsi.stochrsi_k().iloc[-2]
            signals['stochrsi'] = round(srsi_k, 3)
            if srsi_k < 0.2 and srsi_k > srsi_k_prev: buy_score  += 1
            if srsi_k > 0.8 and srsi_k < srsi_k_prev: sell_score += 1
        except: pass

        # BB bounce
        try:
            bb2  = BollingerBands(close, window=20, window_dev=2)
            pct_b = bb2.bollinger_pband().iloc[-1]
            signals['bb_pct'] = round(pct_b, 3)
            if pct_b < 0.05: buy_score  += 1
            if pct_b > 0.95: sell_score += 1
        except: pass

        # ATR trend
        try:
            atr14 = AverageTrueRange(high, low, close, window=14).average_true_range()
            atr_curr = atr14.iloc[-1]
            atr_prev = atr14.iloc[-5]
            signals['atr'] = round(atr_curr, 2)
            signals['atr_pct'] = round(atr_curr / close.iloc[-1] * 100, 2)
            if atr_curr < atr_prev and close.iloc[-1] > close.iloc[-5]: buy_score  += 1
            if atr_curr < atr_prev and close.iloc[-1] < close.iloc[-5]: sell_score += 1
        except: pass

        # S/R simple (high/low of last 20 bars)
        try:
            recent_high = high.iloc[-20:-1].max()
            recent_low  = low.iloc[-20:-1].min()
            price       = close.iloc[-1]
            signals['sr_high'] = round(recent_high, 2)
            signals['sr_low']  = round(recent_low,  2)
            if price <= recent_low  * 1.005: buy_score  += 1
            if price >= recent_high * 0.995: sell_score += 1
        except: pass

        return {
            'buy_score':  min(buy_score,  16),
            'sell_score': min(sell_score, 16),
            'signals': signals
        }

    def classify_score(self, score):
        if score >= 13: return 'S'
        if score >= 10: return 'A'
        if score >= 7:  return 'B'
        if score >= 5:  return 'C'
        return 'F'

    def analyze(self, symbol):
        df_1d = self.fetch_ohlcv(symbol, '1d', limit=210)
        df_4h = self.fetch_ohlcv(symbol, '4h', limit=200)

        trend_1d = self.calc_1d_trend(df_1d)
        score_4h = self.calc_4h_score(df_4h)

        price = trend_1d.get('price', 0)
        direction = trend_1d.get('direction', 'neutral')
        buy_score  = score_4h['buy_score']
        sell_score = score_4h['sell_score']

        # MTF confirmation: score AND direction alignment
        mtf_buy  = buy_score  >= 3 and direction == 'bullish'
        mtf_sell = sell_score >= 3 and direction == 'bearish'
        mtf_score = buy_score if direction == 'bullish' else (sell_score if direction == 'bearish' else 0)

        return {
            'symbol':     symbol,
            'symbol_key': symbol.replace('/', '_'),
            'price':      price,
            'trend_1d':   trend_1d,
            'score_4h':   score_4h,
            'buy_score':  buy_score,
            'sell_score': sell_score,
            'mtf_score':  mtf_score,
            'mtf_class':  self.classify_score(mtf_score),
            'mtf_buy':    mtf_buy,
            'mtf_sell':   mtf_sell,
            'direction':  direction,
            'updated':    datetime.now().isoformat(),
        }
