import { afterEach, expect, it, vi } from "vitest";
import { act, useState } from "react";
import { ExplorePanel, csvOf, type ExplorePanelProps } from "../src/features/lab/Explore";
import type { ExploreResult, ScanResult } from "../src/features/lab/client";
import { defaultSpec } from "../src/features/lab/spec-model";
import type { Chip } from "../src/features/lab/Workbench";
import { catalogueFixture as cat, flush, mount, seriesFixture, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); });
// Download Excel: keep the workbook rows and the file name instead of saving a file
const saved = vi.hoisted(() => ({ sheets: [] as { name: string; rows: unknown[][] }[], file: "" }));
vi.mock("../src/features/lab/xlsx", async (orig) => ({ ...(await orig<typeof import("../src/features/lab/xlsx")>()),
  xlsx: (sheets: { name: string; rows: unknown[][] }[]) => { saved.sheets = sheets; return new Uint8Array(); },
  saveBytes: (_b: Uint8Array, name: string) => { saved.file = name; } }));
const LABEL = "User-defined indicator strategy · simulated · not a book method";
const scanFixture = (n: number): ScanResult => ({
  label: LABEL, kind: "lab_scan", apply: "current", from: "2024-01-01", to: "2024-12-31", notes: [], symbols: 1, refused: [],
  rows: Array.from({ length: n }, (_, i) => ["NIFTY 50", `2024-01-${String(1 + (i % 28)).padStart(2, "0")}`, i % 2 ? "Sell" : "Buy", 100 + ((i * 37) % 50)]),
});
const exploreFixture: ExploreResult = {
  label: LABEL, kind: "lab_explore", apply: "current", from: "2024-01-01", to: "2024-01-03", notes: [],
  dates: ["2024-01-01", "2024-01-02", "2024-01-03"], open: [1, 2, 3], high: [2, 3, 4], low: [0.5, 1.5, 2.5], close: [1.5, 2.5, 3.5], volume: [100, 200, 300],
  signals: { buy: [0, 1, 0], sell: [0, 0, 1], short: [0, 0, 0], cover: [0, 0, 0] },
};
const props = (patch: Partial<ExplorePanelProps> = {}): ExplorePanelProps => ({
  spec: defaultSpec(), catalogue: cat, chips: [], portfolio: false, market: "in", ticker: "NIFTY 50",
  onScan: vi.fn().mockResolvedValue(scanFixture(3)), onExplore: vi.fn().mockResolvedValue(exploreFixture), loadSeries: vi.fn().mockRejectedValue(new Error("no data")), ...patch,
});
const click = (el: Element | null) => act(async () => { (el as HTMLElement).click(); });
const pick = (host: HTMLElement, label: string) => [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === label)!;
const table = (host: HTMLElement, name: string) => [...host.querySelectorAll(`[data-lab-analysis="${name}"] tbody tr`)].map((r) => [...r.children].map((c) => c.textContent));
const heads = (host: HTMLElement, name: string) => [...host.querySelectorAll(`[data-lab-analysis="${name}"] th`)].map((c) => c.textContent);

