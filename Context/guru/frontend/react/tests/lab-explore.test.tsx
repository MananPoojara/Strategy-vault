import { afterEach, expect, it, vi } from "vitest";
import { act, useState } from "react";
import { ExplorePanel, csvOf, type ExplorePanelProps } from "../src/features/lab/Explore";
import type { ExploreResult, ScanResult } from "../src/features/lab/client";
import { defaultSpec } from "../src/features/lab/spec-model";
import type { Chip } from "../src/features/lab/Workbench";
import { catalogueFixture as cat, flush, mount, seriesFixture, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); });
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
  spec: defaultSpec(), catalogue: cat, chips: [], portfolio: false, market: "in",
  onScan: vi.fn().mockResolvedValue(scanFixture(3)), onExplore: vi.fn().mockResolvedValue(exploreFixture), loadSeries: vi.fn(), ...patch,
});
const click = (el: Element | null) => act(async () => { (el as HTMLElement).click(); });
const pick = (host: HTMLElement, label: string) => [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === label)!;
const table = (host: HTMLElement, name: string) => [...host.querySelectorAll(`[data-lab-analysis="${name}"] tbody tr`)].map((r) => [...r.children].map((c) => c.textContent));
const heads = (host: HTMLElement, name: string) => [...host.querySelectorAll(`[data-lab-analysis="${name}"] th`)].map((c) => c.textContent);

it("is collapsed until opened, and Scan lists Symbol, Date/Time, Trade, Close", async () => {
  const p = props();
  const host = await mount(<ExplorePanel {...p} />);
  const details = host.querySelector<HTMLDetailsElement>("[data-lab-explore]")!;
  expect(details.open).toBe(false);
  expect(host.querySelector<HTMLOptionElement>('select[aria-label="Apply to"] option[value="all"]')!.disabled).toBe(true);
  await click(host.querySelector('[data-lab-action="scan"]'));
  await flush();
  expect(p.onScan).toHaveBeenCalledWith("current");
  expect(heads(host, "scan")).toEqual(["Symbol", "Date/Time", "Trade", "Close"]);
  expect(table(host, "scan")).toEqual([["NIFTY 50", "2024-01-01", "Buy", "100.00"], ["NIFTY 50", "2024-01-02", "Sell", "137.00"], ["NIFTY 50", "2024-01-03", "Buy", "124.00"]]);
  expect(host.textContent).toContain("not filled trades");
});

it("sorts by a clicked column (again reverses), pages 100 rows and copies every row as CSV", async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
  const host = await mount(<ExplorePanel {...props({ onScan: vi.fn().mockResolvedValue(scanFixture(150)) })} />);
  await click(host.querySelector('[data-lab-action="scan"]'));
  await flush();
  expect(table(host, "scan")).toHaveLength(100);
  expect(host.querySelector("[data-lab-pager]")!.textContent).toContain("1–100 of 150");
  const close = () => [...host.querySelectorAll('[data-lab-analysis="scan"] th button')][3];
  await click(close());
  const up = table(host, "scan").map((r) => Number(r[3]));
  expect(up[0]).toBe(100);
  expect(up).toEqual([...up].sort((a, b) => a - b));
  expect(host.querySelector('[data-lab-analysis="scan"] th[aria-sort="ascending"]')).not.toBeNull();
  await click(close());
  expect(Number(table(host, "scan")[0][3])).toBe(149);
  await click(pick(host, "Next ›"));
  expect(table(host, "scan")).toHaveLength(50);
  await click(pick(host, "Copy CSV"));
  const csv = writeText.mock.calls[0][0] as string;
  expect(csv.split("\n")).toHaveLength(151);
  expect(csv.split("\n")[0]).toBe("Symbol,Date/Time,Trade,Close");
  expect(csv.split("\n")[1]).toMatch(/,149$/);
});

it("Apply to: All symbols is offered with a portfolio universe and sent as apply=all", async () => {
  const p = props({ portfolio: true });
  const host = await mount(<ExplorePanel {...p} />);
  const select = host.querySelector<HTMLSelectElement>('select[aria-label="Apply to"]')!;
  expect(select.querySelector<HTMLOptionElement>('option[value="all"]')!.disabled).toBe(false);
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")!.set!.call(select, "all");
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
  await click(host.querySelector('[data-lab-action="scan"]'));
  expect(p.onScan).toHaveBeenCalledWith("all");
});

it("Explore: OHLCV, one column per chart indicator output, signals as 0/1, recent bars and failed columns disclosed", async () => {
  const loadSeries = vi.fn(({ id }: { id: string }) => id === "ema"
    ? Promise.resolve({ ...seriesFixture("ema", ["ema"], { ema: [1.23456, null, 3] }), dates: ["2024-01-01", "2024-01-02", "2024-01-03"] })
    : Promise.reject(new Error("no data")));
  const chips: Chip[] = [{ key: "c1", id: "ema", params: { length: 9 } }, { key: "c2", id: "rsi", params: { length: 14 } }, { key: "c3", id: "or_range", params: {} }];
  const p = props({ chips, loadSeries });
  const host = await mount(<ExplorePanel {...p} />);
  await click(pick(host, "Explore"));
  await click(host.querySelector('[data-lab-action="explore"]'));
  await flush();
  expect(loadSeries).toHaveBeenCalledTimes(2); // the not-computed chip is not requested
  expect(loadSeries).toHaveBeenCalledWith({ id: "ema", tf: "D", p: { length: 9 }, vs: undefined });
  expect(heads(host, "explore")).toEqual(["Date", "Open", "High", "Low", "Close", "Volume", "EMA(9)", "Buy", "Sell", "Short", "Cover"]);
  expect(table(host, "explore")[0]).toEqual(["2024-01-01", "1.00", "2.00", "0.50", "1.50", "100", "1.2346", "0", "0", "0", "0"]);
  expect(table(host, "explore")[1][7]).toBe("1");
  expect(host.querySelector("[data-lab-explore-failed]")!.textContent).toContain("RSI(14): no data");
  await click(pick(host, "Recent bar(s)"));
  const n = host.querySelector<HTMLInputElement>('input[aria-label="Recent bars"]')!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(n, "2");
    n.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(table(host, "explore").map((r) => r[0])).toEqual(["2024-01-02", "2024-01-03"]);
});

it("a rule error is shown and a changed spec clears the list", async () => {
  const bad = await mount(<ExplorePanel {...props({ onScan: vi.fn().mockRejectedValue(new Error("Add an entry rule.")) })} />);
  await click(bad.querySelector('[data-lab-action="scan"]'));
  await flush();
  expect(bad.textContent).toContain("Add an entry rule.");
  await unmount();
  function Harness() {
    const [spec, setSpec] = useState(defaultSpec());
    return <><button type="button" onClick={() => setSpec({ ...spec, entry: { all: [] } })}>change rules</button><ExplorePanel {...props({ spec })} /></>;
  }
  const host = await mount(<Harness />);
  await click(host.querySelector('[data-lab-action="scan"]'));
  await flush();
  expect(table(host, "scan")).toHaveLength(3);
  await click(pick(host, "change rules"));
  expect(host.querySelector('[data-lab-analysis="scan"]')).toBeNull();
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
