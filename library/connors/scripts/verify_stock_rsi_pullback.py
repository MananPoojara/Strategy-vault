"""Independent, bounded diagnostics for two source predicates on three actual stocks.

Reads the privately staged Session 1 PDF and immutable admitted stock CSVs.
The original pooled US studies and their execution/indicator conventions are
not reproduced or certified by these sequential Indian-stock diagnostics.
"""
import csv
import hashlib
import http.cookiejar
import json
from decimal import Decimal, getcontext
from pathlib import Path
import subprocess
import urllib.parse
import urllib.request

getcontext().prec = 50
FIRST, LAST = "2020-01-01", "2025-12-31"
SYMBOLS = ("RELIANCE", "HDFCBANK", "TCS")
METHODS = ("rsi2_5d", "f3a")
PRICE_TOL = Decimal("0.000001")
RETURN_TOL = Decimal("0.0000006")


def moving_average(values, period):
    """Direct Decimal window means, independent of the Rust rolling float sum."""
    return [None if i + 1 < period else sum(values[i + 1 - period:i + 1]) / period
            for i in range(len(values))]


def wilder_rsi2(values):
    """Explicit Wilder/first-two-deltas seed: a declared diagnostic convention."""
    result = [None] * len(values)
    if len(values) < 3:
        return result
    changes = [values[i] - values[i - 1] for i in range(1, len(values))]
    up = sum(max(change, Decimal(0)) for change in changes[:2]) / 2
    down = sum(max(-change, Decimal(0)) for change in changes[:2]) / 2
    for i in range(2, len(values)):
        if i > 2:
            change = values[i] - values[i - 1]
            up = (up + max(change, Decimal(0))) / 2
            down = (down + max(-change, Decimal(0))) / 2
        result[i] = Decimal(50) if up + down == 0 else 100 * up / (up + down)
    return result


def diagnostic_trades(rows, method, first=FIRST, last=LAST):
    close = [Decimal(row["close"]) for row in rows]
    ma200 = moving_average(close, 200)
    ma5 = moving_average(close, 5)
    rsi = wilder_rsi2(close)
    selected = [i for i, row in enumerate(rows) if first <= row["date"] <= last]
    if not selected:
        raise ValueError("Selected dates are unavailable")
    start, end = selected[0], selected[-1]
    expected = []
    i = start
    while i < end:
        trend = ma200[i] is not None and close[i] > ma200[i]
        signal = trend and (
            rsi[i] is not None and rsi[i] < 2 if method == "rsi2_5d"
            else i >= 9 and close[i] == min(close[i - 9:i + 1])
        )
        if not signal:
            i += 1
            continue
        entry = i
        if method == "f3a":
            exit_i = min(entry + 5, end)
            marked_open = entry + 5 > end
        else:
            exit_i = entry + 1
            while exit_i <= end and not (ma5[exit_i] is not None and close[exit_i] > ma5[exit_i]):
                exit_i += 1
            marked_open = exit_i > end
            exit_i = min(exit_i, end)
        expected.append({
            "entry": rows[entry]["date"], "exit": rows[exit_i]["date"],
            "entry_i": entry - start, "exit_i": exit_i - start,
            "entry_price": close[entry], "exit_price": close[exit_i],
            "open": marked_open, "bars": exit_i - entry,
            "return_value": close[exit_i] / close[entry] - 1,
            "entry_ma200": ma200[entry], "entry_rsi2": rsi[entry],
            "entry_low10": min(close[entry - 9:entry + 1]) if entry >= 9 else None,
            "exit_ma5": ma5[exit_i],
        })
        i = exit_i + 1  # Declared sequential simulation, no exit-bar re-entry.
    return expected, [row["date"] for row in rows[start:end + 1]]


