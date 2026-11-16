import { act, useState } from "react";
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { DrawingLayer, type Drawing } from "../src/features/lab/DrawingLayer";
import { shapeOf, withPts, type ToolDef } from "../src/features/lab/drawings/model";
import { GROUPS, TOOLS } from "../src/features/lab/drawings/registry";
import type { LabChartApi, LabSeriesApi } from "../src/features/lab/globals";
import { mount, unmount } from "./fixtures/lab";

afterEach(unmount);
// jsdom lays nothing out: a 500 px wide drawing layer, one bar every 10 px, price p at y = p / 10
beforeAll(() => { Object.defineProperty(SVGElement.prototype, "clientWidth", { configurable: true, get: () => 500 }); });
const d = ["2024-01-01", "2024-01-02", "2024-01-03", "2024-01-04", "2024-01-05", "2024-01-08"];
const setRange = vi.fn();
const chart = { priceScale: () => ({ width: () => 50 }), timeScale: () => ({ subscribeVisibleLogicalRangeChange: vi.fn(), unsubscribeVisibleLogicalRangeChange: vi.fn(),
  logicalToCoordinate: (l: number) => l * 10, coordinateToLogical: (x: number) => x / 10, getVisibleLogicalRange: () => ({ from: 0, to: 5 }), setVisibleLogicalRange: setRange }) } as unknown as LabChartApi;
const series = { priceToCoordinate: (p: number) => p / 10, coordinateToPrice: (y: number) => y * 10 } as unknown as LabSeriesApi;
const bars = { o: [100, 100, 100, 100, 100, 100], h: [150, 150, 150, 150, 150, 150], l: [50, 50, 50, 50, 50, 50], c: [120, 120, 120, 120, 120, 120], v: [] };
// test tools: a 3-point triangle, a multi-point path and a one-click text
const poly: ToolDef["render"] = (s, c) => <><path d={s.pts.map(([t, p], k) => `${k ? "L" : "M"}${c.x(t)} ${c.y(p)}`).join("")} stroke={c.color} fill="none" />{c.handles()}</>;
TOOLS.tri = { id: "tri", name: "Triangle", section: "SHAPES", icon: null, points: 3, render: poly };
TOOLS.poly = { id: "poly", name: "Polyline", section: "SHAPES", icon: null, points: "multi", render: poly };
TOOLS.note = { id: "note", name: "Text", section: "TEXT", icon: null, points: 1, text: true,
  render: (s, c) => <><text x={c.x(s.pts[0][0]) ?? 0} y={c.y(s.pts[0][1]) ?? 0}>{s.text}</text>{c.hitArea(0, 0, 10, 10)}</> };

type Opts = Partial<{ tool: string | null; magnet: "weak" | "strong"; stay: boolean; locked: boolean; eraser: boolean }>;
/** The layer with its drawings kept, as Workbench keeps them. */
async function layer(start: Drawing[], opts: Opts = {}) {
  const saved: Drawing[][] = [], done = vi.fn();
  function Host() {
    const [drawings, set] = useState(start), [tool, setTool] = useState(opts.tool ?? null);
    return <div><DrawingLayer chart={chart} series={series} times={d} bars={bars} drawings={drawings} tool={tool} color="#2962ff" height={300}
      magnet={opts.magnet} stay={opts.stay} locked={opts.locked} eraser={opts.eraser} onChange={(n) => { saved.push(n); set(n); }} onDone={() => { done(); setTool(null); }} /></div>;
  }
  const host = await mount(<Host />);
  const svg = () => host.querySelector("[data-lab-draw-layer]")!;
  const ptr = (type: string, x: number, y: number, el: Element = svg()) =>
    act(async () => { el.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y })); });
  const click = async (x: number, y: number, el?: Element) => { await ptr("pointermove", x, y, el); await ptr("pointerdown", x, y, el); await ptr("pointerup", x, y, el); };
  return { host, svg, ptr, click, done, last: () => saved[saved.length - 1], saved };
}

