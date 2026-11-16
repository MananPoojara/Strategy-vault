import { act, useState } from "react";
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { DrawingLayer, type Drawing } from "../src/features/lab/DrawingLayer";
import { GROUPS } from "../src/features/lab/drawings/registry";
import type { LabChartApi, LabSeriesApi } from "../src/features/lab/globals";
import { mount, unmount } from "./fixtures/lab";

afterEach(unmount);
// jsdom lays nothing out: a 500 px wide drawing layer (plot 450), one bar every 10 px, price p at y = p / 10
beforeAll(() => { Object.defineProperty(SVGElement.prototype, "clientWidth", { configurable: true, get: () => 500 }); });
const d = ["2024-01-01", "2024-01-02", "2024-01-03", "2024-01-04", "2024-01-05", "2024-01-08"];
const chart = { priceScale: () => ({ width: () => 50 }), timeScale: () => ({ subscribeVisibleLogicalRangeChange: vi.fn(), unsubscribeVisibleLogicalRangeChange: vi.fn(),
  logicalToCoordinate: (l: number) => l * 10, coordinateToLogical: (x: number) => x / 10 }) } as unknown as LabChartApi;
const series = { priceToCoordinate: (p: number) => p / 10, coordinateToPrice: (y: number) => y * 10 } as unknown as LabSeriesApi;
const bars = { o: [0, 0, 0, 0, 0, 0], h: [0, 0, 0, 0, 0, 0], l: [0, 0, 0, 0, 0, 0], c: [1000, 2000, 4000, 3000, 3000, 3000], v: [] };

async function layer(start: Drawing[], tool: string | null = null) {
  const saved: Drawing[][] = [];
  function Host() {
    const [drawings, set] = useState(start), [t, setT] = useState(tool);
    return <div><DrawingLayer chart={chart} series={series} times={d} bars={bars} drawings={drawings} tool={t} color="#2962ff" height={300}
      onChange={(n) => { saved.push(n); set(n); }} onDone={() => setT(null)} /></div>;
  }
  const host = await mount(<Host />);
  const svg = () => host.querySelector("[data-lab-draw-layer]")!;
  const ptr = (type: string, x: number, y: number, el: Element = svg()) =>
    act(async () => { el.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y })); });
  await ptr("pointermove", 0, 0, host.firstElementChild!); // the layer has its width from the second render on
  const click = async (x: number, y: number) => { await ptr("pointermove", x, y); await ptr("pointerdown", x, y); await ptr("pointerup", x, y); };
  return { host, ptr, click, last: () => saved[saved.length - 1] };
}
const lines = (g: Element) => [...g.querySelectorAll("line:not(.lab-draw-hit)")].map((l) => ["x1", "y1", "x2", "y2"].map((a) => Number(l.getAttribute(a))));
const drawn = (host: Element, kind: string) => host.querySelector(`[data-lab-drawing="${kind}"]`)!;

it("the Lines group lists TradingView's tools in its order under LINES, CHANNELS and PITCHFORKS", () => {
  const tools = GROUPS.find((g) => g.id === "lines")!.tools;
  expect(tools.map((t) => t.name)).toEqual(["Trend line", "Ray", "Info line", "Extended line", "Trend angle", "Horizontal line", "Horizontal ray", "Vertical line",
    "Cross line", "Parallel channel", "Regression trend", "Flat top/bottom", "Disjoint channel", "Pitchfork", "Schiff pitchfork", "Modified Schiff pitchfork", "Inside pitchfork"]);
  expect([...new Set(tools.map((t) => t.section))]).toEqual(["LINES", "CHANNELS", "PITCHFORKS"]);
  expect(Object.fromEntries(tools.filter((t) => t.shortcut).map((t) => [t.id, t.shortcut]))).toEqual({ t: "Alt+T", h: "Alt+H", hray: "Alt+J", vline: "Alt+V", crossline: "Alt+C" });
});

it("Info line labels the price change, bars and angle; Trend angle prints the angle", async () => {
  const { host, click, last } = await layer([], "infoline");
  await click(0, 100); await click(20, 120);
  expect(last()).toEqual([{ kind: "infoline", pts: [[d[0], 1000], [d[2], 1200]] }]);
  expect(drawn(host, "infoline").querySelector("[data-lab-label]")!.textContent).toBe("200.00 (20.00%)2 bars, 2d−45.00°");
  await unmount();
  const a = await layer([{ kind: "trendangle", pts: [[d[0], 1000], [d[2], 800]] }]);
  expect(drawn(a.host, "trendangle").querySelector("[data-lab-angle]")!.textContent).toBe("45.00°");
  expect(a.host.querySelector('[data-i="0"][data-part="line"]')).not.toBeNull();
});

it("Horizontal ray runs to the right edge, Vertical line spans the height with its date, Cross line does both", async () => {
  const { host, click, last } = await layer([], "hray");
  await click(20, 50);
  expect(last()).toEqual([{ kind: "hray", pts: [[d[2], 500]] }]);
  expect(lines(drawn(host, "hray"))).toEqual([[20, 50, 450, 50]]);
  expect(drawn(host, "hray").textContent).toBe("500.00");
  await unmount();
  const v = await layer([{ kind: "vline", pts: [[d[3], 500]] }, { kind: "crossline", pts: [[d[1], 700]] }]);
  expect(lines(drawn(v.host, "vline"))).toEqual([[30, 0, 30, 300]]);
  expect(drawn(v.host, "vline").querySelector("[data-lab-label]")!.textContent).toBe(d[3]);
  expect(lines(drawn(v.host, "crossline"))).toEqual([[0, 70, 450, 70], [10, 0, 10, 300]]);
  expect(drawn(v.host, "crossline").textContent).toBe(`700.00${d[1]}`);
});

