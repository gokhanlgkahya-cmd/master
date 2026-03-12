"""
RiskPlatform v2.0 — Sinyal Motoru
===================================
7 Sinyal: Squeeze, Range, Hype, Trend, NewTrend, Pullback, AlphaTrend
MTF: 1D (yön) + 4H (giriş)
Veri: Binance ccxt (kripto) + yfinance (BIST/emtia/FX)
Bar: 420 (EMA200 güvenilirliği için)
"""

import ccxt
import yfinance as yf
import pandas as pd
import numpy as np
from ta.momentum import RSIIndicator, StochRSIIndicator
from ta.trend import MACD, EMAIndicator, ADXIndicator
from ta.volatility import BollingerBands, AverageTrueRange, KeltnerChannel
from datetime import datetime
from typing import Dict, List, Optional, Any


# ═══════════════════════════════════════════════════════════════
# YARDIMCI FONKSİYONLAR
# ═══════════════════════════════════════════════════════════════

def crossover(series_a: pd.Series, series_b: pd.Series) -> pd.Series:
    """Pine Script ta.crossover karşılığı: a, b'yi yukarı kesiyor."""
    return (series_a > series_b) & (series_a.shift(1) <= series_b.shift(1))


def crossunder(series_a: pd.Series, series_b: pd.Series) -> pd.Series:
    """Pine Script ta.crossunder karşılığı: a, b'yi aşağı kesiyor."""
    return (series_a < series_b) & (series_a.shift(1) >= series_b.shift(1))


def linreg(series: pd.Series, length: int, offset: int = 0) -> pd.Series:
    """Lineer regresyon değeri (LazyBear Squeeze momentum val)."""
    result = pd.Series(index=series.index, dtype=float)
    for i in range(length - 1, len(series)):
        y = series.iloc[i - length + 1:i + 1].values
        if len(y) < length or np.any(np.isnan(y)):
            result.iloc[i] = np.nan
            continue
        x = np.arange(length)
        try:
            coeffs = np.polyfit(x, y, 1)
            result.iloc[i] = coeffs[0] * (length - 1 - offset) + coeffs[1]
        except:
            result.iloc[i] = np.nan
    return result


def barssince(condition: pd.Series) -> pd.Series:
    """Pine Script ta.barssince karşılığı."""
    result = pd.Series(index=condition.index, dtype=float)
    count = np.nan
    for i in range(len(condition)):
        if condition.iloc[i]:
            count = 0
        elif not np.isnan(count):
            count += 1
        result.iloc[i] = count
    return result


# ═══════════════════════════════════════════════════════════════
# İNDİKATÖR HESAPLAMA
# ═══════════════════════════════════════════════════════════════

