import { ToastProvider } from './components/Toast.jsx';
import { useWebSocket } from './hooks/useWebSocket.js';
import { AnalyticsTab } from './components/AnalyticsTab.jsx';
import { useState, useEffect, useCallback, useRef } from "react";
import {
  LineChart, Line, AreaChart, Area, BarChart, Bar,
  XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid, ReferenceLine
} from "recharts";

// ─────────────────────────────────────────────────────────
// API HELPERS
// ─────────────────────────────────────────────────────────
const API = "http://localhost:5000/api";
async function apiFetch(path) {
  try { const r = await fetch(`${API}${path}`); return r.ok ? await r.json() : null; } catch { return null; }
}
async function apiPost(path, body) {
  try { const r = await fetch(`${API}${path}`, { method:"POST", headers:{"Content-Type":"application/json"}, body:JSON.stringify(body) }); return await r.json(); } catch { return null; }
}
async function apiPut(path, body) {
  try { const r = await fetch(`${API}${path}`, { method:"PUT", headers:{"Content-Type":"application/json"}, body:JSON.stringify(body) }); return await r.json(); } catch { return null; }
}
async function apiDelete(path) {
  try { const r = await fetch(`${API}${path}`, { method:"DELETE" }); return await r.json(); } catch { return null; }
}

// ─────────────────────────────────────────────────────────
// SYMBOL DATABASE — 90+ BIST + Kripto + Emtia + Faiz
// ─────────────────────────────────────────────────────────
const SYMBOL_DB = {
  BIST: {
    "Bankacılık": [
      { code:"AKBNK", name:"Akbank",             base:43.20 },
      { code:"GARAN", name:"Garanti BBVA",        base:110.40 },
      { code:"ISCTR", name:"İş Bankası C",        base:12.85 },
      { code:"YKBNK", name:"Yapı Kredi",          base:35.60 },
      { code:"HALKB", name:"Halkbank",            base:22.30 },
      { code:"VAKBN", name:"VakıfBank",           base:18.70 },
      { code:"TSKB",  name:"TSKB",                base:8.24  },
      { code:"ALBRK", name:"Albaraka Türk",       base:7.15  },
    ],
    "Holding": [
      { code:"KCHOL", name:"Koç Holding",         base:210.50 },
      { code:"SAHOL", name:"Sabancı Holding",     base:87.30  },
      { code:"SISE",  name:"Şişecam",             base:42.10  },
      { code:"TKFEN", name:"Tekfen Holding",      base:68.90  },
      { code:"DOHOL", name:"Doğan Holding",       base:6.82   },
      { code:"AGHOL", name:"AG Anadolu Grubu",    base:145.20 },
    ],
    "Teknoloji": [
      { code:"ASELS", name:"Aselsan",             base:98.70  },
      { code:"TUPRS", name:"Tüpraş",              base:198.30 },
      { code:"LOGO",  name:"Logo Yazılım",        base:267.40 },
      { code:"NETAS", name:"Netaş",               base:34.80  },
      { code:"INDES", name:"İndeks Bilgisayar",   base:82.60  },
      { code:"ESCOM", name:"Escort Teknoloji",    base:48.30  },
      { code:"DENGE", name:"Denge Yatırım",       base:12.40  },
    ],
    "Enerji": [
      { code:"ENKAI", name:"Enka İnşaat",         base:54.20  },
      { code:"AKSEN", name:"Aksa Enerji",         base:32.70  },
      { code:"ZOREN", name:"Zorlu Enerji",        base:7.82   },
      { code:"AYEN",  name:"Ayen Enerji",         base:16.45  },
      { code:"ODAS",  name:"Odaş Elektrik",       base:28.90  },
      { code:"ANEL",  name:"Anel Elektrik",       base:21.30  },
    ],
    "Perakende": [
      { code:"BIMAS", name:"BİM Mağazalar",       base:598.00 },
      { code:"MIGROS",name:"Migros",              base:312.50 },
      { code:"MGROS", name:"Migros Ticaret",      base:308.20 },
      { code:"SOKM",  name:"Şok Marketler",       base:87.40  },
      { code:"CRFSA", name:"CarrefourSA",         base:62.10  },
    ],
    "İnşaat-GYO": [
      { code:"EMLAK", name:"Emlak Konut GYO",     base:9.34   },
      { code:"TOASO", name:"Tofaş Oto Fab.",      base:312.40 },
      { code:"TTKOM", name:"Türk Telekom",        base:24.80  },
      { code:"TCELL", name:"Turkcell",            base:67.20  },
      { code:"ALKIM", name:"Alkim Kimya",         base:378.00 },
      { code:"EKGYO", name:"Emlak GYO",           base:9.42   },
      { code:"ISGYO", name:"İş GYO",              base:12.30  },
    ],
    "Sanayi": [
      { code:"EREGL", name:"Ereğli Demir Çelik",  base:43.20  },
      { code:"KRDMD", name:"Kardemir D",          base:8.76   },
      { code:"ARCLK", name:"Arçelik",             base:178.40 },
      { code:"VESBE", name:"Vestel Beyaz Eşya",   base:87.60  },
      { code:"FROTO", name:"Ford Otosan",         base:1024.00},
      { code:"OTKAR", name:"Otokar",              base:3120.00},
      { code:"TRKCM", name:"Trakya Cam",          base:36.40  },
      { code:"OYAKC", name:"Oyak Çimento",        base:28.70  },
    ],
    "Finans-Sigorta": [
      { code:"THYAO", name:"Türk Hava Yolları",   base:312.00 },
      { code:"TAVHL", name:"TAV Havalimanları",   base:178.90 },
      { code:"MAVI",  name:"Mavi Giyim",          base:245.60 },
      { code:"PGSUS", name:"Pegasus",             base:412.30 },
      { code:"VESTL", name:"Vestel",              base:38.90  },
      { code:"TTRAK", name:"Türk Traktör",        base:2840.00},
    ],
  },
  KRIPTO: {
    "Büyük Kap": [
      { code:"BTC/USDT",  name:"Bitcoin",          base:68000 },
      { code:"ETH/USDT",  name:"Ethereum",         base:3200  },
      { code:"BNB/USDT",  name:"BNB",              base:580   },
      { code:"SOL/USDT",  name:"Solana",           base:145   },
      { code:"XRP/USDT",  name:"Ripple",           base:0.62  },
    ],
    "Orta Kap": [
      { code:"ADA/USDT",  name:"Cardano",          base:0.52  },
      { code:"AVAX/USDT", name:"Avalanche",        base:38.50 },
      { code:"DOT/USDT",  name:"Polkadot",         base:8.20  },
      { code:"LINK/USDT", name:"Chainlink",        base:18.40 },
      { code:"MATIC/USDT",name:"Polygon",          base:0.92  },
      { code:"ATOM/USDT", name:"Cosmos",           base:9.80  },
      { code:"FTM/USDT",  name:"Fantom",           base:0.78  },
    ],
    "DeFi": [
      { code:"UNI/USDT",  name:"Uniswap",          base:10.20 },
      { code:"AAVE/USDT", name:"Aave",             base:189.00},
      { code:"CRV/USDT",  name:"Curve",            base:0.58  },
      { code:"SNX/USDT",  name:"Synthetix",        base:2.84  },
    ],
  },
  EMTIA: {
    "Kıymetli Metaller": [
      { code:"XAU/USD",  name:"Altın (Gold)",      base:2180  },
      { code:"XAG/USD",  name:"Gümüş (Silver)",    base:24.80 },
      { code:"XPT/USD",  name:"Platin",            base:920   },
      { code:"XPD/USD",  name:"Paladyum",          base:980   },
    ],
    "Enerji": [
      { code:"CL/USD",   name:"Ham Petrol (WTI)",  base:82.40 },
      { code:"BR/USD",   name:"Brent Petrol",      base:87.60 },
      { code:"NG/USD",   name:"Doğal Gaz",         base:2.18  },
    ],
    "Tarım": [
      { code:"W/USD",    name:"Buğday",            base:545   },
      { code:"CORN/USD", name:"Mısır",             base:432   },
    ],
  },
  FAIZ: {
    "Tahvil": [
      { code:"US10Y",    name:"ABD 10Y Tahvil",    base:4.13  },
      { code:"US2Y",     name:"ABD 2Y Tahvil",     base:4.68  },
      { code:"TR10Y",    name:"TR 10Y Tahvil",     base:28.50 },
      { code:"TLREF",    name:"TLREF (O/N)",       base:42.00 },
      { code:"EURIBOR",  name:"Euribor 3M",        base:3.92  },
    ],
    "Döviz": [
      { code:"USD/TRY",  name:"Dolar/TL",          base:32.80 },
      { code:"EUR/TRY",  name:"Euro/TL",           base:35.60 },
      { code:"EUR/USD",  name:"EUR/USD",           base:1.085 },
      { code:"GBP/USD",  name:"GBP/USD",           base:1.268 },
    ],
  },
};

const INDEX_DB = {
  BIST100:  { name:"BIST 100",  base:10240 },
  BIST30:   { name:"BIST 30",   base:13850 },
  SP500:    { name:"S&P 500",   base:5240  },
  NASDAQ:   { name:"NASDAQ",    base:16420 },
  DJI:      { name:"DOW JONES", base:39280 },
  DAX:      { name:"DAX",       base:18340 },
  VIX:      { name:"VIX",       base:18.50 },
  DXY:      { name:"DXY",       base:104.2 },
  GOLD:     { name:"Altın",     base:2180  },
};

// ─────────────────────────────────────────────────────────
// RISK ENGINE
// ─────────────────────────────────────────────────────────
class RiskEngine {
  constructor(config = {}) {
    this.portfolioSize    = config.portfolioSize   || 100000;
    this.maxPortfolioRisk = config.maxPortfolioRisk || 6;   // %
    this.maxTradeRisk     = config.maxTradeRisk     || 2;   // %
    this.maxMonthlyLoss   = config.maxMonthlyLoss   || 2;   // %
    this.riskPerTrade     = config.riskPerTrade     || 0.5; // %
  }

  calcATR(prices, period = 14) {
    if (!prices || prices.length < period + 1) return 0;
    const trs = prices.slice(1).map((p, i) => {
      const prev = prices[i];
      return Math.max(p.high - p.low, Math.abs(p.high - prev.close), Math.abs(p.low - prev.close));
    });
    return trs.slice(-period).reduce((a, b) => a + b, 0) / period;
  }

  calcPositionSize(price, atr, vix = 18) {
    const dynamicMult  = 2.0 * (1 + (vix - 15) / 30);
    const stopDist     = atr * dynamicMult;
    const riskDollars  = Math.min(Math.max(this.portfolioSize * (this.riskPerTrade / 100), 10000), 50000);
    const units        = riskDollars / stopDist;
    const positionSize = units * price;

    const maxWeightPct = vix < 15 ? 8 : vix < 20 ? 4 : vix < 30 ? 2 : 0;
    const maxPosition  = this.portfolioSize * (maxWeightPct / 100);
    const finalSize    = Math.min(positionSize, maxPosition);

    return {
      units:        Math.floor(units),
      positionSize: Math.round(finalSize),
      stopDistance: Math.round(stopDist * 100) / 100,
      riskDollars:  Math.round(riskDollars),
      stopPrice:    Math.round((price - stopDist) * 100) / 100,
      targetPrice:  Math.round((price + stopDist * 2) * 100) / 100,
      rr:           2.0,
      maxWeightPct,
      portfolioPct: Math.round(finalSize / this.portfolioSize * 1000) / 10,
      atr,
      atrPct:       Math.round(atr / price * 10000) / 100,
    };
  }

