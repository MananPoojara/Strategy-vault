import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act } from "react";
import * as lab from "../src/features/lab/client";
import { FormulaContext, errorPosition, formulaIndicator, formulaLabel, parseFormulaInfo, type FormulaScope } from "../src/features/lab/formula";
import { FormulaEditor, type FormulaEditorProps } from "../src/features/lab/FormulaEditor";
import { LibraryRail } from "../src/features/lab/LibraryRail";
import { OperandEditor } from "../src/features/lab/OperandEditor";
import { Workbench } from "../src/features/lab/Workbench";
import { optimizablePaths } from "../src/features/lab/optimize-paths";
import { defaultSpec } from "../src/features/lab/spec-model";
import { operandKind } from "../src/features/lab/rule-tree";
import { LabError } from "../src/features/lab/client";
import type { FormulaInfo, Operand, SavedIndicator, Spec } from "../src/features/lab/types";
import { catalogueFixture as cat, catalogue7Fixture as cat7, fetchRouter, flush, jsonResponse, mount, seriesFixture, unmount } from "./fixtures/lab";

const FORMULA = 'Fast = Param("Fast", 12, 2, 50, 1);\nOsc = EMA(C, Fast) - EMA(C, 26);\nPlot(Osc, "Osc");\nPlot(EMA(Osc, 9), "Signal");';
const info: FormulaInfo = { params: [{ key: "Fast", default: 12, min: 2, max: 50, step: 1 }], plots: ["Osc", "Signal"], placement: "pane" };
const row: SavedIndicator = { id: 7, name: "My Momentum", formula: FORMULA, blocks: null, version: 2, createdAt: "2026-10-19", updatedAt: "2026-10-19" };

beforeEach(() => lab.clearLabCache());
afterEach(async () => { await unmount(); vi.unstubAllGlobals(); delete window.LightweightCharts; });
const click = (el: Element | null) => act(async () => (el as HTMLElement).click());
const choose = (el: HTMLSelectElement, value: string) => act(async () => { el.value = value; el.dispatchEvent(new Event("change", { bubbles: true })); });
const typeText = (el: HTMLTextAreaElement | HTMLInputElement, value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(Object.getPrototypeOf(el), "value")!.set!.call(el, value);
  el.dispatchEvent(new Event("input", { bubbles: true }));
});

it("client: checks formulas, loads formula series and stores indicators with blocks", async () => {
  const f = fetchRouter([
    [/\/lab\/formula/, () => jsonResponse({ ok: true, ...info })],
    [/\/lab\/series/, () => jsonResponse({ ...seriesFixture("x", ["Osc"], { Osc: [1] }), formula: info })],
    ["/api/app/lab/indicators", (_u, init) => jsonResponse(init?.method === "DELETE" ? null : init?.method ? row : [row], init?.method === "DELETE" ? 204 : 200)],
  ]);
  vi.stubGlobal("fetch", f);
  expect(await lab.formulaInfo("in", FORMULA)).toEqual(info);
  await lab.formulaInfo("in", FORMULA); // cached per text
  await lab.series("in", { id: "my:7", ticker: "NIFTY 50", tf: "W", p: { Fast: 9 }, formula: FORMULA, name: "My Momentum" });
  expect(await lab.indicators.list()).toEqual([row]);
  await lab.indicators.create({ name: "A", formula: "Plot(C, \"C\");", blocks: { b: 1 } });
  await lab.indicators.update(7, 2, { name: "A", formula: "Plot(C, \"C\");", blocks: null });
  expect(await lab.indicators.remove(7, 2)).toBeNull();
  const urls = f.calls.map((c) => new URL(c.url, "http://x"));
  expect(urls[0].pathname).toBe("/api/in/lab/formula");
  expect(urls[0].searchParams.get("formula")).toBe(FORMULA);
  expect(f.calls.filter((c) => c.url.includes("lab/formula"))).toHaveLength(1);
  expect(urls[1].pathname).toBe("/api/in/lab/series");
  expect(Object.fromEntries(urls[1].searchParams)).toEqual({ formula: FORMULA, name: "My Momentum", ticker: "NIFTY 50", tf: "W", Fast: "9" });
  expect(JSON.parse(String(f.calls[3].init?.body))).toEqual({ name: "A", formula: 'Plot(C, "C");', blocks: { b: 1 } });
  expect(f.calls[4].url).toBe("/api/app/lab/indicators/7");
  expect(JSON.parse(String(f.calls[4].init?.body))).toMatchObject({ version: 2, blocks: null });
  expect(f.calls[5].url).toBe("/api/app/lab/indicators/7?version=2");
});

