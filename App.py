"""
RiskPlatform v2.0 — Ana Backend Server
========================================
Flask API — Karar Platformu (İşlem platformu DEĞİL)
Portföy: $100,000 | Refresh: Saatlik | Haftalık tarama: Pazartesi

Motto: "Dışarıdaki pozisyonları tek noktadan yönet —
        sinyal üret, karar öner, risk kontrol et."
"""

from flask import Flask, jsonify, request
from flask_cors import CORS
import threading
import time
import sqlite3
import json
from datetime import datetime, timedelta
from typing import Dict, List, Any, Optional

# Platform modülleri
from trading_bot_analysis import (
    MultiTimeframeAnalyzer, CRYPTO_SYMBOLS, BIST_TICKER_MAP,
    COMMODITY_FX, SECTOR_MAP,
)
from macro_data import fetch_all_macro, make_decision, get_sector_trend
from risk_multipliers import (
    calculate_position_risk, reassess_open_position, scan_watchlist,
    check_economic_calendar, calc_drawdown_multiplier, PORTFOLIO_SIZE,
    find_group,
)


# ═══════════════════════════════════════════════════════════════
# FLASK APP
# ═══════════════════════════════════════════════════════════════

app = Flask(__name__)
CORS(app, resources={r"/api/*": {"origins": "*"}})

# ─── Sabitler ───
TCMB_KEY = 'o5OlG3017a'
REFRESH_INTERVAL = 3600     # 1 saat (saniye)
MACRO_INTERVAL = 900        # 15 dakika
DB_PATH = 'riskplatform.db'
VERSION = 'v2.0'

# ─── Haftalık tarama havuzu ───
WEEKLY_SCAN_POOL = {
    'crypto': CRYPTO_SYMBOLS,  # ['BTC/USDT', 'ETH/USDT']
    'bist': list(BIST_TICKER_MAP.keys()),  # 30 hisse
    'commodity_fx': list(COMMODITY_FX.keys()),  # XAU, XAG, USD/TRY, EUR/USD
}

# ─── In-memory cache ───
signals_cache: Dict = {}
signals_lock = threading.Lock()
signals_last_update: Optional[str] = None

macro_cache: Dict = {}
macro_lock = threading.Lock()
macro_last_update: Optional[str] = None

watchlist_cache: List[Dict] = []
watchlist_lock = threading.Lock()

# Rate limiting
rate_limit_store: Dict = {}
rate_limit_lock = threading.Lock()


# ═══════════════════════════════════════════════════════════════
# VERİTABANI
# ═══════════════════════════════════════════════════════════════

def init_db():
    """SQLite veritabanını oluştur/güncelle."""
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()

    # Pozisyonlar (açık + kapalı)
    c.execute('''CREATE TABLE IF NOT EXISTS positions (
        id              INTEGER PRIMARY KEY AUTOINCREMENT,
        symbol          TEXT NOT NULL,
        direction       TEXT NOT NULL DEFAULT 'LONG',
        lot_size        REAL NOT NULL,
        entry_price     REAL NOT NULL,
        initial_stop    REAL NOT NULL,
        current_stop    REAL,
        target_price    REAL,
        risk_pct        REAL DEFAULT 0.5,
        risk_usd        REAL DEFAULT 500,
        signal_type     TEXT,
        trade_type      TEXT DEFAULT 'swing',
        asset_class     TEXT,
        sector          TEXT,
        note            TEXT,
        status          TEXT DEFAULT 'OPEN',
        close_price     REAL,
        close_reason    TEXT,
        pnl_usd         REAL,
        pnl_pct         REAL,
        opened_at       TEXT,
        closed_at       TEXT,
        c1_val          REAL, c2_val REAL, c3_val REAL,
        c5_val          REAL, c6_val REAL, c7_val REAL,
        kelly_base      REAL
    )''')

    # İşlem geçmişi (kapalı işlemler — Kelly hesabı için)
    c.execute('''CREATE TABLE IF NOT EXISTS trade_history (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        symbol      TEXT,
        direction   TEXT,
        pnl_usd     REAL,
        pnl_pct     REAL,
        result      TEXT,
        signal_type TEXT,
        duration_days INTEGER,
        closed_at   TEXT
    )''')

    # Haftalık tarama sonuçları
    c.execute('''CREATE TABLE IF NOT EXISTS weekly_scan (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        symbol      TEXT NOT NULL,
        asset_class TEXT,
        sector      TEXT,
        signal_1d   TEXT,
        direction   TEXT,
        adx         REAL,
        atr_pct     REAL,
        score       REAL,
        scanned_at  TEXT
    )''')

    # Watchlist (aktif izleme listesi)
    c.execute('''CREATE TABLE IF NOT EXISTS watchlist (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        symbol      TEXT NOT NULL UNIQUE,
        asset_class TEXT,
        sector      TEXT,
        source      TEXT DEFAULT 'scan',
        added_at    TEXT
    )''')

    # Aksiyon log (öneriler + kararlar)
    c.execute('''CREATE TABLE IF NOT EXISTS action_log (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        symbol      TEXT,
        action      TEXT,
        priority    TEXT,
        reason      TEXT,
        user_decision TEXT,
        created_at  TEXT
    )''')

    # Sistem log
    c.execute('''CREATE TABLE IF NOT EXISTS logs (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        level      TEXT,
        service    TEXT,
        message    TEXT,
        created_at TEXT
    )''')

    conn.commit()
    conn.close()
    print(f"[DB] SQLite veritabanı hazır: {DB_PATH}")


