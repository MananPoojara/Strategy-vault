#!/usr/bin/env python3
"""Intraday wave 2a: compare the engine's daily-bar trade ledgers (HTTP API) with crabel_w2a_ref.py.
Fields: direction, entry date, entry price, exit date, exit price, open flag; every trade, every window.
usage: compare_w2a.py PORT us|in JOBS.json OUT.json   (JOBS: {"id": [params, ...]})"""
import bisect, json, os, sys, urllib.error, urllib.parse, urllib.request
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, "/home/server/.cache/strategy-vault-webapp/claude-continuation-20261020/authors/kaeppel/admission")
import crabel_w2a_ref as R
from kpss_ref import load_instrument, load_stock  # plain CSV loaders (shared with the other admissions)

PORT, MARKET, JOBS, OUT = sys.argv[1], sys.argv[2], json.load(open(sys.argv[3])), sys.argv[4]
D = "/home/server/.cache/strategy-vault-webapp/data"
OPEN_FROM = {"SPX": "2014-01-01", "NDX": "2001-01-01", "DJI": "2016-01-01"}  # cash-index opens are stale before these
if MARKET == "us":
    BARS = {t: load_instrument(f"{D}/us_data.csv", t) for t in ("SPY", "QQQ", "SPX", "NDX", "DJI")}
    U = "/home/server/.cache/strategy-vault/us-market-data/daily/"
    STK = {"AAPL": "c220fe04bfd335ae85d6e1ad19464b3aa58026473f102f47012d4618f3bc5319", "MSFT": "264221e30a9c4fb6bd46a082532526fd9638e617f23af048c56ed7aca9e0783f"}
    API = {t: f"basket:us_sp500:{t}" for t in STK}
else:
    BARS = {t: load_instrument(f"{D}/idx_data.csv", t) for t in ("NIFTY 50", "NIFTYBANK")}
    U = "/home/server/.cache/strategy-vault/market-data-repaired-20261015-v2/daily/nifty50/"
    STK = {"RELIANCE": "9e95498193cc63fcd447", "TCS": "d628d9926390ba4143dd"}
    API = {t: f"basket:nifty50:{t}" for t in STK}
for t, h in STK.items():
    BARS[t] = load_stock(f"{U}{h}.csv")


def api(id_, t, a, b, prm):
    q = {"id": id_, "ticker": t, "from": a, "to": b}
    q.update({f"p.{k}": v for k, v in prm.items()})
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/api/guru/connors/run?{urllib.parse.urlencode(q)}", timeout=600) as r:
            return json.loads(r.read().decode())
    except urllib.error.HTTPError as e:
        return f"HTTP {e.code}: " + e.read().decode()


def same(x, y):
    return abs(x - y) <= 2.01e-6 * max(1.0, abs(x)) if isinstance(x, float) and isinstance(y, float) else x == y


rows, tot = [], {"windows": 0, "trades": 0, "mismatches": 0, "refused": 0}
for id_, vs in JOBS.items():
    for t, B in BARS.items():
        dates = [x[0] for x in B]
        for prm in vs:
            a_ = max(OPEN_FROM.get(t, dates[0]), dates[10]); b_ = dates[-1]
            got = api(id_, API.get(t, t), a_, b_, prm)
            if isinstance(got, str) or "trades" not in got:
                tot["refused"] += 1; rows.append(dict(ticker=t, id=id_, params=prm, refused=str(got)[:300])); print(rows[-1]); continue
            a_ = got.get("from") or a_
            a = bisect.bisect_left(dates, a_); b = bisect.bisect_right(dates, b_) - 1
            rust = [(int(x["dir"]), x["entry"], x["legs"][0][1], x["exit"], x["exit_px"], bool(x["open"])) for x in got["trades"]]
            ref = [(x["dir"], dates[x["ei"]], x["epx"], dates[x["xi"]], x["xpx"], x["open"]) for x in R.backtest(id_, B, a, b, prm)]
            mm = [(k, r1, r2) for k, (r1, r2) in enumerate(zip(rust, ref)) if not all(same(p, q) for p, q in zip(r1, r2))]
            n = len(mm) + abs(len(rust) - len(ref))
            tot["windows"] += 1; tot["trades"] += len(ref); tot["mismatches"] += n
            rows.append(dict(ticker=t, id=id_, params=prm, window=f"{dates[a]}..{dates[b]}", rust=len(rust), ref=len(ref), mismatches=n, first=str(mm[:2])))
            print(t, id_, prm, rows[-1]["window"], f"rust={len(rust)} ref={len(ref)} mismatches={n}", rows[-1]["first"] if n else "", flush=True)
json.dump(dict(totals=tot, rows=rows), open(OUT, "w"), indent=1)
print("TOTAL", tot)
