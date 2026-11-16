import { act, useState } from "react";
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { DrawingLayer, type Drawing } from "../src/features/lab/DrawingLayer";
import { GROUPS } from "../src/features/lab/drawings/registry";
import type { LabChartApi, LabSeriesApi } from "../src/features/lab/globals";
import { mount, unmount } from "./fixtures/lab";

afterEach(unmount);
// jsdom lays nothing out: a 500 px wide drawing layer (plot 450), one bar every 10 px, price p at y = p / 10
beforeAll(() => { Object.defineProperty(SVGElement.prototype, "clientWidth", { configurable: true, get: () => 500 }); });
const d = Array.from({ length: 10 }, (_, k) => `2024-01-${String(k + 1).padStart(2, "0")}`);
const chart = { priceScale: () => ({ width: () => 50 }), timeScale: () => ({ subscribeVisibleLogicalRangeChange: vi.fn(), unsubscribeVisibleLogicalRangeChange: vi.fn(),
  logicalToCoordinate: (l: number) => l * 10, coordinateToLogical: (x: number) => x / 10, getVisibleLogicalRange: () => ({ from: 0, to: 9 }), setVisibleLogicalRange: vi.fn() }) } as unknown as LabChartApi;
const series = { priceToCoordinate: (p: number) => p / 10, coordinateToPrice: (y: number) => y * 10 } as unknown as LabSeriesApi;

async function layer(start: Drawing[], tool: string | null = null) {
  const saved: Drawing[][] = [];
  function Host() {
    const [drawings, set] = useState(start), [t, setTool] = useState(tool);
    return <div><DrawingLayer chart={chart} series={series} times={d} drawings={drawings} tool={t} color="#2962ff" height={300}
      onChange={(n) => { saved.push(n); set(n); }} onDone={() => setTool(null)} /></div>;
  }
  const host = await mount(<Host />);
  const svg = () => host.querySelector("[data-lab-draw-layer]")!;
  const ptr = (type: string, x: number, y: number, el: Element = svg()) =>
    act(async () => { el.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y })); });
  const click = async (x: number, y: number) => { await ptr("pointermove", x, y); await ptr("pointerdown", x, y); await ptr("pointerup", x, y); };
  /** Clicks one point per bar, at these prices. */
  const draw = async (...prices: number[]) => { for (const [k, p] of prices.entries()) await click(k * 10, p / 10); };
  return { host, ptr, click, draw, last: () => saved[saved.length - 1] };
}
const g = (host: Element, kind: string) => host.querySelector(`[data-lab-drawing="${kind}"]`)!;
const points = (el: Element) => [...el.querySelectorAll("[data-lab-point]")].map((n) => n.getAttribute("data-lab-point"));
const ratios = (el: Element) => Object.fromEntries([...el.querySelectorAll("[data-lab-ratio]")].map((n) => [n.getAttribute("data-lab-ratio"), n.textContent]));

it("lists TradingView's patterns, Elliott waves and cycles in its order and sections", () => {
  const tools = GROUPS.find((x) => x.id === "patterns")!.tools;
  expect(tools.map((t) => [t.section, t.name, t.points])).toEqual([
    ["CHART PATTERNS", "XABCD pattern", 5], ["CHART PATTERNS", "Cypher pattern", 5], ["CHART PATTERNS", "Head and shoulders", 7],
    ["CHART PATTERNS", "ABCD pattern", 4], ["CHART PATTERNS", "Triangle pattern", 4], ["CHART PATTERNS", "Three drives pattern", 7],
    ["ELLIOTT WAVES", "Elliott impulse wave (12345)", 6], ["ELLIOTT WAVES", "Elliott correction wave (ABC)", 4], ["ELLIOTT WAVES", "Elliott triangle wave (ABCDE)", 6],
    ["ELLIOTT WAVES", "Elliott double combo wave (WXY)", 4], ["ELLIOTT WAVES", "Elliott triple combo wave (WXYXZ)", 6],
    ["CYCLES", "Cyclic lines", 2], ["CYCLES", "Time cycles", 2], ["CYCLES", "Sine line", 2]]);
});

it("XABCD: five clicks, the preview following the pointer, filled XAB and BCD, its four ratios", async () => {
  const { host, click, ptr } = await layer([], "xabcd");
  await click(0, 30); await click(10, 10);
  await ptr("pointermove", 20, 20);
  expect(g(host, "xabcd").querySelector('path[stroke-width="2"]')!.getAttribute("d")).toBe("M0 30L10 10L20 20");
  await unmount();
  const l = await layer([], "xabcd");
  await l.draw(300, 100, 200, 150, 280);
  expect(l.last()).toEqual([{ kind: "xabcd", pts: [[d[0], 300], [d[1], 100], [d[2], 200], [d[3], 150], [d[4], 280]] }]);
  const el = g(l.host, "xabcd");
  expect(points(el)).toEqual(["X", "A", "B", "C", "D"]);
  expect([...el.querySelectorAll('path[opacity="0.2"]')].map((p) => p.getAttribute("d"))).toEqual(["M0 30L10 10L20 20Z", "M20 20L30 15L40 28Z"]);
  // AB/XA on X–B, BC/AB on A–C, CD/BC on B–D, AD/XA on X–D
  expect(ratios(el)).toEqual({ "0-2": "0.500", "1-3": "0.500", "2-4": "2.600", "0-4": "0.900" });
});

it("Cypher: XC as an extension of XA and CD as a retracement of XC", async () => {
  const { host, draw } = await layer([], "cypher");
  await draw(300, 100, 200, 50, 250);
  const el = g(host, "cypher");
  expect(points(el)).toEqual(["X", "A", "B", "C", "D"]);
  expect(ratios(el)).toEqual({ "0-2": "0.500", "0-3": "1.250", "0-4": "0.800" });
});

