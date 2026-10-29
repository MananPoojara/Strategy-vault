import { expect, it, vi } from "vitest";
import { act } from "react";
import { distribution, formulaText, parameterLines, ReportPanel, settingsLines, symbolRows } from "../src/features/lab/Report";
import { defaultSpec } from "../src/features/lab/spec-model";
import type { LabReport, ReportSide, Trade } from "../src/features/lab/types";
import { mount } from "./fixtures/lab";

const group = { count: 1, pct: 0.5, total: 0.1, avg: 0.1, avg_bars: 3, largest: 0.1, bars_in_largest: 3, max_consecutive: 1 };
const side = (trades: number): ReportSide => ({ trades, avg: 0.02, avg_bars: 3, profit_factor: 2, payoff: 1.5, winners: group, losers: { ...group, total: -0.05, avg: -0.05, largest: -0.05 } });
const report: LabReport = { all: side(2), long: side(2), short: side(0), system: { net_profit: 0.12, k_ratio: 1.234, recovery_factor: null }, notes: ["declared note"] };
const trade = (ret: number, mae: number, mfe: number) => ({ ret, mae, mfe }) as Trade;

it("buckets trade returns by 5 % including negatives and the edges", () => {
  expect(distribution([])).toEqual([]);
  const bins = distribution([-0.06, -0.01, 0, 0.049, 0.05, 0.12]);
  expect(bins.map((b) => [Math.round(b.from * 100), b.count])).toEqual([[-10, 1], [-5, 1], [0, 2], [5, 1], [10, 1]]);
});

it("one AmiBroker table: All | Long | Short (always), no Buy&Hold column without buy-and-hold figures, capital block first, system rows last, the declared notes", async () => {
  const host = await mount(<ReportPanel report={report} trades={[trade(0.1, -0.02, 0.12), trade(-0.05, -0.07, 0.01)]} />);
  const head = [...host.querySelectorAll("[data-lab-report-stats] table:first-of-type thead th")].map((th) => th.textContent);
  expect(head).toEqual(["Statistics", "All trades", "Long trades", "Short trades"]);
  const rows = [...host.querySelectorAll("[data-lab-report-stats] tbody tr")].map((r) => [...r.children].map((c) => c.textContent));
  expect(rows.slice(0, 3)).toEqual([["Initial capital", "1.000", "1.000", "1.000"], ["Ending capital", "1.120", "—", "—"], ["Net Profit", "0.120", "—", "—"]]);
  expect(rows.find((r) => r[0] === "Transaction costs")).toEqual(["Transaction costs", "—", "—", "—"]);
  expect(rows.find((r) => r[0] === "All trades")).toEqual(["All trades", "2", "2", "0"]);
  const system = host.querySelector("[data-lab-report-system]")!.textContent!;
  expect(system).toContain("K-Ratio1.234——");
  expect(system).toContain("Recovery Factor———"); // null from the engine stays a dash, never a number
  expect(rows.find((r) => r[0] === "Profit Factor")).toEqual(["Profit Factor", "2.00", "2.00", "2.00"]);
  expect(host.querySelector("[data-lab-profit-distribution]")).not.toBeNull();
  expect(host.querySelectorAll("[data-lab-excursion=mae] line.dot")).toHaveLength(2);
  expect(host.querySelector("[data-lab-report-constants]")!.textContent).toContain("Risk-free rate 5 %");
  expect(host.textContent).toContain("declared note");
  const custom = await mount(<ReportPanel report={{ ...report, rates: { sharpe: 6.5, upi: 3 } }} trades={[]} />);
  expect(custom.querySelector("[data-lab-report-constants]")!.textContent).toContain("Risk-free rate 6.5 % (Sharpe of trades) · 3 % (Ulcer performance index)");
});

