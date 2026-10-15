import { afterEach, expect, it, vi } from "vitest";
import { act, useState, type ComponentProps } from "react";
import { OptimizePanel, colValue } from "../src/features/lab/OptimizePanel";
import { num, pct } from "../src/features/lab/format";
import { defaultSpec, setAt } from "../src/features/lab/spec-model";
import type { OptimizeResult, Spec } from "../src/features/lab/types";
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

it("the Values column is never empty: fixed when unticked, the value count when ticked; the run controls share one row", async () => {
  const host = await mount(<OptimizePanel {...base} onOptimize={vi.fn()} />);
  const values = () => host.querySelector('[data-lab-dim="entry.all[0].rhs.p.length"] .lab-dim-values')!.textContent;
  expect(values()).toBe("fixed");
  await tick(host, "entry.all[0].rhs.p.length");
  expect(values()).toBe("5");
  const row = host.querySelector(".lab-optimize-run")!;
  expect(row.contains(host.querySelector('[data-lab-action="optimize"]'))).toBe(true);
  expect(row.textContent).toContain("Held back");
  expect(row.textContent).toContain("Variants tried this session: 0");
});

it("onTried fires once per finished optimization", async () => {
  const onTried = vi.fn();
  const host = await mount(<OptimizePanel {...base} onTried={onTried} onOptimize={vi.fn().mockResolvedValue(optimizeFixture)} />);
  await tick(host, "entry.all[0].rhs.p.length");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await flush();
  expect(onTried).toHaveBeenCalledTimes(1);
  expect(onTried).toHaveBeenCalledWith(3);
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

const P = "entry.all[0].rhs.p.length";
// The backend ranks on the in-sample window and returns the ranked values as is_* (api.rs rank_basis).
const IS_EXTRA = ["is_monthly_thickness", "is_expectancy_ratio", "is_mar"];
const splitFx: OptimizeResult = {
  ...optimizeFixture, cols: [...optimizeFixture.cols, ...IS_EXTRA], base: [...optimizeFixture.base, 0.2, 0.08, 0.3],
  picked: { "1": [...optimizeFixture.picked["1"], 0.33, 0.13, 0.5], "0": [...optimizeFixture.picked["0"], 0.29, 0.1, 0.4] },
};
splitFx.picked["1"][splitFx.cols.indexOf("is_cagr")] = 0.11; // differs from the full-period cagr 0.07
const runWith = async (fx: OptimizeResult, extra: Partial<ComponentProps<typeof OptimizePanel>> = {}) => {
  const host = await mount(<OptimizePanel {...base} {...extra} onOptimize={vi.fn().mockResolvedValue(fx)} />);
  await tick(host, P);
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await flush();
  return host;
};
const rowCells = (host: HTMLElement, n: number) => [...host.querySelectorAll("table.ocmp tbody tr")[n].children].map((c) => c.textContent);

it("with a split the ranking columns show the in-sample values with their percentiles", async () => {
  const host = await runWith(splitFx);
  const one = rowCells(host, 1);
  const pick = splitFx.picked["1"];
  expect(one[2]).toBe(`${pct(pick[splitFx.cols.indexOf("is_cagr")])} · p90`);
  expect(one[2]).not.toContain(pct(pick[splitFx.cols.indexOf("cagr")]));
  expect(one[3]).toBe(`${num(pick[splitFx.cols.indexOf("is_sharpe")])} · p85`);
  expect(one[6]).toBe(`${num(pick[splitFx.cols.indexOf("is_mar")])} · p88`);
  expect(rowCells(host, 0)[2]).toBe(pct(splitFx.base[splitFx.cols.indexOf("is_cagr") - 1]));
});

it("without a split the ranking columns show the full-period values", async () => {
  const host = await runWith({ ...splitFx, split: null, ranked_on: "full" });
  expect(host.textContent).toContain("Ranked on: full period");
  expect(rowCells(host, 1)[2]).toBe(`${pct(0.07)} · p90`);
  expect(host.textContent).not.toContain("Held back · display only");
});

it("dim values are formatted; a top row without picked values gets no Apply; Apply buttons are named", async () => {
  const fx: OptimizeResult = { ...optimizeFixture, picked: { "1": [0.1 + 0.2, ...optimizeFixture.picked["1"].slice(1)] } };
  const host = await runWith(fx);
  expect(rowCells(host, 1)[1]).toBe("0.30");
  expect(rowCells(host, 0)[1]).toBe("50");
  const buttons = [...host.querySelectorAll<HTMLButtonElement>("table.ocmp tbody button")];
  expect(buttons.map((b) => b.getAttribute("aria-label"))).toEqual(["Apply #1"]);
});

it("gl results carry the provisional strip and the instrument label", async () => {
  const host = await runWith(optimizeFixture, { market: "gl", instrumentLabel: "DAX · IG CFD" });
  const strip = host.querySelector("[data-lab-optimize-result] [data-lab-provisional]");
  expect(strip?.textContent).toContain("DAX · IG CFD");
});

it("a failed run after a successful one clears the table", async () => {
  const onOptimize = vi.fn().mockResolvedValueOnce(optimizeFixture).mockRejectedValueOnce(new Error("engine failed"));
  const host = await mount(<OptimizePanel {...base} onOptimize={onOptimize} />);
  await tick(host, P);
  const go = () => act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await go(); await flush();
  expect(host.querySelector("table.ocmp")).not.toBeNull();
  await go(); await flush();
  expect(host.querySelector("table.ocmp")).toBeNull();
  expect(host.querySelector("[data-lab-error]")).not.toBeNull();
});

it("429: the main button stays disabled until the countdown ends", async () => {
  vi.useFakeTimers();
  const busy = Object.assign(new Error("An optimization is already running. Please retry when it finishes."), { status: 429, kind: "gateway", retryAfter: 5 });
  const host = await mount(<OptimizePanel {...base} onOptimize={vi.fn().mockRejectedValueOnce(busy)} />);
  await tick(host, P);
  const main = () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!;
  await act(async () => main().click());
  await act(async () => { await Promise.resolve(); });
  expect(main().disabled).toBe(true);
  await act(async () => { vi.advanceTimersByTime(4000); });
  expect(main().disabled).toBe(true);
  await act(async () => { vi.advanceTimersByTime(1100); });
  expect(main().disabled).toBe(false);
});

type Props = ComponentProps<typeof OptimizePanel>;
let update: (p: Partial<Props>) => void = () => {};
function Harness(p: Props) {
  const [o, set] = useState<Partial<Props>>({});
  update = (x) => set((cur) => ({ ...cur, ...x }));
  return <OptimizePanel {...p} {...o} />;
}

it("a market change resets the result and ignores a run still in flight", async () => {
  let resolve: (r: OptimizeResult) => void = () => {};
  const onOptimize = vi.fn().mockResolvedValueOnce(optimizeFixture).mockReturnValueOnce(new Promise<OptimizeResult>((r) => { resolve = r; }));
  const host = await mount(<Harness {...base} onOptimize={onOptimize} />);
  await tick(host, P);
  const go = () => act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await go(); await flush();
  expect(host.querySelector("[data-lab-optimize-result]")).not.toBeNull();
  await act(async () => update({ market: "us" }));
  expect(host.querySelector("[data-lab-optimize-result]")).toBeNull();
  await go();
  await act(async () => update({ market: "in" }));
  await act(async () => resolve(optimizeFixture));
  await flush();
  expect(host.querySelector("[data-lab-optimize-result]")).toBeNull();
  expect(host.querySelector('[data-lab-action="optimize"]')!.hasAttribute("disabled")).toBe(false);
});

it("the Now row shows the settings that were optimized, not later edits", async () => {
  const host = await mount(<Harness {...base} onOptimize={vi.fn().mockResolvedValue(optimizeFixture)} />);
  await tick(host, P);
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await flush();
  await act(async () => update({ spec: setAt(defaultSpec(), P, 60) }));
  expect(rowCells(host, 0)[1]).toBe("50");
});

it("blocks Optimize when the spec with dims is too long, and explains a hold-back without a run", async () => {
  const long = { ...defaultSpec(), name: "x".repeat(7600) } as Spec;
  const host = await mount(<OptimizePanel {...base} spec={long} dates={[]} onOptimize={vi.fn()} />);
  await tick(host, "entry.all[0].rhs.p.length");
  expect(host.textContent).toContain("These rules are too long to send.");
  expect(host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.disabled).toBe(true);
  const split = host.querySelector<HTMLSelectElement>('select[aria-label="Held back"]')!;
  await act(async () => { split.value = "30"; split.dispatchEvent(new Event("change", { bubbles: true })); });
  expect(host.textContent).toContain("Run a backtest first to set the hold-back date.");
  expect(host.textContent).not.toContain("Not enough history");
});

it("offers nested settings under group captions and sends their exact path", async () => {
  const spec = setAt(defaultSpec(), "entry.all[1]", { any: [{ lhs: { s: "close" }, op: ">", rhs: { s: "ema", p: { length: 20 } } }] });
  const onOptimize = vi.fn().mockResolvedValue(optimizeFixture);
  const host = await mount(<OptimizePanel {...base} spec={spec} onOptimize={onOptimize} />);
  expect([...host.querySelectorAll("tr.lab-dim-group")].map((r) => r.textContent)).toEqual(["Entry", "Exit"]);
  await tick(host, "entry.all[1].any[0].rhs.p.length");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  expect(onOptimize.mock.calls[0][0].optimize[0].path).toBe("entry.all[1].any[0].rhs.p.length");
});
it("optimization shows server execution notes and adaptations without inventing risk events", async () => {
  const result: OptimizeResult = { ...optimizeFixture, execution_model: { kind: "index", mode: "unit", executable: false, notes: ["Server note"], events: null }, adaptations: ["Server adaptation"] };
  const host = await mount(<OptimizePanel {...base} onOptimize={async () => result} />);
  await tick(host, "entry.all[0].rhs.p.length");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  expect(host.querySelector("[data-lab-optimize-result] [data-lab-execution-notes]")?.textContent).toBe("Server note");
  expect(host.querySelector("[data-lab-optimize-result] [data-lab-adaptations]")?.textContent).toBe("Server adaptation");
  expect(host.querySelector("[data-lab-events]")).toBeNull();
});
it("money optimization is labelled Money even without run-only ledger totals", async () => {
  const result: OptimizeResult = { ...optimizeFixture, money: { capital: 100000, basis: "Declared costs" }, execution_model: { kind: "etf", mode: "money", executable: true, notes: [], events: null } };
  const host = await mount(<OptimizePanel {...base} onOptimize={async () => result} />);
  await tick(host, "entry.all[0].rhs.p.length");
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  expect(host.querySelector("[data-lab-optimize-result] [data-lab-money]")?.textContent).toContain("Money");
  expect(host.querySelector("[data-lab-optimize-result]")?.textContent).not.toContain("Unit study");
});
it("a changed portfolio cohort invalidates pending results and their Apply actions",async()=>{
 const {portfolioCatalogueFixture}=await import("./fixtures/portfolio");let release!:(r:OptimizeResult)=>void;const onOptimize=vi.fn(()=>new Promise<OptimizeResult>(resolve=>{release=resolve;}));
 const start:Spec={...defaultSpec(),execution:{capital:1000},portfolio:{universe:{basket:"nifty50"},mode:"signals",max_open:2}};
 function Harness(){const[spec,setSpec]=useState(start);return <><button onClick={()=>setSpec({...spec,portfolio:{...spec.portfolio!,universe:{tickers:["A","B"]}}})}>Change cohort</button><OptimizePanel {...base} catalogue={portfolioCatalogueFixture} spec={spec} onOptimize={onOptimize}/></>;}
 const host=await mount(<Harness/>);await tick(host,"portfolio.max_open");
 await act(async()=>host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());expect(onOptimize).toHaveBeenCalledTimes(1);
 await act(async()=>[...host.querySelectorAll<HTMLButtonElement>("button")].find(b=>b.textContent==="Change cohort")!.click());
 await act(async()=>release(optimizeFixture));await flush();expect(host.querySelector('[data-lab-optimize-result]')).toBeNull();expect(host.querySelector('[aria-label="Apply #1"]')).toBeNull();
 expect(host.querySelector<HTMLSelectElement>('[aria-label="Held back"]')).toBeNull();
 expect(host.querySelector(".empty")?.textContent).toBe("Portfolio optimization ranks the full period using shared cash.");
});
