import { act, useState } from "react";
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { DrawingLayer, type Drawing } from "../src/features/lab/DrawingLayer";
import { GROUPS } from "../src/features/lab/drawings/registry";
import type { LabChartApi, LabSeriesApi } from "../src/features/lab/globals";
import { mount, unmount } from "./fixtures/lab";

afterEach(unmount);
// jsdom lays nothing out: a 500 px wide drawing layer (plot 450, axis 50), one bar every `bar` px, price p at y = p / 10
beforeAll(() => { Object.defineProperty(SVGElement.prototype, "clientWidth", { configurable: true, get: () => 500 }); });
const d = ["2024-01-01", "2024-01-02", "2024-01-03", "2024-01-04", "2024-01-05", "2024-01-08"];
const chartOf = (bar: number) => ({ priceScale: () => ({ width: () => 50 }), timeScale: () => ({ subscribeVisibleLogicalRangeChange: vi.fn(), unsubscribeVisibleLogicalRangeChange: vi.fn(),
  logicalToCoordinate: (l: number) => l * bar, coordinateToLogical: (x: number) => x / bar }) }) as unknown as LabChartApi;
const series = { priceToCoordinate: (p: number) => p / 10, coordinateToPrice: (y: number) => y * 10 } as unknown as LabSeriesApi;

async function layer(start: Drawing[], tool: string | null = null, bar = 10) {
  const saved: Drawing[][] = [];
  function Host() {
    const [drawings, set] = useState(start), [t, setTool] = useState(tool);
    return <div><DrawingLayer chart={chartOf(bar)} series={series} times={d} drawings={drawings} tool={t} color="#2962FF" height={300}
      onChange={(n) => { saved.push(n); set(n); }} onDone={() => setTool(null)} /></div>;
  }
  const host = await mount(<Host />);
  const svg = () => host.querySelector("[data-lab-draw-layer]")!;
  const ptr = (type: string, x: number, y: number, el: Element = svg()) =>
    act(async () => { el.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y })); });
  const click = async (x: number, y: number) => { await ptr("pointermove", x, y); await ptr("pointerdown", x, y); await ptr("pointerup", x, y); };
  const type = (text: string) => act(async () => {
    const box = host.querySelector<HTMLTextAreaElement>("[data-lab-draw-text]")!;
    box.value = text; box.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  });
  const g = (kind: string) => host.querySelector(`[data-lab-drawing="${kind}"]`)!;
  return { host, svg, ptr, click, type, g, last: () => saved[saved.length - 1] };
}
const texts = (el: Element, sel = "text") => [...el.querySelectorAll(sel)].map((t) => t.textContent);

it("lists TradingView's text and notes tools in its order, each selectable", async () => {
  const text = GROUPS.find((g) => g.id === "text")!.tools;
  expect(text.map((t) => t.name)).toEqual(["Text", "Anchored text", "Note", "Price note", "Pin", "Table", "Callout", "Comment", "Price label", "Signpost", "Flag mark"]);
  expect(new Set(text.map((t) => t.section))).toEqual(new Set(["TEXT AND NOTES"]));
  const { host } = await layer(text.map((t) => ({ kind: t.id, pts: [[d[2], 1000], [d[4], 500]], text: "x" })));
  text.forEach((_, i) => expect(host.querySelector(`[data-i="${i}"][data-part="line"]`)).not.toBeNull());
});

it("Text opens the editor with TradingView's default \"Text\"; Shift+Enter lines are drawn as lines", async () => {
  const { host, click, type, g, last } = await layer([], "text");
  await click(20, 100);
  expect(last()).toEqual([{ kind: "text", pts: [[d[2], 1000]], text: "Text" }]);
  expect(host.querySelector<HTMLTextAreaElement>("[data-lab-draw-text]")!.value).toBe("Text");
  await type("Breakout\nretest");
  expect(last()).toEqual([{ kind: "text", pts: [[d[2], 1000]], text: "Breakout\nretest" }]);
  expect(texts(g("text"), "tspan")).toEqual(["Breakout", "retest"]);
  expect(g("text").querySelector("text")!.getAttribute("x")).toBe("20"); // drawn from the point, where the editor opened
});

