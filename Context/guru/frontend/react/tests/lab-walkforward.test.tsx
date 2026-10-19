import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { OptimizePanel } from "../src/features/lab/OptimizePanel";
import { WF_LABEL, wfWindows } from "../src/features/lab/WalkForward";
import { defaultSpec } from "../src/features/lab/spec-model";
import type { Spec, WalkforwardOptions, WalkforwardResult } from "../src/features/lab/types";
import { catalogueFixture as cat, flush, mount, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); });
const base = { spec: defaultSpec(), catalogue: cat, market: "in" as const, role: "researcher" as const, dates: [], onApply: vi.fn(), onTried: vi.fn(), tried: 0, onOptimize: vi.fn() };
const PATH = "entry.all[0].rhs.p.length";
const m = (r: number) => ({ total_return: r, cagr: r, sharpe: 1, max_dd: -0.05, mar: 2, trades: 12, win_rate: 0.5, profit_factor: 1.4 });
const fixture: WalkforwardResult = {
  label: "User-defined indicator strategy · simulated · not a book method", kind: "lab_walkforward", target: "score", unit: "years", is: 2, oos: 1, step: 1, anchored: false, combos: 5, ms: 10,
  windows: [
    { is: { from: "2015-01-01", to: "2016-12-30" }, oos: { from: "2017-01-02", to: "2017-12-29" }, eligible: 4, params: { [PATH]: 52 }, is_metrics: m(0.2), oos_metrics: m(0.1) },
    { is: { from: "2016-01-01", to: "2017-12-29" }, oos: { from: "2018-01-01", to: "2018-12-31" }, eligible: 0, params: null, is_metrics: null, oos_metrics: null, skipped: "no eligible combination in-sample; out-of-sample held flat" },
  ],
  oos_equity: { dates: ["2017-01-02", "2017-06-30", "2017-12-29", "2018-12-31"], equity: [1, 1.05, 1.1, 1.1] },
  summary: { total_return: 0.1, cagr: 0.05, max_dd: -0.05, trades: 12, win_rate: 0.5, windows: 2, skipped: 1, from: "2017-01-02", to: "2018-12-31" },
};
const tick = async (host: HTMLElement) => act(async () => host.querySelector<HTMLInputElement>(`[data-lab-dim="${PATH}"] input[type=checkbox]`)!.click());
const change = async (el: HTMLInputElement | HTMLSelectElement, value: string) => act(async () => {
  const proto = el instanceof HTMLSelectElement ? HTMLSelectElement.prototype : HTMLInputElement.prototype;
  Object.getOwnPropertyDescriptor(proto, "value")!.set!.call(el, value);
  el.dispatchEvent(new Event(el instanceof HTMLSelectElement ? "change" : "input", { bubbles: true }));
});

it("walk-forward form: needs a ticked setting, sends the grid and the window options", async () => {
  const onWalkforward = vi.fn().mockResolvedValue(fixture);
  const host = await mount(<OptimizePanel {...base} onWalkforward={onWalkforward} />);
  const wf = host.querySelector("[data-lab-walkforward]")!;
  expect(wf.textContent).toContain(WF_LABEL);
  const button = () => wf.querySelector<HTMLButtonElement>('[data-lab-action="walkforward"]')!;
  expect(button().disabled).toBe(true);
  await tick(host);
  expect(button().disabled).toBe(false);
  const target = wf.querySelector<HTMLSelectElement>('select[aria-label="Walk-forward target"]')!;
  expect([target.value, target.selectedOptions[0].textContent]).toEqual(["mar", "CAR/MDD"]); // AmiBroker's default target
  await change(wf.querySelector<HTMLSelectElement>('select[aria-label="Walk-forward unit"]')!, "months");
  await change(wf.querySelector<HTMLSelectElement>('select[aria-label="Walk-forward target"]')!, "sharpe");
  await change(wf.querySelector<HTMLInputElement>('input[aria-label="Walk-forward Step"]')!, "3");
  await act(async () => wf.querySelector<HTMLInputElement>('input[aria-label="Anchored"]')!.click());
  await act(async () => button().click());
  await flush();
  const [spec, o] = onWalkforward.mock.calls[0] as [Spec, WalkforwardOptions];
  expect(spec.optimize).toEqual([{ path: PATH, from: 50, to: 54, step: 1 }]);
  expect(o).toEqual({ is: 2, oos: 1, step: 3, unit: "months", anchored: true, target: "sharpe" });
  // a step shorter than the out-of-sample length would overlap windows
  await change(wf.querySelector<HTMLInputElement>('input[aria-label="Walk-forward Out-of-sample"]')!, "4");
  expect(button().disabled).toBe(true);
});