def db_execute(query: str, params: tuple = (), fetch: bool = False):
    """Güvenli DB işlemi."""
    try:
        conn = sqlite3.connect(DB_PATH)
        conn.row_factory = sqlite3.Row
        c = conn.cursor()
        c.execute(query, params)
        if fetch:
            rows = [dict(r) for r in c.fetchall()]
            conn.close()
            return rows
        conn.commit()
        last_id = c.lastrowid
        conn.close()
        return last_id
    except Exception as e:
        print(f"[DB] Hata: {e}")
        return [] if fetch else None


def db_log(level: str, service: str, message: str):
    db_execute(
        "INSERT INTO logs (level,service,message,created_at) VALUES (?,?,?,?)",
        (level, service, message, datetime.now().isoformat())
    )


# ═══════════════════════════════════════════════════════════════
# PORTFÖY HESAPLAMALARI
# ═══════════════════════════════════════════════════════════════

def get_portfolio_state() -> Dict[str, Any]:
    """Portföy durumunu hesapla."""
    positions = db_execute(
        "SELECT * FROM positions WHERE status='OPEN'", fetch=True
    ) or []

    # Trade history (Kelly için)
    trades = db_execute(
        "SELECT pnl_usd as pnl, result FROM trade_history ORDER BY id",
        fetch=True
    ) or []

    total_risk_pct = sum(float(p.get('risk_pct', 0)) for p in positions)
    total_pnl = sum(float(p.get('pnl_usd', 0) or 0) for p in positions)
    portfolio_value = PORTFOLIO_SIZE + total_pnl

    # Peak hesabı (basit — ileride günlük NAV tablosuyla geliştirilir)
    peak_value = max(PORTFOLIO_SIZE, portfolio_value)

    # Consecutive losses
    consecutive_losses = 0
    for t in reversed(trades):
        if t.get('result') == 'loss':
            consecutive_losses += 1
        else:
            break

    return {
        'portfolio_value': round(portfolio_value, 2),
        'peak_value': round(peak_value, 2),
        'total_risk_pct': round(total_risk_pct, 2),
        'remaining_risk_pct': round(6.0 - total_risk_pct, 2),
        'open_positions': positions,
        'position_count': len(positions),
        'total_pnl': round(total_pnl, 2),
        'trades_history': trades,
        'consecutive_losses': consecutive_losses,
    }


def get_open_positions_for_risk() -> List[Dict]:
    """Risk hesabı formatında açık pozisyonlar."""
    positions = db_execute(
        "SELECT symbol, risk_pct, sector, asset_class FROM positions WHERE status='OPEN'",
        fetch=True
    ) or []
    result = []
    for p in positions:
        group = find_group(p.get('symbol', ''))
        result.append({
            'symbol': p['symbol'],
            'risk_pct': float(p.get('risk_pct', 0)),
            'group': group,
        })
    return result


# ═══════════════════════════════════════════════════════════════
# ARKA PLAN THREAD'LER
# ═══════════════════════════════════════════════════════════════

def refresh_signals():
    """Saatlik sinyal güncelleme — aktif liste (watchlist + açık pozisyonlar)."""
    global signals_last_update
    analyzer = MultiTimeframeAnalyzer()

    while True:
        try:
            # Aktif listeyi belirle: watchlist + açık pozisyonlar
            watchlist_symbols = db_execute(
                "SELECT symbol FROM watchlist", fetch=True
            ) or []
            open_pos_symbols = db_execute(
                "SELECT DISTINCT symbol FROM positions WHERE status='OPEN'", fetch=True
            ) or []

            active_symbols = set()
            for row in watchlist_symbols:
                active_symbols.add(row['symbol'])
            for row in open_pos_symbols:
                active_symbols.add(row['symbol'])

            # Eğer aktif liste boşsa varsayılan kripto
            if not active_symbols:
                active_symbols = set(CRYPTO_SYMBOLS)

            print(f"\n[SIGNAL] Güncelleniyor... {len(active_symbols)} sembol ({datetime.now().strftime('%H:%M:%S')})")

            results = analyzer.scan_all(list(active_symbols))

            with signals_lock:
                signals_cache.clear()
                signals_cache.update(results)
                signals_last_update = datetime.now().isoformat()

            print(f"[SIGNAL] Tamamlandı — {len(results)} sembol")
            db_log('INFO', 'SignalService', f"{len(results)} sembol güncellendi")

        except Exception as e:
            print(f"[SIGNAL] Hata: {e}")
            db_log('ERROR', 'SignalService', str(e))

        time.sleep(REFRESH_INTERVAL)


