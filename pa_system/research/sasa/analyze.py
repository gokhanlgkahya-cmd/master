"""SASA temel skor + grafik (deneme). Girdi: build.py çıktısı."""
import sys, os, json
import numpy as np, pandas as pd
import matplotlib; matplotlib.use("Agg")
import matplotlib.pyplot as plt

OUT, RAW = sys.argv[1], sys.argv[2]
m = pd.read_csv(os.path.join(OUT, "sasa_series.csv"), index_col=0, parse_dates=["pub_date"])
px = pd.read_csv(os.path.join(OUT, "sasa_price.csv"), index_col=0, parse_dates=True)

# TMS 29 kırılması: 2022Q4 ve 2023Q4 yıl sonu tabloları düzeltilmiş, ara dönemler nominal.
# Akım (TTM) değerleri penceresi bu çeyreklere değen dönemlerde geçersiz.
BROKEN = {"2022Q4", "2023Q4"}
q = list(m.index)
valid_ttm = pd.Series([not any(q[j] in BROKEN for j in range(max(0, i - 3), i + 1)) for i in range(len(q))], index=q)
flow_cols = [c for c in m.columns if c.endswith("_ttm_usd") or c in
             ("gross_margin", "ebitda_margin", "net_margin", "roe", "nd_ebitda", "cfo_ebitda",
              "fcf_margin", "sales_yoy", "ebitda_yoy", "ep", "ev_ebitda")]
m.loc[~valid_ttm, flow_cols] = np.nan
m.loc[m.index < "2024Q4", ["sales_yoy", "ebitda_yoy"]] = m.loc[m.index < "2024Q4", ["sales_yoy", "ebitda_yoy"]].where(valid_ttm.shift(4, fill_value=False))
# Özkaynak (P/D) dönem içi kırıldığı için yalnız TMS29 dönemi kullanılır
m.loc[m.index < "2024Q1", "pb"] = np.nan

ERA = m.index >= "2024Q4"   # tutarlı TMS 29 dönemi (TTM penceresi tamamen içinde)

def direction(s, higher_better=True):
    s = s.dropna()
    s = s[s.index >= "2024Q4"]
    if len(s) < 5: return np.nan, np.nan
    last4 = s.iloc[-4:].values
    slope = np.polyfit(range(4), last4, 1)[0]
    scale = np.nanstd(s.values) or 1
    z = slope / scale * (1 if higher_better else -1)
    d = 0.5 + 0.5 * np.tanh(z)
    ch = np.diff(s.values) * (1 if higher_better else -1)
    c = (ch > 0).mean()
    return 0.5 * d + 0.5 * c, z

def level(s, higher_better=True):
    s = s.dropna()
    if len(s) < 4: return np.nan
    pct = (s < s.iloc[-1]).mean() + 0.5 * (s == s.iloc[-1]).mean()
    return pct if higher_better else 1 - pct

METRICS = {  # sütun: [(metrik, yüksek iyi mi)]
    "Değerleme (20)": [("ep", True), ("ev_ebitda", False)],
    "Kârlılık (20)": [("roe", True), ("ebitda_margin", True)],
    "Büyüme (25)": [("sales_yoy", True), ("ebitda_yoy", True)],
    "Bilanço (20)": [("nd_ebitda", False), ("netfx_eq", True)],
    "Kalite (15)": [("cfo_ebitda", True), ("fcf_margin", True)],
}
W = {"Değerleme (20)": 20, "Kârlılık (20)": 20, "Büyüme (25)": 25, "Bilanço (20)": 20, "Kalite (15)": 15}
rows, pill = [], {}
for p, ms in METRICS.items():
    ds, ls = [], []
    for col, hb in ms:
        d, z = direction(m[col], hb); l = level(m[col], hb)
        rows.append((p, col, m[col].dropna().iloc[-1], l, d, z))
        ds.append(d); ls.append(l)
    D, L = np.nanmean(ds), np.nanmean(ls)
    pill[p] = (0.6 * D + 0.4 * L, D, L)
score = sum(W[p] * pill[p][0] for p in W) / sum(W.values()) * 100
dir_strength = sum(W[p] * pill[p][1] for p in W) / sum(W.values())
mult = 1 + 0.15 * (score - 50) / 50
det = pd.DataFrame(rows, columns=["sutun", "metrik", "son", "seviye", "yon", "egim_z"])
print(det.round(3).to_string(index=False))
for p, (s, d, l) in pill.items(): print(f"{p:16s} skor={s*100:5.1f} yon={d*100:5.1f} seviye={l*100:5.1f}")
arrow = "↑" if dir_strength > 0.6 else ("↓" if dir_strength < 0.4 else "→")
print(f"TEMEL SKOR={score:.1f}  YON={arrow} ({dir_strength*100:.0f})  CARPAN={mult:.3f}")
json.dump({"score": score, "direction": arrow, "dir_strength": dir_strength, "mult": mult,
           "pillars": {p: v[0] for p, v in pill.items()}}, open(os.path.join(OUT, "score.json"), "w"), ensure_ascii=False)

