import { XLSX_TYPE } from "../src/features/lab/xlsx";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { readFileSync } from "node:fs";
import { act } from "react";
import { RunResults, RunTables, drawdown, kpiItems, tradeTable } from "../src/features/lab/RunResults";
import { PROVISIONAL_NOTE } from "../src/features/lab/labels";
import { mount, runFixture, run7Fixture, money7Fixture, unmount } from "./fixtures/lab";

beforeEach(() => {
  window.VaultReport = { monthly: vi.fn(() => ({ months: { "2024-01": 0.02 }, annual: { "2024": 0.05 }, years: ["2024"], averages: [], annualAverage: 0.05 })),
    csv: vi.fn(() => "a,b") };
});
afterEach(async () => { await unmount(); delete (window as { VaultRuntime?: Window["VaultRuntime"] }).VaultRuntime; document.documentElement.removeAttribute("data-theme"); delete window.VaultReport; delete window.LightweightCharts; vi.unstubAllGlobals(); });

it("label verbatim, own start copy, KPIs and costs", async () => {
  const host = await mount(<RunResults result={runFixture} market="in" />);
  expect(host.querySelector("section.box[data-lab-result]")).not.toBeNull();
  expect(host.querySelector("[data-server-label]")?.textContent).toBe(runFixture.label);
  expect(host.textContent).toContain("Results start on 2024-01-08, not the requested start");
  expect(host.textContent).not.toContain("stale");
  expect(kpiItems(runFixture.stats).map((k) => [k.label, k.value])).toEqual([
    ["Net Profit %", "5.0%"], ["CAR", "31.0%"], ["MaxSysDD", "-1.0%"], ["CAR/MDD", "—"], ["# trades", "2"], ["# winners", "1 (50.0%)"], ["# losers", "1 (50.0%)"],
    ["Avg. profit/loss %", "0.02%"], ["Sharpe", "1.42"], ["Buy & hold CAGR", "40.0%"]]);
  expect(kpiItems({ ...runFixture.stats, mar: 31.6, trades: 10, win_rate: 0.3 }).filter((k) => ["CAR/MDD", "# winners", "# losers"].includes(k.label)).map((k) => k.value)).toEqual(["31.60", "3 (30.0%)", "7 (70.0%)"]);
  expect(host.textContent).toContain("Exposure 60.0% · before costs: 5.1%");
  expect(host.textContent).toContain("Fees 5 bps + slippage 2 bps per side · taxes not modelled");
  expect(host.querySelector("[data-lab-provisional]")).toBeNull();
  expect(host.textContent).not.toMatch(/\bverified\b/i);
  expect(host.textContent).not.toMatch(/TradingView-matched/i);
});

it("no start disclosure when start is null; null stats show a dash", async () => {
  const host = await mount(<RunResults result={{ ...runFixture, start: null }} market="in" />);
  expect(host.textContent).not.toContain("Results start on");
  expect(kpiItems({ ...runFixture.stats, sharpe: null }).find((k) => k.label === "Sharpe")!.value).toBe("—");
});

it("tradesheet: AmiBroker columns, oldest first, symbol, side, fill, open trade and friendly reasons", () => {
  const { csv } = tradeTable(runFixture);
  expect(csv[0]).toEqual(["Symbol", "Trade", "Order", "Date", "Price", "Ex. date", "Ex. Price", "Cum. %", "% Profit", "# bars", "Profit/bar", "Exit reason"]);
  expect(csv[1]).toEqual(["NIFTY 50", "Long", "Next open", "2024-01-08", "101.00", "2024-01-15", "102.00", "1.0%", "0.99%", "2", "0.330%", "Exit rule 1: bars_held >= 2"]);
  expect(csv[2]).toEqual(["NIFTY 50", "Short", "Close", "2024-02-01", "105.00", "Open", "106.00", "0.0%", "-0.95%", "1", "-0.475%", "Still open at the end"]);
  expect(tradeTable({ trades: [{ ...runFixture.trades[0], entry_kind: "limit" }] }).csv[1][2]).toBe("Limit");
  expect(tradeTable({ trades: [{ ...runFixture.trades[0], entry_kind: "weird" }] }).csv[1][2]).toBe("weird");
});