def refresh_macro():
    """15 dakikada bir makro veri güncelle."""
    global macro_last_update
    while True:
        try:
            print(f"\n[MACRO] Güncelleniyor... ({datetime.now().strftime('%H:%M:%S')})")
            data = fetch_all_macro(tcmb_key=TCMB_KEY)
            decision = make_decision(data)

            with macro_lock:
                macro_cache.clear()
                macro_cache['indices'] = data
                macro_cache['decision'] = decision
                macro_cache['derived'] = data.get('_derived', {})
                macro_last_update = datetime.now().isoformat()

            print(f"[MACRO] Tamamlandı — {len(data)} enstrüman")
            db_log('INFO', 'MacroService', f"{len(data)} enstrüman güncellendi")

        except Exception as e:
            print(f"[MACRO] Hata: {e}")
            db_log('ERROR', 'MacroService', str(e))

        time.sleep(MACRO_INTERVAL)


# ═══════════════════════════════════════════════════════════════
# RATE LIMITER
# ═══════════════════════════════════════════════════════════════

def check_rate_limit(ip: str, limit: int = 200, window: int = 60) -> bool:
    now = time.time()
    with rate_limit_lock:
        if ip not in rate_limit_store:
            rate_limit_store[ip] = []
        rate_limit_store[ip] = [t for t in rate_limit_store[ip] if now - t < window]
        if len(rate_limit_store[ip]) >= limit:
            return False
        rate_limit_store[ip].append(now)
        return True


@app.before_request
def before():
    ip = request.remote_addr or '0.0.0.0'
    if not check_rate_limit(ip):
        return jsonify({'status': 'error', 'message': 'Rate limit (200/dk)'}), 429


# ═══════════════════════════════════════════════════════════════
# ENDPOINTS — SİSTEM
# ═══════════════════════════════════════════════════════════════

@app.route('/api/health')
def health():
    with signals_lock:
        sig_count = len(signals_cache)
    with macro_lock:
        macro_ok = len(macro_cache) > 0
    calendar = check_economic_calendar()
    return jsonify({
        'status': 'ok', 'version': VERSION,
        'signal_count': sig_count, 'macro': macro_ok,
        'signals_updated': signals_last_update,
        'macro_updated': macro_last_update,
        'calendar': calendar,
        'server_time': datetime.now().isoformat(),
    })


@app.route('/api/portfolio')
def get_portfolio():
    """Portföy durumu — KPI'lar."""
    state = get_portfolio_state()
    dd = calc_drawdown_multiplier(
        state['portfolio_value'], state['peak_value'],
        state['consecutive_losses']
    )
    return jsonify({
        'status': 'ok',
        'portfolio_value': state['portfolio_value'],
        'peak_value': state['peak_value'],
        'total_pnl': state['total_pnl'],
        'total_risk_pct': state['total_risk_pct'],
        'remaining_risk_pct': state['remaining_risk_pct'],
        'position_count': state['position_count'],
        'consecutive_losses': state['consecutive_losses'],
        'drawdown': dd,
        'calendar': check_economic_calendar(),
    })


# ═══════════════════════════════════════════════════════════════
# ENDPOINTS — SİNYALLER
# ═══════════════════════════════════════════════════════════════

@app.route('/api/signals')
def get_signals():
    with signals_lock:
        data = dict(signals_cache)
    return jsonify({
        'status': 'ok', 'data': data,
        'count': len(data), 'updated': signals_last_update,
    })


@app.route('/api/signals/<symbol>')
def get_signal_detail(symbol):
    with signals_lock:
        data = signals_cache.get(symbol)
    if not data:
        return jsonify({'status': 'error', 'message': f'{symbol} bulunamadı'}), 404
    return jsonify({'status': 'ok', 'data': data})


@app.route('/api/signals/refresh', methods=['POST'])
def manual_refresh():
    """Manuel sinyal yenileme."""
    threading.Thread(target=_do_refresh, daemon=True).start()
    return jsonify({'status': 'ok', 'message': 'Yenileme başlatıldı'})


def _do_refresh():
    global signals_last_update
    analyzer = MultiTimeframeAnalyzer()
    watchlist = db_execute("SELECT symbol FROM watchlist", fetch=True) or []
    open_pos = db_execute("SELECT DISTINCT symbol FROM positions WHERE status='OPEN'", fetch=True) or []
    symbols = set(r['symbol'] for r in watchlist + open_pos) or set(CRYPTO_SYMBOLS)

    results = analyzer.scan_all(list(symbols))
    with signals_lock:
        signals_cache.clear()
        signals_cache.update(results)
        signals_last_update = datetime.now().isoformat()


# ═══════════════════════════════════════════════════════════════
# ENDPOINTS — MAKRO
# ═══════════════════════════════════════════════════════════════

@app.route('/api/macro')
def get_macro():
    with macro_lock:
        data = dict(macro_cache)
    return jsonify({'status': 'ok', 'data': data, 'updated': macro_last_update})


