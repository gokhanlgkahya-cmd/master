from flask import Flask, jsonify, request
from flask_cors import CORS
from trading_bot_analysis import MultiTimeframeAnalyzer
from macro_data import fetch_all_macro, make_decision
import threading, time, sqlite3, json, random
from datetime import datetime

# ─────────────────────────────────────────────────────────
# Flask + CORS + SocketIO (opsiyonel)
# ─────────────────────────────────────────────────────────
app = Flask(__name__)
CORS(app, resources={r"/api/*": {"origins": "*"}})

# Flask-SocketIO kuruluysa kullan, yoksa devam et
try:
    from flask_socketio import SocketIO, emit, join_room, leave_room
    socketio = SocketIO(app, cors_allowed_origins="*", async_mode='threading')
    WS_ENABLED = True
    print("[WS] Flask-SocketIO aktif")
except ImportError:
    socketio = None
    WS_ENABLED = False
    print("[WS] Flask-SocketIO yok — pip install flask-socketio ile kur")

# ─────────────────────────────────────────────────────────
# CACHE (in-memory)
# ─────────────────────────────────────────────────────────
signals_cache       = {}
signals_lock        = threading.Lock()
signals_last_update = None

macro_cache         = {}
macro_lock          = threading.Lock()
macro_last_update   = None

price_cache         = {}   # canlı simüle fiyatlar
price_lock          = threading.Lock()

# Rate limiting (IP bazlı, in-memory)
rate_limit_store    = {}
rate_limit_lock     = threading.Lock()

TCMB_KEY = 'o5OlG3017a'
SYMBOLS  = ['BTC/USDT', 'ETH/USDT', 'SOL/USDT', 'BNB/USDT', 'XRP/USDT']

# Tüm izlenen coinler (genişletilmiş liste)
ALL_COINS = [
    # Büyük kap
    {'symbol':'BTC/USDT','name':'Bitcoin',   'category':'large_cap','base':68000},
    {'symbol':'ETH/USDT','name':'Ethereum',  'category':'large_cap','base':3200 },
    {'symbol':'BNB/USDT','name':'BNB',       'category':'large_cap','base':580  },
    {'symbol':'SOL/USDT','name':'Solana',    'category':'large_cap','base':145  },
    {'symbol':'XRP/USDT','name':'Ripple',    'category':'large_cap','base':0.62 },
    # Orta kap
    {'symbol':'ADA/USDT', 'name':'Cardano',  'category':'mid_cap','base':0.52 },
    {'symbol':'AVAX/USDT','name':'Avalanche','category':'mid_cap','base':38.5 },
    {'symbol':'DOT/USDT', 'name':'Polkadot', 'category':'mid_cap','base':8.20 },
    {'symbol':'LINK/USDT','name':'Chainlink','category':'mid_cap','base':18.4 },
    {'symbol':'MATIC/USDT','name':'Polygon', 'category':'mid_cap','base':0.92 },
    {'symbol':'ATOM/USDT','name':'Cosmos',   'category':'mid_cap','base':9.80 },
    # DeFi
    {'symbol':'UNI/USDT', 'name':'Uniswap',  'category':'defi','base':10.2  },
    {'symbol':'AAVE/USDT','name':'Aave',      'category':'defi','base':189.0 },
    {'symbol':'CRV/USDT', 'name':'Curve',     'category':'defi','base':0.58  },
]

# ─────────────────────────────────────────────────────────
# RATE LIMITER
# ─────────────────────────────────────────────────────────
def check_rate_limit(ip, limit=100, window=60):
    now = time.time()
    with rate_limit_lock:
        if ip not in rate_limit_store:
            rate_limit_store[ip] = []
        # Eski istekleri temizle
        rate_limit_store[ip] = [t for t in rate_limit_store[ip] if now - t < window]
        if len(rate_limit_store[ip]) >= limit:
            return False
        rate_limit_store[ip].append(now)
        return True

# ─────────────────────────────────────────────────────────
# SQLite VERITABANI
# ─────────────────────────────────────────────────────────
DB_PATH = 'riskplatform.db'

