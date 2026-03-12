import requests
import yfinance as yf
from datetime import datetime, timedelta

YF_TICKERS = {
    'VIX':   '^VIX',
    'DXY':   'DX-Y.NYB',
    'US10Y': '^TNX',
    'EURUSD':'EURUSD=X',
    'XAU':   'GC=F',
    'XAG':   'SI=F',
    'XU30':  'XU030.IS',
    'BTC':   'BTC-USD',
    'ETH':   'ETH-USD',
}

def fetch_yf(ticker_map):
    result = {}
    for name, ticker in ticker_map.items():
        try:
            t  = yf.Ticker(ticker)
            df = t.history(period='5d', interval='1d')
            if df is None or df.empty:
                raise ValueError("empty")
            price = float(df['Close'].iloc[-1])
            prev  = float(df['Close'].iloc[-2]) if len(df) > 1 else price
            chg   = (price - prev) / prev * 100
            direction = 'UP' if chg > 0 else 'DOWN'
            result[name] = {'value': round(price,4), 'change_pct': round(chg,2), 'direction': direction}
            print(f"  [YF] {name:8s} {price:12.4f}  {direction}  {chg:+.2f}%")
        except Exception as e:
            print(f"  [YF] {name} hata: {e}")
            result[name] = {'value': 0, 'change_pct': 0, 'direction': 'UNKNOWN'}
    return result

def fetch_coingecko():
    result = {}
    try:
        url  = 'https://api.coingecko.com/api/v3/global'
        resp = requests.get(url, timeout=10)
        data = resp.json().get('data', {})

        total_mcap = data.get('total_market_cap', {}).get('usd', 0)
        btc_dom    = data.get('market_cap_percentage', {}).get('btc', 0)
        total_str  = f"{total_mcap/1e9:.2f}B"
        prev_mcap  = data.get('total_market_cap_yesterday', {}).get('usd', total_mcap) if 'total_market_cap_yesterday' in data else total_mcap
        chg        = (total_mcap - prev_mcap) / prev_mcap * 100 if prev_mcap else 0

        result['TOTAL'] = {'value': total_mcap, 'display': total_str, 'change_pct': round(chg,2), 'direction': 'UP' if chg > 0 else 'DOWN'}
        result['BTCD']  = {'value': round(btc_dom,2), 'display': f"{btc_dom:.2f}%", 'direction': 'DOMINANT' if btc_dom > 55 else 'ALTCOIN'}
        print(f"  [CG] TOTAL     {total_str}  {'UP' if chg >= 0 else 'DOWN'}")
        print(f"  [CG] BTCD      {btc_dom:.2f}%   {'DOMINANT' if btc_dom > 55 else 'ALTCOIN'}")
    except Exception as e:
        print(f"  [CG] Hata: {e}")
        result['TOTAL'] = {'value': 0, 'display': '0B', 'change_pct': 0, 'direction': 'UNKNOWN'}
        result['BTCD']  = {'value': 0, 'display': '0%', 'direction': 'UNKNOWN'}
    return result

def fetch_tlref(tcmb_key):
    try:
        url = (
            f"https://evds2.tcmb.gov.tr/service/evds/"
            f"series=TP.TF.AA.BAS.YB.TL.O.N"
            f"&startDate=01-01-2024&endDate=31-12-2025"
            f"&type=json&key={tcmb_key}"
        )
        resp = requests.get(url, timeout=10)
        data = resp.json()
        items = data.get('items', [])
        for item in reversed(items):
            val = item.get('TP_TF_AA_BAS_YB_TL_O_N')
            if val and val not in ['', None]:
                tlref_val = float(val)
                print(f"  [TCMB] TLREF   {tlref_val:.4f}%")
                return {'value': round(tlref_val,4), 'direction': 'UP' if tlref_val > 40 else 'DOWN'}
    except Exception as e:
        print(f"  [TCMB] TLREF hata: {e}")
    print(f"  [TCMB] TLREF   0.0000  UNKNOWN")
    return {'value': 0, 'direction': 'UNKNOWN'}

def fetch_all_macro(tcmb_key=''):
    print(f"  [Macro] Veri cekiliyor... ({datetime.now().strftime('%H:%M:%S')})")
    result = {}
    result.update(fetch_yf(YF_TICKERS))
    result.update(fetch_coingecko())
    result['TLREF'] = fetch_tlref(tcmb_key)
    print(f"  [Macro] Tamamlandi - {len(result)} enstuman")
    return result