@app.route('/api/macro/decision')
def get_macro_decision():
    with macro_lock:
        decision = macro_cache.get('decision', [])
        derived = macro_cache.get('derived', {})
    return jsonify({
        'status': 'ok',
        'decision': decision,
        'derived': derived,
        'updated': macro_last_update,
    })


# ═══════════════════════════════════════════════════════════════
# ENDPOINTS — POZİSYON YÖNETİMİ
# ═══════════════════════════════════════════════════════════════

@app.route('/api/positions', methods=['GET'])
def get_positions():
    """Açık pozisyonlar."""
    positions = db_execute(
        "SELECT * FROM positions WHERE status='OPEN' ORDER BY opened_at DESC",
        fetch=True
    ) or []

    # Güncel fiyat ve P&L güncelle
    with signals_lock:
        for pos in positions:
            sym_key = pos['symbol'].replace('/', '_').replace('.', '_')
            sig = signals_cache.get(sym_key)
            if sig:
                current_price = sig.get('price', pos['entry_price'])
                entry = float(pos['entry_price'])
                lot = float(pos['lot_size'])
                mult = 1 if pos['direction'] == 'LONG' else -1
                pnl_usd = (current_price - entry) * lot * mult
                pnl_pct = ((current_price - entry) / entry * 100) * mult
                pos['current_price'] = round(current_price, 6)
                pos['pnl_usd'] = round(pnl_usd, 2)
                pos['pnl_pct'] = round(pnl_pct, 2)
                pos['days_open'] = (datetime.now() - datetime.fromisoformat(pos['opened_at'])).days if pos.get('opened_at') else 0

    return jsonify({'status': 'ok', 'data': positions, 'count': len(positions)})


@app.route('/api/positions', methods=['POST'])
def create_position():
    """Yeni pozisyon aç (dışarıdaki işlemi kaydet)."""
    data = request.get_json()
    if not data:
        return jsonify({'status': 'error', 'message': 'JSON body gerekli'}), 400

    required = ['symbol', 'direction', 'lot_size', 'entry_price', 'initial_stop']
    for field in required:
        if field not in data:
            return jsonify({'status': 'error', 'message': f'{field} eksik'}), 400

    symbol = data['symbol']
    asset_class = data.get('asset_class', 'crypto')
    sector = SECTOR_MAP.get(symbol.replace('.IS', ''), None)

    # Risk hesapla
    state = get_portfolio_state()
    open_positions = get_open_positions_for_risk()

    with macro_lock:
        derived = macro_cache.get('derived', {})

    with signals_lock:
        sym_key = symbol.replace('/', '_').replace('.', '_')
        sig = signals_cache.get(sym_key, {})

    risk_result = calculate_position_risk(
        portfolio_value=state['portfolio_value'],
        peak_value=state['peak_value'],
        open_positions=open_positions,
        candidate={
            'symbol': symbol,
            'signal_status': sig.get('signal_status_1d', 'none'),
            'signal_type': data.get('signal_type', 'swing'),
            'adx': sig.get('adx', 20),
            'adx_prev': sig.get('adx_prev', 20),
            'days_since_signal': sig.get('days_since_signal', 0),
            'price_vs_ema200_pct': sig.get('price_vs_ema200_pct', 0),
        },
        market_data={
            'vix': derived.get('vix_spot', 18),
            'vvix_change_5d': derived.get('vvix_change_5d', 0),
            'dxy_trend': derived.get('dxy_trend', 'FLAT'),
            'vix_spot': derived.get('vix_spot'),
            'vix_futures_m1': derived.get('vix_futures'),
            'us10y_trend': derived.get('us10y_trend'),
            'xu100_trend': derived.get('xu100_trend'),
            'sector_trend': derived.get(f'{sector}_trend') if sector else None,
            'btc_dominance': derived.get('btc_dominance'),
            'avg_daily_volume_usd': data.get('avg_daily_volume_usd', 10_000_000),
            'spread_pct': data.get('spread_pct', 0.05),
        },
        trades_history=state['trades_history'],
    )

    risk_pct = data.get('risk_pct', risk_result.get('risk_pct', 0.5))
    risk_usd = PORTFOLIO_SIZE * risk_pct / 100

    pos_id = db_execute(
        """INSERT INTO positions 
           (symbol, direction, lot_size, entry_price, initial_stop, current_stop,
            target_price, risk_pct, risk_usd, signal_type, trade_type,
            asset_class, sector, note, status, opened_at,
            c1_val, c2_val, c3_val, c5_val, c6_val, c7_val, kelly_base)
           VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)""",
        (
            symbol, data['direction'], data['lot_size'], data['entry_price'],
            data['initial_stop'], data.get('current_stop', data['initial_stop']),
            data.get('target_price'), risk_pct, risk_usd,
            data.get('signal_type', 'manual'), data.get('trade_type', 'swing'),
            asset_class, sector, data.get('note', ''),
            'OPEN', datetime.now().isoformat(),
            risk_result.get('multipliers', {}).get('c1_correlation'),
            risk_result.get('multipliers', {}).get('c2_macro'),
            risk_result.get('multipliers', {}).get('c3_technical'),
            risk_result.get('multipliers', {}).get('c5_drawdown'),
            risk_result.get('multipliers', {}).get('c6_liquidity'),
            risk_result.get('multipliers', {}).get('c7_time'),
            risk_result.get('base'),
        )
    )

    db_log('INFO', 'Position', f"Açıldı: {symbol} {data['direction']} lot={data['lot_size']} @ {data['entry_price']}")

    return jsonify({
        'status': 'ok', 'position_id': pos_id,
        'risk_calculation': risk_result,
    })


