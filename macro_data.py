"""
RiskPlatform v2.0 — Makro Veri Modülü
=======================================
Kaynaklar: yfinance (VIX/VVIX/DXY/US10Y/XU100/sektörler/BTC/ETH/XAU/XAG)
           CoinGecko (TOTAL mcap, BTC dominance)
           TCMB EVDS (TLREF)
Eklenen: VVIX, VIX Futures, 6 BIST sektör endeksi, dinamik tarih
"""

import requests
import yfinance as yf
from datetime import datetime, timedelta
from typing import Dict, Any, Optional
import numpy as np


# ═══════════════════════════════════════════════════════════════
# TICKER LİSTELERİ
# ═══════════════════════════════════════════════════════════════

# Ana makro göstergeler
YF_MACRO = {
    'VIX':    '^VIX',
    'VVIX':   '^VVIX',
    'VIX_FUT':'VX=F',       # VIX front month futures (term structure için)
    'DXY':    'DX-Y.NYB',
    'US10Y':  '^TNX',
    'EURUSD': 'EURUSD=X',
    'XAU':    'GC=F',
    'XAG':    'SI=F',
}

# BIST endeksler
YF_BIST_INDEX = {
    'XU100':  'XU100.IS',
    'XU030':  'XU030.IS',
    'XBANK':  'XBANK.IS',
    'XUSIN':  'XUSIN.IS',
    'XHOLD':  'XHOLD.IS',
    'XULAS':  'XULAS.IS',
    'XKMYA':  'XKMYA.IS',
    'XGIDA':  'XGIDA.IS',
}

# Kripto (yfinance üzerinden — sadece makro referans)
YF_CRYPTO = {
    'BTC':    'BTC-USD',
    'ETH':    'ETH-USD',
}


# ═══════════════════════════════════════════════════════════════
# VERİ ÇEKME FONKSİYONLARI
# ═══════════════════════════════════════════════════════════════

def fetch_yf_batch(ticker_map: Dict[str, str], period: str = '10d') -> Dict[str, Dict]:
    """
    yfinance'den toplu veri çek.
    Returns: {name: {'value': float, 'prev': float, 'change_pct': float,
                     'direction': str, 'history_5d': list}}
    """
    result = {}
    for name, ticker in ticker_map.items():
        try:
            t = yf.Ticker(ticker)
            df = t.history(period=period, interval='1d')
            if df is None or df.empty or len(df) < 2:
                raise ValueError(f"Yetersiz veri: {len(df) if df is not None else 0} bar")

            price = float(df['Close'].iloc[-1])
            prev = float(df['Close'].iloc[-2])
            chg = (price - prev) / prev * 100 if prev != 0 else 0

            # Son 5 günlük değişim (VVIX yönü için)
            if len(df) >= 6:
                price_5d_ago = float(df['Close'].iloc[-6])
                chg_5d = (price - price_5d_ago) / price_5d_ago if price_5d_ago != 0 else 0
            else:
                chg_5d = 0

            # Son 5 gün geçmişi
            history_5d = [round(float(x), 4) for x in df['Close'].tail(5).values]

            # Trend yönü (5 günlük basit)
            if len(df) >= 5:
                trend_prices = df['Close'].tail(5).values
                slope = np.polyfit(range(len(trend_prices)), trend_prices, 1)[0]
                if slope > 0:
                    direction = 'UP'
                elif slope < 0:
                    direction = 'DOWN'
                else:
                    direction = 'FLAT'
            else:
                direction = 'UP' if chg > 0 else 'DOWN' if chg < 0 else 'FLAT'

            result[name] = {
                'value': round(price, 4),
                'prev': round(prev, 4),
                'change_pct': round(chg, 2),
                'change_5d_pct': round(chg_5d * 100, 2),
                'change_5d_ratio': round(chg_5d, 4),
                'direction': direction,
                'history_5d': history_5d,
            }
            print(f"  [YF] {name:8s} {price:>12.4f}  {direction:<5s}  {chg:+.2f}%  (5d: {chg_5d*100:+.2f}%)")

        except Exception as e:
            print(f"  [YF] {name} hata: {e}")
            result[name] = {
                'value': 0, 'prev': 0, 'change_pct': 0,
                'change_5d_pct': 0, 'change_5d_ratio': 0,
                'direction': 'UNKNOWN', 'history_5d': [],
            }
    return result


