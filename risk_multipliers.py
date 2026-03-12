"""
RiskPlatform v2.0 — Çarpan Tabloları Motoru
=============================================
Portföy: $100,000 | Max Aylık Risk: %6 | İşlem Risk: %0.5-%2.0
Formül: risk% = clamp(BASE × Ç1 × Ç2 × Ç3 × Ç5 × Ç6 × Ç7, 0.5, 2.0)
BASE = Ç4 (Kelly Criterion)

Çarpanlar:
  Ç1 = Korelasyon (portföy çeşitlendirme)
  Ç2 = Makro yönlendirme (VIX/VVIX/DXY + Term Structure)
  Ç3 = 1D teknik güç (sinyal + ADX tükenme)
  Ç4 = Kelly Criterion (dinamik BASE)
  Ç5 = Drawdown protokolü (psikolojik koruma)
  Ç6 = Likidite faktörü (hacim/spread)
  Ç7 = Zaman çürümesi (pozisyon yaşı)
"""

import numpy as np
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Tuple, Any


# ═══════════════════════════════════════════════════════════════
# SABİTLER & KONFİGÜRASYON
# ═══════════════════════════════════════════════════════════════

PORTFOLIO_SIZE = 100_000
MAX_MONTHLY_RISK_PCT = 6.0
MIN_TRADE_RISK_PCT = 0.5
MAX_TRADE_RISK_PCT = 2.0

# Korelasyon grupları
GRUP_ENSTRUMAN = {
    'G1': ['BTC/USDT', 'ETH/USDT', 'BTC', 'ETH'],
    'G2': ['GARAN', 'AKBNK', 'YKBNK', 'ISCTR', 'HALKB',
           'GARAN.IS', 'AKBNK.IS', 'YKBNK.IS', 'ISCTR.IS', 'HALKB.IS'],
    'G3': ['EREGL', 'FROTO', 'ARCLK', 'TOASO', 'KRDMD',
           'EREGL.IS', 'FROTO.IS', 'ARCLK.IS', 'TOASO.IS', 'KRDMD.IS'],
    'G4': ['KCHOL', 'SAHOL', 'SISE', 'TKFEN', 'DOHOL',
           'KCHOL.IS', 'SAHOL.IS', 'SISE.IS', 'TKFEN.IS', 'DOHOL.IS'],
    'G5': ['THYAO', 'PGSUS', 'TAVHL', 'TCELL', 'TTKOM',
           'THYAO.IS', 'PGSUS.IS', 'TAVHL.IS', 'TCELL.IS', 'TTKOM.IS'],
    'G6': ['TUPRS', 'SODA', 'PETKM', 'GUBRF', 'ALKIM',
           'TUPRS.IS', 'SODA.IS', 'PETKM.IS', 'GUBRF.IS', 'ALKIM.IS'],
    'G7': ['BIMAS', 'ULKER', 'CCOLA', 'TATGD', 'BANVT',
           'BIMAS.IS', 'ULKER.IS', 'CCOLA.IS', 'TATGD.IS', 'BANVT.IS'],
    'G8': ['XAU/USD', 'XAG/USD', 'XAUUSD', 'XAGUSD', 'GC=F', 'SI=F'],
    'G9': ['USD/TRY', 'EUR/USD', 'USDTRY', 'EURUSD', 'EURUSD=X'],
}

# Statik korelasyon matrisi (simetrik — sadece üst üçgen tanımlı)
_CORR_RAW = {
    ('G1', 'G1'): 1.00, ('G1', 'G2'): 0.25, ('G1', 'G3'): 0.20,
    ('G1', 'G4'): 0.25, ('G1', 'G5'): 0.15, ('G1', 'G6'): 0.30,
    ('G1', 'G7'): 0.10, ('G1', 'G8'): 0.35, ('G1', 'G9'): 0.40,
    ('G2', 'G2'): 1.00, ('G2', 'G3'): 0.70, ('G2', 'G4'): 0.80,
    ('G2', 'G5'): 0.50, ('G2', 'G6'): 0.45, ('G2', 'G7'): 0.35,
    ('G2', 'G8'): 0.10, ('G2', 'G9'): 0.55,
    ('G3', 'G3'): 1.00, ('G3', 'G4'): 0.65, ('G3', 'G5'): 0.45,
    ('G3', 'G6'): 0.55, ('G3', 'G7'): 0.40, ('G3', 'G8'): 0.15,
    ('G3', 'G9'): 0.50,
    ('G4', 'G4'): 1.00, ('G4', 'G5'): 0.55, ('G4', 'G6'): 0.50,
    ('G4', 'G7'): 0.40, ('G4', 'G8'): 0.10, ('G4', 'G9'): 0.55,
    ('G5', 'G5'): 1.00, ('G5', 'G6'): 0.35, ('G5', 'G7'): 0.30,
    ('G5', 'G8'): 0.10, ('G5', 'G9'): 0.45,
    ('G6', 'G6'): 1.00, ('G6', 'G7'): 0.30, ('G6', 'G8'): 0.25,
    ('G6', 'G9'): 0.50,
    ('G7', 'G7'): 1.00, ('G7', 'G8'): 0.05, ('G7', 'G9'): 0.40,
    ('G8', 'G8'): 1.00, ('G8', 'G9'): 0.60,
    ('G9', 'G9'): 1.00,
}

# Simetrik matris oluştur
KORELASYON_MATRISI = {}
for (g1, g2), val in _CORR_RAW.items():
    KORELASYON_MATRISI[(g1, g2)] = val
    KORELASYON_MATRISI[(g2, g1)] = val

# Varlık sınıfı eşleme
ASSET_CLASS_MAP = {
    'G1': 'crypto', 'G2': 'bist', 'G3': 'bist', 'G4': 'bist',
    'G5': 'bist', 'G6': 'bist', 'G7': 'bist',
    'G8': 'metal', 'G9': 'fx_bond',
}