it("Explore only: no Scan choice, no fold-out header, no explanatory note", async () => {
  const host = await mount(<ExplorePanel {...props()} />);
  expect(host.querySelector("[data-lab-explore]")!.tagName).toBe("SECTION"); // always open
  expect(host.querySelector("summary")).toBeNull();
  expect(host.querySelector('[data-lab-action="scan"]')).toBeNull();
  expect([...host.querySelectorAll("button")].some((b) => b.textContent === "Scan")).toBe(false);
  expect(host.textContent).not.toContain("not filled trades");
  expect(host.querySelector('[data-lab-action="explore"]')).not.toBeNull();
});
it("sorts by a clicked column (again reverses), pages 100 rows and copies every row as CSV", async () => {
  const n = 150, dates = Array.from({ length: n }, (_, i) => new Date(Date.UTC(2024, 0, 1 + i)).toISOString().slice(0, 10));
  const close = Array.from({ length: n }, (_, i) => 100 + ((i * 37) % 50));
  const big: ExploreResult = { ...exploreFixture, dates, open: close, high: close, low: close, close, volume: null,
    signals: { buy: dates.map(() => 0), sell: dates.map(() => 0), short: dates.map(() => 0), cover: dates.map(() => 0) } };
  const host = await mount(<ExplorePanel {...props({ onExplore: vi.fn().mockResolvedValue(big) })} />);
  await click(host.querySelector('[data-lab-action="explore"]'));
  await flush();
  expect(table(host, "explore")).toHaveLength(100);
  expect(host.querySelector("[data-lab-pager]")!.textContent).toContain("1–100 of 150");
  const closeHead = () => [...host.querySelectorAll('[data-lab-analysis="explore"] th button')][5];
  await click(closeHead());
  const up = table(host, "explore").map((r) => Number(r[5]));
  expect(up[0]).toBe(100);
  expect(up).toEqual([...up].sort((a, b) => a - b));
  await click(closeHead());
  expect(Number(table(host, "explore")[0][5])).toBe(149);
  await click(pick(host, "Next ›"));
  expect(table(host, "explore")).toHaveLength(50);
  await click(pick(host, "Download Excel")); // every row in the shown order, numbers as numbers, dates DD-MM-YYYY
  expect(saved.file).toBe("NIFTY 50 Explore 01-01-2024 to 03-01-2024.xlsx");
  const rows = saved.sheets[0].rows;
  expect(rows).toHaveLength(151);
  expect(rows[0]).toEqual(["Symbol", "Date", "Open", "High", "Low", "Close", "Volume", "Entry", "Exit"]);
  expect(rows[1][0]).toBe("NIFTY 50"); expect(rows[1][5]).toBe(149); expect(String(rows[1][1])).toMatch(/^\d{2}-\d{2}-2024$/); expect(rows[1][6]).toBeNull(); // no volume: blank
});
it("Explore: OHLCV, one column per indicator the rules read, Entry / Exit as 0/1, recent bars and failed columns disclosed", async () => {
  const loadSeries = vi.fn(({ id }: { id: string }) => id === "ema"
    ? Promise.resolve({ ...seriesFixture("ema", ["ema"], { ema: [1.23456, null, 3] }), dates: ["2024-01-01", "2024-01-02", "2024-01-03"] })
    : Promise.reject(new Error("no data")));
  // the chart's indicators are not columns; the rules' are (EMA in the entry, RSI in the exit, each once)
  const chips: Chip[] = [{ key: "c1", id: "or_range", params: {} }];
  const spec = { ...defaultSpec(), entry: { all: [{ lhs: { s: "close" }, op: "crosses_above", rhs: { s: "ema", p: { length: 9 } } }] },
    exit: { any: [{ lhs: { s: "rsi", p: { length: 14 } }, op: ">", rhs: { value: 70 } }, { lhs: { s: "rsi", p: { length: 14 }, offset: 1 }, op: ">", rhs: { value: 80 } }] } } as ExplorePanelProps["spec"];
  const p = props({ chips, loadSeries, spec });
  const host = await mount(<ExplorePanel {...p} />);
  await click(host.querySelector('[data-lab-action="explore"]'));
  await flush();
  expect(loadSeries).toHaveBeenCalledTimes(2); // RSI once although two rules read it
  expect(loadSeries).toHaveBeenCalledWith(expect.objectContaining({ id: "ema", tf: "D", p: { length: 9 } }));
  expect(heads(host, "explore")).toEqual(["Symbol", "Date", "Open", "High", "Low", "Close", "Volume", "EMA(9)", "Entry", "Exit"]); // the instrument first, Volume after OHLC
  expect(table(host, "explore")[0]).toEqual(["NIFTY 50", "01-01-2024", "1.00", "2.00", "0.50", "1.50", "100", "1.23", "0", "0"]);
  expect(table(host, "explore")[1][8]).toBe("1");
  expect(host.querySelector("[data-lab-explore-failed]")!.textContent).toContain("RSI(14): no data");
  await click(pick(host, "Recent bar(s)"));
  const n = host.querySelector<HTMLInputElement>('input[aria-label="Recent bars"]')!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(n, "2");
    n.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(table(host, "explore").map((r) => r[1])).toEqual(["02-01-2024", "03-01-2024"]);
});

it("a rule error is shown and a changed spec clears the list", async () => {
  const bad = await mount(<ExplorePanel {...props({ onExplore: vi.fn().mockRejectedValue(new Error("Add an entry rule.")) })} />);
  await click(bad.querySelector('[data-lab-action="explore"]'));
  await flush();
  expect(bad.textContent).toContain("Add an entry rule.");
  await unmount();
  function Harness() {
    const [spec, setSpec] = useState(defaultSpec());
    return <><button type="button" onClick={() => setSpec({ ...spec, entry: { all: [] } })}>change rules</button><ExplorePanel {...props({ spec })} /></>;
  }
  const host = await mount(<Harness />);
  await click(host.querySelector('[data-lab-action="explore"]'));
  await flush();
  expect(table(host, "explore")).toHaveLength(3);
  await click(pick(host, "change rules"));
  expect(host.querySelector('[data-lab-analysis="explore"]')).toBeNull();
});
it("CSV quotes cells with commas, quotes or line breaks", () => {
  expect(csvOf(["a", "b"], [["x,y", 'say "hi"'], [null, 2]])).toBe('a,b\n"x,y","say ""hi"""\n,2');
});

