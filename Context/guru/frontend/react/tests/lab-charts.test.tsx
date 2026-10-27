import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { aggregate, booleanMarkers, candles, isBooleanOutput, linePoints, ohlcFromSeries, periodKey, rangeStart } from "../src/features/lab/bars";
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
