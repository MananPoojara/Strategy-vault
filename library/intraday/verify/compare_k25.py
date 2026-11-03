#!/usr/bin/env python3
"""K25 (3 Nov 2026): compare the engine's trade ledgers (HTTP API on a private port) with the independent references,
field by field, on the engine's own minute files (data/intraday):
  ss_momentum_pinball -> indep_ref.pinball (Street Smarts Ch 7, intraday phase 1), now with the NIFTY 50 minutes from
                         2026-08-11 rejected (the engine's NIFTY_BAD_FROM), as for the wave-1 rules (K24)
  tdperl_rebo         -> indep_rebo.py (new, Perl Ch 9 pp.157-160)
Usage: compare_k25.py PORT us|in OUT_JSON"""
import bisect, json, math, os, sys, urllib.error, urllib.parse, urllib.request
os.environ.setdefault("M1_DIR", "/home/server/.cache/strategy-vault-webapp/data/intraday")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import indep_ref as R0  # noqa: E402
import indep_rebo as RB  # noqa: E402

PORT, MARKET, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
D_ = "/home/server/.cache/strategy-vault-webapp/data"
WIN = {"us": {"SPY": [("2000-01-03", "2025-10-21"), ("2000-01-03", "2010-12-31"), ("2011-01-03", "2025-10-21")],
              "SPX": [("2008-01-02", "2026-08-14"), ("2008-01-02", "2016-12-30"), ("2017-01-03", "2026-08-14")]},
       "in": {"NIFTY 50": [("2008-01-01", "2026-10-07"), ("2008-01-01", "2016-12-30"), ("2017-01-02", "2026-10-07")],
              "NIFTYBANK": [("2010-03-02", "2026-04-22"), ("2010-03-02", "2017-12-29"), ("2018-01-01", "2026-04-22")]}}[MARKET]
PATH = f"{D_}/us_data.csv" if MARKET == "us" else f"{D_}/idx_data.csv"
IDS = ["ss_momentum_pinball", "tdperl_rebo"]
REAL_OPEN = {"SPX": "2014-01-01"}


def rnd(v):
    return math.copysign(math.floor(abs(v) * 1e6 + 0.5), v) / 1e6


def kind_pin(r):
    r = r.lower()
    return "open" if r == "open" else "stop" if "stop" in r else "target" if "limit" in r else "close"


def kind_rebo(r):
    return "open" if r.lower() == "open" else "break"


def api(id_, t, a, b):
    q = urllib.parse.urlencode({"id": id_, "ticker": t, "from": a, "to": b})
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/api/guru/connors/run?{q}", timeout=900) as r:
            return json.loads(r.read().decode())
    except urllib.error.HTTPError as e:
        return f"HTTP {e.code}: " + e.read().decode()


class Rejecting:
    def __init__(self, t):
        self.m, self.t = R0.Minutes(t), t

    def day(self, d):
        return None if self.t == "NIFTY 50" and d >= "2026-08-11" else self.m.day(d)


res = {"windows": [], "ambiguous": {}, "totals": {"trades": 0, "mismatches": 0, "refused": 0}}
for t, wins in WIN.items():
    Dd = R0.load_daily(PATH, t)
    dates = [x[0] for x in Dd]
    M = Rejecting(t)
    for id_ in IDS:
        for a_, b_ in wins:
            if id_ == "tdperl_rebo" and t in REAL_OPEN:  # gate clamp_open: the levels are set off the daily open
                if b_ < REAL_OPEN[t]:
                    continue
                a_ = max(a_, REAL_OPEN[t])
            got = api(id_, t, a_, b_)
            if isinstance(got, str):
                w = {"ticker": t, "id": id_, "from": a_, "to": b_, "refused": got[:300]}
                res["totals"]["refused"] += 1; res["windows"].append(w); print(w, flush=True); continue
            gd = got.get("dates") or []
            a_eff, b_eff = (gd[0], gd[-1]) if gd else (a_, b_)
            a = bisect.bisect_left(dates, a_eff); b = bisect.bisect_right(dates, b_eff) - 1
            amb = []
            if id_ == "tdperl_rebo":
                ref = [(x["dir"], x["entry"], rnd(x["entry_px"]), x["exit"], rnd(x["exit_px"]), kind_rebo(x["reason"]), x["open"]) for x in RB.backtest(Dd, M, a, b, amb)]
                k_rust = kind_rebo
            else:
                ref = [(x["dir"], x["entry"], rnd(x["entry_px"]), x["exit"], rnd(x["exit_px"]), x["reason"], x["open"]) for x in R0.pinball(Dd, M, a, b, t, amb)]
                k_rust = kind_pin
            rust = [(int(x["dir"]), x["entry"], rnd(x["legs"][0][1]), x["exit"], rnd(x["exit_px"]), k_rust(x["reason"]), bool(x["open"])) for x in got["trades"]]
            mism = [(i, r, e) for i, (r, e) in enumerate(zip(ref, rust)) if r != e]
            if len(ref) != len(rust):
                mism.append(("count", len(ref), len(rust)))
            rets = [x["dir"] * (x["exit_px"] / x["legs"][0][1] - 1) for x in got["trades"] if not x["open"]]
            stats = {"closed": len(rets), "win_pct": round(100 * sum(r > 0 for r in rets) / len(rets), 2) if rets else None,
                     "avg_pct": round(100 * sum(rets) / len(rets), 4) if rets else None,
                     "net_pct": round(100 * (math.prod(1 + r for r in rets) - 1), 4) if rets else None}
            w = {"ticker": t, "id": id_, "from": dates[a], "to": dates[b], "ref_trades": len(ref), "engine_trades": len(rust),
                 "engine_stats": stats, "ambiguous": len(amb), "engine_ambiguous_marked": sum("ambiguous" in x["reason"] for x in got["trades"]),
                 "nifty_after_bad": sum(x["entry"] >= "2026-08-11" for x in got["trades"]) if t == "NIFTY 50" else None,
                 "mismatches": len(mism), "first_mismatches": [str(m) for m in mism[:5]]}
            res["ambiguous"][f"{id_}|{t}|{dates[a]}|{dates[b]}"] = amb
            res["totals"]["trades"] += len(ref); res["totals"]["mismatches"] += len(mism)
            res["windows"].append(w)
            print({k: w[k] for k in w if k != "first_mismatches"}, w["first_mismatches"][:2], flush=True)
json.dump(res, open(OUT, "w"), indent=1)
print("TOTAL", res["totals"])
