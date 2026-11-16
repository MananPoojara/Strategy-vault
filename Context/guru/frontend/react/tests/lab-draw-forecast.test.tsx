import { act, useState } from "react";
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { DrawingLayer, type Drawing } from "../src/features/lab/DrawingLayer";
import { FORECAST, anchoredVwap, volumeProfile } from "../src/features/lab/drawings/forecast";
import type { Bars } from "../src/features/lab/drawings/model";
import type { LabChartApi, LabSeriesApi } from "../src/features/lab/globals";
import { mount, unmount } from "./fixtures/lab";

afterEach(unmount);
// jsdom lays nothing out: a 500 px wide drawing layer, one bar every 10 px, price p at y = p / 10
beforeAll(() => { Object.defineProperty(SVGElement.prototype, "clientWidth", { configurable: true, get: () => 500 }); });
const d = ["2024-01-01", "2024-01-02", "2024-01-03", "2024-01-04", "2024-01-05", "2024-01-08"];
const chart = { priceScale: () => ({ width: () => 50 }), timeScale: () => ({ subscribeVisibleLogicalRangeChange: vi.fn(), unsubscribeVisibleLogicalRangeChange: vi.fn(),
  logicalToCoordinate: (l: number) => l * 10, coordinateToLogical: (x: number) => x / 10, getVisibleLogicalRange: () => ({ from: 0, to: 5 }), setVisibleLogicalRange: vi.fn() }) } as unknown as LabChartApi;
const series = { priceToCoordinate: (p: number) => p / 10, coordinateToPrice: (y: number) => y * 10 } as unknown as LabSeriesApi;
const bars = { o: [95, 100, 115, 110, 120, 135], h: [110, 120, 130, 125, 140, 150], l: [90, 100, 105, 110, 115, 130], c: [100, 115, 110, 120, 135, 140], v: [10, 20, 30, null, 40, 50] };

async function layer(start: Drawing[], tool: string | null = null, data: Omit<Bars, "d"> = bars) {
  const saved: Drawing[][] = [];
  function Host() {
    const [drawings, set] = useState(start), [t, setTool] = useState(tool);
    return <div><DrawingLayer chart={chart} series={series} times={d} bars={data} drawings={drawings} tool={t} color="#2962ff" height={300}
      onChange={(n) => { saved.push(n); set(n); }} onDone={() => setTool(null)} /></div>;
  }
  const host = await mount(<Host />);
  const svg = () => host.querySelector("[data-lab-draw-layer]")!;
  const ptr = (type: string, x: number, y: number, el: Element = svg()) =>
    act(async () => { el.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y })); });
  const click = async (x: number, y: number) => { await ptr("pointermove", x, y); await ptr("pointerdown", x, y); await ptr("pointerup", x, y); };
  const drag = async (el: Element, x0: number, y0: number, x: number, y: number) => { await ptr("pointerdown", x0, y0, el); await ptr("pointermove", x, y); await ptr("pointerup", x, y); };
  const texts = (sel: string) => [...host.querySelectorAll(`${sel} text`)].map((t) => t.textContent);
  return { host, ptr, click, drag, texts, last: () => saved[saved.length - 1] };
}
const attr = (el: Element | null, ...names: string[]) => names.map((n) => Number(el!.getAttribute(n)));

it("lists TradingView's forecasting, volume-based and measurer tools in its order", () => {
  expect(FORECAST.map((t) => [t.section, t.name])).toEqual([
    ["FORECASTING", "Long position"], ["FORECASTING", "Short position"], ["FORECASTING", "Position forecast"], ["FORECASTING", "Bars pattern"],
    ["FORECASTING", "Ghost feed"], ["FORECASTING", "Sector"], ["VOLUME-BASED", "Anchored VWAP"], ["VOLUME-BASED", "Fixed range volume profile"],
    ["VOLUME-BASED", "Anchored volume profile"], ["MEASURERS", "Price range"], ["MEASURERS", "Date range"], ["MEASURERS", "Date and price range"]]);
});

