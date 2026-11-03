#!/usr/bin/env python3
"""Intraday wave 3 (K24): compare the engine's trade ledgers (HTTP API on a private port) with the independent references,
field by field, on the engine's own minute files (data/intraday), and count the minutes decided by a convention.
  tdperl_tdline_short                 -> indep_w3.py (new, from Perl Ch 4)
  the 12 wave-1 ids still withheld by the gate -> indep_w1.py, ss_80_20 -> indep_ref.py (earlier lanes, from the books)
NIFTY 50 minutes from 2026-08-11 are rejected (bad closing minutes) in every reference, as in the engine.
Usage: compare_w3.py PORT us|in OUT_JSON [id-prefix]"""
import bisect, json, math, os, sys, urllib.error, urllib.parse, urllib.request
os.environ.setdefault("M1_DIR", "/home/server/.cache/strategy-vault-webapp/data/intraday")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import indep_ref as R0  # noqa: E402
import indep_w1 as R1  # noqa: E402
import indep_w3 as R3  # noqa: E402

PORT, MARKET, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
FILT = sys.argv[4] if len(sys.argv) > 4 else ""  # optional id prefix
D_ = "/home/server/.cache/strategy-vault-webapp/data"
ALL4 = ["ats_10pct_oops", "ats_wide_range_gap", "btp2_10pct_oops", "btp2_10pct_oops_long", "btp2_10pct_oops_short",
        "btp2_wide_range_exhaustion_gap_short", "jb30d_gap_pct", "ss_80_20", "tdnmtt_trap_long", "tdnmtt_trap_short", "tdperl_tdline_short"]
SPX_ONLY = ["hfm_vix", "baboops_oops", "jb30d_gap_tick"]
WIN = {"us": {"SPY": [("2000-01-03", "2025-10-21"), ("2000-01-03", "2010-12-31"), ("2011-01-03", "2025-10-21")],
              "SPX": [("2008-01-02", "2026-08-14"), ("2008-01-02", "2016-12-30"), ("2017-01-03", "2026-08-14")]},
       "in": {"NIFTY 50": [("2008-01-01", "2026-10-07"), ("2008-01-01", "2016-12-30"), ("2017-01-02", "2026-10-07")],
              "NIFTYBANK": [("2010-03-02", "2026-04-22"), ("2010-03-02", "2017-12-29"), ("2018-01-01", "2026-04-22")]}}[MARKET]
PATH = f"{D_}/us_data.csv" if MARKET == "us" else f"{D_}/idx_data.csv"
VIX = R1.load_vix() if MARKET == "us" else None
REAL_OPEN = {"SPX": "2014-01-01"}  # every id here reads the daily open: gate clamp_open


def rnd(v):
    return math.copysign(math.floor(abs(v) * 1e6 + 0.5), v) / 1e6


def kind(r):
    r = r.lower()
    if r == "open":
        return "open"
    for k in ("objective", "failure", "break"):
        if k in r:
            return k
    return "stop" if "stop" in r else "close"


def kind3(r):  # indep_w3 reasons
    r = r.lower()
    return "open" if r == "open" else "objective" if "objective" in r else "failure" if "failure" in r else "break"


def api(id_, t, a, b):
    q = urllib.parse.urlencode({"id": id_, "ticker": t, "from": a, "to": b})
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/api/guru/connors/run?{q}", timeout=900) as r:
            return json.loads(r.read().decode())
    except urllib.error.HTTPError as e:
        return f"HTTP {e.code}: " + e.read().decode()


class Rejecting:
    """indep_ref.Minutes with the NIFTY 50 days from 2026-08-11 rejected (the engine's NIFTY_BAD_FROM)."""
    def __init__(self, t):
        self.m, self.t = R0.Minutes(t), t

    def day(self, d):
        return None if self.t == "NIFTY 50" and d >= "2026-08-11" else self.m.day(d)


