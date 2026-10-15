import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { OptimizePanel, colValue } from "../src/features/lab/OptimizePanel";
import { defaultSpec } from "../src/features/lab/spec-model";
import type { Spec } from "../src/features/lab/types";
import { catalogueFixture as cat, flush, mount, optimizeFixture, unmount } from "./fixtures/lab";

afterEach(async () => { await unmount(); vi.useRealTimers(); });
const dates = Array.from({ length: 300 }, (_, i) => new Date(Date.UTC(2020, 0, 1 + i)).toISOString().slice(0, 10));
const base = { spec: defaultSpec(), catalogue: cat, market: "in" as const, role: "researcher" as const, dates, onApply: vi.fn(), onTried: vi.fn(), tried: 0 };
const tick = async (host: HTMLElement, path: string) => act(async () => host.querySelector<HTMLInputElement>(`[data-lab-dim="${path}"] input[type=checkbox]`)!.click());

it("viewers see the researcher notice and no controls", async () => {
  const host = await mount(<OptimizePanel {...base} role="viewer" onOptimize={vi.fn()} />);
  expect(host.textContent).toContain("Researcher access is required for optimization.");
  expect(host.querySelector('[data-lab-action="optimize"]')).toBeNull();
});

it("ticking a setting fills a small range; split sends a date; optimize carries the dims", async () => {
  const onOptimize = vi.fn().mockResolvedValue(optimizeFixture);
  const host = await mount(<OptimizePanel {...base} onOptimize={onOptimize} />);
  await tick(host, "entry.all[0].rhs.p.length");
  expect(host.textContent).toContain("5 combinations");
  const split = host.querySelector<HTMLSelectElement>('select[aria-label="Held back"]')!;
  await act(async () => { split.value = "30"; split.dispatchEvent(new Event("change", { bubbles: true })); });
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await flush();
  const [spec, splitDate] = onOptimize.mock.calls[0] as [Spec, string];
  expect(spec.optimize).toEqual([{ path: "entry.all[0].rhs.p.length", from: 50, to: 54, step: 1 }]);
  expect(splitDate).toBe(dates[210]);
  expect(base.onTried).toHaveBeenCalledWith(3);
});

it("result: ranked-on text, Now and top 3, held-back columns, Apply writes the setting", async () => {
  const onApply = vi.fn();
  const host = await mount(<OptimizePanel {...base} onApply={onApply} onOptimize={vi.fn().mockResolvedValue(optimizeFixture)} />);
  await tick(host, "entry.all[0].rhs.p.length");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await flush();
  const result = host.querySelector("[data-lab-optimize-result]")!;
  expect(result.textContent).toContain("Ranked on: in-sample only");
  expect(result.textContent).toContain("Held back · display only");
  expect(result.textContent).toContain("trades that cross the split date are cut at the boundary");
  expect([...result.querySelectorAll("tbody tr")].map((r) => r.firstElementChild!.textContent)).toEqual(["Now", "#1", "#2"]);
  await act(async () => result.querySelector<HTMLButtonElement>("tbody tr:nth-child(2) button")!.click());
  expect((onApply.mock.calls[0][0] as Spec).entry.all[0].rhs).toEqual({ s: "ema", p: { length: 20 } });
  expect((onApply.mock.calls[0][0] as Spec).optimize).toBeUndefined();
  expect(colValue(optimizeFixture, optimizeFixture.base, "cagr")).toBe(0.05);
  expect(colValue(optimizeFixture, optimizeFixture.picked["1"], "oos_trades")).toBe(7);
});

it("429 shows a countdown and re-enables retry", async () => {
  vi.useFakeTimers();
  const busy = Object.assign(new Error("An optimization is already running. Please retry when it finishes."), { status: 429, kind: "gateway", retryAfter: 5 });
  const onOptimize = vi.fn().mockRejectedValueOnce(busy).mockResolvedValueOnce(optimizeFixture);
  const host = await mount(<OptimizePanel {...base} onOptimize={onOptimize} />);
  await tick(host, "entry.all[0].rhs.p.length");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await act(async () => { await Promise.resolve(); });
  const retry = () => host.querySelector<HTMLButtonElement>("[data-lab-error] button")!;
  expect(retry().textContent).toBe("Try again in 5 s");
  expect(host.querySelector("[data-lab-optimize-result]")).toBeNull();
  await act(async () => { vi.advanceTimersByTime(5000); });
  await act(async () => retry().click());
  await act(async () => { await Promise.resolve(); await Promise.resolve(); });
  expect(onOptimize).toHaveBeenCalledTimes(2);
});

it("the engine timeout text is shown as given, with no numbers", async () => {
  const timeout = Object.assign(new Error("optimization stopped after 300 s; narrow the grid. No partial ranking is published."), { status: 400, kind: "engine", engineStatus: "invalid" });
  const host = await mount(<OptimizePanel {...base} onOptimize={vi.fn().mockRejectedValue(timeout)} />);
  await tick(host, "entry.all[0].rhs.p.length");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await flush();
  expect(host.querySelector("[data-lab-error]")?.textContent).toContain("No partial ranking is published.");
  expect(host.querySelector("table.ocmp")).toBeNull();
});

it("no eligible settings: nothing is ranked", async () => {
  const none = { ...optimizeFixture, rank: { ...optimizeFixture.rank, eligible: 0, distinct: 0, top3: [], stats: undefined } };
  const host = await mount(<OptimizePanel {...base} onOptimize={vi.fn().mockResolvedValue(none)} />);
  await tick(host, "entry.all[0].rhs.p.length");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await flush();
  expect(host.textContent).toContain("No setting had enough trades to rank (at least 10). Nothing is ranked.");
});