  calcKelly(winRate, avgWin, avgLoss) {
    if (winRate <= 0.52 || avgLoss === 0) return { valid: false, kelly: 0, reason: 'p<0.52' };
    const b = avgWin / avgLoss;
    if (b < 1.3) return { valid: false, kelly: 0, reason: 'b<1.3' };
    const p = winRate, q = 1 - p;
    const fullKelly   = (p * b - q) / b;
    const quarterKelly = fullKelly / 4;
    return {
      valid: true, p, b,
      fullKelly:    Math.round(fullKelly * 1000) / 10,
      quarterKelly: Math.round(quarterKelly * 1000) / 10,
      kellySizeUSD: Math.round(this.portfolioSize * quarterKelly / 100),
    };
  }

  calcVaR(returns, confidence = 0.95) {
    if (!returns || returns.length < 10) return 0;
    const sorted = [...returns].sort((a, b) => a - b);
    const idx    = Math.floor((1 - confidence) * sorted.length);
    return Math.abs(sorted[idx] || 0);
  }

  calcSharpe(returns, rf = 0.05) {
    if (!returns || returns.length < 2) return 0;
    const mean = returns.reduce((a, b) => a + b, 0) / returns.length;
    const std  = Math.sqrt(returns.reduce((a, b) => a + (b - mean) ** 2, 0) / returns.length);
    return std === 0 ? 0 : ((mean * 252 - rf) / (std * Math.sqrt(252)));
  }

  calcDrawdown(navHistory) {
    if (!navHistory || navHistory.length < 2) return { current: 0, max: 0 };
    let peak = navHistory[0], maxDD = 0;
    for (const val of navHistory) {
      if (val > peak) peak = val;
      const dd = (val - peak) / peak * 100;
      if (dd < maxDD) maxDD = dd;
    }
    const current = (navHistory[navHistory.length-1] - peak) / peak * 100;
    return { current: Math.round(current * 100) / 100, max: Math.round(maxDD * 100) / 100 };
  }

  analyzeIndexImpact(indexChg, positionDirection) {
    if (Math.abs(indexChg) < 0.5) return 'NEUTRAL';
    if ((indexChg > 0 && positionDirection === 'LONG') ||
        (indexChg < 0 && positionDirection === 'SHORT')) return 'POSITIVE';
    return 'NEGATIVE';
  }

  generateIndexRecommendation(indices) {
    const vix   = indices.VIX   || 18;
    const bist  = indices.BIST100 || 0;
    const sp500 = indices.SP500   || 0;
    if (vix > 30) return 'CRISIS: Tüm pozisyonları kapat, altın/cash';
    if (vix > 20) return 'DEFANSIF: Pozisyon boyutlarını yarıya indir';
    if (bist > 0 && sp500 > 0) return 'RISK-ON: Normal alokasyon, seçici pozisyon';
    return 'IZLE: Piyasa yönü belirsiz';
  }
}

// ─────────────────────────────────────────────────────────
// PRICE HOOK — API + Fallback Simülasyon
// ─────────────────────────────────────────────────────────
function usePrices(symbolMap) {
  const [prices, setPrices] = useState(() => {
    const init = {};
    Object.entries(symbolMap).forEach(([k, v]) => {
      init[k] = { price: v.base, prev: v.base, change: 0, changePct: 0 };
    });
    return init;
  });
  const [apiConnected, setApiConnected] = useState(false);

  // API'den sinyal verilerini kullanarak fiyatları güncelle
  useEffect(() => {
    let isMounted = true;
    
    const fetchPrices = async () => {
      try {
        const signalsRes = await fetch('http://localhost:5000/api/signals').catch(() => null);
        
        if (signalsRes?.ok && isMounted) {
          const signalsData = await signalsRes.json();
          const updates = {};
          
          Object.values(signalsData.data || {}).forEach(signal => {
            const originalKey = signal.symbol || signal.symbol_key?.replace('_', '/');
            
            if (originalKey && signal.price) {
              const baseSymbol = symbolMap[originalKey];
              const base = baseSymbol?.base || signal.price;
              
              updates[originalKey] = {
                price: signal.price,
                prev: prices[originalKey]?.price || base,
                change: signal.price - base,
                changePct: ((signal.price - base) / base * 100) || 0
              };
            }
          });
          
          if (Object.keys(updates).length > 0) {
            setPrices(prev => ({ ...prev, ...updates }));
            setApiConnected(true);
          }
        } else if (isMounted) {
          setApiConnected(false);
        }
      } catch (error) {
        if (isMounted) {
          console.log('API bağlantı hatası:', error);
          setApiConnected(false);
        }
      }
    };
    
    fetchPrices();
    const id = setInterval(fetchPrices, 10000);
    
    return () => {
      isMounted = false;
      clearInterval(id);
    };
  }, []);

  // Fallback: simüle fiyatlar (API bağlı değilse)
  useEffect(() => {
    if (apiConnected) return;
    const id = setInterval(() => {
      setPrices(prev => {
        const next = { ...prev };
        Object.keys(next).forEach(k => {
          const { price } = next[k];
          const delta = price * (Math.random() - 0.5) * 0.016;
          const np    = Math.max(price + delta, 0.0001);
          const base  = symbolMap[k]?.base || price;
          next[k] = {
            price:     Math.round(np * 10000) / 10000,
            prev:      price,
            change:    Math.round((np - base) * 10000) / 10000,
            changePct: Math.round((np - base) / base * 10000) / 100,
          };
        });
        return next;
      });
    }, 4000);
    return () => clearInterval(id);
  }, [apiConnected]);

  return prices;
}

// ─────────────────────────────────────────────────────────
// HELPERS
// ─────────────────────────────────────────────────────────
const fmt = (n, dec = 2) => {
  if (n === null || n === undefined || isNaN(n)) return '—';
  if (Math.abs(n) >= 1e9) return (n / 1e9).toFixed(2) + 'B';
  if (Math.abs(n) >= 1e6) return (n / 1e6).toFixed(2) + 'M';
  if (Math.abs(n) >= 1000) return n.toLocaleString('tr-TR', { minimumFractionDigits: dec, maximumFractionDigits: dec });
  return Number(n).toFixed(dec);
};

const fmtPct = (n, dec = 2) => {
  if (n === null || n === undefined || isNaN(n)) return '—';
  return (n >= 0 ? '+' : '') + Number(n).toFixed(dec) + '%';
};

const pctColor = (v) => v >= 0 ? '#2ea043' : '#da3633';

function generatePriceHistory(base, points = 50) {
  const data = [];
  let price = base * 0.9;
  for (let i = 0; i < points; i++) {
    price *= 1 + (Math.random() - 0.48) * 0.025;
    data.push({ t: i, price: Math.round(price * 100) / 100 });
  }
  return data;
}

// ─────────────────────────────────────────────────────────
// STYLES
// ─────────────────────────────────────────────────────────
const S = {
  app: {
    minHeight: '100vh', background: '#0d1117', color: '#c9d1d9',
    fontFamily: '"Segoe UI", system-ui, -apple-system, sans-serif',
    fontSize: 13,
  },
  header: {
    background: '#161b22', borderBottom: '1px solid #30363d',
    padding: '0 20px', height: 52, display: 'flex', alignItems: 'center',
    justifyContent: 'space-between', position: 'sticky', top: 0, zIndex: 100,
  },
  headerLeft: { display: 'flex', alignItems: 'center', gap: 16 },
  logo: { fontSize: 16, fontWeight: 700, color: '#58a6ff', letterSpacing: 1 },
  logoSub: { fontSize: 10, color: '#6e7681', letterSpacing: 2, marginTop: 1 },
  liveTag: {
    fontSize: 9, padding: '2px 8px', borderRadius: 20,
    background: '#0d2a12', color: '#3fb950', border: '1px solid #1a4a1a',
    letterSpacing: 1.5, fontWeight: 700,
    animation: 'none',
  },
  headerRight: { display: 'flex', alignItems: 'center', gap: 12 },
  headerTime: { fontSize: 11, color: '#6e7681', letterSpacing: 1, fontFamily: 'monospace' },
  nav: {
    background: '#161b22', borderBottom: '1px solid #21262d',
    display: 'flex', padding: '0 20px', gap: 2, overflowX: 'auto',
  },
  navBtn: (active) => ({
    padding: '10px 16px', background: 'transparent',
    border: 'none', cursor: 'pointer', fontSize: 12, fontWeight: 500,
    color: active ? '#f0f6fc' : '#8b949e',
    borderBottom: active ? '2px solid #58a6ff' : '2px solid transparent',
    whiteSpace: 'nowrap', transition: 'all 0.15s',
  }),
  content: { padding: 20, maxWidth: 1800, margin: '0 auto' },
  grid: (cols) => ({
    display: 'grid', gridTemplateColumns: `repeat(${cols}, 1fr)`, gap: 16,
  }),
  card: {
    background: '#161b22', border: '1px solid #21262d', borderRadius: 8,
    padding: 16, position: 'relative', overflow: 'hidden',
  },
  cardTitle: {
    fontSize: 11, fontWeight: 600, color: '#8b949e',
    letterSpacing: 1.5, textTransform: 'uppercase', marginBottom: 12,
  },
  kpiVal: { fontSize: 26, fontWeight: 700, color: '#f0f6fc', lineHeight: 1 },
  kpiSub: { fontSize: 11, color: '#6e7681', marginTop: 4 },
  table: { width: '100%', borderCollapse: 'collapse' },
  th: {
    padding: '8px 12px', background: '#0d1117', fontSize: 10,
    color: '#6e7681', fontWeight: 600, textAlign: 'left',
    letterSpacing: 1, textTransform: 'uppercase', borderBottom: '1px solid #21262d',
  },
  td: {
    padding: '10px 12px', borderBottom: '1px solid #161b22',
    fontSize: 12, verticalAlign: 'middle',
  },
  badge: (color) => ({
    display: 'inline-block', padding: '2px 8px', borderRadius: 4,
    fontSize: 10, fontWeight: 700, letterSpacing: 1,
    background: color + '22', color, border: `1px solid ${color}44`,
  }),
  btn: (variant = 'primary') => ({
    padding: '7px 16px', borderRadius: 6, border: 'none', cursor: 'pointer',
    fontSize: 12, fontWeight: 600, letterSpacing: 0.5, transition: 'all 0.15s',
    background: variant === 'primary' ? '#238636' : variant === 'danger' ? '#da3633' : variant === 'blue' ? '#1f6feb' : '#21262d',
    color: '#fff',
  }),
  input: {
    background: '#0d1117', border: '1px solid #30363d', borderRadius: 6,
    color: '#c9d1d9', padding: '7px 12px', fontSize: 12, outline: 'none',
    width: '100%',
  },
  select: {
    background: '#0d1117', border: '1px solid #30363d', borderRadius: 6,
    color: '#c9d1d9', padding: '7px 12px', fontSize: 12, outline: 'none',
    cursor: 'pointer', width: '100%',
  },
  label: { fontSize: 11, color: '#8b949e', marginBottom: 4, display: 'block' },
  row: { display: 'flex', gap: 12, alignItems: 'flex-end', flexWrap: 'wrap' },
  section: { marginBottom: 24 },
  sectionTitle: {
    fontSize: 13, fontWeight: 700, color: '#c9d1d9',
    marginBottom: 12, paddingBottom: 8, borderBottom: '1px solid #21262d',
  },
  divider: { height: 1, background: '#21262d', margin: '16px 0' },
  tooltip: {
    background: '#161b22', border: '1px solid #30363d',
    borderRadius: 6, padding: '8px 12px', fontSize: 11,
  },
};

