import { act } from "react";
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { autoPivotPeriod, periodLevelLines } from "../src/features/lab/bars";
import { DrawingLayer, FIB_LEVELS, dateSpan, priceChange, type Drawing } from "../src/features/lab/DrawingLayer";
import type { LabChartApi, LabSeriesApi } from "../src/features/lab/globals";
import { initialLabState, labReducer } from "../src/features/lab/lab-state";
import type { Indicator } from "../src/features/lab/types";
import { mount, unmount } from "./fixtures/lab";

afterEach(unmount);
// jsdom lays nothing out: a 500 px wide drawing layer, one bar every 10 px, price p at y = p / 10
beforeAll(() => { Object.defineProperty(SVGElement.prototype, "clientWidth", { configurable: true, get: () => 500 }); });
const times = ["2024-01-01", "2024-01-02", "2024-01-03", "2024-01-04", "2024-01-05", "2024-01-08"];
const chart = { priceScale: () => ({ width: () => 50 }), timeScale: () => ({ subscribeVisibleLogicalRangeChange: vi.fn(), unsubscribeVisibleLogicalRangeChange: vi.fn(),
  logicalToCoordinate: (l: number) => l * 10, coordinateToLogical: (x: number) => x / 10 }) } as unknown as LabChartApi;
const series = { priceToCoordinate: (p: number) => p / 10, coordinateToPrice: (y: number) => y * 10 } as unknown as LabSeriesApi;
const draw = (drawings: Drawing[]) => mount(<DrawingLayer chart={chart} series={series} times={times} drawings={drawings} tool={null} color="#2962ff" height={300} onChange={vi.fn()} onDone={vi.fn()} />);

it("Ray runs past its second point to the chart's edge, Extended line both ways; a plain trend line stops at its points", async () => {
  const host = await draw([{ kind: "t", a: [times[1], 200], b: [times[2], 300] }, { kind: "t", a: [times[1], 200], b: [times[2], 300], ext: "right" },
    { kind: "t", a: [times[1], 200], b: [times[2], 300], ext: "both" }]);
  await act(async () => { host.querySelector("[data-lab-draw-layer]")!.parentElement!.dispatchEvent(new Event("pointermove")); }); // laid out: redrawn at its width
  const lines = [...host.querySelectorAll('[data-lab-drawing="t"]')].map((g) => g.querySelector("line")!);
  const at = (l: Element) => ["x1", "y1", "x2", "y2"].map((k) => Number(l.getAttribute(k)));
  expect(at(lines[0])).toEqual([10, 20, 20, 30]);
  expect(at(lines[1])).toEqual([10, 20, 450, 460]); // the plot ends at the price axis (500 − 50)
  expect(at(lines[2])).toEqual([0, 10, 450, 460]);
});

it("a dropped drawing hides its end handles until hovered or selected", async () => {
  const host = await draw([{ kind: "t", a: [times[1], 200], b: [times[2], 300] }]);
  expect([...host.querySelectorAll(".lab-draw-end")].every((c) => c.classList.contains("idle"))).toBe(true);
});

it("Fib retracement: TradingView's levels from the second point (0) to the first (1), labelled with their prices", async () => {
  const host = await draw([{ kind: "fib", a: [times[1], 1000], b: [times[3], 2000] }]);
  const labels = [...host.querySelectorAll("[data-lab-fib] text")].map((t) => t.textContent);
  expect(labels).toHaveLength(FIB_LEVELS.length);
  expect(labels[0]).toBe("0 (2,000.00)");
  expect(labels[4]).toBe("0.618 (1,382.00)");
  expect(labels[6]).toBe("1 (1,000.00)");
});

it("Price range, Date range and Date and price range read out the change, the bars and the days", async () => {
  expect(priceChange(100, 120.5)).toBe("20.50 (20.50%)");
  expect(priceChange(200, 150)).toBe("−50.00 (−25.00%)");
  expect(dateSpan(4, "2024-01-01", "2024-01-05")).toBe("4 bars, 4d");
  const host = await draw([{ kind: "pr", a: [times[0], 200], b: [times[2], 150] }, { kind: "dr", a: [times[0], 200], b: [times[5], 150] },
    { kind: "dpr", a: [times[0], 100], b: [times[5], 110] }]);
  const read = [...host.querySelectorAll("[data-lab-measure]")].map((g) => [...g.querySelectorAll("text")].map((t) => t.textContent));
  expect(read).toEqual([["−50.00 (−25.00%)"], ["5 bars, 7d"], ["10.00 (10.00%)", "5 bars, 7d"]]);
});