def calc_indicators(df: pd.DataFrame) -> pd.DataFrame:
    """
    Tüm indikatörleri hesapla.
    Gerekli sütunlar: open, high, low, close, volume
    """
    if df is None or len(df) < 210:
        return df

    close = df['close']
    high = df['high']
    low = df['low']
    volume = df['volume']

    # ─── EMA'lar (20, 50, 200) ───
    df['ema20'] = EMAIndicator(close, window=20).ema_indicator()
    df['ema50'] = EMAIndicator(close, window=50).ema_indicator()
    df['ema200'] = EMAIndicator(close, window=200).ema_indicator()

    # ─── RSI + RSI MA5 ───
    rsi_ind = RSIIndicator(close, window=14)
    df['rsi'] = rsi_ind.rsi()
    df['rsi_ma5'] = df['rsi'].rolling(5).mean()

    # ─── ADX ───
    adx_ind = ADXIndicator(high, low, close, window=14)
    df['adx'] = adx_ind.adx()
    df['adx_pos'] = adx_ind.adx_pos()  # +DI
    df['adx_neg'] = adx_ind.adx_neg()  # -DI

    # ─── Bollinger Bands ───
    bb = BollingerBands(close, window=20, window_dev=2)
    df['bb_upper'] = bb.bollinger_hband()
    df['bb_lower'] = bb.bollinger_lband()
    df['bb_mid'] = bb.bollinger_mavg()
    df['bb_pct'] = bb.bollinger_pband()

    # ─── Keltner Channel ───
    kc = KeltnerChannel(high, low, close, window=20)
    df['kc_upper'] = kc.keltner_channel_hband()
    df['kc_lower'] = kc.keltner_channel_lband()
    df['kc_mid'] = (df['kc_upper'] + df['kc_lower']) / 2

    # ─── Squeeze (LazyBear) ───
    df['sqz_on'] = (df['bb_lower'] > df['kc_lower']) & (df['bb_upper'] < df['kc_upper'])
    df['sqz_off'] = ~df['sqz_on']

    # Squeeze momentum value (LazyBear yöntemi)
    highest_high = high.rolling(20).max()
    lowest_low = low.rolling(20).min()
    avg_hl = (highest_high + lowest_low) / 2
    avg_mid = (avg_hl + df['kc_mid']) / 2
    delta = close - avg_mid
    df['sqz_val'] = linreg(delta, 20, 0)

    # ─── ATR ───
    atr_ind = AverageTrueRange(high, low, close, window=14)
    df['atr'] = atr_ind.average_true_range()
    df['atr_pct'] = df['atr'] / close * 100

    # ─── MACD ───
    macd_ind = MACD(close)
    df['macd'] = macd_ind.macd()
    df['macd_signal'] = macd_ind.macd_signal()
    df['macd_hist'] = macd_ind.macd_diff()

    # ─── Volume ───
    df['vol_ma20'] = volume.rolling(20).mean()
    df['vol_ratio'] = volume / df['vol_ma20']

    # ─── AlphaTrend (Kıvanç Özbilgiç) ───
    # Parametreler: period=14, coeff=1, RSI kullan
    alpha_period = 14
    alpha_coeff = 1
    alpha_atr = df['atr']  # SMA(TR, 14) yaklaşık ATR(14)
    
    up_t = low - alpha_atr * alpha_coeff
    down_t = high + alpha_atr * alpha_coeff
    
    # RSI >= 50 koşulu
    use_condition = df['rsi'] >= 50
    
    alpha_trend = pd.Series(0.0, index=df.index)
    for i in range(1, len(df)):
        prev = alpha_trend.iloc[i - 1]
        if use_condition.iloc[i]:
            alpha_trend.iloc[i] = max(up_t.iloc[i], prev) if not np.isnan(prev) else up_t.iloc[i]
        else:
            alpha_trend.iloc[i] = min(down_t.iloc[i], prev) if not np.isnan(prev) else down_t.iloc[i]
    
    df['alpha_trend'] = alpha_trend
    df['alpha_trend_2'] = alpha_trend.shift(2)

    return df


# ═══════════════════════════════════════════════════════════════
# 7 SİNYAL HESAPLAMA
# ═══════════════════════════════════════════════════════════════

