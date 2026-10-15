import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { readFileSync } from "node:fs";
import { act } from "react";
import { RunResults, RunTables, drawdown, kpiItems, tradeRows } from "../src/features/lab/RunResults";
import { PROVISIONAL_NOTE } from "../src/features/lab/labels";
import { mount, runFixture, unmount } from "./fixtures/lab";

beforeEach(() => {
  window.VaultReport = { monthly: vi.fn(() => ({ months: { "2024-01": 0.02 }, annual: { "2024": 0.05 }, years: ["2024"], averages: [], annualAverage: 0.05 })),
    csv: vi.fn(() => "a,b") };
});
afterEach(async () => { await unmount(); delete window.VaultReport; delete window.LightweightCharts; vi.unstubAllGlobals(); });

it("label verbatim, own start copy, KPIs and costs", async () => {
  const host = await mount(<RunResults result={runFixture} market="in" />);
  expect(host.querySelector("section.box[data-lab-result]")).not.toBeNull();
  expect(host.querySelector("[data-server-label]")?.textContent).toBe(runFixture.label);
  expect(host.textContent).toContain("Results start on 2024-01-08, not the requested start");
  expect(host.textContent).not.toContain("stale");
  expect(kpiItems(runFixture.stats).map((k) => [k.label, k.value])).toEqual([
    ["Net return", "5.0%"], ["CAGR", "31.0%"], ["Max drawdown", "-1.0%"], ["Sharpe", "1.42"],
    ["Trades", "2"], ["Win rate", "50.0%"], ["Avg trade", "0.02%"], ["Buy & hold CAGR", "40.0%"]]);
  expect(host.textContent).toContain("Exposure 60.0% · before costs: 5.1%");
  expect(host.textContent).toContain("Fees 5 bps + slippage 2 bps per side · taxes not modelled");
  expect(host.querySelector("[data-lab-provisional]")).toBeNull();
  expect(host.textContent).not.toMatch(/\bverified\b/i);
  expect(host.textContent).not.toMatch(/TradingView-matched/i);
});

it("no start disclosure when start is null; null stats show a dash", async () => {
  const host = await mount(<RunResults result={{ ...runFixture, start: null }} market="in" />);
  expect(host.textContent).not.toContain("Results start on");
  expect(kpiItems({ ...runFixture.stats, sharpe: null })[3].value).toBe("—");
});

it("tradesheet: newest first, side, fill, open trade and friendly reasons", () => {
  const rows = tradeRows(runFixture.trades);
  expect(rows[0]).toEqual(["2024-02-01", "Short", "Close", "105.00", "Open", "106.00", "-0.95%", "1", "Still open at the end"]);
  expect(rows[1]).toEqual(["2024-01-08", "Long", "Next open", "101.00", "2024-01-15", "102.00", "0.99%", "2", "Exit rule 1: bars_held >= 2"]);
  expect(tradeRows([{ ...runFixture.trades[0], entry_kind: "limit" }])[0][2]).toBe("Limit");
  expect(tradeRows([{ ...runFixture.trades[0], entry_kind: "weird" }])[0][2]).toBe("weird");
});

it("caps the table at 200 rows and points to the CSV", async () => {
  const many = { ...runFixture, trades: Array.from({ length: 205 }, () => runFixture.trades[0]) };
  const host = await mount(<RunTables result={many} />);
  expect(host.querySelectorAll("table.tl tbody tr")).toHaveLength(200);
  expect(host.textContent).toContain("205 trades · newest first");
  expect(host.textContent).toContain("5 older trades are in the CSV.");
});

it("no trades shows the fixed copy", async () => {
  const host = await mount(<RunTables result={{ ...runFixture, trades: [] }} />);
  expect(host.textContent).toContain("No trades. The entry conditions never lined up on this instrument.");
  expect(host.querySelector("table.tl")).toBeNull();
});

