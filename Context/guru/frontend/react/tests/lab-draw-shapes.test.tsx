import { act, useState } from "react";
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { DrawingLayer, type Drawing } from "../src/features/lab/DrawingLayer";
import type { Pt } from "../src/features/lab/drawings/model";
import { GROUPS, TOOLS } from "../src/features/lab/drawings/registry";
import { SHAPES } from "../src/features/lab/drawings/shapes";
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

/** The layer with its drawings kept, as Workbench keeps them. */
async function layer(start: Drawing[], tool: string | null = null) {
  const saved: Drawing[][] = [];
  function Host() {
    const [drawings, set] = useState(start), [t, setTool] = useState(tool);
    return <div><DrawingLayer chart={chart} series={series} times={d} bars={bars} drawings={drawings} tool={t} color="#2962ff" height={300}
      onChange={(n) => { saved.push(n); set(n); }} onDone={() => setTool(null)} /></div>;
  }
  const host = await mount(<Host />);
  const svg = () => host.querySelector("[data-lab-draw-layer]")!;
  const ptr = (type: string, x: number, y: number, el: Element = svg()) =>
    act(async () => { el.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y })); });
  const click = async (x: number, y: number, el?: Element) => { await ptr("pointermove", x, y, el); await ptr("pointerdown", x, y, el); await ptr("pointerup", x, y, el); };
  const g = (id: string) => host.querySelector(`[data-lab-drawing="${id}"]`)!;
  return { host, svg, ptr, click, g, last: () => saved[saved.length - 1], saved };
}
/** Draws a tool by clicking each [x, y] and returns its rendered group and the saved drawing. */
async function draw(tool: string, clicks: [number, number][]) {
  const l = await layer([], tool);
  for (const [x, y] of clicks) await l.click(x, y);
  return l;
}
const nums = (s: string | null) => (s ?? "").match(/-?\d+(\.\d+)?(e-?\d+)?/g)!.map(Number);

it("lists TradingView's Geometric shapes in order under BRUSHES, ARROWS and SHAPES", () => {
  expect(GROUPS.find((g) => g.id === "shapes")!.tools.map((t) => `${t.section}:${t.name}`)).toEqual([
    "BRUSHES:Brush", "BRUSHES:Highlighter", "ARROWS:Arrow marker", "ARROWS:Arrow", "ARROWS:Arrow mark up", "ARROWS:Arrow mark down",
    "SHAPES:Rectangle", "SHAPES:Rotated rectangle", "SHAPES:Path", "SHAPES:Circle", "SHAPES:Ellipse", "SHAPES:Polyline",
    "SHAPES:Triangle", "SHAPES:Arc", "SHAPES:Curve", "SHAPES:Double curve"]);
  expect(TOOLS.rect.shortcut).toBe("Alt+Shift+R");
});

it("Brush records the pressed path as a smooth stroke; Highlighter is thick and translucent", async () => {
  const b = await layer([], "brush");
  await b.ptr("pointermove", 0, 10); await b.ptr("pointerdown", 0, 10);
  for (const [x, y] of [[10, 20], [20, 15], [30, 30]]) await b.ptr("pointermove", x, y);
  await b.ptr("pointerup", 30, 30);
  expect(b.last()).toEqual([{ kind: "brush", pts: [[d[0], 100], [d[1], 200], [d[2], 150], [d[3], 300]] }]);
  const path = b.g("brush").querySelector("path")!;
  expect(path.getAttribute("d")).toBe("M0 10Q10 20 15 17.5Q20 15 25 22.5L30 30");
  expect(path.getAttribute("stroke")).toBe("#2962ff");
  await unmount();
  const h = await layer([{ kind: "highlighter", pts: [[d[0], 100], [d[3], 300]] }]);
  const hp = h.g("highlighter").querySelector("path")!;
  expect([hp.getAttribute("stroke-width"), hp.getAttribute("stroke-opacity")]).toEqual(["20", "0.2"]);
});