it("Long position: one click places +2% target and −1% stop boxes; dragging the target and width handles reshapes it", async () => {
  const { host, click, drag, texts, last } = await layer([], "long");
  await click(20, 100); // entry 1,000 on the third bar
  const pts = (last()[0] as { pts: [string, number][] }).pts;
  expect(pts.map((p) => p[0])).toEqual([d[2], d[2], d[2], d[2]]);
  expect(pts.map((p) => p[1])).toEqual([1000, expect.closeTo(1020), expect.closeTo(990), 1000]);
  const g = host.querySelector('[data-lab-drawing="long"]')!, [green, red] = g.querySelectorAll("rect[fill='#089981'], rect[fill='#F23645']");
  expect(attr(green, "x", "y", "width", "height").map((v) => +v.toFixed(6))).toEqual([20, 100, 200, 2]); // 20 bars wide by default
  expect(attr(red, "y", "height").map((v) => +v.toFixed(6))).toEqual([99, 1]);
  expect(texts("[data-lab-position]")).toEqual(["Target: 1,020.00 (2.00%) 20.00", "Stop: 990.00 (1.00%) 10.00", "Risk/Reward Ratio: 2"]);
  await drag(g.querySelector('[data-part="p1"]')!, 20, 102, 20, 150);
  expect(texts('[data-lab-position="ratio"]')).toEqual(["Risk/Reward Ratio: 50"]);
  await drag(host.querySelector('[data-lab-drawing="long"] [data-part="p3"]')!, 220, 100, 50, 100);
  expect((last()[0] as { pts: [string, number][] }).pts[3][0]).toBe(d[5]);
  expect(Number(host.querySelector('[data-lab-drawing="long"] rect[fill="#089981"]')!.getAttribute("width"))).toBe(30);
});

it("Short position mirrors it: the target below the entry, the stop above", async () => {
  const { click, texts, last } = await layer([], "short");
  await click(20, 100);
  expect((last()[0] as { pts: [string, number][] }).pts.map((p) => +p[1].toFixed(6))).toEqual([1000, 980, 1010, 1000]);
  expect(texts("[data-lab-position]")).toEqual(["Target: 980.00 (2.00%) 20.00", "Stop: 1,010.00 (1.00%) 10.00", "Risk/Reward Ratio: 2"]);
});

it("Position forecast: an arrow from the first bar's close to the forecast point with the change", async () => {
  const { host, click, texts, last } = await layer([], "forecast");
  await click(0, 50); await click(40, 20);
  expect(last()).toEqual([{ kind: "forecast", pts: [[d[0], 500], [d[4], 200]] }]);
  expect(attr(host.querySelector('[data-lab-drawing="forecast"] line'), "x1", "y1", "x2", "y2")).toEqual([0, 10, 40, 20]); // starts at the close, 100
  expect(texts("[data-lab-forecast]")).toEqual(["100.00 (100.00%)", "4 bars, 4d"]);
});

it("Bars pattern copies the bars between the two points and moves the copy", async () => {
  const { host, click, drag, last } = await layer([], "barspat");
  await click(10, 100); await click(30, 100);
  expect(last()).toEqual([{ kind: "barspat", pts: [[d[1], 1000], [d[3], 1000]], props: { from: d[1], to: d[3], base: 1000 } }]);
  const wicks = () => [...host.querySelectorAll("[data-lab-ghost] line")].map((l) => attr(l, "x1", "y1", "y2"));
  expect(wicks()).toEqual([[10, 12, 10], [20, 13, 10.5], [30, 12.5, 11]]); // over the copied bars' highs and lows
  await drag(host.querySelector('[data-i="0"][data-part="line"]')!, 20, 11, 40, 21); // two bars right, 100 up
  expect(last()).toEqual([{ kind: "barspat", pts: [[d[3], 1100], [d[5], 1100]], props: { from: d[1], to: d[3], base: 1000 } }]);
  expect(wicks()).toEqual([[30, 22, 20], [40, 23, 20.5], [50, 22.5, 21]]);
});

it("Ghost feed steps a candle per bar along the clicked path, finishing on a double-click", async () => {
  const { host, click, ptr, last } = await layer([], "ghost");
  await click(0, 100); await click(30, 70); await click(30, 70); await ptr("dblclick", 30, 70);
  expect(last()).toEqual([{ kind: "ghost", pts: [[d[0], 1000], [d[3], 700]] }]);
  const bodies = [...host.querySelectorAll("[data-lab-ghost] rect")];
  expect(bodies.map((r) => [...attr(r, "x", "y", "height").map((v) => +v.toFixed(6)), r.getAttribute("fill")]))
    .toEqual([[7, 90, 10, "#F23645"], [17, 80, 10, "#F23645"], [27, 70, 10, "#F23645"]]);
});

it("Sector draws a wedge round the first point from the second point's radius to the third point's angle", async () => {
  const { host, click, texts, last } = await layer([], "sector");
  await click(0, 100); await click(20, 100); await click(20, 80);
  expect(last()).toEqual([{ kind: "sector", pts: [[d[0], 1000], [d[2], 1000], [d[2], 800]] }]);
  const r = 20, a = -Math.PI / 4;
  expect(host.querySelector('[data-lab-drawing="sector"] path')!.getAttribute("d")).toBe(`M0 100L20 100A20 20 0 0 0 ${r * Math.cos(a)} ${100 + r * Math.sin(a)}Z`);
  expect(texts("[data-lab-sector]")).toEqual(["−200.00 (−20.00%)", "2 bars, 2d"]);
});