# Ç2 Makro çarpan tabloları — tam matris
# Yapı: MACRO_TABLE[regime][(vvix_dir, dxy_state)] = {asset_class: multiplier}
MACRO_TABLE = {
    'LOW': {
        ('STABLE', 'WEAK'):   {'crypto': 1.4, 'bist': 1.2, 'metal': 1.1, 'fx_bond': 0.7},
        ('STABLE', 'STRONG'): {'crypto': 1.0, 'bist': 1.1, 'metal': 0.8, 'fx_bond': 1.0},
        ('STABLE', 'FLAT'):   {'crypto': 1.2, 'bist': 1.2, 'metal': 1.0, 'fx_bond': 0.8},
        ('RISING', 'WEAK'):   {'crypto': 1.1, 'bist': 1.0, 'metal': 1.2, 'fx_bond': 0.8},
        ('RISING', 'STRONG'): {'crypto': 0.8, 'bist': 0.9, 'metal': 0.9, 'fx_bond': 1.1},
        ('RISING', 'FLAT'):   {'crypto': 0.9, 'bist': 1.0, 'metal': 1.1, 'fx_bond': 0.9},
        ('FALLING', 'WEAK'):  {'crypto': 1.4, 'bist': 1.3, 'metal': 1.0, 'fx_bond': 0.7},
        ('FALLING', 'STRONG'):{'crypto': 1.1, 'bist': 1.2, 'metal': 0.8, 'fx_bond': 0.9},
        ('FALLING', 'FLAT'):  {'crypto': 1.2, 'bist': 1.2, 'metal': 0.9, 'fx_bond': 0.8},
    },
    'NORMAL': {
        ('STABLE', 'WEAK'):   {'crypto': 1.3, 'bist': 1.2, 'metal': 1.1, 'fx_bond': 0.7},
        ('STABLE', 'STRONG'): {'crypto': 0.9, 'bist': 1.0, 'metal': 0.8, 'fx_bond': 1.0},
        ('STABLE', 'FLAT'):   {'crypto': 1.1, 'bist': 1.1, 'metal': 1.0, 'fx_bond': 0.8},
        ('RISING', 'WEAK'):   {'crypto': 0.9, 'bist': 0.9, 'metal': 1.3, 'fx_bond': 0.8},
        ('RISING', 'STRONG'): {'crypto': 0.7, 'bist': 0.8, 'metal': 1.0, 'fx_bond': 1.1},
        ('RISING', 'FLAT'):   {'crypto': 0.8, 'bist': 0.9, 'metal': 1.1, 'fx_bond': 1.0},
        ('FALLING', 'WEAK'):  {'crypto': 1.4, 'bist': 1.3, 'metal': 1.0, 'fx_bond': 0.7},
        ('FALLING', 'STRONG'):{'crypto': 1.0, 'bist': 1.1, 'metal': 0.8, 'fx_bond': 0.9},
        ('FALLING', 'FLAT'):  {'crypto': 1.2, 'bist': 1.2, 'metal': 0.9, 'fx_bond': 0.8},
    },
    'ELEVATED': {
        ('STABLE', 'WEAK'):   {'crypto': 0.7, 'bist': 0.7, 'metal': 1.4, 'fx_bond': 0.9},
        ('STABLE', 'STRONG'): {'crypto': 0.5, 'bist': 0.6, 'metal': 1.1, 'fx_bond': 1.2},
        ('STABLE', 'FLAT'):   {'crypto': 0.6, 'bist': 0.7, 'metal': 1.3, 'fx_bond': 1.0},
        ('RISING', 'WEAK'):   {'crypto': 0.4, 'bist': 0.5, 'metal': 1.4, 'fx_bond': 1.0},
        ('RISING', 'STRONG'): {'crypto': 0.3, 'bist': 0.4, 'metal': 1.1, 'fx_bond': 1.3},
        ('RISING', 'FLAT'):   {'crypto': 0.4, 'bist': 0.5, 'metal': 1.3, 'fx_bond': 1.1},
        ('FALLING', 'WEAK'):  {'crypto': 1.0, 'bist': 1.0, 'metal': 1.2, 'fx_bond': 0.8},
        ('FALLING', 'STRONG'):{'crypto': 0.7, 'bist': 0.8, 'metal': 1.0, 'fx_bond': 1.0},
        ('FALLING', 'FLAT'):  {'crypto': 0.8, 'bist': 0.9, 'metal': 1.1, 'fx_bond': 0.9},
    },
    'CRISIS': {
        ('STABLE', 'WEAK'):   {'crypto': 0.0, 'bist': 0.3, 'metal': 1.4, 'fx_bond': 1.3},
        ('STABLE', 'STRONG'): {'crypto': 0.0, 'bist': 0.3, 'metal': 1.4, 'fx_bond': 1.3},
        ('STABLE', 'FLAT'):   {'crypto': 0.0, 'bist': 0.3, 'metal': 1.4, 'fx_bond': 1.3},
        ('RISING', 'WEAK'):   {'crypto': 0.0, 'bist': 0.0, 'metal': 1.3, 'fx_bond': 1.4},
        ('RISING', 'STRONG'): {'crypto': 0.0, 'bist': 0.0, 'metal': 1.3, 'fx_bond': 1.4},
        ('RISING', 'FLAT'):   {'crypto': 0.0, 'bist': 0.0, 'metal': 1.3, 'fx_bond': 1.4},
        ('FALLING', 'WEAK'):  {'crypto': 0.5, 'bist': 0.6, 'metal': 1.3, 'fx_bond': 1.0},
        ('FALLING', 'STRONG'):{'crypto': 0.3, 'bist': 0.4, 'metal': 1.1, 'fx_bond': 1.2},
        ('FALLING', 'FLAT'):  {'crypto': 0.4, 'bist': 0.5, 'metal': 1.2, 'fx_bond': 1.1},
    },
}

# VIX Term Structure düzeltmeleri
TERM_STRUCTURE_ADJ = {
    'CRISIS': {
        ('BACKWARDATION', 'FALLING'): {'crypto': 1.2, 'bist': 1.2, 'metal': 1.0, 'fx_bond': 1.0},
        ('BACKWARDATION', 'STABLE'):  {'crypto': 1.1, 'bist': 1.1, 'metal': 1.0, 'fx_bond': 1.0},
        ('BACKWARDATION', 'RISING'):  {'crypto': 1.0, 'bist': 1.0, 'metal': 1.0, 'fx_bond': 1.0},
        ('CONTANGO', 'RISING'):       {'crypto': 0.8, 'bist': 0.8, 'metal': 1.0, 'fx_bond': 1.0},
        ('CONTANGO', 'STABLE'):       {'crypto': 0.9, 'bist': 0.9, 'metal': 1.0, 'fx_bond': 1.0},
        ('STEEP_CONTANGO', 'RISING'): {'crypto': 0.7, 'bist': 0.7, 'metal': 1.0, 'fx_bond': 1.0},
    },
    'ELEVATED': {
        ('STEEP_CONTANGO', 'RISING'): {'crypto': 0.85, 'bist': 0.85, 'metal': 1.0, 'fx_bond': 1.0},
        ('STEEP_CONTANGO', 'STABLE'): {'crypto': 0.90, 'bist': 0.90, 'metal': 1.0, 'fx_bond': 1.0},
        ('BACKWARDATION', 'FALLING'): {'crypto': 1.15, 'bist': 1.15, 'metal': 1.0, 'fx_bond': 1.0},
        ('BACKWARDATION', 'STABLE'):  {'crypto': 1.10, 'bist': 1.10, 'metal': 1.0, 'fx_bond': 1.0},
    },
}


# ═══════════════════════════════════════════════════════════════
# YARDIMCI FONKSİYONLAR
# ═══════════════════════════════════════════════════════════════

def _safe_float(val, default=0.0):
    """Güvenli float dönüşümü."""
    if val is None:
        return default
    try:
        return float(val)
    except (ValueError, TypeError):
        return default


def _clamp(value, lo, hi):
    """Değeri [lo, hi] aralığına sıkıştır."""
    return max(lo, min(hi, value))


def find_group(symbol: str) -> Optional[str]:
    """Enstrümanın korelasyon grubunu bul."""
    # Normalize: .IS ekini kaldır, / kaldır
    sym_clean = symbol.upper().replace('.IS', '').replace('/', '')
    for group, members in GRUP_ENSTRUMAN.items():
        for m in members:
            if m.upper().replace('.IS', '').replace('/', '') == sym_clean:
                return group
    return None


def get_asset_class(symbol: str) -> str:
    """Enstrümanın varlık sınıfını belirle."""
    group = find_group(symbol)
    if group:
        return ASSET_CLASS_MAP.get(group, 'bist')
    # Fallback: sembol adından tahmin
    sym = symbol.upper()
    if 'USDT' in sym or 'BTC' in sym or 'ETH' in sym:
        return 'crypto'
    if 'XAU' in sym or 'XAG' in sym or 'GC=' in sym or 'SI=' in sym:
        return 'metal'
    if 'USD' in sym or 'EUR' in sym or 'GBP' in sym or 'TRY' in sym:
        return 'fx_bond'
    return 'bist'


# ═══════════════════════════════════════════════════════════════
# Ç4 — KELLY CRITERION (DİNAMİK BASE)
# ═══════════════════════════════════════════════════════════════