it("pages the table 100 rows at a time, oldest first, with a newest-first toggle", async () => {
  const many = { ...runFixture, trades: Array.from({ length: 205 }, (_, i) => ({ ...runFixture.trades[0], entry: `D${String(i).padStart(3, "0")}` })) };
  const host = await mount(<RunTables result={many} />);
  const firstDate = () => host.querySelector("table.tl tbody tr")!.children[3].textContent;
  const click = (text: string) => act(async () => [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === text)!.click());
  expect(host.querySelectorAll("table.tl tbody tr")).toHaveLength(100);
  expect(host.textContent).toContain("205 trades");
  expect(host.querySelector("[data-lab-pager]")!.textContent).toContain("1–100 of 205");
  expect(firstDate()).toBe("D000");
  await click("Next ›"); await click("Next ›");
  expect(host.querySelectorAll("table.tl tbody tr")).toHaveLength(5);
  expect(firstDate()).toBe("D200");
  await click("Newest first");
  expect(firstDate()).toBe("D204");
  expect(host.querySelector("[data-lab-pager]")!.textContent).toContain("1–100 of 205");
});

it("no trades shows the fixed copy", async () => {
  const host = await mount(<RunTables result={{ ...runFixture, trades: [] }} />);
  expect(host.textContent).toContain("No trades. The entry conditions never lined up on this instrument.");
  expect(host.querySelector("table.tl")).toBeNull();
});

it("Excel download: a Tradesheet with every trade, and a Rules sheet when the test behind it is known", async () => {
  const blobs: Blob[] = [], names: string[] = [];
  Object.assign(URL, { createObjectURL: vi.fn((b: Blob) => { blobs.push(b); return "blob:x"; }), revokeObjectURL: vi.fn() }); // jsdom has neither
  const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) { names.push(this.download); });
  const many = { ...runFixture, trades: Array.from({ length: 205 }, () => runFixture.trades[0]) };
  const host = await mount(<RunTables result={many} />);
  await act(async () => [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Download Excel")!.click());
  expect(names).toEqual([`lab-NIFTY 50-${runFixture.from}-${runFixture.to}.xlsx`]);
  expect(blobs[0].type).toBe(XLSX_TYPE);
  const text = new TextDecoder().decode(new Uint8Array(await blobs[0].arrayBuffer()));
  expect(text).toContain("Tradesheet");
  expect(text).not.toContain("Rules"); // no test context: trades only
  click.mockRestore();
});

it("monthly table from VaultReport; table and CSV button absent without it", async () => {
  let host = await mount(<RunTables result={runFixture} />);
  expect((window.VaultReport!.monthly as ReturnType<typeof vi.fn>).mock.calls[0][0].equity[0]).toBe(1);
  const cells = [...host.querySelectorAll(".lab-monthly tbody td")].map((td) => td.textContent);
  expect(cells[0]).toBe("2024");
  expect(cells[1]).toBe("2.0%");
  expect(cells[13]).toBe("5.0%");
  expect(host.querySelector(".lab-monthly caption")!.textContent).toBe("Profit Table");
  expect(host.querySelector(".lab-monthly thead th:last-child")!.textContent).toBe("Yr%");
  await unmount();
  delete window.VaultReport;
  host = await mount(<RunTables result={runFixture} />);
  expect(host.querySelector(".lab-monthly")).toBeNull();
  expect(host.querySelector(".lab-monthly")).toBeNull(); // the Excel download needs no VaultReport
});

it("monthly normalizes equity to start at 1", async () => {
  await mount(<RunTables result={{ ...runFixture, equity: runFixture.equity.map((v) => v * 2) }} />);
  expect((window.VaultReport!.monthly as ReturnType<typeof vi.fn>).mock.calls[0][0].equity).toEqual(runFixture.equity);
});

