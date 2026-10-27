import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { SimpleOptimize } from "../src/features/lab/SimpleOptimize";
import { candleCountNode } from "../src/features/lab/spec-model";
import { pct } from "../src/features/lab/format";
import type { LabOptimizeResult, OptimizeResult, Spec } from "../src/features/lab/types";
import { catalogueFixture as cat, flush, mount, optimizeFixture, unmount } from "./fixtures/lab";

afterEach(unmount);
const L = "entry.all[0].lhs.p.length", T = "entry.all[0].rhs.value", C = "exit.any[0].rhs.value";
const spec: Spec = { v: 1, entry: { all: [{ lhs: { s: "rsi", p: { length: 14 } }, op: "<", rhs: { value: 30 } }] }, exit: { any: [candleCountNode("up", 1)] } };
const props = { spec, catalogue: cat, onApply: vi.fn() };
/** api.rs TABLE_KEYS: each table row is the dim values, then these. */
const TABLE_COLS = ["total_return", "exposure", "cagr", "max_dd", "mar", "trades", "win_rate", "profit_factor", "avg_trade", "sharpe"];
const net = (len: number, below: number) => (len - 10) * 0.05 + below * 0.001;
/** RSI length 12–16 × "below" 28–32, net profit rising with both; one combination without a figure (null). */
function fakeResult(cols = TABLE_COLS): OptimizeResult {
  const table: (number | null)[][] = [];
  for (const len of [12, 13, 14, 15, 16]) for (const below of [28, 29, 30, 31, 32]) {
    const m: Record<string, number | null> = { total_return: len === 12 && below === 28 ? null : net(len, below), exposure: 0.4, cagr: 0.05,
      max_dd: -0.1 - len / 1000, mar: 0.5, trades: len + below, win_rate: 0.55, profit_factor: 1.4, avg_trade: 0.002, sharpe: 0.9 };
    table.push([len, below, ...cols.map((c) => m[c])]);
  }
  return { ...optimizeFixture, split: null, ranked_on: "full", combos: 25, ms: 40,
    dims: [{ path: L, values: [12, 13, 14, 15, 16] }, { path: T, values: [28, 29, 30, 31, 32] }],
    cols: [L, T, "total_return", "cagr", "sharpe", "max_dd", "trades"], base: [net(14, 30), 0.05, 0.9, -0.114, 44],
    rank: { ...optimizeFixture.rank, eligible: 25, distinct: 25, total: 25, top3: [] }, picked: {}, rows: [], table_cols: cols, table };
}

const line = (host: HTMLElement, path: string) => host.querySelector<HTMLElement>(`[data-lab-sopt-line="${path}"]`)!;
const toggle = (host: HTMLElement, path: string) => act(async () => line(host, path).querySelector<HTMLInputElement>("input[type=checkbox]")!.click());
const field = (host: HTMLElement, path: string, label: "From" | "To" | "Step") => line(host, path).querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!;
const runButton = (host: HTMLElement) => host.querySelector<HTMLButtonElement>('[data-lab-action="simple-optimize"]')!;
const countText = (host: HTMLElement) => host.querySelector(".lab-sopt-count")?.textContent;
const typeInto = (input: HTMLInputElement, value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
});

it("lists each number of the rules in words with that number marked; rule on/off switches are not listed", async () => {
  const host = await mount(<SimpleOptimize {...props} onOptimize={vi.fn()} />);
  expect(host.querySelector("h2")!.textContent).toBe("Test every combination");
  const lines = [...host.querySelectorAll<HTMLElement>("[data-lab-sopt-line]")];
  expect(lines.map((l) => l.dataset.labSoptLine)).toEqual([L, T, C]);
  expect(lines.map((l) => [l.querySelector(".lab-sopt-words")!.textContent, l.querySelector(".lab-sopt-words b")!.textContent])).toEqual([
    ["Buy when RSI(14) is below 30", "14"], ["Buy when RSI(14) is below 30", "30"], ["Sell at the 1st up close after entry", "1"]]);
  expect(line(host, C).querySelector("input[type=checkbox]")!.getAttribute("aria-label")).toBe("Try a range: Sell at the Nth up close after entry");
  expect(line(host, L).querySelector("input[type=number]")).toBeNull(); // the range opens only on "Try a range"
  expect(host.textContent).toContain("Turn on “Try a range” for at least one number.");
  expect(runButton(host).disabled).toBe(true);
});