def calc_kelly_base(trades: List[Dict]) -> Dict[str, Any]:
    """
    Quarter Kelly hesaplaması.
    
    Args:
        trades: [{'pnl': float, 'result': 'win'|'loss', 'signal_type': str}, ...]
    
    Returns:
        {'base': float, 'win_rate': float, 'avg_rr': float, 
         'full_kelly': float, 'data_quality': str, 'trade_count': int}
    """
    result = {
        'base': 1.0, 'win_rate': 0.0, 'avg_rr': 0.0,
        'full_kelly': 0.0, 'data_quality': 'INSUFFICIENT', 'trade_count': 0
    }
    
    if not trades or len(trades) < 10:
        return result
    
    result['trade_count'] = len(trades)
    
    # Son 50 işlem (veya mevcut tüm işlemler)
    recent = trades[-50:] if len(trades) >= 50 else trades
    
    wins = [t for t in recent if t.get('result') == 'win']
    losses = [t for t in recent if t.get('result') == 'loss']
    
    if not wins or not losses:
        return result
    
    win_rate = len(wins) / len(recent)
    avg_win = np.mean([abs(_safe_float(t.get('pnl'), 0)) for t in wins])
    avg_loss = np.mean([abs(_safe_float(t.get('pnl'), 1)) for t in losses])
    avg_rr = avg_win / avg_loss if avg_loss > 0 else 1.0
    
    result['win_rate'] = round(win_rate, 4)
    result['avg_rr'] = round(avg_rr, 2)
    
    # Yetersiz performans kontrolü
    if win_rate < 0.40 or avg_rr < 1.0:
        result['base'] = 0.5
        result['data_quality'] = 'WEAK' if len(trades) >= 50 else 'INSUFFICIENT'
        return result
    
    # Kelly: f* = (p*b - q) / b
    p, q, b = win_rate, 1 - win_rate, avg_rr
    full_kelly = (p * b - q) / b
    quarter_kelly = full_kelly * 0.25
    
    result['full_kelly'] = round(full_kelly * 100, 2)
    
    # Veri güvenirliği çarpanı
    if len(trades) < 50:
        confidence = 0.3  # Çok az veri
        result['data_quality'] = 'INSUFFICIENT'
    elif len(trades) < 100:
        confidence = 0.5  # Yarı güven
        result['data_quality'] = 'PARTIAL'
    else:
        confidence = 1.0  # Tam güven
        result['data_quality'] = 'FULL'
    
    # BASE = quarter_kelly * 10 * confidence
    # Referans: %6.9 quarter kelly → BASE 0.69, %8.1 → BASE 0.81
    base = quarter_kelly * 10 * confidence
    result['base'] = round(_clamp(base, 0.5, 2.0), 2)
    
    return result


# ═══════════════════════════════════════════════════════════════
# Ç1 — KORELASYON ÇARPANI
# ═══════════════════════════════════════════════════════════════

def calc_correlation_multiplier(
    candidate_symbol: str,
    open_positions: List[Dict],
    max_same_group: int = 2
) -> Dict[str, Any]:
    """
    Portföy çeşitlendirme çarpanı.
    
    Args:
        candidate_symbol: Aday enstrüman
        open_positions: [{'symbol': str, 'risk_pct': float}, ...]
        max_same_group: Aynı grupta izin verilen max pozisyon
    
    Returns:
        {'c1': float, 'candidate_group': str, 'max_correlation': float,
         'reason': str, 'blocked': bool}
    """
    result = {
        'c1': 1.0, 'candidate_group': None, 'max_correlation': 0.0,
        'reason': '', 'blocked': False
    }
    
    cand_group = find_group(candidate_symbol)
    result['candidate_group'] = cand_group
    
    if cand_group is None:
        result['reason'] = 'Grup bulunamadı — nötr'
        return result
    
    # Açık pozisyon yoksa
    if not open_positions:
        result['reason'] = 'Açık pozisyon yok — nötr'
        return result
    
    # Grup sayılarını hesapla
    group_counts = {}
    for pos in open_positions:
        pg = find_group(pos.get('symbol', ''))
        if pg:
            group_counts[pg] = group_counts.get(pg, 0) + 1
    
    # Hard block: aynı grupta max_same_group'a ulaşıldıysa
    if group_counts.get(cand_group, 0) >= max_same_group:
        result['c1'] = 0.0
        result['blocked'] = True
        result['reason'] = f'{cand_group} grubunda {max_same_group}+ pozisyon — HARD BLOCK'
        return result
    
    # Max korelasyonu bul
    max_corr = 0.0
    max_corr_group = None
    for pos in open_positions:
        pg = find_group(pos.get('symbol', ''))
        if pg is None:
            continue
        pair = (cand_group, pg)
        corr = KORELASYON_MATRISI.get(pair, 0.3)  # Varsayılan orta korelasyon
        if corr > max_corr:
            max_corr = corr
            max_corr_group = pg
    
    result['max_correlation'] = round(max_corr, 2)
    
    # Korelasyona göre çarpan
    if max_corr >= 0.99:
        c1 = 0.4
        reason = f'Aynı grup ({max_corr_group}) — çok kıs'
    elif max_corr >= 0.70:
        c1 = 0.5
        reason = f'Çok yüksek korelasyon ({max_corr_group}: {max_corr:.2f})'
    elif max_corr >= 0.50:
        c1 = 0.7
        reason = f'Yüksek korelasyon ({max_corr_group}: {max_corr:.2f})'
    elif max_corr >= 0.30:
        c1 = 0.9
        reason = f'Orta korelasyon ({max_corr_group}: {max_corr:.2f}) — makul'
    elif max_corr >= 0.15:
        c1 = 1.1
        reason = f'Düşük korelasyon ({max_corr_group}: {max_corr:.2f}) — iyi çeşitlendirme'
    else:
        c1 = 1.3
        reason = f'Çok düşük korelasyon ({max_corr_group}: {max_corr:.2f}) — mükemmel'
    
    result['c1'] = c1
    result['reason'] = reason
    return result


# ═══════════════════════════════════════════════════════════════
# Ç2 — MAKRO YÖNLENDİRME ÇARPANI
# ═══════════════════════════════════════════════════════════════

def get_vix_regime(vix: float) -> str:
    """VIX rejimini belirle."""
    vix = _safe_float(vix)
    if vix < 15: return 'LOW'
    if vix < 20: return 'NORMAL'
    if vix < 30: return 'ELEVATED'
    return 'CRISIS'


def get_vvix_direction(vvix_change_5d: float) -> str:
    """VVIX 5 günlük yön."""
    chg = _safe_float(vvix_change_5d)
    if chg > 0.15: return 'RISING'
    if chg < -0.15: return 'FALLING'
    return 'STABLE'


def get_dxy_state(dxy_trend: str) -> str:
    """DXY durumu normalize."""
    if dxy_trend in ('up', 'UP', 'STRONG', 'yükseliyor'):
        return 'STRONG'
    if dxy_trend in ('down', 'DOWN', 'WEAK', 'düşüyor'):
        return 'WEAK'
    return 'FLAT'


def get_vix_term_structure(vix_spot: float, vix_futures_m1: float) -> str:
    """VIX term structure."""
    spot = _safe_float(vix_spot)
    fut = _safe_float(vix_futures_m1)
    if spot <= 0 or fut <= 0:
        return 'CONTANGO'  # Varsayılan
    spread_pct = (fut - spot) / spot * 100
    if spread_pct > 5: return 'STEEP_CONTANGO'
    if spread_pct > 0: return 'CONTANGO'
    if spread_pct > -5: return 'FLAT'
    return 'BACKWARDATION'