def init_db():
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()

    c.execute('''CREATE TABLE IF NOT EXISTS signal_history (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        symbol      TEXT NOT NULL,
        timeframe   TEXT DEFAULT '4h',
        signal_type TEXT,
        mtf_score   INTEGER,
        mtf_class   TEXT,
        price       REAL,
        trend_1d    TEXT,
        indicators  TEXT,
        created_at  TEXT
    )''')

    c.execute('''CREATE TABLE IF NOT EXISTS favorites (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        symbol     TEXT NOT NULL UNIQUE,
        name       TEXT,
        added_at   TEXT
    )''')

    c.execute('''CREATE TABLE IF NOT EXISTS alerts (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        symbol       TEXT NOT NULL,
        condition    TEXT NOT NULL,
        target_price REAL,
        signal_class TEXT,
        message      TEXT,
        is_active    INTEGER DEFAULT 1,
        triggered_at TEXT,
        created_at   TEXT
    )''')

    c.execute('''CREATE TABLE IF NOT EXISTS logs (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        level      TEXT,
        service    TEXT,
        message    TEXT,
        created_at TEXT
    )''')

    conn.commit()
    conn.close()
    print("[DB] SQLite veritabani hazir:", DB_PATH)

def db_log(level, service, message):
    try:
        conn = sqlite3.connect(DB_PATH)
        conn.execute("INSERT INTO logs (level,service,message,created_at) VALUES (?,?,?,?)",
                     (level, service, message, datetime.now().isoformat()))
        conn.commit()
        conn.close()
    except: pass

def save_signal_to_db(data):
    try:
        conn = sqlite3.connect(DB_PATH)
        conn.execute(
            "INSERT INTO signal_history (symbol,timeframe,signal_type,mtf_score,mtf_class,price,trend_1d,indicators,created_at) VALUES (?,?,?,?,?,?,?,?,?)",
            (
                data.get('symbol'), '4h',
                'BUY' if data.get('mtf_buy') else 'SELL' if data.get('mtf_sell') else 'NEUTRAL',
                data.get('mtf_score', 0),
                data.get('mtf_class', 'F'),
                data.get('price', 0),
                json.dumps(data.get('trend_1d', {})),
                json.dumps(data.get('score_4h', {}).get('signals', {})),
                datetime.now().isoformat()
            )
        )
        conn.commit()
        conn.close()
    except Exception as e:
        print(f"[DB] Kayit hatasi: {e}")

def check_alerts_trigger(symbol, price, mtf_class):
    """Alarm koşullarını kontrol et ve tetiklenen alarmları döndür."""
    triggered = []
    try:
        conn = sqlite3.connect(DB_PATH)
        rows = conn.execute(
            "SELECT id, condition, target_price, signal_class, message FROM alerts WHERE symbol=? AND is_active=1",
            (symbol,)
        ).fetchall()
        for row in rows:
            aid, cond, target, sig_cls, msg = row
            hit = False
            if cond == '>'    and price > target:    hit = True
            if cond == '<'    and price < target:    hit = True
            if cond == 'class' and mtf_class == sig_cls: hit = True
            if hit:
                conn.execute("UPDATE alerts SET is_active=0, triggered_at=? WHERE id=?",
                             (datetime.now().isoformat(), aid))
                triggered.append({'id': aid, 'symbol': symbol, 'message': msg or f"{symbol} alarm tetiklendi"})
        conn.commit()
        conn.close()
    except Exception as e:
        print(f"[DB] Alarm kontrol hatasi: {e}")
    return triggered

