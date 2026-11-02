import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import * as lab from "../src/features/lab/client";
import { OptimizePanel } from "../src/features/lab/OptimizePanel";
import { evalTemplate, optimizationWorkbook, sheetFileName } from "../src/features/lab/optimize-sheet";
import { xlsx } from "../src/features/lab/xlsx";
import { defaultSpec } from "../src/features/lab/spec-model";
import type { OptimizeResult, Spec } from "../src/features/lab/types";
import { catalogueFixture as cat, fetchRouter, flush, jsonResponse, mount, optimizeFixture, unmount } from "./fixtures/lab";

const saved: { name: string; bytes: Uint8Array }[] = [];
vi.mock("../src/features/lab/xlsx", async (orig) => ({ ...(await orig<typeof import("../src/features/lab/xlsx")>()), saveBytes: (bytes: Uint8Array, name: string) => saved.push({ name, bytes }) }));
afterEach(async () => { await unmount(); vi.unstubAllGlobals(); saved.length = 0; });

const P = "entry.all[0].rhs.p.length";
const COLS = ["Net Profit", "Net % Profit", "Exposure %", "CAR", "RAR", "Max. Trade Drawdown", "Max. Trade % Drawdown", "Max. Sys Drawdown", "Max. Sys % Drawdown",
  "Recovery Factor", "CAR/MDD", "RAR/MDD", "Profit Factor", "Payoff Ratio", "Standard Error", "RRR", "Ulcer Index", "Ulcer Perf. Index", "Sharpe Ratio", "K-Ratio",
  "# Trades", "Avg Profit/Loss", "Avg % Profit/Loss", "Avg Bars Held", "# of winners", "% of Winners", "W. Tot. Profit", "W. Avg. Profit", "W. Avg % Profit",
  "W. Avg. Bars Held", "# of losers", "% of Losers", "L. Tot. Loss", "L. Avg. Loss", "L. Avg % Loss", "L. Avg. Bars Held"];
const row = (dim: number, carMdd: number | null) => [dim, ...COLS.map((c, i) => (c === "CAR/MDD" ? carMdd : c === "Net Profit" ? null : i + 0.5))];
const withSheet: OptimizeResult = { ...optimizeFixture, sheet_cols: COLS, sheet: [row(10, 0.2), row(20, 0.9), row(30, null), row(40, 0.5)] };
const spec: Spec = { ...defaultSpec(), name: "EMA cross", direction: "short", optimize: [{ path: P, from: 10, to: 40, step: 10 }] } as Spec;
const base = { spec: defaultSpec(), catalogue: cat, market: "in" as const, role: "researcher" as const, dates: [], onApply: vi.fn(), onTried: vi.fn(), tried: 0 };
const runWith = async (fx: OptimizeResult) => {
  const host = await mount(<OptimizePanel {...base} onOptimize={vi.fn().mockResolvedValue(fx)} />);
  await act(async () => host.querySelector<HTMLInputElement>(`[data-lab-dim="${P}"] input[type=checkbox]`)!.click());
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize"]')!.click());
  await flush();
  return host;
};

it("single-instrument optimize asks for sheet=1; a portfolio optimize does not", async () => {
  const f = fetchRouter([["/api/", () => jsonResponse({})]]);
  vi.stubGlobal("fetch", f);
  await lab.optimize("in", defaultSpec(), { ticker: "NIFTY 50" });
  await lab.optimize("in", { ...defaultSpec(), portfolio: { universe: { basket: "nifty50" }, mode: "signals", max_open: 2 } } as Spec & { portfolio: NonNullable<Spec["portfolio"]> }, {});
  const [one, many] = f.calls.map((c) => new URL(c.url, "http://x").searchParams);
  expect(one.get("sheet")).toBe("1");
  expect(many.has("sheet")).toBe(false);
});

it("the ranked cells show plain values, as AmiBroker's result list: no pNN suffix, tooltip or caption", async () => {
  const host = await runWith(optimizeFixture);
  const cell = [...host.querySelectorAll("table.ocmp tbody tr")[1].children][2] as HTMLElement;
  expect(cell.textContent).not.toMatch(/· p\d+/);
  expect(cell.title).toBe("");
  expect(host.querySelector("[data-lab-pct-note]")).toBeNull();
});

it("no sheet in the response: no workbook button; with one: the button saves the Optims workbook", async () => {
  let host = await runWith(optimizeFixture);
  expect(host.querySelector('[data-lab-action="optimize-sheet"]')).toBeNull();
  await unmount();
  host = await runWith(withSheet);
  await act(async () => host.querySelector<HTMLButtonElement>('[data-lab-action="optimize-sheet"]')!.click());
  expect(saved.map((s) => s.name)).toEqual(["NIFTY 50 Long Daily Optims.xlsx"]);
  expect(new TextDecoder().decode(saved[0].bytes)).toContain('<sheet name="Optimization"');
});