def fetch_coingecko() -> Dict[str, Dict]:
    """CoinGecko global veri: TOTAL market cap + BTC dominance."""
    result = {}
    try:
        url = 'https://api.coingecko.com/api/v3/global'
        resp = requests.get(url, timeout=10)
        data = resp.json().get('data', {})

        total_mcap = data.get('total_market_cap', {}).get('usd', 0)
        btc_dom = data.get('market_cap_percentage', {}).get('btc', 0)
        total_str = f"{total_mcap / 1e9:.2f}B"

        mcap_chg_24h = data.get('market_cap_change_percentage_24h_usd', 0)

        result['TOTAL'] = {
            'value': total_mcap, 'display': total_str,
            'change_pct': round(mcap_chg_24h, 2),
            'direction': 'UP' if mcap_chg_24h > 0 else 'DOWN',
        }
        result['BTCD'] = {
            'value': round(btc_dom, 2),
            'display': f"{btc_dom:.2f}%",
            'direction': 'DOMINANT' if btc_dom > 55 else 'ALTCOIN' if btc_dom < 50 else 'GECIS',
        }
        print(f"  [CG] TOTAL    {total_str:>10s}  {'UP' if mcap_chg_24h >= 0 else 'DOWN'}")
        print(f"  [CG] BTCD     {btc_dom:>9.2f}%  {result['BTCD']['direction']}")

    except Exception as e:
        print(f"  [CG] Hata: {e}")
        result['TOTAL'] = {'value': 0, 'display': '0B', 'change_pct': 0, 'direction': 'UNKNOWN'}
        result['BTCD'] = {'value': 0, 'display': '0%', 'direction': 'UNKNOWN'}
    return result


def fetch_tlref(tcmb_key: str) -> Dict[str, Any]:
    """TCMB EVDS'den TLREF çek (dinamik tarih)."""
    try:
        # Dinamik tarih — her zaman geçerli
        now = datetime.now()
        start = (now - timedelta(days=90)).strftime('%d-%m-%Y')
        end = now.strftime('%d-%m-%Y')

        url = (
            f"https://evds2.tcmb.gov.tr/service/evds/"
            f"series=TP.TF.AA.BAS.YB.TL.O.N"
            f"&startDate={start}&endDate={end}"
            f"&type=json&key={tcmb_key}"
        )
        resp = requests.get(url, timeout=10)
        data = resp.json()
        items = data.get('items', [])
        for item in reversed(items):
            val = item.get('TP_TF_AA_BAS_YB_TL_O_N')
            if val and val not in ('', None):
                tlref_val = float(val)
                print(f"  [TCMB] TLREF  {tlref_val:>9.4f}%")
                return {
                    'value': round(tlref_val, 4),
                    'direction': 'HIGH' if tlref_val > 40 else 'MODERATE' if tlref_val > 25 else 'LOW',
                }
    except Exception as e:
        print(f"  [TCMB] TLREF hata: {e}")
    print(f"  [TCMB] TLREF      0.0000  UNKNOWN")
    return {'value': 0, 'direction': 'UNKNOWN'}


# ═══════════════════════════════════════════════════════════════
# BİRLEŞİK VERİ ÇEKME
# ═══════════════════════════════════════════════════════════════

def fetch_all_macro(tcmb_key: str = '') -> Dict[str, Dict]:
    """
    Tüm makro verileri tek çağrıda topla.
    Returns: tüm endeksler + türetilmiş metrikler
    """
    print(f"  [Macro] Veri çekiliyor... ({datetime.now().strftime('%H:%M:%S')})")
    result = {}

    # yfinance batch
    result.update(fetch_yf_batch(YF_MACRO))
    result.update(fetch_yf_batch(YF_BIST_INDEX))
    result.update(fetch_yf_batch(YF_CRYPTO))

    # CoinGecko
    result.update(fetch_coingecko())

    # TCMB
    if tcmb_key:
        result['TLREF'] = fetch_tlref(tcmb_key)
    else:
        result['TLREF'] = {'value': 0, 'direction': 'UNKNOWN'}

    # Türetilmiş metrikler
    result['_derived'] = derive_metrics(result)

    print(f"  [Macro] Tamamlandı — {len(result)} enstrüman")
    return result


