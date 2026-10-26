import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act } from "react";
import { Workbench } from "../src/features/lab/Workbench";
import { drawItems, rebuild, slotAt, tradeMarks, type Slot } from "../src/features/lab/timeless";
import type { ChartColors } from "../src/features/lab/charts";
import type { Catalogue, Indicator, Trade } from "../src/features/lab/types";
import { catalogueFixture as cat, flush, mount, runFixture, seriesFixture, unmount } from "./fixtures/lab";

// Engine outputs below are the hand-checked cases of the engine's own unit tests (x_atlas.rs / x_timeless.rs).
const days = (n: number) => Array.from({ length: n }, (_, i) => `2024-01-${String(i + 1).padStart(2, "0")}`);
const res = (id: string, series: Record<string, (number | null)[]>) => ({ ...seriesFixture(id, Object.keys(series), series), dates: days(Object.values(series)[0].length) });
const edges = (slots: Slot[]) => slots.map((s) => [s.open, s.close]);

it("rebuilds Renko bricks from bricks / brick_high / brick_low, several per bar, dated by the bars that built them", () => {
  // closes 50 52 54 55 56 51 50 45 49 50, box 2 (TradingView's 2-box reversal)
  const r = rebuild("renko", res("renko", {
    bricks: [0, 1, 1, 0, 1, -1, -1, -2, 0, 1], brick_high: [50, 52, 54, 54, 56, 54, 52, 48, 48, 50], brick_low: [50, 50, 52, 52, 54, 52, 50, 46, 46, 48],
    direction: [0, 1, 1, 1, 1, -1, -1, -1, -1, 1],
  }));
  expect(edges(r.slots)).toEqual([[50, 52], [52, 54], [54, 56], [54, 52], [52, 50], [50, 48], [48, 46], [48, 50]]);
  expect(r.total).toBe(8);
  expect(r.slots.map((s) => s.up)).toEqual([true, true, true, false, false, false, false, true]);
  // the brick 54→56 was built over 2024-01-04..05; both bricks of bar 8 formed that day
  expect([r.slots[2].from, r.slots[2].to]).toEqual(["2024-01-04", "2024-01-05"]);
  expect(r.slots.slice(5, 7).map((s) => [s.from, s.at])).toEqual([["2024-01-08", "2024-01-08"], ["2024-01-08", "2024-01-08"]]);
  // the cap keeps the latest bricks
  expect(edges(rebuild("renko", res("renko", { bricks: [0, 1, 1, 0, 1, -1, -1, -2, 0, 1], brick_high: [50, 52, 54, 54, 56, 54, 52, 48, 48, 50],
    brick_low: [50, 50, 52, 52, 54, 52, 50, 46, 46, 48], direction: [0, 1, 1, 1, 1, -1, -1, -1, -1, 1] }), {}, 3).slots)).toEqual([[50, 48], [48, 46], [48, 50]]);
  // AmiBroker grid anchor: the seed down brick 54→52 is drawn on the anchor bar (bricks 0)
  const grid = rebuild("renko", res("renko", { bricks: [0, 0, 0, 0, 1], brick_high: [54, 54, 54, 54, 56], brick_low: [52, 52, 52, 52, 54], direction: [-1, -1, -1, -1, 1] }));
  expect(edges(grid.slots)).toEqual([[54, 52], [54, 56]]);
});

it("rebuilds one Line Break line per new_line bar", () => {
  // closes 100 105 110 115 98 112 116, 3 lines, AmiBroker rule
  const r = rebuild("line_break", res("line_break", {
    line_open: [100, 100, 105, 110, 110, 110, 110], line_close: [100, 105, 110, 115, 98, 98, 116], new_line: [0, 1, 1, 1, -1, 0, 1],
  }));
  expect(edges(r.slots)).toEqual([[100, 105], [105, 110], [110, 115], [110, 98], [110, 116]]);
  expect(r.slots.map((s) => s.up)).toEqual([true, true, true, false, true]);
  expect([r.slots[4].from, r.slots[4].to]).toEqual(["2024-01-06", "2024-01-07"]);
});