it("the workbook: Rules rows as the team's, AmiBroker header order, row-relative formulas, sorted on CAR/MDD", () => {
  const book = optimizationWorkbook(withSheet, { ranSpec: spec, label: () => "EMA length", group: () => "Entry", instrumentLabel: "Nifty 50" })!;
  expect(optimizationWorkbook(optimizeFixture, { ranSpec: spec, label: String, group: () => undefined })).toBeNull();
  const [rules, opt] = book;
  expect(rules.rows.map((r) => r.slice(1, 3))).toEqual([
    [], ["Timeframe:", "Daily"], ["Index:", "Nifty 50"], ["Period:", "04-01-2010 to 14-10-2026 (ranked on the window before 03-01-2022)"],
    ["Brokerage:", "5 bps per side"], ["Open Position:", 1], ["%Equity:", ""], ["AFL Path:", "Strategy Vault · Indicators lab"],
    ["AFL Final check by:"], ["Done By:"], ["Verified By:"], [], ["Strategy:", "EMA cross"], ["Position:", "Short"],
    ["Filter:", ""], ["Setup:", ""], ["Entry:", "EMA length - 10,40,10"], ["Exit:", ""]]);
  const head = opt.rows[0];
  expect(head.slice(0, 37)).toEqual(["No.", ...COLS]);
  expect(head.slice(37)).toEqual(["EMA length", "Monthly Thickness", "Expectancy Ratio", "Final Avg % Profit/Loss", "Final W Avg", "Final L Avg", "Final M. Thk",
    "Final Expectancy Ratio", "Total Profit", "Total Net Profit", "Holding Days"]);
  // AmiBroker's columns land where the team's files have them (X = Avg % Profit/Loss, Y = Avg Bars Held, V = # Trades), the dim at AL
  expect(opt.rows.slice(1).map((r) => [r[0], r[37]])).toEqual([[2, 20], [4, 40], [1, 10], [3, 30]]); // CAR/MDD 0.9, 0.5, 0.2, blank
  expect(opt.rows[1][1]).toBeNull(); // a value the lab did not compute stays blank
  expect(opt.rows[1][38]).toEqual({ f: "X2/(Y2-1)*21" });
  expect(opt.rows[2][39]).toEqual({ f: "((AD3/ABS(AJ3))*AA3-AG3)/100" });
  expect(opt.rows[1][40]).toEqual({ f: "X2+(((7/5*(Y2-1))*0.0166)-0.3)" });
  expect(opt.rows[1][44]).toEqual({ f: "((AP2*(AA2/100))-(ABS(AQ2)*(AG2/100)))/(ABS(AQ2))" }); // Final W = AP, Final L = AQ after one dim (AQ/AR after two, as the team file)
  expect(opt.rows[1].slice(45)).toEqual([{ f: "V2*X2" }, { f: "V2*AO2" }, { f: "V2*(Y2-1)*7/5" }]); // short Holding Days as the team's short file
  // hidden as "Need this on Excel Download": No., RAR, Recovery Factor, RAR/MDD, Avg Profit/Loss, # of winners, W. Tot. Profit, W./L. Avg. Bars Held
  expect(opt.hidden!.map((i) => head[i])).toEqual(["No.", "RAR", "Recovery Factor", "RAR/MDD", "Avg Profit/Loss", "# of winners", "W. Tot. Profit", "W. Avg. Bars Held", "L. Avg. Bars Held"]);
  const text = new TextDecoder().decode(xlsx(book));
  // the Optimization sheet's hidden columns keep their fitted width, so unhiding shows them as they are
  const hiddenCols = [...text.matchAll(/<col min="(\d+)" max="\d+" width="\d+" hidden="1"/g)].map((m) => Number(m[1]));
  expect(hiddenCols).toEqual([1, 6, 11, 13, 23, 26, 28, 31, 37]); // A, F, K, M, W, Z, AB, AE, AK
  expect(text).toContain('<c r="AM2" s="6"><f>X2/(Y2-1)*21</f></c>'); // a formula as a two-decimal number
  expect(text).toContain('<autoFilter ref="A1:AV5"/>');
  expect(text).toContain('<calcPr fullCalcOnLoad="1"/>');
  expect(sheetFileName(withSheet, { ranSpec: spec, label: String, group: () => undefined, instrumentLabel: "Nifty 50" })).toBe("Nifty 50 Short Daily Optims.xlsx");
});

it("the cost-adjusted columns follow the side: long as in Long Daily.xlsx, long-and-short only the side-free columns", () => {
  const ctx = (direction: string) => ({ ranSpec: { ...spec, direction } as Spec, label: () => "EMA length", group: () => "Entry" });
  const long = optimizationWorkbook(withSheet, ctx("long"))![1];
  expect(long.rows[1][40]).toEqual({ f: "X2-(((7/5*(Y2-1))*0.033)+0.3)" });
  expect(long.rows[1][47]).toEqual({ f: "V2*(Y2-1)*7/5" });
  expect(optimizationWorkbook(withSheet, ctx("both"))![1].rows[0].slice(38)).toEqual(["Monthly Thickness", "Expectancy Ratio", "Total Profit"]);
});

