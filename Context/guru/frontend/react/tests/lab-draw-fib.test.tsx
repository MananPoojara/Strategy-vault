import { act, useState } from "react";
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { DrawingLayer, type Drawing } from "../src/features/lab/DrawingLayer";
import { FIB, FIB_LEVELS } from "../src/features/lab/drawings/fib";
import type { LabChartApi, LabSeriesApi } from "../src/features/lab/globals";
import { mount, unmount } from "./fixtures/lab";

afterEach(unmount);
// jsdom lays nothing out: a 500 px wide drawing layer, one bar every 10 px, price p at y = p / 10
beforeAll(() => { Object.defineProperty(SVGElement.prototype, "clientWidth", { configurable: true, get: () => 500 }); });
const d = ["2024-01-01", "2024-01-02", "2024-01-03", "2024-01-04", "2024-01-05", "2024-01-08"];
const chart = { priceScale: () => ({ width: () => 50 }), timeScale: () => ({ subscribeVisibleLogicalRangeChange: vi.fn(), unsubscribeVisibleLogicalRangeChange: vi.fn(),
  logicalToCoordinate: (l: number) => l * 10, coordinateToLogical: (x: number) => x / 10, getVisibleLogicalRange: () => ({ from: 0, to: 5 }), setVisibleLogicalRange: vi.fn() }) } as unknown as LabChartApi;
const series = { priceToCoordinate: (p: number) => p / 10, coordinateToPrice: (y: number) => y * 10 } as unknown as LabSeriesApi;
const bars = { o: [100, 100, 100, 100, 100, 100], h: [150, 150, 150, 150, 150, 150], l: [50, 50, 50, 50, 50, 50], c: [120, 120, 120, 120, 120, 120], v: [] };

type Opts = Partial<{ tool: string | null; eraser: boolean }>;
/** The layer with its drawings kept, as Workbench keeps them; laid out (redrawn at its width). */
async function layer(start: Drawing[], opts: Opts = {}) {
  const saved: Drawing[][] = [], done = vi.fn();
  function Host() {
    const [drawings, set] = useState(start), [tool, setTool] = useState(opts.tool ?? null);
    return <div><DrawingLayer chart={chart} series={series} times={d} bars={bars} drawings={drawings} tool={tool} color="#2962ff" height={300}
      eraser={opts.eraser} onChange={(n) => { saved.push(n); set(n); }} onDone={() => { done(); setTool(null); }} /></div>;
  }
  const host = await mount(<Host />);
  const svg = () => host.querySelector("[data-lab-draw-layer]")!;
  const ptr = (type: string, x: number, y: number, el: Element = svg()) =>
    act(async () => { el.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y })); });
  const click = async (x: number, y: number, el?: Element) => { await ptr("pointermove", x, y, el); await ptr("pointerdown", x, y, el); await ptr("pointerup", x, y, el); };
  await act(async () => { svg().parentElement!.dispatchEvent(new Event("pointermove")); });
  return { host, svg, ptr, click, done, last: () => saved[saved.length - 1], saved };
}
const num = (el: Element | null, ...ks: string[]) => ks.map((k) => Number(el!.getAttribute(k)));
const texts = (host: Element, sel: string) => [...host.querySelectorAll(sel)].map((t) => t.textContent);
/** The slope of a level's line, dy/dx. */
const slope = (host: Element, key: string) => { const [x1, y1, x2, y2] = num(host.querySelector(`[data-lab-fib="${key}"] line`), "x1", "y1", "x2", "y2"); return (y2 - y1) / (x2 - x1); };
const one = (kind: string, pts: [number, number][]): Drawing => ({ kind, pts: pts.map(([i, p]) => [d[i], p]) });