@app.route('/api/positions/<int:pos_id>', methods=['PUT'])
def update_position(pos_id):
    """Pozisyon güncelle (stop, hedef, not)."""
    data = request.get_json()
    if not data:
        return jsonify({'status': 'error', 'message': 'JSON body gerekli'}), 400

    updates = []
    params = []
    for field in ['current_stop', 'target_price', 'note', 'risk_pct']:
        if field in data:
            updates.append(f"{field}=?")
            params.append(data[field])

    if not updates:
        return jsonify({'status': 'error', 'message': 'Güncellenecek alan yok'}), 400

    params.append(pos_id)
    db_execute(f"UPDATE positions SET {','.join(updates)} WHERE id=?", tuple(params))
    db_log('INFO', 'Position', f"Güncellendi: #{pos_id} {updates}")

    return jsonify({'status': 'ok', 'message': 'Güncellendi'})


@app.route('/api/positions/<int:pos_id>/close', methods=['POST'])
def close_position(pos_id):
    """Pozisyon kapat."""
    data = request.get_json() or {}
    close_price = data.get('close_price')
    close_reason = data.get('reason', 'manual')

    pos = db_execute("SELECT * FROM positions WHERE id=?", (pos_id,), fetch=True)
    if not pos:
        return jsonify({'status': 'error', 'message': 'Pozisyon bulunamadı'}), 404
    pos = pos[0]

    if not close_price:
        # Güncel fiyattan kapat
        with signals_lock:
            sym_key = pos['symbol'].replace('/', '_').replace('.', '_')
            sig = signals_cache.get(sym_key, {})
            close_price = sig.get('price', pos['entry_price'])

    entry = float(pos['entry_price'])
    lot = float(pos['lot_size'])
    mult = 1 if pos['direction'] == 'LONG' else -1
    pnl_usd = (close_price - entry) * lot * mult
    pnl_pct = ((close_price - entry) / entry * 100) * mult
    result = 'win' if pnl_usd > 0 else 'loss'
    duration = (datetime.now() - datetime.fromisoformat(pos['opened_at'])).days if pos.get('opened_at') else 0

    # Pozisyonu kapat
    db_execute(
        """UPDATE positions SET status='CLOSED', close_price=?, close_reason=?,
           pnl_usd=?, pnl_pct=?, closed_at=? WHERE id=?""",
        (close_price, close_reason, round(pnl_usd, 2), round(pnl_pct, 2),
         datetime.now().isoformat(), pos_id)
    )

    # Trade history'ye kaydet (Kelly için)
    db_execute(
        """INSERT INTO trade_history 
           (symbol, direction, pnl_usd, pnl_pct, result, signal_type, duration_days, closed_at)
           VALUES (?,?,?,?,?,?,?,?)""",
        (pos['symbol'], pos['direction'], round(pnl_usd, 2), round(pnl_pct, 2),
         result, pos.get('signal_type', ''), duration, datetime.now().isoformat())
    )

    db_log('INFO', 'Position', f"Kapatıldı: {pos['symbol']} P&L: ${pnl_usd:.2f} ({result})")

    return jsonify({
        'status': 'ok', 'result': result,
        'pnl_usd': round(pnl_usd, 2), 'pnl_pct': round(pnl_pct, 2),
    })


# ═══════════════════════════════════════════════════════════════
# ENDPOINTS — WATCHLIST
# ═══════════════════════════════════════════════════════════════

@app.route('/api/watchlist', methods=['GET'])
def get_watchlist():
    rows = db_execute("SELECT * FROM watchlist ORDER BY added_at DESC", fetch=True) or []
    return jsonify({'status': 'ok', 'data': rows, 'count': len(rows)})


@app.route('/api/watchlist', methods=['POST'])
def add_to_watchlist():
    data = request.get_json() or {}
    symbol = data.get('symbol')
    if not symbol:
        return jsonify({'status': 'error', 'message': 'symbol gerekli'}), 400

    asset_class = data.get('asset_class', 'crypto')
    sector = SECTOR_MAP.get(symbol.replace('.IS', ''), None)

    db_execute(
        "INSERT OR REPLACE INTO watchlist (symbol, asset_class, sector, source, added_at) VALUES (?,?,?,?,?)",
        (symbol, asset_class, sector, data.get('source', 'manual'), datetime.now().isoformat())
    )
    return jsonify({'status': 'ok', 'message': f'{symbol} watchlist\'e eklendi'})