it("MAE/MFE switch to AmiBroker distribution histograms", async () => {
  const host = await mount(<ReportPanel report={report} trades={[trade(0.1, -0.02, 0.12), trade(-0.05, -0.07, 0.01)]} />);
  await act(async () => [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "MAE/MFE distribution")!.click());
  expect(host.querySelectorAll("[data-lab-excursion=mae] line.dot")).toHaveLength(0);
  expect(host.querySelectorAll("[data-lab-excursion=mae] rect")).toHaveLength(2); // -10..-5 % and -5..0 % buckets
  expect(host.querySelector("[data-lab-excursion=mfe] figcaption")!.textContent).toContain("Max. Favourable Excursion distribution");
});

it("money mode opens with capital, ending capital and costs; Buy&Hold names the symbol", async () => {
  const withBh: LabReport = { ...report, buy_hold: { net_profit: 0.3 } };
  const run = { from: "2020-01-01", to: "2021-01-01", instrument: { ticker: "NIFTY 50" }, money: { capital: 100000, final_equity: 112500, costs_total: 321.5 } };
  const host = await mount(<ReportPanel report={withBh} trades={[]} run={run} />);
  const rows = [...host.querySelectorAll("[data-lab-report-stats] tbody tr")].map((r) => [...r.children].map((c) => c.textContent));
  expect(rows.slice(0, 3)).toEqual([["Initial capital", "100,000.00", "100,000.00", "100,000.00", "100,000.00"], ["Ending capital", "112,500.00", "—", "—", "130,000.00"], ["Net Profit", "12,500.00", "—", "—", "30,000.00"]]);
  expect(rows.find((r) => r[0] === "Transaction costs")![1]).toBe("321.50");
  expect(host.querySelector("[data-lab-report-stats] thead th:last-child")!.textContent).toBe("Buy&Hold (NIFTY 50)");
  expect(host.querySelector("[data-lab-report-constants]")!.textContent).toContain("Buy&Hold: NIFTY 50");
});

it("is labelled simulated · not a book method, Buy&Hold beside the system rows, gross profit rows and the readable Settings that ran", async () => {
  const withBh: LabReport = { ...report, all: { ...side(2), winners: { ...group, profit: 0.2 } }, buy_hold: { net_profit: 0.3, max_trade_dd: 0, sharpe_trades: null } };
  const spec = { ...defaultSpec(), exit: { any: [{ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } }], stop_loss: { pct: 2 }, trailing: { profit_pct: 40 } },
    execution: { prices: { buy: "close" as const }, delays: { buy: 0 }, reentry: { stop_loss: 3 }, capital: 1e5, limits: { round_lot: 10 } } };
  const host = await mount(<ReportPanel report={withBh} trades={[trade(0.1, -0.02, 0.12)]} spec={spec} run={{ from: "2020-01-01", to: "2021-01-01" }} />);
  expect(host.querySelector("h3")!.textContent).toContain("simulated · not a book method");
  const rows = [...host.querySelectorAll("[data-lab-report-stats] tbody tr")].map((r) => [...r.children].map((c) => c.textContent));
  expect(rows.find((r) => r[0] === "Net Profit %")).toEqual(["Net Profit %", "12.00%", "—", "—", "30.00%"]);
  expect(rows.find((r) => r[0] === "Max. trade % drawdown")!.at(-1)).toBe("—");
  expect(rows.find((r) => r[0] === "Total Profit (× initial equity)")![1]).toBe("0.200");
  const settings = host.querySelector("[data-lab-report-settings]")!.textContent!;
  for (const text of ["Range01-01-2020 → 01-01-2021", "PeriodicityDaily", "Initial equity100,000", "Buy priceClose, delay 0 bars", "Sell priceOpen, delay 1 bar", "Stop-loss 2%", "Trailing stop 40% of profit", "N-bar stop 5 bars", "3 bars after a stop-loss", "round lot 10"]) expect(settings).toContain(text);
  expect(settingsLines(defaultSpec())).toEqual([["Periodicity", "Daily"], ["Positions", "Long"], ["Entry price", "Next open"], ["Exit price", "Next open"], ["Orders and stops", "N-bar stop 5 bars"]]);
});