def make_decision(data):
    """
    Makro karar akisi:
    VIX -> RISK ON/OFF
      RISK ON  -> DXY -> EURUSD / Kripto (TOTAL -> BTCD -> BTC / ETH)
      RISK OFF -> DXY -> XAU/XAG ratio
    XU30 -> BIST yonu
    """
    decisions = []

    vix   = data.get('VIX',   {}).get('value', 0)
    dxy   = data.get('DXY',   {}).get('direction', 'UNKNOWN')
    xau   = data.get('XAU',   {}).get('value', 0)
    xag   = data.get('XAG',   {}).get('value', 0)
    btcd  = data.get('BTCD',  {}).get('value', 0)
    total = data.get('TOTAL', {}).get('direction', 'UNKNOWN')
    xu30  = data.get('XU30',  {}).get('direction', 'UNKNOWN')
    tlref = data.get('TLREF', {}).get('value', 0)

    # VIX rejimi
    if vix == 0:
        vix_regime = 'UNKNOWN'
    elif vix < 15:
        vix_regime = 'LOWVol'
    elif vix < 20:
        vix_regime = 'NORMALVol'
    elif vix < 30:
        vix_regime = 'ELEVATEDVol'
    else:
        vix_regime = 'CRISISVol'

    decisions.append({'category': 'MAKRO', 'indicator': 'VIX', 'value': vix, 'regime': vix_regime})

    # RISK OFF yolu
    if vix >= 20:
        decisions.append({'category': 'MAKRO', 'indicator': 'RISK', 'signal': 'RISK OFF', 'action': 'Kripto/Hisse azalt, Altin artir'})
        if dxy == 'DOWN':
            decisions.append({'category': 'EMTIA', 'indicator': 'DXY', 'signal': 'DXY ZAYIF', 'action': 'XAU AL'})
            if xau > 0 and xag > 0:
                ratio = xau / xag
                if ratio > 80:
                    decisions.append({'category': 'EMTIA', 'indicator': 'XAU/XAG', 'signal': f'Oran:{ratio:.0f}', 'action': 'XAG AL (ucuz)'})
                else:
                    decisions.append({'category': 'EMTIA', 'indicator': 'XAU/XAG', 'signal': f'Oran:{ratio:.0f}', 'action': 'XAU AL'})
        else:
            decisions.append({'category': 'MAKRO', 'indicator': 'DXY', 'signal': 'DXY GUCLU', 'action': 'Stablecoin / USD park'})

    # RISK ON yolu
    else:
        decisions.append({'category': 'MAKRO', 'indicator': 'RISK', 'signal': 'RISK ON', 'action': 'Risk varliklara agirlik ver'})
        if dxy == 'DOWN':
            decisions.append({'category': 'MAKRO', 'indicator': 'DXY', 'signal': 'DXY ZAYIF', 'action': 'Kripto + Altin + EM AL'})
        else:
            decisions.append({'category': 'MAKRO', 'indicator': 'DXY', 'signal': 'DXY GUCLU', 'action': 'USD varlik + Deger hisseleri'})

        if total == 'UP':
            if btcd > 55:
                decisions.append({'category': 'KRIPTO', 'indicator': 'BTC.D', 'signal': f'{btcd:.1f}% DOMINANT', 'action': 'BTC agirlik max'})
            elif btcd < 50:
                decisions.append({'category': 'KRIPTO', 'indicator': 'BTC.D', 'signal': f'{btcd:.1f}% ALT SEZONU', 'action': 'ETH + Beta altcoin AL'})
            else:
                decisions.append({'category': 'KRIPTO', 'indicator': 'BTC.D', 'signal': f'{btcd:.1f}% GECIS', 'action': 'BTC core + buyuk alt secici'})
        else:
            decisions.append({'category': 'KRIPTO', 'indicator': 'TOTAL', 'signal': 'TOTAL ASAGI', 'action': 'Kripto kapat, stablecoin park'})

    # BIST yonu
    if xu30 == 'UP':
        if tlref > 38:
            decisions.append({'category': 'BIST', 'indicator': 'XU30+TLREF', 'signal': 'YUKARI', 'action': 'BIST AL'})
        else:
            decisions.append({'category': 'BIST', 'indicator': 'XU30', 'signal': 'YUKARI / TLREF ZAYIF', 'action': 'BIST secici (reel faiz kontrol et)'})
    else:
        decisions.append({'category': 'BIST', 'indicator': 'XU30', 'signal': 'ASAGI', 'action': 'BIST BEKLE - TL tahvil park'})

    return decisions
