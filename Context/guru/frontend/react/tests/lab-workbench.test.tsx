import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import { act, useState } from "react";
import { Workbench, type Chip } from "../src/features/lab/Workbench";
import { variantOptions } from "../src/features/lab/ParamsDialog";
import { catalogueFixture as cat, dates, flush, mount, runFixture, seriesFixture, unmount } from "./fixtures/lab";

/** Chip settings live in the AmiBroker-style Parameters window; clicking a chip opens it. */
const openParams = (host: HTMLElement, key: string) => act(async () => host.querySelector<HTMLButtonElement>(`[data-lab-chip="${key}"] .lab-chip-open`)!.click());

type Spy = { setData: ReturnType<typeof vi.fn>; setMarkers: ReturnType<typeof vi.fn>; applyOptions: ReturnType<typeof vi.fn>; createPriceLine?: ReturnType<typeof vi.fn>; removePriceLine?: ReturnType<typeof vi.fn> };
const truthModule = { exports: {} };
runInNewContext(readFileSync("public/backtest-trust.js", "utf8"), { module: truthModule });
const truth = truthModule.exports as NonNullable<Window["VaultTruth"]>;

let charts: { remove: ReturnType<typeof vi.fn>; series: Spy[]; kinds: string[]; click?: (p: { time?: string; point?: { x: number; y: number } }) => void; crosshair?: (p: { time?: string }) => void; removeSeries: ReturnType<typeof vi.fn> }[] = [];
beforeEach(() => {
  charts = [];
  window.VaultTruth = truth;
  const make = () => {
    const c = { remove: vi.fn(), series: [] as Spy[], kinds: [] as string[], click: undefined as ((p: object) => void) | undefined, crosshair: undefined as ((p: { time?: string }) => void) | undefined, removeSeries: vi.fn() };
    charts.push(c);
    const add = (kind: string) => () => { const s = { setData: vi.fn(), setMarkers: vi.fn(), applyOptions: vi.fn(), createPriceLine: vi.fn((o: object) => o), removePriceLine: vi.fn(), coordinateToPrice: vi.fn((y: number) => y * 10), priceToCoordinate: (p: number) => p / 10 }; c.series.push(s); c.kinds.push(kind); return s; };
    return { addCandlestickSeries: add("candle"), addLineSeries: add("line"), addAreaSeries: add("area"), remove: c.remove, resize: vi.fn(), removeSeries: c.removeSeries,
      subscribeCrosshairMove: (h: (p: { time?: string }) => void) => { c.crosshair = h; },
      subscribeClick: (h: (p: object) => void) => { c.click = h; }, unsubscribeClick: () => { c.click = undefined; },
      applyOptions: vi.fn(), timeScale: () => ({ fitContent: vi.fn(), setVisibleRange: vi.fn(), coordinateToLogical: (x: number) => x / 10, logicalToCoordinate: (l: number) => l * 10 }) };
  };
  window.LightweightCharts = { createChart: vi.fn(make), CrosshairMode: { Normal: 0 } } as never;
});
afterEach(async () => { await unmount(); delete window.LightweightCharts; delete window.VaultTruth; });

const ohlc = (id: string) => seriesFixture(id, [id], { [id]: [1, 2, 3, 4, 5, 6] });
const load = vi.fn(async (q: { id: string }) => {
  if (["open", "high", "low", "close"].includes(q.id)) return ohlc(q.id);
  if (q.id === "fractals") return seriesFixture("fractals", ["up", "down"], { up: [null, 3, null, null, 5, null], down: [null, null, null, null, null, null] });
  if (q.id === "doji") return seriesFixture("doji", ["doji"], { doji: [0, 1, 0, 0, 1, 0] });
  if (q.id === "rsi") throw Object.assign(new Error("rsi: needs more history"), { status: 400, kind: "engine", engineStatus: "needs_data" });
  return seriesFixture(q.id, [q.id], { [q.id]: [null, 1, 2, 3, 4, 5] });
});
const chips: Chip[] = [
  { key: "c1", id: "ema", params: { length: 20 } }, { key: "c2", id: "fractals", params: {} },
  { key: "c3", id: "doji", params: {} }, { key: "c4", id: "stoch", params: { length: 14, variant: 1 } }, { key: "c5", id: "rsi", params: { length: 14 } },
];
const props = { market: "in" as const, ticker: "NIFTY 50", tf: "D", range: "All" as const, catalogue: cat, chips, loadSeries: load, onChipChange: vi.fn(), onChipRemove: vi.fn() };

it("draws candles at long daily ranges too", async () => {
  await mount(<Workbench {...props} chips={[]} />);
  await flush();
  expect(charts[0].kinds[0]).toBe("candle");
  expect(charts[0].series[0].setData.mock.calls[0][0]).toHaveLength(6);
});

it("draws candles, overlays with gaps, boolean markers and a pane; a failing chip stays local", async () => {
  const host = await mount(<Workbench {...props} range="1Y" />);
  await flush();
  const price = charts[0];
  expect(price.kinds[0]).toBe("candle");
  expect(price.series[0].setData.mock.calls[0][0]).toHaveLength(6);
  const lines = price.series.slice(1).map((s) => s.setData.mock.calls[0][0]);
  expect(lines.some((points: { value?: number }[]) => points[0].value === undefined)).toBe(true);
  expect(price.series[0].setMarkers.mock.calls.at(-1)![0].filter((m: { text: string }) => m.text === "Doji")).toHaveLength(2);
  expect(charts.length).toBeGreaterThanOrEqual(2);
  expect(host.querySelector('[data-lab-chip="c5"] [data-lab-error]')?.textContent).toContain("rsi: needs more history");
  await openParams(host, "c4");
  expect(host.querySelector<HTMLSelectElement>('.lab-params select')?.value).toBe("1");
  // the choices are the catalogue's own variant names, not a fixed TradingView/AmiBroker pair
  expect([...host.querySelectorAll('.lab-params select')[0].querySelectorAll("option")].map((o) => o.textContent)).toEqual(["TradingView", "AmiBroker ratio of sums"]);
  expect(host.querySelector(".lab-params [data-lab-ab-defaults]")?.textContent).toBe("AmiBroker defaults");
  // the failed pane chip draws no chart; the stoch pane does
  expect(charts).toHaveLength(2);
  expect(host.querySelector("[data-lab-chart]")?.closest("[hidden]")).toBeNull();
  expect(host.querySelector("[data-lab-chart]")?.getAttribute("aria-label")).toBe("NIFTY 50 price chart");
  expect(host.querySelector('[data-lab-pane="c4"]')?.getAttribute("role")).toBe("img");
});