it("lists the indicator parameters the rules use", () => {
  const spec = { ...defaultSpec(), entry: { all: [{ lhs: { s: "rsi", p: { length: 14 } }, op: "<", rhs: { value: 30 } }, { lhs: { s: "close" }, op: ">", rhs: { s: "sma", p: { length: 200 } } }] } };
  expect(parameterLines(spec as never)).toEqual([["rsi length", "14"], ["sma length", "200"]]);
});

const rowsOf = (host: HTMLElement) => [...host.querySelectorAll("[data-lab-report-stats] tbody tr")].map((r) => [...r.children].map((c) => c.textContent));
const pageButton = (host: HTMLElement, label: string) => [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === label)!;

it("Buy&Hold per-side column: report.buyhold fills the trade rows, null stays a dash, older responses keep the dashes", async () => {
  const bhSide: ReportSide = { ...side(1), avg: 0.3, profit_factor: null, losers: { ...group, count: 0, avg: null } };
  const host = await mount(<ReportPanel report={{ ...report, buy_hold: { net_profit: 0.3 }, buyhold: bhSide }} trades={[]} />);
  const rows = rowsOf(host);
  expect(rows.find((r) => r[0] === "All trades")).toEqual(["All trades", "2", "2", "0", "1"]);
  expect(rows.find((r) => r[0] === "Avg. Profit/Loss %")!.at(-1)).toBe("30.00%");
  expect(rows.find((r) => r[0] === "Profit Factor")!.at(-1)).toBe("—");
  expect(rows.filter((r) => r[0] === "Avg. Loss %")[0].at(-1)).toBe("—");
  expect(host.querySelector("[data-lab-report-constants]")!.textContent).toContain("one buy-and-hold trade");
  const old = await mount(<ReportPanel report={{ ...report, buy_hold: { net_profit: 0.3 } }} trades={[]} />);
  expect(rowsOf(old).find((r) => r[0] === "All trades")).toEqual(["All trades", "2", "2", "0", "—"]);
  const nullSide = await mount(<ReportPanel report={{ ...report, buyhold: null }} trades={[]} />);
  expect(nullSide.querySelectorAll("[data-lab-report-stats] table:first-of-type thead th")).toHaveLength(4); // no buy-and-hold figures: no column
});