# ─────────────────────────────────────────────────────────
# ARKA PLAN THREADLER
# ─────────────────────────────────────────────────────────
def refresh_all():
    """Her 4 dakikada bir MTF sinyalleri güncelle."""
    global signals_last_update
    analyzer = MultiTimeframeAnalyzer(SYMBOLS)
    while True:
        try:
            print(f"\n[API] Kripto sinyaller guncelleniyor... {datetime.now().strftime('%H:%M:%S')}")
            results = {}
            for sym in SYMBOLS:
                try:
                    data = analyzer.analyze(sym)
                    if data:
                        key = sym.replace('/', '_')
                        results[key] = data
                        save_signal_to_db(data)

                        # Alarm kontrolü
                        triggered = check_alerts_trigger(sym, data.get('price', 0), data.get('mtf_class', 'F'))
                        for alert in triggered:
                            print(f"[ALARM] {alert['message']}")
                            db_log('WARN', 'AlertService', f"Alarm tetiklendi: {alert['message']}")
                            if WS_ENABLED:
                                socketio.emit('signal_alert', alert)

                        print(f"[API] {sym} OK  skor:{data.get('mtf_score',0)}/16  sinif:{data.get('mtf_class','?')}")
                except Exception as e:
                    print(f"[API] {sym} hata: {e}")

            with signals_lock:
                signals_cache.clear()
                signals_cache.update(results)
                signals_last_update = datetime.now().isoformat()

            print(f"[API] Kripto guncellendi - {len(results)} sembol")
            db_log('INFO', 'SignalService', f"{len(results)} sembol guncellendi")

            if WS_ENABLED:
                socketio.emit('signal_update', {'count': len(results), 'updated': signals_last_update})

        except Exception as e:
            print(f"[API] Genel hata: {e}")
            db_log('ERROR', 'SignalService', str(e))
        time.sleep(240)

def refresh_macro():
    """Her 15 dakikada bir makro veri güncelle."""
    global macro_last_update
    while True:
        try:
            print(f"\n[MACRO] Makro veri guncelleniyor... {datetime.now().strftime('%H:%M:%S')}")
            data = fetch_all_macro(tcmb_key=TCMB_KEY)
            decision = make_decision(data)
            with macro_lock:
                macro_cache.clear()
                macro_cache['indices']  = data
                macro_cache['decision'] = decision
                macro_last_update = datetime.now().isoformat()
            print(f"[MACRO] Guncellendi - {len(data)} enstuman")
            db_log('INFO', 'MacroService', f"{len(data)} enstruman guncellendi")

            if WS_ENABLED:
                vix = data.get('VIX', {}).get('value', 0)
                socketio.emit('macro_update', {'vix': vix, 'updated': macro_last_update})

        except Exception as e:
            print(f"[MACRO] Hata: {e}")
            db_log('ERROR', 'MacroService', str(e))
        time.sleep(900)

def broadcast_prices():
    """Her 3 saniyede bir simüle fiyat yayını (WebSocket)."""
    # Başlangıç fiyatları
    prices = {c['symbol']: c['base'] for c in ALL_COINS}
    while True:
        time.sleep(3)
        updates = {}
        with price_lock:
            for sym, price in prices.items():
                delta  = price * (random.random() - 0.5) * 0.008
                new_p  = max(price + delta, 0.001)
                prices[sym] = new_p
                price_cache[sym] = round(new_p, 6)
                updates[sym.replace('/', '_')] = round(new_p, 6)

        if WS_ENABLED:
            socketio.emit('price_update', updates)

# ─────────────────────────────────────────────────────────
# MIDDLEWARE — rate limit
# ─────────────────────────────────────────────────────────
@app.before_request
def before():
    ip = request.remote_addr or '0.0.0.0'
    if not check_rate_limit(ip, limit=200, window=60):
        return jsonify({'status': 'error', 'message': 'Rate limit asild (200 req/dk)'}), 429

# ─────────────────────────────────────────────────────────
# ENDPOINTS — SİSTEM
# ─────────────────────────────────────────────────────────
@app.route('/api/health')
def health():
    with signals_lock:
        syms = list(signals_cache.keys())
    with macro_lock:
        macro_ok = len(macro_cache) > 0
    return jsonify({
        'status':          'ok',
        'version':         'v3.2',
        'symbols':         syms,
        'symbol_count':    len(syms),
        'macro':           macro_ok,
        'websocket':       WS_ENABLED,
        'signals_updated': signals_last_update,
        'macro_updated':   macro_last_update,
        'server_time':     datetime.now().isoformat(),
    })