it("lists TradingView's Gann and Fibonacci tools in its order, under FIBONACCI and GANN", () => {
  expect(FIB.map((t) => [t.name, t.section, t.points])).toEqual([
    ["Fib retracement", "FIBONACCI", 2], ["Trend-based fib extension", "FIBONACCI", 3], ["Fib channel", "FIBONACCI", 3], ["Fib time zone", "FIBONACCI", 2],
    ["Fib speed resistance fan", "FIBONACCI", 2], ["Trend-based fib time", "FIBONACCI", 3], ["Fib circles", "FIBONACCI", 2], ["Fib spiral", "FIBONACCI", 2],
    ["Fib speed resistance arcs", "FIBONACCI", 2], ["Fib wedge", "FIBONACCI", 3], ["Pitchfan", "FIBONACCI", 3],
    ["Gann box", "GANN", 2], ["Gann square fixed", "GANN", 1], ["Gann square", "GANN", 2], ["Gann fan", "GANN", 2]]);
  expect(FIB.every((t) => t.icon)).toBe(true);
});

it("each new tool is placed by clicking its points, stored as pts, drawn while placed, and can be moved and erased", async () => {
  const clicks: [number, number][] = [[0, 10], [20, 30], [40, 20]], want = [[d[0], 100], [d[2], 300], [d[4], 200]];
  for (const t of FIB.filter((t) => t.id !== "fib")) {
    const n = t.points as number, l = await layer([], { tool: t.id });
    for (const [k, [x, y]] of clicks.slice(0, n).entries()) {
      await l.click(x, y);
      if (k < n - 1) { await l.ptr("pointermove", x + 10, y + 5); expect(l.host.querySelector(`[data-lab-drawing="${t.id}"]`), t.id).not.toBeNull(); }
    }
    expect(l.last(), t.id).toEqual([{ kind: t.id, pts: want.slice(0, n) }]);
    const hit = l.host.querySelector('[data-i="0"][data-part="line"]');
    expect(hit, t.id).not.toBeNull();
    await unmount();
    // moved one bar right as a whole
    const m = await layer([one(t.id, [[0, 100], [2, 300], [3, 200]].slice(0, n) as [number, number][])]);
    const h = m.host.querySelector('[data-i="0"][data-part="line"]')!, [x, y] = [Number(h.getAttribute("x1") ?? 0), Number(h.getAttribute("y1") ?? 0)];
    await m.ptr("pointerdown", x, y, h); await m.ptr("pointermove", x + 10, y); await m.ptr("pointerup", x + 10, y);
    expect(m.last(), t.id).toEqual([one(t.id, [[1, 100], [3, 300], [4, 200]].slice(0, n) as [number, number][])]);
    await unmount();
    const e = await layer([one(t.id, [[0, 100], [2, 300], [3, 200]].slice(0, n) as [number, number][])], { eraser: true });
    await e.ptr("pointerdown", 0, 0, e.host.querySelector('[data-i="0"][data-part="line"]')!);
    expect(e.last(), t.id).toEqual([]);
    await unmount();
  }
});

it("Fib retracement still stores and draws the earlier { a, b } form", async () => {
  const { host, click, last } = await layer([], { tool: "fib" });
  await click(10, 100); await click(30, 200);
  expect(last()).toEqual([{ kind: "fib", a: [d[1], 1000], b: [d[3], 2000] }]);
  expect(texts(host, "[data-lab-fib] text")).toHaveLength(FIB_LEVELS.length);
});

it("Trend-based fib extension projects the 1→2 move from point 3, drawn from point 2 to point 3", async () => {
  const { host } = await layer([one("fibext", [[0, 100], [1, 300], [2, 200]])]);
  const labels = texts(host, "[data-lab-fib] text");
  expect(labels).toEqual(FIB_LEVELS.map(([lv]) => `${lv} (${(200 + 200 * lv).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })})`));
  expect(labels).toEqual(expect.arrayContaining(["0 (200.00)", "0.382 (276.40)", "1 (400.00)", "1.618 (523.60)", "2.618 (723.60)", "4.236 (1,047.20)"]));
  const l = host.querySelector('[data-lab-fib="1.618"] line');
  expect(num(l, "x1", "x2", "y1")).toEqual([10, 20, 52.36]);
});

