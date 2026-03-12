from flask import Flask, jsonify
from flask_cors import CORS
from trading_bot_analysis import MultiTimeframeAnalyzer
from macro_data import fetch_all_macro, make_decision
import threading, time
from datetime import datetime

app  = Flask(__name__)
CORS(app)

signals_cache       = {}
signals_lock        = threading.Lock()
signals_last_update = None

macro_cache         = {}
macro_lock          = threading.Lock()
macro_last_update   = None

TCMB_KEY = 'o5OlG3017a'
SYMBOLS  = ['BTC/USDT', 'ETH/USDT', 'SOL/USDT', 'BNB/USDT', 'XRP/USDT']

def refresh_all():
    global signals_last_update
    analyzer = MultiTimeframeAnalyzer(SYMBOLS)
    while True:
        try:
            print(f"\n[API] Kripto sinyaller guncelleniyor...")
            results = {}
            for sym in SYMBOLS:
                try:
                    data = analyzer.analyze(sym)
                    if data:
                        results[sym.replace('/', '_')] = data
                        print(f"[API] {sym} OK")
                except Exception as e:
                    print(f"[API] {sym} hata: {e}")
            with signals_lock:
                signals_cache.clear()
                signals_cache.update(results)
                signals_last_update = datetime.now().isoformat()
            print(f"[API] Kripto guncellendi - {len(results)} sembol")
        except Exception as e:
            print(f"[API] Genel hata: {e}")
        time.sleep(240)

def refresh_macro():
    global macro_last_update
    while True:
        try:
            print(f"\n[MACRO] Makro veri guncelleniyor...")
            data = fetch_all_macro(tcmb_key=TCMB_KEY)
            decision = make_decision(data)
            with macro_lock:
                macro_cache.clear()
                macro_cache['indices']  = data
                macro_cache['decision'] = decision
                macro_last_update = datetime.now().isoformat()
            print(f"[MACRO] Guncellendi - {len(data)} enstuman")
        except Exception as e:
            print(f"[MACRO] Hata: {e}")
        time.sleep(900)

@app.route('/api/health')
def health():
    with signals_lock:
        syms = list(signals_cache.keys())
    with macro_lock:
        macro_ok = len(macro_cache) > 0
    return jsonify({
        'status': 'ok', 'symbols': syms,
        'symbol_count': len(syms), 'macro': macro_ok,
        'signals_updated': signals_last_update,
        'macro_updated': macro_last_update,
    })

@app.route('/api/signals')
def get_signals():
    with signals_lock:
        data = dict(signals_cache)
    return jsonify({'status': 'ok', 'data': data, 'count': len(data), 'updated': signals_last_update})

@app.route('/api/signals/<symbol>')
def get_symbol(symbol):
    with signals_lock:
        data = signals_cache.get(symbol)
    if not data:
        return jsonify({'status': 'error', 'message': f'{symbol} bulunamadi'}), 404
    return jsonify({'status': 'ok', 'data': data})

@app.route('/api/scan')
def scan():
    with signals_lock:
        items = list(signals_cache.values())
    items.sort(key=lambda x: x.get('buy_score', 0) - x.get('sell_score', 0), reverse=True)
    return jsonify({'status': 'ok', 'data': items, 'count': len(items)})

@app.route('/api/macro')
def get_macro():
    with macro_lock:
        data = dict(macro_cache)
    return jsonify({'status': 'ok', 'data': data, 'updated': macro_last_update})

@app.route('/api/macro/decision')
def get_macro_decision():
    with macro_lock:
        decision = macro_cache.get('decision', [])
        indices  = macro_cache.get('indices', {})
    vix_val   = indices.get('VIX', {}).get('value', 0) if indices else 0
    risk_mode = 'RISK OFF' if vix_val > 20 else 'RISK ON'
    return jsonify({
        'status': 'ok', 'decision': decision,
        'risk_mode': risk_mode, 'vix': vix_val,
        'updated': macro_last_update
    })

if __name__ == '__main__':
    print("[API] RiskPlatform API Server baslatiliyor...")
    print(f"[API] Semboller: {SYMBOLS}")
    print("[API] Ilk veri yukleniyor (1-2 dakika)...")
    threading.Thread(target=refresh_all,   daemon=True).start()
    threading.Thread(target=refresh_macro, daemon=True).start()
    print("[API] http://localhost:5000")
    print("[API] Endpoints: /api/health  /api/signals  /api/scan  /api/macro  /api/macro/decision")
    app.run(host='0.0.0.0', port=5000, debug=False)