it("Head and shoulders: the neckline through both necks, cut where the outer legs cross it, and the three labels", async () => {
  const { host, draw, last } = await layer([], "headshoulders");
  await draw(100, 200, 150, 300, 150, 200, 100);
  expect(last()[0]).toMatchObject({ kind: "headshoulders" });
  const el = g(host, "headshoulders"), n = el.querySelector("[data-lab-neckline]")!;
  expect(["x1", "y1", "x2", "y2"].map((a) => n.getAttribute(a))).toEqual(["5", "15", "55", "15"]);
  expect(points(el)).toEqual(["Left Shoulder", "Head", "Right Shoulder"]);
  expect(el.querySelectorAll('path[opacity="0.2"]')).toHaveLength(3);
});

it("ABCD: BC/AB and CD/BC, in TradingView's green", async () => {
  const { host, draw } = await layer([], "abcd");
  await draw(100, 300, 200, 400);
  const el = g(host, "abcd");
  expect(ratios(el)).toEqual({ "0-2": "0.500", "1-3": "2.000" });
  expect(el.querySelector('path[stroke-width="2"]')!.getAttribute("stroke")).toBe("#089981");
});

it("Triangle pattern: the A–C and B–D lines meet at the apex", async () => {
  const { host, click } = await layer([], "trianglepattern");
  await click(0, 10); await click(10, 50); await click(20, 20); await click(30, 40);
  const el = g(host, "trianglepattern");
  expect(el.querySelector("[data-lab-triangle]")!.getAttribute("d")).toBe("M0 10L45 32.5L10 50Z");
  expect(points(el)).toEqual(["A", "B", "C", "D"]);
});

it("Three drives: the retracements and drive extensions", async () => {
  const { host, draw } = await layer([], "threedrives");
  await draw(50, 100, 300, 200, 400, 300, 500);
  expect(ratios(g(host, "threedrives"))).toEqual({ "1-3": "0.500", "2-4": "2.000", "3-5": "0.500", "4-6": "2.000" });
});

it("Elliott waves label their points at TradingView's Intermediate degree; the start has no label", async () => {
  const { host, draw, last } = await layer([], "elliott-impulse");
  await draw(100, 200, 150, 400, 300, 500);
  expect(last()[0]).toMatchObject({ kind: "elliott-impulse" });
  expect(points(g(host, "elliott-impulse"))).toEqual(["(1)", "(2)", "(3)", "(4)", "(5)"]);
  await unmount();
  const pts = (n: number) => Array.from({ length: n }, (_, k): [string, number] => [d[k], k % 2 ? 200 : 100]);
  const l = await layer([{ kind: "elliott-correction", pts: pts(4) }, { kind: "elliott-triangle", pts: pts(6) },
    { kind: "elliott-double", pts: pts(4) }, { kind: "elliott-triple", pts: pts(6) }]);
  expect(["elliott-correction", "elliott-triangle", "elliott-double", "elliott-triple"].map((k) => points(g(l.host, k)).join(""))).toEqual(
    ["(A)(B)(C)", "(A)(B)(C)(D)(E)", "(W)(X)(Y)", "(W)(X)(Y)(X)(Z)"]);
});

it("Cyclic lines repeat the two points' spacing across the plot; Time cycles draw a half-circle over each", async () => {
  const c = await layer([], "cyclic");
  await c.click(10, 10); await c.click(30, 10);
  const xs = [...g(c.host, "cyclic").querySelectorAll("line[data-lab-cycle]")].map((n) => Number(n.getAttribute("x1")));
  expect(xs).toEqual(Array.from({ length: 23 }, (_, k) => 10 + 20 * k)); // up to the plot's 450 px
  await unmount();
  const t = await layer([], "timecycles");
  await t.click(10, 20); await t.click(30, 10);
  const arcs = [...g(t.host, "timecycles").querySelectorAll("path[data-lab-cycle]")].map((n) => n.getAttribute("d"));
  expect(arcs).toHaveLength(22);
  expect(arcs[0]).toBe("M10 20A10 10 0 0 1 30 20");
});

it("Sine line: through the first point's peak and the second point's trough, continued across the plot", async () => {
  const { host, click } = await layer([], "sine");
  await click(10, 10); await click(30, 30);
  const p = g(host, "sine").querySelector("[data-lab-sine]")!.getAttribute("d")!;
  expect(p.startsWith("M0 20L2 ")).toBe(true);
  expect(p).toContain("L10 10L");
  expect(p).toContain("L30 30L");
  expect(p).toContain("L50 10L");
  expect(p.endsWith("L450 10")).toBe(true);
});

it("a pattern can be selected, moved and reshaped by its handles", async () => {
  const start: Drawing = { kind: "xabcd", pts: [[d[0], 300], [d[1], 100], [d[2], 200], [d[3], 150], [d[4], 280]] };
  const { host, ptr, last } = await layer([start]);
  await ptr("pointerdown", 10, 10, host.querySelector('[data-i="0"][data-part="line"]')!);
  await ptr("pointermove", 20, 20); await ptr("pointerup", 20, 20);
  expect(last()).toEqual([{ kind: "xabcd", pts: [[d[1], 400], [d[2], 200], [d[3], 300], [d[4], 250], [d[5], 380]] }]);
  await ptr("pointerdown", 50, 38, host.querySelector('[data-i="0"][data-part="p4"]')!);
  await ptr("pointermove", 60, 10); await ptr("pointerup", 60, 10);
  expect(last()[0]).toMatchObject({ pts: [[d[1], 400], [d[2], 200], [d[3], 300], [d[4], 250], [d[6], 100]] });
});
