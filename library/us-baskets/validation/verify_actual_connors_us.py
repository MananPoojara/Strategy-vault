"""Bounded independent source-predicate/ledger checks, using private NAS copies.

No source files are changed. API credentials/cookies are never printed or saved.
This is a single-stock sequential diagnostic, not the author's pooled study.
"""
import csv
import hashlib
import http.cookiejar
import io
import json
from decimal import Decimal, getcontext
from datetime import datetime
from pathlib import Path
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "connors/scripts"))
from verify_stock_rsi_pullback import moving_average, wilder_rsi2, check_actual, source_record

getcontext().prec = 50
FIRST, LAST = "2000-01-01", "2020-09-04"
SYMBOLS = ("AAPL", "MSFT")
METHODS = ("f1a", "f3a", "rsi2_5d")
SIDE_COST = Decimal("0.0003")
TOL = Decimal("0.0000006")
OUTPUT = Path(__file__).resolve().parent


def oracle(rows, method, first=FIRST, last=LAST, side_cost=SIDE_COST):
    """Predicates from the supplied slides; explicit simulation conventions."""
    if method not in METHODS:
        raise ValueError("Unreviewed method is outside this oracle")
    close = [Decimal(row["close"]) for row in rows]
    trend = moving_average(close, 200)
    ma5 = moving_average(close, 5)
    rsi = wilder_rsi2(close)
    selected = [i for i, row in enumerate(rows) if first <= row["date"] <= last]
    if len(selected) < 2:
        raise ValueError("Requested source window is unavailable")
    start, end = selected[0], selected[-1]
    trades = []
    i = start
    while i < end:
        signal = trend[i] is not None and close[i] > trend[i]
        if method == "f3a":
            signal = signal and i >= 9 and close[i] == min(close[i - 9:i + 1])
        elif method == "rsi2_5d":
            signal = signal and rsi[i] is not None and rsi[i] < 2
        if not signal:
            i += 1
            continue
        if method in ("f1a", "f3a"):
            exit_i = min(i + 5, end)
            marked_open = i + 5 > end
        else:
            exit_i = i + 1
            while exit_i <= end and not (ma5[exit_i] is not None and close[exit_i] > ma5[exit_i]):
                exit_i += 1
            marked_open = exit_i > end
            exit_i = min(exit_i, end)
        gross = close[exit_i] / close[i] - 1
        trades.append({"entry": rows[i]["date"], "exit": rows[exit_i]["date"],
                       "entry_i": i - start, "exit_i": exit_i - start,
                       "entry_price": close[i], "exit_price": close[exit_i],
                       "open": marked_open, "bars": exit_i - i,
                       "gross_return": gross, "return_value": gross - 2 * side_cost})
        i = exit_i + 1
    dates = [row["date"] for row in rows[start:end + 1]]
    equity = [Decimal(1)] * len(dates)
    gross_equity = [Decimal(1)] * len(dates)
    net_cap, gross_cap = Decimal(1), Decimal(1)
    next_i = 0
    for trade in trades:
        entry, exit_i = trade["entry_i"], trade["exit_i"]
        for k in range(next_i, entry):
            equity[k], gross_equity[k] = net_cap, gross_cap
        for k in range(entry, exit_i + 1):
            gross = close[start + k] / trade["entry_price"] - 1
            # Entered cost at entry; additional exit cost only at execution or
            # terminal liquidation estimate. This is an explicit model policy.
            charged = side_cost * (2 if k == exit_i else 1)
            equity[k] = net_cap * (1 + gross - charged)
            gross_equity[k] = gross_cap * (1 + gross)
        net_cap, gross_cap = equity[exit_i], gross_equity[exit_i]
        next_i = exit_i + 1
    for k in range(next_i, len(dates)):
        equity[k], gross_equity[k] = net_cap, gross_cap
    return trades, dates, equity, gross_equity


def fetch(opener, base, path, params=None, method="GET", body=None):
    target = base + path + ("?" + urllib.parse.urlencode(params) if params else "")
    headers = {"Origin": base}
    if body is not None:
        body = json.dumps(body).encode()
        headers["Content-Type"] = "application/json"
    request = urllib.request.Request(target, data=body, headers=headers, method=method)
    try:
        with opener.open(request, timeout=45) as response:
            data = response.read()
            return response.status, json.loads(data) if data else None
    except urllib.error.HTTPError as error:
        # Read only engine error text for expected read-only blocked requests;
        # authentication responses are deliberately not captured.
        return error.code, error.read(2000).decode(errors="replace") if "/guru/" in path else "Request failed"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def blocked(opener, base, method, ticker, extra=None):
    params = {"id": method, "ticker": ticker, "from": FIRST, "to": LAST,
              "cost": 0, "slippage": 3, **(extra or {})}
    status, text = fetch(opener, base, "/api/us/guru/connors/run", params)
    require(status == 400 and isinstance(text, str) and '"equity":[' not in text,
            "Unsupported route/coverage unexpectedly published a backtest")
    return {"strategy": method, "ticker": ticker, "override": extra or {},
            "http_status": status, "reason": text}


