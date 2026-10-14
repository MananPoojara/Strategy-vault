#!/usr/bin/env python3
"""Exercise the built React UI with synthetic browser-contract fixtures.

No books, market prices, private credentials or real performance are loaded.
These fixtures verify navigation, settings and displayed-value propagation, not
source rules, real market performance or the portfolio trading calculation.
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
from urllib.parse import parse_qs, urlencode, urlparse

ROOT = Path(__file__).resolve().parents[1]
INDIA_STOCK = "basket:nifty50:RELIANCE"
INDIA_OTHER_STOCK = "basket:nse500:TCS"
INDIA_DUPLICATE_STOCK = "basket:nse500:RELIANCE"
US_STOCK = "basket:us_sp500:AAPL"
FIXTURE_DATES = ["2010-01-04", "2010-01-05", "2010-01-06", "2010-01-07",
                 "2010-01-08", "2010-01-11", "2010-01-12", "2010-01-13",
                 "2010-01-14", "2010-01-15"]
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


def fixture_prices():
    """Deliberately artificial prices used only by the browser contract."""
    close = [100, 110, 120, 120, 115, 108, 100, 200, 300, 500]
    return {"d": FIXTURE_DATES, "o": close, "h": [x + 1 for x in close],
            "l": [x - 1 for x in close], "c": close, "v": [1000] * len(close),
            "bar_i": list(range(len(close))), "full_history": True,
            "_test_fixture": "synthetic browser contract; not market prices"}


def fixture_run(symbol):
    """A recorded UI payload, not a calculation or a verified strategy result."""
    trades = []
    for entry_i, exit_i, ret, entry_price, exit_price, opened in [
        (0, 2, .2, 100, 120, False), (3, 5, -.1, 120, 108, False),
        (6, 9, 4, 100, 500, True),
    ]:
        trades.append({"entry": FIXTURE_DATES[entry_i], "exit": FIXTURE_DATES[exit_i],
            "entry_i": entry_i, "exit_i": exit_i, "avg_px": entry_price,
            "exit_px": exit_price, "dir": 1, "ret": ret, "open": opened,
            "bars": exit_i - entry_i, "legs": [[entry_i, entry_price, 1]],
            "reason": "Synthetic open mark" if opened else "Synthetic closed exit",
            "at_exit": {}, "accounting": {"symbol": symbol.split(":")[-1],
                "signal_side": "LONG", "pts": exit_price - entry_price,
                "gross_ret": ret, "net_ret": ret, "days_held": exit_i - entry_i,
                "slippage_ret": 0, "mae_ret_observed": -.01,
                "mfe_ret_observed": max(ret, 0)}})
    stats = {"trades": 2, "net_profit": .1, "total_return": 4.4, "avg_net_pnl": .05,
        "avg_trade": .05, "win_rate": .5, "expectancy_ratio": .5, "max_dd": -.1,
        "sharpe": .7, "cagr": .12, "exposure": .6, "years": 1,
        "bh_cagr": .1, "bh_max_dd": -.12, "max_dd_days": 2, "t_stat": .4,
        "monthly_thickness": .01, "avg_bars": 2, "sortino": .8, "mar": 1.2,
        "vol": .2, "avg_win": .2, "avg_loss": -.1, "best": .2,
        "worst": -.1, "payoff": 2, "profit_factor": 2}
    return {"dates": FIXTURE_DATES, "equity": [1, 1.1, 1.2, 1.2, 1.15, 1.08, 1.08, 2, 3, 5.4],
        "bh": [1, 1.1, 1.2, 1.2, 1.15, 1.08, 1, 2, 3, 5],
        "gross_equity": [1, 1.1, 1.2, 1.2, 1.15, 1.08, 1.08, 2, 3, 5.4],
        "stats": stats, "gross_stats": dict(stats), "trades": trades,
        "ohlc": fixture_prices(), "params": {},
        "provenance": {"symbol": symbol.split(":")[-1], "source": "Synthetic browser-contract fixture"},
        "accounting_basis": "Artificial recorded browser fixture; not a trading or source verification.",
        "_test_fixture": True}


def fixture_basket(query):
    """A separate portfolio UI response; it never aliases a single-stock /run."""
    limit = int(query.get("max_positions", ["10"])[0])
    if query.get("membership", ["current"])[0] == "past":
        return {"kind": "basket", "status": "needs_data", "blocked":
            "Synthetic fixture: dated historical membership is unavailable. "
            "Auxiliary columns do not establish a verified membership contract.",
            "stats": None, "dates": [], "equity": [], "trades": [],
            "_test_fixture": True}
    data = fixture_run(INDIA_STOCK)
    data.update({"kind": "basket", "status": "ready", "equity": [1] * len(FIXTURE_DATES),
        "gross_equity": [1] * len(FIXTURE_DATES), "bh": [1] * len(FIXTURE_DATES),
        "portfolio": {"max_positions": limit, "slot_allocation": 1 / limit,
            "historical_membership_verified": False, "compounding_supported": False,
            "closed_contribution": .01, "source_members": 2, "accepted_positions": 3,
            "rejected_capacity": 0, "rejected_cash": 0, "cash": [1] * len(FIXTURE_DATES),
            "positions": [0] * len(FIXTURE_DATES),
            "allocation_basis": "Equal initial slots; unused slots stay in cash; synthetic UI contract.",
            "ranking": "Alphabetical symbol order",
            "signal_mode": "Recorded browser-only candidate ledger",
            "timing_basis": "Synthetic open/close phases",
            "gross_basis": "Same synthetic accepted fills with zero costs"},
        "coverage": [{"symbol": "RELIANCE", "first": FIXTURE_DATES[0], "last": FIXTURE_DATES[-1],
                      "status": "ready", "note": "Browser-only synthetic coverage fixture"}],
        "notices": ["Synthetic fixture; current watchlist membership is not independently verified."],
        "accounting_basis": "Synthetic equal-slot portfolio UI fixture; no market performance."})
    data["stats"].update({"net_profit": .01, "total_return": 0, "max_dd": 0,
                         "sharpe": 0, "cagr": 0})
    return data


class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, *_args):
        pass


class Fixtures:
    def __init__(self, *, range_errors=False, failed_owners=(), valid_chande=False):
        self.requests = []
        self.range_errors = range_errors
        self.failed_owners = set(failed_owners)
        self.catalogues = {"connors": CATALOGUE, "ross": [], "chande": []}
        if valid_chande:
            self.catalogues["chande"] = [definition("ui_chande_index", "ch_bta", "index")]

    def empty_range(self, query):
        if not self.range_errors:
            return False
        start, end = query.get("from", [""])[0], query.get("to", [""])[0]
        # The fixture metadata describes 2000–2020 coverage. This is not a
        # trading calculation; it only reproduces the server's range error.
        return bool((start and start > "2020-09-11")
                    or (end and end < "2000-01-03"))

    @staticmethod
    def compatible(row, selected):
        kind = row["instrument_policy"]["requirement"]
        return ((kind == "individual_stock" and selected in (INDIA_STOCK, INDIA_OTHER_STOCK, INDIA_DUPLICATE_STOCK, US_STOCK))
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
        owner = path.split("/guru/", 1)[1].split("/", 1)[0] if "/guru/" in path else None
        if path.endswith("/summary") and (self.empty_range(query) or owner in self.failed_owners):
            await route.fulfill(status=400, content_type="text/plain",
                                headers={"X-Strategy-Vault-Fixture": "browser-contract-only"},
                                body="empty date range")
            return
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
                       else [ticker("NIFTY 50"), ticker(INDIA_STOCK, "RELIANCE"),
                             ticker(INDIA_OTHER_STOCK, "TCS"), ticker(INDIA_DUPLICATE_STOCK, "RELIANCE")])
        elif path in ("/api/in/gurus", "/api/us/gurus"):
            payload = [{"id": key, "name": name, "tagline": "Browser fixture"}
                       for key, name in [("connors", "Larry Connors"),
                                         ("ross", "Joe Ross"), ("chande", "Tushar Chande")]]
        elif path.endswith("/strategies"):
            payload = self.catalogues.get(owner, [])
        elif path.endswith("/params"):
            payload = {}
        elif path.endswith("/basket-run"):
            payload = fixture_basket(query)
        elif path.endswith("/run"):
            payload = fixture_run(query.get("ticker", [INDIA_STOCK])[0])
        elif path.endswith("/ohlc") or path.startswith("/data/ohlc-"):
            payload = fixture_prices()
        elif path.endswith(("/summary", "/readiness")):
            selected = query.get("ticker", [""])[0]
            rows = self.catalogues.get(owner, [])
            if path.endswith("/readiness"):
                selected_id = query.get("id", [""])[0]
                if selected_id:
                    rows = [row for row in rows if row["id"] == selected_id]
                payload = {"strategies": [
                    {"id": row["id"], "status": "ready" if self.compatible(row, selected) and not self.empty_range(query) else "needs_data",
                     "note": "Synthetic browser fixture: selected range has no data. Coverage: 2000-01-03 to 2020-09-11."
                     if self.empty_range(query) else "Synthetic browser fixture: incompatible selected instrument."}
                    for row in rows
                ]}
            else:
                # These finite placeholders are only navigation contract values.
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
            if not us:
                payload["baskets"].append({"id": "nse500", "name": "Other source-listed fixture stocks",
                    "price_status": "ready", "symbols": [INDIA_OTHER_STOCK, INDIA_DUPLICATE_STOCK],
                    "note": "Synthetic membership metadata; no market data loaded."})
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


async def empty_range_recovery(page, base, fixtures, expect):
    query = urlencode({"basket": "nifty50", "tk": INDIA_STOCK,
                       "from": "2024-01-01", "to": "2025-12-31"})
    await page.goto(base + "/#/?" + query, wait_until="domcontentloaded")
    await expect(page.get_by_role("region", name="Library filters")).to_be_visible()
    await expect(page.get_by_text("Could not load the vault", exact=True)).to_have_count(0)
    notice = page.locator("[data-summary-unavailable]")
    await expect(notice).to_be_visible()
    await expect(notice).to_contain_text("RELIANCE")
    await expect(notice).to_contain_text("2020-09-11")
    selected_count = page.locator(".library-summary > div").filter(has_text="Backtest families").locator("dd")
    await expect(selected_count).to_have_text("0")

    # A range failure must not prevent using the filters or opening a book.
    await page.get_by_role("group", name="Browse library").get_by_role("button", name="Books & reading", exact=True).click()
    await page.locator('[data-library-book="stc9"] h3 a').click()
    await expect(page.locator(".ghero h1")).to_have_text("Swing Trading College IX")
    await expect(page.locator(".lstrip > div").nth(1).locator("dd")).to_have_text("0")
    await expect(page.get_by_text("Could not load the vault", exact=True)).to_have_count(0)

    await page.go_back(wait_until="domcontentloaded")
    await page.get_by_role("button", name="Use available history", exact=True).click()
    current = parse_qs(urlparse(page.url).fragment.partition("?")[2])
    assert current.get("tk") == [INDIA_STOCK] and current.get("basket") == ["nifty50"], current
    assert not current.get("from") and not current.get("to"), current
    await expect(page.locator("[data-summary-unavailable]")).to_have_count(0)
    await expect(page.locator(".library-summary > div").filter(has_text="Backtest families").locator("dd")).to_have_text("11")

    select = page.get_by_label("Backtest data", exact=True)
    await expect(select).to_be_visible()
    await select.select_option("i:NIFTY 50")
    await expect(select).to_have_value("i:NIFTY 50")
    current = parse_qs(urlparse(page.url).fragment.partition("?")[2])
    # The router omits its default Nifty ticker from the URL.
    assert current.get("tk", ["NIFTY 50"]) == ["NIFTY 50"] and not current.get("basket"), current
    assert not current.get("from") and not current.get("to"), current
    await expect(page.locator("[data-summary-unavailable]")).to_have_count(0)
    await page.get_by_role("group", name="Browse library").get_by_role("button", name="Authors", exact=True).click()
    connors = page.locator('[data-library-author="connors"]')
    await expect(connors.locator(".lc-stats > div").filter(has_text="Backtests available").locator("dd")).to_have_text("1")
    # Known coverage may reject the empty range before any HTTP request. The
    # user-facing state and explicit recovery must work in either path.
    assert any(item["path"].endswith("/summary") and item["query"].get("ticker") == [INDIA_STOCK]
               and item["query"].get("from") in (None, [""]) for item in fixtures.requests)
    assert any(item["path"].endswith("/summary") and item["query"].get("ticker") == ["NIFTY 50"]
               and item["query"].get("from") in (None, [""]) for item in fixtures.requests)


async def partial_owner_failure(page, base, fixtures, expect):
    await page.goto(base + "/#/", wait_until="domcontentloaded")
    chande = page.locator('[data-library-author="chande"]')
    connors = page.locator('[data-library-author="connors"]')
    await expect(chande).to_be_visible()
    await expect(chande.locator(".lc-stats > div").filter(has_text="Backtests available").locator("dd")).to_have_text("1")
    await expect(connors.locator(".lc-stats > div").filter(has_text="Backtests available").locator("dd")).to_have_text("0")
    await expect(page.locator(".library-summary > div").filter(has_text="Backtest families").locator("dd")).to_have_text("1")
    notice = page.locator("[data-summary-unavailable]")
    await expect(notice).to_be_visible()
    await expect(notice).to_contain_text("Nifty 50")
    await expect(page.get_by_text("Could not load the vault", exact=True)).to_have_count(0)
    await chande.locator(".lc-open").click()
    await expect(page.locator(".ghero h1")).to_have_text("Tushar Chande")
    await expect(page.locator(".lstrip > div").nth(1).locator("dd")).to_have_text("1")
    assert any(item["path"].endswith("/guru/chande/summary") for item in fixtures.requests)
    assert any(item["path"].endswith("/guru/connors/summary") for item in fixtures.requests)


def current_route_query(page):
    return parse_qs(urlparse(page.url).fragment.partition("?")[2])


def response_query(response, suffix, key, value):
    parsed = urlparse(response.url)
    return parsed.path.endswith(suffix) and parse_qs(parsed.query).get(key) == [value]


async def strategy_settings_and_stock(page, base, fixtures, expect):
    query = urlencode({"tk": INDIA_STOCK, "from": FIXTURE_DATES[0], "to": FIXTURE_DATES[-1]})
    await page.goto(base + "/#/s/hbqts_ma5_25?" + query, wait_until="domcontentloaded")
    await expect(page.locator('[data-tile="0"]')).to_have_text("+10.00%")
    await expect(page.locator("[data-return-definition]")).to_contain_text("sum of closed-trade")
    stock = page.get_by_label("Stock", exact=True)
    await expect(stock).to_have_value(INDIA_STOCK)
    options = await stock.locator("option").evaluate_all("els => els.map(e => ({value:e.value,text:e.textContent}))")
    assert sum(item["text"] == "RELIANCE" for item in options) == 1, options
    assert any(item["value"] == INDIA_OTHER_STOCK for item in options), options
    assert await page.locator("[data-market-controls]").evaluate("""el => {
        const index = [...el.querySelectorAll('.swl')].find(n => n.textContent === 'Index').closest('.swg');
        const stock = el.querySelector('#strategy-stock').closest('label');
        const basket = el.querySelector('.basket-selector');
        return Boolean(index.compareDocumentPosition(stock) & Node.DOCUMENT_POSITION_FOLLOWING)
            && Boolean(stock.compareDocumentPosition(basket) & Node.DOCUMENT_POSITION_FOLLOWING);
    }""")

    # The artificial recorded result is reused: changing this report mode must
    # leave other tiles, trade rows, monthly returns and equity output unchanged.
    other_tiles = await page.locator("[data-tile]").all_text_contents()
    trade_rows = await page.locator("#tradesheet tbody").inner_text()
    heatmap = await page.locator("#heatbox").inner_text()
    equity_caption = await page.locator(".box").filter(has=page.locator("#c-eq")).locator(".cap").inner_text()
    runs_before = sum(item["path"].endswith("/run") for item in fixtures.requests)
    settings = page.get_by_role("button", name="Strategy settings", exact=True)
    await settings.click()
    dialog = page.get_by_role("dialog", name="Strategy settings", exact=True)
    await expect(dialog).to_be_visible()
    await expect(dialog).to_have_attribute("aria-modal", "true")
    await dialog.locator('input[value="compound"]').check()
    cancelled_url = page.url
    await dialog.get_by_role("button", name="Cancel", exact=True).click()
    await expect(dialog).to_have_count(0)
    assert page.url == cancelled_url
    await expect(page.locator('[data-tile="0"]')).to_have_text("+10.00%")
    await expect(settings).to_be_focused()

    await settings.click()
    await dialog.locator('input[value="compound"]').check()
    await dialog.get_by_role("button", name="Apply", exact=True).click()
    await expect(page.locator('[data-tile="0"]')).to_have_text("+8.00%")
    await expect(page.locator("[data-return-definition]")).to_contain_text("product of")
    assert current_route_query(page).get("report_mode") == ["compound"]
    assert (await page.locator("[data-tile]").all_text_contents())[1:] == other_tiles[1:]
    assert await page.locator("#tradesheet tbody").inner_text() == trade_rows
    assert await page.locator("#heatbox").inner_text() == heatmap
    assert await page.locator(".box").filter(has=page.locator("#c-eq")).locator(".cap").inner_text() == equity_caption
    assert sum(item["path"].endswith("/run") for item in fixtures.requests) == runs_before

    for action in ("like", "save"):
        button = page.locator(f'[data-action="{action}"]')
        await expect(button.locator("svg")).to_have_count(1)
        assert not (await button.inner_text()).strip()
        await button.click()
        await expect(button).to_have_attribute("aria-pressed", "true")
        await expect(button.locator("svg")).to_have_count(1)
        assert not (await button.inner_text()).strip()
        await button.click()
        await expect(button).to_have_attribute("aria-pressed", "false")
        assert not (await button.inner_text()).strip()
    for action in ("share-menu", "pdf"):
        button = page.locator(f'[data-action="{action}"]')
        await expect(button.locator("svg")).to_have_count(1)
        assert await button.get_attribute("aria-label")
        assert not (await button.inner_text()).strip()

    await page.reload(wait_until="domcontentloaded")
    await expect(page.locator('[data-tile="0"]')).to_have_text("+8.00%")
    assert current_route_query(page).get("report_mode") == ["compound"]
    async with page.expect_response(lambda response: response_query(response, "/run", "ticker", INDIA_OTHER_STOCK)):
        await page.get_by_label("Stock", exact=True).select_option(INDIA_OTHER_STOCK)
    await expect(page.get_by_label("Stock", exact=True)).to_have_value(INDIA_OTHER_STOCK)
    await expect(page.locator('[data-tile="0"]')).to_have_text("+8.00%")
    current = current_route_query(page)
    assert current.get("tk") == [INDIA_OTHER_STOCK] and not current.get("basket"), current
    assert current.get("run_scope", ["single"]) == ["single"], current
    assert any(item["path"].endswith("/run") and item["query"].get("ticker") == [INDIA_OTHER_STOCK]
               for item in fixtures.requests)

    # An explicitly named qualified source stays selected even when the same
    # stock is also listed by another basket.
    await page.goto(base + "/#/s/hbqts_ma5_25?" + urlencode({"tk": INDIA_DUPLICATE_STOCK}), wait_until="domcontentloaded")
    await expect(page.get_by_label("Stock", exact=True)).to_have_value(INDIA_DUPLICATE_STOCK)
    options = await page.get_by_label("Stock", exact=True).locator("option").all_text_contents()
    assert options.count("RELIANCE") == 1, options


async def strategy_basket_controls(page, base, fixtures, expect):
    await page.goto(base + "/#/s/hbqts_ma5_25?" + urlencode({"tk": INDIA_STOCK}), wait_until="domcontentloaded")
    await expect(page.locator('[data-tile="0"]')).to_have_text("+10.00%")
    begin = len(fixtures.requests)
    await page.locator('[data-basket="nifty50"]').click()
    await expect(page.locator("[data-basket-result]")).to_be_visible()
    current = current_route_query(page)
    assert current.get("run_scope") == ["basket"] and current.get("basket") == ["nifty50"], current
    requests = fixtures.requests[begin:]
    assert any(item["path"].endswith("/basket-run") and item["query"].get("basket") == ["nifty50"]
               and item["query"].get("max_positions") == ["10"] for item in requests), requests
    assert not any(item["path"].endswith("/run") for item in requests), requests
    await expect(page.get_by_label("Stock", exact=True)).to_have_value("")
    await expect(page.locator('[data-position-choice="10"]')).to_have_attribute("aria-pressed", "true")

    await page.get_by_role("button", name="Strategy settings", exact=True).click()
    dialog = page.get_by_role("dialog", name="Strategy settings", exact=True)
    await expect(dialog).to_contain_text("equal-slot portfolio calculation")
    await expect(dialog.get_by_role("button", name="Apply", exact=True)).to_be_disabled()
    await dialog.get_by_role("button", name="Cancel", exact=True).click()

    await page.locator('[data-position-choice="20"]').click()
    await expect(page.locator('[data-position-choice="20"]')).to_have_attribute("aria-pressed", "true")
    await expect(page.locator("[data-basket-result]")).to_be_visible()
    assert current_route_query(page).get("max_positions") == ["20"]
    assert any(item["path"].endswith("/basket-run") and item["query"].get("max_positions") == ["20"]
               for item in fixtures.requests[begin:])
    await page.locator('[data-position-choice="custom"]').click()
    custom = page.locator("#position-limit")
    await expect(custom).to_be_visible()
    await custom.locator('input[name="max_positions"]').fill("7")
    async with page.expect_response(lambda response: response_query(response, "/basket-run", "max_positions", "7")):
        await custom.get_by_role("button", name="Apply", exact=True).click()
    await expect(page.locator("[data-basket-result]")).to_be_visible()
    assert current_route_query(page).get("max_positions") == ["7"]
    assert any(item["path"].endswith("/basket-run") and item["query"].get("max_positions") == ["7"]
               for item in fixtures.requests[begin:])

    past = page.locator('[data-membership="past"]')
    title = await past.get_attribute("title")
    assert title and "\n" in title and "dated membership" in title.lower() and "review" in title.lower(), title
    assert "removed" not in title.lower(), title
    await past.click()
    withheld = page.locator('[data-backtest-state="needs_data"]')
    await expect(withheld).to_be_visible()
    await expect(withheld).to_contain_text("historical membership is unavailable")
    await expect(page.locator("[data-tile]")).to_have_count(0)
    await expect(page.locator("#c-eq,#c-px,#c-dd,#c-yr")).to_have_count(0)
    await expect(page.locator("[data-market-controls]")).to_be_visible()
    await expect(page.locator("[data-source-audit]")).to_be_visible()
    await expect(page.get_by_text("Could not load the vault", exact=True)).to_have_count(0)
    assert any(item["path"].endswith("/basket-run") and item["query"].get("membership") == ["past"]
               for item in fixtures.requests[begin:])

    # An individual Stock selection exits portfolio mode and uses its actual
    # source-qualified price key instead of retaining the basket universe.
    await page.get_by_label("Stock", exact=True).select_option(INDIA_OTHER_STOCK)
    await expect(page.locator('[data-tile="0"]')).to_have_text("+10.00%")
    current = current_route_query(page)
    assert current.get("run_scope", ["single"]) == ["single"] and not current.get("basket"), current
    assert current.get("tk") == [INDIA_OTHER_STOCK], current
    assert any(item["path"].endswith("/run") and item["query"].get("ticker") == [INDIA_OTHER_STOCK]
               for item in fixtures.requests[begin:])


async def smoke(dist):
    from playwright.async_api import async_playwright, expect
    server = ThreadingHTTPServer(("127.0.0.1", 0), partial(QuietHandler, directory=str(dist)))
    server.daemon_threads = True
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    base = "http://127.0.0.1:" + str(server.server_port)
    errors = []
    try:
        async with async_playwright() as playwright:
            browser = await playwright.chromium.launch(headless=True, timeout=20000)
            try:
                async def journeys():
                    cases = [
                        (stock_discovery, Fixtures()),
                        (etf_selection, Fixtures()),
                        (empty_range_recovery, Fixtures(range_errors=True)),
                        (partial_owner_failure, Fixtures(failed_owners=("connors",), valid_chande=True)),
                        (strategy_settings_and_stock, Fixtures()),
                        (strategy_basket_controls, Fixtures()),
                    ]
                    for journey, fixtures in cases:
                        # Each journey owns its browser context and runtime caches.
                        context = await browser.new_context(viewport={"width": 1440, "height": 1000}, service_workers="block")
                        try:
                            await context.route("**/*", fixtures.route)
                            page = await context.new_page()
                            page.set_default_timeout(12000)
                            page.on("pageerror", lambda error: errors.append(str(error)))
                            await asyncio.wait_for(journey(page, base, fixtures, expect), timeout=40)
                        finally:
                            await context.close()
                    return len(cases)

                # Leave time for startup/cleanup within the existing three-minute CI step.
                completed = await asyncio.wait_for(journeys(), timeout=140)
                assert not errors, errors
                print(json.dumps({"suite": "synthetic author-discovery browser contract",
                                  "journeys_passed": completed, "page_errors": errors,
                                  "financial_verification": False}))
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