def calc_macro_multiplier(
    vix: float,
    vvix_change_5d: float,
    dxy_trend: str,
    asset_class: str,
    vix_spot: float = None,
    vix_futures_m1: float = None,
    us10y_trend: str = None,
    xu100_trend: str = None,
    sector_trend: str = None,
    btc_dominance: float = None,
    candidate_symbol: str = None,
) -> Dict[str, Any]:
    """
    Makro yönlendirme çarpanı — VIX Term Structure dahil.
    
    Returns:
        {'c2': float, 'regime': str, 'vvix_dir': str, 'dxy_state': str,
         'term_structure': str, 'breakdown': dict}
    """
    regime = get_vix_regime(vix)
    vvix_dir = get_vvix_direction(vvix_change_5d)
    dxy_state = get_dxy_state(dxy_trend)
    
    # Term structure
    term_struct = 'CONTANGO'
    if vix_spot is not None and vix_futures_m1 is not None:
        term_struct = get_vix_term_structure(vix_spot, vix_futures_m1)
    
    result = {
        'c2': 1.0, 'regime': regime, 'vvix_dir': vvix_dir,
        'dxy_state': dxy_state, 'term_structure': term_struct,
        'breakdown': {}
    }
    
    # Ana tablodan çarpanı al
    regime_table = MACRO_TABLE.get(regime, {})
    key = (vvix_dir, dxy_state)
    row = regime_table.get(key)
    
    if row is None:
        # Fallback: en yakın eşleşmeyi dene (wildcard)
        for k, v in regime_table.items():
            if k[0] == vvix_dir:
                row = v
                break
        if row is None:
            row = {'crypto': 0.8, 'bist': 0.8, 'metal': 1.0, 'fx_bond': 0.9}
    
    base_mult = row.get(asset_class, 0.8)
    result['breakdown']['base_macro'] = base_mult
    
    # VIX Term Structure düzeltmesi
    ts_adj_regime = TERM_STRUCTURE_ADJ.get(regime, {})
    ts_key = (term_struct, vvix_dir)
    ts_row = ts_adj_regime.get(ts_key)
    ts_mult = 1.0
    if ts_row:
        ts_mult = ts_row.get(asset_class, 1.0)
    result['breakdown']['term_structure_adj'] = ts_mult
    
    # CRISIS soft allow: backwardation + falling ise minimum 0.3
    if regime == 'CRISIS' and term_struct == 'BACKWARDATION' and vvix_dir == 'FALLING':
        if asset_class in ('crypto', 'bist') and base_mult < 0.3:
            base_mult = 0.3
            result['breakdown']['crisis_soft_allow'] = True
    
    c2 = base_mult * ts_mult
    
    # US10Y bonusu: faiz düşüyorsa → safe haven'a para akıyor
    if us10y_trend in ('FALLING', 'down', 'DOWN') and regime in ('ELEVATED', 'CRISIS'):
        if asset_class == 'metal':
            c2 *= 1.1
            result['breakdown']['us10y_bonus'] = 1.1
    
    # BIST iç rotasyon (XU100 + sektör uyumu)
    if asset_class == 'bist' and c2 > 0:
        xu = (xu100_trend or '').upper()
        sec = (sector_trend or '').upper()
        bist_adj = 1.0
        if xu == 'UP' and sec == 'UP':
            bist_adj = 1.1
        elif xu == 'UP' and sec == 'DOWN':
            bist_adj = 0.8
        elif xu == 'DOWN' and sec == 'UP':
            bist_adj = 0.7
        elif xu == 'DOWN' and sec == 'DOWN':
            bist_adj = 0.5
        c2 *= bist_adj
        result['breakdown']['bist_rotation'] = bist_adj
    
    # BTC.D iç rotasyon (kripto)
    if asset_class == 'crypto' and c2 > 0 and btc_dominance is not None:
        sym = (candidate_symbol or '').upper()
        btcd = _safe_float(btc_dominance)
        if btcd > 55:
            btc_adj = 1.2 if 'BTC' in sym else 0.8
        elif btcd < 50:
            btc_adj = 0.8 if 'BTC' in sym else 1.3
        else:
            btc_adj = 1.0
        c2 *= btc_adj
        result['breakdown']['btcd_rotation'] = btc_adj
    
    result['c2'] = round(c2, 3)
    return result


# ═══════════════════════════════════════════════════════════════
# Ç3 — 1D TEKNİK ÇARPAN
# ═══════════════════════════════════════════════════════════════

# Sinyal durumu → temel çarpan
SIGNAL_BASE = {
    'trend_alpha':  1.4,   # Trend + AlphaTrend ikisi aktif
    'trend_only':   1.2,   # Sadece Trend Buy/Sell
    'alpha_only':   1.1,   # Sadece AlphaTrend
    'new_trend':    0.8,   # NewTrend (erken sinyal)
    'ema_only':     0.6,   # Sadece EMA dizilimi
    'none':         0.4,   # Sinyal yok
    'conflict':     0.0,   # 1D yön çelişkisi — hard block
}

# ADX seviyesi → çarpan
ADX_SCALE = [
    (15,  0.85),   # < 15: trend yok
    (20,  0.95),   # 15-20: zayıf
    (25,  1.00),   # 20-25: oluşuyor
    (30,  1.10),   # 25-30: optimal ✓
    (35,  1.10),   # 30-35: optimal ✓
    (40,  1.05),   # 35-40: güçlü ama olgun
    (45,  1.00),   # 40-45: nötr
    (50,  0.90),   # 45-50: tükenme riski
    (999, 0.80),   # > 50: yüksek tükenme
]


def _get_adx_mult(adx: float) -> float:
    """ADX değerine göre çarpan."""
    adx = _safe_float(adx, 20)
    for threshold, mult in ADX_SCALE:
        if adx < threshold:
            return mult
    return 0.80


def calc_technical_multiplier(
    signal_status: str,
    adx: float,
    adx_prev: float = None,
    days_since_signal: int = 0,
    price_vs_ema200_pct: float = 0.0,
) -> Dict[str, Any]:
    """
    1D teknik güç çarpanı.
    
    Args:
        signal_status: 'trend_alpha', 'trend_only', 'alpha_only', 
                       'new_trend', 'ema_only', 'none', 'conflict'
        adx: Güncel ADX değeri
        adx_prev: 5 bar önceki ADX (yön tespiti)
        days_since_signal: Sinyal kaç bar önce tetiklendi
        price_vs_ema200_pct: Fiyatın EMA200'den sapma yüzdesi
    
    Returns:
        {'c3': float, 'signal_base': float, 'adx_mult': float,
         'adx_dir_bonus': float, 'age_mult': float, 'ema_penalty': float,
         'blocked': bool}
    """
    result = {
        'c3': 0.0, 'signal_base': 0.0, 'adx_mult': 1.0,
        'adx_dir_bonus': 1.0, 'age_mult': 1.0, 'ema_penalty': 1.0,
        'blocked': False
    }
    
    # Sinyal temel çarpanı
    sig_base = SIGNAL_BASE.get(signal_status, 0.4)
    result['signal_base'] = sig_base
    
    if sig_base == 0.0:
        result['blocked'] = True
        return result
    
    # ADX skalası
    adx_val = _safe_float(adx, 20)
    adx_mult = _get_adx_mult(adx_val)
    result['adx_mult'] = adx_mult
    
    # ADX yön bonusu
    adx_dir = 1.0
    if adx_prev is not None:
        adx_p = _safe_float(adx_prev, adx_val)
        if adx_val > adx_p:
            adx_dir = 1.05  # Yükseliyor
        elif adx_val < adx_p:
            adx_dir = 0.95  # Düşüyor
    result['adx_dir_bonus'] = adx_dir
    
    # Sinyal yaşı
    days = max(0, days_since_signal)
    if days <= 3:
        age_mult = 1.0
    elif days <= 10:
        age_mult = 0.9
    else:
        age_mult = 0.7
    result['age_mult'] = age_mult
    
    # EMA200 mesafe cezası (mean reversion riski)
    ema_dist = abs(_safe_float(price_vs_ema200_pct))
    ema_penalty = 0.8 if ema_dist > 15 else 1.0
    result['ema_penalty'] = ema_penalty
    
    # Final
    c3 = sig_base * adx_mult * adx_dir * age_mult * ema_penalty
    result['c3'] = round(c3, 3)
    return result