def calc_signals(df: pd.DataFrame) -> pd.DataFrame:
    """
    7 sinyal tipi hesapla — Pine Script birebir çeviri.
    Her sinyal Buy ve Sell olarak ayrı sütun.
    """
    if df is None or len(df) < 210:
        return df

    close = df['close']
    rsi = df['rsi']
    rsi_ma5 = df['rsi_ma5']
    adx = df['adx']
    bb_upper = df['bb_upper']
    bb_lower = df['bb_lower']
    kc_upper = df['kc_upper']
    kc_lower = df['kc_lower']
    ema20 = df['ema20']
    ema50 = df['ema50']
    ema200 = df['ema200']
    sqz_on = df['sqz_on']
    sqz_off = df['sqz_off']
    sqz_val = df['sqz_val']
    alpha_trend = df['alpha_trend']
    alpha_trend_2 = df['alpha_trend_2']

    # ─── 1. SQUEEZE ───
    df['squeeze_buy'] = (
        sqz_off &
        sqz_on.shift(1) &
        (sqz_val > 0) &
        crossover(sqz_val, sqz_val.shift(1)) &
        (rsi_ma5 < rsi)
    ).fillna(False)

    df['squeeze_sell'] = (
        sqz_off &
        sqz_on.shift(1) &
        (sqz_val < 0) &
        crossunder(sqz_val, sqz_val.shift(1)) &
        (rsi < rsi_ma5)
    ).fillna(False)

    # ─── 2. RANGE ───
    df['range_buy'] = (
        (crossover(rsi, rsi_ma5) & (rsi < 30) & (rsi_ma5 < rsi_ma5.shift(1)) & (adx < 20)) |
        crossover(rsi, pd.Series(30, index=df.index))
    ).fillna(False)

    df['range_sell'] = (
        (crossunder(rsi, rsi_ma5) & (rsi > 70) & (rsi_ma5 > rsi_ma5.shift(1)) & (adx < 20)) |
        crossunder(rsi, pd.Series(70, index=df.index))
    ).fillna(False)

    # ─── 3. HYPE ───
    bb_expanding = (bb_upper > bb_upper.shift(1)) & (bb_lower < bb_lower.shift(1))
    bb_outside_kc = (bb_upper > kc_upper) & (bb_lower < kc_lower)

    df['hype_buy'] = (
        bb_expanding & bb_outside_kc &
        (ema200 < ema50) & (ema50 < ema20) &
        crossover(close, ema20) &
        ((adx > 25) | (adx > adx.shift(1)))
    ).fillna(False)

    df['hype_sell'] = (
        bb_expanding & bb_outside_kc &
        (ema200 > ema50) & (ema50 > ema20) &
        (ema20 > close) &
        ((adx > 25) | (adx > adx.shift(1)))
    ).fillna(False)

    # ─── 4. ALPHATREND ───
    at_buy_signal = crossover(alpha_trend, alpha_trend_2)
    at_sell_signal = crossunder(alpha_trend, alpha_trend_2)

    k1 = barssince(at_buy_signal)
    k2 = barssince(at_sell_signal)
    o1 = barssince(at_buy_signal.shift(1))
    o2 = barssince(at_sell_signal.shift(1))

    df['alpha_buy'] = (at_buy_signal & (o1 > k2)).fillna(False)
    df['alpha_sell'] = (at_sell_signal & (o2 > k1)).fillna(False)

    # ─── 5. TREND ───
    df['trend_buy'] = (
        ((close > ema50) & crossover(ema50, ema200) & crossover(ema20, ema50)) |
        (df['alpha_buy'] & (adx >= 20))
    ).fillna(False)

    df['trend_sell'] = (
        ((close < ema50) & crossunder(ema50, ema200) & crossunder(ema20, ema50)) |
        (df['alpha_sell'] & (adx >= 20))
    ).fillna(False)

    # ─── 6. NEWTREND ───
    adx_declining = (adx < adx.shift(1)) & (adx.shift(1) < adx.shift(3)) & (adx.shift(3) < adx.shift(5))
    bb_upper_declining = (bb_upper < bb_upper.shift(1)) & (bb_upper.shift(1) < bb_upper.shift(2))
    bb_lower_rising = (bb_lower > bb_lower.shift(1)) & (bb_lower.shift(1) > bb_lower.shift(2))

    df['newtrend_buy'] = (
        adx_declining & bb_upper_declining &
        ((adx > 65) | (adx < 15)) &
        (ema200 > close) & (ema50 < ema200)
    ).fillna(False)

    df['newtrend_sell'] = (
        adx_declining & bb_lower_rising &
        ((adx > 65) | (adx < 15)) &
        (ema200 < close) & (ema50 > ema200)
    ).fillna(False)

    # ─── 7. PULLBACK ───
    ema_gap = ema200 - ema50
    ema_gap_prev = ema_gap.shift(1)

    df['pullback_buy'] = (
        (rsi_ma5 > rsi_ma5.shift(1)) &
        (close < ema200) & (close > ema50) &
        (ema_gap < ema_gap_prev) &
        (ema20 < ema50)
    ).fillna(False)

    df['pullback_sell'] = (
        (rsi_ma5 < rsi_ma5.shift(1)) &
        (close > ema200) & (close < ema50) &
        (ema_gap > ema_gap_prev) &
        (ema20 > ema50)
    ).fillna(False)

    return df