it("formula helpers: parse, labels, error positions and chart indicator", () => {
  expect(() => parseFormulaInfo({ params: [{ key: "x" }], plots: [] })).toThrow();
  expect(parseFormulaInfo({ params: [], plots: ["A"], placement: "price" }).placement).toBe("price");
  expect(errorPosition("line 2, col 5: unknown function FOO")).toEqual({ line: 2, col: 5 });
  expect(formulaLabel({ formula: FORMULA, name: "My Momentum", p: { Fast: 9 }, out: "Osc" })).toBe("My Momentum(Fast=9).Osc");
  expect(formulaLabel({ formula: FORMULA, name: "My Momentum", p: { Fast: 12 } }, info)).toBe("My Momentum");
  const ind = formulaIndicator(row, info);
  expect(ind).toMatchObject({ id: "my:7", placement: "pane", outputs: ["Osc", "Signal"], formula: FORMULA, computed: true });
  expect(ind.params[0]).toMatchObject({ key: "Fast", label: "Fast", integer: true, step: 1 });
  expect(operandKind({ formula: FORMULA, name: "x" } as Operand, cat7)).toBe("formula");
});

it("library: My indicators comes first, starts closed, adds and edits", async () => {
  const onAdd = vi.fn(), onEdit = vi.fn();
  const host = await mount(<LibraryRail catalogue={cat} market="in" hasVolume tf="D" onAdd={vi.fn()} onRead={vi.fn()}
    mine={{ list: [row], canEdit: true, onAdd, onEdit }} />);
  const heads = [...host.querySelectorAll<HTMLButtonElement>(".lab-group-h")];
  expect(heads[0].textContent).toBe("My indicators 1");
  expect(heads[0].getAttribute("aria-expanded")).toBe("false");
  await click(heads[0]);
  await click(host.querySelector('[data-lab-item="my:7"]'));
  expect(onAdd).toHaveBeenCalledWith(row);
  await click(host.querySelector('[aria-label="Edit My Momentum"]'));
  expect(onEdit).toHaveBeenCalledWith(row);
  await click([...host.querySelectorAll("button")].find((b) => b.textContent === "+ New indicator")!);
  expect(onEdit).toHaveBeenLastCalledWith(null);
});

const editor = (patch: Partial<FormulaEditorProps> = {}) => {
  const props: FormulaEditorProps = {
    row: null, functions: [{ name: "EMA", args: "x, n", desc: "Exponential moving average" }], canEdit: true,
    check: vi.fn(async () => info), api: { create: vi.fn(async (d) => ({ ...row, id: 8, version: 1, ...d })), update: vi.fn(async (_i, _v, d) => ({ ...row, version: 3, ...d })), remove: vi.fn(async () => null) },
    onSaved: vi.fn(), onDeleted: vi.fn(), onClose: vi.fn(), ...patch,
  };
  return props;
};

it("editor: Check shows Params and Plots; Save sends name, formula and blocks", async () => {
  const p = editor();
  const host = await mount(<FormulaEditor {...p} />);
  const button = (text: string) => [...host.querySelectorAll("button")].find((b) => b.textContent === text)!;
  await typeText(host.querySelector<HTMLTextAreaElement>('textarea[aria-label="Formula"]')!, FORMULA);
  await click(button("Check"));
  expect(host.querySelector(".lab-formula-result")!.textContent).toContain("Fast 12 (2–50, step 1)");
  expect(host.querySelector(".lab-formula-result")!.textContent).toContain("Osc, Signal");
  await click(button("Save"));
  expect(host.textContent).toContain("Give the indicator a name.");
  await typeText(host.querySelector<HTMLInputElement>(".lab-formula-name input")!, "My Momentum");
  await click(button("Save"));
  expect(p.api.create).toHaveBeenCalledWith({ name: "My Momentum", formula: FORMULA, blocks: null });
  expect(p.onSaved).toHaveBeenCalledWith(expect.objectContaining({ id: 8 }), info);
  // now an existing row: Save updates it, Save as new appears
  await click(button("Save"));
  expect(p.api.update).toHaveBeenCalledWith(8, 1, { name: "My Momentum", formula: FORMULA, blocks: null });
  expect(button("Save as new")).toBeDefined();
  expect(host.textContent).toContain("EMA(x, n)");
  expect(host.textContent).toContain("Bollinger %B");
});

it("editor: an error shows its line and column and blocks saving", async () => {
  const p = editor({ row, check: vi.fn(async () => { throw new LabError("line 2, col 7: unknown function FOO", 400, "engine", "invalid"); }) });
  const host = await mount(<FormulaEditor {...p} />);
  await flush(); // an opened indicator is checked at once
  expect(host.querySelector('[role="alert"]')!.textContent).toContain("line 2, col 7: unknown function FOO");
  expect(host.querySelector("mark")!.textContent).toBe("E") // line 2 "Osc = EMA(…", col 7;
  expect(host.querySelector("textarea")!.getAttribute("aria-invalid")).toBe("true");
  await click([...host.querySelectorAll("button")].find((b) => b.textContent === "Save")!);
  expect(p.api.update).not.toHaveBeenCalled();
  expect(host.textContent).toContain("Fix the formula error before saving.");
});