# ---------- Grafikler ----------
x = np.arange(len(q)); lab = [s if s.endswith("Q4") or s == q[-1] else "" for s in q]
def shade(ax):
    for b in BROKEN:
        i = q.index(b); ax.axvspan(i - 3.5, i + 0.5, color="#999", alpha=0.12, lw=0)
def fmt(ax, title):
    ax.set_title(title, fontsize=10, loc="left"); ax.set_xticks(x); ax.set_xticklabels(lab, fontsize=7, rotation=45)
    ax.grid(alpha=0.25); shade(ax)

fig, axs = plt.subplots(3, 2, figsize=(13, 12))
ax = axs[0, 0]
ax.plot(x, m.sales_ttm_usd / 1e6, "-o", ms=3, color="#2a6fdb", label="Satış TTM")
ax2 = ax.twinx(); ax2.plot(x, m.ebitda_ttm_usd / 1e6, "-o", ms=3, color="#e07b00", label="Çekirdek FAVÖK TTM")
ax.set_ylabel("Satış (mn USD)"); ax2.set_ylabel("FAVÖK (mn USD)"); fmt(ax, "1) Satış ve çekirdek FAVÖK (TTM, USD)")
ax.legend(loc="upper left", fontsize=8); ax2.legend(loc="lower right", fontsize=8)

ax = axs[0, 1]
for c, l, col in [("gross_margin", "Brüt", "#2a6fdb"), ("ebitda_margin", "FAVÖK", "#e07b00"), ("net_margin", "Net", "#c0392b")]:
    ax.plot(x, m[c] * 100, "-o", ms=3, label=l, color=col)
ax.axhline(0, color="k", lw=0.6); ax.set_ylabel("%"); fmt(ax, "2) Marjlar (TTM)"); ax.legend(fontsize=8)

ax = axs[1, 0]
ax.bar(x, m.netdebt_usd / 1e6, color="#7f8c8d", alpha=0.6, label="Net borç (mn USD)")
ax2 = ax.twinx(); ax2.plot(x, m.nd_ebitda, "-o", ms=3, color="#c0392b", label="Net borç / FAVÖK (x)")
fmt(ax, "3) Net borç ve kaldıraç"); ax.legend(loc="upper left", fontsize=8); ax2.legend(loc="upper right", fontsize=8)

ax = axs[1, 1]
ax.plot(x, m.netfx_eq * 100, "-o", ms=3, color="#8e44ad")
ax.axhline(0, color="k", lw=0.6); ax.set_ylabel("% özkaynak"); fmt(ax, "4) Net döviz pozisyonu / özkaynak")

ax = axs[2, 0]
ax.bar(x - 0.2, m.cfo_ttm_usd / 1e6, width=0.4, color="#27ae60", label="Faaliyet nakdi TTM")
ax.bar(x + 0.2, m.fcf_ttm_usd / 1e6, width=0.4, color="#16a085", alpha=0.6, label="Serbest nakit akımı TTM")
ax.axhline(0, color="k", lw=0.6); ax.set_ylabel("mn USD"); fmt(ax, "5) Nakit akımı (TTM, USD)"); ax.legend(fontsize=8)

ax = axs[2, 1]
mc = px.PD_USD.resample("ME").last() / 1e6
ax.plot(mc.index, mc.values, color="#34495e", label="Piyasa değeri (mn USD, aylık)")
ax.set_ylabel("Piyasa değeri (mn USD)")
ax2 = ax.twinx()
ev = m.dropna(subset=["ebitda_ttm_usd"])
for k, seg in enumerate([ev[ev.index < "2023Q1"], ev[ev.index >= "2024Q4"]]):
    ax2.plot(seg.pub_date, seg.ebitda_ttm_usd / 1e6, "o-", ms=3, color="#e07b00", label="FAVÖK TTM (yayın tarihinde)" if k == 0 else None)
ax2.set_ylabel("FAVÖK (mn USD)")
ax.set_title("6) Fiyat ve FAVÖK: temel uyumsuzluk", fontsize=10, loc="left"); ax.grid(alpha=0.25)
ax.legend(loc="upper left", fontsize=8); ax2.legend(loc="upper right", fontsize=8)

fig.suptitle(f"SASA — Temel seri (USD, TTM) · Temel skor {score:.0f}/100 · Yön {arrow} · Çarpan {mult:.2f}\n"
             "Gri bant: TMS 29 kırılması nedeniyle akım verisi geçersiz (2022Q4–2024Q3)", fontsize=11)
fig.tight_layout(rect=[0, 0, 1, 0.95])
fig.savefig(os.path.join(OUT, "sasa_temel.png"), dpi=130)

v = m[["ev_ebitda", "pb", "ep"]].copy()
print(v.dropna(how="all").round(2).tail(10).to_string())