it("Arrow marker is a filled arrow with its tip at the second point; Arrow is a line with a head", async () => {
  const m = await draw("arrowmarker", [[0, 10], [40, 10]]);
  expect(m.last()).toEqual([{ kind: "arrowmarker", pts: [[d[0], 100], [d[4], 100]] }]);
  const md = m.g("arrowmarker").querySelector("path")!.getAttribute("d")!;
  expect(md).toContain("L40 10L"); // the tip
  expect(nums(md).filter((_, k) => k % 2 === 0)).toEqual([0, 26, 26, 40, 26, 26, 0]); // head 35% of the length from the tip
  await unmount();
  const a = await draw("arrow", [[0, 10], [40, 10]]);
  const ad = a.g("arrow").querySelector("path")!.getAttribute("d")!;
  expect(ad.startsWith("M0 10L40 10M")).toBe(true);
  expect(ad).toMatch(/L40 10L/);
});

it("Arrow mark up is green with its tip at the point and the arrow below; Arrow mark down is red, above it", async () => {
  const u = await draw("arrowup", [[20, 10]]);
  expect(u.last()).toEqual([{ kind: "arrowup", pts: [[d[2], 100]] }]);
  const up = u.g("arrowup").querySelector("path")!;
  expect(up.getAttribute("fill")).toBe("#089981");
  expect(up.getAttribute("d")!.startsWith("M20 10L29 20")).toBe(true);
  await unmount();
  const dn = await draw("arrowdown", [[20, 30]]);
  const down = dn.g("arrowdown").querySelector("path")!;
  expect(down.getAttribute("fill")).toBe("#F23645");
  expect(down.getAttribute("d")!.startsWith("M20 30L29 20")).toBe(true);
});

it("Rectangle spans its two corners with a purple border and a 20% fill", async () => {
  const { g, last } = await draw("rect", [[40, 30], [10, 10]]);
  expect(last()).toEqual([{ kind: "rect", pts: [[d[4], 300], [d[1], 100]] }]);
  const r = g("rect").querySelector("rect")!;
  expect(["x", "y", "width", "height", "stroke", "fill-opacity"].map((k) => r.getAttribute(k))).toEqual(["10", "10", "30", "20", "#9C27B0", "0.2"]);
});

it("Rotated rectangle: two points are a side, the third sets the width", async () => {
  const { g, last } = await draw("rotrect", [[0, 10], [40, 10], [20, 30]]);
  expect(last()).toEqual([{ kind: "rotrect", pts: [[d[0], 100], [d[4], 100], [d[2], 300]] }]);
  expect(g("rotrect").querySelector("path")!.getAttribute("d")).toBe("M0 10L40 10L40 30L0 30Z");
  expect(g("rotrect").querySelector('[data-part="p2"]')!.getAttribute("cx")).toBe("20"); // on the opposite side's middle
});

it("Path takes clicks until a double-click and ends in an arrow head", async () => {
  const { ptr, click, g, last } = await layer([], "path");
  await click(0, 10); await click(10, 20); await click(30, 5); await click(30, 5);
  await ptr("dblclick", 30, 5);
  expect(last()).toEqual([{ kind: "path", pts: [[d[0], 100], [d[1], 200], [d[3], 50]] }]);
  expect(g("path").querySelector("path")!.getAttribute("d")).toMatch(/^M0 10L10 20L30 5M.+L30 5L/);
});

it("Circle: a centre and a point on it; Ellipse: an axis and the other half-axis", async () => {
  const c = await draw("circle", [[20, 20], [50, 20]]);
  expect(c.g("circle").querySelector("path")!.getAttribute("d")).toBe("M-10 20A30 30 0 1 0 50 20A30 30 0 1 0 -10 20Z");
  await unmount();
  const e = await draw("ellipse", [[10, 20], [50, 20], [30, 10]]);
  expect(e.last()).toEqual([{ kind: "ellipse", pts: [[d[1], 200], [d[5], 200], [d[3], 100]] }]);
  expect(e.g("ellipse").querySelector("path")!.getAttribute("d")).toBe("M10 20A20 10 0 1 0 50 20A20 10 0 1 0 10 20Z");
});