it("Anchored text keeps its screen position as fractions of the layer, whatever the bars' scale", async () => {
  const { click, ptr, g, last } = await layer([], "atext");
  await click(30, 60);
  expect(last()).toEqual([{ kind: "atext", pts: [[d[3], 600]], text: "Text", props: { fx: 30 / 450, fy: 60 / 300, t: d[3], p: 600 } }]);
  // drag it one bar right and 10 px down
  await ptr("pointerdown", 32, 50, g("atext").querySelector('[data-part="line"]')!); await ptr("pointermove", 42, 60); await ptr("pointerup", 42, 60);
  expect(g("atext").querySelector("text")!.getAttribute("x")).toBe("40");
  const a = last()[0];
  expect(a).toMatchObject({ pts: [[d[4], 700]] });
  await unmount();
  // the bars are now 20 px apart: a Text at that bar moves to x 80, the anchored one keeps 30 px plus the one-bar move
  const zoomed = await layer([a, { kind: "text", pts: [[d[4], 700]], text: "Text" }], null, 20);
  expect(zoomed.g("text").querySelector("text")!.getAttribute("x")).toBe("80");
  await zoomed.ptr("pointermove", 0, 0); // the layer is laid out on the next redraw
  expect(zoomed.g("atext").querySelector("text")!.getAttribute("x")).toBe("50");
});

it("Note and Pin show a marker and their text in a popup while hovered or selected", async () => {
  const { host, ptr, g } = await layer([{ kind: "note", pts: [[d[2], 1000]], text: "Earnings" }, { kind: "pin", pts: [[d[4], 1000]], text: "Gap" }]);
  const tip = (k: string) => g(k).querySelector("[data-lab-tip]")!;
  expect(tip("note").getAttribute("opacity")).toBe("0");
  expect(tip("note").querySelector("set")!.getAttribute("begin")).toBe("lab-tip-0.mouseover");
  expect(texts(tip("note"), "tspan")).toEqual(["Earnings"]);
  expect(texts(tip("pin"), "tspan")).toEqual(["Gap"]);
  expect(g("pin").querySelector("path")!.getAttribute("d")).toMatch(/^M40 100C/); // the pin's tip on the point
  await ptr("pointerdown", 20, 90, host.querySelector('[data-i="0"][data-part="line"]')!);
  expect(tip("note").getAttribute("opacity")).toBe("1");
});

it("Price note draws a line from the first point to a box with its price", async () => {
  const { click, g, last } = await layer([], "pnote");
  await click(10, 50); await click(40, 20);
  expect(last()).toEqual([{ kind: "pnote", pts: [[d[1], 500], [d[4], 200]] }]);
  const ln = g("pnote").querySelector("line")!;
  expect(["x1", "y1", "x2", "y2"].map((a) => ln.getAttribute(a))).toEqual(["10", "50", "40", "20"]);
  expect(texts(g("pnote").querySelector("[data-lab-label]")!)).toEqual(["500.00"]);
});

it("Table draws a 2x2 grid by default; rows are lines and cells are split by |", async () => {
  const { click, type, g, last } = await layer([], "table");
  await click(20, 100);
  expect(texts(g("table"), "[data-lab-cell]")).toEqual(["Text", "Text", "Text", "Text"]);
  await type("Entry|Stop|Target\n100|95");
  expect(last()[0]).toMatchObject({ kind: "table", text: "Entry|Stop|Target\n100|95" });
  expect(texts(g("table"), "[data-lab-cell]")).toEqual(["Entry", "Stop", "Target", "100", "95", ""]);
});

it("Callout points from its text bubble to the second point", async () => {
  const { click, type, g, last } = await layer([], "callout");
  await click(10, 100); await click(40, 200);
  await type("Support");
  expect(last()).toEqual([{ kind: "callout", pts: [[d[1], 1000], [d[4], 2000]], text: "Support" }]);
  expect(g("callout").querySelector("g path")!.getAttribute("d")).toContain("L40 200L");
  expect(texts(g("callout"), "tspan")).toEqual(["Support"]);
});

it("Comment, Price label, Signpost and Flag mark sit on their point", async () => {
  const { g } = await layer([{ kind: "comment", pts: [[d[1], 500]], text: "Comment" }, { kind: "plabel", pts: [[d[2], 1234]] },
    { kind: "signpost", pts: [[d[3], 1000]], text: "Signpost" }, { kind: "flag", pts: [[d[5], 800]] }]);
  expect(g("comment").querySelector("path")!.getAttribute("d")).toMatch(/^M10 50V/); // the bubble's square corner
  expect(texts(g("comment"), "tspan")).toEqual(["Comment"]);
  expect(g("plabel").querySelector("path")!.getAttribute("d")).toMatch(/^M20 123.4L/);
  expect(texts(g("plabel"))).toEqual(["1,234.00"]);
  const post = g("signpost").querySelector("line")!;
  expect([post.getAttribute("y1"), post.getAttribute("y2")]).toEqual(["100", "40"]);
  expect(texts(g("signpost"), "tspan")).toEqual(["Signpost"]);
  expect(g("flag").querySelector("path")!.getAttribute("d")).toBe("M50 54h16l-4 5 4 5H50Z");
});
