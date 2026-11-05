#!/usr/bin/env python3
"""K39: engine trade ledgers (HTTP API, private port) vs the independent references (indep_ep.py), field by field: direction,
entry date, entry price, exit date, exit price, exit reason, open flag; in order. Windows: each ticker's full minute history
and two halves. Usage (via run_engine.sh): compare_ep.py us|in OUT_JSON PORT"""
import bisect, json, math, os, sys, urllib.error, urllib.parse, urllib.request
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, "/home/server/.cache/strategy-vault-webapp/claude-continuation-20261020/worktrees/titles-k39-1104/library/intraday/verify")
import indep_ep as E  # noqa: E402
import indep_ref as R0  # noqa: E402  (daily file reader only)
MKT, OUT, PORT = sys.argv[1], sys.argv[2], sys.argv[-1]
D_ = "/home/server/.cache/strategy-vault-webapp/data"
ALL4 = ["acbff_open_rel", "acbff_2bar_open", "acbff_final_third", "acbff_1230", "acbff_9oc", "acbff_midpoint", "acbff_rsi_5min",
        "atm04_60min", "jmls_bar_breakout", "jmls_ma_cross_30m"]
US = ["jmls_ls_model"]
WIN = {"us": {"SPY": [("2000-01-03", "2025-10-21"), ("2000-01-03", "2010-12-31"), ("2011-01-03", "2025-10-21")],
              "SPX": [("2008-01-02", "2026-08-14"), ("2008-01-02", "2016-12-30"), ("2017-01-03", "2026-08-14")]},
       "in": {"NIFTY 50": [("2008-01-01", "2026-10-07"), ("2008-01-01", "2016-12-30"), ("2017-01-02", "2026-10-07")],
              "NIFTYBANK": [("2010-03-02", "2026-04-22"), ("2010-03-02", "2017-12-29"), ("2018-01-01", "2026-04-22")]}}[MKT]
PATH = f"{D_}/us_data.csv" if MKT == "us" else f"{D_}/idx_data.csv"
rnd = lambda v: math.copysign(math.floor(abs(v) * 1e6 + 0.5), v) / 1e6


def api(id_, t, a, b, prm):
    q = urllib.parse.urlencode({"id": id_, "ticker": t, "from": a, "to": b, **{f"p.{k}": v for k, v in prm.items()}})
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/api/guru/connors/run?{q}", timeout=1800) as r:
            return json.loads(r.read().decode())
    except urllib.error.HTTPError as e:
        return f"HTTP {e.code}: " + e.read().decode()


res = {"windows": [], "totals": {"trades": 0, "mismatches": 0, "refused": 0}}
for t, wins in WIN.items():
    dates = [x[0] for x in R0.load_daily(PATH, t)]
    M = E.Day(t)
    runs = [(i, {}) for i in ALL4 + (US if MKT == "us" else [])] + [("atm04_60min", {"third": 1})]
    for id_, prm in runs:
        for a_, b_ in wins:
            got = api(id_, t, a_, b_, prm)
            if isinstance(got, str):
                w = {"ticker": t, "id": id_, "prm": prm, "from": a_, "to": b_, "refused": got[:300]}
                res["totals"]["refused"] += 1; res["windows"].append(w); print(w, flush=True); continue
            a = bisect.bisect_left(dates, got["dates"][0]); b = bisect.bisect_right(dates, got["dates"][-1]) - 1
            ref = [(x[0], x[1], rnd(x[2]), x[3], rnd(x[4]), x[5], x[6]) for x in E.backtest(id_, dates, M, a, b, prm)]
            eng = [(int(x["dir"]), x["entry"], rnd(x["legs"][0][1]), x["exit"], rnd(x["exit_px"]), x["reason"], bool(x["open"])) for x in got["trades"]]
            mism = [(k, r, e) for k, (r, e) in enumerate(zip(ref, eng)) if r != e]
            if len(ref) != len(eng):
                mism.append(("count", len(ref), len(eng)))
            s = got.get("stats") or {}
            w = {"ticker": t, "id": id_, "prm": prm, "from": dates[a], "to": dates[b], "ref_trades": len(ref), "engine_trades": len(eng),
                 "win_rate": s.get("win_rate"), "avg_trade": s.get("avg_trade"), "compounded": s.get("compounded_return"),
                 "mismatches": len(mism), "first": [str(m) for m in mism[:3]]}
            res["totals"]["trades"] += len(ref); res["totals"]["mismatches"] += len(mism)
            res["windows"].append(w); print(w, flush=True)
# decision #177 (M1-style): SPX first on the overlap, then SPY on the window the SPX run used; tests fixed in CODING-1103g:
# (a) opposite sign of the average trade, (b) SPY avg > 0 and SPX avg > 3x SPY and > SPY + 0.10 pp, (c) SPY compounded > 0
# and SPX compounded > 10x SPY's.
if MKT == "us":
    K = ("trades", "win_rate", "avg_trade", "compounded_return", "max_dd")
    sg = lambda v: (v > 0) - (v < 0)
    res["m1"] = []
    for id_ in ALL4 + US:
        x = api(id_, "SPX", "2008-01-02", "2025-10-21", {})
        y = api(id_, "SPY", x["dates"][0], x["dates"][-1], {}) if not isinstance(x, str) else "no SPX run"
        if isinstance(x, str) or isinstance(y, str):
            res["m1"].append({"id": id_, "refused": str(x)[:200] + " | " + str(y)[:200]}); continue
        X, Y = ({k: (z.get("stats") or {}).get(k) for k in K} for z in (x, y))
        f = []
        if sg(X["avg_trade"]) != sg(Y["avg_trade"]): f.append("a")
        if Y["avg_trade"] > 0 and X["avg_trade"] > 3 * Y["avg_trade"] and X["avg_trade"] - Y["avg_trade"] > 0.001: f.append("b")
        if Y["compounded_return"] > 0 and X["compounded_return"] > 10 * Y["compounded_return"]: f.append("c")
        r = {"id": id_, "from": x["dates"][0], "to": x["dates"][-1], "SPX": X, "SPY": Y, "flags": f}
        res["m1"].append(r); print("M1", r, flush=True)
json.dump(res, open(OUT, "w"), indent=1)
print("TOTAL", res["totals"])