def check_actual(actual, expected, dates, symbol):
    differences = []
    if actual.get("dates") != dates:
        differences.append({"field": "date_window"})
    if len(actual.get("trades", [])) != len(expected):
        differences.append({"field": "position_count", "expected": len(expected), "actual": len(actual.get("trades", []))})
    for index, (trade, oracle) in enumerate(zip(actual.get("trades", []), expected)):
        for field in ("entry", "exit", "exit_i", "open", "bars"):
            if trade.get(field) != oracle[field]:
                differences.append({"row": index, "field": field, "actual": trade.get(field), "expected": oracle[field]})
        if trade.get("dir") != 1 or trade.get("entry_kind") != "close" or len(trade.get("legs", [])) != 1:
            differences.append({"row": index, "field": "direction_or_fill_type"})
            continue
        leg = trade["legs"][0]
        if leg[0] != oracle["entry_i"] or Decimal(str(leg[2])) != 1:
            differences.append({"row": index, "field": "entry_index_or_weight"})
        for field, value, tolerance in (
            ("entry_price", leg[1], PRICE_TOL),
            ("exit_price", trade["exit_px"], PRICE_TOL),
            ("return_value", trade["ret"], RETURN_TOL),
        ):
            if abs(Decimal(str(value)) - oracle[field]) > tolerance:
                differences.append({"row": index, "field": field, "actual": value, "expected": str(oracle[field])})
        if trade.get("accounting", {}).get("symbol") != symbol:
            differences.append({"row": index, "field": "actual_symbol"})
    closed = [trade["return_value"] for trade in expected if not trade["open"]]
    if actual.get("stats", {}).get("trades") != len(closed):
        differences.append({"field": "closed_count"})
    if abs(Decimal(str(actual["stats"]["net_profit"])) - sum(closed)) > RETURN_TOL:
        differences.append({"field": "closed_net_sum"})
    if closed and abs(Decimal(str(actual["stats"]["avg_net_pnl"])) - sum(closed) / len(closed)) > RETURN_TOL:
        differences.append({"field": "average_closed_return"})
    return differences


def source_record(runtime):
    manifest = json.loads((runtime / "book-index.json").read_text())
    matches = [(key, item) for key, item in manifest["files"].items()
               if item["filename"] == "100406_STC_Session_1_Final useful.pdf"]
    if len(matches) != 1:
        raise ValueError("Session 1 PDF is not uniquely identified in staged files")
    key, item = matches[0]
    path = runtime / "books" / item["path"]
    before = path.stat()
    text = subprocess.run(["pdftotext", "-f", "13", "-l", "33", "-layout", str(path), "-"],
                          check=True, capture_output=True, text=True, timeout=30).stdout
    # Check the actual staged source table evidence, not code's claimed numbers.
    for phrase in ("510,866", "0.71%", "73,413", "1.25%", "*RSI2 < 2", "C > 5MA", "5 Day Hold"):
        if phrase not in text:
            raise ValueError(f"Required source evidence unavailable: {phrase}")
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    after = path.stat()
    if (before.st_size, before.st_mtime_ns) != (after.st_size, after.st_mtime_ns):
        raise ValueError("Source PDF changed during review")
    return {
        "local_reader_id": key, "source_path": item["path"], "source_sha256": digest,
        "source_unchanged": True, "source_pages_read": "Session 1 PDF / printed slides 13, 17 and 33; extracted supplied range 13–33",
        "printed_claims": {
            "f3a": {"page": 17, "trades": 510866, "average_return_percent": "0.71", "win_rate_percent": "55", "entry": "Close above 200MA; ten-day lowest close", "exit": "Five-day hold"},
            "rsi2_5d": {"page": 33, "trades": 73413, "average_return_percent": "1.25", "win_rate_percent": "70", "entry": "Close above 200MA; RSI2 below 2", "exit": "Close above 5MA"},
        },
        "claims_kind": "Printed pooled US-stock study reference claims. Their universe, exact dates, author ledger, fill rules and RSI seed were not supplied by these tables; these figures are not expected individual Indian-stock results.",
    }