it("walk-forward result: IS/OOS rows per window, held-flat window, summary and stitched equity", async () => {
  const host = await mount(<OptimizePanel {...base} onWalkforward={vi.fn().mockResolvedValue(fixture)} />);
  await tick(host);
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="walkforward"]')!.click());
  await flush();
  const result = host.querySelector("[data-lab-wf-result]")!;
  const rows = [...result.querySelectorAll("tbody tr")].map((r) => [...r.children].map((c) => c.textContent));
  expect(rows.map((r) => r[0])).toEqual(["IS", "OOS", "IS", "OOS"]);
  expect(rows[1].slice(1, 3)).toEqual(["2017-01-02", "2017-12-29"]);
  expect(rows[1][3]).toContain("52");
  expect(rows[1][4]).toBe("10.0%");
  expect(rows[3][3]).toBe("held flat");
  expect(result.querySelector("[data-lab-wf-summary]")!.textContent).toContain("1 held flat");
  expect(result.textContent).toContain("simulated · not a book method");
  expect(result.querySelector("[data-lab-wf-equity] polyline")!.getAttribute("points")!.split(" ")).toHaveLength(4);
});

it("walk-forward is absent without a handler", async () => {
  const host = await mount(<OptimizePanel {...base} />);
  expect(host.querySelector("[data-lab-walkforward]")).toBeNull();
});

it("walk-forward counts its variants, shows open-at-edge windows and shares the optimizer slot", async () => {
  let finish: (r: WalkforwardResult) => void = () => {};
  const onTried = vi.fn();
  const open = { ...fixture, windows: [{ ...fixture.windows[0], open_at_end: true }, fixture.windows[1]], summary: { ...fixture.summary, open_at_edge: 1 }, notes: ["declared walk-forward note"] };
  const host = await mount(<OptimizePanel {...base} onTried={onTried} onWalkforward={vi.fn(() => new Promise<WalkforwardResult>((r) => { finish = r; }))} />);
  await tick(host);
  const optimize = () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!;
  expect(optimize().disabled).toBe(false);
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="walkforward"]')!.click());
  expect(optimize().disabled).toBe(true); // one engine slot: no optimization while the walk-forward runs
  await act(async () => finish(open));
  await flush();
  expect(optimize().disabled).toBe(false);
  expect(onTried).toHaveBeenCalledWith(5 * 2); // combinations × windows
  const result = host.querySelector("[data-lab-wf-result]")!;
  expect(result.querySelector("[data-lab-wf-summary]")!.textContent).toContain("1 window ended with a position open");
  expect(result.textContent).toContain("open at end");
  expect(result.querySelector("[data-lab-wf-notes]")!.textContent).toContain("declared walk-forward note");
});

it("a 429 wait on the optimizer also holds the walk-forward button", async () => {
  const busy = Object.assign(new Error("busy"), { status: 429, retryAfter: 30 });
  const host = await mount(<OptimizePanel {...base} onOptimize={vi.fn().mockRejectedValue(busy)} onWalkforward={vi.fn()} />);
  await tick(host);
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await flush();
  expect(host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.disabled).toBe(true);
  expect(host.querySelector<HTMLButtonElement>('[data-lab-action="walkforward"]')!.disabled).toBe(true);
});