it("equity chart keeps its log scale and full data and is removed on unmount", async () => {
  const removed: number[] = [], options: Record<string, unknown>[] = [], series: { kind: string; data: unknown[] }[] = [];
  window.LightweightCharts = { createChart: (_el, o) => {
    const id = options.push(o);
    const add = (kind: string) => () => { const s = { kind, data: [] as unknown[] }; series.push(s);
      return { setData: (d: unknown[]) => { s.data = d; }, applyOptions() {} }; };
    return { addCandlestickSeries: add("candle"), addLineSeries: add("line"), addAreaSeries: add("area"),
      timeScale: () => ({ fitContent() {}, setVisibleRange() {} }), applyOptions() {}, resize() {}, remove: () => removed.push(id) };
  } };
  await mount(<RunResults result={runFixture} market="in" />);
  expect(options).toHaveLength(1);
  expect((options[0].rightPriceScale as { mode: number }).mode).toBe(1);
  expect(series.map((s) => s.kind)).toEqual(["area", "line"]);
  expect(series[0].data[2]).toEqual({ time: runFixture.dates[2], value: 1.02 });
  expect(series[1].data[1]).toEqual({ time: runFixture.dates[1], value: 1.01 });
  expect(drawdown(runFixture.equity)[3]).toBeCloseTo(1.01 / 1.02 - 1);
  await unmount();
  expect(removed).toEqual([1]);
});

it("provisional strip on gl and drawdown math", async () => {
  const gl = { ...runFixture, provisional: true as const, instrument: { ...runFixture.instrument, label: "Provisional · TradingView export HSI:HSI, daily" } };
  const host = await mount(<RunResults result={gl} market="gl" />);
  expect(host.querySelector("[data-lab-provisional]")?.textContent).toContain(PROVISIONAL_NOTE);
  expect(host.querySelector("[data-lab-provisional]")?.textContent).toContain("HSI:HSI");
  expect(drawdown([1, 2, 1, 3]).map((v) => Number(v.toFixed(2)))).toEqual([0, 0, -0.5, 0]);
});

it("monthly table and tradesheet live in RunTables, outside the backtest summary box", async () => {
  const summary = await mount(<RunResults result={runFixture} market="in" />);
  expect(summary.querySelector("[data-lab-result] .lab-monthly, [data-lab-result] .lab-trades, [data-lab-result] table")).toBeNull();
  expect([...summary.querySelectorAll("button")].some((b) => b.textContent === "Download Excel")).toBe(false);
  await unmount();
  const host = await mount(<RunTables result={runFixture} />);
  const box = host.querySelector("section[data-lab-result-tables]")!;
  expect(box.querySelector("[data-lab-result]")).toBeNull();
  expect(box.querySelector(".lab-monthly")).not.toBeNull();
  expect(box.querySelector(".lab-trades table.tl")).not.toBeNull();
  expect([...box.querySelectorAll("button")].some((b) => b.textContent === "Download Excel")).toBe(true);
});