it("Fib channel: parallel copies of the 1→2 line, level 1 through point 3", async () => {
  const { host } = await layer([one("fibchannel", [[0, 100], [2, 100], [1, 300]])]);
  const y = (lv: number) => num(host.querySelector(`[data-lab-fib="${lv}"] line`), "x1", "y1", "x2", "y2");
  expect(y(0)).toEqual([0, 10, 20, 10]);
  expect(y(1)).toEqual([0, 30, 20, 30]);
  expect(y(0.5)).toEqual([0, 20, 20, 20]);
  expect(y(1.618)[1]).toBeCloseTo(42.36);
});

it("Fib time zone: vertical lines at Fibonacci multiples of the bars between the points, inside the plot", async () => {
  const { host } = await layer([one("fibtimezone", [[0, 100], [1, 100]])]);
  expect(texts(host, '[data-lab-drawing="fibtimezone"] [data-lab-fib] text')).toEqual(["0", "1", "2", "3", "5", "8", "13", "21", "34"]); // 55 bars is past the 450 px plot
  expect(num(host.querySelector('[data-lab-fib="13"] line'), "x1", "y1", "y2")).toEqual([130, 0, 300]);
});

it("Trend-based fib time: time levels from point 3 by the bars from point 1 to point 2", async () => {
  const { host } = await layer([one("fibtrendtime", [[0, 100], [2, 300], [1, 200]])]);
  const x = (lv: number) => num(host.querySelector(`[data-lab-fib="${lv}"] line`), "x1")[0];
  expect(x(0)).toBe(10);
  expect(x(0.618)).toBeCloseTo(22.36);
  expect(x(1)).toBe(30);
  expect(x(1.618)).toBeCloseTo(42.36);
});

it("Fib speed resistance fan: rays from point 1 through the price and time levels at point 2", async () => {
  const { host } = await layer([one("fibfan", [[0, 100], [4, 300]])]); // (0,10) to (40,30)
  expect(slope(host, "p0")).toBeCloseTo(0.5);
  expect(slope(host, "p0.5")).toBeCloseTo(0.25);
  expect(slope(host, "p0.75")).toBeCloseTo(0.125);
  expect(slope(host, "t0.5")).toBeCloseTo(1);
  expect(slope(host, "t0.75")).toBeCloseTo(2);
});

it("Fib circles around point 1, level 1 through point 2", async () => {
  const { host } = await layer([one("fibcircles", [[2, 100], [5, 500]])]); // (20,10) to (50,50): 50 px
  expect(num(host.querySelector('[data-lab-fib="1"] circle'), "cx", "cy", "r")).toEqual([20, 10, 50]);
  expect(num(host.querySelector('[data-lab-fib="0.5"] circle'), "r")).toEqual([25]);
  expect(texts(host, '[data-lab-drawing="fibcircles"] [data-lab-fib] text')).toEqual(FIB_LEVELS.slice(1).map(([lv]) => String(lv)));
});

it("Fib spiral passes through point 2 and grows by φ every quarter turn", async () => {
  const { host } = await layer([one("fibspiral", [[2, 100], [5, 500]])]);
  const pts = host.querySelector('[data-lab-fib="spiral"]')!.getAttribute("d")!.slice(1).split("L").map((p) => p.split(" ").map(Number));
  expect(pts[160][0]).toBeCloseTo(50); expect(pts[160][1]).toBeCloseTo(50);
  expect(Math.hypot(pts[180][0] - 20, pts[180][1] - 10)).toBeCloseTo(50 * (1 + Math.sqrt(5)) / 2);
});

it("Fib speed resistance arcs: half circles around point 1 on point 2's side", async () => {
  const { host } = await layer([one("fibarcs", [[2, 300], [5, 700]])]); // (20,30) to (50,70): below
  expect(host.querySelector('[data-lab-fib="1"] path')!.getAttribute("d")).toBe("M-30 30A50 50 0 0 0 70 30");
  expect(host.querySelector('[data-lab-fib="0.5"] path')!.getAttribute("d")).toBe("M-5 30A25 25 0 0 0 45 30");
});