it("edits and removes chips through callbacks", async () => {
  const onChipChange = vi.fn(), onChipRemove = vi.fn();
  const host = await mount(<Workbench {...props} onChipChange={onChipChange} onChipRemove={onChipRemove} />);
  await openParams(host, "c1");
  const input = host.querySelector<HTMLInputElement>('.lab-params input[type=number]')!;
  const type = (value: string) => act(async () => {
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    setter.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await type("30");
  expect(onChipChange).toHaveBeenCalledWith("c1", { length: 30 });
  await type("");
  await type("0");
  await type("9000");
  expect(onChipChange).toHaveBeenCalledTimes(1);
  await type("12.6");
  expect(onChipChange).toHaveBeenLastCalledWith("c1", { length: 13 });
  await act(async () => host.querySelector<HTMLButtonElement>('[aria-label="Remove Exponential Moving Average"]')!.click());
  expect(onChipRemove).toHaveBeenCalledWith("c1");
});

it("weekly view marks the last bar in progress and maps trades", async () => {
  const host = await mount(<Workbench {...props} tf="W" chips={[]} trades={runFixture.trades} />);
  await flush();
  expect(host.textContent).toContain("The last weekly bar is still in progress");
  expect(host.textContent).toContain("Trades run on daily bars and are mapped onto this timeframe.");
  const markers = charts[0].series[0].setMarkers.mock.calls.at(-1)![0];
  expect(markers.some((m: { text: string }) => m.text === "in progress")).toBe(true);
  expect(markers.some((m: { shape: string; position: string }) => m.shape === "arrowDown" && m.position === "aboveBar")).toBe(true);
  // 2024-02-01 is not a week end; it moves to 2024-02-02, and markers stay time-sorted
  expect(markers.find((m: { shape: string; position: string; time: string }) => m.shape === "arrowDown" && m.position === "aboveBar" && m.time === "2024-02-02").time).toBe("2024-02-02");
  const times = markers.map((m: { time: string }) => m.time);
  expect(times).toEqual([...times].sort());
  expect(charts[0].series[0].setData.mock.calls[0][0]).toHaveLength(4);
});

it("an OHLC failure replaces the chart with the error and draws nothing", async () => {
  const failing = vi.fn(async (q: { id: string }) => {
    if (q.id === "close") throw Object.assign(new Error("NIFTY 50 is not staged"), { status: 400, kind: "engine", engineStatus: "needs_data" });
    return load(q);
  });
  const host = await mount(<Workbench {...props} chips={[]} loadSeries={failing} market="gl" instrumentLabel="Provisional · TradingView export IG:DAX, daily" />);
  await flush();
  expect(charts).toHaveLength(0);
  expect(host.querySelector("[data-lab-chart]")?.closest("[hidden]")).not.toBeNull();
  expect(host.querySelector("[data-lab-error]")?.textContent).toContain("NIFTY 50 is not staged");
  expect(host.querySelector("[data-lab-provisional]")?.textContent).toContain("IG:DAX");
});

it("renders chips and notes without the chart library", async () => {
  delete window.LightweightCharts;
  const host = await mount(<Workbench {...props} chips={[]} />);
  await flush();
  expect(host.querySelectorAll("[data-lab-chart-label]")).toHaveLength(0);
  expect(host.textContent).toContain("Indicators are calculated from the full history");
  expect(host.textContent).toContain("NIFTY 50 · Daily bars");
});

it("removes charts on unmount", async () => {
  await mount(<Workbench {...props} />);
  await flush();
  await unmount();
  expect(charts.length).toBeGreaterThan(1);
  expect(charts.every((c) => c.remove.mock.calls.length === 1)).toBe(true);
});

it("chip edits reuse the cached OHLC; only changed chips are refetched", async () => {
  const spy = vi.fn(load);
  function Harness() {
    const [list, setList] = useState<Chip[]>([{ key: "c1", id: "ema", params: { length: 20 } }]);
    return <Workbench {...props} chips={list} loadSeries={spy}
      onChipChange={(key, params) => setList((l) => l.map((c) => (c.key === key ? { ...c, params } : c)))} />;
  }
  const host = await mount(<Harness />);
  await flush();
  await openParams(host, "c1");
  const input = host.querySelector<HTMLInputElement>('.lab-params input[type=number]')!;
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
  for (const v of ["2", "20", "200"]) {
    await act(async () => { setter.call(input, v); input.dispatchEvent(new Event("input", { bubbles: true })); });
    await flush();
  }
  expect(spy.mock.calls.filter(([q]) => q.id === "close")).toHaveLength(1);
  expect(spy.mock.calls.filter(([q]) => q.id === "ema").map(([q]) => (q as { p?: unknown }).p)).toContainEqual({ length: 200 });
});

it("a failed OHLC load is retried on the next run", async () => {
  let fail = true;
  const spy = vi.fn(async (q: { id: string }) => {
    if (q.id === "close" && fail) throw Object.assign(new Error("busy"), { status: 503, kind: "gateway" });
    return load(q);
  });
  function Harness() {
    const [tf, setTf] = useState("D");
    return <><button type="button" data-tf onClick={() => setTf("W")} /><Workbench {...props} chips={[]} tf={tf} loadSeries={spy} /></>;
  }
  const host = await mount(<Harness />);
  await flush();
  expect(charts).toHaveLength(0);
  fail = false;
  await act(async () => host.querySelector<HTMLButtonElement>("[data-tf]")!.click());
  await flush();
  expect(spy.mock.calls.filter(([q]) => q.id === "close")).toHaveLength(2);
  expect(charts).toHaveLength(1);
});

it("does not draw after unmount", async () => {
  let release!: () => void;
  const gate = new Promise<void>((r) => { release = r; });
  const slow = vi.fn(async (q: { id: string }) => { await gate; return load(q); });
  await mount(<Workbench {...props} loadSeries={slow} />);
  await unmount();
  release();
  await flush();
  expect(window.LightweightCharts!.createChart).not.toHaveBeenCalled();
});

it("a run superseded by a rerender draws nothing (its charts would never be disposed)", async () => {
  let release!: () => void;
  const gate = new Promise<void>((r) => { release = r; });
  const slow = vi.fn(async (q: { id: string }) => { await gate; return load(q); });
  function Harness() {
    const [tf, setTf] = useState("D");
    return <><button type="button" data-tf onClick={() => setTf("W")} /><Workbench {...props} chips={[]} tf={tf} loadSeries={slow} /></>;
  }
  const host = await mount(<Harness />);
  await act(async () => host.querySelector<HTMLButtonElement>("[data-tf]")!.click());
  release();
  await flush();
  expect(window.LightweightCharts!.createChart).toHaveBeenCalledTimes(1);
});

it("an indicator that is not computed is never requested and shows its note", async () => {
  const spy = vi.fn(load);
  const host = await mount(<Workbench {...props} chips={[{ key: "x1", id: "or_range", params: {} }]} loadSeries={spy} />);
  await flush();
  expect(spy.mock.calls.some(([q]) => q.id === "or_range")).toBe(false);
  const chip = host.querySelector('[data-lab-chip="x1"]')!;
  expect(chip.textContent).toContain("Needs intraday (minute) bars; not available yet.");
  expect(chip.querySelector("input")).toBeNull();
  expect(chip.tagName).toBe("DIV");
});

it("an indicator can use its own timeframe: weekly EMA on a daily chart, set from its Parameters window", async () => {
  const spy = vi.fn(load), onTf = vi.fn();
  const host = await mount(<Workbench {...props} chips={[{ key: "w1", id: "ema", params: { length: 20 }, tf: "W" }]} loadSeries={spy} onChipTfChange={onTf} />);
  await flush();
  expect((spy.mock.calls.find(([q]) => q.id === "ema")![0] as { tf?: string }).tf).toBe("W");
  expect((spy.mock.calls.find(([q]) => q.id === "close")![0] as { tf?: string }).tf).toBe("D"); // the candles stay daily
  expect(host.querySelector('[data-lab-chip="w1"]')!.textContent).toContain("Weekly");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-chip="w1"] .lab-chip-open')!.click());
  const select = host.querySelector<HTMLSelectElement>('select[aria-label$="· Timeframe"]')!;
  expect(select.value).toBe("W");
  await act(async () => { select.value = ""; select.dispatchEvent(new Event("change", { bubbles: true })); });
  expect(onTf).toHaveBeenCalledWith("w1", undefined);
});

it("draws horizontal and trend lines by press and drag with a live preview; lines move, reshape and delete; kept per instrument and timeframe", async () => {
  localStorage.removeItem("lab.drawings|in|NIFTY 50|D");
  const host = await mount(<Workbench {...props} chips={[]} />);
  await flush();
  const saved = () => JSON.parse(localStorage.getItem("lab.drawings|in|NIFTY 50|D") ?? "[]");
  const press = async (sel: string) => { await act(async () => host.querySelector<HTMLButtonElement>(sel)!.click()); await flush(); };
  const ptr = async (type: string, x: number, y: number, el: Element = host.querySelector("[data-lab-draw-layer]")!) =>
    act(async () => { el.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y })); });
  const d = dates;
  await press('[data-lab-draw="h"]');
  expect(host.querySelector("[data-lab-draw-hint]")!.textContent).toContain("click");
  await ptr("pointermove", 20, 100);
  expect(host.querySelector('[data-lab-draw-layer] [data-lab-drawing="h"] line')!.getAttribute("stroke-dasharray")).toBe("6 4"); // the preview follows the pointer
  await ptr("pointerdown", 20, 100); await ptr("pointerup", 20, 100);
  expect(saved()).toEqual([{ kind: "h", price: 1000 }]);
  expect(host.querySelector("[data-lab-draw-hint]")).toBeNull(); // one line per use of the tool, as TradingView
  await press('[data-lab-draw="t"]');
  await ptr("pointerdown", 40, 30);
  await ptr("pointermove", 10, 20);
  expect(host.querySelectorAll('[data-lab-draw-layer] [data-lab-drawing="t"]')).toHaveLength(1); // drawn while dragging
  await ptr("pointerup", 10, 20);
  expect(saved()[1]).toEqual({ kind: "t", a: [d[4], 300], b: [d[1], 200] });
  // click, click works too: the end follows the pointer until the second click
  await press('[data-lab-draw="t"]');
  await ptr("pointerdown", 0, 10); await ptr("pointerup", 0, 10);
  await ptr("pointermove", 20, 10);
  expect(saved()).toHaveLength(2);
  await ptr("pointerdown", 20, 10); await ptr("pointerup", 20, 10);
  expect(saved()[2]).toEqual({ kind: "t", a: [d[0], 100], b: [d[2], 100] });
  // drag the first trend line by its body one bar right and 100 up; drag its end; select and Delete removes it
  await ptr("pointerdown", 40, 30, host.querySelector('[data-i="1"][data-part="line"]')!);
  await ptr("pointermove", 50, 40); await ptr("pointerup", 50, 40);
  expect(saved()[1]).toEqual({ kind: "t", a: [d[5], 400], b: [d[2], 300] });
  await ptr("pointerdown", 20, 30, host.querySelector('[data-i="1"][data-part="b"]')!);
  await ptr("pointermove", 30, 50); await ptr("pointerup", 30, 50);
  expect(saved()[1]).toEqual({ kind: "t", a: [d[5], 400], b: [d[3], 500] });
  expect(host.querySelector("[data-lab-draw-selected]")!.textContent).toContain("Delete removes it");
  await act(async () => { window.dispatchEvent(new KeyboardEvent("keydown", { key: "Delete" })); });
  expect(saved()).toEqual([{ kind: "h", price: 1000 }, { kind: "t", a: [d[0], 100], b: [d[2], 100] }]);
  await press("[data-lab-draw-undo]");
  expect(saved()).toEqual([{ kind: "h", price: 1000 }]);
  await press("[data-lab-draw-clear]");
  expect(host.querySelector("[data-lab-draw-clear]")).toBeNull();
  expect(host.querySelectorAll("[data-lab-drawing]")).toHaveLength(0);
});