# ═══════════════════════════════════════════════════════════════
# Ç5 — DRAWDOWN PROTOKOLÜ
# ═══════════════════════════════════════════════════════════════

# Drawdown % → çarpan
DD_SCALE = [
    (2,  1.00),   # <%2: normal
    (5,  0.85),   # %2-5: dikkatli
    (10, 0.65),   # %5-10: risk azalt
    (15, 0.45),   # %10-15: yüksek konviksiyon
    (999, 0.25),  # >%15: moratoryum
]

# Seri kayıp → çarpan
STREAK_SCALE = [
    (1, 1.00),
    (2, 0.85),
    (3, 0.65),
    (4, 0.45),
    (999, 0.25),
]


def calc_drawdown_multiplier(
    portfolio_value: float,
    peak_value: float,
    consecutive_losses: int = 0,
    recovery_days: int = 0,
) -> Dict[str, Any]:
    """
    Drawdown + seri kayıp çarpanı.
    
    Args:
        recovery_days: Moratoryumdan çıktıktan sonra geçen gün
                       (kademeli toparlanma için)
    
    Returns:
        {'c5': float, 'dd_pct': float, 'dd_mult': float, 
         'streak_mult': float, 'moratorium': dict, 'severity': str}
    """
    pv = _safe_float(portfolio_value, PORTFOLIO_SIZE)
    pk = _safe_float(peak_value, PORTFOLIO_SIZE)
    pk = max(pk, pv)  # Peak en az mevcut değer kadar
    
    dd_pct = (pk - pv) / pk * 100 if pk > 0 else 0.0
    
    # Drawdown bazlı çarpan
    dd_mult = 1.0
    for threshold, mult in DD_SCALE:
        if dd_pct < threshold:
            dd_mult = mult
            break
    
    # Seri kayıp bazlı çarpan
    streak = max(0, consecutive_losses)
    streak_mult = 1.0
    for threshold, mult in STREAK_SCALE:
        if streak <= threshold:
            streak_mult = mult
            break
    
    # Baskın olan (düşük olan) uygulanır
    c5 = min(dd_mult, streak_mult)
    
    # Toparlanma rampası (moratoryumdan çıkış)
    if recovery_days > 0 and recovery_days <= 6:
        if recovery_days <= 3:
            c5 = min(c5, 0.35)  # İlk 3 gün: temkinli
        else:
            c5 = min(c5, 0.45)  # 3-6 gün: normal
    
    # Severity
    if c5 >= 0.85:
        severity = 'NORMAL'
    elif c5 >= 0.65:
        severity = 'CAUTION'
    elif c5 >= 0.45:
        severity = 'WARNING'
    elif c5 > 0.25:
        severity = 'DANGER'
    else:
        severity = 'CRITICAL'
    
    # Moratoryum kontrolü
    moratorium = {'active': False, 'hours': 0, 'allowed_assets': []}
    if c5 <= 0.25:
        moratorium = {
            'active': True,
            'hours': 24 if dd_pct < 20 else 48 if dd_pct < 25 else 72,
            'allowed_assets': ['XAU/USD', 'XAG/USD'],
            'max_risk_pct': 0.5,
            'reason': f'DD {dd_pct:.1f}%, streak {streak}'
        }
    
    return {
        'c5': round(c5, 2),
        'dd_pct': round(dd_pct, 2),
        'dd_mult': dd_mult,
        'streak_mult': streak_mult,
        'consecutive_losses': streak,
        'moratorium': moratorium,
        'severity': severity,
    }


# ═══════════════════════════════════════════════════════════════
# Ç6 — LİKİDİTE FAKTÖRÜ
# ═══════════════════════════════════════════════════════════════

# Hacim ($) → skor
VOL_SCALE = [
    (50_000,      0.0),    # <$50K: hard block
    (100_000,     0.50),   # $50K-100K
    (500_000,     0.65),   # $100K-500K
    (1_000_000,   0.80),   # $500K-1M
    (5_000_000,   0.90),   # $1M-5M
    (10_000_000,  0.95),   # $5M-10M
    (float('inf'), 1.0),   # $10M+
]

# Spread % → skor
SPREAD_SCALE = [
    (0.05, 1.00),
    (0.10, 0.95),
    (0.20, 0.90),
    (0.30, 0.80),
    (0.50, 0.65),
    (float('inf'), 0.50),
]

# Pozisyon/Hacim oranı → skor
PV_SCALE = [
    (0.01, 1.00),
    (0.02, 0.90),
    (0.05, 0.70),
    (0.10, 0.50),
    (float('inf'), 0.0),  # >%10: hard block
]

# BIST saat düzeltmeleri
BIST_SESSION_ADJ = {
    'open':   0.70,   # 09:30-09:45
    'close':  0.80,   # 17:45-18:00
    'lunch':  0.90,   # 12:30-13:30
    'normal': 1.00,
}


def _scale_lookup(value: float, scale: list) -> float:
    """Ölçek tablosundan değer bul."""
    for threshold, score in scale:
        if value < threshold:
            return score
    return scale[-1][1]


def calc_liquidity_multiplier(
    symbol: str,
    avg_daily_volume_usd: float,
    spread_pct: float = 0.05,
    position_size_usd: float = 0.0,
    market_session: str = 'normal',
) -> Dict[str, Any]:
    """
    Likidite çarpanı.
    
    Returns:
        {'c6': float, 'vol_score': float, 'spread_score': float,
         'pv_score': float, 'session_adj': float, 'blocked': bool,
         'volume_anomaly': bool}
    """
    vol_usd = _safe_float(avg_daily_volume_usd, 0)
    spread = _safe_float(spread_pct, 0.1)
    pos_size = _safe_float(position_size_usd, 0)
    
    # Hacim skoru
    vol_score = _scale_lookup(vol_usd, VOL_SCALE)
    
    # Spread skoru
    spread_score = _scale_lookup(spread, SPREAD_SCALE)
    
    # PV oranı skoru
    pv_ratio = pos_size / vol_usd if vol_usd > 0 else 999
    pv_score = _scale_lookup(pv_ratio, PV_SCALE)
    
    # En düşük skor
    base_mult = min(vol_score, spread_score, pv_score)
    blocked = base_mult <= 0.0
    
    # BIST saat düzeltmesi
    session_adj = 1.0
    sym_upper = symbol.upper()
    is_bist = sym_upper.endswith('.IS') or find_group(symbol) in ('G2','G3','G4','G5','G6','G7')
    if is_bist:
        session_adj = BIST_SESSION_ADJ.get(market_session, 1.0)
    
    c6 = base_mult * session_adj if not blocked else 0.0
    
    return {
        'c6': round(c6, 2),
        'vol_score': vol_score,
        'spread_score': spread_score,
        'pv_score': pv_score,
        'pv_ratio': round(pv_ratio, 4) if pv_ratio < 100 else 999,
        'session_adj': session_adj,
        'blocked': blocked,
        'volume_anomaly': False,  # Anomali tespiti ayrı yapılır
    }