@app.route('/api/status')
def status():
    try:
        conn = sqlite3.connect(DB_PATH)
        sig_count = conn.execute("SELECT COUNT(*) FROM signal_history").fetchone()[0]
        fav_count = conn.execute("SELECT COUNT(*) FROM favorites").fetchone()[0]
        alr_count = conn.execute("SELECT COUNT(*) FROM alerts WHERE is_active=1").fetchone()[0]
        conn.close()
    except:
        sig_count = fav_count = alr_count = 0
    return jsonify({
        'status': 'ok', 'db': 'sqlite',
        'total_signals': sig_count,
        'favorites': fav_count,
        'active_alerts': alr_count,
        'cached_symbols': len(signals_cache),
        'ws_enabled': WS_ENABLED,
    })

# ─────────────────────────────────────────────────────────
# ENDPOINTS — SİNYALLER
# ─────────────────────────────────────────────────────────
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
        return jsonify({'status': 'error', 'message': f'{symbol} cache\'de yok'}), 404
    return jsonify({'status': 'ok', 'data': data})

@app.route('/api/scan')
def scan():
    """En güçlü sinyalleri sıralı döndür."""
    min_score = request.args.get('min_score', 0, type=int)
    direction = request.args.get('direction', 'all')  # all / buy / sell
    with signals_lock:
        items = list(signals_cache.values())

    # Filtrele
    if direction == 'buy':
        items = [i for i in items if i.get('mtf_buy')]
    elif direction == 'sell':
        items = [i for i in items if i.get('mtf_sell')]
    if min_score > 0:
        items = [i for i in items if i.get('mtf_score', 0) >= min_score]

    items.sort(key=lambda x: x.get('mtf_score', 0), reverse=True)
    return jsonify({'status': 'ok', 'data': items, 'count': len(items)})

# ─────────────────────────────────────────────────────────
# ENDPOINTS — GEÇMİŞ
# ─────────────────────────────────────────────────────────
@app.route('/api/history')
def get_history():
    """Tüm sinyal geçmişi."""
    limit = request.args.get('limit', 50, type=int)
    try:
        conn = sqlite3.connect(DB_PATH)
        rows = conn.execute(
            "SELECT symbol, signal_type, mtf_score, mtf_class, price, created_at FROM signal_history ORDER BY id DESC LIMIT ?",
            (limit,)
        ).fetchall()
        conn.close()
        data = [{'symbol': r[0], 'signal_type': r[1], 'mtf_score': r[2],
                 'mtf_class': r[3], 'price': r[4], 'created_at': r[5]} for r in rows]
        return jsonify({'status': 'ok', 'data': data, 'count': len(data)})
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

@app.route('/api/history/<symbol>')
def get_symbol_history(symbol):
    """Belirli sembol için sinyal geçmişi."""
    limit = request.args.get('limit', 20, type=int)
    sym_normalized = symbol.replace('_', '/')
    try:
        conn = sqlite3.connect(DB_PATH)
        rows = conn.execute(
            "SELECT signal_type, mtf_score, mtf_class, price, trend_1d, indicators, created_at FROM signal_history WHERE symbol=? ORDER BY id DESC LIMIT ?",
            (sym_normalized, limit)
        ).fetchall()
        conn.close()
        data = [{'signal_type': r[0], 'mtf_score': r[1], 'mtf_class': r[2],
                 'price': r[3], 'trend_1d': json.loads(r[4] or '{}'),
                 'indicators': json.loads(r[5] or '{}'), 'created_at': r[6]} for r in rows]
        return jsonify({'status': 'ok', 'symbol': sym_normalized, 'data': data})
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

# ─────────────────────────────────────────────────────────
# ENDPOINTS — MARKET / COINS
# ─────────────────────────────────────────────────────────
@app.route('/api/coins')
def get_coins():
    """İzlenen tüm coin listesi + canlı simüle fiyatlar."""
    category = request.args.get('category')
    with price_lock:
        current_prices = dict(price_cache)

    result = []
    for c in ALL_COINS:
        if category and c['category'] != category:
            continue
        sym = c['symbol']
        price = current_prices.get(sym, c['base'])
        result.append({
            'symbol':   sym,
            'name':     c['name'],
            'category': c['category'],
            'price':    round(price, 6),
            'base':     c['base'],
            'change_pct': round((price - c['base']) / c['base'] * 100, 2),
        })

    return jsonify({'status': 'ok', 'data': result, 'count': len(result)})

