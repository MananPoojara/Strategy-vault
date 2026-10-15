import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act, useState } from "react";
import { Workbench, type Chip } from "../src/features/lab/Workbench";
import { catalogueFixture as cat, flush, mount, runFixture, seriesFixture, unmount } from "./fixtures/lab";

type Spy = { setData: ReturnType<typeof vi.fn>; setMarkers: ReturnType<typeof vi.fn>; applyOptions: ReturnType<typeof vi.fn> };
let charts: { remove: ReturnType<typeof vi.fn>; series: Spy[]; kinds: string[] }[] = [];
beforeEach(() => {
  charts = [];
  const make = () => {
    const c = { remove: vi.fn(), series: [] as Spy[], kinds: [] as string[] };
    charts.push(c);
    const add = (kind: string) => () => { const s = { setData: vi.fn(), setMarkers: vi.fn(), applyOptions: vi.fn() }; c.series.push(s); c.kinds.push(kind); return s; };
    return { addCandlestickSeries: add("candle"), addLineSeries: add("line"), addAreaSeries: add("area"), remove: c.remove, resize: vi.fn(),
      applyOptions: vi.fn(), timeScale: () => ({ fitContent: vi.fn(), setVisibleRange: vi.fn() }) };
  };
  window.LightweightCharts = { createChart: vi.fn(make) } as never;
});
afterEach(async () => { await unmount(); delete window.LightweightCharts; });

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

it("draws candles, overlays with gaps, boolean markers and a pane; a failing chip stays local", async () => {
  const host = await mount(<Workbench {...props} />);
  await flush();
  const price = charts[0];
  expect(price.kinds[0]).toBe("candle");
  expect(price.series[0].setData.mock.calls[0][0]).toHaveLength(6);
  const lines = price.series.slice(1).map((s) => s.setData.mock.calls[0][0]);
  expect(lines.some((points: { value?: number }[]) => points[0].value === undefined)).toBe(true);
  expect(price.series[0].setMarkers.mock.calls.at(-1)![0].filter((m: { text: string }) => m.text === "Doji")).toHaveLength(2);
  expect(charts.length).toBeGreaterThanOrEqual(2);
  expect(host.querySelector('[data-lab-chip="c5"] [data-lab-error]')?.textContent).toContain("rsi: needs more history");
  expect(host.querySelector<HTMLSelectElement>('[data-lab-chip="c4"] select')?.value).toBe("1");
  expect(host.querySelector('[data-lab-chip="c4"] select')?.textContent).toContain("AmiBroker");
  // the failed pane chip draws no chart; the stoch pane does
  expect(charts).toHaveLength(2);
  expect(host.querySelector("[data-lab-chart]")?.closest("[hidden]")).toBeNull();
  expect(host.querySelector("[data-lab-chart]")?.getAttribute("aria-label")).toBe("NIFTY 50 price chart");
  expect(host.querySelector('[data-lab-pane="c4"]')?.getAttribute("role")).toBe("img");
});

it("edits and removes chips through callbacks", async () => {
  const onChipChange = vi.fn(), onChipRemove = vi.fn();
  const host = await mount(<Workbench {...props} onChipChange={onChipChange} onChipRemove={onChipRemove} />);
  const input = host.querySelector<HTMLInputElement>('[data-lab-chip="c1"] input')!;
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
  expect(markers.some((m: { text: string }) => m.text === "Short")).toBe(true);
  // 2024-02-01 is not a week end; it moves to 2024-02-02, and markers stay time-sorted
  expect(markers.find((m: { text: string }) => m.text === "Short").time).toBe("2024-02-02");
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
  expect(host.textContent).toContain("No indicators on the chart");
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
  const input = host.querySelector<HTMLInputElement>('[data-lab-chip="c1"] input')!;
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

it("the provisional strip uses the note the server sent", async () => {
  const note = "Provisional · server note";
  const spy = vi.fn(async (q: { id: string }) => ({ ...(await load(q)), provisional: true as const, provisional_note: note }));
  const host = await mount(<Workbench {...props} market="gl" chips={[]} loadSeries={spy} instrumentLabel="IG:DAX" />);
  await flush();
  expect(host.querySelector("[data-lab-provisional]")?.textContent).toBe(`${note} · IG:DAX`);
});

it("keeps every pane on one time range and crosshair bar, and unsubscribes on unmount", async () => {
  type Handler = (r: { from: string; to: string } | null) => void;
  const subs: { range: Handler[]; cross: ((p: { time?: string }) => void)[]; setRange: ReturnType<typeof vi.fn>; setCross: ReturnType<typeof vi.fn>; unRange: ReturnType<typeof vi.fn>; unCross: ReturnType<typeof vi.fn> }[] = [];
  window.LightweightCharts = { createChart: vi.fn(() => {
    const s = { range: [] as Handler[], cross: [] as ((p: { time?: string }) => void)[], setRange: vi.fn(), setCross: vi.fn(), unRange: vi.fn(), unCross: vi.fn() };
    subs.push(s);
    const series = () => ({ setData: vi.fn(), setMarkers: vi.fn(), applyOptions: vi.fn() });
    return { addCandlestickSeries: series, addLineSeries: series, addAreaSeries: series, remove: vi.fn(), resize: vi.fn(), applyOptions: vi.fn(),
      subscribeCrosshairMove: (h: never) => s.cross.push(h), unsubscribeCrosshairMove: s.unCross, setCrosshairPosition: s.setCross, clearCrosshairPosition: vi.fn(),
      timeScale: () => ({ fitContent: vi.fn(), setVisibleRange: s.setRange, subscribeVisibleTimeRangeChange: (h: Handler) => s.range.push(h), unsubscribeVisibleTimeRangeChange: s.unRange }) };
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
  pane.range[0]({ from: "2024-01-01", to: "2025-01-01" });
  expect(price.setRange).toHaveBeenLastCalledWith({ from: "2024-01-01", to: "2025-01-01" });
  expect(pane.setRange.mock.calls.length).toBe(initial); // no feedback to the source
  pane.cross[0]({ time: "2024-02-01" });
  expect(price.setCross).toHaveBeenCalledWith(0, "2024-02-01", expect.anything());
  // a pane that is not under the pointer does not echo
  price.range[0]({ from: "2020-01-01", to: "2021-01-01" });
  expect(pane.setRange.mock.calls.length).toBe(initial);
  await unmount();
  expect(price.unRange).toHaveBeenCalledTimes(1);
  expect(pane.unRange).toHaveBeenCalledTimes(1);
  expect(pane.unCross).toHaveBeenCalledTimes(1);
});