it("oscillator panes draw TradingView's threshold levels (Stochastic 80 / 20); the Properties window edits or removes them", async () => {
  const onLevels = vi.fn();
  const host = await mount(<Workbench {...props} chips={[chips[3]]} onChipLevels={onLevels} />);
  await flush();
  const paneLine = charts[1].series[0];
  expect(paneLine.createPriceLine!.mock.calls.map(([o]) => (o as { price: number }).price)).toEqual([80, 20]);
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-chip="c4"] .lab-chip-open')!.click());
  const field = host.querySelector<HTMLInputElement>('input[aria-label$="· Levels"]')!;
  expect(field.value).toBe("80, 20");
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(field, "75, 25");
    field.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await act(async () => field.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })));
  expect(onLevels).toHaveBeenLastCalledWith("c4", [75, 25]);
  await unmount();
  await mount(<Workbench {...props} chips={[{ ...chips[3], levels: [] }]} />);
  await flush();
  expect(charts.at(-1)!.series[0].createPriceLine).not.toHaveBeenCalled(); // [] = no lines
});

it("the provisional strip uses the note the server sent", async () => {
  const note = "Provisional · server note";
  const spy = vi.fn(async (q: { id: string }) => ({ ...(await load(q)), provisional: true as const, provisional_note: note }));
  const host = await mount(<Workbench {...props} market="gl" chips={[]} loadSeries={spy} instrumentLabel="IG:DAX" />);
  await flush();
  expect(host.querySelector("[data-lab-provisional]")?.textContent).toBe(`${note} · IG:DAX`);
});