it("the registry keeps TradingView's group order and draws drawings saved by earlier versions as they were", async () => {
  expect(GROUPS.map((g) => g.id)).toEqual(["lines", "fib", "patterns", "forecast", "shapes", "text"]);
  expect(["t", "ray", "x", "h", "fib", "pr", "dr", "dpr"].every((id) => TOOLS[id])).toBe(true);
  const h: Drawing = { kind: "h", price: 100 }, t: Drawing = { kind: "t", a: [d[1], 200], b: [d[2], 300], ext: "right" };
  expect(shapeOf(t)).toEqual({ kind: "t", pts: [[d[1], 200], [d[2], 300]], ext: "right" });
  expect(withPts(t, [[d[0], 1], [d[3], 2]])).toEqual({ kind: "t", a: [d[0], 1], b: [d[3], 2], ext: "right" }); // kept in its stored form
  expect(withPts(h, [["", 50]])).toEqual({ kind: "h", price: 50 });
  const { host } = await layer([h, t, { kind: "fib", a: [d[1], 1000], b: [d[3], 2000] }, { kind: "dpr", a: [d[0], 100], b: [d[5], 110] }]);
  expect([...host.querySelectorAll("[data-lab-drawing]")].map((g) => g.getAttribute("data-lab-drawing"))).toEqual(["h", "t", "fib", "dpr"]);
  const more = await layer([{ kind: "t", a: [d[1], 200], b: [d[2], 300] }, { kind: "t", a: [d[1], 200], b: [d[2], 300], ext: "both" },
    { kind: "pr", a: [d[0], 100], b: [d[2], 120] }, { kind: "dr", a: [d[0], 100], b: [d[2], 120] }]);
  expect([...more.host.querySelectorAll("[data-lab-drawing]")].map((g) => g.getAttribute("data-lab-drawing"))).toEqual(["t", "t", "pr", "dr"]);
});

it("every tool id is unique, each group's sections are contiguous, and make() stores a tool's kind", () => {
  const ids = GROUPS.flatMap((g) => g.tools.map((t) => t.id));
  expect(new Set(ids).size).toBe(ids.length);
  for (const g of GROUPS) {
    const runs = g.tools.map((t) => t.section).filter((s, k, a) => s !== a[k - 1]);
    expect(new Set(runs).size, g.id).toBe(runs.length); // a header is printed once per section
  }
  const pts: [string, number][] = [[d[1], 100], [d[2], 200], [d[3], 150], [d[4], 250], [d[5], 120]];
  for (const t of ids.map((id) => TOOLS[id])) expect(TOOLS[shapeOf(t.make?.(pts, { x: () => null, y: () => null, plot: 0, height: 0 } as never) ?? { kind: t.id, pts }).kind], t.id).toBeDefined();
});

it("a three-point tool takes three clicks, the next point following the pointer", async () => {
  const { host, click, ptr, last } = await layer([], { tool: "tri" });
  await click(0, 10); await click(20, 20);
  expect(last()).toBeUndefined();
  await ptr("pointermove", 40, 10);
  expect(host.querySelector('[data-lab-drawing="tri"] path')!.getAttribute("d")).toBe("M0 10L20 20L40 10");
  await click(40, 10);
  expect(last()).toEqual([{ kind: "tri", pts: [[d[0], 100], [d[2], 200], [d[4], 100]] }]);
});

it("a multi-point tool adds a point per click and finishes on a double-click", async () => {
  const { svg, click, ptr, last, done } = await layer([], { tool: "poly" });
  await click(0, 10); await click(10, 20); await click(30, 5);
  await click(30, 5); // the double-click's second click adds nothing
  await ptr("dblclick", 30, 5);
  expect(last()).toEqual([{ kind: "poly", pts: [[d[0], 100], [d[1], 200], [d[3], 50]] }]);
  expect(done).toHaveBeenCalled();
  expect(svg().querySelectorAll('[data-lab-drawing="poly"]')).toHaveLength(1);
});

it("Strong magnet snaps to the bar's nearest open, high, low or close; Weak magnet only within 30 px", async () => {
  const strong = await layer([], { tool: "h", magnet: "strong" });
  await strong.click(20, 60); // price 600: the high, 150, is the nearest
  expect(strong.last()).toEqual([{ kind: "h", price: 150 }]);
  await unmount();
  const weak = await layer([], { tool: "h", magnet: "weak", stay: true });
  await weak.click(20, 60);
  expect(weak.last()).toEqual([{ kind: "h", price: 600 }]);
  await weak.click(20, 12.5); // the close, 120, is half a pixel away
  expect(weak.last()).toEqual([{ kind: "h", price: 600 }, { kind: "h", price: 120 }]);
});

