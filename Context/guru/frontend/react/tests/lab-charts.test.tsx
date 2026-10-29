import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { aggregate, booleanMarkers, candles, isBooleanOutput, linePoints, zonePairs, ohlcFromSeries, periodKey, rangeStart } from "../src/features/lab/bars";
import { createLabChart, useThemeKey } from "../src/features/lab/charts";
import { mount, seriesFixture, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); vi.unstubAllGlobals(); document.documentElement.removeAttribute("data-theme"); });

const px = { d: ["2024-01-05", "2024-01-08", "2024-01-09", "2024-01-15", "2024-02-01"], o: [1, 2, 3, 4, 5], h: [2, 6, 4, 5, 9], l: [0.5, 1, 2, 3, 4], c: [1.5, 3, 3.5, 4.5, 8] };

it("aggregates Monday-start weeks, months, quarters and years, labelled by the last daily date", () => {
  expect(periodKey("2024-01-07", "W")).toBe("2024-01-01");
  expect(periodKey("2024-05-20", "Q")).toBe("2024-Q2");
  const w = aggregate(px, "W");
  expect(w.d).toEqual(["2024-01-05", "2024-01-09", "2024-01-15", "2024-02-01"]);
  expect([w.o[1], w.h[1], w.l[1], w.c[1]]).toEqual([2, 6, 1, 3.5]);
  expect(aggregate(px, "M").d).toEqual(["2024-01-15", "2024-02-01"]);
  expect(aggregate(px, "Y").d).toEqual(["2024-02-01"]);
  expect(aggregate(px, "D")).toBe(px);
});

it("builds OHLC from series responses, dropping incomplete bars", () => {
  const s = (id: string, v: (number | null)[]) => seriesFixture(id, [id], { [id]: v });
  const out = ohlcFromSeries({ open: s("open", [1, 2, null, 4, 5, 6]), high: s("high", [2, 3, 4, 5, 6, 7]), low: s("low", [0, 1, 2, 3, 4, 5]), close: s("close", [1, 2, 3, 4, 5, 6]) });
  expect(out.d).toHaveLength(5);
  expect(candles(out)[0]).toEqual({ time: "2024-01-05", open: 1, high: 2, low: 0, close: 1 });
});

it("draws gaps for nulls, samples period ends, and turns 0/1 outputs into markers", () => {
  expect(linePoints(["a", "b", "c"], [1, null, 3])).toEqual([{ time: "a", value: 1 }, { time: "b" }, { time: "c", value: 3 }]);
  expect(linePoints(["a", "b", "c"], [1, 2, 3], new Set(["c"]))).toEqual([{ time: "c", value: 3 }]);
  expect(isBooleanOutput([0, 1, null, 0])).toBe(true);
  expect(isBooleanOutput([0, 1, 2])).toBe(false);
  expect(booleanMarkers(["a", "b", "c"], [0, 1, 1], "Doji", "#f00")).toEqual([
    { time: "b", position: "aboveBar", shape: "circle", color: "#f00", text: "Doji" },
    { time: "c", position: "aboveBar", shape: "circle", color: "#f00", text: "Doji" }]);
  expect(rangeStart(["2020-01-01", "2023-06-01", "2024-06-01", "2025-06-01"], "1Y")).toBe("2024-06-01");
  expect(rangeStart(["2020-01-01"], "All")).toBeNull();
});

it("creates a themed chart and disposes it; no library means no chart", () => {
  const remove = vi.fn(), disconnect = vi.fn(), observe = vi.fn();
  vi.stubGlobal("ResizeObserver", class { observe = observe; disconnect = disconnect; });
  const createChart = vi.fn(() => ({ remove, resize: vi.fn() }) as never);
  const el = document.createElement("div");
  document.body.append(el);
  el.style.setProperty("--panel", "#123456"); // jsdom does not inherit custom properties; set them on the host
  const made = createLabChart(el, 300, {}, { createChart });
  expect((createChart.mock.calls[0] as unknown[])[1]).toMatchObject({ height: 300, layout: { background: { color: "#123456" } } });
  made!.dispose();
  expect(remove).toHaveBeenCalled();
  expect(disconnect).toHaveBeenCalled();
  expect(createLabChart(el, 300, {}, undefined)).toBeNull();
  el.remove();
});

it("useThemeKey changes when the theme attribute changes", async () => {
  const Probe = () => <span>{useThemeKey() || "default"}</span>;
  const host = await mount(<Probe />);
  expect(host.textContent).toBe("default");
  await act(async () => { document.documentElement.setAttribute("data-theme", "midnight"); await new Promise((r) => setTimeout(r, 0)); });
  expect(host.textContent).toBe("midnight");
});