it("keeps every pane on one bar range and crosshair bar, and unsubscribes on unmount", async () => {
  type Handler = (r: { from: number; to: number } | null) => void;
  const subs: { range: Handler[]; cross: ((p: { time?: string }) => void)[]; setRange: ReturnType<typeof vi.fn>; setCross: ReturnType<typeof vi.fn>; unRange: ReturnType<typeof vi.fn>; unCross: ReturnType<typeof vi.fn> }[] = [];
  window.LightweightCharts = { createChart: vi.fn(() => {
    const s = { range: [] as Handler[], cross: [] as ((p: { time?: string }) => void)[], setRange: vi.fn(), setCross: vi.fn(), unRange: vi.fn(), unCross: vi.fn() };
    subs.push(s);
    const series = () => ({ setData: vi.fn(), setMarkers: vi.fn(), applyOptions: vi.fn() });
    return { addCandlestickSeries: series, addLineSeries: series, addAreaSeries: series, remove: vi.fn(), resize: vi.fn(), applyOptions: vi.fn(),
      subscribeCrosshairMove: (h: never) => s.cross.push(h), unsubscribeCrosshairMove: s.unCross, setCrosshairPosition: s.setCross, clearCrosshairPosition: vi.fn(),
      timeScale: () => ({ fitContent: vi.fn(), setVisibleRange: s.setRange, setVisibleLogicalRange: s.setRange, subscribeVisibleLogicalRangeChange: (h: Handler) => s.range.push(h), unsubscribeVisibleLogicalRangeChange: s.unRange }) };
  }) } as never;
  const host = await mount(<Workbench {...props} chips={[chips[0], chips[3]]} />);
  await flush();
  expect(subs).toHaveLength(2);
  const [price, pane] = subs;
  const initial = price.setRange.mock.calls.length;
  expect(initial).toBeGreaterThan(0);
  expect(pane.setRange.mock.calls.length).toBe(initial);
  // the pane under the pointer drives the others
  host.querySelector('[data-lab-pane="c4"]')!.dispatchEvent(new Event("pointerenter"));
  pane.range[0]({ from: 2.5, to: 40 }); // bar positions: also covers the empty space after the last bar
  expect(price.setRange).toHaveBeenLastCalledWith({ from: 2.5, to: 40 });
  expect(pane.setRange.mock.calls.length).toBe(initial); // no feedback to the source
  pane.cross[0]({ time: "2024-02-01" });
  expect(price.setCross).toHaveBeenCalledWith(0, "2024-02-01", expect.anything());
  // a pane that is not under the pointer does not echo
  price.range[0]({ from: 0, to: 10 });
  expect(pane.setRange.mock.calls.length).toBe(initial);
  await unmount();
  expect(price.unRange).toHaveBeenCalledTimes(2); // the pane sync and the drawings layer
  expect(pane.unRange).toHaveBeenCalledTimes(1);
  expect(pane.unCross).toHaveBeenCalledTimes(1);
});

it("marks short entry above the bar and its cover below", async () => {
  const t = { ...runFixture.trades[0], dir: -1 as const };
  await mount(<Workbench {...props} chips={[]} trades={[t]} />);
  await flush();
  const markers = charts[0].series[0].setMarkers.mock.calls.at(-1)![0];
  expect(markers).toEqual([
    { time: "2024-01-08", position: "aboveBar", shape: "arrowDown", color: "#c43c3c", size: 1 },
    { time: "2024-01-15", position: "belowBar", shape: "arrowUp", color: "#2e7d4f", size: 1 },
  ]);
});
it("shows unique server series adaptations verbatim", async () => {
  const adapted = async (q: { id: string }) => ({ ...(await load(q)), adaptations: ["Future session dates are read."] });
  const host = await mount(<Workbench {...props} chips={chips.slice(0, 1)} loadSeries={adapted} />);
  await flush();
  expect([...host.querySelectorAll("[data-lab-adaptations]")].map((n) => n.textContent)).toEqual(["Future session dates are read."]);
});


