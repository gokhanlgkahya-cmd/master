"""SASA temel seri oluşturma (deneme). Kaynak: İş Yatırım MaliTablo + HisseTekil."""
import json, glob, os, sys
import numpy as np, pandas as pd

RAW = sys.argv[1]
OUT = sys.argv[2]

def load(ex):
    rows = {}
    for f in sorted(glob.glob(os.path.join(RAW, f"{ex}_*.json"))):
        y0, p0 = map(int, os.path.basename(f)[len(ex) + 1:-5].split("_"))
        per = [(y, p) for y in range(2019, 2027) for p in (3, 6, 9, 12) if (y, p) <= (2026, 9)]
        i = per.index((y0, p0)); chunk = per[i:i + 4]
        while len(chunk) < 4: chunk.append(chunk[-1])
        for v in json.load(open(f))["value"]:
            for k, yp in enumerate(chunk, 1):
                rows.setdefault(yp, {})[v["itemCode"]] = float(v[f"value{k}"] or 0)
    df = pd.DataFrame.from_dict(rows, orient="index").sort_index()
    df.index = pd.MultiIndex.from_tuples(df.index, names=["y", "p"])
    return df

FLOW = {"sales": "3C", "gross": "3D", "mkt": "3DA", "ga": "3DB", "rd": "3DC", "ebit": "3DF",
        "np": "3Z", "da": "4B", "cfo": "4C", "capex": "4CAI", "fcf": "4CB"}
BAL = {"cash": "1AA", "std": "2AA", "ltd": "2BA", "eq": "2O", "netfx": "4BE", "ta": "1BL"}

def quarterly(df):
    q = pd.DataFrame(index=df.index)
    for n, c in FLOW.items():
        s = df[c].copy()
        prev = s.groupby(level="y").shift(1)
        q[n] = np.where(df.index.get_level_values("p") == 3, s, s - prev)
    for n, c in BAL.items(): q[n] = df[c]
    q["ebitda"] = q.gross + q.mkt + q.ga + q.rd + q.da          # çekirdek FAVÖK (diğer faaliyet hariç)
    q["debt"] = q["std"] + q.ltd
    q["netdebt"] = q.debt - q.cash
    return q

usd, tr = quarterly(load("USD")), quarterly(load("TRY"))
idx = [f"{y}Q{p//3}" for y, p in usd.index]
usd.index = tr.index = idx

ttm = lambda s: s.rolling(4).sum()
m = pd.DataFrame(index=idx)
for n in ["sales", "ebitda", "np", "cfo", "fcf", "capex"]: m[f"{n}_ttm_usd"] = ttm(usd[n])
m["sales_q_usd"] = usd.sales
m["gross_margin"] = ttm(usd.gross) / m.sales_ttm_usd
m["ebitda_margin"] = m.ebitda_ttm_usd / m.sales_ttm_usd
m["net_margin"] = m.np_ttm_usd / m.sales_ttm_usd
m["roe"] = m.np_ttm_usd / ((usd["eq"] + usd["eq"].shift(4)) / 2)
m["netdebt_usd"] = usd.netdebt
m["nd_ebitda"] = usd.netdebt / m.ebitda_ttm_usd
m["netfx_eq"] = tr.netfx / tr["eq"]
m["cfo_ebitda"] = m.cfo_ttm_usd / m.ebitda_ttm_usd
m["fcf_margin"] = m.fcf_ttm_usd / m.sales_ttm_usd
m["sales_yoy"] = m.sales_ttm_usd.pct_change(4)
m["ebitda_yoy"] = m.ebitda_ttm_usd.pct_change(4)

# Piyasa değeri: yaklaşık KAP yayın tarihinde (lookahead önlemi; üretimde KAP zaman damgası kullanılacak)
px = pd.DataFrame(json.load(open(os.path.join(RAW, "px.json")))["value"])
px["date"] = pd.to_datetime(px.HGDG_TARIH, format="%d-%m-%Y")
px = px.set_index("date").sort_index()
lag = {1: (5, 15), 2: (8, 20), 3: (11, 15), 4: (3, 10)}
pub = []
for s in idx:
    y, qn = int(s[:4]), int(s[-1]); mo, d = lag[qn]
    pub.append(pd.Timestamp(y + (1 if qn == 4 else 0), mo, d))
m["pub_date"] = pub
mc = [px.PD_USD.asof(d) if d <= px.index[-1] else np.nan for d in pub]
m["mcap_usd"] = mc
m["ep"] = m.np_ttm_usd / m.mcap_usd
m["ev_ebitda"] = (m.mcap_usd + m.netdebt_usd) / m.ebitda_ttm_usd
m["pb"] = m.mcap_usd / usd["eq"]

m.to_csv(os.path.join(OUT, "sasa_series.csv"))
px[["HGDG_KAPANIS", "PD_USD", "DD_DEGER"]].to_csv(os.path.join(OUT, "sasa_price.csv"))
pd.set_option("display.width", 250, "display.max_columns", 40)
show = m[["sales_ttm_usd", "ebitda_ttm_usd", "np_ttm_usd", "ebitda_margin", "net_margin", "roe",
          "netdebt_usd", "nd_ebitda", "netfx_eq", "cfo_ebitda", "fcf_ttm_usd", "sales_yoy",
          "mcap_usd", "ep", "ev_ebitda", "pb"]].copy()
for c in ["sales_ttm_usd", "ebitda_ttm_usd", "np_ttm_usd", "netdebt_usd", "fcf_ttm_usd", "mcap_usd"]:
    show[c] = (show[c] / 1e6).round(0)
print(show.round(3).to_string())
print("Son fiyat:", px.index[-1].date(), "PD_USD (mn):", round(px.PD_USD.iloc[-1] / 1e6))