it("Formula page renders the rules as labelled AFL-like text with stops and trade settings, and copies it", async () => {
  const spec = { ...defaultSpec(), direction: "both" as const,
    entry: { all: [{ lhs: { s: "ema", p: { length: 9 } }, op: "crosses_above", rhs: { s: "ema", p: { length: 21 } } }, { lhs: { s: "rsi", p: { length: 14 }, offset: 1 }, op: "between", lo: { value: 30 }, hi: { value: 70 } }] },
    exit: { any: [{ lhs: { s: "close" }, op: "<", rhs: { highest: { s: "high" }, bars: 20 } }, { lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } }], stop_loss: { pct: 2 }, trailing: { profit_pct: 40 } },
    short_side: { entry: { all: [{ not: { lhs: { s: "close" }, op: ">", rhs: { value: 100 } } }] }, exit: { any: [] }, },
    execution: { sizing: { pct_equity: 10 } } };
  const text = formulaText(spec as never);
  expect(text.split("\n")).toEqual([
    "// Readable rendering of the lab rules in AFL-like syntax: not real AFL, not run by AmiBroker.",
    "Buy = (Cross(EMA(9), EMA(21)) AND (Ref(RSI(14), -1) >= 30 AND Ref(RSI(14), -1) <= 70));",
    "Sell = (C < HHV(H, 20));",
    "ApplyStop(stopTypeLoss, stopModePercent, 2); // long side",
    "ApplyStop(stopTypeTrailing, stopModeRisk, 40); // long side",
    "ApplyStop(stopTypeNBar, stopModeBars, 5); // long side",
    "Short = (NOT C > 100);",
    "Cover = 0;",
    "SetTradeDelays(1, 1, 1, 1);",
    "BuyPrice = Open; SellPrice = Open;",
    "SetPositionSize(10, spsPercentOfEquity);",
    'SetOption("ActivateStopsImmediately", True);',
  ]);
  // a filter with "also sell when it turns false" (the AFLs' ST Exit / Base exit)
  const filtered = formulaText({ ...defaultSpec(), entry: { all: [defaultSpec().entry.all[0], { lhs: { s: "close" }, op: ">", rhs: { value: 100 }, filter: true, filter_exit: true }] } }).split("\n");
  expect(filtered.slice(1, 4)).toEqual(["Buy = (C > EMA(50) AND C > 100);", "Sell = 0;", "Sell = Sell OR NOT (C > 100);"]);
  expect(formulaText({ ...defaultSpec(), execution: { prices: { buy: "close" }, delays: { buy: 0 } } }).split("\n").slice(-4, -1)).toEqual(["SetTradeDelays(0, 1, 0, 0);", "BuyPrice = Close;", "SellPrice = Open;"]);
  // Stops-tab and General settings a spec changes show in the formula: ExitAtStop, N-bar priority, same-bar exit, account margin, tick size
  const set = formulaText({ ...spec, execution: { exit_at_stop: { stop_loss: 0, bars: 2 }, nbar_has_priority: true, activate_stops_immediately: false, allow_same_bar_exit: false, tick_size: 0.05, capital: 1e5, limits: { margin_pct: 50 } } } as never).split("\n");
  expect(set).toEqual(expect.arrayContaining(["ApplyStop(stopTypeLoss, stopModePercent, 2, 0); // long side", "ApplyStop(stopTypeTrailing, stopModeRisk, 40); // long side",
    "ApplyStop(stopTypeNBar, stopModeBars, 5, 2); // long side // N-bar stop has priority", 'SetOption("ActivateStopsImmediately", False);', 'SetOption("AllowSameBarExit", False);', 'SetOption("AccountMargin", 50);', "TickSize = 0.05;"]));
  const host = await mount(<ReportPanel report={report} trades={[]} spec={spec as never} />);
  // Stats shows the statistics only: no Formula or Symbols page
  expect(host.querySelector("[aria-label='Report page']")).toBeNull();
  expect(host.querySelector("[data-lab-report-formula], [data-lab-report-symbols]")).toBeNull();
  expect(host.querySelector("[data-lab-report-stats]")).not.toBeNull();
});

it("Symbols page: one row for a single instrument, per-symbol rows for a portfolio trade list", async () => {
  const run = { from: "2020-01-01", to: "2021-01-01", dates: ["2020-01-01", "2020-06-01", "2021-01-01"], instrument: { ticker: "NIFTY 50" }, money: { capital: 1e5, final_equity: 112500, costs_total: 0 } };
  const ts = [{ ret: 0.1, open: false }, { ret: -0.05, open: false }, { ret: 0.2, open: true }] as Trade[];
  expect(symbolRows(ts, run)).toEqual([["NIFTY 50", "2020-01-01", "2021-01-01", "3", "3", "12,500.00", "50.0%"]]);
  expect(symbolRows([], undefined, report)[0]).toEqual(["This instrument", "—", "—", "—", "0", "12.00%", "—"]);
  const pt = (ticker: string, entry: string, exit: string, entry_px: number, exit_px: number, qty: number, dir: 1 | -1, open = false) =>
    ({ ticker, entry, exit, entry_px, exit_px, qty, dir, open, costs: 10, ret: ((exit_px - entry_px) / entry_px) * dir }) as unknown as Trade;
  const port = [pt("TCS", "2020-02-01", "2020-03-01", 100, 110, 10, 1), pt("INFY", "2020-01-05", "2020-01-20", 50, 45, 20, 1), pt("TCS", "2020-04-01", "2020-05-01", 100, 90, 5, -1), pt("TCS", "2020-06-01", "", 100, 120, 5, 1, true)];
  expect(symbolRows(port, run)).toEqual([["INFY", "2020-01-05", "2020-01-20", "—", "1", "-110.00", "0.0%"], ["TCS", "2020-02-01", "2020-05-01", "—", "3", "130.00", "100.0%"]]);
});