it("rebuilds Point & Figure columns: start on new_col, extent at the column's last bar, dates up to the day before the next column", () => {
  // closes 100.5 101.2 103.4 104.9 102 101.9 101 105 106 102 100 99, box 1, reversal 3
  const r = rebuild("pnf", res("pnf", {
    col_dir: [-1, -1, -1, 1, 1, 1, -1, 1, 1, -1, -1, -1], col_high: [101, 101, 101, 104, 104, 104, 103, 105, 106, 105, 105, 105],
    col_low: [101, 101, 101, 102, 102, 102, 101, 102, 102, 102, 100, 99], boxes: [1, 1, 1, 3, 3, 3, 3, 4, 5, 4, 6, 7],
    new_col: [0, 0, 0, 1, 0, 0, 1, 1, 0, 1, 0, 0], boxes_added: [-1, 0, 0, 3, 0, 0, -3, 4, 1, -4, -2, -1],
    rev_level: [104, 104, 104, 101, 101, 101, 104, 102, 103, 105, 103, 102],
  }), { reversal: 3 });
  expect(r.slots.map((s) => [s.up ? "X" : "O", s.low, s.high, s.boxes, s.box])).toEqual([
    ["O", 101, 101, 1, 1], ["X", 102, 104, 3, 1], ["O", 101, 103, 3, 1], ["X", 102, 106, 5, 1], ["O", 99, 105, 7, 1]]);
  // a column also holds its quiet days, so its dates and the trade markers on it agree
  expect(r.slots.map((s) => [s.from, s.to])).toEqual([["2024-01-01", "2024-01-03"], ["2024-01-04", "2024-01-06"], ["2024-01-07", "2024-01-07"],
    ["2024-01-08", "2024-01-09"], ["2024-01-10", "2024-01-12"]]);
  // a date inside a column maps to that column
  expect(["2023-12-31", "2024-01-03", "2024-01-05", "2024-01-09", "2024-01-31"].map((d) => slotAt(r.slots, d))).toEqual([-1, 0, 1, 3, 4]);
});

it("rebuilds Kagi lines with the thickness change at the shoulder or waist", () => {
  // closes 10 15 12 9 11 13 16 14, reversal 2 points
  const r = rebuild("kagi", res("kagi", {
    tip: [10, 15, 12, 9, 11, 13, 16, 14], direction: [0, 1, -1, -1, 1, 1, 1, -1], reversal: [0, 0, 1, 0, 1, 0, 0, 1],
    shoulder: [10, 10, 15, 15, 15, 15, 15, 16], waist: [10, 10, 10, 10, 9, 9, 9, 9], yang: [0, 1, 1, -1, -1, -1, 1, 1],
    yang_flip: [0, 0, 0, 0, 0, 0, 1, 0], yin_flip: [0, 0, 0, 1, 0, 0, 0, 0],
  }));
  expect(r.slots.map((s) => [s.open, s.close, s.thick0, s.split, s.thick1])).toEqual([
    [10, 15, true, undefined, true], [15, 9, true, 10, false], [9, 16, false, 15, true], [16, 14, true, undefined, true]]);
  expect(r.slots.map((s) => [s.from, s.to])).toEqual([["2024-01-02", "2024-01-02"], ["2024-01-03", "2024-01-04"], ["2024-01-05", "2024-01-07"], ["2024-01-08", "2024-01-08"]]);
});