@app.route('/api/prices')
def get_prices():
    """Tüm canlı fiyatlar (WebSocket alternatifi)."""
    with price_lock:
        data = dict(price_cache)
    return jsonify({'status': 'ok', 'data': data, 'updated': datetime.now().isoformat()})

# ─────────────────────────────────────────────────────────
# ENDPOINTS — FAVORİLER
# ─────────────────────────────────────────────────────────
@app.route('/api/favorites', methods=['GET'])
def get_favorites():
    try:
        conn = sqlite3.connect(DB_PATH)
        rows = conn.execute("SELECT symbol, name, added_at FROM favorites ORDER BY added_at DESC").fetchall()
        conn.close()
        data = [{'symbol': r[0], 'name': r[1], 'added_at': r[2]} for r in rows]
        return jsonify({'status': 'ok', 'data': data})
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

@app.route('/api/favorites', methods=['POST'])
def add_favorite():
    body = request.get_json() or {}
    symbol = body.get('symbol', '').strip().upper()
    name   = body.get('name', symbol)
    if not symbol:
        return jsonify({'status': 'error', 'message': 'symbol gerekli'}), 400
    try:
        conn = sqlite3.connect(DB_PATH)
        conn.execute("INSERT OR IGNORE INTO favorites (symbol, name, added_at) VALUES (?,?,?)",
                     (symbol, name, datetime.now().isoformat()))
        conn.commit()
        conn.close()
        return jsonify({'status': 'ok', 'message': f'{symbol} favorilere eklendi'})
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

@app.route('/api/favorites/<symbol>', methods=['DELETE'])
def delete_favorite(symbol):
    try:
        conn = sqlite3.connect(DB_PATH)
        conn.execute("DELETE FROM favorites WHERE symbol=?", (symbol.replace('_', '/'),))
        conn.commit()
        conn.close()
        return jsonify({'status': 'ok', 'message': f'{symbol} favorilerden cikarildi'})
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

# ─────────────────────────────────────────────────────────
# ENDPOINTS — ALARMLAR
# ─────────────────────────────────────────────────────────
@app.route('/api/alerts', methods=['GET'])
def get_alerts():
    active_only = request.args.get('active', '1') == '1'
    try:
        conn = sqlite3.connect(DB_PATH)
        query = "SELECT id,symbol,condition,target_price,signal_class,message,is_active,triggered_at,created_at FROM alerts"
        if active_only:
            query += " WHERE is_active=1"
        query += " ORDER BY id DESC"
        rows = conn.execute(query).fetchall()
        conn.close()
        data = [{'id': r[0], 'symbol': r[1], 'condition': r[2], 'target_price': r[3],
                 'signal_class': r[4], 'message': r[5], 'is_active': bool(r[6]),
                 'triggered_at': r[7], 'created_at': r[8]} for r in rows]
        return jsonify({'status': 'ok', 'data': data})
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

@app.route('/api/alerts', methods=['POST'])
def add_alert():
    body = request.get_json() or {}
    symbol    = body.get('symbol', '').strip()
    condition = body.get('condition', '>')
    target    = body.get('target_price')
    sig_cls   = body.get('signal_class')
    message   = body.get('message', '')

    if not symbol:
        return jsonify({'status': 'error', 'message': 'symbol gerekli'}), 400
    if condition not in ('>', '<', 'class'):
        return jsonify({'status': 'error', 'message': 'condition: >, < veya class'}), 400

    try:
        conn = sqlite3.connect(DB_PATH)
        conn.execute(
            "INSERT INTO alerts (symbol,condition,target_price,signal_class,message,is_active,created_at) VALUES (?,?,?,?,?,1,?)",
            (symbol, condition, target, sig_cls, message, datetime.now().isoformat())
        )
        conn.commit()
        conn.close()
        return jsonify({'status': 'ok', 'message': f'{symbol} alarmi olusturuldu'})
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

@app.route('/api/alerts/<int:alert_id>', methods=['DELETE'])
def delete_alert(alert_id):
    try:
        conn = sqlite3.connect(DB_PATH)
        conn.execute("DELETE FROM alerts WHERE id=?", (alert_id,))
        conn.commit()
        conn.close()
        return jsonify({'status': 'ok', 'message': 'Alarm silindi'})
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

