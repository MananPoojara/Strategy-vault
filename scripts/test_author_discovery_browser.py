#!/usr/bin/env python3
"""Exercise the built React UI with synthetic browser-contract fixtures.

No books, market prices, private credentials or real performance are loaded.
These fixtures verify navigation/count propagation, not trading calculations.
Playwright is imported only by the executable entry point, so ordinary Python
unit-test discovery does not require an installed browser dependency.
"""
import argparse
import asyncio
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import threading
from urllib.parse import parse_qs, urlparse

ROOT = Path(__file__).resolve().parents[1]
INDIA_STOCK = "basket:nifty50:RELIANCE"
US_STOCK = "basket:us_sp500:AAPL"
STOCK_IDS = [
    "hbqts_ma5_25", "hbqts_ma1_36", "hbqts_limit_entry",
    "hbqts_fixed_interval", "hbqts_profit_target", "hbqts_entry_id1",
    "hbqts_bb_breakout", "hbqts_cgo", "hbqts_stoch_rsi", "hbqts_dpo",
    "hbqts_dom", "hbqts_macd_daily",
]


def definition(method, book, kind, family=None):
    return {
        "id": method, "book": book, "family": family or method,
        "family_name": "UI fixture: " + (family or method),
        "name": "UI fixture: " + method, "variant": "", "group": "UI fixture",
        "dir": "Long", "status": "live", "needs_series": [],
        "needs_volume": False, "needs_vix": False, "rules": [],
        "notes": "Synthetic navigation fixture; no strategy or book verification.",
        "claim": None,
        "instrument_policy": {
            "requirement": kind, "allowed_types": [kind],
            "preferred_basket": "nifty50" if kind == "individual_stock" else None,
            "execution_scope": "single_symbol", "source_rule_reviewed": False,
            "source_basis": "Synthetic browser-contract fixture only.",
            "source_performance_reproduced": False,
        },
    }


CATALOGUE = [
    definition(method, "hbqts", "individual_stock",
               "hbqts_ma" if method.startswith("hbqts_ma") and method != "hbqts_macd_daily" else None)
    for method in STOCK_IDS
] + [definition("hbqts_cmo", "hbqts", "etf"),
     definition("r3", "stc9", "index")]


def ticker(name, symbol=None):
    return {"ticker": name, "symbol": symbol or name,
            "first": "2000-01-03", "last": "2020-09-11", "bars": 1000,
            "_test_fixture": True}


class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, *_args):
        pass


class Fixtures:
    def __init__(self):
        self.requests = []

    @staticmethod
    def compatible(row, selected):
        kind = row["instrument_policy"]["requirement"]
        return ((kind == "individual_stock" and selected in (INDIA_STOCK, US_STOCK))
                or (kind == "etf" and selected in ("SPY", "QQQ"))
                or (kind == "index" and selected in ("NIFTY 50", "SPX")))

    async def route(self, route):
        request = route.request
        parsed = urlparse(request.url)
        if parsed.hostname not in ("127.0.0.1", "localhost"):
            await route.abort()
            return
        path, query = parsed.path, parse_qs(parsed.query)
        if not path.startswith(("/api/", "/data/")):
            await route.continue_()
            return
        self.requests.append({"path": path, "query": query, "method": request.method})
        payload, status = None, 200
        if path == "/api/app/me":
            payload = {"user": {"id": "fixture-user", "name": "Browser fixture",
                                "email": "fixture@example.invalid", "role": "researcher"},
                       "settings": {"theme": "aqua"}}
        elif path == "/api/app/settings":
            payload = {"settings": {}}
        elif path.endswith("/tickers"):
            payload = ([ticker("SPX"), ticker("SPY"), ticker("QQQ"), ticker(US_STOCK, "AAPL")]
                       if path.startswith("/api/us/")
                       else [ticker("NIFTY 50"), ticker(INDIA_STOCK, "RELIANCE")])
        elif path in ("/api/in/gurus", "/api/us/gurus"):
            payload = [{"id": key, "name": name, "tagline": "Browser fixture"}
                       for key, name in [("connors", "Larry Connors"),
                                         ("ross", "Joe Ross"), ("chande", "Tushar Chande")]]
        elif path.endswith("/strategies"):
            payload = CATALOGUE if "/guru/connors/" in path else []
        elif path.endswith("/params"):
            payload = {}
        elif path.endswith(("/summary", "/readiness")):
            selected = query.get("ticker", [""])[0]
            rows = CATALOGUE if "/guru/connors/" in path else []
            if path.endswith("/readiness"):
                payload = {"strategies": [
                    {"id": row["id"], "status": "ready" if self.compatible(row, selected) else "needs_data",
                     "note": "Synthetic browser fixture: incompatible selected instrument."}
                    for row in rows
                ]}
            else:
                # Zero-return placeholders are finite UI-contract values. No price
                # series, real P&L, original-book statistics or /run is fabricated.
                payload = [{"id": row["id"], "stats": {
                    "trades": 25, "net_profit": 0, "total_return": 0,
                    "sharpe": 0, "cagr": 0, "max_dd": 0, "win_rate": 0,
                    "t_stat": 0, "avg_bars": 3, "years": 10,
                    "monthly_thickness": 0, "expectancy_ratio": 0, "mar": 0,
                }, "spark": [], "_test_fixture": True}
                    for row in rows if self.compatible(row, selected)]
        elif path in ("/data/baskets.json", "/data/baskets-us.json"):
            us = path.endswith("-us.json")
            payload = {"baskets": [{"id": "us_sp500" if us else "nifty50",
                "name": "Browser fixture stocks", "price_status": "ready",
                "symbols": [US_STOCK if us else INDIA_STOCK],
                "note": "Synthetic membership metadata; no market data loaded."}]}
        else:
            status, payload = 404, {"error": "No data in browser-contract fixture"}
        await route.fulfill(status=status, content_type="application/json",
                            headers={"X-Strategy-Vault-Fixture": "browser-contract-only"},
                            body=json.dumps(payload))


