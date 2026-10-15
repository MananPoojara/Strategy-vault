import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act } from "react";
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
  expect(host.querySelector('[data-lab-chip="c4"] select')?.textContent).toContain("AmiBroker");
  // the failed pane chip draws no chart; the stoch pane does
  expect(charts).toHaveLength(2);
  expect(host.querySelector<HTMLElement>("[data-lab-chart]")?.hidden).toBe(false);
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
  expect(charts.every((c) => c.remove.mock.calls.length === 1)).toBe(true);
});