// ─────────────────────────────────────────────────────────
// TICKER BAR
// ─────────────────────────────────────────────────────────
function TickerBar({ prices }) {
  const items = [
    { key: 'BTC/USDT', label: 'BTC' },
    { key: 'ETH/USDT', label: 'ETH' },
    { key: 'SOL/USDT', label: 'SOL' },
    { key: 'XAU/USD',  label: 'ALTIN' },
    { key: 'USD/TRY',  label: 'USD/TRY' },
    { key: 'BIST100',  label: 'BIST100' },
    { key: 'VIX',      label: 'VIX' },
    { key: 'US10Y',    label: 'US10Y' },
  ];

  return (
    <div style={{
      background: '#010409', borderBottom: '1px solid #21262d',
      padding: '4px 20px', display: 'flex', gap: 24, overflowX: 'auto',
      alignItems: 'center',
    }}>
      {items.map(({ key, label }) => {
        const p = prices[key];
        if (!p) return null;
        const up = p.changePct >= 0;
        return (
          <div key={key} style={{ display: 'flex', gap: 6, alignItems: 'center', flexShrink: 0 }}>
            <span style={{ fontSize: 9, color: '#6e7681', letterSpacing: 1 }}>{label}</span>
            <span style={{ fontSize: 11, color: '#f0f6fc', fontFamily: 'monospace', fontWeight: 600 }}>
              {fmt(p.price)}
            </span>
            <span style={{ fontSize: 10, color: pctColor(p.changePct), fontFamily: 'monospace' }}>
              {fmtPct(p.changePct)}
            </span>
          </div>
        );
      })}
    </div>
  );
}