# ═══════════════════════════════════════════════════════════════
# Ç7 — ZAMAN ÇÜRÜMESİ
# ═══════════════════════════════════════════════════════════════

# Sinyal tipi → trade tipi eşleme (Kural 7d)
SIGNAL_TRADE_TYPE = {
    'squeeze': 'swing',
    'range': 'swing',
    'hype': 'swing',
    'pullback': 'swing',
    'trend': 'position',
    'alpha_trend': 'position',
    'new_trend': 'position',
}

TRADE_TYPE_PARAMS = {
    'swing':    {'threshold': 10, 'decay_rate': 0.04},
    'position': {'threshold': 25, 'decay_rate': 0.02},
}


def calc_time_decay_multiplier(
    days_open: int,
    pnl_pct: float = 0.0,
    adx_value: float = 20.0,
    adx_trending_up: bool = False,
    price_above_ema50: bool = True,
    alpha_trend_aligned: bool = True,
    signal_type: str = 'swing',
) -> Dict[str, Any]:
    """
    Zaman çürümesi çarpanı.
    
    Returns:
        {'c7': float, 'days_open': int, 'trade_type': str,
         'base_decay': float, 'pnl_adj': str, 'mature_trend': bool}
    """
    trade_type = SIGNAL_TRADE_TYPE.get(signal_type, signal_type)
    if trade_type not in TRADE_TYPE_PARAMS:
        trade_type = 'swing'
    
    params = TRADE_TYPE_PARAMS[trade_type]
    threshold = params['threshold']
    decay_rate = params['decay_rate']
    days = max(0, days_open)
    
    # Temel çürüme hesabı
    half_thresh = threshold * 0.5
    if days <= half_thresh:
        base_decay = 1.0
    elif days <= threshold:
        excess = days - half_thresh
        base_decay = max(0.60, 1.0 - excess * decay_rate * 0.5)
    else:
        pre_decay = half_thresh * decay_rate * 0.5
        excess = days - threshold
        base_decay = max(0.60, 1.0 - pre_decay - excess * decay_rate)
    
    # P&L ayarlaması
    pnl = _safe_float(pnl_pct)
    pnl_adj = 'normal'
    if pnl > 5:
        # Karda: çürüme yavaşlar (Kural 7a)
        decay_amount = 1.0 - base_decay
        base_decay = 1.0 - decay_amount * 0.5
        pnl_adj = 'slow (karda >%5)'
    elif pnl < -3:
        # Zararda: çürüme hızlanır (Kural 7b)
        decay_amount = 1.0 - base_decay
        base_decay = max(0.50, 1.0 - decay_amount * 1.5)
        pnl_adj = 'fast (zararda <%−3)'
    
    # Mature trend istisnası (Kural 7c)
    mature_trend = False
    adx_v = _safe_float(adx_value, 20)
    if (days >= 12 and adx_v > 25 and adx_trending_up 
            and price_above_ema50 and alpha_trend_aligned and pnl > 0):
        mature_trend = True
        base_decay = max(base_decay, 0.85)
    
    c7 = round(min(1.0, base_decay), 2)
    
    return {
        'c7': c7,
        'days_open': days,
        'trade_type': trade_type,
        'base_decay': round(base_decay, 3),
        'pnl_adj': pnl_adj,
        'mature_trend': mature_trend,
    }


# ═══════════════════════════════════════════════════════════════
# ANA HESAPLAMA — TÜM ÇARPANLARI BİRLEŞTİR
# ═══════════════════════════════════════════════════════════════

