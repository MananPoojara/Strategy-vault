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

it("tradesheet: the analyst's columns, oldest first, open trade; the Excel download adds the friendly exit reason", () => {
  const { csv, header } = tradeTable(runFixture);
  expect(header).toEqual(["Symbol", "Trade type", "Entry date", "Entry price", "Exit date", "Exit price", "% Change", "Bars held", "Days", "Net profit", "DD %"]);
  expect(csv[0]).toEqual([...header, "Exit reason"]);
  expect(csv[1]).toEqual(["NIFTY 50", "Long", "08-01-2024", "101.00", "15-01-2024", "102.00", "0.99%", "2", "7", "0.99%", "0.0%", "Exit rule 1: bars_held >= 2"]);
  expect(csv[2].slice(0, 8)).toEqual(["NIFTY 50", "Short", "01-02-2024", "105.00", "Open", "106.00", "-0.95%", "1"]);
  expect(csv[2].at(-1)).toBe("Still open at the end");
});

it("pages the table 100 rows at a time, oldest first, with a newest-first toggle", async () => {
  const many = { ...runFixture, trades: Array.from({ length: 205 }, (_, i) => ({ ...runFixture.trades[0], entry: `D${String(i).padStart(3, "0")}` })) };
  const host = await mount(<RunTables result={many} />);
  const firstDate = () => host.querySelector("table.tl tbody tr")!.children[2].textContent; // Entry date
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

it("no Simple Excel button: the tradesheet downloads only as the team's TS", async () => {
  const host = await mount(<RunTables result={runFixture} />);
  expect([...host.querySelectorAll("button")].some((b) => b.textContent === "Simple Excel")).toBe(false);
});

it("monthly table from VaultReport; table and CSV button absent without it", async () => {
  let host = await mount(<RunTables result={runFixture} />);
  expect((window.VaultReport!.monthly as ReturnType<typeof vi.fn>).mock.calls[0][0].equity[0]).toBe(1);
  const cells = [...host.querySelectorAll(".lab-monthly tbody td")].map((td) => td.textContent);
  expect(cells[0]).toBe("2024");
  expect(cells[1]).toBe("2.0%");
  expect(cells[14]).toBe("5.0%"); // Yr%, after the Total column
  expect(cells.slice(1, 13).filter((c) => c === "0.0%" || c === "—")).toEqual([]); // a month with no return is blank
  expect(host.querySelector(".lab-monthly caption")!.textContent).toBe("YOY-Monthly");
  expect(host.querySelector(".lab-monthly thead th:nth-last-child(2)")!.textContent).toBe("Total");
  expect(host.querySelector("[data-lab-pivot-total] td")!.textContent).toBe("Total");
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
  expect([...summary.querySelectorAll("button")].some((b) => b.textContent === "Simple Excel")).toBe(false);
  await unmount();
  const host = await mount(<RunTables result={runFixture} />);
  const box = host.querySelector("section[data-lab-result-tables]")!;
  expect(box.querySelector("[data-lab-result]")).toBeNull();
  expect(box.querySelector(".lab-monthly")).not.toBeNull();
  expect(box.querySelector(".lab-trades table.tl")).not.toBeNull();
  expect(box.querySelector("[data-lab-download-trades]")).toBeNull();
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
  expect(tradeTable(runFixture).header).toEqual(["Symbol", "Trade type", "Entry date", "Entry price", "Exit date", "Exit price", "% Change", "Bars held", "Days", "Net profit", "DD %"]);
  // MAE/MFE columns appear only when the engine sends excursions
  const excursions = { ...runFixture, trades: runFixture.trades.map((t) => ({ ...t, mae: -0.01, mfe: 0.02 })) };
  expect(tradeTable(excursions).header).toEqual(expect.arrayContaining(["MAE", "MFE"]));
  expect(tradeTable(run7Fixture).header).toContain("Scale in");
  expect(tradeTable(run7Fixture).header).toContain("Scale out");
  const money = tradeTable(money7Fixture);
  // money mode: cash net profit and cumulative profit, then shares, position value and costs
  expect(money.header).toEqual(expect.arrayContaining(["Net profit", "DD %", "Shares", "Position value", "Costs"]));
  expect(money.header.some((h) => h.startsWith("Cum."))).toBe(false); // no running-total column on screen
  expect(money.csv.at(-1)![1]).toBe("Short");
  expect(tradeTable({ ...money7Fixture, ledger: money7Fixture.ledger!.slice(1) }).header).not.toContain("Shares");
});
it("the running total compounds in trade order (open trade included); Net profit is on the starting equity; DD % is from the running peak; Days are calendar days", () => {
  const base = runFixture.trades[0];
  const trades = [{ ...base, entry: "2020-01-02", exit: "2020-01-09", ret: 0.1, bars: 4, open: false }, { ...base, entry: "2020-02-03", exit: "2020-02-03", ret: -0.05, bars: 0, open: false }, { ...base, entry: "2020-03-02", exit: "2020-03-31", ret: 0.02, bars: 1, open: true }];
  const t = tradeTable({ trades }), col = (h: string) => t.header.indexOf(h);
  // rows are oldest first
  expect(t.csv.slice(1).map((r) => r[col("Net profit")])).toEqual(["10.00%", "-5.50%", "2.09%"]); // 1.1 × −5 %, 1.045 × 2 %
  expect(t.csv.slice(1).map((r) => r[col("DD %")])).toEqual(["0.0%", "-5.0%", "-3.1%"]); // 1.045 / 1.1, 1.0659 / 1.1
  expect(t.csv.slice(1).map((r) => r[col("Days")])).toEqual(["7", "0", "29"]);
  // money mode: summed ledger P&L in money, and money per bar
  const ledger = trades.map((x, i) => ({ entry: x.entry, qty: 1, notional: 1, costs: 0, pnl: [1000, -500, 200][i] }));
  const m = tradeTable({ trades, ledger, money: { capital: 100_000 } });
  expect(m.csv.slice(1).map((r) => r[m.header.indexOf("Net profit")])).toEqual(["1,000.00", "-500.00", "200.00"]);
  expect(m.csv.slice(1).map((r) => r[m.header.indexOf("DD %")])).toEqual(["0.0%", "-0.5%", "-0.3%"]); // 100,500 / 101,000, 100,700 / 101,000
  expect(m.cls[1][m.header.indexOf("Net profit")]).toBe("n neg");
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
  expect(renderAreaChart).not.toHaveBeenCalled();expect(host.querySelector('[data-lab-drawdown]')?.textContent).toContain("-10.00% · 01-01-2024");
  await unmount();
  await mount(<RunResults result={{...runFixture,equity:runFixture.equity.map(()=>1)}} market="in" />);
  expect(renderAreaChart.mock.calls[0][1]).toEqual(runFixture.dates);expect(renderAreaChart.mock.calls[0][2]).toEqual(runFixture.dates.map(()=>0));
});

it("Download TS sits under the tradesheet: the format picked from the start date, the template's costs shown and editable", async () => {
  const { TsDownload } = await import("../src/features/lab/TsDownload");
  const ctx = { spec: { v: 1, entry: { all: [] }, exit: { any: [] } }, own: { v: 1, entry: { all: [] }, exit: { any: [] } }, catalogue: { indicators: [] } } as never;
  const host = await mount(<TsDownload result={{ ...runFixture, from: "2000-01-03" }} explain={ctx} />);
  expect(host.querySelector('[aria-label="Tradesheet format"] [aria-pressed="true"], [aria-label="Tradesheet format"] [aria-checked="true"]')!.textContent).toBe("From 2000");
  expect(["Impact cost %", "Rollover Long %", "Rollover Short %"].map((l) => host.querySelector<HTMLInputElement>(`input[aria-label="${l}"]`)!.value)).toEqual(["0.3", "1", "-0.5"]);
  // Download all exits: typed variants give one sheet each, with the count shown before running
  const many = host.querySelector("[data-lab-ts-many]")!;
  const type = async (label: string, v: string) => act(async () => { const i = many.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!;
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(i, v); i.dispatchEvent(new Event("input", { bubbles: true })); });
  await type("N candle", "3, 5, 7"); await type("ATR (period-initial,favour)", "5-1,1; 5-2,2");
  expect(many.textContent).toContain("5 sheets");
  const later = await mount(<TsDownload result={{ ...runFixture, from: "2010-01-04" }} explain={ctx} />);
  expect(later.querySelector('[aria-label="Tradesheet format"] [aria-pressed="true"], [aria-label="Tradesheet format"] [aria-checked="true"]')!.textContent).toBe("From 2008");
});