# ═══════════════════════════════════════════════════════════════
# SİNYAL DURUMU ÇIKARIMI
# ═══════════════════════════════════════════════════════════════

def extract_signal_status(df: pd.DataFrame, lookback: int = 5) -> Dict[str, Any]:
    """
    Son N bar içinde aktif sinyalleri tespit et.
    
    Returns:
        {
            'direction': 'bullish' | 'bearish' | 'neutral',
            'signal_status_1d': 'trend_alpha' | 'trend_only' | ... | 'conflict',
            'signal_status_4h': 'squeeze_buy' | 'range_sell' | ... | None,
            'active_signals': [...],
            'adx': float, 'adx_prev': float,
            'days_since_signal': int,
            'price_vs_ema200_pct': float,
            'atr_pct': float,
            ...
        }
    """
    if df is None or len(df) < 210:
        return {'direction': 'neutral', 'signal_status_1d': 'none',
                'signal_status_4h': None, 'active_signals': []}

    last = df.iloc[-1]
    recent = df.iloc[-lookback:]

    result = {
        'price': round(float(last['close']), 6),
        'ema20': round(float(last.get('ema20', 0)), 6),
        'ema50': round(float(last.get('ema50', 0)), 6),
        'ema200': round(float(last.get('ema200', 0)), 6),
        'rsi': round(float(last.get('rsi', 50)), 2),
        'rsi_ma5': round(float(last.get('rsi_ma5', 50)), 2),
        'adx': round(float(last.get('adx', 20)), 2),
        'adx_prev': round(float(df['adx'].iloc[-6]) if len(df) > 6 else 20, 2),
        'atr': round(float(last.get('atr', 0)), 6),
        'atr_pct': round(float(last.get('atr_pct', 0)), 2),
        'sqz_val': round(float(last.get('sqz_val', 0)), 6),
        'sqz_on': bool(last.get('sqz_on', False)),
        'bb_pct': round(float(last.get('bb_pct', 0.5)), 4),
        'vol_ratio': round(float(last.get('vol_ratio', 1)), 2),
        'macd_hist': round(float(last.get('macd_hist', 0)), 6),
        'alpha_trend': round(float(last.get('alpha_trend', 0)), 6),
    }

    # EMA200 mesafe %
    ema200_val = last.get('ema200', 0)
    price_val = last['close']
    if ema200_val and ema200_val > 0:
        result['price_vs_ema200_pct'] = round((price_val - ema200_val) / ema200_val * 100, 2)
    else:
        result['price_vs_ema200_pct'] = 0.0

    # EMA dizilimi
    if last.get('ema20', 0) > last.get('ema50', 0) > last.get('ema200', 0):
        result['ema_alignment'] = 'bullish'
    elif last.get('ema20', 0) < last.get('ema50', 0) < last.get('ema200', 0):
        result['ema_alignment'] = 'bearish'
    else:
        result['ema_alignment'] = 'mixed'

    # Aktif sinyalleri topla
    buy_signals = []
    sell_signals = []
    signal_types = ['squeeze', 'range', 'hype', 'trend', 'newtrend', 'pullback', 'alpha']

    for sig in signal_types:
        buy_col = f'{sig}_buy'
        sell_col = f'{sig}_sell'
        if buy_col in recent.columns and recent[buy_col].any():
            # Son tetiklenme kaç bar önce
            bars_ago = int(lookback - 1 - recent[buy_col][::-1].idxmax()) if recent[buy_col].any() else lookback
            # Basit: son barda mı tetiklendi kontrol
            for i in range(len(recent) - 1, -1, -1):
                if recent.iloc[i].get(buy_col, False):
                    bars_ago = len(recent) - 1 - i
                    break
            buy_signals.append({'type': sig, 'bars_ago': bars_ago})
        if sell_col in recent.columns and recent[sell_col].any():
            for i in range(len(recent) - 1, -1, -1):
                if recent.iloc[i].get(sell_col, False):
                    bars_ago = len(recent) - 1 - i
                    break
            sell_signals.append({'type': sig, 'bars_ago': bars_ago})

    result['buy_signals'] = buy_signals
    result['sell_signals'] = sell_signals
    result['active_signals'] = buy_signals + sell_signals

    # En yakın sinyalin yaşı
    all_bars_ago = [s['bars_ago'] for s in result['active_signals']]
    result['days_since_signal'] = min(all_bars_ago) if all_bars_ago else 99

    # Yön belirleme
    buy_types = set(s['type'] for s in buy_signals)
    sell_types = set(s['type'] for s in sell_signals)

    has_buy = len(buy_signals) > 0
    has_sell = len(sell_signals) > 0

    if has_buy and not has_sell:
        result['direction'] = 'bullish'
    elif has_sell and not has_buy:
        result['direction'] = 'bearish'
    elif has_buy and has_sell:
        result['direction'] = 'conflict'
    else:
        # Sinyal yok — EMA dizilimine bak
        result['direction'] = result['ema_alignment'] if result['ema_alignment'] != 'mixed' else 'neutral'

    return result