@app.route('/api/watchlist/<symbol>', methods=['DELETE'])
def remove_from_watchlist(symbol):
    db_execute("DELETE FROM watchlist WHERE symbol=?", (symbol,))
    return jsonify({'status': 'ok', 'message': f'{symbol} kaldırıldı'})


# ═══════════════════════════════════════════════════════════════
# ENDPOINTS — AKSİYON PANELİ
# ═══════════════════════════════════════════════════════════════

@app.route('/api/recommendations')
def get_recommendations():
    """
    Açık pozisyonlar + watchlist için öneriler.
    Dashboard'daki aksiyon panelini besler.
    """
    state = get_portfolio_state()
    recommendations = []

    with macro_lock:
        derived = macro_cache.get('derived', {})

    # ─── Açık pozisyonlar için reassess ───
    for pos in state['open_positions']:
        sym_key = pos['symbol'].replace('/', '_').replace('.', '_')
        with signals_lock:
            sig = signals_cache.get(sym_key, {})

        days_open = (datetime.now() - datetime.fromisoformat(pos['opened_at'])).days if pos.get('opened_at') else 0

        # Güncel fiyat & P&L
        current_price = sig.get('price', float(pos['entry_price']))
        entry = float(pos['entry_price'])
        mult = 1 if pos['direction'] == 'LONG' else -1
        pnl_pct = ((current_price - entry) / entry * 100) * mult

        reass = reassess_open_position(
            position={
                'symbol': pos['symbol'],
                'direction': pos['direction'],
                'days_open': days_open,
                'pnl_pct': pnl_pct,
                'entry_price': entry,
                'current_price': current_price,
                'initial_stop': float(pos.get('initial_stop', 0)),
                'current_stop': float(pos.get('current_stop', 0)),
                'signal_type': pos.get('signal_type', 'swing'),
                'adx': sig.get('adx', 20),
                'adx_trending_up': sig.get('adx', 20) > sig.get('adx_prev', 20),
                'price_above_ema50': current_price > sig.get('ema50', 0) if sig.get('ema50') else True,
                'alpha_trend_aligned': sig.get('direction_1d') != 'conflict',
                'has_counter_signal_4h': bool(sig.get('signals_4h', {}).get('sell_signals' if pos['direction'] == 'LONG' else 'buy_signals', [])),
                'has_counter_signal_1d': sig.get('direction_1d') in ('bearish',) if pos['direction'] == 'LONG' else sig.get('direction_1d') in ('bullish',),
            },
            market_data={'vix': derived.get('vix_spot', 18)},
            portfolio_value=state['portfolio_value'],
            peak_value=state['peak_value'],
            consecutive_losses=state['consecutive_losses'],
        )

        recommendations.append({
            'type': 'position',
            'symbol': pos['symbol'],
            'direction': pos['direction'],
            'days_open': days_open,
            'entry_price': entry,
            'current_price': round(current_price, 6),
            'pnl_pct': round(pnl_pct, 2),
            'pnl_usd': round(pnl_pct / 100 * entry * float(pos['lot_size']) * mult, 2),
            'stop': float(pos.get('current_stop', 0)),
            'action': reass['primary_action'],
            'all_actions': reass['all_actions'],
            'c7': reass['c7']['c7'],
        })

    # ─── Watchlist için risk tarama ───
    watchlist = db_execute("SELECT * FROM watchlist", fetch=True) or []
    open_pos_risk = get_open_positions_for_risk()

    watchlist_candidates = []
    for w in watchlist:
        sym = w['symbol']
        sym_key = sym.replace('/', '_').replace('.', '_')
        with signals_lock:
            sig = signals_cache.get(sym_key, {})

        if not sig:
            continue

        sector = SECTOR_MAP.get(sym.replace('.IS', ''), None)

        watchlist_candidates.append({
            'symbol': sym,
            'signal_status': sig.get('signal_status_1d', 'none'),
            'signal_type': sig.get('signal_4h', 'swing') or 'swing',
            'signal_4h': sig.get('signal_4h'),
            'adx': sig.get('adx', 20),
            'adx_prev': sig.get('adx_prev', 20),
            'days_since_signal': sig.get('days_since_signal', 99),
            'price_vs_ema200_pct': sig.get('price_vs_ema200_pct', 0),
        })

    market_data_for_scan = {
        'vix': derived.get('vix_spot', 18),
        'vvix_change_5d': derived.get('vvix_change_5d', 0),
        'dxy_trend': derived.get('dxy_trend', 'FLAT'),
        'vix_spot': derived.get('vix_spot'),
        'vix_futures_m1': derived.get('vix_futures'),
        'us10y_trend': derived.get('us10y_trend'),
        'xu100_trend': derived.get('xu100_trend'),
        'avg_daily_volume_usd': 10_000_000,
        'spread_pct': 0.05,
    }

    scan_results = scan_watchlist(
        candidates=watchlist_candidates,
        open_positions=open_pos_risk,
        market_data=market_data_for_scan,
        portfolio_value=state['portfolio_value'],
        peak_value=state['peak_value'],
        trades_history=state['trades_history'],
    )

    for r in scan_results:
        cand = r.get('candidate', {})
        sym = cand.get('symbol', '?')
        sym_key = sym.replace('/', '_').replace('.', '_')
        with signals_lock:
            sig = signals_cache.get(sym_key, {})

        recommendations.append({
            'type': 'opportunity',
            'symbol': sym,
            'price': sig.get('price', 0),
            'atr_pct': sig.get('atr_pct', 0),
            'signal_1d': sig.get('signal_status_1d', 'none'),
            'signal_4h': r.get('signal_4h_type'),
            'entry_light': r.get('entry_light', 'BEKLE'),
            'risk_pct': r.get('risk_pct', 0),
            'risk_usd': r.get('risk_usd', 0),
            'status': r.get('status', 'BLOCK'),
            'reason': r.get('reason', ''),
            'multipliers': r.get('multipliers', {}),
        })

    # Sıralama: pozisyon aksiyonları önce (priority HIGH > MEDIUM > LOW), sonra fırsatlar
    priority_order = {'HIGH': 0, 'MEDIUM': 1, 'LOW': 2, 'INFO': 3}

    def sort_reco(r):
        if r['type'] == 'position':
            return (0, priority_order.get(r.get('action', {}).get('priority', 'INFO'), 3))
        else:
            light_score = 0 if r.get('entry_light') == 'AL' else 5
            return (1, light_score, -r.get('risk_pct', 0))

    recommendations.sort(key=sort_reco)

    return jsonify({
        'status': 'ok',
        'recommendations': recommendations,
        'portfolio': {
            'value': state['portfolio_value'],
            'remaining_risk': state['remaining_risk_pct'],
            'drawdown': calc_drawdown_multiplier(
                state['portfolio_value'], state['peak_value'],
                state['consecutive_losses']
            ),
        },
        'calendar': check_economic_calendar(),
        'updated': signals_last_update,
    })


