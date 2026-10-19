import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import * as lab from "../src/features/lab/client";
import { OptimizePanel } from "../src/features/lab/OptimizePanel";
import { num, pct } from "../src/features/lab/format";
import { defaultSpec } from "../src/features/lab/spec-model";
import type { IndividualOptimizeResult, OptimizeResult, Spec } from "../src/features/lab/types";
import { catalogueFixture as cat, fetchRouter, flush, jsonResponse, mount, optimizeFixture, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); vi.unstubAllGlobals(); });
const P = "entry.all[0].rhs.p.length", Q = "exit.any[0].rhs.value";
const base = { spec: defaultSpec(), catalogue: cat, market: "in" as const, role: "researcher" as const, dates: [], onApply: vi.fn(), onTried: vi.fn(), tried: 0 };
const go = async (host: HTMLElement) => { await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click()); await flush(); };
const tick = async (host: HTMLElement, path: string) => act(async () => host.querySelector<HTMLInputElement>(`[data-lab-dim="${path}"] input[type=checkbox]`)!.click());
// a 3 × 2 grid with one missing combination; table rows: dims then table_cols
const twoDims: OptimizeResult = { ...optimizeFixture, split: null, ranked_on: "full", combos: 6, dims: [{ path: P, values: [10, 20, 30] }, { path: Q, values: [1, 2] }],
  table_cols: ["total_return", "cagr", "trades"],
  table: [[10, 1, 0.1, 0.01, 5], [10, 2, 0.2, 0.02, 6], [20, 1, 0.3, 0.03, 7], [20, 2, 0.4, 0.04, 8], [30, 1, 0.5, 0.05, 9], [30, 2, null, null, 0]],
  info: { ms: 1234, combos: 6, threads: 4, warnings: ["the window starts 2020-01-02: stale opens"] } };

it("two optimized settings draw a 3D surface of the chosen metric with a heatmap toggle, axes and a legend", async () => {
  const host = await mount(<OptimizePanel {...base} onOptimize={vi.fn().mockResolvedValue(twoDims)} />);
  await tick(host, P); await go(host);
  const surface = host.querySelector("[data-lab-optimize-surface]")!;
  const svg = surface.querySelector('[data-lab-surface="3d"]')!;
  expect(svg.getAttribute("aria-label")).toContain("CAR");
  expect(svg.querySelectorAll(".lab-surface-quad")).toHaveLength(1); // 2 × 1 facets, the one touching the missing corner is open
  expect([...svg.querySelectorAll(".lab-surface-name")].map((t) => t.textContent)).toEqual(["Entry 1: EMA Length", "Exit 1: number"]);
  expect(surface.querySelector(".lab-surface-legend")!.textContent).toContain(pct(0.01, 2));
  expect(surface.querySelector(".lab-surface-legend")!.textContent).toContain(pct(0.05, 2)); // range of every value, the missing one excluded
  await act(async () => surface.querySelector<HTMLButtonElement>('[aria-label="Chart"] button:last-child')!.click());
  const heat = surface.querySelector('[data-lab-surface="heat"]')!;
  expect(heat.querySelectorAll("rect")).toHaveLength(6);
  expect(heat.querySelector("rect title")!.textContent).toContain("CAR");
  const metric = surface.querySelector<HTMLSelectElement>('select[aria-label="Metric"]')!;
  await act(async () => { metric.value = "trades"; metric.dispatchEvent(new Event("change", { bubbles: true })); });
  expect(surface.querySelector(".lab-surface-legend")!.textContent).toContain("# Trades");
});

it("no surface with one optimized setting; the Info log lists elapsed time, combinations, threads and warnings", async () => {
  const host = await mount(<OptimizePanel {...base} onOptimize={vi.fn().mockResolvedValue({ ...optimizeFixture, info: twoDims.info })} />);
  await tick(host, P); await go(host);
  expect(host.querySelector("[data-lab-optimize-surface]")).toBeNull();
  const info = host.querySelector("[data-lab-info]")!;
  expect(info.textContent).toBe("▸ Info · 1 warning");
  await act(async () => info.querySelector("button")!.click());
  expect([...info.querySelectorAll("li")].map((l) => l.textContent)).toEqual(["Elapsed: 1.23 s", "Combinations evaluated: 6", "Threads: 4", "the window starts 2020-01-02: stale opens"]);
});