def classify_1d_status(signals: Dict) -> str:
    """
    1D sinyal durumunu Ç3 formatına çevir.
    Returns: 'trend_alpha', 'trend_only', 'alpha_only', 'new_trend', 
             'ema_only', 'none', 'conflict'
    """
    buy_types = set(s['type'] for s in signals.get('buy_signals', []))
    sell_types = set(s['type'] for s in signals.get('sell_signals', []))
    direction = signals.get('direction', 'neutral')

    # Çelişki kontrolü
    if direction == 'conflict':
        return 'conflict'

    active = buy_types | sell_types

    if 'trend' in active and 'alpha' in active:
        return 'trend_alpha'
    if 'trend' in active:
        return 'trend_only'
    if 'alpha' in active:
        return 'alpha_only'
    if 'newtrend' in active:
        return 'new_trend'

    # Sinyal yok ama EMA dizilimi var mı?
    if signals.get('ema_alignment') in ('bullish', 'bearish'):
        return 'ema_only'

    return 'none'


def classify_4h_signal(signals: Dict, direction_1d: str) -> Optional[str]:
    """
    4H giriş sinyalini belirle. 1D yönüyle uyumlu olmalı.
    Returns: 'squeeze_buy', 'range_sell', etc. veya None
    """
    if direction_1d == 'conflict':
        return None

    # 1D yönüne göre sadece uyumlu sinyaller
    if direction_1d in ('bullish', 'neutral'):
        candidates = signals.get('buy_signals', [])
        suffix = '_buy'
    elif direction_1d == 'bearish':
        candidates = signals.get('sell_signals', [])
        suffix = '_sell'
    else:
        return None

    if not candidates:
        return None

    # Öncelik sırası: squeeze > hype > pullback > range
    priority = {'squeeze': 0, 'hype': 1, 'pullback': 2, 'range': 3}
    # Sadece 4H sinyalleri (trend/newtrend/alpha 1D'ye ait)
    entry_signals = [s for s in candidates if s['type'] in ('squeeze', 'hype', 'pullback', 'range')]

    if not entry_signals:
        return None

    entry_signals.sort(key=lambda s: (priority.get(s['type'], 99), s['bars_ago']))
    best = entry_signals[0]
    return f"{best['type']}{suffix}"