it("keeps Heikin Ashi candles one per bar and places fills at real prices on the element holding the date", () => {
  const r = rebuild("heikin_ashi", res("heikin_ashi", { open: [null, 10.5, 11], high: [null, 12, 13], low: [null, 9, 10], close: [null, 11, 11.5] }));
  expect(r.slots.map((s) => [s.at, s.open, s.high, s.low, s.close])).toEqual([["2024-01-02", 10.5, 12, 9, 11], ["2024-01-03", 11, 13, 10, 11.5]]);
  const t = (patch: Partial<Trade>) => ({ ...runFixture.trades[0], legs: undefined, ...patch }) as Trade;
  const marks = tradeMarks([
    t({ dir: 1, entry: "2024-01-02", entry_px: 10.8, exit: "2024-01-09", exit_px: 11.6, open: false }),
    t({ dir: -1, entry: "2024-01-01", entry_px: 9, exit: "2024-01-03", exit_px: null, open: false }), // before the first candle; exit has no price
  ], r.slots);
  expect(marks).toEqual([{ slot: 0, price: 10.8, buy: true, leg: false }, { slot: 1, price: 11.6, buy: false, leg: false }]);
});

it("draws X/O glyphs, Kagi thickness and fill triangles", () => {
  const calls: string[] = [];
  // records method calls and line widths
  const g = new Proxy({}, { get: (_, k) => () => calls.push(String(k)), set: (_, k, v) => { if (k === "lineWidth") calls.push(`w${v}`); return true; } }) as CanvasRenderingContext2D;
  const colors = { green: "g", red: "r", panel: "p" } as ChartColors;
  const item = (slot: Partial<Slot>, marks = [] as { slot: number; price: number; buy: boolean; leg: boolean }[]) => ({ x: 10, originalData: { time: 1, slot: { at: "", from: "", to: "", open: 0, high: 0, low: 0, close: 0, up: true, ...slot }, marks } });
  const toY = (p: number) => 1000 - p * 10;
  drawItems(g, "pnf", { bars: [item({ up: false, low: 99, high: 105, boxes: 7, box: 1 })], barSpacing: 20, visibleRange: { from: 0, to: 1 } }, toY, colors);
  expect(calls.filter((c) => c === "arc")).toHaveLength(7);
  calls.length = 0;
  drawItems(g, "kagi", { bars: [item({ open: 15, close: 9, split: 10, thick0: true, thick1: false }, [{ slot: 0, price: 12, buy: true, leg: false }]), item({ open: 9, close: 16, thick1: true })], barSpacing: 20, visibleRange: { from: 0, to: 1 } }, toY, colors);
  // thick down to the waist, thin below it, thin connector, then the buy triangle
  expect(calls.filter((c) => c.startsWith("w"))).toEqual(["w3", "w1.2", "w1.2", "w1"]);
  expect(calls.filter((c) => c === "fill")).toHaveLength(1);
});

// ---- the chart-type switch on the workbench
let created: { kinds: string[]; data: unknown[][]; options: Record<string, unknown>[] }[] = [];
beforeEach(() => {
  created = [];
  const make = () => {
    const c = { kinds: [] as string[], data: [] as unknown[][], options: [] as Record<string, unknown>[] };
    created.push(c);
    const add = (kind: string) => () => { c.kinds.push(kind); return { setData: (d: unknown[]) => c.data.push(d), setMarkers: vi.fn(), applyOptions: vi.fn(), createPriceLine: vi.fn() }; };
    return { addCandlestickSeries: add("candle"), addLineSeries: add("line"), addAreaSeries: add("area"), addCustomSeries: add("custom"), remove: vi.fn(), resize: vi.fn(),
      applyOptions: (o: Record<string, unknown>) => c.options.push(o), timeScale: () => ({ fitContent: vi.fn(), setVisibleRange: vi.fn(), setVisibleLogicalRange: vi.fn() }) };
  };
  window.LightweightCharts = { createChart: vi.fn(make), CrosshairMode: { Normal: 0 } } as never;
});
afterEach(async () => { await unmount(); delete window.LightweightCharts; });