it("inherits the software font and price-chart interaction options", async () => {
  const previousFont = document.body.style.fontFamily;
  document.body.style.fontFamily = '"House Sans", sans-serif';
  try {
    await mount(<Workbench {...props} chips={[]} />);
    await flush();
    const options = vi.mocked(window.LightweightCharts!.createChart).mock.calls[0][1];
    expect(options).toMatchObject({
      layout: { attributionLogo: false, fontFamily: getComputedStyle(document.body).fontFamily, fontSize: 11, textColor: "#8b8d93" },
      grid: { vertLines: { visible: true }, horzLines: { color: "#e3e3e4" } },
      timeScale: { minBarSpacing: 0.05 }, crosshair: { mode: 0 }, handleScale: { axisPressedMouseMove: true, mouseWheel: true }, handleScroll: { mouseWheel: true, pressedMouseMove: true },
    });
  } finally { document.body.style.fontFamily = previousFont; }
});

it("uses the shared action arrows for every scale-in leg, preserving short covers on weekly bars", async () => {
  const trade = { ...runFixture.trades[0], dir: -1 as const,
    legs: [["2024-01-08", 101, 0.5], ["2024-01-09", 103, 0.5]] as [string, number, number][] };
  const original = structuredClone(trade);
  const host = await mount(<Workbench {...props} chips={[]} tf="W" trades={[trade]} />);
  await flush();
  const actions = charts[0].series[0].setMarkers.mock.calls.at(-1)![0].filter((m: { shape: string }) => m.shape !== "square");
  expect(actions).toEqual([
    { time: "2024-01-09", position: "aboveBar", shape: "arrowDown", color: "#c43c3c", size: 1 },
    { time: "2024-01-09", position: "aboveBar", shape: "arrowDown", color: "#c43c3c", size: 0.7 },
    { time: "2024-01-15", position: "belowBar", shape: "arrowUp", color: "#2e7d4f", size: 1 },
  ]);
  expect(trade).toEqual(original);
  expect(host.textContent).toContain("Buy");
  expect(host.textContent).toContain("Sell");
});

it("omits actions outside the available price dates without fabricating a bar or hiding valid legs", async () => {
  const trades = [
    { ...runFixture.trades[0], entry: "2023-12-29", exit: "2024-01-15",
      legs: [["2023-12-29", 98, 0.5], ["2024-01-09", 103, 0.5]] as [string, number, number][] },
    { ...runFixture.trades[0], entry: "2024-01-10", exit: "2024-02-05" },
  ];
  await mount(<Workbench {...props} chips={[]} trades={trades} />);
  await flush();
  expect(charts[0].series[0].setMarkers.mock.calls.at(-1)![0]).toEqual([
    { time: "2024-01-09", position: "belowBar", shape: "arrowUp", color: "#2e7d4f", size: 0.7 },
    { time: "2024-01-15", position: "aboveBar", shape: "arrowDown", color: "#c43c3c", size: 1 },
  ]);
});

it("waits for an explicit pair comparison and keys requests by the selected instrument", async () => {
  const pair={...cat.indicators[0],id:"ratio",name:"Ratio",placement:"pane" as const,params:[],outputs:["value"],computed:true,needs:{...cat.indicators[0].needs,second_symbol:true}};
  const catalogue={...cat,max_foreign:4,indicators:[...cat.indicators,pair]};const onVs=vi.fn();
  const pairLoad=vi.fn(async(q:{id:string;vs?:string})=>q.id==="ratio"?seriesFixture("ratio",["value"],{value:[2,2,2,2,2,2]}):ohlc(q.id));
  const options=[{ticker:"SPX",label:"S&P 500",disabled:false,hasVolume:false}];
  const host=await mount(<Workbench {...props} market="us" ticker="SPY" catalogue={catalogue} chips={[{key:"p",id:"ratio",params:{}}]}
    loadSeries={pairLoad} comparisonOptions={options} onChipVsChange={onVs} />);
  await flush();expect(pairLoad.mock.calls.some(([q])=>q.id==="ratio")).toBe(false);
  await openParams(host,"p");
  const comparison=host.querySelector<HTMLSelectElement>('[aria-label="Ratio · Compared with"]');expect(comparison).not.toBeNull();
  await act(async()=>{comparison!.value="SPX";comparison!.dispatchEvent(new Event("change",{bubbles:true}));});
  expect(onVs).toHaveBeenCalledWith("p","SPX");
  await unmount();
  await mount(<Workbench {...props} market="us" ticker="SPY" catalogue={catalogue} chips={[{key:"p",id:"ratio",params:{},vs:"SPX"}]}
    loadSeries={pairLoad} comparisonOptions={options} onChipVsChange={onVs} />);await flush();
  expect(pairLoad.mock.calls.some(([q])=>q.id==="ratio"&&q.vs==="SPX")).toBe(true);
});

it("suppresses unsupported pair queries on older engines, non-daily charts and unavailable comparisons", async () => {
  const pair={...cat.indicators[0],id:"ratio",name:"Ratio",placement:"pane" as const,params:[],outputs:["value"],computed:true,needs:{...cat.indicators[0].needs,second_symbol:true}};
  for (const [max_foreign,tf,vs] of [[undefined,"D","SPX"],[0,"D","SPX"],[-1,"D","SPX"],[4,"W","SPX"],[4,"D","UNKNOWN"]] as const) {
    const pairLoad=vi.fn(async(q:{id:string})=>ohlc(q.id));
    const host=await mount(<Workbench {...props} catalogue={{...cat,max_foreign,indicators:[pair]}} tf={tf} chips={[{key:"p",id:"ratio",params:{},vs}]}
      loadSeries={pairLoad} comparisonOptions={[{ticker:"SPX",label:"S&P 500",disabled:false,hasVolume:false}]} />);await flush();
    expect(pairLoad.mock.calls.some(([q])=>q.id==="ratio")).toBe(false);expect(host.querySelector('[data-lab-pane="p"]')).toBeNull();
    if (!(max_foreign && max_foreign>0)) expect(host.querySelector('[aria-label="Ratio · Compared with"]')).toBeNull();
    await unmount();
  }
});