// ─────────────────────────────────────────────────────────
// DASHBOARD TAB
// ─────────────────────────────────────────────────────────
function DashboardTab({ prices, engine }) {
  const [signals, setSignals] = useState(null);
  const [macro, setMacro] = useState(null);
  const [apiOk, setApiOk] = useState(false);
  const [lastFetch, setLastFetch] = useState(null);

  const fetchData = useCallback(async () => {
    try {
      // MTF Sinyalleri
      const sigRes = await fetch('http://localhost:5000/api/signals').catch(() => null);
      if (sigRes?.ok) {
        const sigData = await sigRes.json();
        setSignals(sigData);
        setApiOk(true);
      }
      
      // Makro veriler
      const macRes = await fetch('http://localhost:5000/api/macro').catch(() => null);
      if (macRes?.ok) {
        const macData = await macRes.json();
        setMacro(macData);
      }
      
      setLastFetch(new Date().toLocaleTimeString('tr-TR'));
    } catch (error) {
      console.log('Veri çekme hatası:', error);
    }
  }, []);

  useEffect(() => { 
    fetchData(); 
    const id = setInterval(fetchData, 30000); 
    return () => clearInterval(id); 
  }, [fetchData]);

  // Portfolio KPIs from API
  const [portfolio, setPortfolio] = useState(null);
  const [recos, setRecos] = useState(null);
  useEffect(() => {
    const fetchPortfolio = async () => {
      const [p, r] = await Promise.all([apiFetch('/portfolio'), apiFetch('/recommendations')]);
      if (p) setPortfolio(p);
      if (r) setRecos(r);
    };
    fetchPortfolio();
    const pid = setInterval(fetchPortfolio, 60000);
    return () => clearInterval(pid);
  }, []);

  const port = portfolio || {};
  const dd = port.drawdown || {};
  const posRecos = (recos?.recommendations || []).filter(r => r.type === 'position');

  // KPI cards
  const kpiCards = [
    { label: 'PORTFÖY', value: port.portfolio_value ? '$'+fmt(port.portfolio_value) : '$'+fmt(engine.portfolioSize), sub: port.peak_value ? `Peak: $${fmt(port.peak_value)}` : 'USD toplam değer', color: '#58a6ff' },
    { label: 'GÜNLÜK P&L', value: port.total_pnl != null ? (port.total_pnl>=0?'+':'')+`$${fmt(port.total_pnl)}` : fmtPct(0), sub: `${port.position_count || 0} açık pozisyon`, color: pctColor(port.total_pnl || 0) },
    { label: 'KALAN RİSK', value: port.remaining_risk_pct != null ? fmtPct(port.remaining_risk_pct) : '%6.00', sub: `Kullanılan: %${fmt(port.total_risk_pct || 0)}`, color: (port.remaining_risk_pct || 6) > 2 ? '#3fb950' : '#e3b341' },
    { label: 'DRAWDOWN', value: dd.dd_pct != null ? fmtPct(-dd.dd_pct) : '-0.0%', sub: `Streak: ${port.consecutive_losses || 0} kayıp`, color: (dd.dd_pct||0) < 5 ? '#3fb950' : '#f85149' },
    { label: 'Ç5 DURUM', value: dd.severity || 'NORMAL', sub: `Çarpan: ×${dd.c5 || 1.0}`, color: dd.severity === 'CRITICAL' ? '#f85149' : dd.severity === 'WARNING' ? '#e3b341' : '#3fb950' },
    { label: 'WIN RATE', value: port.consecutive_losses != null ? `${port.position_count || 0} poz` : '—', sub: 'Aktif pozisyonlar', color: '#3fb950' },
  ];

  const btcHist = generatePriceHistory(prices['BTC/USDT']?.price || 68000, 40);

  // Macro indices
  const macroIndices = macro?.data?.indices || {};
  const vixVal = macroIndices.VIX?.value || 0;
  const vixRegime = vixVal > 30 ? 'CRISIS' : vixVal > 20 ? 'ELEVATED' : vixVal > 15 ? 'NORMAL' : 'LOW';
  const vixColor  = vixVal > 30 ? '#f85149' : vixVal > 20 ? '#e3b341' : '#3fb950';

  return (
    <div>
      {/* KPI Row */}
      <div style={{ ...S.grid(6), marginBottom: 16 }}>
        {kpiCards.map(k => (
          <div key={k.label} style={{ ...S.card, borderLeft: `3px solid ${k.color}` }}>
            <div style={S.cardTitle}>{k.label}</div>
            <div style={{ ...S.kpiVal, color: k.color }}>{k.value}</div>
            <div style={S.kpiSub}>{k.sub}</div>
          </div>
        ))}
      </div>

      {/* Charts + Macro */}
      <div style={{ ...S.grid(3), marginBottom: 16 }}>
        {/* BTC Chart */}
        <div style={{ ...S.card, gridColumn: 'span 2' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 }}>
            <div>
              <div style={S.cardTitle}>BTC/USDT — 4H</div>
              <div style={{ fontSize: 22, fontWeight: 700, color: '#f0f6fc' }}>
                ${fmt(prices['BTC/USDT']?.price || 68000)}
              </div>
            </div>
            <div style={{ textAlign: 'right' }}>
              <div style={{ color: pctColor(prices['BTC/USDT']?.changePct || 0), fontSize: 14, fontWeight: 700 }}>
                {fmtPct(prices['BTC/USDT']?.changePct || 0)}
              </div>
              <div style={{ fontSize: 10, color: '#6e7681' }}>simüle veri</div>
            </div>
          </div>
          <ResponsiveContainer width="100%" height={180}>
            <AreaChart data={btcHist}>
              <defs>
                <linearGradient id="btcGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%"  stopColor="#f7931a" stopOpacity={0.3}/>
                  <stop offset="95%" stopColor="#f7931a" stopOpacity={0}/>
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#21262d" />
              <XAxis dataKey="t" hide />
              <YAxis domain={['auto','auto']} tick={{ fill:'#6e7681', fontSize:10 }} width={60} tickFormatter={v => fmt(v,0)} />
              <Tooltip contentStyle={S.tooltip} formatter={v => ['$'+fmt(v), 'Fiyat']} />
              <Area type="monotone" dataKey="price" stroke="#f7931a" fill="url(#btcGrad)" strokeWidth={2} dot={false} />
            </AreaChart>
          </ResponsiveContainer>
        </div>

        {/* Macro Panel */}
        <div style={S.card}>
          <div style={S.cardTitle}>MAKRO GÖSTERGELER</div>
          {macroIndices && Object.keys(macroIndices).length > 0 ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {Object.entries({
                VIX: macroIndices.VIX, DXY: macroIndices.DXY,
                'US10Y': macroIndices.US10Y, XAU: macroIndices.XAU,
                'BTC.D': macroIndices.BTCD, TOTAL: macroIndices.TOTAL,
                XU30: macroIndices.XU30,
              }).map(([k, v]) => v ? (
                <div key={k} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <span style={{ fontSize: 10, color: '#6e7681', minWidth: 60 }}>{k}</span>
                  <span style={{ fontFamily: 'monospace', fontSize: 12 }}>
                    {v.display || fmt(v.value)}
                  </span>
                  <span style={{ fontSize: 10, color: v.direction === 'UP' || v.direction === 'DOMINANT' ? '#3fb950' : '#f85149', fontWeight: 700 }}>
                    {v.direction}
                  </span>
                </div>
              ) : null)}
            </div>
          ) : (
            <div style={{ textAlign: 'center', padding: 20, color: '#6e7681' }}>
              <div style={{ fontSize: 10, marginBottom: 8 }}>API bekleniyor</div>
              <div style={{ fontSize: 9 }}>localhost:5000/api/macro</div>
              <button style={{ ...S.btn('blue'), marginTop: 8, fontSize: 10 }} onClick={fetchData}>Yenile</button>
            </div>
          )}
          {macroIndices && (
            <div style={{ marginTop: 12, padding: '8px 10px', background: `${vixColor}11`, borderRadius: 6, border: `1px solid ${vixColor}33` }}>
              <div style={{ fontSize: 10, color: vixColor, fontWeight: 700 }}>
                REJİM: {vixRegime} VIX {vixVal.toFixed(1)}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Signals */}
      <div style={{ ...S.grid(2), marginBottom: 16 }}>
        <div style={S.card}>
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 12 }}>
            <div style={S.cardTitle}>MTF SİNYALLER</div>
            <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
              {apiOk ? <span style={{ ...S.badge('#3fb950'), fontSize: 9 }}>API BAĞLI</span>
                     : <span style={{ ...S.badge('#f85149'), fontSize: 9 }}>API YOK</span>}
              {lastFetch && <span style={{ fontSize: 9, color: '#6e7681' }}>{lastFetch}</span>}
            </div>
          </div>
          {signals?.data && Object.keys(signals.data).length > 0 ? (
            <table style={S.table}>
              <thead><tr>
                <th style={S.th}>Sembol</th>
                <th style={S.th}>Fiyat</th>
                <th style={S.th}>1D Trend</th>
                <th style={S.th}>ADX</th>
                <th style={S.th}>Sinyal</th>
              </tr></thead>
              <tbody>
                {Object.values(signals.data).map(s => (
                  <tr key={s.symbol} style={{ background: '#0d1117' }}>
                    <td style={S.td}><strong style={{ color: '#f7931a' }}>{s.symbol}</strong></td>
                    <td style={{ ...S.td, fontFamily: 'monospace' }}>${fmt(s.price)}</td>
                    <td style={S.td}>
                      <span style={{ 
                        color: s.direction_1d === 'bullish' ? '#3fb950' : 
                               s.direction_1d === 'bearish' ? '#f85149' : '#e3b341', 
                        fontSize: 10, fontWeight: 600 
                      }}>
                        {s.direction_1d?.toUpperCase() || 'NÖTR'}
                      </span>
                    </td>
                    <td style={{ ...S.td, fontFamily: 'monospace' }}>
                      {s.signals_1d?.adx ? s.signals_1d.adx.toFixed(1) : '—'}
                    </td>
                    <td style={S.td}>
                      <span style={S.badge(
                        s.signals_1d?.buy_signals?.length > 0 ? '#3fb950' : 
                        s.signals_1d?.sell_signals?.length > 0 ? '#f85149' : '#6e7681'
                      )}>
                        {s.signals_1d?.buy_signals?.length > 0 ? 'AL' : 
                         s.signals_1d?.sell_signals?.length > 0 ? 'SAT' : 'BEKLE'}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <div style={{ textAlign: 'center', padding: 30, color: '#6e7681' }}>
              <div>MTF sinyaller yükleniyor...</div>
              <div style={{ fontSize: 10, marginTop: 4 }}>api_server.py çalıştırın: localhost:5000</div>
            </div>
          )}
        </div>

        {/* Macro Decisions */}
        <div style={S.card}>
          <div style={S.cardTitle}>MAKRO KARAR AKIŞI</div>
          {macro?.data?.decision ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
              {macro.data.decision.map((d, i) => (
                <div key={i} style={{
                  padding: '8px 12px', borderRadius: 6,
                  background: d.category === 'MAKRO' ? '#1a2333' : d.category === 'KRIPTO' ? '#1a1a33' : d.category === 'BIST' ? '#1a3322' : '#1a2a1a',
                  borderLeft: `3px solid ${d.category === 'MAKRO' ? '#58a6ff' : d.category === 'KRIPTO' ? '#d2a8ff' : d.category === 'BIST' ? '#3fb950' : '#e3b341'}`,
                }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <span style={{ fontSize: 9, color: '#6e7681', letterSpacing: 1 }}>{d.category} · {d.indicator}</span>
                    <span style={{ fontSize: 9, color: '#e3b341', fontWeight: 700 }}>{d.signal}</span>
                  </div>
                  <div style={{ fontSize: 12, color: '#c9d1d9', marginTop: 2, fontWeight: 600 }}>{d.action}</div>
                </div>
              ))}
            </div>
          ) : (
            <div style={{ color: '#6e7681', textAlign: 'center', padding: 20 }}>
              Makro kararlar yükleniyor...<br/>
              <span style={{ fontSize: 10 }}>api_server.py gerekli</span>
            </div>
          )}
        </div>
      </div>

      {/* AKSİYON PANELİ — Açık Pozisyonlar */}
      {posRecos.length > 0 && (
        <div style={{ ...S.card, marginTop: 12 }}>
          <div style={S.cardTitle}>AKSİYON PANELİ — AÇIK POZİSYONLAR</div>
          {posRecos.map((r, i) => (
            <div key={i} style={{ padding: '10px 12px', borderRadius: 6, marginBottom: 8, background: '#0d1117',
              borderLeft: `3px solid ${r.action?.priority === 'HIGH' ? '#f85149' : r.action?.priority === 'MEDIUM' ? '#e3b341' : '#58a6ff'}` }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
                <span style={{ fontWeight: 700, color: '#58a6ff' }}>{r.symbol} — {r.direction}</span>
                <span style={{ fontSize: 10, color: '#6e7681' }}>{r.days_open}g | Ç7: {r.c7}</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 6 }}>
                <span style={{ fontFamily: 'monospace', fontSize: 12 }}>Giriş: {fmt(r.entry_price)} → {fmt(r.current_price)}</span>
                <span style={{ color: pctColor(r.pnl_pct), fontWeight: 700 }}>{fmtPct(r.pnl_pct)} (${fmt(r.pnl_usd)})</span>
              </div>
              <div style={{ fontSize: 11, marginBottom: 6 }}>
                <span style={{ ...S.badge(r.action?.priority === 'HIGH' ? '#f85149' : r.action?.priority === 'MEDIUM' ? '#e3b341' : '#3fb950'), marginRight: 8 }}>{r.action?.priority}</span>
                {r.action?.reason}
              </div>
              <div style={{ display: 'flex', gap: 8 }}>
                <button style={S.btn('primary')} onClick={() => apiPost('/action-log', { ...r.action, symbol: r.symbol, decision: 'apply' })}>✓ Uygula</button>
                <button style={S.btn('secondary')} onClick={() => apiPost('/action-log', { ...r.action, symbol: r.symbol, decision: 'reject' })}>✗ Reddet</button>
                <button style={S.btn('secondary')} onClick={() => apiPost('/action-log', { ...r.action, symbol: r.symbol, decision: 'defer' })}>⏸ Ertele</button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ─────────────────────────────────────────────────────────
// YENİ İŞLEM TAB
// ─────────────────────────────────────────────────────────
function YeniIslemTab({ prices, engine }) {
  const [marketType, setMarketType] = useState('KRIPTO');
  const [sector,     setSector]     = useState('');
  const [symbol,     setSymbol]     = useState('');
  const [direction,  setDirection]  = useState('LONG');
  const [price,      setPrice]      = useState('');
  const [atr,        setAtr]        = useState('');
  const [stop,       setStop]       = useState('');
  const [target,     setTarget]     = useState('');
  const [vix,        setVix]        = useState('18.5');
  const [note,       setNote]       = useState('');
  const [calc,       setCalc]       = useState(null);
  const [submitted,  setSubmitted]  = useState(false);

  const sectors = SYMBOL_DB[marketType] ? Object.keys(SYMBOL_DB[marketType]) : [];
  const symbols = sector && SYMBOL_DB[marketType]?.[sector] ? SYMBOL_DB[marketType][sector] : [];

  useEffect(() => { setSector(''); setSymbol(''); }, [marketType]);
  useEffect(() => {
    const sym = symbols.find(s => s.code === symbol);
    if (sym) {
      const p = prices[sym.code]?.price || sym.base;
      setPrice(p.toFixed(4));
      const defaultAtr = (p * 0.025).toFixed(4);
      setAtr(defaultAtr);
    }
  }, [symbol]);

  const calcSize = () => {
    const p   = parseFloat(price);
    const a   = parseFloat(atr);
    const v   = parseFloat(vix) || 18;
    if (!p || !a) return;
    const res = engine.calcPositionSize(p, a, v);
    const rr  = stop && target
      ? Math.abs((parseFloat(target) - p) / (p - parseFloat(stop)))
      : 2;
    setCalc({ ...res, rr: Math.round(rr * 100) / 100 });
  };

  const handleSubmit = async () => {
    if (!symbol || !price || !calc) return;
    const res = await apiPost('/positions', {
      symbol, direction, lot_size: calc.units || 1,
      entry_price: parseFloat(price), initial_stop: parseFloat(stop) || calc.stopPrice,
      target_price: parseFloat(target) || calc.targetPrice,
      signal_type: 'manual', note,
    });
    if (res?.status === 'ok') {
      setSubmitted(true);
      setTimeout(() => setSubmitted(false), 3000);
    }
  };

  const rr = calc?.rr || 0;
  const rrColor = rr >= 2 ? '#3fb950' : rr >= 1.5 ? '#e3b341' : '#f85149';

  return (
    <div style={{ maxWidth: 900 }}>
      <div style={{ ...S.card, marginBottom: 16 }}>
        <div style={S.sectionTitle}>Yeni İşlem Kurulumu</div>

        {/* Piyasa Tipi */}
        <div style={S.section}>
          <div style={S.label}>Piyasa Tipi</div>
          <div style={{ display: 'flex', gap: 8 }}>
            {['BIST','KRIPTO','EMTIA','FAIZ'].map(m => (
              <button key={m} onClick={() => setMarketType(m)}
                style={{ ...S.btn(marketType === m ? 'blue' : 'secondary'), minWidth: 90, fontSize: 12 }}>
                {m}
              </button>
            ))}
          </div>
        </div>

        {/* Sektör + Sembol */}
        <div style={{ ...S.row, marginBottom: 16 }}>
          <div style={{ flex: 1 }}>
            <label style={S.label}>Sektör / Kategori</label>
            <select style={S.select} value={sector} onChange={e => setSector(e.target.value)}>
              <option value="">— Seç —</option>
              {sectors.map(s => <option key={s} value={s}>{s}</option>)}
            </select>
          </div>
          <div style={{ flex: 2 }}>
            <label style={S.label}>Enstrüman</label>
            <select style={S.select} value={symbol} onChange={e => setSymbol(e.target.value)} disabled={!sector}>
              <option value="">— Sembol Seç —</option>
              {symbols.map(s => (
                <option key={s.code} value={s.code}>
                  {s.code} — {s.name}
                </option>
              ))}
            </select>
          </div>
          <div style={{ flex: 1 }}>
            <label style={S.label}>Yön</label>
            <div style={{ display: 'flex', gap: 8 }}>
              {['LONG','SHORT'].map(d => (
                <button key={d} onClick={() => setDirection(d)}
                  style={{ ...S.btn(direction === d ? (d === 'LONG' ? 'primary' : 'danger') : 'secondary'), flex: 1 }}>
                  {d}
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Price + ATR + VIX */}
        <div style={{ ...S.row, marginBottom: 16 }}>
          {[
            { label: 'Giriş Fiyatı', val: price, set: setPrice, ph: '68000' },
            { label: 'ATR14',        val: atr,   set: setAtr,   ph: '1850' },
            { label: 'VIX',          val: vix,   set: setVix,   ph: '18.5' },
            { label: 'Stop Fiyatı',  val: stop,  set: setStop,  ph: 'Opsiyonel' },
            { label: 'Hedef Fiyat',  val: target,set: setTarget,ph: 'Opsiyonel' },
          ].map(f => (
            <div key={f.label} style={{ flex: 1 }}>
              <label style={S.label}>{f.label}</label>
              <input style={S.input} value={f.val} onChange={e => f.set(e.target.value)} placeholder={f.ph} type="number" />
            </div>
          ))}
        </div>

        {/* Not */}
        <div style={{ marginBottom: 16 }}>
          <label style={S.label}>Not / Gerekçe</label>
          <input style={S.input} value={note} onChange={e => setNote(e.target.value)} placeholder="MTF skor, kurulum gerekçesi..." />
        </div>

        {/* Buttons */}
        <div style={{ display: 'flex', gap: 12 }}>
          <button style={S.btn('blue')} onClick={calcSize}>Pozisyon Hesapla</button>
          <button style={{ ...S.btn('primary'), opacity: !calc || !symbol ? 0.5 : 1 }}
            onClick={handleSubmit} disabled={!calc || !symbol}>
            {submitted ? '✓ Kaydedildi' : 'İşlemi Kaydet'}
          </button>
        </div>
      </div>

      {/* Calculation Result */}
      {calc && (
        <div style={{ ...S.card, borderLeft: `3px solid ${rrColor}` }}>
          <div style={S.sectionTitle}>Pozisyon Analizi — {symbol} {direction}</div>
          <div style={S.grid(4)}>
            {[
              { label: 'Pozisyon Boyutu', val: '$' + fmt(calc.positionSize), color: '#58a6ff' },
              { label: 'Portföy %',       val: calc.portfolioPct + '%', color: '#d2a8ff' },
              { label: 'Risk $',          val: '$' + fmt(calc.riskDollars), color: '#e3b341' },
              { label: 'R/R Oranı',       val: calc.rr + ':1', color: rrColor },
              { label: 'Stop Fiyatı',     val: '$' + fmt(calc.stopPrice), color: '#f85149' },
              { label: 'Hedef Fiyatı',    val: '$' + fmt(calc.targetPrice), color: '#3fb950' },
              { label: 'ATR%',            val: calc.atrPct + '%', color: '#c9d1d9' },
              { label: 'Max Ağırlık',     val: calc.maxWeightPct + '%', color: '#c9d1d9' },
            ].map(item => (
              <div key={item.label} style={{ ...S.card, padding: 12 }}>
                <div style={{ fontSize: 10, color: '#6e7681', marginBottom: 4 }}>{item.label}</div>
                <div style={{ fontSize: 18, fontWeight: 700, color: item.color, fontFamily: 'monospace' }}>{item.val}</div>
              </div>
            ))}
          </div>

          {/* Risk checks */}
          <div style={{ marginTop: 12, padding: 12, background: '#0d1117', borderRadius: 6 }}>
            <div style={{ fontSize: 10, color: '#6e7681', marginBottom: 8, letterSpacing: 1 }}>RİSK KONTROL LİSTESİ</div>
            <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
              {[
                { check: calc.riskDollars <= engine.portfolioSize * 0.005, label: 'Risk ≤ %0.5' },
                { check: calc.rr >= 2.0, label: 'R/R ≥ 2.0' },
                { check: calc.portfolioPct <= calc.maxWeightPct, label: `Boyut ≤ ${calc.maxWeightPct}%` },
                { check: parseFloat(vix) < 30, label: 'VIX < 30' },
                { check: parseFloat(vix) < 20, label: 'VIX < 20' },
              ].map(c => (
                <div key={c.label} style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
                  <span style={{ color: c.check ? '#3fb950' : '#f85149', fontSize: 14 }}>{c.check ? '✓' : '✗'}</span>
                  <span style={{ fontSize: 11, color: c.check ? '#3fb950' : '#f85149' }}>{c.label}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// ─────────────────────────────────────────────────────────
// PORTFÖY TAB
// ─────────────────────────────────────────────────────────
// PORTFÖY TAB (DÜZELTİLMİŞ)
// ─────────────────────────────────────────────────────────
function PortfoyTab({ prices, engine }) {
  const [positions, setPositions] = useState([]);
  const [portfolio, setPortfolio] = useState(null);
  const [loading, setLoading] = useState(true);
  const [closingPosition, setClosingPosition] = useState(null);
  const [partialCloseModal, setPartialCloseModal] = useState(null);

  // fetchData fonksiyonunu TANIMLA
  const fetchData = useCallback(async () => {
    setLoading(true);
    const [posRes, portRes] = await Promise.all([
      apiFetch('/positions'),
      apiFetch('/portfolio')
    ]);
    
    if (posRes?.data) {
      setPositions(posRes.data.map(p => ({
        id: p.id, 
        symbol: p.symbol, 
        direction: p.direction,
        size: p.lot_size, 
        entryPrice: p.entry_price,
        stopPrice: p.current_stop || p.initial_stop,
        targetPrice: p.target_price || 0,
        openDate: p.opened_at?.split('T')[0] || '',
      })));
    }
    
    if (portRes) setPortfolio(portRes);
    setLoading(false);
  }, []);

  // fetchData'yı useEffect'te kullan
  useEffect(() => {
    fetchData();
    const id = setInterval(fetchData, 30000);
    return () => clearInterval(id);
  }, [fetchData]);

  const closePosition = async (positionId, exitPrice, reason = 'manual') => {
    const res = await apiPost('/positions/close', {
      position_id: positionId,
      exit_price: exitPrice || prices[positionId]?.price,
      close_reason: reason,
      closed_at: new Date().toISOString()
    });
    
    if (res?.status === 'ok') {
      fetchData(); // Verileri yenile
      setClosingPosition(null);
      alert('Pozisyon başarıyla kapatıldı');
    }
  };

  const partialClose = async (positionId, percentage, exitPrice) => {
    const res = await apiPost('/positions/partial-close', {
      position_id: positionId,
      percentage, // 0.25, 0.5, 0.75
      exit_price: exitPrice
    });
    
    if (res?.status === 'ok') {
      fetchData(); // Verileri yenile
      setPartialCloseModal(null);
      alert(`Pozisyonun %${percentage*100} kapatıldı`);
    }
  };

  if (loading) {
    return (
      <div style={{ textAlign: 'center', padding: 50, color: '#6e7681' }}>
        Veriler yükleniyor...
      </div>
    );
  }

  const totalPnl = positions.reduce((acc, p) => {
    const curPrice = prices[p.symbol]?.price || p.entryPrice;
    const mult     = p.direction === 'LONG' ? 1 : -1;
    return acc + (curPrice - p.entryPrice) * p.size * mult;
  }, 0);

  return (
    <div>
      {/* Summary */}
      <div style={{ ...S.grid(4), marginBottom: 16 }}>
        {[
          { label: 'TOPLAM P&L', val: totalPnl !== 0 ? (totalPnl >= 0 ? '+' : '') + '$' + fmt(totalPnl) : '—', color: pctColor(totalPnl) },
          { label: 'AÇIK POZİSYON', val: positions.length || '—', color: '#58a6ff' },
          { label: 'PORTFÖY DEĞERİ', val: portfolio?.portfolio_value ? '$' + fmt(portfolio.portfolio_value) : '—', color: '#c9d1d9' },
          { label: 'GÜN P&L%', val: portfolio?.daily_pnl_pct ? fmtPct(portfolio.daily_pnl_pct) : '—', color: pctColor(portfolio?.daily_pnl_pct || 0) },
        ].map(k => (
          <div key={k.label} style={S.card}>
            <div style={S.cardTitle}>{k.label}</div>
            <div style={{ fontSize: 22, fontWeight: 700, color: k.color, fontFamily: 'monospace' }}>{k.val}</div>
          </div>
        ))}
      </div>

      {/* Positions Table */}
      <div style={{ ...S.card, marginBottom: 16 }}>
        <div style={S.cardTitle}>AÇIK POZİSYONLAR</div>
        <table style={S.table}>
          <thead><tr>
            {['Sembol','Yön','Boyut','Giriş','Güncel','P&L $','P&L %','Stop','Hedef','Durum','İşlemler'].map(h =>
              <th key={h} style={S.th}>{h}</th>
            )}
          </tr></thead>
          <tbody>
            {positions.map(p => {
              const cur  = prices[p.symbol]?.price || p.entryPrice;
              const mult = p.direction === 'LONG' ? 1 : -1;
              const pnl  = (cur - p.entryPrice) * p.size * mult;
              const pnlPct = (cur - p.entryPrice) / p.entryPrice * 100 * mult;
              const status = pnlPct > 0 ? 'KARDA' : pnlPct < 0 ? 'ZARARDA' : 'NÖTR';

              return (
                <tr key={p.id} style={{ background: '#0d1117' }}>
                  <td style={S.td}><strong style={{ color: '#58a6ff' }}>{p.symbol}</strong></td>
                  <td style={S.td}><span style={S.badge(p.direction === 'LONG' ? '#3fb950' : '#f85149')}>{p.direction}</span></td>
                  <td style={{ ...S.td, fontFamily: 'monospace' }}>{p.size}</td>
                  <td style={{ ...S.td, fontFamily: 'monospace' }}>{fmt(p.entryPrice)}</td>
                  <td style={{ ...S.td, fontFamily: 'monospace' }}>{cur ? fmt(cur) : '—'}</td>
                  <td style={{ ...S.td, color: pctColor(pnl), fontFamily: 'monospace' }}>{pnl ? (pnl>=0?'+':'') + fmt(pnl) : '—'}</td>
                  <td style={{ ...S.td, color: pctColor(pnlPct), fontFamily: 'monospace' }}>{pnlPct ? fmtPct(pnlPct) : '—'}</td>
                  <td style={{ ...S.td, color: '#f85149', fontFamily: 'monospace' }}>{fmt(p.stopPrice)}</td>
                  <td style={{ ...S.td, color: '#3fb950', fontFamily: 'monospace' }}>{fmt(p.targetPrice)}</td>
                  <td style={S.td}><span style={S.badge(status === 'KARDA' ? '#3fb950' : status === 'ZARARDA' ? '#f85149' : '#e3b341')}>{status}</span></td>
                  <td style={S.td}>
                    <div style={{ display: 'flex', gap: 4 }}>
                      <button 
                        style={{ ...S.btn('danger'), fontSize: 11, padding: '4px 8px' }}
                        onClick={() => setClosingPosition(p)}
                      >
                        Kapat
                      </button>
                      <button 
                        style={{ ...S.btn('secondary'), fontSize: 11, padding: '4px 8px' }}
                        onClick={() => setPartialCloseModal(p)}
                      >
                        Kısmi
                      </button>
                    </div>
                  </td>
                </tr>
              );
            })}
            {positions.length === 0 && (
              <tr><td colSpan={11} style={{ ...S.td, textAlign: 'center', color: '#6e7681' }}>Açık pozisyon bulunmuyor</td></tr>
            )}
          </tbody>
        </table>
      </div>

      {/* Pozisyon Kapatma Modal */}
      {closingPosition && (
        <ClosePositionModal
          position={closingPosition}
          currentPrice={prices[closingPosition.symbol]?.price}
          onClose={() => setClosingPosition(null)}
          onSubmit={(id, price, reason) => closePosition(id, price, reason)}
        />
      )}

      {/* Kısmi Kapatma Modal */}
      {partialCloseModal && (
        <PartialCloseModal
          position={partialCloseModal}
          currentPrice={prices[partialCloseModal.symbol]?.price}
          onClose={() => setPartialCloseModal(null)}
          onSubmit={(id, percentage, price) => partialClose(id, percentage, price)}
        />
      )}
    </div>
  );
}

// Pozisyon Kapatma Modal
function ClosePositionModal({ position, currentPrice, onClose, onSubmit }) {
  const [exitPrice, setExitPrice] = useState(currentPrice || '');
  const [reason, setReason] = useState('manual');
  
  return (
    <div style={{
      position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
      background: 'rgba(0,0,0,0.7)', display: 'flex',
      alignItems: 'center', justifyContent: 'center', zIndex: 1000
    }}>
      <div style={{ ...S.card, width: 400, maxWidth: '90%' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 16 }}>
          <h3 style={{ color: '#f0f6fc', margin: 0 }}>
            {position.symbol} - Pozisyon Kapat
          </h3>
          <button onClick={onClose} style={{ background: 'none', border: 'none', color: '#8b949e', cursor: 'pointer', fontSize: 16 }}>
            ✕
          </button>
        </div>
        
        <div style={{ marginBottom: 16 }}>
          <label style={S.label}>Pozisyon Bilgisi</label>
          <div style={{ background: '#0d1117', padding: 12, borderRadius: 6 }}>
            <div>Yön: <strong style={{ color: position.direction === 'LONG' ? '#3fb950' : '#f85149' }}>{position.direction}</strong></div>
            <div>Giriş: ${fmt(position.entryPrice)}</div>
            <div>Boyut: {position.size} lot</div>
          </div>
        </div>
        
        <div style={{ marginBottom: 16 }}>
          <label style={S.label}>Çıkış Fiyatı</label>
          <input
            style={S.input}
            type="number"
            value={exitPrice}
            onChange={(e) => setExitPrice(e.target.value)}
            step="0.01"
            placeholder="Çıkış fiyatı"
          />
          {currentPrice && (
            <div style={{ fontSize: 11, color: '#6e7681', marginTop: 4 }}>
              Güncel fiyat: ${fmt(currentPrice)}
            </div>
          )}
        </div>
        
        <div style={{ marginBottom: 24 }}>
          <label style={S.label}>Kapanış Nedeni</label>
          <select style={S.select} value={reason} onChange={(e) => setReason(e.target.value)}>
            <option value="manual">Manuel Kapatma</option>
            <option value="take_profit">Kar Aldı</option>
            <option value="stop_loss">Stop Loss</option>
            <option value="signal">Sinyal Kapattı</option>
            <option value="risk">Risk Yönetimi</option>
          </select>
        </div>
        
        <div style={{ display: 'flex', gap: 8 }}>
          <button 
            style={{ ...S.btn('danger'), flex: 1 }}
            onClick={() => onSubmit(position.id, parseFloat(exitPrice), reason)}
          >
            Pozisyonu Kapat
          </button>
          <button 
            style={{ ...S.btn('secondary'), flex: 1 }}
            onClick={onClose}
          >
            İptal
          </button>
        </div>
        
        {exitPrice && position.entryPrice && (
          <div style={{ marginTop: 12, padding: 8, background: '#0d1117', borderRadius: 4 }}>
            P&L: <span style={{ color: pctColor((exitPrice - position.entryPrice) / position.entryPrice * 100) }}>
              {fmtPct((exitPrice - position.entryPrice) / position.entryPrice * 100)}
            </span>
          </div>
        )}
      </div>
    </div>
  );
}

// Kısmi Kapatma Modal
function PartialCloseModal({ position, currentPrice, onClose, onSubmit }) {
  const [percentage, setPercentage] = useState(50);
  const [exitPrice, setExitPrice] = useState(currentPrice || '');
  
  return (
    <div style={{
      position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
      background: 'rgba(0,0,0,0.7)', display: 'flex',
      alignItems: 'center', justifyContent: 'center', zIndex: 1000
    }}>
      <div style={{ ...S.card, width: 400 }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 16 }}>
          <h3 style={{ color: '#f0f6fc', margin: 0 }}>
            {position.symbol} - Kısmi Kapat
          </h3>
          <button onClick={onClose} style={{ background: 'none', border: 'none', color: '#8b949e', cursor: 'pointer', fontSize: 16 }}>
            ✕
          </button>
        </div>
        
        <div style={{ marginBottom: 16 }}>
          <label style={S.label}>Kapatılacak Yüzde</label>
          <input
            style={{ ...S.input, marginBottom: 8 }}
            type="range"
            min="10"
            max="90"
            step="10"
            value={percentage}
            onChange={(e) => setPercentage(e.target.value)}
          />
          <div style={{ textAlign: 'center', fontSize: 18, fontWeight: 700, color: '#58a6ff', marginTop: 8 }}>
            %{percentage}
          </div>
          <div style={{ fontSize: 12, color: '#6e7681', textAlign: 'center' }}>
            {position.size} lot → {Math.round(position.size * percentage / 100)} lot kapanacak
          </div>
        </div>
        
        <div style={{ marginBottom: 24 }}>
          <label style={S.label}>Çıkış Fiyatı</label>
          <input
            style={S.input}
            type="number"
            value={exitPrice}
            onChange={(e) => setExitPrice(e.target.value)}
            placeholder="Çıkış fiyatı"
          />
        </div>
        
        <div style={{ display: 'flex', gap: 8 }}>
          <button 
            style={{ ...S.btn('blue'), flex: 1 }}
            onClick={() => onSubmit(position.id, percentage/100, parseFloat(exitPrice))}
          >
            Kısmi Kapat
          </button>
          <button 
            style={{ ...S.btn('secondary'), flex: 1 }}
            onClick={onClose}
          >
            İptal
          </button>
        </div>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────
// RİSK ANALİZ TAB
// ─────────────────────────────────────────────────────────
function RiskAnalizTab({ engine }) {
  const mockReturns = Array.from({ length: 100 }, () => (Math.random() - 0.5) * 0.04);
  const sharpe = engine.calcSharpe(mockReturns);
  const var95  = engine.calcVaR(mockReturns, 0.95);
  const navHist = Array.from({ length: 60 }, (_, i) =>
    engine.portfolioSize * (1 + Math.sin(i * 0.2) * 0.03 - i * 0.001)
  );
  const dd = engine.calcDrawdown(navHist);
  const kelly = engine.calcKelly(0.58, 1350, 620);

  const returnDist = Array.from({ length: 20 }, (_, i) => {
    const x = -0.05 + i * 0.005;
    return { x: (x * 100).toFixed(1), count: Math.round(Math.exp(-((x - 0.002) ** 2) / 0.0004) * 30) };
  });

  const stressCases = [
    { name: '2008 Krizi',          equity: -42, crypto: -70, gold: +12, comment: 'Sistemik çöküş' },
    { name: 'COVID Marş 2020',     equity: -35, crypto: -65, gold: +8,  comment: 'Ani panik satışı' },
    { name: 'TR Kur Krizi 2021',   equity: -28, crypto: -20, gold: +22, comment: 'TL bazlı kayıp' },
    { name: 'Terra/Luna Mayıs 2022',equity: -18, crypto: -80, gold: +5, comment: 'Kripto özgül kriz' },
    { name: 'FTX Kasım 2022',      equity: -8,  crypto: -65, gold: +2,  comment: 'Borsa iflası' },
  ];

  const portfolioAlloc = [
    { name:'BTC',    pct:20, scenario2008: -42*0.20, scenarioCovid: -35*0.20 },
    { name:'ETH',    pct:15, scenario2008: -55*0.15, scenarioCovid: -45*0.15 },
    { name:'BIST',   pct:20, scenario2008: -35*0.20, scenarioCovid: -28*0.20 },
    { name:'XAU',    pct:10, scenario2008:  12*0.10, scenarioCovid:   8*0.10 },
    { name:'Nakit',  pct:35, scenario2008:   0*0.35, scenarioCovid:   0*0.35 },
  ];

  const portfolio2008Loss = portfolioAlloc.reduce((a,b) => a + b.scenario2008, 0);
  const portfolioCovidLoss = portfolioAlloc.reduce((a,b) => a + b.scenarioCovid, 0);

  return (
    <div>
      {/* Risk KPIs */}
      <div style={{ ...S.grid(4), marginBottom: 16 }}>
        {[
          { label: 'SHARPE',        val: sharpe.toFixed(2),        color: sharpe > 1.5 ? '#3fb950' : sharpe > 1 ? '#e3b341' : '#f85149', sub: 'Hedef: >2.0' },
          { label: 'VaR %95',       val: (var95*100).toFixed(2)+'%',color: '#f85149', sub: 'Günlük max kayıp' },
          { label: 'MAX DRAWDOWN',  val: dd.max.toFixed(2)+'%',     color: dd.max < -10 ? '#f85149' : '#e3b341', sub: 'Zirve: $1.02M' },
          { label: 'KELLY (1/4)',   val: kelly.valid ? kelly.quarterKelly+'%' : 'KAPALI', color: kelly.valid ? '#3fb950' : '#f85149', sub: kelly.valid ? `p:${(kelly.p*100).toFixed(0)}% b:${kelly.b?.toFixed(2)}` : kelly.reason },
        ].map(k => (
          <div key={k.label} style={S.card}>
            <div style={S.cardTitle}>{k.label}</div>
            <div style={{ fontSize: 24, fontWeight: 700, color: k.color, fontFamily: 'monospace' }}>{k.val}</div>
            <div style={S.kpiSub}>{k.sub}</div>
          </div>
        ))}
      </div>

      <div style={{ ...S.grid(2), marginBottom: 16 }}>
        {/* Return Distribution */}
        <div style={S.card}>
          <div style={S.cardTitle}>GETİRİ DAĞILIMI</div>
          <ResponsiveContainer width="100%" height={180}>
            <BarChart data={returnDist} barCategoryGap={2}>
              <CartesianGrid strokeDasharray="3 3" stroke="#21262d"/>
              <XAxis dataKey="x" tick={{ fill:'#6e7681', fontSize:9 }} tickFormatter={v => v+'%'} />
              <YAxis tick={{ fill:'#6e7681', fontSize:9 }} />
              <Tooltip contentStyle={S.tooltip} formatter={v => [v, 'Frekans']} />
              <Bar dataKey="count" fill="#58a6ff" opacity={0.8} />
              <ReferenceLine x="-2.5" stroke="#f85149" strokeDasharray="3 3" label={{ value: 'VaR', fill:'#f85149', fontSize:9 }} />
            </BarChart>
          </ResponsiveContainer>
        </div>

        {/* Correlation Matrix */}
        <div style={S.card}>
          <div style={S.cardTitle}>KORELASYON MATRİSİ</div>
          {[
            ['',      'BTC',   'ETH',   'SOL',   'XAU',   'BIST'],
            ['BTC',   '1.00',  '0.90',  '0.85',  '-0.40', '0.42'],
            ['ETH',   '0.90',  '1.00',  '0.82',  '-0.38', '0.38'],
            ['SOL',   '0.85',  '0.82',  '1.00',  '-0.35', '0.35'],
            ['XAU',   '-0.40', '-0.38', '-0.35', '1.00',  '0.15'],
            ['BIST',  '0.42',  '0.38',  '0.35',  '0.15',  '1.00'],
          ].map((row, ri) => (
            <div key={ri} style={{ display: 'flex', gap: 2, marginBottom: 2 }}>
              {row.map((cell, ci) => {
                const val = parseFloat(cell);
                const isHeader = ri === 0 || ci === 0;
                const bg = isHeader ? '#21262d' :
                  val > 0.7 ? '#da363344' : val > 0.4 ? '#e3b34122' :
                  val < -0.3 ? '#3fb95022' : '#0d1117';
                return (
                  <div key={ci} style={{
                    flex: 1, padding: '4px 2px', textAlign: 'center',
                    background: bg, borderRadius: 2,
                    fontSize: 10, fontFamily: 'monospace',
                    color: isHeader ? '#8b949e' : val > 0.7 ? '#f85149' : val < -0.3 ? '#3fb950' : '#c9d1d9',
                    fontWeight: isHeader ? 700 : 400,
                  }}>
                    {cell}
                  </div>
                );
              })}
            </div>
          ))}
          <div style={{ fontSize: 9, color: '#6e7681', marginTop: 8 }}>
            ρ &gt; 0.70 → Yüksek korelasyon | Max 2 pozisyon aynı grubtan
          </div>
        </div>
      </div>

      {/* Stress Tests */}
      <div style={S.card}>
        <div style={S.cardTitle}>STRES TESTLERİ — Portföy Etkisi</div>
        <table style={S.table}>
          <thead><tr>
            {['Senaryo','Hisse','Kripto','Altın','Portföy Etkisi','Yorum'].map(h => <th key={h} style={S.th}>{h}</th>)}
          </tr></thead>
          <tbody>
            {stressCases.map(s => {
              const portEffect = (s.equity * 0.20 + s.crypto * 0.35 + s.gold * 0.10) / 100 * engine.portfolioSize;
              return (
                <tr key={s.name} style={{ background: '#0d1117' }}>
                  <td style={{ ...S.td, fontWeight: 600 }}>{s.name}</td>
                  <td style={{ ...S.td, color: pctColor(s.equity), fontFamily: 'monospace' }}>{fmtPct(s.equity, 0)}</td>
                  <td style={{ ...S.td, color: pctColor(s.crypto), fontFamily: 'monospace' }}>{fmtPct(s.crypto, 0)}</td>
                  <td style={{ ...S.td, color: pctColor(s.gold),   fontFamily: 'monospace' }}>{fmtPct(s.gold, 0)}</td>
                  <td style={{ ...S.td, color: pctColor(portEffect), fontFamily: 'monospace', fontWeight: 700 }}>
                    ${fmt(portEffect)} ({fmtPct(portEffect/engine.portfolioSize*100, 1)})
                  </td>
                  <td style={{ ...S.td, fontSize: 11, color: '#6e7681' }}>{s.comment}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────
// GEÇMİŞ TAB
// ─────────────────────────────────────────────────────────
function GecmisTab() {
  const trades = Array.from({ length: 20 }, (_, i) => {
    const symbols = ['BTC/USDT','ETH/USDT','SOL/USDT','GARAN','AKBNK','XAU/USD'];
    const sym     = symbols[i % symbols.length];
    const win     = Math.random() > 0.42;
    const entry   = 50000 + Math.random() * 20000;
    const pnlPct  = win ? Math.random() * 8 + 0.5 : -(Math.random() * 4 + 0.5);
    const size    = Math.round(Math.random() * 50000 + 10000);
    return {
      id: i+1, date: `2026-02-${String(i+1).padStart(2,'0')}`,
      symbol: sym, direction: Math.random()>0.5?'LONG':'SHORT',
      entry: Math.round(entry), exit: Math.round(entry*(1+pnlPct/100)),
      size, pnl: Math.round(size*pnlPct/100), pnlPct: Math.round(pnlPct*100)/100,
      win, mtfClass: ['A','B','B','A','S','C'][i%6], score: 5+Math.floor(Math.random()*8),
    };
  });

  const wins     = trades.filter(t => t.win).length;
  const totalPnl = trades.reduce((a,b) => a+b.pnl, 0);
  const winPnl   = trades.filter(t=>t.win).reduce((a,b) => a+b.pnl, 0);
  const lossPnl  = trades.filter(t=>!t.win).reduce((a,b) => a+Math.abs(b.pnl), 0);
  const pf       = lossPnl ? (winPnl / lossPnl).toFixed(2) : '∞';

  const pnlByDay = trades.slice(0,10).map((t,i) => ({ day:i+1, pnl: t.pnl, cum: trades.slice(0,i+1).reduce((a,b)=>a+b.pnl,0) }));

  return (
    <div>
      <div style={{ ...S.grid(5), marginBottom: 16 }}>
        {[
          { label:'WIN RATE',      val: `${wins}/${trades.length} (${Math.round(wins/trades.length*100)}%)`, color: wins/trades.length > 0.5 ? '#3fb950':'#f85149' },
          { label:'TOPLAM P&L',    val: '$'+fmt(totalPnl), color: pctColor(totalPnl) },
          { label:'PROFIT FACTOR', val: pf, color: parseFloat(pf) > 1.5 ? '#3fb950':'#e3b341' },
          { label:'ORT R/R',       val: '2.1:1', color: '#3fb950' },
          { label:'İŞLEM SAYISI',  val: trades.length, color: '#58a6ff' },
        ].map(k => (
          <div key={k.label} style={S.card}>
            <div style={S.cardTitle}>{k.label}</div>
            <div style={{ fontSize: 20, fontWeight: 700, color: k.color, fontFamily: 'monospace' }}>{k.val}</div>
          </div>
        ))}
      </div>

      <div style={{ ...S.grid(2), marginBottom: 16 }}>
        <div style={S.card}>
          <div style={S.cardTitle}>KÜMÜLATİF P&L</div>
          <ResponsiveContainer width="100%" height={160}>
            <AreaChart data={pnlByDay}>
              <defs>
                <linearGradient id="pnlGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%"  stopColor="#3fb950" stopOpacity={0.3}/>
                  <stop offset="95%" stopColor="#3fb950" stopOpacity={0}/>
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#21262d"/>
              <XAxis dataKey="day" tick={{ fill:'#6e7681', fontSize:10 }} />
              <YAxis tick={{ fill:'#6e7681', fontSize:10 }} tickFormatter={v => '$'+fmt(v,0)} width={60}/>
              <Tooltip contentStyle={S.tooltip} formatter={v => ['$'+fmt(v), 'P&L']}/>
              <ReferenceLine y={0} stroke="#6e7681"/>
              <Area type="monotone" dataKey="cum" stroke="#3fb950" fill="url(#pnlGrad)" strokeWidth={2} dot={false}/>
            </AreaChart>
          </ResponsiveContainer>
        </div>

        <div style={S.card}>
          <div style={S.cardTitle}>SINIF PERFORMANSI</div>
          {[
            { cls:'S', trades:3, wr:'100%', avgRR:'3.2:1', color:'#3fb950' },
            { cls:'A', trades:6, wr:'83%',  avgRR:'2.4:1', color:'#3fb950' },
            { cls:'B', trades:8, wr:'62%',  avgRR:'1.9:1', color:'#e3b341' },
            { cls:'C', trades:3, wr:'33%',  avgRR:'1.3:1', color:'#f85149' },
          ].map(r => (
            <div key={r.cls} style={{ display:'flex', gap:12, alignItems:'center', padding:'8px 0', borderBottom:'1px solid #21262d' }}>
              <span style={{ ...S.badge(r.color), minWidth:24, textAlign:'center' }}>{r.cls}</span>
              <span style={{ fontSize:11, color:'#6e7681', flex:1 }}>{r.trades} işlem</span>
              <span style={{ fontSize:11, color:r.color, minWidth:40 }}>{r.wr}</span>
              <span style={{ fontSize:11, color:'#8b949e', fontFamily:'monospace' }}>{r.avgRR}</span>
            </div>
          ))}
        </div>
      </div>

      <div style={S.card}>
        <div style={S.cardTitle}>İŞLEM GEÇMİŞİ</div>
        <div style={{ overflowX: 'auto' }}>
          <table style={S.table}>
            <thead><tr>
              {['#','Tarih','Sembol','Yön','Giriş','Çıkış','Boyut','P&L','P&L%','Sınıf'].map(h => <th key={h} style={S.th}>{h}</th>)}
            </tr></thead>
            <tbody>
              {trades.map(t => (
                <tr key={t.id} style={{ background: t.win ? '#0d2214' : '#1a0d0d' }}>
                  <td style={S.td}>{t.id}</td>
                  <td style={{ ...S.td, fontSize:10, color:'#6e7681' }}>{t.date}</td>
                  <td style={{ ...S.td, fontWeight:600, color:'#58a6ff' }}>{t.symbol}</td>
                  <td style={S.td}><span style={S.badge(t.direction==='LONG'?'#3fb950':'#f85149')}>{t.direction}</span></td>
                  <td style={{ ...S.td, fontFamily:'monospace' }}>{fmt(t.entry)}</td>
                  <td style={{ ...S.td, fontFamily:'monospace' }}>{fmt(t.exit)}</td>
                  <td style={{ ...S.td, fontFamily:'monospace' }}>${fmt(t.size,0)}</td>
                  <td style={{ ...S.td, color:pctColor(t.pnl), fontFamily:'monospace', fontWeight:700 }}>
                    {t.pnl>=0?'+':''}{fmt(t.pnl)}
                  </td>
                  <td style={{ ...S.td, color:pctColor(t.pnlPct), fontFamily:'monospace' }}>{fmtPct(t.pnlPct)}</td>
                  <td style={S.td}><span style={S.badge('#3fb950')}>{t.mtfClass}</span></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────
// GRAFIKLER TAB
// ─────────────────────────────────────────────────────────
function GrafiklerTab({ prices }) {
  const [selectedSym, setSelectedSym] = useState('BTC/USDT');
  const symList = ['BTC/USDT','ETH/USDT','SOL/USDT','XAU/USD','USD/TRY'];
  const hist = generatePriceHistory(prices[selectedSym]?.price || 50000, 80);
  const volData = hist.map((d, i) => ({ ...d, vol: Math.round(Math.random() * 1000 + 200) }));

  return (
    <div>
      <div style={{ display:'flex', gap:8, marginBottom:16 }}>
        {symList.map(s => (
          <button key={s} style={S.btn(selectedSym===s?'blue':'secondary')} onClick={() => setSelectedSym(s)}>
            {s}
          </button>
        ))}
      </div>

      <div style={{ ...S.card, marginBottom:16 }}>
        <div style={{ display:'flex', justifyContent:'space-between', alignItems:'center', marginBottom:12 }}>
          <div style={S.cardTitle}>{selectedSym} — FİYAT GRAFİĞİ</div>
          <div style={{ fontSize:20, fontWeight:700, color:'#f0f6fc', fontFamily:'monospace' }}>
            ${fmt(prices[selectedSym]?.price || 0)}
            <span style={{ fontSize:12, marginLeft:8, color:pctColor(prices[selectedSym]?.changePct||0) }}>
              {fmtPct(prices[selectedSym]?.changePct||0)}
            </span>
          </div>
        </div>
        <ResponsiveContainer width="100%" height={280}>
          <AreaChart data={hist}>
            <defs>
              <linearGradient id="priceGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%"  stopColor="#58a6ff" stopOpacity={0.3}/>
                <stop offset="95%" stopColor="#58a6ff" stopOpacity={0}/>
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="#21262d"/>
            <XAxis dataKey="t" tick={{ fill:'#6e7681', fontSize:10 }} />
            <YAxis domain={['auto','auto']} tick={{ fill:'#6e7681', fontSize:10 }} width={70} tickFormatter={v=>fmt(v,0)} />
            <Tooltip contentStyle={S.tooltip} formatter={v => ['$'+fmt(v), 'Fiyat']} />
            <Area type="monotone" dataKey="price" stroke="#58a6ff" fill="url(#priceGrad)" strokeWidth={2} dot={false}/>
          </AreaChart>
        </ResponsiveContainer>
      </div>

      <div style={S.card}>
        <div style={S.cardTitle}>HACİM</div>
        <ResponsiveContainer width="100%" height={120}>
          <BarChart data={volData.slice(-40)}>
            <CartesianGrid strokeDasharray="3 3" stroke="#21262d" vertical={false}/>
            <XAxis dataKey="t" hide />
            <YAxis tick={{ fill:'#6e7681', fontSize:9 }} />
            <Tooltip contentStyle={S.tooltip} />
            <Bar dataKey="vol" fill="#1f6feb" opacity={0.7} />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────
// UYUMLULUK TAB
// ─────────────────────────────────────────────────────────
function UyumlulukTab({ engine }) {
  const rules = [
    { id:'H1', name:'VIX > 30 → Kripto 0%',           status:'OK',  detail:'VIX 18.5 — limit aşılmadı' },
    { id:'H2', name:'Aylık DD > -6% → Kapat',          status:'OK',  detail:'Aylık DD: -1.2%' },
    { id:'H3', name:'3 Art Arda Stop → 24s Yasak',     status:'OK',  detail:'Son 3 işlem: K K K → 0 stop streak' },
    { id:'H4', name:'MTF Skor < 5 → Yasak',            status:'OK',  detail:'Tüm açık pozisyonlar skor ≥ 7' },
    { id:'H5', name:'R/R < 1.5:1 → Yasak',             status:'WARN',detail:'ETH pozisyonu R/R: 1.7:1 (sınırda)' },
    { id:'H6', name:'1D Trend Yatay → Yasak',          status:'OK',  detail:'Tüm pozisyonlar trend aligned' },
    { id:'K1', name:'Max Risk/İşlem ≤ %0.5',           status:'OK',  detail:'En büyük risk: $4,200 (%0.42)' },
    { id:'K2', name:'Max Pozisyon ≤ MaxWeight%',        status:'OK',  detail:'BTC: %8 — limit: %8' },
    { id:'K3', name:'Korelasyon: Max 2 aynı grup',      status:'OK',  detail:'BTC+ETH açık = 2 (limit: 2)' },
    { id:'K4', name:'Sektör Konsantrasyonu ≤ %25',     status:'WARN',detail:'Kripto: %27 (2 puan üstünde)' },
    { id:'K5', name:'Likidite: Hacim > Poz × 10',      status:'OK',  detail:'Tüm pozisyonlar likit' },
    { id:'P1', name:'Günlük DD ≤ -2%',                 status:'OK',  detail:'Günlük DD: -0.8%' },
    { id:'P2', name:'Haftalık DD ≤ -4%',               status:'OK',  detail:'Haftalık DD: -2.1%' },
    { id:'P3', name:'Win Rate ≥ 50% (6ay)',            status:'OK',  detail:'Rolling win rate: 58%' },
  ];

  const okCount   = rules.filter(r=>r.status==='OK').length;
  const warnCount = rules.filter(r=>r.status==='WARN').length;
  const failCount = rules.filter(r=>r.status==='FAIL').length;

  return (
    <div>
      <div style={{ ...S.grid(4), marginBottom:16 }}>
        {[
          { label:'UYUMLU', val:okCount,   color:'#3fb950' },
          { label:'UYARI',  val:warnCount, color:'#e3b341' },
          { label:'İHLAL',  val:failCount, color:'#f85149' },
          { label:'TOPLAM KURAL', val:rules.length, color:'#58a6ff' },
        ].map(k => (
          <div key={k.label} style={{ ...S.card, borderLeft:`3px solid ${k.color}` }}>
            <div style={S.cardTitle}>{k.label}</div>
            <div style={{ fontSize:32, fontWeight:700, color:k.color }}>{k.val}</div>
          </div>
        ))}
      </div>

      <div style={S.card}>
        <div style={S.cardTitle}>KURAL UYUMLULUK RAPORU</div>
        <table style={S.table}>
          <thead><tr>
            {['Kural ID','Kural','Durum','Detay'].map(h=><th key={h} style={S.th}>{h}</th>)}
          </tr></thead>
          <tbody>
            {rules.map(r => (
              <tr key={r.id} style={{ background: r.status==='FAIL'?'#1a0d0d':r.status==='WARN'?'#1a1a0d':'#0d1117' }}>
                <td style={{ ...S.td, fontFamily:'monospace', fontWeight:700, color:'#8b949e' }}>{r.id}</td>
                <td style={{ ...S.td, fontSize:12 }}>{r.name}</td>
                <td style={S.td}>
                  <span style={S.badge(r.status==='OK'?'#3fb950':r.status==='WARN'?'#e3b341':'#f85149')}>
                    {r.status === 'OK' ? '✓ OK' : r.status === 'WARN' ? '⚠ UYARI' : '✗ İHLAL'}
                  </span>
                </td>
                <td style={{ ...S.td, fontSize:11, color:'#8b949e' }}>{r.detail}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────
// WATCHLIST TAB
// ─────────────────────────────────────────────────────────
function WatchlistTab() {
  const [watchlist, setWatchlist] = useState([]);
  const [signals, setSignals] = useState({});
  const [newSymbol, setNewSymbol] = useState('');
  const [scanning, setScanning] = useState(false);

  const fetchData = useCallback(async () => {
    const [w, s] = await Promise.all([apiFetch('/watchlist'), apiFetch('/signals')]);
    if (w?.data) setWatchlist(w.data);
    if (s?.data) setSignals(s.data);
  }, []);
  useEffect(() => { fetchData(); }, [fetchData]);

  const addSymbol = async () => {
    if (!newSymbol) return;
    await apiPost('/watchlist', { symbol: newSymbol, source: 'manual' });
    setNewSymbol(''); fetchData();
  };
  const removeSymbol = async (sym) => { await apiDelete(`/watchlist/${sym}`); fetchData(); };
  const runScan = async () => {
    setScanning(true);
    await apiPost('/weekly-scan');
    setTimeout(() => { fetchData(); setScanning(false); }, 5000);
  };

  return (
    <div>
      <div style={{ display:'flex', gap:12, marginBottom:16 }}>
        <input style={{ ...S.input, maxWidth:200 }} value={newSymbol} onChange={e => setNewSymbol(e.target.value.toUpperCase())} placeholder="Sembol (BTC/USDT)" onKeyDown={e => e.key==='Enter' && addSymbol()} />
        <button style={S.btn('blue')} onClick={addSymbol}>+ Ekle</button>
        <button style={S.btn('primary')} onClick={runScan}>{scanning ? 'Taranıyor...' : 'Haftalık Tarama'}</button>
        <button style={S.btn('secondary')} onClick={fetchData}>↻ Yenile</button>
      </div>
      <div style={S.card}>
        <div style={S.cardTitle}>İZLEME LİSTESİ ({watchlist.length} enstrüman)</div>
        <table style={S.table}>
          <thead><tr>
            {['Sembol','Sınıf','Sektör','Kaynak','Fiyat','ATR%','1D Sinyal','4H Sinyal','ADX','Yön','Kaldır'].map(h => <th key={h} style={S.th}>{h}</th>)}
          </tr></thead>
          <tbody>
            {watchlist.map(w => {
              const symKey = w.symbol.replace('/','_').replace('.','_');
              const sig = signals[symKey] || {};
              return (
                <tr key={w.symbol} style={{ background:'#0d1117' }}>
                  <td style={{ ...S.td, fontWeight:700, color:'#58a6ff' }}>{w.symbol}</td>
                  <td style={S.td}>{w.asset_class||'—'}</td>
                  <td style={S.td}>{w.sector||'—'}</td>
                  <td style={S.td}><span style={S.badge(w.source==='scan'?'#3fb950':'#58a6ff')}>{w.source}</span></td>
                  <td style={{ ...S.td, fontFamily:'monospace' }}>{sig.price ? fmt(sig.price, sig.price>100?2:4) : '—'}</td>
                  <td style={{ ...S.td, color:(sig.atr_pct||0)>3?'#f85149':'#e3b341' }}>{sig.atr_pct ? fmt(sig.atr_pct)+'%' : '—'}</td>
                  <td style={S.td}><span style={{ fontSize:10 }}>{sig.signal_status_1d||'—'}</span></td>
                  <td style={S.td}>{sig.signal_4h ? <span style={S.badge('#3fb950')}>{sig.signal_4h}</span> : <span style={{ fontSize:10, color:'#6e7681' }}>BEKLE</span>}</td>
                  <td style={{ ...S.td, fontFamily:'monospace' }}>{sig.adx ? fmt(sig.adx,1) : '—'}</td>
                  <td style={S.td}><span style={{ color:sig.direction_1d==='bullish'?'#3fb950':sig.direction_1d==='bearish'?'#f85149':'#6e7681', fontSize:10, fontWeight:600 }}>{(sig.direction_1d||'—').toUpperCase()}</span></td>
                  <td style={S.td}><button style={{ ...S.btn('secondary'), fontSize:10 }} onClick={() => removeSymbol(w.symbol)}>✗</button></td>
                </tr>
              );
            })}
            {watchlist.length === 0 && <tr><td colSpan={11} style={{ ...S.td, textAlign:'center', color:'#6e7681' }}>Watchlist boş — Haftalık Tarama yapın veya sembol ekleyin</td></tr>}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────
// MAIN APP
// ─────────────────────────────────────────────────────────
const ALL_SYMBOLS = {};
Object.values(SYMBOL_DB).forEach(market => {
  Object.values(market).forEach(sector => {
    sector.forEach(s => { ALL_SYMBOLS[s.code] = s; });
  });
});
Object.entries(INDEX_DB).forEach(([k,v]) => { ALL_SYMBOLS[k] = v; });

const TABS = [
  { key:'dashboard', label:'Dashboard' },
  { key:'portfoy',   label:'Portföy'  },
  { key:'yeniIslem', label:'Yeni İşlem'},
  { key:'analytics', label:'Analytics' }, // YENİ
  { key:'watchlist', label:'Watchlist' },
  { key:'riskAnaliz',label:'Risk Analiz'},
  { key:'gecmis',    label:'Geçmiş'   },
  { key:'grafikler', label:'Grafikler' },
  { key:'uyumluluk', label:'Uyumluluk' },
];

export default function App() {
  const [tab, setTab] = useState('dashboard');
  const [time, setTime] = useState(new Date());
  const engine = new RiskEngine({ portfolioSize: 100000 });
  const prices = usePrices(ALL_SYMBOLS);
  const apiConnected = Object.values(prices).some(p => p.price !== null);
  
  const timeStr = time.toLocaleTimeString('tr-TR', { hour12: false });
  const dateStr = time.toLocaleDateString('tr-TR', { day:'2-digit', month:'2-digit', year:'numeric' });

  const renderTab = () => {
    switch (tab) {
      case 'dashboard':  return <DashboardTab  prices={prices} engine={engine} />;
      case 'portfoy':    return <PortfoyTab     prices={prices} engine={engine} />;
      case 'yeniIslem':  return <YeniIslemTab   prices={prices} engine={engine} />;
      case 'watchlist':  return <WatchlistTab />;
      case 'riskAnaliz': return <RiskAnalizTab  engine={engine} />;
      case 'gecmis':     return <GecmisTab />;
      case 'grafikler':  return <GrafiklerTab   prices={prices} />;
      case 'uyumluluk':  return <UyumlulukTab   engine={engine} />;
      case 'analytics': return <AnalyticsTab />;
      default:           return null;
    }
  };

  return (
    <div style={S.app}>
      {/* HEADER */}
      <div style={S.header}>
        <div style={S.headerLeft}>
          <div>
            <div style={S.logo}>▪ RISKPLATFORM</div>
            <div style={S.logoSub}>KARAR DESTEK SİSTEMİ</div>
          </div>
          <span style={S.liveTag}>● CANLI</span>
          <span style={{ ...S.liveTag, background:'#0d1a2a', color:'#58a6ff', border:'1px solid #1a3a5a' }}>
            v2.0
          </span>
        </div>
        <div style={S.headerRight}>
          <span style={S.headerTime}>{timeStr} · {dateStr}</span>
          <span style={{ fontSize:11, color:'#6e7681' }}>$100,000 AUM</span>
        </div>
      </div>

      {/* TICKER */}
      <TickerBar prices={prices} />

      {/* NAV */}
      <div style={S.nav}>
        {TABS.map(t => (
          <button key={t.key} style={S.navBtn(tab===t.key)} onClick={() => setTab(t.key)}>
            {t.label}
          </button>
        ))}
      </div>

      {/* CONTENT */}
      <div style={S.content}>
        {renderTab()}
      </div>
    </div>
  );
}