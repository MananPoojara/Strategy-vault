#!/usr/bin/env python3
"""Wave 1: compare the engine's trade ledgers (HTTP API on a private port) with indep_w1.py, field by field.
Usage: compare_w1.py PORT us|in OUT_JSON [id-prefix filter]"""
import bisect, json, math, os, sys, urllib.error, urllib.parse, urllib.request
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import indep_w1 as R

PORT, MARKET, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
FILT = sys.argv[4] if len(sys.argv) > 4 else ""
D_ = "/home/server/.cache/strategy-vault-webapp/data"
ALL4 = ["ats_10pct_oops", "ats_wide_range_gap", "btp2_10pct_oops", "btp2_10pct_oops_long", "btp2_10pct_oops_short",
        "btp2_wide_range_exhaustion_gap_short", "tdnmtt_trap_long", "tdnmtt_trap_short", "lwdg2_0105", "jb30d_gap_pct"]
SPX_ONLY = ["hfm_vix", "baboops_oops", "lwlts_mon_vb", "jb30d_gap_tick"] + R.TCDT_SAME
WIN = {"us": {"SPY": [("2000-01-03", "2010-12-31"), ("2011-01-03", "2025-10-21")],
              "SPX": [("2008-01-02", "2016-12-30"), ("2017-01-03", "2026-08-14")]},
       "in": {"NIFTY 50": [("2008-01-01", "2016-12-30"), ("2017-01-02", "2026-10-07")],
              "NIFTYBANK": [("2010-03-02", "2017-12-29"), ("2018-01-01", "2026-04-22")]}}[MARKET]
PATH = f"{D_}/us_data.csv" if MARKET == "us" else f"{D_}/idx_data.csv"
VIX = R.load_vix() if MARKET == "us" else None
# the engine's accuracy gate starts open-reading rules where the cash index's daily opens are real (gate.rs real_open_from)
import re
GATE = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../Context/guru/backend/src/gate.rs")).read()
CLAMP = set(re.findall(r'id: r##"(\w+)"##.*?clamp_open: true', GATE))
REAL_OPEN = {"SPX": "2014-01-01", "NDX": "2001-01-01", "DJI": "2016-01-01"}


def rnd(v):
    return math.copysign(math.floor(abs(v) * 1e6 + 0.5), v) / 1e6


def kind(r):
    r = r.lower()
    return "open" if r == "open" else "stop" if "stop" in r else "close"


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
    ids = ALL4 + (SPX_ONLY if t == "SPX" else [])
    for id_ in [i for i in ids if i.startswith(FILT)]:
        for a_, b_ in wins:
            if (id_ in CLAMP or id_ in SPX_ONLY) and t in REAL_OPEN:  # SPX-only wave-1 rules read the daily open
                if b_ < REAL_OPEN[t]:
                    continue
                a_ = max(a_, REAL_OPEN[t])
            got = api(id_, t, a_, b_)
            if isinstance(got, str):
                w = {"ticker": t, "id": id_, "from": a_, "to": b_, "refused": got[:300]}
                if "begin" in got or "clamp" in got or "real only" in got:  # window entirely before the real-open date
                    w["skipped"] = True
                else:
                    res["totals"]["refused"] += 1
                res["windows"].append(w); print(w); continue
            a_eff = got.get("from") or a_  # the engine may move the start (real-open clamp)
            a = bisect.bisect_left(dates, a_eff); b = bisect.bisect_right(dates, b_) - 1
            amb = []
            ref = [(x["dir"], x["entry"], rnd(x["entry_px"]), x["exit"], rnd(x["exit_px"]), x["reason"], x["open"])
                   for x in R.backtest(id_, Dd, M, a, b, t, amb, VIX)]
            w = {"ticker": t, "id": id_, "from": dates[a], "to": dates[b], "ref_trades": len(ref), "ambiguous_minutes": len(amb)}
            res["ambiguous"][f"{id_}|{t}|{dates[a]}"] = amb
            rust = [(int(x["dir"]), x["entry"], rnd(x["legs"][0][1]), x["exit"], rnd(x["exit_px"]), kind(x["reason"]), bool(x["open"])) for x in got["trades"]]
            mism = [(i, r, e) for i, (r, e) in enumerate(zip(ref, rust)) if r != e]
            if len(ref) != len(rust):
                mism.append(("count", len(ref), len(rust)))
            w.update(engine_trades=len(rust), mismatches=len(mism), first_mismatches=[str(m) for m in mism[:5]])
            res["totals"]["trades"] += len(ref); res["totals"]["mismatches"] += len(mism)
            res["windows"].append(w)
            print({k: w[k] for k in w if k != "first_mismatches"}, w["first_mismatches"][:2], flush=True)
json.dump(res, open(OUT, "w"), indent=1)
print("TOTAL", res["totals"])