it("CSV download uses VaultReport.csv with every trade and the file name", async () => {
  const createObjectURL = vi.fn((_: Blob) => "blob:x"), revokeObjectURL = vi.fn();
  Object.assign(URL, { createObjectURL, revokeObjectURL }); // jsdom has neither
  const names: string[] = [];
  const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) { names.push(this.download); });
  const many = { ...runFixture, trades: Array.from({ length: 205 }, () => runFixture.trades[0]) };
  const host = await mount(<RunTables result={many} />);
  await act(async () => [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Download CSV")!.click());
  const rows = (window.VaultReport!.csv as ReturnType<typeof vi.fn>).mock.calls[0][0] as string[][];
  expect(rows[0][0]).toBe("Entry date");
  expect(rows).toHaveLength(206);
  expect(createObjectURL.mock.calls[0][0].type).toBe("text/csv;charset=utf-8");
  expect(names).toEqual([`lab-NIFTY 50-${runFixture.from}-${runFixture.to}.csv`]);
  expect(revokeObjectURL).toHaveBeenCalledWith("blob:x");
  click.mockRestore();
});

it("monthly table from VaultReport; table and CSV button absent without it", async () => {
  let host = await mount(<RunTables result={runFixture} />);
  expect((window.VaultReport!.monthly as ReturnType<typeof vi.fn>).mock.calls[0][0].equity[0]).toBe(1);
  const cells = [...host.querySelectorAll(".lab-monthly tbody td")].map((td) => td.textContent);
  expect(cells[0]).toBe("2024");
  expect(cells[1]).toBe("2.0%");
  expect(cells[13]).toBe("5.0%");
  await unmount();
  delete window.VaultReport;
  host = await mount(<RunTables result={runFixture} />);
  expect(host.querySelector(".lab-monthly")).toBeNull();
  expect([...host.querySelectorAll("button")].some((b) => b.textContent === "Download CSV")).toBe(false);
});

it("monthly normalizes equity to start at 1", async () => {
  await mount(<RunTables result={{ ...runFixture, equity: runFixture.equity.map((v) => v * 2) }} />);
  expect((window.VaultReport!.monthly as ReturnType<typeof vi.fn>).mock.calls[0][0].equity).toEqual(runFixture.equity);
});

it("equity chart on a log scale and drawdown chart; both removed on unmount", async () => {
  const removed: number[] = [], options: Record<string, unknown>[] = [], series: { kind: string; data: unknown[] }[] = [];
  window.LightweightCharts = { createChart: (_el, o) => {
    const id = options.push(o);
    const add = (kind: string) => () => { const s = { kind, data: [] as unknown[] }; series.push(s);
      return { setData: (d: unknown[]) => { s.data = d; }, applyOptions() {} }; };
    return { addCandlestickSeries: add("candle"), addLineSeries: add("line"), addAreaSeries: add("area"),
      timeScale: () => ({ fitContent() {}, setVisibleRange() {} }), applyOptions() {}, resize() {}, remove: () => removed.push(id) };
  } };
  await mount(<RunResults result={runFixture} market="in" />);
  expect(options).toHaveLength(2);
  expect((options[0].rightPriceScale as { mode: number }).mode).toBe(1);
  expect(series.map((s) => s.kind)).toEqual(["area", "line", "area"]);
  expect(series[0].data[2]).toEqual({ time: runFixture.dates[2], value: 1.02 });
  expect(series[1].data[1]).toEqual({ time: runFixture.dates[1], value: 1.01 });
  expect((series[2].data[3] as { value: number }).value).toBeCloseTo(1.01 / 1.02 - 1);
  await unmount();
  expect(removed.sort()).toEqual([1, 2]);
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
  expect([...summary.querySelectorAll("button")].some((b) => b.textContent === "Download CSV")).toBe(false);
  await unmount();
  const host = await mount(<RunTables result={runFixture} />);
  const box = host.querySelector("section[data-lab-result-tables]")!;
  expect(box.querySelector("[data-lab-result]")).toBeNull();
  expect(box.querySelector(".lab-monthly")).not.toBeNull();
  expect(box.querySelector(".lab-trades table.tl")).not.toBeNull();
  expect([...box.querySelectorAll("button")].some((b) => b.textContent === "Download CSV")).toBe(true);
});

it("library rail is sticky below the topbar, with a static bounded stacked layout", () => {
  const css = readFileSync("src/features/lab/library.css", "utf8");
  expect(css).toMatch(/\.lab-library \{[^}]*position: sticky;[^}]*top: calc\(58px \+ env\(safe-area-inset-top[^}]*max-height: calc\(100vh[^}]*overflow: auto;/);
  expect(css).toMatch(/@media \(max-width: 900px\) \{ \.lab-library \{ position: static; max-height: 60vh;/);
});