async def stock_discovery(page, base, fixtures, expect):
    await page.goto(base + "/#/", wait_until="domcontentloaded")
    bandy = page.locator('[data-library-author="bandy"]')
    await expect(bandy).to_be_visible()
    count = bandy.locator(".lc-stats > div").filter(has_text="Backtests available").locator("dd")
    await expect(count).to_have_text("11")
    await expect(bandy.locator('[data-backtest-context="bandy"]')).to_contain_text("RELIANCE")
    await expect(bandy.locator('[data-backtest-context="bandy"]')).to_contain_text("2010-01-01 to 2020-09-11")
    order = await page.locator("[data-library-author]").evaluate_all("els => els.map(el => el.dataset.libraryAuthor)")
    assert order.index("bandy") < order.index("connors"), order
    await bandy.locator(".lc-open").click()
    await expect(page.locator('[data-book="hbqts"]')).to_be_visible()
    query = parse_qs(urlparse(page.url).fragment.split("?", 1)[1])
    assert query["basket"] == ["nifty50"] and query["tk"] == [INDIA_STOCK], query
    assert query["from"] == ["2010-01-01"] and query["to"] == ["2020-09-11"], query
    await page.locator('[data-book="hbqts"]').click()
    await expect(page.locator(".ghero h1")).to_have_text("Quantitative Trading Systems")
    await expect(page.locator("[data-list-period]")).to_contain_text("2010-01-01 to 2020-09-11")
    await expect(page.locator(".lstrip > div").nth(1).locator("dd")).to_have_text("12")
    query = parse_qs(urlparse(page.url).fragment.split("?", 1)[1])
    assert query["book"] == ["hbqts"] and query["tk"] == [INDIA_STOCK], query
    assert query["from"] == ["2010-01-01"] and query["to"] == ["2020-09-11"], query
    assert any(item["path"].endswith("/summary")
               and item["query"].get("ticker") == [INDIA_STOCK]
               and item["query"].get("from") == ["2010-01-01"]
               and item["query"].get("to") == ["2020-09-11"]
               for item in fixtures.requests)


async def etf_selection(page, base, fixtures, expect):
    await page.goto(base + "/#/?region=us", wait_until="domcontentloaded")
    select = page.get_by_label("Backtest data", exact=True)
    await expect(select).to_be_visible()
    await select.select_option("i:SPY")
    await expect(select).to_have_value("i:SPY")
    await expect(page.locator('[data-backtest-context="bandy"]')).to_contain_text("SPY")
    query = parse_qs(urlparse(page.url).fragment.split("?", 1)[1])
    assert query["region"] == ["us"] and query["tk"] == ["SPY"], query
    assert not query.get("basket"), query
    assert any(item["path"].startswith("/api/us/") and item["path"].endswith("/summary")
               and item["query"].get("ticker") == ["SPY"] for item in fixtures.requests)


async def smoke(dist):
    from playwright.async_api import async_playwright, expect
    server = ThreadingHTTPServer(("127.0.0.1", 0), partial(QuietHandler, directory=str(dist)))
    server.daemon_threads = True
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    base = "http://127.0.0.1:" + str(server.server_port)
    fixtures, errors = Fixtures(), []
    try:
        async with async_playwright() as playwright:
            browser = await playwright.chromium.launch(headless=True, timeout=20000)
            try:
                context = await browser.new_context(viewport={"width": 1440, "height": 1000}, service_workers="block")
                await context.route("**/*", fixtures.route)
                page = await context.new_page()
                page.set_default_timeout(12000)
                page.on("pageerror", lambda error: errors.append(str(error)))
                try:
                    await asyncio.wait_for(stock_discovery(page, base, fixtures, expect), timeout=40)
                    await asyncio.wait_for(etf_selection(page, base, fixtures, expect), timeout=40)
                    assert not errors, errors
                    print(json.dumps({"suite": "synthetic author-discovery browser contract",
                                      "journeys_passed": 2, "page_errors": errors,
                                      "financial_verification": False}))
                finally:
                    await context.close()
            finally:
                await browser.close()
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dist", type=Path, default=ROOT / "Context/guru/frontend/react/dist")
    args = parser.parse_args()
    if not (args.dist / "index.html").is_file():
        parser.error("Build the React frontend before running this browser contract test.")
    asyncio.run(smoke(args.dist.resolve()))


if __name__ == "__main__":
    main()