# ─────────────────────────────────────────────────────────
# ENDPOINTS — MAKRO
# ─────────────────────────────────────────────────────────
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
    vix_val    = indices.get('VIX', {}).get('value', 0) if indices else 0
    btcd_val   = indices.get('BTCD', {}).get('value', 0) if indices else 0
    risk_mode  = 'CRISIS' if vix_val > 30 else 'ELEVATED' if vix_val > 20 else 'NORMAL' if vix_val > 15 else 'LOW'
    return jsonify({
        'status': 'ok', 'decision': decision,
        'risk_mode': risk_mode, 'vix': vix_val, 'btcd': btcd_val,
        'updated': macro_last_update
    })

# ─────────────────────────────────────────────────────────
# ENDPOINTS — LOGLAR
# ─────────────────────────────────────────────────────────
@app.route('/api/logs')
def get_logs():
    limit = request.args.get('limit', 50, type=int)
    level = request.args.get('level')
    try:
        conn = sqlite3.connect(DB_PATH)
        query = "SELECT level,service,message,created_at FROM logs"
        params = []
        if level:
            query += " WHERE level=?"; params.append(level)
        query += " ORDER BY id DESC LIMIT ?"
        params.append(limit)
        rows = conn.execute(query, params).fetchall()
        conn.close()
        data = [{'level': r[0], 'service': r[1], 'message': r[2], 'created_at': r[3]} for r in rows]
        return jsonify({'status': 'ok', 'data': data})
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 500

# ─────────────────────────────────────────────────────────
# WEBSOCKET EVENTS (Flask-SocketIO)
# ─────────────────────────────────────────────────────────
if WS_ENABLED:
    @socketio.on('connect')
    def on_connect():
        print(f"[WS] Client baglandi: {request.sid}")
        emit('connected', {'status': 'ok', 'sid': request.sid})

    @socketio.on('disconnect')
    def on_disconnect():
        print(f"[WS] Client ayrildi: {request.sid}")

    @socketio.on('subscribe')
    def on_subscribe(data):
        symbol = data.get('symbol', 'ALL')
        join_room(symbol)
        emit('subscribed', {'symbol': symbol})
        print(f"[WS] {request.sid} → {symbol} abone oldu")

    @socketio.on('unsubscribe')
    def on_unsubscribe(data):
        symbol = data.get('symbol', 'ALL')
        leave_room(symbol)
        emit('unsubscribed', {'symbol': symbol})

# ─────────────────────────────────────────────────────────
# MAIN
# ─────────────────────────────────────────────────────────
if __name__ == '__main__':
    print("=" * 60)
    print("  RiskPlatform API Server v3.2")
    print("=" * 60)

    init_db()
    db_log('INFO', 'System', 'Server baslatildi')

    print(f"[API] Semboller: {SYMBOLS}")
    print("[API] Threadler baslatiliyor...")

    threading.Thread(target=refresh_all,     daemon=True).start()
    threading.Thread(target=refresh_macro,   daemon=True).start()
    threading.Thread(target=broadcast_prices,daemon=True).start()

    print("\n[API] Endpoints:")
    print("  GET  /api/health")
    print("  GET  /api/status")
    print("  GET  /api/signals")
    print("  GET  /api/signals/<symbol>")
    print("  GET  /api/scan?direction=buy&min_score=7")
    print("  GET  /api/history")
    print("  GET  /api/history/<symbol>")
    print("  GET  /api/coins?category=large_cap")
    print("  GET  /api/prices")
    print("  GET  /api/macro")
    print("  GET  /api/macro/decision")
    print("  GET/POST/DELETE  /api/favorites")
    print("  GET/POST/DELETE  /api/alerts")
    print("  GET  /api/logs")
    print(f"\n[WS]  WebSocket: {'AKTIF (flask-socketio)' if WS_ENABLED else 'KAPALI'}")
    print("[API] http://localhost:5000")
    print("=" * 60)

    if WS_ENABLED:
        socketio.run(app, host='0.0.0.0', port=5000, debug=False, allow_unsafe_werkzeug=True)
    else:
        app.run(host='0.0.0.0', port=5000, debug=False)