const individual: IndividualOptimizeResult = {
  label: "User-defined indicator strategy · simulated", kind: "lab_individual_optimization", origin: null, universe: { basket: "nifty50", symbols: 3 }, combos: 2,
  dims: [{ path: P, values: [10, 20] }], cols: [P, "total_return", "cagr", "sharpe", "max_dd", "trades"], table_cols: ["total_return", "trades"],
  results: [
    { ticker: "A", from: "2020-01-01", to: "2024-01-01", eligible: 2, min_trades: 5, best: [20, 0.3, 0.06, 1.1, -0.1, 12], table: [[10, 0.1, 6], [20, 0.3, 12]], table_rows: 2 },
    { ticker: "B", from: "2020-01-01", to: "2024-01-01", eligible: 0, min_trades: 5, best: null, table: [[10, 0, 1], [20, 0, 2]], table_rows: 2 },
    { ticker: "C", skipped: "C: no data in the window" },
  ],
  notes: ["Each symbol is optimized alone."], info: { ms: 50, combos: 4, threads: 2, warnings: ["C skipped: C: no data in the window"] },
};
it("Individual optimize: portfolio settings drop out, each symbol's best row, skipped symbols, per-symbol tables and the Info log", async () => {
  const { portfolioCatalogueFixture } = await import("./fixtures/portfolio");
  const spec: Spec = { ...defaultSpec(), execution: { capital: 1000 }, portfolio: { universe: { basket: "nifty50" }, mode: "signals", max_open: 2 } };
  const onIndividual = vi.fn().mockResolvedValue(individual), onOptimize = vi.fn(), onTried = vi.fn();
  const host = await mount(<OptimizePanel {...base} onTried={onTried} catalogue={portfolioCatalogueFixture} spec={spec} onOptimize={onOptimize} onIndividual={onIndividual} />);
  expect(host.querySelector('[data-lab-dim="portfolio.max_open"]')).not.toBeNull();
  const box = [...host.querySelectorAll<HTMLLabelElement>("label.lab-check")].find((l) => l.textContent === "Individual optimize")!;
  await act(async () => box.querySelector("input")!.click());
  expect(host.querySelector('[data-lab-dim="portfolio.max_open"]')).toBeNull();
  expect(host.querySelector("[data-lab-individual-note]")!.textContent).toContain("12,000");
  await tick(host, P); await go(host);
  expect(onOptimize).not.toHaveBeenCalled();
  expect((onIndividual.mock.calls[0][0] as Spec).optimize).toEqual([{ path: P, from: 50, to: 54, step: 1 }]);
  expect(onTried).toHaveBeenCalledWith(4);
  const rows = (t: string) => host.querySelector(`[data-lab-individual-row="${t}"]`)!.textContent;
  expect([...host.querySelectorAll('[data-lab-individual-row="A"] td')].map((c) => c.textContent)).toEqual(["A", "20", pct(0.3, 2), pct(0.06, 2), pct(-0.1, 2), num(1.1), "12", "2"]);
  expect(rows("B")).toContain("No combination with at least 5 trades");
  expect(rows("C")).toContain("Skipped: C: no data in the window");
  const tables = [...host.querySelectorAll("[data-lab-optimize-all] > button")].map((b) => b.textContent);
  expect(tables).toEqual(["▸ A · all results · 2 combinations", "▸ B · all results · 2 combinations"]);
  expect(host.querySelector("[data-lab-individual]")!.textContent).toContain("Each symbol is optimized alone.");
  expect(host.querySelector("[data-lab-info]")!.textContent).toBe("▸ Info · 1 warning");
});

it("optimizeIndividual calls /lab/optimize with mode=individual and the table", async () => {
  const f = fetchRouter([["/api/", () => jsonResponse(individual)]]);
  vi.stubGlobal("fetch", f);
  await lab.optimizeIndividual("in", defaultSpec(), { from: "2020-01-01" });
  const url = new URL(f.calls[0].url, "http://x");
  expect([url.pathname, url.searchParams.get("mode"), url.searchParams.get("table"), url.searchParams.get("from"), url.searchParams.has("to")]).toEqual(["/api/in/lab/optimize", "individual", "1", "2020-01-01", false]);
});