it("Parallel channel: the third point sets the parallel line, with a dashed middle line", async () => {
  const { host, click, ptr, last } = await layer([], "channel");
  await click(0, 100); await ptr("pointermove", 20, 200);
  expect(lines(drawn(host, "channel"))).toEqual([[0, 100, 20, 200]]); // the base line while the second point follows
  await click(20, 200); await click(10, 50);
  expect(last()).toEqual([{ kind: "channel", pts: [[d[0], 1000], [d[2], 2000], [d[1], 500]] }]);
  expect(lines(drawn(host, "channel"))).toEqual([[0, 100, 20, 200], [0, 0, 20, 100], [0, 50, 20, 150]]);
  expect(drawn(host, "channel").querySelector('line[stroke-dasharray="4 4"]')!.getAttribute("y1")).toBe("50");
});

it("Regression trend fits the closes between its bars, with ±2 standard deviations and Pearson's R", async () => {
  const { host } = await layer([{ kind: "regression", pts: [[d[2], 0], [d[0], 0]] }]); // drawn right to left
  // closes 1000, 2000, 4000: slope 1500, fit 833.33…3833.33, residual deviation √(166666.67 / 2) = 288.68
  const dev = Math.sqrt((166.6667 ** 2 + 333.3333 ** 2 + 166.6667 ** 2) / 2), [u, b, l] = lines(drawn(host, "regression"));
  [[u, 2], [b, 0], [l, -2]].forEach(([ln, z]) => {
    const [x1, y1, x2, y2] = ln as number[];
    expect([x1, x2]).toEqual([0, 20]);
    expect(y1).toBeCloseTo((833.3333 + (z as number) * dev) / 10, 3); expect(y2).toBeCloseTo((3833.3333 + (z as number) * dev) / 10, 3);
  });
  expect(drawn(host, "regression").querySelector("[data-lab-pearson]")!.textContent).toBe("0.98");
  // handle 0 belongs to the first point (the right end)
  expect(host.querySelector('[data-part="p0"]')!.getAttribute("cx")).toBe("20");
});

it("Flat top/bottom draws a flat line at the third point's price; Disjoint channel mirrors the first line from the third point", async () => {
  const { host } = await layer([{ kind: "flattb", pts: [[d[0], 1000], [d[2], 2000], [d[4], 500]] }, { kind: "disjoint", pts: [[d[0], 1000], [d[2], 2000], [d[3], 2500]] }]);
  expect(lines(drawn(host, "flattb"))).toEqual([[0, 100, 20, 200], [0, 50, 20, 50]]);
  expect(lines(drawn(host, "disjoint"))).toEqual([[0, 100, 20, 200], [30, 250, 50, 150]]);
  expect(host.querySelectorAll('[data-i="1"][data-part="line"]')).toHaveLength(3); // the fill and both lines select it
});

it("pitchforks: the median runs from the style's origin through the middle of points 2 and 3, the levels parallel to it", async () => {
  const pts: [string, number][] = [[d[0], 1000], [d[2], 2000], [d[2], 0]]; // pixels (0,100), (20,200), (20,0)
  const { host } = await layer(["pitchfork", "schiff", "mschiff", "ipitchfork"].map((kind) => ({ kind, pts })));
  const level = (kind: string, lv: number) => ["x1", "y1", "x2", "y2"].map((a) => Number(drawn(host, kind).querySelector(`[data-lab-level="${lv}"]`)!.getAttribute(a)));
  expect(level("pitchfork", 0)).toEqual([20, 100, 450, 100]);
  expect(level("pitchfork", 1)).toEqual([20, 0, 450, 0]);
  expect(level("pitchfork", -1)).toEqual([20, 200, 450, 200]);
  expect(level("pitchfork", 0.5)).toEqual([20, 50, 450, 50]);
  // Schiff: origin (0, 150), direction (20, −50); Modified Schiff and Inside: origin (10, 150), direction (10, −50)
  expect(level("schiff", 0)).toEqual([20, 100, 450, 100 - 50 * 430 / 20]);
  expect(level("mschiff", 0)).toEqual([20, 100, 450, 100 - 50 * 430 / 10]);
  expect(level("ipitchfork", 0)).toEqual(level("mschiff", 0));
  expect(drawn(host, "pitchfork").querySelector('[data-lab-level="0"]')!.getAttribute("stroke")).toBe("#F23645");
});

it("a pitchfork is placed with three clicks and moved by dragging", async () => {
  const { host, click, ptr, last } = await layer([], "pitchfork");
  await click(0, 100); await click(20, 200); await click(20, 0);
  expect(last()).toEqual([{ kind: "pitchfork", pts: [[d[0], 1000], [d[2], 2000], [d[2], 0]] }]);
  await ptr("pointerdown", 100, 100, host.querySelector('[data-i="0"][data-part="line"]')!);
  await ptr("pointermove", 110, 110); await ptr("pointerup", 110, 110);
  expect(last()).toEqual([{ kind: "pitchfork", pts: [[d[1], 1100], [d[3], 2100], [d[3], 100]] }]);
});