it("resizes an opted-in pane vertically while preserving the visible dates", () => {
  let notify: ResizeObserverCallback | undefined;
  vi.stubGlobal("ResizeObserver", class { constructor(fn: ResizeObserverCallback) { notify = fn; } observe(){} disconnect(){} });
  const resize = vi.fn(), setVisibleRange = vi.fn(), range = { from:"2024-01-01", to:"2024-06-01" };
  const library = { createChart: vi.fn(() => ({ resize, remove:vi.fn(), timeScale:() => ({ getVisibleRange:() => range, setVisibleRange }) }) as never) };
  const el = document.createElement("div");
  Object.defineProperties(el, { clientWidth:{ value:800 }, clientHeight:{ value:240 } });
  const view = createLabChart(el,150,{},library,true);
  notify!([],{} as ResizeObserver);
  expect(resize).toHaveBeenCalledWith(800,240);
  expect(setVisibleRange).toHaveBeenCalledWith(range);
  view!.dispose();
});

it("bull / bear signals are small unlabelled dots under / over the bar; top+bottom pairs are zones", () => {
  expect(booleanMarkers(["a", "b"], [1, 0], "FVG.bull", "#0a0", undefined, "bull")).toEqual([{ time: "a", position: "belowBar", shape: "circle", color: "#0a0", text: "", size: 0.6 }]);
  expect(booleanMarkers(["a"], [1], "FVG.bear", "#a00", undefined, "bear")[0]).toMatchObject({ position: "aboveBar", text: "" });
  expect(zonePairs(["bull", "bear", "bull_top", "bull_bottom", "bear_top", "bear_bottom", "msh"])).toEqual(["bull", "bear"]);
  expect(zonePairs(["upper", "lower", "x_top"])).toEqual([]);
});

it("BOS / CHoCH / EQH events become labelled lines from the swing bar to the break", async () => {
  const { breakLines, breakOutputs } = await import("../src/features/lab/bars");
  const px = { d: ["a", "b", "c", "d", "e"], o: [1, 1, 1, 1, 1], h: [2, 5, 3, 4, 6], l: [1, 2, 1, 2, 3], c: [1.5, 3, 2, 3, 5.5] };
  const outputs = ["bull_bos", "bear_bos", "bull_level", "bear_level", "trend"];
  const series = { bull_bos: [0, 0, 0, 0, 1], bear_bos: [0, 0, 0, 0, 0], bull_level: [null, null, null, null, 5], bear_level: [null, null, null, null, null], trend: [0, 0, 0, 0, 1] };
  const lines = breakLines(outputs, series, px.d, px);
  expect(lines).toEqual([{ from: 1, to: 4, price: 5, text: "BOS", up: true, dotted: false, above: true }]);
  expect([...breakOutputs(lines, outputs)].sort()).toEqual(["bear_bos", "bear_level", "bull_bos", "bull_level"]);
  expect(breakLines(["bull", "bull_level"], { bull: [1], bull_level: [2] }, ["a"], px)).toEqual([]);
  // a gap candle breaking a swing high still matches the high, not a nearby low
  const gap = { d: ["a", "b", "c"], o: [1, 1, 9], h: [5, 4, 10], l: [1, 5, 9], c: [4, 4.5, 9.5] };
  expect(breakLines(["bull_bos", "bull_level"], { bull_bos: [0, 0, 1], bull_level: [null, null, 5] }, gap.d, gap)[0].from).toBe(0);
});

it("Strong / Weak High-Low: the latest top and bottom only, labelled by the swing bias", async () => {
  const { strongWeakLines } = await import("../src/features/lab/bars");
  const px = { d: ["a", "b", "c", "d"], o: [1, 1, 1, 1], h: [3, 6, 4, 5], l: [1, 2, 0.5, 2], c: [2, 5, 1, 4] };
  const series = { trailing_high: [null, 6, 6, 6], trailing_low: [null, 1, 0.5, 0.5], bias: [null, 1, 1, -1] };
  expect(strongWeakLines(["trailing_high", "trailing_low", "bias"], series, px.d, px)).toEqual([
    { from: 1, to: 3, price: 6, text: "Strong High", up: false, dotted: false, above: true, solid: true, atEnd: true },
    { from: 2, to: 3, price: 0.5, text: "Weak Low", up: true, dotted: false, above: false, solid: true, atEnd: true }]);
  expect(strongWeakLines(["trailing_high"], series, px.d, px)).toEqual([]);
});