# ═══════════════════════════════════════════════════════════════
# VERİ ÇEKME
# ═══════════════════════════════════════════════════════════════

class DataFetcher:
    """Binance (kripto) ve yfinance (BIST/emtia/FX) veri çekici."""

    def __init__(self):
        self.exchange = ccxt.binance({'enableRateLimit': True})

    def fetch_crypto(self, symbol: str, timeframe: str, limit: int = 420) -> Optional[pd.DataFrame]:
        """Binance'den OHLCV çek."""
        try:
            print(f"  [{datetime.now().strftime('%H:%M:%S')}] Kripto: {symbol} ({timeframe}, {limit} bar)...")
            raw = self.exchange.fetch_ohlcv(symbol, timeframe, limit=limit)
            df = pd.DataFrame(raw, columns=['timestamp', 'open', 'high', 'low', 'close', 'volume'])
            df['timestamp'] = pd.to_datetime(df['timestamp'], unit='ms')
            df = df.astype({'open': float, 'high': float, 'low': float, 'close': float, 'volume': float})
            print(f"  [{datetime.now().strftime('%H:%M:%S')}] OK — {len(df)} bar")
            return df
        except Exception as e:
            print(f"  Kripto veri hatası ({symbol}): {e}")
            return None

    def fetch_yfinance(self, ticker: str, period: str = '2y', interval: str = '1d') -> Optional[pd.DataFrame]:
        """yfinance'den veri çek (BIST, emtia, FX)."""
        try:
            print(f"  [{datetime.now().strftime('%H:%M:%S')}] yfinance: {ticker} ({interval})...")
            t = yf.Ticker(ticker)
            df = t.history(period=period, interval=interval)
            if df is None or df.empty:
                raise ValueError("Boş veri")
            df = df.reset_index()
            df.columns = [c.lower() for c in df.columns]
            # yfinance sütun isimlerini normalize et
            if 'date' in df.columns:
                df = df.rename(columns={'date': 'timestamp'})
            if 'datetime' in df.columns:
                df = df.rename(columns={'datetime': 'timestamp'})
            # Gerekli sütunların varlığını kontrol et
            for col in ['open', 'high', 'low', 'close', 'volume']:
                if col not in df.columns:
                    df[col] = 0.0
            print(f"  [{datetime.now().strftime('%H:%M:%S')}] OK — {len(df)} bar")
            return df
        except Exception as e:
            print(f"  yfinance veri hatası ({ticker}): {e}")
            return None


# ═══════════════════════════════════════════════════════════════
# ANA ANALİZ SINIFI
# ═══════════════════════════════════════════════════════════════

# BIST sembol → yfinance ticker eşleme
BIST_TICKER_MAP = {
    'GARAN': 'GARAN.IS', 'AKBNK': 'AKBNK.IS', 'YKBNK': 'YKBNK.IS',
    'ISCTR': 'ISCTR.IS', 'HALKB': 'HALKB.IS',
    'EREGL': 'EREGL.IS', 'FROTO': 'FROTO.IS', 'ARCLK': 'ARCLK.IS',
    'TOASO': 'TOASO.IS', 'KRDMD': 'KRDMD.IS',
    'KCHOL': 'KCHOL.IS', 'SAHOL': 'SAHOL.IS', 'SISE': 'SISE.IS',
    'TKFEN': 'TKFEN.IS', 'DOHOL': 'DOHOL.IS',
    'THYAO': 'THYAO.IS', 'PGSUS': 'PGSUS.IS', 'TAVHL': 'TAVHL.IS',
    'TCELL': 'TCELL.IS', 'TTKOM': 'TTKOM.IS',
    'TUPRS': 'TUPRS.IS', 'SODA': 'SODA.IS', 'PETKM': 'PETKM.IS',
    'GUBRF': 'GUBRF.IS', 'ALKIM': 'ALKIM.IS',
    'BIMAS': 'BIMAS.IS', 'ULKER': 'ULKER.IS', 'CCOLA': 'CCOLA.IS',
    'TATGD': 'TATGD.IS', 'BANVT': 'BANVT.IS',
}