it("Stay in drawing mode keeps the tool for the next drawing", async () => {
  const { click, last, done } = await layer([], { tool: "h", stay: true });
  await click(10, 10); await click(10, 20);
  expect(last()).toEqual([{ kind: "h", price: 100 }, { kind: "h", price: 200 }]);
  expect(done).not.toHaveBeenCalled();
});

it("Lock all drawings: they cannot be moved or deleted", async () => {
  const { host, ptr, saved } = await layer([{ kind: "t", a: [d[1], 200], b: [d[2], 300] }], { locked: true });
  const hit = host.querySelector('[data-i="0"][data-part="line"]')!;
  await ptr("pointerdown", 15, 25, hit); await ptr("pointermove", 35, 45); await ptr("pointerup", 35, 45);
  await act(async () => { window.dispatchEvent(new KeyboardEvent("keydown", { key: "Delete" })); });
  expect(saved).toHaveLength(0);
  expect(host.querySelector("[data-lab-draw-selected]")).toBeNull();
});

it("the Eraser deletes the drawing clicked", async () => {
  const { host, ptr, last } = await layer([{ kind: "h", price: 100 }, { kind: "t", a: [d[1], 200], b: [d[2], 300] }], { eraser: true });
  await ptr("pointerdown", 15, 25, host.querySelector('[data-i="1"][data-part="line"]')!);
  expect(last()).toEqual([{ kind: "h", price: 100 }]);
});

it("a text tool opens the text editor after placing; Enter keeps the text, a double-click edits it", async () => {
  const { host, click, ptr, last } = await layer([], { tool: "note" });
  await click(20, 10);
  const box = host.querySelector<HTMLTextAreaElement>("[data-lab-draw-text]")!;
  expect(box).not.toBeNull();
  await act(async () => { box.value = "Breakout"; box.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })); });
  expect(last()).toEqual([{ kind: "note", pts: [[d[2], 100]], text: "Breakout" }]);
  expect(host.querySelector("[data-lab-draw-text]")).toBeNull();
  await ptr("dblclick", 5, 5, host.querySelector('[data-i="0"]')!);
  expect(host.querySelector<HTMLTextAreaElement>("[data-lab-draw-text]")!.value).toBe("Breakout");
});

it("Measure shows a date-and-price-range box until the next click, saving nothing; Zoom in sets the dragged bars", async () => {
  const m = await layer([], { tool: "measure" });
  await m.ptr("pointerdown", 0, 10); await m.ptr("pointermove", 30, 20); await m.ptr("pointerup", 30, 20);
  expect([...m.host.querySelectorAll("[data-lab-ruler] [data-lab-measure] text")].map((t) => t.textContent)).toEqual(["100.00 (100.00%)", "3 bars, 3d"]);
  expect(m.saved).toHaveLength(0);
  await m.ptr("pointerdown", 50, 50);
  expect(m.host.querySelector("[data-lab-ruler]")).toBeNull();
  await unmount();
  const z = await layer([], { tool: "zoom" });
  await z.ptr("pointerdown", 10, 10); await z.ptr("pointermove", 40, 30);
  expect(z.host.querySelector("[data-lab-zoom-box]")).not.toBeNull();
  await z.ptr("pointerup", 40, 30);
  expect(setRange).toHaveBeenLastCalledWith({ from: 1, to: 4 });
});

it("a damaged saved drawing is skipped, the others still draw", async () => {
  const { DrawingLayer } = await import("../src/features/lab/DrawingLayer");
  const { mount } = await import("./fixtures/lab");
  const { vi } = await import("vitest");
  const ts = { subscribeVisibleLogicalRangeChange: vi.fn(), unsubscribeVisibleLogicalRangeChange: vi.fn(), logicalToCoordinate: (l: number) => l * 10, coordinateToLogical: (x: number) => x / 10 };
  const chart = { priceScale: () => ({ width: () => 50 }), timeScale: () => ts } as never;
  const series = { priceToCoordinate: (p: number) => p / 10, coordinateToPrice: (y: number) => y * 10 } as never;
  const host = await mount(<DrawingLayer chart={chart} series={series} times={["2024-01-01", "2024-01-02"]} tool={null} color="#2962ff" height={300}
    drawings={[{ kind: "long", pts: [["2024-01-01", 100]] }, { kind: "h", price: 100 }]} onChange={vi.fn()} onDone={vi.fn()} />);
  expect(host.querySelector('[data-lab-drawing="h"]')).not.toBeNull();
});
