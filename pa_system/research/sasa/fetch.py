"""SASA ham verisini İş Yatırım'dan indirir (deneme).

Kullanım:  python fetch.py <raw_klasoru>
Sonra:     python build.py <raw_klasoru> out  &&  python analyze.py out <raw_klasoru>

Not: Resmî API değildir; uç noktalar değişebilir. Üretimde veri katmanı (IMPLEMENTATION_PLAN F1) kullanılacak.
"""
import json, os, sys, urllib.request

RAW = sys.argv[1]
os.makedirs(RAW, exist_ok=True)
BASE = "https://www.isyatirim.com.tr/_layouts/15/IsYatirim.Website/Common/Data.aspx"
UA = {"User-Agent": "Mozilla/5.0"}

def get(url, out):
    with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=60) as r:
        data = r.read()
    json.loads(data)  # geçerli JSON mu
    open(os.path.join(RAW, out), "wb").write(data)

periods = [(y, p) for y in range(2019, 2027) for p in (3, 6, 9, 12) if (y, p) <= (2026, 9)]
for ex in ("TRY", "USD"):
    for i in range(0, len(periods), 4):
        chunk = periods[i:i + 4]
        while len(chunk) < 4: chunk.append(chunk[-1])
        q = "&".join(f"year{j+1}={y}&period{j+1}={p}" for j, (y, p) in enumerate(chunk))
        get(f"{BASE}/MaliTablo?companyCode=SASA&exchange={ex}&financialGroup=XI_29&{q}",
            f"{ex}_{chunk[0][0]}_{chunk[0][1]}.json")
get(f"{BASE.replace('IsYatirim.Website', 'Isyatirim.Website')}/HisseTekil?hisse=SASA&startdate=01-01-2019&enddate=20-11-2026", "px.json")
print("tamam:", RAW)