def source(runtime):
    record = source_record(runtime)
    path = runtime / "books" / record["source_path"]
    text = subprocess.run(["pdftotext", "-f", "13", "-l", "14", "-layout", str(path), "-"],
                          check=True, capture_output=True, text=True, timeout=30).stdout
    for phrase in ("3,944,383", "0.23%", "51%", "C > 200MA", "5 Day"):
        require(phrase in text, "Foundation source table evidence is missing")
    record["printed_claims"]["f1a"] = {"page": 14, "trades": 3944383,
        "average_return_percent": "0.23", "win_rate_percent": "51",
        "entry": "Stock close above 200-day SMA", "exit": "Five-day hold"}
    record["source_pages_read"] = "Session 1 supplied PDF, printed slides 13–14, 17 and 33"
    record["claims_kind"] = "Printed pooled US-stock study figures. The following two-stock sequential diagnostics do not reproduce their universe, original ledger or performance."
    return record


def liquidity_source_evidence(runtime, stock_rows):
    """Independently test observed volume windows, not the API's LF array."""
    path = runtime / "data/us_data.csv"
    before = path.stat()
    data = path.read_bytes()
    benchmark = []
    for row in csv.DictReader(io.StringIO(data.decode())):
        if row["Ticker"] != "SPX":
            continue
        benchmark.append({"date": datetime.strptime(row["Date"], "%d-%m-%Y").date().isoformat(),
                          "close": Decimal(row["Close"]), "volume": Decimal(row.get("Volume") or "0")})
    benchmark.sort(key=lambda row: row["date"])
    valid = {row["date"] for i, row in enumerate(benchmark) if i >= 64
             and all(r["close"] > 0 and r["volume"] > 0 for r in benchmark[i-64:i+1])}
    stock_valid = {row["date"] for i, row in enumerate(stock_rows) if i >= 19
                   and all(Decimal(r["close"]) > 0 and Decimal(r["volume"]) > 0 for r in stock_rows[i-19:i+1])}
    dates = [r["date"] for r in stock_rows if FIRST <= r["date"] <= LAST]
    missing = [d for d in dates if d not in valid or d not in stock_valid]
    after = path.stat()
    require((before.st_size, before.st_mtime_ns) == (after.st_size, after.st_mtime_ns)
            and hashlib.sha256(path.read_bytes()).hexdigest() == hashlib.sha256(data).hexdigest(), "Benchmark source changed during coverage review")
    return {"source_file": "private staged data/us_data.csv", "source_sha256": hashlib.sha256(data).hexdigest(),
            "source_unchanged": True, "source_ticker": "SPX", "required_stock_window": 20, "required_benchmark_window": 65,
            "declared_input_formula": "SMA20(stock close times stock volume) / SMA65(SPX close times SPX volume), times one million; zero/missing volume is unavailable, never filled.",
            "actual_source_bars_with_zero_volume": sum(r["volume"] <= 0 for r in benchmark),
            "requested_days": len(dates), "missing_denominator_or_stock_window_days": len(missing),
            "all_missing_dates": missing, "expected_readiness": "needs_data" if missing else "ready",
            "source_volume_adjustment_conventions_independently_verified": False}