def calculate_position_risk(
    portfolio_value: float = PORTFOLIO_SIZE,
    peak_value: float = PORTFOLIO_SIZE,
    open_positions: List[Dict] = None,
    candidate: Dict = None,
    market_data: Dict = None,
    trades_history: List[Dict] = None,
) -> Dict[str, Any]:
    """
    Ana risk hesaplama fonksiyonu — 7 çarpanı birleştirir.
    
    Args:
        portfolio_value: Mevcut portföy değeri ($)
        peak_value: Portföy peak değeri ($)
        open_positions: [{'symbol': str, 'risk_pct': float, 'group': str, 
                         'days_open': int, 'pnl_pct': float}, ...]
        candidate: {
            'symbol': str,
            'signal_status': str,     # 'trend_alpha', 'conflict', etc.
            'signal_type': str,       # 'squeeze', 'trend', etc.
            'adx': float,
            'adx_prev': float,
            'days_since_signal': int,
            'price_vs_ema200_pct': float,
        }
        market_data: {
            'vix': float, 'vvix_change_5d': float, 'dxy_trend': str,
            'vix_spot': float, 'vix_futures_m1': float,
            'us10y_trend': str, 'xu100_trend': str, 'sector_trend': str,
            'btc_dominance': float,
            'avg_daily_volume_usd': float, 'spread_pct': float,
            'session': str,
        }
        trades_history: [{'pnl': float, 'result': 'win'|'loss'}, ...]
    
    Returns:
        Detaylı sonuç dict'i
    """
    open_positions = open_positions or []
    candidate = candidate or {}
    market_data = market_data or {}
    trades_history = trades_history or []
    
    # ─── ADIM 1: Kalan risk iştahı ───
    total_open_risk = sum(_safe_float(p.get('risk_pct')) for p in open_positions)
    remaining_risk = MAX_MONTHLY_RISK_PCT - total_open_risk
    
    if remaining_risk < MIN_TRADE_RISK_PCT:
        return {
            'status': 'BLOCK',
            'reason': f'Yetersiz kalan risk: %{remaining_risk:.2f} (min %{MIN_TRADE_RISK_PCT})',
            'remaining_risk_pct': round(remaining_risk, 2),
            'total_open_risk_pct': round(total_open_risk, 2),
        }
    
    # ─── ADIM 2: Ç4 — Kelly BASE ───
    kelly = calc_kelly_base(trades_history)
    base = kelly['base']
    
    # ─── ADIM 3: Ç5 — Drawdown (önce — moratoryum kontrolü) ───
    consecutive_losses = 0
    for t in reversed(trades_history):
        if t.get('result') == 'loss':
            consecutive_losses += 1
        else:
            break
    
    dd_result = calc_drawdown_multiplier(
        portfolio_value, peak_value, consecutive_losses
    )
    
    if dd_result['moratorium']['active']:
        allowed = dd_result['moratorium'].get('allowed_assets', [])
        sym = candidate.get('symbol', '')
        if sym not in allowed and get_asset_class(sym) != 'metal':
            return {
                'status': 'MORATORIUM',
                'reason': dd_result['moratorium']['reason'],
                'hours_remaining': dd_result['moratorium']['hours'],
                'allowed_assets': allowed,
                'max_risk_pct': dd_result['moratorium']['max_risk_pct'],
                'dd_pct': dd_result['dd_pct'],
                'severity': dd_result['severity'],
            }
    
    # ─── ADIM 4: Ç1 — Korelasyon ───
    c1_result = calc_correlation_multiplier(
        candidate.get('symbol', ''),
        open_positions,
    )
    
    if c1_result['blocked']:
        return {
            'status': 'BLOCK',
            'reason': c1_result['reason'],
            'c1': 0.0,
            'candidate_group': c1_result['candidate_group'],
        }
    
    # ─── ADIM 5: Ç2 — Makro ───
    asset_class = get_asset_class(candidate.get('symbol', ''))
    
    c2_result = calc_macro_multiplier(
        vix=_safe_float(market_data.get('vix'), 18),
        vvix_change_5d=_safe_float(market_data.get('vvix_change_5d'), 0),
        dxy_trend=market_data.get('dxy_trend', 'FLAT'),
        asset_class=asset_class,
        vix_spot=market_data.get('vix_spot'),
        vix_futures_m1=market_data.get('vix_futures_m1'),
        us10y_trend=market_data.get('us10y_trend'),
        xu100_trend=market_data.get('xu100_trend'),
        sector_trend=market_data.get('sector_trend'),
        btc_dominance=market_data.get('btc_dominance'),
        candidate_symbol=candidate.get('symbol'),
    )
    
    # ─── ADIM 6: Ç3 — Teknik ───
    c3_result = calc_technical_multiplier(
        signal_status=candidate.get('signal_status', 'none'),
        adx=_safe_float(candidate.get('adx'), 20),
        adx_prev=candidate.get('adx_prev'),
        days_since_signal=candidate.get('days_since_signal', 0),
        price_vs_ema200_pct=_safe_float(candidate.get('price_vs_ema200_pct'), 0),
    )
    
    if c3_result['blocked']:
        return {
            'status': 'BLOCK',
            'reason': '1D yön çelişkisi — teknik hard block',
            'c3': 0.0,
            'signal_status': candidate.get('signal_status'),
        }
    
    # ─── ADIM 7: Ön risk (Ç6 hesabı için pozisyon boyutu lazım) ───
    prelim = base * c1_result['c1'] * c2_result['c2'] * c3_result['c3'] * dd_result['c5']
    prelim_risk_pct = _clamp(prelim, MIN_TRADE_RISK_PCT, MAX_TRADE_RISK_PCT)
    prelim_position_usd = portfolio_value * prelim_risk_pct / 100
    
    # ─── ADIM 8: Ç6 — Likidite ───
    c6_result = calc_liquidity_multiplier(
        symbol=candidate.get('symbol', ''),
        avg_daily_volume_usd=_safe_float(market_data.get('avg_daily_volume_usd'), 10_000_000),
        spread_pct=_safe_float(market_data.get('spread_pct'), 0.05),
        position_size_usd=prelim_position_usd,
        market_session=market_data.get('session', 'normal'),
    )
    
    if c6_result['blocked']:
        return {
            'status': 'BLOCK',
            'reason': 'Yetersiz likidite',
            'c6': 0.0,
            'vol_score': c6_result['vol_score'],
            'spread_score': c6_result['spread_score'],
        }
    
    # ─── ADIM 9: Ç7 — Zaman (yeni giriş = 1.0) ───
    c7_result = calc_time_decay_multiplier(
        days_open=0,
        signal_type=candidate.get('signal_type', 'swing'),
    )
    
    # ─── ADIM 10: Ekonomik takvim kontrolü ───
    calendar = check_economic_calendar()
    
    # ─── ADIM 11: Final hesaplama ───
    c1 = c1_result['c1']
    c2 = c2_result['c2']
    c3 = c3_result['c3']
    c5 = dd_result['c5']
    c6 = c6_result['c6']
    c7 = c7_result['c7']
    
    # Ekonomik takvim cezası
    if calendar['fomc_today'] and asset_class in ('crypto', 'metal', 'fx_bond'):
        c2 *= calendar['macro_penalty']
    if calendar['tcmb_today'] and asset_class == 'bist':
        c2 *= calendar['bist_penalty']
    
    raw_risk = base * c1 * c2 * c3 * c5 * c6 * c7
    # raw_risk zaten % cinsinden (BASE=1.0 → %1.0)
    risk_pct = _clamp(raw_risk, MIN_TRADE_RISK_PCT, MAX_TRADE_RISK_PCT)
    risk_pct = min(risk_pct, remaining_risk)  # Kalan risk limiti
    risk_usd = round(portfolio_value * risk_pct / 100, 2)
    
    return {
        'status': 'ALLOW',
        'risk_pct': round(risk_pct, 2),
        'risk_usd': risk_usd,
        'remaining_risk_pct': round(remaining_risk, 2),
        'remaining_after': round(remaining_risk - risk_pct, 2),
        'raw_multiplier': round(raw_risk, 4),
        'base': base,
        'multipliers': {
            'c1_correlation': c1,
            'c2_macro': c2,
            'c3_technical': c3,
            'c5_drawdown': c5,
            'c6_liquidity': c6,
            'c7_time': c7,
        },
        'details': {
            'kelly': kelly,
            'correlation': c1_result,
            'macro': c2_result,
            'technical': c3_result,
            'drawdown': dd_result,
            'liquidity': c6_result,
            'time_decay': c7_result,
        },
        'asset_class': asset_class,
        'candidate_symbol': candidate.get('symbol', ''),
        'calendar': calendar,
    }


# ═══════════════════════════════════════════════════════════════
# POZİSYON YÖNETİMİ — AÇIK POZİSYONLAR İÇİN REASSESS
# ═══════════════════════════════════════════════════════════════