it("changing only vs redraws a mounted pair chip while reusing OHLC requests", async () => {
  const pair={...cat.indicators[0],id:"ratio",name:"Ratio",placement:"pane" as const,params:[],outputs:["value"],computed:true,needs:{...cat.indicators[0].needs,second_symbol:true}};
  const catalogue={...cat,max_foreign:4,indicators:[pair]};const options=["SPX","NDX"].map(ticker=>({ticker,label:ticker,disabled:false,hasVolume:false}));
  const pairLoad=vi.fn(async(q:{id:string;vs?:string})=>q.id==="ratio"?seriesFixture("ratio",["value"],{value:[2,2,2,2,2,2]}):ohlc(q.id));
  function Harness(){const[vs,setVs]=useState<string|undefined>("SPX");return <Workbench {...props} catalogue={catalogue} chips={[{key:"p",id:"ratio",params:{},vs}]} loadSeries={pairLoad} comparisonOptions={options} onChipVsChange={(_,next)=>setVs(next)} />;}
  const host=await mount(<Harness/>);await flush();await openParams(host,"p");
  const select=host.querySelector<HTMLSelectElement>('[aria-label="Ratio · Compared with"]')!;
  await act(async()=>{select.value="NDX";select.dispatchEvent(new Event("change",{bubbles:true}));});await flush();
  expect(pairLoad.mock.calls.filter(([q])=>q.id==="ratio").map(([q])=>q.vs)).toEqual(["SPX","NDX"]);
  expect(pairLoad.mock.calls.filter(([q])=>q.id==="open")).toHaveLength(1);
});