def main():
    runtime = Path.home() / ".cache/strategy-vault-webapp"
    market = Path.home() / ".cache/strategy-vault/us-market-data"
    config = dict(line.split("=", 1) for line in (runtime / ".env").read_text().splitlines()
                  if line and not line.startswith("#"))
    base = config["VAULT_PUBLIC_URL"].rstrip("/")
    report = {"date": "2026-10-13", "source": source(runtime),
        "original_pooled_study_reproduced": False, "all_connors_strategies_certified": False,
        "scope": "Three source predicates, two actual US company stocks, 2000 through 2020-09-04; fees 0 and user-entered slippage 3 bps per side.",
        "conventions_not_certified_as_author_settings": [
            "Single position per stock; signal-close fills; no same-bar re-entry or final-bar new entry.",
            "Five-day hold means five subsequent observed source bars; inclusive ten-close minimum ties qualify.",
            "RSI2 uses Wilder smoothing with the first-two-change seed; RSI exit starts on the next bar.",
            "Terminal positions are marked open at the last close; modeled liquidation cost is included in charts, excluded from closed-return sums.",
            "Compounding begins at unit equity 1, charges entry slippage at entry and exit slippage at exit or terminal liquidation estimate.",
            "Current/current-plus-past filename snapshots do not provide point-in-time membership.",
            "Quotes, corporate-action adjustments, volume-adjustment conventions and source price-unit/tick rules remain independently unverified.",
            "No portfolio sizing, monetary P&L, statutory tax schedule or original US pooled-book replication is claimed."],
        "checks": [], "blocked_checks": [], "session_signed_out": False}
    with (market / "index.csv").open(newline="") as handle:
        selected = {r["ticker"]: r for r in csv.DictReader(handle)
                    if r["basket"] == "us_sp500" and r["ticker"] in SYMBOLS and r["status"] == "ready"}
    require(set(selected) == set(SYMBOLS), "Required AAPL/MSFT sources are not admitted")
    opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
    status, _ = fetch(opener, base, "/api/auth/sign-in/email", method="POST",
                      body={"email": config["VAULT_ADMIN_EMAIL"], "password": config["VAULT_ADMIN_PASSWORD"]})
    require(status == 200, "Private API sign-in failed")
    try:
        status, tickers = fetch(opener, base, "/api/us/tickers")
        require(status == 200 and isinstance(tickers, list), "US ticker registry is unavailable")
        for symbol in SYMBOLS:
            key = f"basket:us_sp500:{symbol}"
            entry = next((t for t in tickers if t.get("ticker") == key), None)
            require(entry is not None and entry.get("currency") == "USD" and entry.get("trusted_market") == "us"
                    and entry.get("asset_type") == "individual_stock", "Live engine has not admitted trusted US stock metadata")
        # Confirm real native compatibility before calculating the oracle.
        status, ready = fetch(opener, base, "/api/us/guru/connors/readiness",
                              {"id": "f1a", "ticker": "basket:us_sp500:AAPL", "from": FIRST, "to": LAST})
        require(status == 200 and ready["strategies"][0]["status"] == "ready", "Actual US native admission failed before calculations")
        del tickers
        for symbol in SYMBOLS:
            meta = selected[symbol]
            path = (market / meta["path"]).resolve()
            require(path.is_relative_to(market.resolve()), "US source path escapes the local cache")
            before = path.stat()
            data = path.read_bytes()
            digest = hashlib.sha256(data).hexdigest()
            require(digest == meta["csv_sha256"], "US stock source checksum mismatch")
            rows = list(csv.DictReader(io.StringIO(data.decode())))
            require(len(rows) == int(meta["bars"]) and rows[0]["date"] == meta["first"] and rows[-1]["date"] == meta["last"], "US source coverage differs from manifest")
            for method in METHODS:
                expected, dates, eq, gross_eq = oracle(rows, method)
                params = {"id": method, "ticker": f"basket:us_sp500:{symbol}", "from": FIRST, "to": LAST,
                          "cost": 0, "slippage": 3, "rollover": 0, "p.trend": 200,
                          "currency": "INR", "native": "false"}
                params.update({"p.rsi_n": 2, "p.rsi_lt": 2, "p.ma": 5} if method == "rsi2_5d" else {"p.hold": 5})
                if method == "f3a": params["p.low_n"] = 10
                status, actual = fetch(opener, base, "/api/us/guru/connors/run", params)
                require(status == 200 and isinstance(actual, dict), "Actual US run was unavailable")
                differences = check_actual(actual, expected, dates, symbol)
                prov = actual.get("provenance", {})
                if prov.get("csv_sha256") != digest or prov.get("trusted_market") != "us" or prov.get("currency") != "USD" or prov.get("asset_type") != "individual_stock":
                    differences.append({"field": "trusted_source_identity"})
                for field in ("quote_accuracy_independently_verified", "corporate_actions_independently_verified", "historical_membership_verified", "source_price_units_verified"):
                    if prov.get(field) is not False: differences.append({"field": field})
                if actual.get("model_initial_equity") != 1: differences.append({"field": "initial_model_baseline"})
                for field, values in (("equity", eq), ("gross_equity", gross_eq)):
                    if len(actual.get(field, [])) != len(values): differences.append({"field": field + "_length"})
                    else:
                        bad = [i for i, (a, e) in enumerate(zip(actual[field], values)) if abs(Decimal(str(a)) - e) > max(TOL, abs(e) * Decimal("0.0000000001"))]
                        if bad: differences.append({"field": field, "mismatched_days": len(bad), "first_index": bad[0]})
                for i, (a, e) in enumerate(zip(actual.get("trades", []), expected)):
                    ledger = a.get("accounting", {})
                    for key, expected_value in (("gross_ret", e["gross_return"]), ("net_ret", e["return_value"]), ("fee_ret", Decimal(0)), ("slippage_ret", 2 * SIDE_COST)):
                        if abs(Decimal(str(ledger.get(key, "NaN"))) - expected_value) > TOL: differences.append({"row": i, "field": "accounting." + key})
                net = [e["return_value"] for e in expected if not e["open"]]
                gross = [e["gross_return"] for e in expected if not e["open"]]
                if abs(Decimal(str(actual["stats"]["total_return"])) - (eq[-1] - 1)) > TOL: differences.append({"field": "compounded_total_return"})
                records = [{k: str(v) if isinstance(v, Decimal) else v for k, v in e.items()} for e in expected]
                ledger_bytes = (json.dumps(records, sort_keys=True, separators=(",", ":")) + "\n").encode()
                ledger_file = f"{symbol}-{method}-independent-ledger.json"
                (OUTPUT / ledger_file).write_bytes(ledger_bytes)
                report["checks"].append({"symbol": symbol, "strategy": method, "first": dates[0], "last": dates[-1],
                    "source": meta["source"], "source_sha256": meta["source_sha256"], "csv_sha256": digest,
                    "records_checked": len(expected), "closed_positions": len(net), "open_positions": len(expected)-len(net),
                    "closed_net_return_sum": str(sum(net)), "average_closed_net_return": str(sum(net)/len(net)) if net else None,
                    "closed_gross_return_sum": str(sum(gross)), "compounded_model_return": str(eq[-1]-1),
                    "first_model_equity": str(eq[0]), "days_checked": len(dates), "fees_bps": 0, "slippage_bps_per_side": 3,
                    "independent_ledger": ledger_file, "independent_ledger_sha256": hashlib.sha256(ledger_bytes).hexdigest(),
                    "mismatches": differences, "source_unchanged": hashlib.sha256(path.read_bytes()).hexdigest() == digest and path.stat().st_mtime_ns == before.st_mtime_ns})
        for method, ticker, extra in [("f1a", "SPX", None), ("f1a", "SPY", None),
             ("r3", "basket:us_sp500:AAPL", None), ("hpetf_3dhl_long", "basket:us_sp500:AAPL", None),
             ("f1a", "basket:us_sp500:AAPL", {"to": "2025-12-31"})]:
            report["blocked_checks"].append(blocked(opener, base, method, ticker, extra))
        status, lf = fetch(opener, base, "/api/us/guru/connors/readiness",
                           {"id": "tps_stocks", "ticker": "basket:us_sp500:AAPL", "from": FIRST, "to": LAST})
        require(status == 200 and lf["strategies"][0]["status"] in ("ready", "needs_data"), "Actual Liquidity Filter readiness unavailable")
        report["actual_liquidity_filter_readiness"] = lf["strategies"][0]
        with (market / selected["AAPL"]["path"]).open(newline="") as handle:
            report["independent_liquidity_coverage"] = liquidity_source_evidence(runtime, list(csv.DictReader(handle)))
        require(lf["strategies"][0]["status"] == report["independent_liquidity_coverage"]["expected_readiness"], "LF data admission differs from independent source-window coverage")
        status, clean = fetch(opener, base, "/api/us/guru/connors/readiness",
                              {"id": "tps_stocks", "ticker": "basket:us_sp500:AAPL", "from": "2002-01-01", "to": LAST})
        clean_expected = "needs_data" if any(d >= "2002-01-01" for d in report["independent_liquidity_coverage"]["all_missing_dates"]) else "ready"
        require(status == 200 and clean["strategies"][0]["status"] == clean_expected, "LF covered-window admission differs from independently observed gaps")
        report["independent_liquidity_coverage"]["actual_fully_covered_window"] = {"from": "2002-01-01", "to": LAST, "readiness": clean["strategies"][0]}
    finally:
        status, _ = fetch(opener, base, "/api/auth/sign-out", method="POST", body={})
        report["session_signed_out"] = status == 200
    report["total_records_checked"] = sum(c["records_checked"] for c in report["checks"])
    report["total_mismatches"] = sum(len(c["mismatches"]) for c in report["checks"])
    (OUTPUT / "ACTUAL-US-CONNORS-2026-10-13.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"records_checked": report["total_records_checked"], "mismatches": report["total_mismatches"],
                      "cases": [{"symbol": c["symbol"], "strategy": c["strategy"], "records": c["records_checked"], "mismatches": len(c["mismatches"])} for c in report["checks"]],
                      "source_study_reproduced": False, "session_signed_out": report["session_signed_out"]}))
    require(report["total_mismatches"] == 0 and all(c["source_unchanged"] for c in report["checks"]) and report["session_signed_out"], "US independent validation failed")


if __name__ == "__main__":
    main()