def derive_metrics(data: Dict) -> Dict[str, Any]:
    """Makro verilerden türetilmiş metrikler hesapla."""
    vix = data.get('VIX', {}).get('value', 0)
    vvix = data.get('VVIX', {}).get('value', 0)
    vvix_5d = data.get('VVIX', {}).get('change_5d_ratio', 0)
    vix_spot = data.get('VIX', {}).get('value', 0)
    vix_fut = data.get('VIX_FUT', {}).get('value', 0)
    dxy = data.get('DXY', {}).get('direction', 'FLAT')
    us10y = data.get('US10Y', {}).get('direction', 'FLAT')
    xu100 = data.get('XU100', {}).get('direction', 'FLAT')
    btcd = data.get('BTCD', {}).get('value', 50)
    xau = data.get('XAU', {}).get('value', 0)
    xag = data.get('XAG', {}).get('value', 0)

    # VIX rejimi
    if vix == 0:
        vix_regime = 'UNKNOWN'
    elif vix < 15:
        vix_regime = 'LOW'
    elif vix < 20:
        vix_regime = 'NORMAL'
    elif vix < 30:
        vix_regime = 'ELEVATED'
    else:
        vix_regime = 'CRISIS'

    # VVIX yönü
    if vvix_5d > 0.15:
        vvix_dir = 'RISING'
    elif vvix_5d < -0.15:
        vvix_dir = 'FALLING'
    else:
        vvix_dir = 'STABLE'

    # VIX term structure
    if vix_spot > 0 and vix_fut > 0:
        ts_spread = (vix_fut - vix_spot) / vix_spot * 100
        if ts_spread > 5:
            term_structure = 'STEEP_CONTANGO'
        elif ts_spread > 0:
            term_structure = 'CONTANGO'
        elif ts_spread > -5:
            term_structure = 'FLAT'
        else:
            term_structure = 'BACKWARDATION'
    else:
        term_structure = 'CONTANGO'

    # Risk modu
    if vix >= 30:
        risk_mode = 'CRISIS'
    elif vix >= 20:
        risk_mode = 'RISK OFF'
    else:
        risk_mode = 'RISK ON'

    # XAU/XAG oranı
    xau_xag_ratio = xau / xag if xag > 0 else 0

    return {
        'vix_regime': vix_regime,
        'vvix_direction': vvix_dir,
        'vvix_change_5d': round(vvix_5d, 4),
        'term_structure': term_structure,
        'vix_spot': round(vix_spot, 2),
        'vix_futures': round(vix_fut, 2),
        'risk_mode': risk_mode,
        'dxy_trend': dxy,
        'us10y_trend': us10y,
        'xu100_trend': xu100,
        'btc_dominance': btcd,
        'xau_xag_ratio': round(xau_xag_ratio, 1),
    }


# ═══════════════════════════════════════════════════════════════
# MAKRO KARAR AKIŞI
# ═══════════════════════════════════════════════════════════════