it("editor: the Blocks tab drives the same draft through the blocksEditor slot", async () => {
  let drive: ((formula: string, blocks: unknown) => void) | undefined;
  const p = editor({ row: { ...row, blocks: { root: 1 } }, blocksEditor: ({ onChange, blocks }) => { drive = onChange; return <p data-blocks>{JSON.stringify(blocks)}</p>; } });
  const host = await mount(<FormulaEditor {...p} />);
  expect(host.querySelector('[role="tab"][aria-selected="true"]')!.textContent).toBe("Blocks");
  expect(host.querySelector("[data-blocks]")!.textContent).toBe('{"root":1}');
  await act(async () => drive!('Plot(C, "C");', { root: 2 }));
  await click([...host.querySelectorAll("button")].find((b) => b.textContent === "Save")!);
  expect(p.api.update).toHaveBeenCalledWith(7, 2, { name: "My Momentum", formula: 'Plot(C, "C");', blocks: { root: 2 } });
});

it("chart: a custom indicator chip loads its series by formula", async () => {
  window.LightweightCharts = { createChart: vi.fn(() => ({ addCandlestickSeries: () => ({ setData: vi.fn(), setMarkers: vi.fn() }), addLineSeries: () => ({ setData: vi.fn() }),
    remove: vi.fn(), resize: vi.fn(), applyOptions: vi.fn(), timeScale: () => ({ fitContent: vi.fn(), setVisibleRange: vi.fn() }) })), CrosshairMode: { Normal: 0 } } as never;
  const ind = formulaIndicator(row, info);
  const load = vi.fn(async (q: { id: string }) => seriesFixture(q.id, q.id.startsWith("my:") ? ["Osc", "Signal"] : [q.id], { [q.id]: [1, 2], Osc: [1, 2], Signal: [1, 2] }));
  const host = await mount(<Workbench market="in" ticker="NIFTY 50" tf="D" range="All" catalogue={{ ...cat, indicators: [...cat.indicators, ind] }}
    chips={[{ key: "c1", id: "my:7", params: { Fast: 9 } }]} loadSeries={load} onChipChange={vi.fn()} onChipRemove={vi.fn()} />);
  await flush();
  expect(load).toHaveBeenCalledWith({ id: "my:7", tf: "D", p: { Fast: 9 }, vs: undefined, formula: FORMULA, name: "My Momentum" });
  expect(host.querySelector('[data-lab-chip="c1"]')!.textContent).toContain("My Momentum");
  expect(host.querySelector('[data-lab-pane="c1"]')).not.toBeNull();
  await click(host.querySelector('[data-lab-chip="c1"] .lab-chip-open'));
  expect(host.querySelector('input[aria-label="My Momentum Fast"]')).not.toBeNull();
});

const scope: FormulaScope = { mine: [row], infos: { [FORMULA]: info } };
it("rules: the operand picker inserts a My indicator with its output and params", async () => {
  const onChange = vi.fn();
  const host = await mount(<FormulaContext.Provider value={scope}>
    <OperandEditor operand={{ s: "close" }} side="entry" catalogue={cat7} onChange={onChange} label="Entry 1 left" />
  </FormulaContext.Provider>);
  await choose(host.querySelector('select[aria-label="Operand type"]')!, "formula");
  const inserted = onChange.mock.calls.at(-1)![0];
  expect(inserted).toEqual({ formula: FORMULA, name: "My Momentum", p: { Fast: 12 }, out: "Osc" });
  await unmount();
  const next = vi.fn();
  const h2 = await mount(<FormulaContext.Provider value={scope}>
    <OperandEditor operand={inserted} side="entry" catalogue={cat7} onChange={next} label="Entry 1 left" />
  </FormulaContext.Provider>);
  await choose(h2.querySelector('select[aria-label="Output"]')!, "Signal");
  expect(next.mock.calls.at(-1)![0]).toMatchObject({ out: "Signal" });
  await typeText(h2.querySelector<HTMLInputElement>('input[aria-label="Fast"]')!, "9");
  expect(next.mock.calls.at(-1)![0]).toMatchObject({ p: { Fast: 9 } });
});

it("optimize: formula Params are listed as My Momentum › Fast", () => {
  const spec: Spec = { ...defaultSpec(), entry: { all: [{ lhs: { formula: FORMULA, name: "My Momentum", p: { Fast: 9 }, out: "Osc" }, op: ">", rhs: { value: 0 } }] } };
  expect(optimizablePaths(spec, cat7).some((s) => s.path.includes(".p.Fast"))).toBe(false); // unknown until checked
  expect(optimizablePaths(spec, cat7, scope.infos).find((s) => s.path === "entry.all[0].lhs.p.Fast"))
    .toMatchObject({ label: "Entry 1: My Momentum › Fast", current: 9, min: 2, max: 50, step: 1, integer: true });
});