it("a range starts around the current value; the count multiplies the value counts; too many or a broken range blocks the run", async () => {
  const host = await mount(<SimpleOptimize {...props} catalogue={{ ...cat, max_combos: 40 }} onOptimize={vi.fn()} />);
  await toggle(host, L);
  expect((["From", "To", "Step"] as const).map((k) => field(host, L, k).value)).toEqual(["12", "16", "1"]);
  expect(line(host, L).querySelector("[role=group]")!.getAttribute("aria-label")).toBe("Range for Buy when RSI(N) is below 30");
  expect(line(host, L).textContent).toContain("5 values");
  expect(countText(host)).toBe("5 combinations");
  await toggle(host, T);
  expect([field(host, T, "From").value, field(host, T, "To").value]).toEqual(["28", "32"]);
  expect(countText(host)).toBe("25 combinations");
  expect(runButton(host).textContent).toBe("Test all 25 combinations");
  expect(runButton(host).disabled).toBe(false);
  await toggle(host, C); // a candle count never starts below 1 ("the 0th up close")
  expect([field(host, C, "From").value, field(host, C, "To").value]).toEqual(["1", "2"]);
  expect(host.textContent).toContain("50 combinations is more than the 40 one test can run.");
  expect(runButton(host).disabled).toBe(true);
  await toggle(host, C);
  expect(field(host, C, "From")).toBeNull();
  await typeInto(field(host, T, "To"), "31");
  expect(countText(host)).toBe("20 combinations");
  expect(runButton(host).disabled).toBe(false);
  await typeInto(field(host, L, "Step"), "");
  expect(host.textContent).toContain("Check each range: From must not be above To, and Step must be above 0.");
  expect(runButton(host).disabled).toBe(true);
});

it("tests every combination, lists the best 20 by net profit, and Use these numbers applies a row without the optimize list", async () => {
  let answer!: (r: LabOptimizeResult) => void;
  const onOptimize = vi.fn((_sent: Spec) => new Promise<LabOptimizeResult>((resolve) => { answer = resolve; }));
  const onApply = vi.fn();
  const host = await mount(<SimpleOptimize {...props} onOptimize={onOptimize} onApply={onApply} />);
  await toggle(host, L);
  await toggle(host, T);
  await act(async () => runButton(host).click());
  expect(onOptimize.mock.calls[0][0].optimize).toEqual([{ path: L, from: 12, to: 16, step: 1 }, { path: T, from: 28, to: 32, step: 1 }]);
  expect(host.querySelector('[role="status"]')!.textContent).toContain("Testing 25 combinations");
  expect(runButton(host).disabled).toBe(true);

  await act(async () => answer(fakeResult()));
  await flush();
  expect(host.querySelector('[role="status"]')).toBeNull();
  const result = host.querySelector("[data-lab-sopt-result]")!;
  expect(result.textContent).toContain(optimizeFixture.label);
  expect(result.textContent).toContain("The best 20 of 25 combinations, highest net profit first.");
  expect([...result.querySelectorAll("thead th")].map((th) => th.textContent)).toEqual(
    ["Buy when RSI(N) is below 30", "Buy when RSI(14) is below N", "Net profit", "Trades", "Win rate", "Worst fall", "Use"]);
  const rows = [...result.querySelectorAll("tbody tr")];
  expect(rows).toHaveLength(20);
  const cells = (r: Element) => [...r.querySelectorAll("td")].slice(0, 6).map((td) => td.textContent);
  expect(cells(rows[0])).toEqual(["16", "32", pct(net(16, 32)), "48", "55%", pct(-0.116)]);
  expect(cells(rows[19]).slice(0, 2)).toEqual(["13", "28"]); // the five RSI length 12 rows (and the gap) fall outside the best 20
  const nets = rows.map((r) => Number.parseFloat(r.children[2].textContent!));
  expect(nets).toEqual([...nets].sort((a, b) => b - a));

  await act(async () => rows[0].querySelector<HTMLButtonElement>("button")!.click());
  expect(rows[0].querySelector("button")!.getAttribute("aria-label")).toBe("Use these numbers: 16, 32");
  const applied = onApply.mock.calls[0][0] as Spec;
  expect(applied.entry.all[0]).toEqual({ lhs: { s: "rsi", p: { length: 16 } }, op: "<", rhs: { value: 32 } });
  expect(applied.exit).toEqual(spec.exit);
  expect(applied.optimize).toBeUndefined();
});

it("shows an engine refusal as text, and leaves out the drawdown column when the result has none", async () => {
  const refusal = Object.assign(new Error("entry.all[0].rhs.value: the optimizer refused this range."), { status: 400, kind: "engine", engineStatus: "invalid" });
  const onOptimize = vi.fn().mockRejectedValueOnce(refusal).mockResolvedValueOnce(fakeResult(TABLE_COLS.filter((c) => c !== "max_dd")));
  const host = await mount(<SimpleOptimize {...props} onOptimize={onOptimize} />);
  await toggle(host, T);
  await act(async () => runButton(host).click());
  await flush();
  expect(host.querySelector('[role="alert"]')!.textContent).toBe("The test did not run: entry.all[0].rhs.value: the optimizer refused this range.");
  expect(runButton(host).disabled).toBe(false);
  await act(async () => runButton(host).click());
  await flush();
  expect(host.querySelector('[role="alert"]')).toBeNull();
  const heads = [...host.querySelectorAll("[data-lab-sopt-result] thead th")].map((th) => th.textContent);
  expect(heads).toEqual(["Buy when RSI(N) is below 30", "Buy when RSI(14) is below N", "Net profit", "Trades", "Win rate", "Use"]);
});