def reassess_open_position(
    position: Dict,
    market_data: Dict,
    portfolio_value: float = PORTFOLIO_SIZE,
    peak_value: float = PORTFOLIO_SIZE,
    consecutive_losses: int = 0,
) -> Dict[str, Any]:
    """
    Açık pozisyonu yeniden değerlendir — Ç7 burada aktif.
    
    Args:
        position: {
            'symbol': str, 'direction': 'LONG'|'SHORT',
            'days_open': int, 'pnl_pct': float,
            'entry_price': float, 'current_price': float,
            'initial_stop': float, 'current_stop': float,
            'signal_type': str,  # giriş sinyali
            'adx': float, 'adx_trending_up': bool,
            'price_above_ema50': bool, 'alpha_trend_aligned': bool,
            'has_counter_signal_4h': bool,  # 4H'de karşı sinyal var mı
            'has_counter_signal_1d': bool,  # 1D'de karşı sinyal var mı
        }
    
    Returns:
        {'action': str, 'priority': str, 'reason': str, 'details': dict}
    """
    days = position.get('days_open', 0)
    pnl = _safe_float(position.get('pnl_pct'), 0)
    sym = position.get('symbol', '')
    
    actions = []
    
    # ─── Ç7: Zaman çürümesi ───
    c7_result = calc_time_decay_multiplier(
        days_open=days,
        pnl_pct=pnl,
        adx_value=_safe_float(position.get('adx'), 20),
        adx_trending_up=position.get('adx_trending_up', False),
        price_above_ema50=position.get('price_above_ema50', True),
        alpha_trend_aligned=position.get('alpha_trend_aligned', True),
        signal_type=position.get('signal_type', 'swing'),
    )
    
    # ─── Ç5: Drawdown (portföy seviyesinde) ───
    dd_result = calc_drawdown_multiplier(
        portfolio_value, peak_value, consecutive_losses
    )
    
    # ─── Karar mantığı ───
    
    # Seviye 3: Tam çıkış koşulları
    counter_1d = position.get('has_counter_signal_1d', False)
    counter_4h = position.get('has_counter_signal_4h', False)
    
    if counter_1d and counter_4h:
        actions.append({
            'action': 'CLOSE_ALL',
            'priority': 'HIGH',
            'reason': f'1D + 4H karşı sinyal — tam çıkış',
            'color': 'red',
        })
    
    if dd_result['moratorium']['active']:
        actions.append({
            'action': 'CLOSE_ALL',
            'priority': 'HIGH',
            'reason': f'Moratoryum aktif (DD {dd_result["dd_pct"]:.1f}%)',
            'color': 'red',
        })
    
    # VIX rejim değişimi
    vix = _safe_float(market_data.get('vix'), 18)
    regime = get_vix_regime(vix)
    asset_class = get_asset_class(sym)
    if regime == 'CRISIS' and asset_class == 'crypto':
        actions.append({
            'action': 'CLOSE_ALL',
            'priority': 'HIGH',
            'reason': f'CRISIS rejimi — kripto kapatılmalı (VIX {vix:.1f})',
            'color': 'red',
        })
    
    # Seviye 2: Pozisyon küçültme
    if counter_4h and not counter_1d:
        actions.append({
            'action': 'SCALE_DOWN_30',
            'priority': 'MEDIUM',
            'reason': '4H karşı sinyal — %30 kapat, kalanı koru',
            'color': 'orange',
        })
    
    if counter_1d and not counter_4h:
        actions.append({
            'action': 'SCALE_DOWN_50',
            'priority': 'MEDIUM',
            'reason': '1D karşı sinyal — %50 kapat, stop sıkılaştır',
            'color': 'orange',
        })
    
    if dd_result['severity'] in ('WARNING', 'DANGER'):
        actions.append({
            'action': 'SCALE_DOWN_25',
            'priority': 'MEDIUM',
            'reason': f'Drawdown uyarı ({dd_result["dd_pct"]:.1f}%) — %25 küçült',
            'color': 'orange',
        })
    
    # Seviye 1: Stop yönetimi
    if c7_result['c7'] < 0.80:
        actions.append({
            'action': 'TIGHTEN_STOP',
            'priority': 'MEDIUM',
            'reason': f'Zaman çürümesi yüksek (Ç7={c7_result["c7"]}) — stop sıkılaştır',
            'color': 'yellow',
        })
    
    if pnl > 3 and not position.get('stop_at_breakeven', False):
        actions.append({
            'action': 'MOVE_STOP_BREAKEVEN',
            'priority': 'LOW',
            'reason': f'P&L +{pnl:.1f}% — stop breakeven\'a çek',
            'color': 'blue',
        })
    
    # Hiçbir aksiyon yoksa
    if not actions:
        if pnl > 0:
            status_msg = f'Sağlıklı — P&L +{pnl:.1f}%, Ç7={c7_result["c7"]}'
            color = 'green'
        elif pnl > -2:
            status_msg = f'İzleniyor — P&L {pnl:.1f}%'
            color = 'blue'
        else:
            status_msg = f'Dikkat — P&L {pnl:.1f}%, stop mesafesi kontrol et'
            color = 'yellow'
        
        actions.append({
            'action': 'HOLD',
            'priority': 'INFO',
            'reason': status_msg,
            'color': color,
        })
    
    # En yüksek öncelikli aksiyonu seç
    priority_order = {'HIGH': 0, 'MEDIUM': 1, 'LOW': 2, 'INFO': 3}
    actions.sort(key=lambda a: priority_order.get(a['priority'], 99))
    
    return {
        'primary_action': actions[0],
        'all_actions': actions,
        'c7': c7_result,
        'c5': dd_result,
        'position': position,
    }


# ═══════════════════════════════════════════════════════════════
# WATCHLIST TARAMA — TÜM ADAYLARI DEĞERLENDIR
# ═══════════════════════════════════════════════════════════════

def scan_watchlist(
    candidates: List[Dict],
    open_positions: List[Dict],
    market_data: Dict,
    portfolio_value: float = PORTFOLIO_SIZE,
    peak_value: float = PORTFOLIO_SIZE,
    trades_history: List[Dict] = None,
) -> List[Dict]:
    """
    Watchlist'teki tüm adayları tarayıp sıralı sonuç döndür.
    
    Args:
        candidates: [{'symbol': str, 'signal_status': str, 
                      'signal_4h': str|None, ...}, ...]
    
    Returns:
        Sıralı sonuç listesi (risk% yüksekten düşüğe)
    """
    results = []
    
    for cand in candidates:
        result = calculate_position_risk(
            portfolio_value=portfolio_value,
            peak_value=peak_value,
            open_positions=open_positions,
            candidate=cand,
            market_data=market_data,
            trades_history=trades_history or [],
        )
        
        # 4H sinyal durumu
        signal_4h = cand.get('signal_4h')
        if signal_4h and signal_4h not in ('none', 'BEKLE', None):
            entry_light = 'AL'
        elif result.get('status') == 'BLOCK':
            entry_light = 'BLOK'
        else:
            entry_light = 'BEKLE'
        
        result['entry_light'] = entry_light
        result['signal_4h_type'] = signal_4h
        result['candidate'] = cand
        results.append(result)
    
    # Sıralama: ALLOW + AL > ALLOW + BEKLE > BLOCK
    def sort_key(r):
        status_score = 0 if r.get('status') == 'ALLOW' else 10
        light_score = 0 if r.get('entry_light') == 'AL' else 5
        risk_score = -r.get('risk_pct', 0)  # Yüksek risk% önce
        return (status_score, light_score, risk_score)
    
    results.sort(key=sort_key)
    return results


# ═══════════════════════════════════════════════════════════════
# EKONOMİK TAKVİM — FOMC + TCMB
# ═══════════════════════════════════════════════════════════════

# 2026 FOMC tarihleri (yılda 8 toplantı)
FOMC_DATES_2026 = [
    '2026-01-28', '2026-01-29',  # Ocak
    '2026-03-18', '2026-03-19',  # Mart
    '2026-05-06', '2026-05-07',  # Mayıs
    '2026-06-17', '2026-06-18',  # Haziran
    '2026-07-29', '2026-07-30',  # Temmuz
    '2026-09-16', '2026-09-17',  # Eylül
    '2026-11-04', '2026-11-05',  # Kasım
    '2026-12-16', '2026-12-17',  # Aralık
]

# 2026 TCMB PPK tarihleri (ayda 1, genelde perşembe)
TCMB_DATES_2026 = [
    '2026-01-23', '2026-02-20', '2026-03-20', '2026-04-17',
    '2026-05-22', '2026-06-19', '2026-07-24', '2026-08-21',
    '2026-09-18', '2026-10-23', '2026-11-20', '2026-12-18',
]


def check_economic_calendar(date_str: str = None) -> Dict[str, Any]:
    """
    Bugün veya verilen tarihte ekonomik takvim etkisi var mı?
    
    Returns:
        {'fomc_today': bool, 'tcmb_today': bool,
         'macro_penalty': float,  # Ç2'ye uygulanacak ek çarpan
         'bist_penalty': float,   # BIST Ç2'ye uygulanacak ek çarpan
         'warning': str|None}
    """
    if date_str is None:
        today = datetime.now().strftime('%Y-%m-%d')
    else:
        today = date_str

    fomc_today = today in FOMC_DATES_2026
    tcmb_today = today in TCMB_DATES_2026

    result = {
        'fomc_today': fomc_today,
        'tcmb_today': tcmb_today,
        'macro_penalty': 1.0,
        'bist_penalty': 1.0,
        'warning': None,
    }

    if fomc_today:
        result['macro_penalty'] = 0.5  # Kripto + emtia Ç2'ye ×0.5
        result['warning'] = 'FOMC günü — yeni pozisyon dikkatli'

    if tcmb_today:
        result['bist_penalty'] = 0.5   # BIST Ç2'ye ×0.5
        if result['warning']:
            result['warning'] += ' + TCMB günü'
        else:
            result['warning'] = 'TCMB PPK günü — BIST yeni pozisyon dikkatli'

    return result