it("Polyline closes and fills its points; Triangle fills its three", async () => {
  const p = await layer([], "polyline");
  await p.click(0, 10); await p.click(20, 30); await p.click(40, 10); await p.click(40, 10);
  await p.ptr("dblclick", 40, 10);
  const pp = p.g("polyline").querySelector("path")!;
  expect([pp.getAttribute("d"), pp.getAttribute("fill-opacity")]).toEqual(["M0 10L20 30L40 10Z", "0.2"]);
  await unmount();
  const t = await draw("triangle", [[0, 10], [20, 30], [40, 10]]);
  expect(t.last()).toEqual([{ kind: "triangle", pts: [[d[0], 100], [d[2], 300], [d[4], 100]] }]);
  expect(t.g("triangle").querySelector("path")!.getAttribute("d")).toBe("M0 10L20 30L40 10Z");
});

it("Arc passes from the first point through the third to the second", async () => {
  const { g } = await draw("arc", [[0, 20], [40, 20], [20, 10]]);
  const arc = nums(g("arc").querySelectorAll("path")[1].getAttribute("d"));
  // the circle through (0,20), (20,10), (40,20) has radius 25; the minor arc, clockwise on screen
  expect(arc.slice(0, 2)).toEqual([0, 20]);
  expect(arc[2]).toBeCloseTo(25); expect(arc[3]).toBeCloseTo(25);
  expect(arc.slice(4)).toEqual([0, 0, 1, 40, 20]);
});

it("Curve bends towards its control point; Double curve has two", async () => {
  const c = await draw("curve", [[0, 20], [40, 20], [20, 0]]);
  expect(c.g("curve").querySelector("path")!.getAttribute("d")).toBe("M0 20Q20 0 40 20");
  await unmount();
  const dc = await draw("dcurve", [[0, 20], [40, 20], [10, 0], [30, 40]]);
  expect(dc.last()).toEqual([{ kind: "dcurve", pts: [[d[0], 200], [d[4], 200], [d[1], 0], [d[3], 400]] }]);
  expect(dc.g("dcurve").querySelector("path")!.getAttribute("d")).toBe("M0 20C10 0 30 40 40 20");
});

it("every shape has hit targets and point handles, so it can be selected, moved and reshaped", async () => {
  const pts = (n: number): Pt[] => [[d[0], 100], [d[4], 100], [d[2], 300], [d[3], 50]].slice(0, n) as Pt[];
  const saved = SHAPES.map((t) => ({ kind: t.id, pts: pts(typeof t.points === "number" ? t.points : 3) }));
  const { host } = await layer(saved);
  SHAPES.forEach((t, i) => {
    expect(host.querySelector(`[data-lab-drawing="${t.id}"] [data-i="${i}"][data-part="line"]`), t.id).not.toBeNull();
    expect(host.querySelector(`[data-lab-drawing="${t.id}"] [data-i="${i}"][data-part="p0"]`), t.id).not.toBeNull();
  });
});

it("a rectangle moves by its inside and reshapes by a corner", async () => {
  const { host, ptr, last } = await layer([{ kind: "rect", pts: [[d[1], 100], [d[3], 300]] }]);
  await ptr("pointerdown", 20, 20, host.querySelector('[data-i="0"][data-part="line"]')!);
  await ptr("pointermove", 30, 30); await ptr("pointerup", 30, 30);
  expect(last()).toEqual([{ kind: "rect", pts: [[d[2], 200], [d[4], 400]] }]);
  await ptr("pointerdown", 40, 40, host.querySelector('[data-i="0"][data-part="p1"]')!);
  await ptr("pointermove", 50, 20); await ptr("pointerup", 50, 20);
  expect(last()).toEqual([{ kind: "rect", pts: [[d[2], 200], [d[5], 200]] }]);
});