res = {"windows": [], "ambiguous": {}, "dropped": {}, "totals": {"trades": 0, "mismatches": 0, "refused": 0}}
for t, wins in WIN.items():
    Dd = R0.load_daily(PATH, t)
    dates = [x[0] for x in Dd]
    M = Rejecting(t)
    for id_ in [i for i in ALL4 + (SPX_ONLY if t == "SPX" else []) if i.startswith(FILT)]:
        for a_, b_ in wins:
            if t in REAL_OPEN:
                if b_ < REAL_OPEN[t]:
                    continue
                a_ = max(a_, REAL_OPEN[t])
            got = api(id_, t, a_, b_)
            if isinstance(got, str):
                w = {"ticker": t, "id": id_, "from": a_, "to": b_, "refused": got[:300]}
                res["totals"]["refused"] += 1; res["windows"].append(w); print(w, flush=True); continue
            a_eff = got.get("from") or a_
            a = bisect.bisect_left(dates, a_eff); b = bisect.bisect_right(dates, b_) - 1
            amb, drop = [], []
            if id_ == "tdperl_tdline_short":
                ref = [(x["dir"], x["entry"], rnd(x["entry_px"]), x["exit"], rnd(x["exit_px"]), kind3(x["reason"]), x["open"])
                       for x in R3.backtest(Dd, M, a, b, 1, amb, drop)]
                k_rust = kind
            elif id_ == "ss_80_20":
                ref = [(x["dir"], x["entry"], rnd(x["entry_px"]), x["exit"], rnd(x["exit_px"]), x["reason"], x["open"]) for x in R0.backtest(id_, Dd, M, a, b, t, amb)]
                k_rust = lambda r: "open" if r.lower() == "open" else "stop" if "stop" in r.lower() else "close"
            else:
                ref = [(x["dir"], x["entry"], rnd(x["entry_px"]), x["exit"], rnd(x["exit_px"]), x["reason"], x["open"]) for x in R1.backtest(id_, Dd, M, a, b, t, amb, VIX)]
                k_rust = lambda r: "open" if r.lower() == "open" else "stop" if "stop" in r.lower() else "close"
            rust = [(int(x["dir"]), x["entry"], rnd(x["legs"][0][1]), x["exit"], rnd(x["exit_px"]), k_rust(x["reason"]), bool(x["open"])) for x in got["trades"]]
            eng_amb = sum("ambiguous" in x["reason"] for x in got["trades"])
            mism = [(i, r, e) for i, (r, e) in enumerate(zip(ref, rust)) if r != e]
            if len(ref) != len(rust):
                mism.append(("count", len(ref), len(rust)))
            rets = [x["dir"] * (x["exit_px"] / x["legs"][0][1] - 1) for x in got["trades"] if not x["open"]]
            stats = {"closed": len(rets), "win_pct": round(100 * sum(r > 0 for r in rets) / len(rets), 2) if rets else None,
                     "avg_pct": round(100 * sum(rets) / len(rets), 4) if rets else None}
            w = {"ticker": t, "id": id_, "from": dates[a], "to": dates[b], "ref_trades": len(ref), "engine_trades": len(rust), "engine_stats": stats,
                 "ambiguous": len(amb), "engine_ambiguous_marked": eng_amb, "dropped": len(drop), "mismatches": len(mism),
                 "first_mismatches": [str(m) for m in mism[:5]]}
            res["ambiguous"][f"{id_}|{t}|{dates[a]}|{dates[b]}"] = amb
            res["dropped"][f"{id_}|{t}|{dates[a]}|{dates[b]}"] = drop
            res["totals"]["trades"] += len(ref); res["totals"]["mismatches"] += len(mism)
            res["windows"].append(w)
            print({k: w[k] for k in w if k != "first_mismatches"}, w["first_mismatches"][:2], flush=True)
json.dump(res, open(OUT, "w"), indent=1)
print("TOTAL", res["totals"])