it("Parameters window: opens for a newly added indicator, previews looks, Defaults resets and Escape cancels", async () => {
  const onLook = vi.fn(), onChange = vi.fn();
  function Harness() {
    const [list, setList] = useState<Chip[]>([{ key: "c1", id: "ema", params: { length: 20 } }]);
    return <>
      <button type="button" data-add onClick={() => setList((l) => [...l, { key: "c2", id: "ema", params: { length: 33 } }])}>add</button>
      <Workbench {...props} chips={list} onChipChange={onChange} onChipLook={onLook} />
    </>;
  }
  const host = await mount(<Harness />);
  await flush();
  expect(host.querySelector(".lab-params")).toBeNull(); // chips present at mount do not pop a window
  await act(async () => host.querySelector<HTMLButtonElement>("[data-add]")!.click());
  const dialog = host.querySelector<HTMLElement>(".lab-params")!;
  expect(dialog.getAttribute("role")).toBe("region");
  expect(dialog.textContent).toContain("Exponential Moving Average");
  expect(host.querySelector<HTMLInputElement>(".lab-params input[type=number]")!.value).toBe("33");
  const style = host.querySelector<HTMLSelectElement>('.lab-params select[aria-label$="Style"]')!;
  await act(async () => { style.value = "dashed"; style.dispatchEvent(new Event("change", { bubbles: true })); });
  expect(onLook).toHaveBeenLastCalledWith("c2", { ema: { style: "dashed" } }); // per plot (output)
  await act(async () => [...dialog.querySelectorAll("button")].find((b) => b.textContent === "Defaults")!.click());
  expect(onChange).toHaveBeenLastCalledWith("c2", { length: cat.indicators.find((i) => i.id === "ema")!.params[0].default });
  expect(onLook).toHaveBeenLastCalledWith("c2", undefined);
  await act(async () => dialog.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
  expect(host.querySelector(".lab-params")).toBeNull();
});

it("trade spans shade won / still-open trades behind the price, and the legend lists each chip", async () => {
  const make = (window.LightweightCharts as unknown as { createChart: (...a: unknown[]) => Record<string, unknown> }).createChart;
  window.LightweightCharts = { CrosshairMode: { Normal: 0 }, createChart: vi.fn((...a: unknown[]) => {
    const chart = make(...a), c = charts.at(-1)!;
    return { ...chart, priceScale: () => ({ applyOptions: vi.fn() }), addHistogramSeries: () => {
      const s = { setData: vi.fn(), setMarkers: vi.fn(), applyOptions: vi.fn() }; c.series.push(s); c.kinds.push("histogram"); return s;
    } };
  }) } as never;
  const trades = [
    { ...runFixture.trades[0], entry: "2024-01-08", exit: "2024-01-09", ret: 0.05, open: false, dir: 1 as const },
    { ...runFixture.trades[0], entry: "2024-02-01", exit: "2024-02-02", ret: -0.01, open: true, dir: -1 as const },
  ];
  const host = await mount(<Workbench {...props} chips={[chips[0]]} trades={trades} />);
  await flush();
  const price = charts[0], spans = price.series[price.kinds.indexOf("histogram")];
  expect(price.kinds.indexOf("histogram")).toBeLessThan(price.kinds.indexOf("candle")); // behind the candles
  const shade = (spans.setData.mock.calls[0][0] as { time: string; color?: string }[]).map((p) => p.color ?? null);
  // won (green tint) on its two bars; the open short (grey) runs to the last bar; never shaded outside a trade
  expect(shade).toEqual([null, "#2e7d4f14", "#2e7d4f14", null, "#8b8d9314", "#8b8d9314"]);
  const legend = host.querySelector(".lab-legend-tv")!.textContent!;
  expect(legend).toContain("NIFTY 50 · Daily · 2024-02-02");
  expect(legend).toContain("EMA(20) = 5.00"); // AmiBroker title: short name(settings) = value
});

it("stacked panes: each pane reads out in its own legend and only the bottom chart draws the date axis", async () => {
  const host = await mount(<Workbench {...props} chips={[chips[0], chips[3]]} />);
  await flush();
  const legends = [...host.querySelectorAll(".lab-legend-tv")].map((el) => el.textContent);
  expect(legends[0]).toContain("EMA(20) = ");
  expect(legends[0]).not.toContain("STOCH");
  expect(host.querySelector('[data-lab-pane="c4"]')!.parentElement!.querySelector(".lab-legend-tv")!.textContent).toBe("STOCH(14, AmiBroker ratio of sums) = 5.00");
  const made = vi.mocked(window.LightweightCharts!.createChart).mock.results.map((r) => r.value.applyOptions as ReturnType<typeof vi.fn>);
  expect(made[0]).toHaveBeenCalledWith({ timeScale: { visible: false } });
  expect(made.at(-1)).toHaveBeenCalledWith({ timeScale: { visible: true } });
  expect(vi.mocked(window.LightweightCharts!.createChart).mock.calls[0][1]).toMatchObject({ rightPriceScale: { minimumWidth: 78 } });
});

it("Parameters: full-range numeric fields, working Inputs and Style tabs, colours named by role", async () => {
  const host = await mount(<Workbench {...props} chips={[{ key: "c1", id: "ema", params: { length: 20 } }, { key: "c2", id: "ema", params: { length: 9 }, looks: { ema: { color: "bh" } } }]} />);
  await flush();
  await openParams(host, "c1");
  expect(host.querySelector('.lab-params input[type=number]')!.getAttribute("max")).toBe("5000");
  expect(host.querySelector('.lab-params input[type=range]')).toBeNull();
  expect([...host.querySelectorAll('.lab-params-tabs [role="tab"]')].map((t) => t.textContent)).toEqual(["Inputs", "Style"]);
  const color = () => host.querySelector<HTMLSelectElement>('.lab-params select[aria-label$="Color"]')!;
  expect([...color().options].map((o) => o.textContent)).toEqual(["Theme main", "Red", "Green", "Theme accent", "Dark grey", "Custom…"]);
  await openParams(host, "c2");
  expect(color().value).toBe("bh"); // a colour saved before stays selectable
  expect(color().selectedOptions[0].textContent).toBe("Muted");
});

it("Parameters: typed values preview live, looks redraw without refetching, each chip gets a fresh window", async () => {
  const onChange = vi.fn();
  function Harness() {
    const [list, setList] = useState<Chip[]>([{ key: "c1", id: "ema", params: { length: 20 } }, { key: "c2", id: "macd", params: { fast: 12, slow: 26, signal: 9 } }]);
    return <Workbench {...props} chips={list} onChipChange={(key, params) => { onChange(key, params); setList((l) => l.map((c) => (c.key === key ? { ...c, params } : c))); }}
      onChipLook={(key, looks) => setList((l) => l.map((c) => (c.key === key ? { ...c, looks } : c)))} />;
  }
  load.mockClear();
  const host = await mount(<Harness />);
  await flush();
  const emaCalls = () => load.mock.calls.filter(([q]) => q.id === "ema").length;
  expect(emaCalls()).toBe(1);
  await openParams(host, "c1");
  const number = host.querySelector<HTMLInputElement>('.lab-params input[type=number]')!;
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(number, "23"); number.dispatchEvent(new Event("input", { bubbles:true })); });
  await flush();
  expect(onChange.mock.calls).toEqual([["c1", { length:23 }]]);
  expect(emaCalls()).toBe(2);
  const style = host.querySelector<HTMLSelectElement>('.lab-params select[aria-label$="Style"]')!;
  await act(async () => { style.value = "dotted"; style.dispatchEvent(new Event("change", { bubbles: true })); });
  await flush();
  expect(emaCalls()).toBe(2); // the look changed, the data did not: served from the series cache
  await openParams(host, "c2");
  expect(host.querySelector(".lab-params")!.textContent).toContain("MACD");
  expect(host.querySelector(".lab-params-sec")).toBeNull(); // fields are directly visible, without a duplicate heading
  expect(host.querySelector(".lab-params [role=tablist]")).not.toBeNull();
  // one Color / Style pair per plot, with different default colours
  const colors = [...host.querySelectorAll<HTMLSelectElement>('.lab-params select[aria-label$="Color"]')];
  expect(colors.map((c) => c.getAttribute("aria-label"))).toEqual(["MACD macd Color", "MACD signal Color", "MACD hist Color"]);
  expect(new Set(colors.map((c) => c.value)).size).toBe(3);
});

it("Parameters: Custom… reveals a colour picker; the hex is kept in the chip look and drawn on the plot", async () => {
  const make = (window.LightweightCharts as unknown as { createChart: (...a: unknown[]) => Record<string, unknown> }).createChart, lineColors: unknown[] = [];
  window.LightweightCharts = { CrosshairMode: { Normal: 0 }, createChart: vi.fn((...a: unknown[]) => {
    const chart = make(...a), add = chart.addLineSeries as (o: { color: string }) => unknown;
    return { ...chart, addLineSeries: (o: { color: string }) => { lineColors.push(o.color); return add(o); } };
  }) } as never;
  const onLook = vi.fn();
  function Harness() {
    const [list, setList] = useState<Chip[]>([{ key: "c1", id: "ema", params: { length: 20 } }]);
    return <Workbench {...props} chips={list} onChipLook={(key, looks) => { onLook(key, looks); setList((l) => l.map((c) => (c.key === key ? { ...c, looks } : c))); }} />;
  }
  const host = await mount(<Harness />);
  await flush();
  await openParams(host, "c1");
  expect(host.querySelector(".lab-params input[type=color]")).toBeNull(); // theme colours stay the default
  const color = host.querySelector<HTMLSelectElement>('.lab-params select[aria-label$="Color"]')!;
  await act(async () => { color.value = "custom"; color.dispatchEvent(new Event("change", { bubbles: true })); });
  expect(onLook).toHaveBeenLastCalledWith("c1", { ema: { color: "#2f5a8f" } }); // starts from the theme colour shown
  const picker = host.querySelector<HTMLInputElement>('.lab-params input[type=color]')!;
  expect(color.value).toBe("custom");
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(picker, "#ff8800"); picker.dispatchEvent(new Event("input", { bubbles: true })); });
  await flush();
  expect(onLook).toHaveBeenLastCalledWith("c1", { ema: { color: "#ff8800" } });
  expect(lineColors.at(-1)).toBe("#ff8800");
  expect(host.querySelector<HTMLElement>('[data-lab-chip="c1"] .lab-dot')!.style.background).toBe("rgb(255, 136, 0)");
  await act(async () => { color.value = "red"; color.dispatchEvent(new Event("change", { bubbles: true })); });
  expect(onLook).toHaveBeenLastCalledWith("c1", { ema: { color: "red" } });
  expect(host.querySelector(".lab-params input[type=color]")).toBeNull();
});