it("Fib wedge: arcs between the rays to points 2 and 3, level 1 at point 2's distance", async () => {
  const { host } = await layer([one("fibwedge", [[0, 100], [3, 100], [0, 400]])]); // apex (0,10), rays right and down
  const [m, a] = host.querySelector('[data-lab-fib="1"] path')!.getAttribute("d")!.split("A");
  expect(m).toBe("M30 10");
  const [r, , , , sweep, x, y] = a.split(" ").map(Number);
  expect([r, sweep, y]).toEqual([30, 1, 40]); expect(x).toBeCloseTo(0);
  expect(host.querySelector('[data-lab-fib="0.5"] path')!.getAttribute("d")!.startsWith("M15 10A15 15 0 0 1")).toBe(true);
});

it("Pitchfan: rays from point 1 through the median and the levels either side", async () => {
  const { host } = await layer([one("pitchfan", [[0, 100], [4, 0], [4, 400]])]); // (0,10); (40,0) and (40,40)
  expect(slope(host, "median")).toBeCloseTo(0.25);
  expect(slope(host, "b1")).toBeCloseTo(-0.25);
  expect(slope(host, "b0.5")).toBeCloseTo(0);
  expect(slope(host, "e1")).toBeCloseTo(0.75);
});

it("Gann box: price and time levels 0, 0.25, 0.382, 0.5, 0.618, 0.75, 1 across the box", async () => {
  const { host } = await layer([one("gannbox", [[0, 100], [4, 300]])]); // (0,10) to (40,30)
  expect(num(host.querySelector('[data-lab-fib="p0.25"] line'), "y1")).toEqual([15]);
  expect(num(host.querySelector('[data-lab-fib="p0.618"] line'), "y1")[0]).toBeCloseTo(22.36);
  expect(num(host.querySelector('[data-lab-fib="t0.382"] line'), "x1")[0]).toBeCloseTo(15.28);
  expect(texts(host, '[data-lab-fib^="p"] text')).toEqual(["0", "0.25", "0.382", "0.5", "0.618", "0.75", "1"]);
});

it("Gann square: the grid, Gann fan lines to the box's edge and arcs; Gann square fixed sizes itself", async () => {
  const { host } = await layer([one("gannsquare", [[0, 100], [4, 300]]), one("gannfixed", [[1, 100]])]);
  const sq = host.querySelector('[data-lab-drawing="gannsquare"]')!;
  expect(num(sq.querySelector('[data-lab-fib="1x2"] line'), "x2", "y2")).toEqual([40, 20]);
  expect(num(sq.querySelector('[data-lab-fib="2x1"] line'), "x2", "y2")).toEqual([20, 30]);
  expect(sq.querySelector('[data-lab-fib="a1"]')!.getAttribute("d")).toBe("M40 10A40 20 0 0 1 0 30");
  // 36 bars by 36 × the average high-low range (100): from (10,10) to bar 37 and price 3,700
  const fx = host.querySelector('[data-lab-drawing="gannfixed"]')!;
  expect(num(fx.querySelector('[data-lab-fib="p1"] line'), "y1")).toEqual([370]);
  expect(num(fx.querySelector('[data-lab-fib="t1"] line'), "x1")).toEqual([370]);
});

it("Gann fan: 1x8 … 8x1 from point 1, 1x1 through point 2, labelled", async () => {
  const { host } = await layer([one("gannfan", [[0, 100], [4, 300]])]); // (0,10) to (40,30): 1x1 falls 0.5 px per px
  expect(slope(host, "1x1")).toBeCloseTo(0.5);
  expect(slope(host, "2x1")).toBeCloseTo(1);
  expect(slope(host, "1x2")).toBeCloseTo(0.25);
  expect(slope(host, "8x1")).toBeCloseTo(4);
  expect(texts(host, '[data-lab-drawing="gannfan"] text')).toEqual(["1x8", "1x4", "1x3", "1x2", "1x1", "2x1", "3x1", "4x1", "8x1"]);
});