const base = cat.indicators[0];
const type = (id: string, name: string, outputs: string[], params: Indicator["params"] = []): Indicator => ({ ...base, id, name, outputs, params, placement: "pane", category: "Price functions" });
const catalogue: Catalogue = { ...cat, indicators: [...cat.indicators,
  type("renko", "Renko (bar-aligned)", ["brick_close", "direction", "bricks", "brick_high", "brick_low"], [{ key: "box", label: "Box size (points, mode 1)", default: 1, min: 0.0001, max: 1e9, step: 0.0001, integer: false }]),
  type("line_break", "Line Break (bar-aligned)", ["line_open", "line_close", "new_line"])] };

it("switches to Renko: one custom series on synthetic times, dated hover, parameters from the Parameters window; Candles stay as before", async () => {
  const load = vi.fn(async (q: { id: string; p?: Record<string, number> }) => {
    if (q.id === "renko") return seriesFixture("renko", ["bricks", "brick_high", "brick_low", "direction"], { bricks: [0, 1, 1, 0, -1, -1], brick_high: [50, 52, 54, 54, 52, 50], brick_low: [50, 50, 52, 52, 50, 48], direction: [0, 1, 1, 1, -1, -1] });
    return seriesFixture(q.id, [q.id], { [q.id]: [1, 2, 3, 4, 5, 6] });
  });
  const host = await mount(<Workbench market="in" ticker="NIFTY 50" tf="W" range="All" catalogue={catalogue} chips={[{ key: "c1", id: "ema", params: { length: 20 } }]} loadSeries={load} onChipChange={vi.fn()} onChipRemove={vi.fn()} />);
  await flush();
  expect(created[0].kinds[0]).toBe("candle");
  const buttons = [...host.querySelectorAll<HTMLButtonElement>('[data-lab-chart-type] button')];
  expect(buttons.map((b) => b.textContent)).toEqual(["Candles", "Heikin Ashi", "Renko", "Point & Figure", "Kagi", "Line Break"]);
  // types this engine does not list are disabled
  expect(buttons.map((b) => b.disabled)).toEqual([false, true, false, true, true, false]);
  await act(async () => buttons[2].click());
  await flush();
  const chart = created.at(-1)!;
  expect(chart.kinds).toEqual(["custom"]);
  // daily outputs, whatever the candle size
  expect(load).toHaveBeenLastCalledWith({ id: "renko", tf: "D", p: {} });
  const items = chart.data[0] as { time: number; slot: Slot }[];
  expect(items.map((i) => i.time)).toEqual([1, 2, 3, 4]);
  expect(items.map((i) => [i.slot.open, i.slot.close])).toEqual([[50, 52], [52, 54], [52, 50], [50, 48]]);
  const fmt = chart.options.find((o) => o.localization) as { localization: { timeFormatter(t: number): string } };
  expect(fmt.localization.timeFormatter(3)).toBe("2024-01-15 → 2024-02-01");
  expect(host.querySelector(".box-h")?.textContent).toContain("NIFTY 50 · Renko from daily bars");
  expect(host.querySelector('[aria-label="Drawing tools"]')?.hasAttribute("hidden")).toBe(true);
  expect(host.querySelector("[data-lab-timeless-note]")?.textContent).toContain("hover a brick for its dates");
  await act(async () => host.querySelector<HTMLButtonElement>("[data-lab-type-params]")!.click());
  expect(host.querySelector(".lab-params b")?.textContent).toBe("Properties of Renko");
  expect(host.querySelector(".lab-params .lab-plot-look")).toBeNull();
  const input = host.querySelector<HTMLInputElement>(".lab-params input[type=number]")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, "2");
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await flush(); await act(async () => new Promise((r) => setTimeout(r, 250))); await flush();
  expect(load).toHaveBeenLastCalledWith({ id: "renko", tf: "D", p: { box: 2 } });
  // back to Candles: the date-axis chart and its chip come back
  await act(async () => buttons[0].click());
  await flush();
  expect(created.at(-1)!.kinds[0]).not.toBe("custom");
  expect(created.some((c) => c.kinds[0] === "candle")).toBe(true);
  expect(host.querySelector('[aria-label="Drawing tools"]')?.hasAttribute("hidden")).toBe(false);
});