it("the derived templates evaluate like Excel: arithmetic, ABS, blanks and division by zero stay empty", () => {
  const v: Record<string, number | null> = { a: 2.5, b: 6, l: -1.25, z: null };
  const get = (k: string) => v[k] ?? null;
  expect(evalTemplate("{a}/({b}-1)*21", get)).toBeCloseTo(2.5 / 5 * 21);
  expect(evalTemplate("{a}-(((7/5*({b}-1))*0.033)+0.3)", get)).toBeCloseTo(2.5 - ((7 / 5 * 5) * 0.033 + 0.3));
  expect(evalTemplate("(({a}/ABS({l}))*{b}-{b})/100", get)).toBeCloseTo(((2.5 / 1.25) * 6 - 6) / 100);
  expect(evalTemplate("{z}*2", get)).toBeNull();
  expect(evalTemplate("{a}/({b}-6)", get)).toBeNull();
});

// the team's screen list: the optimized settings first, then only these columns
const SCREEN = ["EMA Period", "CAR", "Max. Sys % Drawdown", "CAR/MDD", "# Trades", "Avg % Profit/Loss", "Avg Bars Held", "% of Winners",
  "Expectancy Ratio", "Final Avg % Profit/Loss", "Final M. Thk", "Final Expectancy Ratio", "Total Net Profit"];

it("the sheet is on the page too: the team's screen columns, sorted on CAR/MDD, click to sort", async () => {
  const host = await runWith(withSheet);
  const list = host.querySelector("[data-lab-optimize-sheet-list]")!;
  const head = [...list.querySelectorAll("thead th")].map((th) => th.textContent);
  expect(head.map((h) => h!.replace(/^Entry 1: /, ""))).toEqual(SCREEN); // settings first, then the team's screen list
  const settings = () => [...list.querySelectorAll("tbody tr")].map((tr) => tr.children[head.indexOf("Entry 1: EMA Period")].textContent);
  expect(settings()).toEqual(["20", "40", "10", "30"]); // CAR/MDD 0.9, 0.5, 0.2, blank
  const er = head.indexOf("Expectancy Ratio");
  expect(list.querySelector("tbody tr")!.children[er].textContent).not.toBe(""); // a computed value, not a formula
  await act(async () => list.querySelectorAll<HTMLButtonElement>("thead th button")[head.indexOf("Entry 1: EMA Period")].click());
  expect(settings()).toEqual(["40", "30", "20", "10"]); // a setting sorts high to low first
  expect(host.querySelector("[data-lab-optimize-all]")).toBeNull(); // replaces the short All results list
});

it("the top rows (Now, #1-#3) show the same screen columns, each the result-list row of its settings", async () => {
  const fx: OptimizeResult = { ...withSheet, split: null, ranked_on: "full", sheet: [row(10, 0.2), row(20, 0.9), row(30, null), row(40, 0.5), row(50, 0.3)] };
  let host = await runWith(withSheet);
  expect(host.querySelector("[data-lab-optimize-top]")).toBeNull(); // a held-back split keeps its ranking / held-back columns
  await unmount();
  host = await runWith(fx);
  const top = host.querySelector("[data-lab-optimize-top]")!;
  const head = [...top.querySelectorAll("thead th")].map((th) => th.textContent);
  expect(head.slice(1, -1).map((h) => h!.replace(/^Entry 1: /, ""))).toEqual(SCREEN);
  const at = (name: string, col: string) => [...top.querySelectorAll("tbody tr")].find((tr) => tr.children[0].textContent === name)!.children[head.indexOf(col)].textContent;
  // #1 is EMA 20 and #2 EMA 10: each carries its sheet row; "Now" (EMA 50 in the default spec) finds its own row
  expect([at("#1", "Entry 1: EMA Period"), at("#1", "CAR/MDD")]).toEqual(["20", "0.90"]);
  expect([at("#2", "Entry 1: EMA Period"), at("#2", "CAR/MDD")]).toEqual(["10", "0.20"]);
  expect([at("Now", "Entry 1: EMA Period"), at("Now", "CAR/MDD")]).toEqual(["50", "0.30"]);
  expect(top.querySelectorAll('button[aria-label^="Apply"]')).toHaveLength(2);
});

it("xlsx without formulas or a filter is unchanged: no calcPr, no autoFilter", () => {
  const text = new TextDecoder().decode(xlsx([{ name: "A", rows: [["x"], [1]] }]));
  expect(text).not.toContain("calcPr");
  expect(text).not.toContain("autoFilter");
});