# Sektör eşleme
SECTOR_MAP = {
    'GARAN': 'XBANK', 'AKBNK': 'XBANK', 'YKBNK': 'XBANK', 'ISCTR': 'XBANK', 'HALKB': 'XBANK',
    'EREGL': 'XUSIN', 'FROTO': 'XUSIN', 'ARCLK': 'XUSIN', 'TOASO': 'XUSIN', 'KRDMD': 'XUSIN',
    'KCHOL': 'XHOLD', 'SAHOL': 'XHOLD', 'SISE': 'XHOLD', 'TKFEN': 'XHOLD', 'DOHOL': 'XHOLD',
    'THYAO': 'XULAS', 'PGSUS': 'XULAS', 'TAVHL': 'XULAS', 'TCELL': 'XULAS', 'TTKOM': 'XULAS',
    'TUPRS': 'XKMYA', 'SODA': 'XKMYA', 'PETKM': 'XKMYA', 'GUBRF': 'XKMYA', 'ALKIM': 'XKMYA',
    'BIMAS': 'XGIDA', 'ULKER': 'XGIDA', 'CCOLA': 'XGIDA', 'TATGD': 'XGIDA', 'BANVT': 'XGIDA',
}

# Kripto sembolleri
CRYPTO_SYMBOLS = ['BTC/USDT', 'ETH/USDT']

# Emtia/FX
COMMODITY_FX = {
    'XAU/USD': 'GC=F',
    'XAG/USD': 'SI=F',
    'USD/TRY': 'USDTRY=X',
    'EUR/USD': 'EURUSD=X',
}