it("Anchored VWAP weights the typical price by volume from the anchor to the last bar, skipping bars without volume", async () => {
  const vw = anchoredVwap({ d, ...bars }, 2);
  [115, 115, 8650 / 70, 15650 / 120].forEach((v, k) => expect(vw[k]).toBeCloseTo(v, 9));
  const { host, click, last } = await layer([], "avwap");
  await click(20, 50);
  expect(last()).toEqual([{ kind: "avwap", pts: [[d[2], 500]] }]);
  const path = host.querySelector("[data-lab-vwap]")!.getAttribute("d")!;
  expect(path.split("L").map((s) => s.replace("M", "").split(" ").map((n) => +(+n).toFixed(4)))).toEqual([[20, 11.5], [30, 11.5], [40, 12.3571], [50, 13.0417]]);
  expect(host.querySelector('[data-lab-drawing="avwap"] text')!.textContent).toBe("130.42");
  await unmount();
  const empty = await layer([{ kind: "avwap", pts: [[d[2], 500]] }], null, { ...bars, v: [] });
  expect(empty.host.querySelector("[data-lab-vwap-empty]")).not.toBeNull();
  expect(empty.host.querySelector('[data-i="0"][data-part="line"]')).not.toBeNull(); // still selectable
});

it("Fixed range volume profile: 24 rows over the bars' range, the POC line and a 70% value area", async () => {
  const vp = volumeProfile({ d, ...bars }, 0, 1)!;
  expect([vp.lo, vp.hi, vp.step, vp.poc, ...vp.va]).toEqual([90, 120, 1.25, 8, 8, 20]);
  const { host, click, last } = await layer([], "frvp");
  await click(0, 100); await click(10, 100);
  expect(last()).toEqual([{ kind: "frvp", pts: [[d[0], 1000], [d[1], 1000]] }]);
  expect(host.querySelectorAll("[data-lab-vp-row]")).toHaveLength(24);
  expect(host.querySelectorAll("[data-lab-vp-row][data-va]")).toHaveLength(13);
  expect(attr(host.querySelector("[data-lab-poc]"), "x1", "x2", "y1")).toEqual([0, 10, 10.0625]);
  expect(Number(host.querySelector('[data-lab-vp-row="8"] rect')!.getAttribute("width"))).toBe(3); // the POC row spans 30% of the range
});

it("Anchored volume profile runs from the anchor bar to the last bar", async () => {
  const { host, click } = await layer([], "avp");
  await click(40, 50);
  expect(host.querySelectorAll("[data-lab-vp-row]")).toHaveLength(24);
  const [x1, x2, y] = attr(host.querySelector("[data-lab-poc]"), "x1", "x2", "y1");
  expect([x1, x2]).toEqual([40, 50]);
  expect(y).toBeCloseTo((115 + 11.5 * 35 / 24) / 10, 9); // the row where both bars' ranges overlap
});

it("every forecasting tool can be selected and has its handles; old measurer drawings draw as before", async () => {
  const saved: Drawing[] = [
    { kind: "long", pts: [[d[1], 1000], [d[1], 1100], [d[1], 900], [d[4], 1000]] }, { kind: "short", pts: [[d[1], 1000], [d[1], 900], [d[1], 1100], [d[4], 1000]] },
    { kind: "forecast", pts: [[d[0], 500], [d[4], 200]] }, { kind: "barspat", pts: [[d[1], 1000], [d[3], 1000]], props: { from: d[1], to: d[3], base: 1000 } },
    { kind: "ghost", pts: [[d[0], 1000], [d[3], 700]] }, { kind: "sector", pts: [[d[0], 1000], [d[2], 1000], [d[2], 800]] },
    { kind: "avwap", pts: [[d[2], 500]] }, { kind: "frvp", pts: [[d[0], 1000], [d[1], 1000]] }, { kind: "avp", pts: [[d[4], 500]] },
    { kind: "pr", a: [d[0], 200], b: [d[2], 150] }, { kind: "dr", a: [d[0], 200], b: [d[5], 150] }, { kind: "dpr", a: [d[0], 100], b: [d[5], 110] }];
  const { host, texts } = await layer(saved);
  const handles = [4, 4, 2, 1, 2, 3, 1, 2, 1, 2, 2, 2];
  saved.forEach((_, i) => {
    expect(host.querySelector(`[data-i="${i}"][data-part="line"]`)).not.toBeNull();
    expect(host.querySelectorAll(`[data-i="${i}"][data-part^="p"]`)).toHaveLength(handles[i]);
  });
  expect(texts("[data-lab-measure]")).toEqual(["−50.00 (−25.00%)", "5 bars, 7d", "10.00 (10.00%)", "5 bars, 7d"]);
});