# ═══════════════════════════════════════════════════════════════
# ENDPOINTS — HAFTALIK TARAMA
# ═══════════════════════════════════════════════════════════════

@app.route('/api/weekly-scan', methods=['POST'])
def run_weekly_scan():
    """Haftalık geniş tarama başlat (Pazartesi)."""
    threading.Thread(target=_do_weekly_scan, daemon=True).start()
    return jsonify({'status': 'ok', 'message': 'Haftalık tarama başlatıldı'})


def _do_weekly_scan():
    """Geniş havuzda tarama yap, en güçlü adayları watchlist'e ekle."""
    print(f"\n[SCAN] Haftalık tarama başlıyor... ({datetime.now().strftime('%H:%M:%S')})")
    analyzer = MultiTimeframeAnalyzer()

    all_symbols = []
    all_symbols.extend(WEEKLY_SCAN_POOL['crypto'])
    all_symbols.extend(WEEKLY_SCAN_POOL['bist'])
    all_symbols.extend(WEEKLY_SCAN_POOL['commodity_fx'])

    results = analyzer.scan_all(all_symbols)

    # Skorlama: signal_status + adx + direction
    scored = []
    for key, data in results.items():
        score = 0
        status = data.get('signal_status_1d', 'none')
        if status == 'trend_alpha':
            score += 5
        elif status == 'trend_only':
            score += 4
        elif status == 'alpha_only':
            score += 3
        elif status == 'new_trend':
            score += 2
        elif status == 'ema_only':
            score += 1

        adx = data.get('adx', 0)
        if 25 <= adx <= 35:
            score += 2
        elif adx > 20:
            score += 1

        if data.get('signal_4h'):
            score += 3

        scored.append({**data, 'scan_score': score})

    # Sektör başına en iyi 2 + kripto + emtia
    scored.sort(key=lambda x: x['scan_score'], reverse=True)

    # DB'ye kaydet
    db_execute("DELETE FROM weekly_scan")
    for item in scored:
        db_execute(
            """INSERT INTO weekly_scan 
               (symbol, asset_class, sector, signal_1d, direction, adx, atr_pct, score, scanned_at)
               VALUES (?,?,?,?,?,?,?,?,?)""",
            (item['symbol'], item.get('asset_class'), item.get('sector'),
             item.get('signal_status_1d'), item.get('direction_1d'),
             item.get('adx', 0), item.get('atr_pct', 0),
             item.get('scan_score', 0), datetime.now().isoformat())
        )

    # En iyi 12'yi watchlist'e ekle (sektör başına max 2)
    sector_counts = {}
    selected = []
    for item in scored:
        sector = item.get('sector') or item.get('asset_class', 'other')
        if sector_counts.get(sector, 0) >= 2:
            continue
        if item['scan_score'] >= 2:
            selected.append(item)
            sector_counts[sector] = sector_counts.get(sector, 0) + 1
        if len(selected) >= 12:
            break

    # Watchlist güncelle
    db_execute("DELETE FROM watchlist WHERE source='scan'")
    for item in selected:
        db_execute(
            "INSERT OR REPLACE INTO watchlist (symbol, asset_class, sector, source, added_at) VALUES (?,?,?,?,?)",
            (item['symbol'], item.get('asset_class'), item.get('sector'),
             'scan', datetime.now().isoformat())
        )

    print(f"[SCAN] Tamamlandı — {len(results)} tarandı, {len(selected)} watchlist'e eklendi")
    db_log('INFO', 'WeeklyScan', f"{len(results)} tarandı, {len(selected)} seçildi")


