#!/usr/bin/env python3
"""Check the engine's entry fills against the RAW sources (not the derived chain): for every trade of a few rules, the
contract printed in the trade reason must exist in the raw file on the entry date, have traded (NSE Contracts > 0 /
US volume > 0) and its raw close (NSE) or (bid+ask)/2 (US) must equal the printed sale price (2 decimals).
Raw files are read one monthly (NSE) / yearly (US) file at a time.  Usage: spot_check.py PORT us|in OUT_JSON"""
import collections, json, os, re, sys, urllib.parse, urllib.request
import pyarrow.parquet as pq

PORT, MARKET, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
NSE = os.path.expanduser("~/.local/share/strategy-vault/fno_parquet")
US = "/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Manan Pujara/#Data/us data/gh_options_eod_spy_qqq_iwm_2008-2025_2026-08-18/raw"
CASES = {"in": [("oiss_short_put", "NIFTY 50", "2001-06-04", "2026-10-07"), ("oiss_short_call", "NIFTYBANK", "2005-06-10", "2026-10-07"),
                ("spmt_eom_puts", "NIFTY 50", "2001-06-04", "2026-10-07"), ("ccl08_weekly_rsi_options", "NIFTYBANK", "2005-06-10", "2026-10-07"),
                ("mahrs_covered_calls", "NIFTY 50", "2001-06-04", "2026-10-07")],
         "us": [("otvt_cvr1_sell_puts", "SPY", "2008-01-02", "2025-12-12"), ("ccl08_weekly_rsi_options", "QQQ", "2011-03-23", "2025-12-12"),
                ("mahrs_covered_calls", "IWM", "2008-01-02", "2025-12-12")]}[MARKET]
SYM = {"NIFTY 50": "NIFTY", "NIFTYBANK": "BANKNIFTY"}
PAT = [re.compile(r"(?P<t>[PC]) (?P<k>[\d.]+) exp (?P<e>\d{8}) sold (?P<d>\d{4}-\d\d-\d\d) (?P<p>[\d.]+)"),  # OISS
       re.compile(r"sold (?P<k>[\d.]+) (?P<t>put|call) exp (?P<e>\d{8}) (?:at (?P<p>[\d.]+) on (?P<d>\d{4}-\d\d-\d\d)|on (?P<d2>\d{4}-\d\d-\d\d) at (?P<p2>[\d.]+))")]


def entries(id_, t, a, b):
    q = urllib.parse.urlencode({"id": id_, "ticker": t, "from": a, "to": b})
    js = json.loads(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/api/guru/connors/run?{q}", timeout=900).read())
    out = []
    for x in js["trades"]:
        for p in PAT:
            m = p.search(x["reason"])
            if m:
                g = m.groupdict()
                out.append((g.get("d") or g.get("d2"), int(g["e"]), "P" if g["t"] in ("P", "put") else "C", float(g["k"]), float(g.get("p") or g.get("p2"))))
                break
        else:
            out.append(("unparsed", x["reason"][:120]))
    return out


res = {"cases": [], "checked": 0, "bad": 0}
for id_, t, a, b in CASES:
    es = entries(id_, t, a, b)
    bad, by = [], collections.defaultdict(list)
    for e in es:
        if e[0] == "unparsed":
            bad.append(e); continue
        by[e[0][:7] if MARKET == "in" else e[0][:4]].append(e)
    for key, group in sorted(by.items()):
        if MARKET == "in":
            tb = pq.read_table(f"{NSE}/year={key[:4]}/{key}.parquet", filters=[("Symbol", "=", SYM[t]), ("Instrument", "=", "OPTIDX")]).to_pylist()
            raw = {(r["Date"].isoformat(), int(r["ExpiryDate"].strftime("%Y%m%d")), "P" if r["OptionType"] == "PE" else "C", float(r["StrikePrice"])): r for r in tb}
        else:
            dates = sorted({e[0] for e in group})
            tb = pq.read_table(f"{US}/{t.lower()}/options_{key}.parquet", columns=["date", "expiration", "strike", "type", "bid", "ask", "volume"],
                               filters=[("date", "in", dates)]).to_pylist()
            raw = {(r["date"], int(r["expiration"].replace("-", "")), "P" if r["type"] == "put" else "C", float(r["strike"])): r for r in tb}
        for d, e, typ, k, p in group:
            r = raw.get((d, e, typ, k))
            if r is None:
                bad.append((d, e, typ, k, p, "contract not in raw file")); continue
            if MARKET == "in":
                ok = r["Contracts"] > 0 and abs(r["Close"] - p) < 0.0051
                why = f"raw close {r['Close']} contracts {r['Contracts']}"
            else:
                mid = (r["bid"] + r["ask"]) / 2
                ok = r["volume"] > 0 and 0 < r["bid"] <= r["ask"] and abs(mid - p) < 0.0051
                why = f"raw bid {r['bid']} ask {r['ask']} volume {r['volume']}"
            if not ok:
                bad.append((d, e, typ, k, p, why))
    res["cases"].append({"id": id_, "ticker": t, "entries": len(es), "bad": len(bad), "first_bad": [str(x) for x in bad[:5]]})
    res["checked"] += len(es); res["bad"] += len(bad)
    print(res["cases"][-1], flush=True)
json.dump(res, open(OUT, "w"), indent=1)
print("TOTAL checked", res["checked"], "bad", res["bad"])