it("the formula page writes the Dual ATR exit and since-entry values readably", () => {
  const s = { v: 1, entry: { all: [{ lhs: { s: "close" }, op: ">", rhs: { value: 1 } }] },
    exit: { any: [{ lhs: { s: "close" }, op: "<", rhs: { since_entry: "highest", of: { s: "close" }, mul: 0.95 } }],
      dual_atr: { initial: { period: 5, mult: 2, price: "open" }, favour: { period: 5, mult: 4 }, tf: "W", period_end_only: true } } };
  const text = formulaText(s as never);
  expect(text).toContain("Sell = (C < (HighestSince(Buy, C) * 0.95));");
  expect(text).toContain("// Dual ATR exit: initial O - 2 * ATR(5), favour (H + L) / 2 -/+ 4 * ATR(5), ATR = MA(TrueRange) on W bars, exit tested at the period end");
});

it("the formula page and the N-bar stop skip switched-off rules", () => {
  const s = { v: 1, entry: { all: [{ lhs: { s: "close" }, op: ">", rhs: { value: 1 } }, { off: true, lhs: { s: "close" }, op: "<", rhs: { value: 9 } }] },
    exit: { any: [{ off: true, lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } }, { lhs: { s: "close" }, op: "<", rhs: { value: 0.5 } }] } };
  const text = formulaText(s as never);
  expect(text).toContain("Buy = (C > 1);");
  expect(text).toContain("Sell = (C < 0.5);");
  expect(text).not.toContain("stopTypeNBar");
});

it("settings and formula pages show an active date filter and its window-change exit", () => {
  const spec = { ...defaultSpec(), date_filter: { mode: "outside" as const, windows: [["2008-01-01", "2008-01-31"], ["2008-11-21", "2010-12-31"]] as [string, string][], set: { id: 1, name: "Base 2" } } };
  expect(settingsLines(spec).find(([k]) => k === "Date filter")?.[1]).toBe("Base 2: trade only outside 2 windows (2008-01-01 → exit date 2010-12-31); an open trade closes when the window changes");
  const text = formulaText(spec).split("\n");
  expect(text).toContain("InWindow = (DateNum() >= 1080101 AND DateNum() < 1080131) OR (DateNum() >= 1081121 AND DateNum() < 1101231);");
  expect(text).toContain("DateOK = NOT InWindow;");
  expect(text).toContain("Buy = Buy AND DateOK;");
  expect(text).toContain("Sell = Sell OR NOT DateOK;");
  expect(formulaText({ ...spec, date_filter: { ...spec.date_filter, off: true } })).not.toContain("DateOK");
  expect(settingsLines({ ...spec, date_filter: { ...spec.date_filter, exit_at_change: false } }).find(([k]) => k === "Date filter")?.[1]).not.toContain("closes");
});

it("Stats colours profit and drawdown figures by sign, as AmiBroker's report does; counts and dashes stay plain", async () => {
  const host = await mount(<ReportPanel report={{ ...report, system: { ...report.system, max_sys_dd: -0.35, exposure: 0.5 } }} trades={[]} />);
  const cell = (label: string) => [...host.querySelectorAll("[data-lab-report-stats] tbody tr")].find((r) => r.children[0].textContent === label)!.children[1];
  expect(cell("Net Profit %").className).toBe("n pos");
  expect(cell("Max. system % drawdown").className).toBe("n neg");
  expect(cell("Exposure %").className).toBe("n");
  expect(cell("Recovery Factor").className).toBe("n"); // "—"
  expect([...host.querySelectorAll("[data-lab-report-stats] caption")].map((c) => c.textContent)).toEqual(["Returns", "Trades", "Winners", "Losers", "Risk & ratios"]);
});