it("library rail is sticky below the topbar, with a static bounded stacked layout", () => {
  const css = readFileSync("src/features/lab/library.css", "utf8");
  expect(css).toMatch(/\.lab-library \{[^}]*position: sticky;[^}]*top: calc\(58px \+ env\(safe-area-inset-top[^}]*max-height: calc\(100vh[^}]*overflow: auto;/);
  expect(css).toMatch(/@media \(max-width: 900px\) \{ \.lab-library \{ position: static; max-height: 60vh;/);
});


it("shows execution notes, risk events and adaptations verbatim", async () => {
  const host = await mount(<RunResults result={run7Fixture} market="in" />);
  expect(host.querySelector("[data-lab-execution-notes]")!.textContent).toContain(run7Fixture.execution_model!.notes[0]);
  expect(host.querySelector("[data-lab-events]")!.textContent).toContain(run7Fixture.execution_model!.events![0]);
  expect(host.querySelector("[data-lab-adaptations]")!.textContent).toBe(run7Fixture.adaptations![0]);
});
it("shows declared money costs without claiming taxes were omitted", async () => {
  const host = await mount(<RunResults result={money7Fixture} market="us" />);
  expect(host.querySelector("[data-lab-money]")!.textContent).toContain("Money · USD 100,000");
  expect(host.textContent).toContain("Final equity");
  expect(host.textContent).toContain("Costs paid");
  expect(host.textContent).toContain(money7Fixture.money!.basis);
  expect(host.textContent).not.toContain("taxes not modelled");
  expect(host.textContent).not.toContain("bps + slippage");
});
it("trade table adds only populated columns and never misjoins the ledger", () => {
  expect(tradeTable(runFixture).header).toEqual(["Symbol", "Trade", "Order", "Date", "Price", "Ex. date", "Ex. Price", "Cum. %", "% Profit", "# bars", "Profit/bar", "Exit reason"]);
  // MAE/MFE columns appear only when the engine sends excursions
  const excursions = { ...runFixture, trades: runFixture.trades.map((t) => ({ ...t, mae: -0.01, mfe: 0.02 })) };
  expect(tradeTable(excursions).header).toEqual(expect.arrayContaining(["MAE", "MFE"]));
  expect(tradeTable(run7Fixture).header).toContain("Scale in");
  expect(tradeTable(run7Fixture).header).toContain("Scale out");
  const money = tradeTable(money7Fixture);
  // AmiBroker order for the money columns
  expect(money.header.slice(7, 12)).toEqual(["Cum. Profit", "Profit", "% Profit", "Shares", "Position value"]);
  expect(money.header).toContain("Costs");
  expect(money.csv.at(-1)![1]).toBe("Short");
  expect(tradeTable({ ...money7Fixture, ledger: money7Fixture.ledger!.slice(1) }).header).not.toContain("Shares");
});
it("Cum. % compounds in trade order (open trade included); Profit/bar uses AmiBroker bars (entry and exit bar)", () => {
  const base = runFixture.trades[0];
  const trades = [{ ...base, entry: "2020-01-02", ret: 0.1, bars: 4, open: false }, { ...base, entry: "2020-02-03", ret: -0.05, bars: 0, open: false }, { ...base, entry: "2020-03-02", ret: 0.02, bars: 1, open: true }];
  const t = tradeTable({ trades }), col = (h: string) => t.header.indexOf(h);
  // rows are oldest first
  expect(t.csv.slice(1).map((r) => r[col("Cum. %")])).toEqual(["10.0%", "4.5%", "6.6%"]);
  expect(t.csv.slice(1).map((r) => r[col("Profit/bar")])).toEqual(["2.000%", "-5.000%", "1.000%"]);
  // money mode: summed ledger P&L in money, and money per bar
  const ledger = trades.map((x, i) => ({ entry: x.entry, qty: 1, notional: 1, costs: 0, pnl: [1000, -500, 200][i] }));
  const m = tradeTable({ trades, ledger, money: { capital: 100_000 } });
  expect(m.csv.slice(1).map((r) => r[m.header.indexOf("Cum. Profit")])).toEqual(["1,000.00", "500.00", "700.00"]);
  expect(m.csv.slice(1).map((r) => r[m.header.indexOf("Profit/bar")])).toEqual(["200.00", "-500.00", "100.00"]);
  expect(m.cls[1][m.header.indexOf("Profit")]).toBe("n neg");
});
it("preserves the money ledger in its own table when trade identities differ", async () => {
  const result = { ...money7Fixture, ledger: money7Fixture.ledger!.map((row) => ({ ...row, entry: "2020-01-01" })) };
  const host = await mount(<RunTables result={result} />);
  expect(host.querySelector("table[data-lab-ledger]")).not.toBeNull();
  expect(host.querySelector("table[data-lab-trades]")).not.toBeNull();
});
it("shows a nondefault allocation even on a single-leg trade and keeps CSV plain", () => {
  const table = tradeTable({ trades: [{ ...runFixture.trades[0], weight: 0.5 }] });
  expect(table.csv[1][table.header.indexOf("Allocation")]).toBe("50.0%");
  expect(table.csv.flat().every((cell) => typeof cell === "string")).toBe(true);
});

it("calls the exact shared area renderer with full drawdown dates, percentages and worst marker", async () => {
  const renderAreaChart = vi.fn((el: HTMLElement) => { el.innerHTML = '<svg class="shared-test-chart"></svg>'; });
  window.VaultRuntime = { renderAreaChart } as unknown as Window["VaultRuntime"];
  const charts: Record<string, unknown>[] = [];
  window.LightweightCharts = { createChart: (_el, options) => {
    charts.push(options); const add = () => ({setData() {}, applyOptions() {}});
    return {addCandlestickSeries:add,addLineSeries:add,addAreaSeries:add,timeScale:()=>({fitContent() {},setVisibleRange() {}}),applyOptions() {},resize() {},remove() {}};
  } };
  const host = await mount(<RunResults result={runFixture} market="in" />);
  expect(host.querySelector('[data-lab-chart="equity"] h3')?.textContent).toBe("Portfolio Equity");
  expect(host.querySelector('[data-lab-chart="drawdown"] h3')?.textContent).toBe("Underwater Equity");
  expect(charts).toHaveLength(1); expect(charts[0].height).toBe(300);
  expect((charts[0].timeScale as {minBarSpacing:number}).minBarSpacing).toBe(0.01);
  const [el, dates, values, options] = renderAreaChart.mock.calls[0] as unknown as [HTMLElement,string[],number[],{fmt:(n:number)=>string;axFmt:(n:number)=>string;markMin:boolean;h:number;color:string}];
  expect(el).toBe(host.querySelector('[data-lab-drawdown]'));
  expect(dates).toEqual(runFixture.dates); expect(values).toEqual(drawdown(runFixture.equity));
  expect(options).toMatchObject({markMin:true,h:300,color:"#c43c3c"});
  expect(options.fmt(-0.1234)).toBe("-12.34%"); expect(options.axFmt(-0.1234)).toBe("-12%");
  expect(host.querySelector('.shared-test-chart')).not.toBeNull();
  expect(readFileSync("src/compat/runtime.js","utf8")).toMatch(/renderAreaChart:\s*bkArea/);
});
it("redraws the same shared renderer for resizing and theme changes, then cleans up", async () => {
  const observers: {callback:()=>void; disconnect:ReturnType<typeof vi.fn>}[] = [];
  vi.stubGlobal("ResizeObserver",class {disconnect=vi.fn(); constructor(public callback:()=>void){observers.push(this);} observe(){} });
  const renderAreaChart = vi.fn((el:HTMLElement) => {el.innerHTML='<svg></svg>';});
  window.VaultRuntime = {renderAreaChart} as unknown as Window["VaultRuntime"];
  const host=await mount(<RunResults result={runFixture} market="in" />), el=host.querySelector('[data-lab-drawdown]')!;
  const count=renderAreaChart.mock.calls.length;
  await act(async()=>{observers.at(-1)!.callback();});
  expect(renderAreaChart.mock.calls.length).toBeGreaterThan(count);
  await act(async()=>{document.documentElement.setAttribute("data-theme","midnight");await new Promise((r)=>setTimeout(r,0));});
  expect(renderAreaChart.mock.calls.length).toBeGreaterThan(count+1);
  await unmount(); expect(el.childNodes).toHaveLength(0); expect(observers.every((o)=>o.disconnect.mock.calls.length>0)).toBe(true);
});

it("drawdown includes a first-day loss against the declared starting equity", () => {
  expect(drawdown([0.9, 0.8]).map((v) => Number(v.toFixed(6)))).toEqual([-0.1, -0.2]);
  expect(drawdown([1, 1.2, 1.08]).map((v) => Number(v.toFixed(6)))).toEqual([0, 0, -0.1]);
});


it("guards empty and one-session drawdown without fabricating bars; flat periods stay flat", async () => {
  const renderAreaChart=vi.fn();window.VaultRuntime={renderAreaChart} as unknown as Window["VaultRuntime"];
  let host=await mount(<RunResults result={{...runFixture,dates:[],equity:[],bh:[]}} market="in" />);
  expect(renderAreaChart).not.toHaveBeenCalled();expect(host.querySelector('[data-lab-drawdown]')?.textContent).toContain("No drawdown data");
  await unmount();
  host=await mount(<RunResults result={{...runFixture,dates:["2024-01-01"],equity:[0.9],bh:[1]}} market="in" />);
  expect(renderAreaChart).not.toHaveBeenCalled();expect(host.querySelector('[data-lab-drawdown]')?.textContent).toContain("-10.00% · 2024-01-01");
  await unmount();
  await mount(<RunResults result={{...runFixture,equity:runFixture.equity.map(()=>1)}} market="in" />);
  expect(renderAreaChart.mock.calls[0][1]).toEqual(runFixture.dates);expect(renderAreaChart.mock.calls[0][2]).toEqual(runFixture.dates.map(()=>0));
});