it("Pivot Points Standard: one flat labelled line per level per period, the last 15 periods only", () => {
  const px = { d: ["a", "b", "c", "d", "e"], o: [], h: [], l: [], c: [] };
  const lines = periodLevelLines("pivots_std", ["p", "r1"], { p: [10, 10, 12, 12, 12], r1: [11, 11, 13, 13, 13] }, px.d, px);
  expect(lines).toEqual([{ from: 0, to: 2, price: 10, text: "P (10.00)" }, { from: 0, to: 2, price: 11, text: "R1 (11.00)" },
    { from: 2, to: 4, price: 12, text: "P (12.00)" }, { from: 2, to: 4, price: 13, text: "R1 (13.00)" }]);
  const many = Array.from({ length: 40 }, (_, i) => i);
  expect(periodLevelLines("pivots_std", ["p"], { p: many }, many.map(String), { ...px, d: many.map(String) }).length).toBe(15);
  expect(periodLevelLines("ema", ["ema"], { ema: [1, 2] }, ["a", "b"], px)).toEqual([]);
});

it("pivots start on TradingView's Auto timeframe (monthly on 1D, yearly above) and follow the chart's timeframe", () => {
  const pivots = { id: "pivots_std", placement: "price", params: [{ key: "type", default: 0 }, { key: "period", default: 1 }] } as unknown as Indicator;
  const s0 = { ...initialLabState({ region: "in", tk: "NIFTY 50", tf: "D", range: "3Y" } as never), ticker: "NIFTY 50", tf: "D" };
  const s1 = labReducer(s0, { type: "chip-add", indicator: pivots });
  expect(s1.chips[0].params).toEqual({ type: 0, period: autoPivotPeriod("D") });
  expect(labReducer(s1, { type: "tf", tf: "W" }).chips[0].params.period).toBe(4);
  // a period the user chose stays
  const own = labReducer(s1, { type: "chip-set", key: s1.chips[0].key, params: { type: 0, period: 1 } });
  expect(labReducer(own, { type: "tf", tf: "W" }).chips[0].params.period).toBe(1);
});

it("the current pivot period runs past the latest bar as long as the period before it, as TradingView draws it to the period's end", () => {
  const d = ["a", "b", "c", "d", "e", "f"], px = { d, o: [], h: [], l: [], c: [] };
  // periods start at bars 0 and 4: the last one (2 bars so far) runs on to bar 4 + 4 = 8
  expect(periodLevelLines("pivots_std", ["p"], { p: [1, 1, 1, 1, 2, 2] }, d, px).map((l) => [l.from, l.to])).toEqual([[0, 4], [4, 8]]);
});

it("TradingView's number, date and volume look", async () => {
  const { tvPrice, tvDay, tvVolume } = await import("../src/features/lab/Workbench");
  expect(tvPrice(25281.849)).toBe("25,281.85");
  expect(tvDay("2026-11-08")).toBe("Sun 08 Nov '26");
  expect(tvDay({ year: 2026, month: 9, day: 21 })).toBe("Mon 21 Sep '26");
  expect([tvVolume(192_580_000), tvVolume(3210), tvVolume(1.05e9), tvVolume(12)]).toEqual(["192.58M", "3.21K", "1.05B", "12"]);
});

it("on a weekly chart the levels come from the days: a period starts on the week bar holding its first day (TradingView)", () => {
  // week bars dated their last day; the yearly levels change on Thu 2026-01-01, inside the week ending 2026-01-02
  const weeks = { d: ["2025-12-19", "2025-12-26", "2026-01-02", "2026-01-09"], o: [], h: [], l: [], c: [] };
  const days = ["2025-12-15", "2025-12-29", "2025-12-31", "2026-01-01", "2026-01-05"];
  const lines = periodLevelLines("pivots_std", ["p"], { p: [10, 10, 10, 20, 20] }, days, weeks);
  expect(lines.map((l) => [l.from, l.to, l.price])).toEqual([[0, 2, 10], [2, 4, 20]]);
});