def main():
    runtime = Path.home() / ".cache/strategy-vault-webapp"
    native_root = Path.home() / ".cache/strategy-vault/market-data"
    config = dict(line.split("=", 1) for line in (runtime / ".env").read_text().splitlines()
                  if line and not line.startswith("#"))
    report = {
        "review_date": "2026-10-13", "source": source_record(runtime),
        "original_pooled_study_reproduced": False, "all_connors_strategies_certified": False,
        "scope": "Two source predicates with declared sequential close-fill/Wilder diagnostic conventions, three actual Indian constituents, 2020–2025.",
        "diagnostic_conventions_not_certified_as_author_settings": [
            "One position per selected stock; no entry on the exit bar or final bar of the requested window.",
            "Fill at the signal close; five-day hold means five subsequent observed trading bars.",
            "Lowest close includes the signal bar in its ten-close window; equal minimum closes qualify.",
            "RSI2 uses Wilder smoothing seeded from the first two observed daily changes; this supplied table does not identify a platform/seed.",
            "RSI exit is considered from the next bar, at its close; unclosed positions are marked open at the last observed close.",
            "Zero entered fees and slippage isolate rule execution; source costs and actual statutory taxes are not established.",
            "Current basket membership has survivorship bias. Vendor corporate-action adjustments and every quote are not independently certified.",
        ], "checks": [],
    }
    with (native_root / "index.csv").open(newline="") as handle:
        selected = {row["ticker"]: row for row in csv.DictReader(handle)
                    if row["basket"] == "nifty50" and row["ticker"] in SYMBOLS and row["status"] == "ready"}
    if set(selected) != set(SYMBOLS):
        raise ValueError("Required actual stock sources are unavailable")
    base = config["VAULT_PUBLIC_URL"]
    opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
    login = urllib.request.Request(base + "/api/auth/sign-in/email", method="POST",
                                  data=json.dumps({"email": config["VAULT_ADMIN_EMAIL"], "password": config["VAULT_ADMIN_PASSWORD"]}).encode(),
                                  headers={"Content-Type": "application/json", "Origin": base})
    with opener.open(login, timeout=30) as response:
        if response.status != 200:
            raise ValueError("Sign-in failed")
    try:
        for symbol in SYMBOLS:
            source = selected[symbol]
            path = native_root / source["path"]
            before = path.stat()
            checksum = hashlib.sha256(path.read_bytes()).hexdigest()
            if checksum != source["csv_sha256"]:
                raise ValueError("Admitted stock CSV checksum mismatch")
            with path.open(newline="") as handle:
                rows = list(csv.DictReader(handle))
            if len(rows) != int(source["bars"]):
                raise ValueError("Admitted stock CSV count mismatch")
            for method in METHODS:
                expected, dates = diagnostic_trades(rows, method)
                query = {"id": method, "ticker": f"basket:nifty50:{symbol}", "from": FIRST, "to": LAST,
                         "cost": 0, "slippage": 0, "rollover": 0, "p.trend": 200}
                query.update({"p.rsi_n": 2, "p.rsi_lt": 2, "p.ma": 5} if method == "rsi2_5d" else {"p.low_n": 10, "p.hold": 5})
                with opener.open(base + "/api/in/guru/connors/run?" + urllib.parse.urlencode(query), timeout=30) as response:
                    actual = json.load(response)
                differences = check_actual(actual, expected, dates, symbol)
                if actual.get("provenance", {}).get("csv_sha256") != checksum:
                    differences.append({"field": "API_source_checksum"})
                closed = [trade["return_value"] for trade in expected if not trade["open"]]
                report["checks"].append({
                    "strategy": method, "symbol": symbol, "first": dates[0], "last": dates[-1],
                    "csv_sha256": checksum, "position_records_checked": len(expected), "closed_positions": len(closed),
                    "mismatches": differences, "closed_return_sum": str(sum(closed)),
                    "average_closed_return": str(sum(closed) / len(closed)) if closed else None,
                    "first_three_records": [{key: str(value) if isinstance(value, Decimal) else value
                                              for key, value in trade.items()} for trade in expected[:3]],
                    "checked": ["Actual CSV identity", "Source predicates", "Independent Decimal indicators", "Entry/exit indices and dates", "Actual close fill prices", "Direction and weights", "Holding bars", "Open state", "Zero-cost returns", "Closed arithmetic sum and mean"],
                })
            after = path.stat()
            unchanged = (before.st_size, before.st_mtime_ns) == (after.st_size, after.st_mtime_ns) and hashlib.sha256(path.read_bytes()).hexdigest() == checksum
            for check in report["checks"]:
                if check["symbol"] == symbol:
                    check["source_unchanged"] = unchanged
            if not unchanged:
                raise ValueError("Actual stock source changed during review")
    finally:
        logout = urllib.request.Request(base + "/api/auth/sign-out", method="POST", data=b"{}",
                                        headers={"Content-Type": "application/json", "Origin": base})
        with opener.open(logout, timeout=30):
            pass
    report["total_records_checked"] = sum(check["position_records_checked"] for check in report["checks"])
    report["total_mismatches"] = sum(len(check["mismatches"]) for check in report["checks"])
    output = Path(__file__).parents[1] / "STOCK-RSI-PULLBACK-CHECK.json"
    output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"records_checked": report["total_records_checked"], "mismatches": report["total_mismatches"],
                      "by_case": [{"strategy": c["strategy"], "symbol": c["symbol"], "records": c["position_records_checked"], "mismatches": len(c["mismatches"])} for c in report["checks"]]}))
    if report["total_mismatches"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