def make_decision(data: Dict) -> list:
    """
    Makro karar akışı — 5 katmanlı.
    VIX/VVIX → Risk rejimi → DXY → Varlık sınıfı → Sektör
    """
    decisions = []
    derived = data.get('_derived', {})

    vix = data.get('VIX', {}).get('value', 0)
    vix_regime = derived.get('vix_regime', 'UNKNOWN')
    vvix_dir = derived.get('vvix_direction', 'STABLE')
    term_struct = derived.get('term_structure', 'CONTANGO')
    risk_mode = derived.get('risk_mode', 'RISK ON')
    dxy = derived.get('dxy_trend', 'FLAT')
    us10y = derived.get('us10y_trend', 'FLAT')
    xu100 = derived.get('xu100_trend', 'FLAT')
    btcd = derived.get('btc_dominance', 50)
    xau_xag = derived.get('xau_xag_ratio', 0)
    total_dir = data.get('TOTAL', {}).get('direction', 'UNKNOWN')

    # ─── Katman 1: VIX Rejimi ───
    decisions.append({
        'category': 'MAKRO', 'indicator': 'VIX',
        'value': vix, 'regime': vix_regime,
        'signal': f'VIX {vix:.1f}',
        'action': f'Rejim: {vix_regime}',
    })

    # ─── Katman 2: VVIX + Term Structure ───
    decisions.append({
        'category': 'MAKRO', 'indicator': 'VVIX',
        'signal': f'{vvix_dir} (5d: {derived.get("vvix_change_5d", 0)*100:+.1f}%)',
        'action': f'Term Structure: {term_struct}',
    })

    # ─── Katman 3: Risk Yönlendirmesi ───
    if risk_mode == 'CRISIS':
        decisions.append({
            'category': 'MAKRO', 'indicator': 'RISK',
            'signal': 'CRISIS', 'action': 'Kripto kapat, altın+nakit ağırlık',
        })
        if term_struct == 'BACKWARDATION' and vvix_dir == 'FALLING':
            decisions.append({
                'category': 'MAKRO', 'indicator': 'CONTRARIAN',
                'signal': 'Kriz sonu sinyali',
                'action': 'Küçük contrarian pozisyon değerlendir (min risk)',
            })
    elif risk_mode == 'RISK OFF':
        decisions.append({
            'category': 'MAKRO', 'indicator': 'RISK',
            'signal': 'RISK OFF', 'action': 'Kripto/hisse azalt, altın artır',
        })
        if dxy == 'DOWN':
            decisions.append({
                'category': 'EMTIA', 'indicator': 'DXY',
                'signal': 'DXY ZAYIF', 'action': 'XAU AL',
            })
            if xau_xag > 80:
                decisions.append({
                    'category': 'EMTIA', 'indicator': 'XAU/XAG',
                    'signal': f'Oran: {xau_xag:.0f}',
                    'action': 'XAG AL (görece ucuz)',
                })
        else:
            decisions.append({
                'category': 'MAKRO', 'indicator': 'DXY',
                'signal': 'DXY GÜÇLÜ', 'action': 'Stablecoin / USD park',
            })
    else:
        # RISK ON
        decisions.append({
            'category': 'MAKRO', 'indicator': 'RISK',
            'signal': 'RISK ON', 'action': 'Risk varlıklara ağırlık ver',
        })
        if dxy == 'DOWN':
            decisions.append({
                'category': 'MAKRO', 'indicator': 'DXY',
                'signal': 'DXY ZAYIF', 'action': 'Kripto + Altın + EM AL',
            })
        elif dxy == 'UP':
            decisions.append({
                'category': 'MAKRO', 'indicator': 'DXY',
                'signal': 'DXY GÜÇLÜ', 'action': 'USD varlık + değer hisseleri',
            })

    # ─── Katman 4: Kripto İç Rotasyon ───
    if total_dir == 'UP' and risk_mode != 'CRISIS':
        if btcd > 55:
            decisions.append({
                'category': 'KRIPTO', 'indicator': 'BTC.D',
                'signal': f'{btcd:.1f}% DOMINANT',
                'action': 'BTC ağırlık max',
            })
        elif btcd < 50:
            decisions.append({
                'category': 'KRIPTO', 'indicator': 'BTC.D',
                'signal': f'{btcd:.1f}% ALT SEZONU',
                'action': 'ETH + beta altcoin AL',
            })
        else:
            decisions.append({
                'category': 'KRIPTO', 'indicator': 'BTC.D',
                'signal': f'{btcd:.1f}% GEÇİŞ',
                'action': 'BTC core + büyük alt seçici',
            })
    elif total_dir == 'DOWN':
        decisions.append({
            'category': 'KRIPTO', 'indicator': 'TOTAL',
            'signal': 'TOTAL AŞAĞI',
            'action': 'Kripto kapat, stablecoin park',
        })

    # ─── Katman 5: BIST ───
    tlref = data.get('TLREF', {}).get('value', 0)

    if xu100 == 'UP':
        # Sektör rotasyonu
        up_sectors = []
        down_sectors = []
        for sector in ['XBANK', 'XUSIN', 'XHOLD', 'XULAS', 'XKMYA', 'XGIDA']:
            sec_dir = data.get(sector, {}).get('direction', 'FLAT')
            if sec_dir == 'UP':
                up_sectors.append(sector)
            elif sec_dir == 'DOWN':
                down_sectors.append(sector)

        if up_sectors:
            decisions.append({
                'category': 'BIST', 'indicator': 'SEKTÖR',
                'signal': f'XU100 YUKARI',
                'action': f'Güçlü sektörler: {", ".join(up_sectors)}',
            })
        if down_sectors:
            decisions.append({
                'category': 'BIST', 'indicator': 'SEKTÖR',
                'signal': 'Zayıf sektörler',
                'action': f'Kaçın: {", ".join(down_sectors)}',
            })
    else:
        decisions.append({
            'category': 'BIST', 'indicator': 'XU100',
            'signal': 'AŞAĞI',
            'action': 'BIST BEKLE — TL tahvil park',
        })

    # US10Y
    if us10y == 'DOWN' and vix_regime in ('ELEVATED', 'CRISIS'):
        decisions.append({
            'category': 'MAKRO', 'indicator': 'US10Y',
            'signal': 'FAİZ DÜŞÜYOR',
            'action': 'Tahvile kaçış — altın pozitif',
        })

    return decisions


# ═══════════════════════════════════════════════════════════════
# SEKTÖR ENDEKSİ YÖNÜ (Ç2 sektör çarpanı için)
# ═══════════════════════════════════════════════════════════════

def get_sector_trend(macro_data: Dict, symbol: str) -> Optional[str]:
    """
    Verilen sembolün sektör endeksinin yönünü döndür.
    trading_bot_analysis.py'deki SECTOR_MAP ile uyumlu.
    """
    from trading_bot_analysis import SECTOR_MAP
    sector = SECTOR_MAP.get(symbol.replace('.IS', ''), None)
    if sector and sector in macro_data:
        return macro_data[sector].get('direction', 'FLAT')
    return None