it("client: scan and explore call /lab/scan with mode, apply and range; all symbols sends no ticker", async () => {
  const { scan, explore } = await import("../src/features/lab/client");
  const fetchMock = vi.fn().mockImplementation(async () => new Response(JSON.stringify(scanFixture(0)), { status: 200 }));
  vi.stubGlobal("fetch", fetchMock);
  try {
    await scan("in", defaultSpec(), { ticker: "NIFTY 50", apply: "all", from: "2020-01-01" });
    await explore("in", defaultSpec(), { ticker: "NIFTY 50" });
    const [a, b] = fetchMock.mock.calls.map((c) => new URL(String(c[0]), "http://x"));
    expect([a.pathname, a.searchParams.get("mode"), a.searchParams.get("apply"), a.searchParams.get("ticker"), a.searchParams.get("from")]).toEqual(["/api/in/lab/scan", "scan", "all", null, "2020-01-01"]);
    expect([b.pathname, b.searchParams.get("mode"), b.searchParams.get("ticker"), JSON.parse(b.searchParams.get("spec")!)]).toEqual(["/api/in/lab/scan", "explore", "NIFTY 50", defaultSpec()]);
  } finally { vi.unstubAllGlobals(); }
});

it("Explore shows every line of an indicator the rules read, loaded once (Ichimoku: TL, SL and the cloud; here MACD)", async () => {
  const loadSeries = vi.fn(() => Promise.resolve({ ...seriesFixture("macd", ["macd", "signal", "hist"], { macd: [1, 2, 3], signal: [0.5, 1, 1.5], hist: [0.5, 1, 1.5] }), dates: ["2024-01-01", "2024-01-02", "2024-01-03"] }));
  const spec = { ...defaultSpec(), entry: { all: [{ lhs: { s: "macd" }, op: "crosses_above", rhs: { s: "macd", out: "signal" } }] } } as ExplorePanelProps["spec"];
  const host = await mount(<ExplorePanel {...props({ loadSeries, spec })} />);
  await click(host.querySelector('[data-lab-action="explore"]'));
  await flush();
  expect(loadSeries).toHaveBeenCalledTimes(1); // the macd and signal lines of one MACD: one load
  const lines = heads(host, "explore").filter((h) => h!.startsWith("MACD"));
  expect(lines.map((h) => h!.split(".").pop())).toEqual(["macd", "signal", "hist"]);
  expect(table(host, "explore")[2].slice(7, 10)).toEqual(["3.00", "1.50", "1.50"]);
});

it("weekly timeframe: Explore sends tf=W, reads daily-rule indicators on weekly bars (a rule's own timeframe kept) and says one row per week", async () => {
  const { explore } = await import("../src/features/lab/client");
  const fetchMock = vi.fn().mockImplementation(async () => new Response(JSON.stringify(scanFixture(0)), { status: 200 }));
  vi.stubGlobal("fetch", fetchMock);
  try {
    await explore("in", defaultSpec(), { ticker: "NIFTY 50", tf: "W" });
    await explore("in", defaultSpec(), { ticker: "NIFTY 50", tf: "D" });
    const [w, d] = fetchMock.mock.calls.map((c) => new URL(String(c[0]), "http://x").searchParams.get("tf"));
    expect([w, d]).toEqual(["W", null]); // daily sends nothing: the request stays as before
  } finally { vi.unstubAllGlobals(); }
  // weekly rows dated each week's last session; the RSI column is the weekly RSI, a monthly EMA stays monthly
  const weekly: ExploreResult = { ...exploreFixture, tf: "W", dates: ["2024-01-05", "2024-01-12", "2024-01-19"] };
  const loadSeries = vi.fn(({ id }: { id: string }) => Promise.resolve({ ...seriesFixture(id, [id], { [id]: [1, 2, 3, 4] }), dates: ["2024-01-05", "2024-01-08", "2024-01-12", "2024-01-19"] }));
  const spec = { ...defaultSpec(), entry: { all: [{ lhs: { s: "rsi", p: { length: 14 } }, op: ">", rhs: { s: "ema", p: { length: 3 }, tf: "M" } }] } } as ExplorePanelProps["spec"];
  const host = await mount(<ExplorePanel {...props({ tf: "W", spec, loadSeries, onExplore: vi.fn().mockResolvedValue(weekly) })} />);
  await click(host.querySelector('[data-lab-action="explore"]'));
  await flush();
  expect(loadSeries).toHaveBeenCalledWith(expect.objectContaining({ id: "rsi", tf: "W" }));
  expect(loadSeries).toHaveBeenCalledWith(expect.objectContaining({ id: "ema", tf: "M" }));
  expect(table(host, "explore").map((r) => r[1])).toEqual(["05-01-2024", "12-01-2024", "19-01-2024"]);
  expect(table(host, "explore").map((r) => r[7])).toEqual(["1.00", "3.00", "4.00"]); // values on the weekly dates, the mid-week 08-01 skipped
  expect(host.querySelector("[data-lab-explore-tf]")?.textContent).toContain("One row per week");
});