it("variant choices follow the catalogue label; an unnamed value shows its number", () => {
  const p = (label: string, max: number) => ({ key: "variant", label, default: 0, min: 0, max, step: 1, integer: true });
  expect(variantOptions(p("Variant (0 Wilder/TradingView/AmiBroker after warm-up, 1 Cutler/TradeStation RSI JP)", 1)))
    .toEqual([[0, "Wilder/TradingView/AmiBroker after warm-up"], [1, "Cutler/TradeStation RSI JP"]]);
  expect(variantOptions(p("Variant (0 % of MA, 1 % of close (NinjaTrader), 2 close/MA×100 (TradeStation))", 2)).map(([, t]) => t))
    .toEqual(["% of MA", "% of close (NinjaTrader)", "close/MA×100 (TradeStation)"]);
  expect(variantOptions(p("Variant", 2))).toEqual([[0, "0"], [1, "1"], [2, "2"]]);
});

it("switches the floating window between the library and settings without remounting the chart", async () => {
  const host = await mount(<Workbench {...props} library={<p data-browser>Indicator library</p>} />);
  await flush();
  const chart = host.querySelector('[data-lab-chart]');
  const count = charts.length;
  await act(async () => [...host.querySelectorAll<HTMLButtonElement>('.lab-chart-actions button')].find(b => b.textContent === 'Indicators')!.click());
  expect(host.querySelector('[role="dialog"][aria-label="Indicators"] [data-browser]')).not.toBeNull();
  await openParams(host, 'c1');
  expect(host.querySelector('.lab-indicator-browser')!.hasAttribute('hidden')).toBe(true);
  expect(host.querySelector('.lab-params')!.getAttribute('role')).toBe('region');
  await act(async () => host.querySelector<HTMLButtonElement>('[aria-label="Close parameters"]')!.click());
  expect(host.querySelector('.lab-chart-dock-panel')).toBeNull();
  expect(host.querySelector('[data-lab-chart]')).toBe(chart);
  expect(charts).toHaveLength(count);
  expect(charts.every(c => c.remove.mock.calls.length === 0)).toBe(true);
});

it("opens indicator settings directly from its oscillator pane", async () => {
  const host = await mount(<Workbench {...props} />);
  await flush();
  const settings = host.querySelector<HTMLButtonElement>('.lab-pane-box .lab-indicator-actions button[aria-label^="Indicator settings"]')!;
  expect(settings.getAttribute('aria-label')).toContain('Indicator settings for');
  await act(async () => settings.click());
  expect(host.querySelector('.lab-params')).not.toBeNull();
});

it("updates on-chart values on hover with one popup-based display", async () => {
  const host = await mount(<Workbench {...props} />);
  await flush();
  const label = host.querySelector('[data-lab-chart-label="c1"] .lab-indicator-values')!;
  expect(label.textContent).toBe('5.00');
  charts[0].crosshair?.({ time: seriesFixture('ema', ['ema'], { ema:[null, 1, 2, 3, 4, 5] }).dates[1] });
  expect(label.textContent).toBe('1.00');
  expect(host.querySelector('[aria-label="Indicator display"]')).toBeNull();
  expect(host.querySelector('[aria-label="Settings window"]')).toBeNull();
  expect(host.querySelector('.lab-workbench')!.getAttribute('data-indicator-display')).toBe('on-chart');
});

it("hides lines and pattern markers without changing parameters, refetching or resetting the charts", async () => {
  load.mockClear();
  const change = vi.fn();
  const host = await mount(<Workbench {...props} onChipChange={change} />);
  await flush();
  const count = charts.length, calls = load.mock.calls.length, ema = charts[0].series[1];
  const eye = host.querySelector<HTMLButtonElement>('[data-lab-chart-label="c1"] [aria-pressed]')!;
  await act(async () => eye.click());
  expect(eye.getAttribute('aria-pressed')).toBe('true');
  expect(ema.applyOptions).toHaveBeenLastCalledWith({ visible:false });
  await act(async () => eye.click());
  expect(ema.applyOptions).toHaveBeenLastCalledWith({ visible:true });
  const patternEye = host.querySelector<HTMLButtonElement>('[data-lab-chart-label="c3"] [aria-pressed]')!;
  await act(async () => patternEye.click());
  expect(charts[0].series[0].setMarkers!.mock.calls.at(-1)![0].some((m: {text:string}) => m.text === 'Doji')).toBe(false);
  expect(charts).toHaveLength(count);
  expect(load.mock.calls).toHaveLength(calls);
  expect(change).not.toHaveBeenCalled();
});


it("shows default input choices in the chart legend and opens their full settings", async () => {
  const ema = cat.indicators.find((i) => i.id === "ema")!;
  const catalogue = { ...cat, indicators: cat.indicators.map((i) => i.id === "ema" ? { ...i, params: [...i.params, { key:"source", label:"Source (0 close, 1 open)", default:0, min:0, max:1, integer:true, step:1 }] } : i) };
  const host = await mount(<Workbench {...props} catalogue={catalogue} />);
  await flush();
  expect(host.querySelector('[data-lab-chart-label="c1"] .lab-indicator-settings')!.textContent).toContain("close");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-chart-label="c1"] .lab-indicator-name')!.click());
  expect(host.querySelector('.lab-params select[aria-label="' + ema.name + ' Source (0 close, 1 open)"]')).not.toBeNull();
});
