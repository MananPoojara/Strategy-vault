#!/usr/bin/env python3
"""Options phase 2: compare the engine (HTTP API on a private port) with indep_opt.py: every trade (entry date, exit
date, return to 1e-9, open flag) and the whole daily equity curve (the held-contract marks) to 2e-6 (the API rounds
equity to 1e-6).  Usage: compare_opt.py PORT us|in OUT_JSON [id filter]"""
import json, os, sys, urllib.error, urllib.parse, urllib.request
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import indep_opt as R

PORT, MARKET, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
FILT = sys.argv[4] if len(sys.argv) > 4 else ""
IDS = {"in": {"NIFTY 50": ["oiss_short_put", "oiss_short_call", "oiss_plus_three_put", "oiss_plus_three_call", "spmt_eom_puts", "ccl08_weekly_rsi_options", "mahrs_covered_calls"],
              "NIFTYBANK": ["oiss_short_put", "oiss_short_call", "oiss_plus_three_put", "oiss_plus_three_call", "spmt_eom_puts", "ccl08_weekly_rsi_options", "mahrs_covered_calls"]},
       "us": {"SPY": ["otvt_cvr1_sell_puts", "ccl08_weekly_rsi_options", "mahrs_covered_calls"],
              "QQQ": ["ccl08_weekly_rsi_options", "mahrs_covered_calls"], "IWM": ["ccl08_weekly_rsi_options", "mahrs_covered_calls"]}}[MARKET]
WIN = {"NIFTY 50": [("2001-06-04", "2013-12-31"), ("2014-01-01", "2026-10-07")],
       "NIFTYBANK": [("2005-06-10", "2015-12-31"), ("2016-01-01", "2026-10-07")],
       "SPY": [("2008-01-02", "2016-12-30"), ("2017-01-03", "2025-12-12")],
       "QQQ": [("2011-03-23", "2017-12-29"), ("2018-01-02", "2025-12-12")],
       "IWM": [("2008-01-02", "2016-12-30"), ("2017-01-03", "2025-12-12")]}


def api(id_, t, a, b):
    q = urllib.parse.urlencode({"id": id_, "ticker": t, "from": a, "to": b})
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/api/guru/connors/run?{q}", timeout=900) as r:
            return json.loads(r.read().decode())
    except urllib.error.HTTPError as e:
        return f"HTTP {e.code}: " + e.read().decode()


res = {"windows": [], "totals": {"trades": 0, "mismatches": 0, "equity_days": 0, "equity_mismatches": 0, "refused": 0}}
vix = R.load_vix() if MARKET == "us" else None
for t, ids in IDS.items():
    m = R.Mkt(t)
    for id_ in [i for i in ids if i.startswith(FILT)]:
        for a_, b_ in WIN[t]:
            got = api(id_, t, a_, b_)
            if isinstance(got, str):
                w = {"ticker": t, "id": id_, "from": a_, "to": b_, "refused": got[:300]}
                res["totals"]["refused"] += 1
                res["windows"].append(w); print(w, flush=True); continue
            dates = got["dates"]
            a, b = m.dates.index(dates[0]), m.dates.index(dates[-1])
            trades, eq = R.run(id_, m, a, b, vix)
            ref = [(e, x, round(r, 9), o) for e, x, r, o in trades]
            eng = [(x["entry"], x["exit"], round(float(x["ret_full_precision"]), 9), bool(x["open"])) for x in got["trades"]]
            mism = [(i, r, e) for i, (r, e) in enumerate(zip(ref, eng)) if r[:2] != e[:2] or r[3] != e[3] or abs(r[2] - e[2]) > 1e-9]
            if len(ref) != len(eng):
                mism.append(("count", len(ref), len(eng)))
            ee = got["equity"]
            emis = [(dates[k], eq[k], ee[k]) for k in range(min(len(eq), len(ee))) if abs(eq[k] - ee[k]) > 2e-6]
            if len(eq) != len(ee):
                emis.append(("len", len(eq), len(ee)))
            w = {"ticker": t, "id": id_, "from": dates[0], "to": dates[-1], "ref_trades": len(ref), "engine_trades": len(eng),
                 "open": sum(1 for x in ref if x[3]), "mismatches": len(mism), "equity_days": len(ee), "equity_mismatches": len(emis),
                 "final_equity": ee[-1] if ee else None, "first_mismatches": [str(x) for x in mism[:5]], "first_equity_mismatches": [str(x) for x in emis[:5]]}
            for k in ("trades", "mismatches", "equity_days", "equity_mismatches"):
                res["totals"][k] += {"trades": len(ref), "mismatches": len(mism), "equity_days": len(ee), "equity_mismatches": len(emis)}[k]
            res["windows"].append(w)
            print({k: w[k] for k in w if not k.startswith("first_")}, w["first_mismatches"][:2], w["first_equity_mismatches"][:2], flush=True)
json.dump(res, open(OUT, "w"), indent=1)
print("TOTAL", res["totals"])
