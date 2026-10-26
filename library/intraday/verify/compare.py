#!/usr/bin/env python3
"""Compare the engine's trade ledgers (HTTP API on a private port) with indep_ref.py, field by field.
Usage: compare.py PORT us|in OUT_JSON"""
import bisect, json, math, os, sys, urllib.error, urllib.parse, urllib.request
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import indep_ref as R

PORT, MARKET, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
D = "/home/server/.cache/strategy-vault-webapp/data"
IDS = ["ss_momentum_pinball", "ss_80_20", "ats_15min_adx_breakout", "ptjdt_adx15_breakout"]
WIN = {"us": {"SPY": [("2000-01-03", "2010-12-31"), ("2011-01-03", "2025-10-21")],
              "SPX": [("2008-01-02", "2016-12-30"), ("2017-01-03", "2026-08-14")]},
       "in": {"NIFTY 50": [("2008-01-01", "2016-12-30"), ("2017-01-02", "2026-10-07")],
              "NIFTYBANK": [("2010-03-02", "2017-12-29"), ("2018-01-01", "2026-04-22")]}}[MARKET]
PATH = f"{D}/us_data.csv" if MARKET == "us" else f"{D}/idx_data.csv"
OPEN_CUT = {"SPX": "2014-01-01"}  # 80-20 reads the setup day's open: SPX opens are real from 2014 (gate clamp)


def rnd(v):
    return math.copysign(math.floor(abs(v) * 1e6 + 0.5), v) / 1e6


def kind(r):
    r = r.lower()
    return "open" if r == "open" else "stop" if "stop" in r else "target" if "limit" in r else "close"


def api(id_, t, a, b):
    q = urllib.parse.urlencode({"id": id_, "ticker": t, "from": a, "to": b})
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/api/guru/connors/run?{q}", timeout=900) as r:
            return json.loads(r.read().decode())
    except urllib.error.HTTPError as e:
        return f"HTTP {e.code}: " + e.read().decode()


res = {"windows": [], "ambiguous": {}, "totals": {"trades": 0, "mismatches": 0, "refused": 0}}
for t, wins in WIN.items():
    Dd = R.load_daily(PATH, t)
    dates = [x[0] for x in Dd]
    M = R.Minutes(t)
    for id_ in IDS:
        for a_, b_ in wins:
            if id_ == "ss_80_20" and t in OPEN_CUT:
                if b_ < OPEN_CUT[t]:
                    continue
                a_ = max(a_, OPEN_CUT[t])
            got = api(id_, t, a_, b_)
            a = bisect.bisect_left(dates, a_); b = bisect.bisect_right(dates, b_) - 1
            amb = []
            ref = [(x["dir"], x["entry"], rnd(x["entry_px"]), x["exit"], rnd(x["exit_px"]), x["reason"], x["open"]) for x in R.backtest(id_, Dd, M, a, b, t, amb)]
            w = {"ticker": t, "id": id_, "from": dates[a], "to": dates[b], "ref_trades": len(ref), "ambiguous_minutes": len(amb)}
            res["ambiguous"][f"{id_}|{t}|{dates[a]}"] = amb
            if isinstance(got, str):
                w["refused"] = got[:300]; res["totals"]["refused"] += 1; res["windows"].append(w); print(w); continue
            if got.get("from") and got["from"] != dates[a]:
                w["engine_from"] = got["from"]
            rust = [(int(x["dir"]), x["entry"], rnd(x["legs"][0][1]), x["exit"], rnd(x["exit_px"]), kind(x["reason"]), bool(x["open"])) for x in got["trades"]]
            mism = [(i, r, e) for i, (r, e) in enumerate(zip(ref, rust)) if r != e]
            if len(ref) != len(rust):
                mism.append(("count", len(ref), len(rust)))
            w.update(engine_trades=len(rust), mismatches=len(mism), first_mismatches=[str(m) for m in mism[:5]])
            res["totals"]["trades"] += len(ref); res["totals"]["mismatches"] += len(mism)
            res["windows"].append(w)
            print({k: w[k] for k in w if k != "first_mismatches"}, w["first_mismatches"][:2])
json.dump(res, open(OUT, "w"), indent=1)
print("TOTAL", res["totals"])
