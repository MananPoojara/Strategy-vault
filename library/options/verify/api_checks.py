#!/usr/bin/env python3
"""API checks for the options phase 2 rules: catalogue status, and the instruments that must be refused with a reason.
Usage: api_checks.py PORT us|in OUT_JSON"""
import json, sys, urllib.error, urllib.parse, urllib.request

PORT, MARKET, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
RULES = ["oiss_short_put", "oiss_short_call", "oiss_plus_three_put", "oiss_plus_three_call", "otvt_cvr1_sell_puts",
         "ccl08_weekly_rsi_options", "mahrs_covered_calls", "spmt_eom_puts"]


def get(path):
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{PORT}{path}", timeout=600) as r:
            return 200, r.read().decode()
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode()


code, body = get("/api/guru/connors/strategies")
cat = {d["id"]: d for d in json.loads(body) if isinstance(d, dict) and "id" in d} if code == 200 else {}
out = {"status": {i: cat.get(i, {}).get("status") for i in RULES}, "runs": []}
checks = {"us": [("oiss_short_put", "SPY", "2010-01-04", "2012-12-31", False), ("spmt_eom_puts", "SPY", "2010-01-04", "2012-12-31", False),
                 ("otvt_cvr1_sell_puts", "QQQ", "2012-01-03", "2014-12-31", False), ("mahrs_covered_calls", "SPX", "2010-01-04", "2012-12-31", False),
                 ("ccl08_weekly_rsi_options", "MDY", "2010-01-04", "2012-12-31", False), ("mahrs_covered_calls", "SPY", "2007-01-03", "2012-12-31", "auto"),
                 ("mahrs_covered_calls", "SPY", "2024-01-02", "2026-03-31", False), ("mahrs_covered_calls", "SPY", "2010-01-04", "2012-12-31", True)],
          "in": [("oiss_short_put", "NIFTYIT", "2010-01-04", "2012-12-31", False), ("oiss_short_put", "NIFTY 50", "1999-01-04", "2012-12-31", "auto"),
                 ("spmt_eom_puts", "NIFTYBANK", "2012-01-02", "2014-12-31", True), ("otvt_cvr1_sell_puts", "NIFTY 50", "2012-01-02", "2014-12-31", False)]}[MARKET]
for id_, t, a, b, ok in checks:
    code, body = get("/api/guru/connors/run?" + urllib.parse.urlencode({"id": id_, "ticker": t, "from": a, "to": b}))
    # "auto": a start before the option data is moved to their first day (auto_start note), as for VIX/minute data
    row = {"id": id_, "ticker": t, "from": a, "to": b, "http": code, "expect_ok": ok,
           "pass": (code == 200 and '"auto_start"' in body) if ok == "auto" else (code == 200) == ok,
           "note": body[:260] if code != 200 else f"{len(json.loads(body)['trades'])} trades"}
    if code == 200:
        js = json.loads(body)
        row["option_account"] = "option_account" in js
    out["runs"].append(row)
    print(row, flush=True)
print("status", out["status"])
json.dump(out, open(OUT, "w"), indent=1)