class MultiTimeframeAnalyzer:
    """MTF analiz: 1D (yön) + 4H (giriş)."""

    def __init__(self):
        self.fetcher = DataFetcher()

    def _is_crypto(self, symbol: str) -> bool:
        return 'USDT' in symbol or 'BTC' in symbol or 'ETH' in symbol

    def _is_bist(self, symbol: str) -> bool:
        return symbol in BIST_TICKER_MAP or symbol.endswith('.IS')

    def _get_asset_class(self, symbol: str) -> str:
        if self._is_crypto(symbol): return 'crypto'
        if symbol in COMMODITY_FX or 'XAU' in symbol or 'XAG' in symbol: return 'metal'
        if 'USD' in symbol or 'EUR' in symbol: return 'fx_bond'
        return 'bist'

    def analyze(self, symbol: str) -> Optional[Dict[str, Any]]:
        """
        Tek enstrüman tam analiz: 1D + 4H.
        """
        try:
            # ─── Veri çek ───
            if self._is_crypto(symbol):
                df_1d = self.fetcher.fetch_crypto(symbol, '1d', 420)
                df_4h = self.fetcher.fetch_crypto(symbol, '4h', 420)
            elif self._is_bist(symbol):
                ticker = BIST_TICKER_MAP.get(symbol, symbol)
                df_1d = self.fetcher.fetch_yfinance(ticker, period='2y', interval='1d')
                df_4h = self.fetcher.fetch_yfinance(ticker, period='60d', interval='1h')
                # yfinance 4h desteklemiyor, 1h çekip resample yapıyoruz
                if df_4h is not None and len(df_4h) > 0:
                    df_4h = self._resample_to_4h(df_4h)
            elif symbol in COMMODITY_FX:
                ticker = COMMODITY_FX[symbol]
                df_1d = self.fetcher.fetch_yfinance(ticker, period='2y', interval='1d')
                df_4h = self.fetcher.fetch_yfinance(ticker, period='60d', interval='1h')
                if df_4h is not None and len(df_4h) > 0:
                    df_4h = self._resample_to_4h(df_4h)
            else:
                print(f"  Bilinmeyen sembol: {symbol}")
                return None

            if df_1d is None or len(df_1d) < 210:
                print(f"  {symbol}: 1D veri yetersiz ({len(df_1d) if df_1d is not None else 0} bar)")
                return None

            # ─── İndikatörler + sinyaller ───
            df_1d = calc_indicators(df_1d)
            df_1d = calc_signals(df_1d)
            signals_1d = extract_signal_status(df_1d, lookback=5)

            signals_4h = {'direction': 'neutral', 'buy_signals': [], 'sell_signals': [], 'active_signals': []}
            if df_4h is not None and len(df_4h) >= 100:
                df_4h = calc_indicators(df_4h)
                df_4h = calc_signals(df_4h)
                signals_4h = extract_signal_status(df_4h, lookback=3)

            # ─── MTF birleştirme ───
            direction_1d = signals_1d.get('direction', 'neutral')
            signal_status_1d = classify_1d_status(signals_1d)
            signal_4h = classify_4h_signal(signals_4h, direction_1d)

            # ─── Sektör bilgisi ───
            sector = SECTOR_MAP.get(symbol.replace('.IS', ''), None)

            return {
                'symbol': symbol,
                'symbol_key': symbol.replace('/', '_').replace('.', '_'),
                'asset_class': self._get_asset_class(symbol),
                'sector': sector,
                'price': signals_1d.get('price', 0),
                'atr': signals_1d.get('atr', 0),
                'atr_pct': signals_1d.get('atr_pct', 0),
                'direction_1d': direction_1d,
                'signal_status_1d': signal_status_1d,
                'signal_4h': signal_4h,
                'adx': signals_1d.get('adx', 20),
                'adx_prev': signals_1d.get('adx_prev', 20),
                'rsi': signals_1d.get('rsi', 50),
                'rsi_ma5': signals_1d.get('rsi_ma5', 50),
                'ema_alignment': signals_1d.get('ema_alignment', 'mixed'),
                'price_vs_ema200_pct': signals_1d.get('price_vs_ema200_pct', 0),
                'sqz_val': signals_1d.get('sqz_val', 0),
                'sqz_on': signals_1d.get('sqz_on', False),
                'vol_ratio': signals_1d.get('vol_ratio', 1),
                'days_since_signal': signals_1d.get('days_since_signal', 99),
                'signals_1d': signals_1d,
                'signals_4h': signals_4h,
                'updated': datetime.now().isoformat(),
            }

        except Exception as e:
            print(f"  {symbol} analiz hatası: {e}")
            import traceback
            traceback.print_exc()
            return None

    def _resample_to_4h(self, df_1h: pd.DataFrame) -> pd.DataFrame:
        """1H veriyi 4H'ye resample et."""
        try:
            if 'timestamp' in df_1h.columns:
                df_1h = df_1h.set_index('timestamp')
            df_4h = df_1h.resample('4h').agg({
                'open': 'first', 'high': 'max', 'low': 'min',
                'close': 'last', 'volume': 'sum'
            }).dropna()
            df_4h = df_4h.reset_index()
            df_4h = df_4h.rename(columns={'index': 'timestamp'})
            return df_4h
        except Exception as e:
            print(f"  Resample hatası: {e}")
            return df_1h

    def scan_all(self, symbols: List[str]) -> Dict[str, Dict]:
        """Tüm sembolleri tara."""
        results = {}
        for sym in symbols:
            try:
                data = self.analyze(sym)
                if data:
                    results[data['symbol_key']] = data
                    status = data['signal_status_1d']
                    sig4h = data['signal_4h'] or 'BEKLE'
                    print(f"  [OK] {sym:12s} | 1D: {status:12s} | 4H: {sig4h:15s} | "
                          f"ADX: {data['adx']:5.1f} | ATR%: {data['atr_pct']:5.2f}")
            except Exception as e:
                print(f"  [HATA] {sym}: {e}")
        return results