it("wfWindows mirrors the server windows: rolling, anchored, the 21-bar floor and the empty range", () => {
  const days = Array.from({ length: 1100 }, (_, i) => new Date(Date.UTC(2015, 0, 1 + i)).toISOString().slice(0, 10));
  const r = wfWindows(days, { is: 1, oos: 6, step: 6, unit: "months", anchored: false });
  expect(r.error).toBeUndefined();
  expect(r.windows[0]).toEqual({ is: ["2015-01-01", "2015-01-31"], oos: ["2015-02-01", "2015-07-31"] });
  expect(r.windows[1].is[0]).toBe("2015-07-01");
  expect(r.windows.at(-1)!.oos[1]).toBe(days.at(-1));
  expect(wfWindows(days, { is: 1, oos: 6, step: 6, unit: "months", anchored: true }).windows[1].is[0]).toBe("2015-01-01");
  expect(wfWindows(days.filter((_, i) => i % 2 === 0), { is: 1, oos: 1, step: 1, unit: "months", anchored: false }).error).toContain("at least 21");
  expect(wfWindows(days, { is: 5, oos: 1, step: 1, unit: "years", anchored: false }).error).toContain("shorter than one in-sample window");
});

it("walk-forward preview: Optimization target, window table and dates beside the lengths before running; settings get their own column", async () => {
  const days = Array.from({ length: 1500 }, (_, i) => new Date(Date.UTC(2015, 0, 1 + i)).toISOString().slice(0, 10));
  const host = await mount(<OptimizePanel {...base} dates={days} onWalkforward={vi.fn().mockResolvedValue(fixture)} />);
  const wf = host.querySelector("[data-lab-walkforward]")!;
  expect(wf.textContent).toContain("Optimization target");
  expect(wf.textContent).not.toContain("Pick by");
  expect(wf.querySelector('[data-lab-action="walkforward"]')!.className).toContain("primary");
  const preview = wf.querySelector("[data-lab-wf-preview]")!;
  expect([...preview.querySelectorAll("tbody tr")].slice(0, 2).map((r) => [...r.children].map((c) => c.textContent))).toEqual([
    ["IS", "2015-01-01", "2016-12-31"], ["OOS", "2017-01-01", "2017-12-31"]]);
  expect(wf.querySelector(".lab-wf-form")!.textContent).toContain("2015-01-01 – 2016-12-31");
  await tick(host);
  await act(async () => wf.querySelector<HTMLButtonElement>('[data-lab-action="walkforward"]')!.click());
  await flush();
  expect(wf.querySelector("[data-lab-wf-preview]")).toBeNull();
  const heads = [...wf.querySelectorAll("[data-lab-wf-result] thead th")].map((th) => th.textContent);
  expect(heads[3]).toBe("Entry 1: EMA Length");
  expect(wf.querySelector("[data-lab-wf-result] .metrics")!.textContent).toContain("Windows");
  // the optimization empty state sits above the walk-forward section, not under its controls
  const empty = host.querySelector("[data-lab-optimize] > .empty");
  expect(empty === null || !!(empty.compareDocumentPosition(wf) & Node.DOCUMENT_POSITION_FOLLOWING)).toBe(true);
});

it("declared notes say open at end in words, not the response field name", async () => {
  const notes = ["A position … is counted in open_at_edge, not in trades or win rate."];
  const host = await mount(<OptimizePanel {...base} onWalkforward={vi.fn().mockResolvedValue({ ...fixture, notes })} />);
  await tick(host);
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="walkforward"]')!.click());
  await flush();
  expect(host.querySelector("[data-lab-wf-notes]")!.textContent).not.toContain("open_at_edge");
});

it("walk-forward result carries the Info log: elapsed time, combinations over all windows, threads and window warnings", async () => {
  const info = { ms: 2500, combos: 10, threads: 3, warnings: ["window 2016-01-01 – 2017-12-29: no eligible combination in-sample; out-of-sample held flat"] };
  const host = await mount(<OptimizePanel {...base} onWalkforward={vi.fn().mockResolvedValue({ ...fixture, info })} />);
  await tick(host);
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="walkforward"]')!.click());
  await flush();
  const log = host.querySelector("[data-lab-wf-result] [data-lab-info]")!;
  expect(log.textContent).toBe("▸ Info · 1 warning");
  await act(async () => log.querySelector("button")!.click());
  expect([...log.querySelectorAll("li")].map((l) => l.textContent)).toEqual(["Elapsed: 2.50 s", "Combinations evaluated: 10", "Threads: 3", info.warnings[0]]);
});