@app.route('/api/weekly-scan', methods=['GET'])
def get_weekly_scan():
    """Son haftalık tarama sonuçları."""
    rows = db_execute(
        "SELECT * FROM weekly_scan ORDER BY score DESC", fetch=True
    ) or []
    return jsonify({'status': 'ok', 'data': rows, 'count': len(rows)})


# ═══════════════════════════════════════════════════════════════
# ENDPOINTS — GEÇMİŞ
# ═══════════════════════════════════════════════════════════════

@app.route('/api/history')
def get_trade_history():
    limit = request.args.get('limit', 50, type=int)
    rows = db_execute(
        "SELECT * FROM trade_history ORDER BY id DESC LIMIT ?",
        (limit,), fetch=True
    ) or []
    return jsonify({'status': 'ok', 'data': rows, 'count': len(rows)})


@app.route('/api/history/stats')
def get_trade_stats():
    """Kelly için istatistikler."""
    trades = db_execute(
        "SELECT pnl_usd as pnl, result FROM trade_history ORDER BY id",
        fetch=True
    ) or []
    if not trades:
        return jsonify({'status': 'ok', 'total': 0, 'win_rate': 0, 'avg_rr': 0})

    wins = [t for t in trades if t['result'] == 'win']
    losses = [t for t in trades if t['result'] == 'loss']
    win_rate = len(wins) / len(trades) if trades else 0
    avg_win = sum(abs(t['pnl']) for t in wins) / len(wins) if wins else 0
    avg_loss = sum(abs(t['pnl']) for t in losses) / len(losses) if losses else 1

    return jsonify({
        'status': 'ok',
        'total': len(trades),
        'wins': len(wins),
        'losses': len(losses),
        'win_rate': round(win_rate, 4),
        'avg_win': round(avg_win, 2),
        'avg_loss': round(avg_loss, 2),
        'avg_rr': round(avg_win / avg_loss, 2) if avg_loss > 0 else 0,
    })


@app.route('/api/action-log')
def get_action_log():
    limit = request.args.get('limit', 30, type=int)
    rows = db_execute(
        "SELECT * FROM action_log ORDER BY id DESC LIMIT ?",
        (limit,), fetch=True
    ) or []
    return jsonify({'status': 'ok', 'data': rows})


@app.route('/api/action-log', methods=['POST'])
def log_action():
    """Kullanıcı kararını kaydet (Uygula/Reddet/Ertele)."""
    data = request.get_json() or {}
    db_execute(
        "INSERT INTO action_log (symbol, action, priority, reason, user_decision, created_at) VALUES (?,?,?,?,?,?)",
        (data.get('symbol'), data.get('action'), data.get('priority'),
         data.get('reason'), data.get('decision'), datetime.now().isoformat())
    )
    return jsonify({'status': 'ok'})


# ═══════════════════════════════════════════════════════════════
# ENDPOINTS — EKONOMİK TAKVİM
# ═══════════════════════════════════════════════════════════════

@app.route('/api/calendar')
def get_calendar():
    return jsonify({'status': 'ok', 'data': check_economic_calendar()})


# ═══════════════════════════════════════════════════════════════
# BAŞLATMA
# ═══════════════════════════════════════════════════════════════

if __name__ == '__main__':
    print(f"[API] RiskPlatform {VERSION} başlatılıyor...")
    print(f"[API] Portföy: ${PORTFOLIO_SIZE:,.0f}")
    print(f"[API] Refresh: {REFRESH_INTERVAL}s sinyal, {MACRO_INTERVAL}s makro")

    init_db()

    # Arka plan thread'leri
    threading.Thread(target=refresh_signals, daemon=True).start()
    threading.Thread(target=refresh_macro, daemon=True).start()

    print(f"[API] http://localhost:5000")
    print(f"[API] Endpoints:")
    print(f"  /api/health          — Sistem durumu")
    print(f"  /api/portfolio       — Portföy KPI")
    print(f"  /api/signals         — Sinyaller")
    print(f"  /api/macro           — Makro veri")
    print(f"  /api/macro/decision  — Makro kararlar")
    print(f"  /api/positions       — Pozisyonlar (GET/POST)")
    print(f"  /api/positions/ID    — Güncelle (PUT)")
    print(f"  /api/positions/ID/close — Kapat (POST)")
    print(f"  /api/watchlist       — İzleme listesi")
    print(f"  /api/recommendations — Aksiyon paneli")
    print(f"  /api/weekly-scan     — Haftalık tarama")
    print(f"  /api/history         — İşlem geçmişi")
    print(f"  /api/calendar        — Ekonomik takvim")

    app.run(host='0.0.0.0', port=5000, debug=False)
