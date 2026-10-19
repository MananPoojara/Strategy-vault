import { afterEach, expect, it, vi } from "vitest";
import { act, useState } from "react";
import { FormulaBlocks } from "../src/features/lab/FormulaBlocks";
import { FormulaEditor } from "../src/features/lab/FormulaEditor";
import { emptyBlocks, fn, generate, price, readBlocks, type Blocks } from "../src/features/lab/blocks-model";
import { LabError } from "../src/features/lab/client";
import type { FormulaInfo } from "../src/features/lab/types";
import { catalogueFixture as cat, flush, mount, unmount } from "./fixtures/lab";

const inds = cat.indicators;
const info: FormulaInfo = { params: [], plots: ["Line 1"], placement: "pane" };
afterEach(async () => { await unmount(); vi.restoreAllMocks(); delete (document as { elementFromPoint?: unknown }).elementFromPoint; });

function harness(start: { formula: string; blocks: unknown }, extra: { check?: (f: string) => Promise<FormulaInfo>; showFormula?: () => void } = {}) {
  const calls: { formula: string; blocks: Blocks }[] = [];
  function H() {
    const [s, setS] = useState(start);
    return <FormulaBlocks formula={s.formula} blocks={s.blocks} indicators={inds} debounceMs={0} {...extra}
      onChange={(formula, blocks) => { calls.push({ formula, blocks: blocks as Blocks }); setS({ formula, blocks }); }} />;
  }
  return { calls, node: <H /> };
}
const fromModel = (m: Blocks) => ({ formula: generate(m).text, blocks: m });
const last = (calls: { formula: string }[]) => calls[calls.length - 1]?.formula;
const q = <T extends Element = HTMLElement>(host: Element, sel: string) => host.querySelector<T & HTMLElement>(sel)!;
const pal = (host: Element, id: string) => q(host, `[data-fb-pal="${id}"]`);
const click = (el: Element) => act(async () => (el as HTMLElement).click());
const focus = (el: Element) => act(async () => (el as HTMLElement).focus());
const key = (el: Element, k: string, mods: KeyboardEventInit = {}) => act(async () => { el.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, ...mods })); });
const typeText = (el: HTMLInputElement, value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(el, value);
  el.dispatchEvent(new Event("input", { bubbles: true }));
});
const pointer = (target: EventTarget, type: string, x: number, y: number) =>
  act(async () => { target.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y, button: 0, pointerId: 1, pointerType: "touch" })); });
/** Drag `from` and release over `over` (elementFromPoint is stubbed: jsdom has no layout). */
async function drag(from: Element, over: () => Element | null) {
  (document as { elementFromPoint?: unknown }).elementFromPoint = () => over();
  await pointer(from, "pointerdown", 10, 10);
  await pointer(window, "pointermove", 60, 40);
  await pointer(window, "pointermove", 120, 80);
}
const drop = () => pointer(window, "pointerup", 120, 80);

it("pointer drag from the palette fills a slot and reports formula + blocks", async () => {
  const h = harness(fromModel(emptyBlocks()));
  const host = await mount(h.node);
  await drag(pal(host, "price:C"), () => q(host, '[data-fb-slot="p0"]'));
  expect(document.querySelector(".fb-ghost")!.textContent).toBe("Close");
  expect(q(host, '[data-fb-slot="p0"]').getAttribute("data-over")).toBe("ok");
  await drop();
  expect(document.querySelector(".fb-ghost")).toBeNull();
  expect(last(h.calls)).toBe('Plot(C, "Line 1");');
  expect(readBlocks(h.calls[0].blocks)).toEqual({ v: 1, lets: [], plots: [{ name: "Line 1", onPrice: false, expr: price("C") }] });
  expect(q(host, '[aria-label="Generated formula"]').textContent).toBe('Plot(C, "Line 1");');
});

it("a wrong-type drop is refused with a visible reason", async () => {
  const h = harness(fromModel({ v: 1, lets: [], plots: [{ name: "X", onPrice: false, expr: fn("ema", null, null) }] }));
  const host = await mount(h.node);
  await drag(pal(host, "price:C"), () => q(host, '[data-fb-slot="p0.1"]'));
  expect(q(host, '[data-fb-slot="p0.1"]').getAttribute("data-over")).toBe("no");
  expect(document.querySelector(".fb-ghost small")!.textContent).toMatch(/length slot needs a whole number/);
  await drop();
  expect(h.calls).toHaveLength(0);
  expect(q(host, '[role="status"]').textContent).toMatch(/The length slot needs a whole number .* Close gives a series/);
  expect(q(host, '[role="status"]').className).toContain("err");
});

it("placed blocks move between slots and drop on the bin to delete", async () => {
  const h = harness(fromModel({ v: 1, lets: [], plots: [{ name: "X", onPrice: false, expr: fn("sub", price("C"), null) }] }));
  const host = await mount(h.node);
  await drag(q(host, '[data-fb-block="p0.0"]'), () => q(host, '[data-fb-slot="p0.1"]'));
  expect(q(host, '[data-fb-block="p0.0"]').getAttribute("data-dragging")).toBe("true");
  await drop();
  expect(last(h.calls)).toBe('Plot(? - C, "X");');
  await drag(q(host, '[data-fb-block="p0.1"]'), () => q(host, "[data-fb-bin]"));
  await drop();
  expect(last(h.calls)).toBe('Plot(? - ?, "X");');
  // a block cannot be dropped inside itself
  await drag(q(host, '[data-fb-block="p0"]'), () => q(host, '[data-fb-slot="p0.1"]'));
  await drop();
  expect(q(host, '[role="status"]').textContent).toBe("A block cannot go inside itself.");
});

it("keyboard: focus a slot, choose blocks with Enter/click, arrows move, Delete removes; undo and redo", async () => {
  const h = harness(fromModel(emptyBlocks()));
  const host = await mount(h.node);
  await focus(q(host, '[data-fb-slot="p0"]'));
  await typeText(q<HTMLInputElement>(host, 'input[aria-label="Search blocks"]'), "ema");
  expect([...host.querySelectorAll("[data-fb-pal]")].map((b) => b.getAttribute("data-fb-pal"))).toEqual(["fn:ema"]);
  await click(pal(host, "fn:ema"));
  expect(last(h.calls)).toBe('Plot(EMA(?, ?), "Line 1");');
  expect(q(host, '[data-fb-slot="p0.0"]').getAttribute("aria-current")).toBe("true"); // the next empty slot is selected
  await typeText(q<HTMLInputElement>(host, 'input[aria-label="Search blocks"]'), "");
  await click(pal(host, "price:C"));
  await click(pal(host, "num"));
  expect(last(h.calls)).toBe('Plot(EMA(C, 1), "Line 1");');
  await act(async () => { const i = q<HTMLInputElement>(host, 'input[aria-label="Number in Line 1"]'); Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(i, "2.5"); i.dispatchEvent(new Event("input", { bubbles: true })); });
  expect(last(h.calls)).toBe('Plot(EMA(C, 1), "Line 1");'); // 2.5 is refused in a whole-number slot
  expect(q(host, '[role="status"]').textContent).toMatch(/length needs a whole number/);
  await typeText(q<HTMLInputElement>(host, 'input[aria-label="Number in Line 1"]'), "21");
  expect(last(h.calls)).toBe('Plot(EMA(C, 21), "Line 1");');
  const ema = q(host, '[data-fb-block="p0"]');
  await focus(ema);
  await key(ema, "ArrowRight");
  expect(document.activeElement).toBe(q(host, '[data-fb-block="p0.0"]'));
  await key(document.activeElement!, "Delete");
  expect(last(h.calls)).toBe('Plot(EMA(?, 21), "Line 1");');
  expect(document.activeElement).toBe(q(host, '[data-fb-slot="p0.0"]'));
  await key(document.activeElement!, "z", { ctrlKey: true });
  expect(last(h.calls)).toBe('Plot(EMA(C, 21), "Line 1");');
  await key(q(host, ".fb-canvas"), "z", { ctrlKey: true, shiftKey: true });
  expect(last(h.calls)).toBe('Plot(EMA(?, 21), "Line 1");');
  await click([...host.querySelectorAll("button")].find((b) => b.textContent === "Undo")!);
  expect(last(h.calls)).toBe('Plot(EMA(C, 21), "Line 1");');
  await click([...host.querySelectorAll("button")].find((b) => b.textContent === "Redo")!);
  expect(last(h.calls)).toBe('Plot(EMA(?, 21), "Line 1");');
});

it("rows: named results, parameters, lines on price; results rename everywhere", async () => {
  const h = harness(fromModel(emptyBlocks()));
  const host = await mount(h.node);
  const button = (t: string) => [...host.querySelectorAll("button")].find((b) => b.textContent === t)!;
  await click(button("+ Name a result"));
  expect(document.activeElement).toBe(q(host, '[data-fb-slot="l0"]'));
  await click(pal(host, "param"));
  expect(last(h.calls)).toBe('Value = Param("Periods", 14, 1, 200, 1);\nPlot(?, "Line 1");');
  await click(q(host, '[aria-label="Edit Parameter Periods"]'));
  await typeText(q<HTMLInputElement>(host, 'input[aria-label="Parameter Periods default"]'), "20");
  await typeText(q<HTMLInputElement>(host, '[aria-label="Parameter Periods settings"] input[type=text]'), "Len");
  expect(last(h.calls)).toBe('Value = Param("Len", 20, 1, 200, 1);\nPlot(?, "Line 1");');
  await focus(q(host, '[data-fb-slot="p0"]'));
  expect(host.querySelector('[data-fb-pal="fn:ma"]')).toBeNull(); // groups past "My results" start closed
  await click([...host.querySelectorAll(".fb-pal-h")].find((b) => b.textContent!.startsWith("Smoothing & windows"))!);
  await click(pal(host, "fn:ma"));
  await click(pal(host, "price:C"));
  await click(pal(host, "ref:Value"));
  await typeText(q<HTMLInputElement>(host, 'input[aria-label="Name of result 1"]'), "N");
  expect(last(h.calls)).toBe('N = Param("Len", 20, 1, 200, 1);\nPlot(MA(C, N), "Line 1");');
  await click(q(host, 'input[type=checkbox]'));
  await typeText(q<HTMLInputElement>(host, 'input[aria-label="Name of line 1"]'), 'Mid "x"');
  expect(last(h.calls)).toBe('N = Param("Len", 20, 1, 200, 1);\nPlot(MA(C, N), "Mid \\"x\\"", "price");');
  await click(q(host, '[aria-label="Remove result N"]'));
  expect(q(host, '[role="status"]').textContent).toMatch(/still used/);
  await click(button("+ Add line"));
  expect(last(h.calls)).toMatch(/Plot\(\?, "Line 2"\);$/);
});

it("live Check runs debounced and maps an engine error back to the block", async () => {
  const check = vi.fn(async (f: string) => { if (f.includes("O")) throw new LabError("line 1, col 6: bad price here", 400, "engine", "invalid"); return info; });
  const h = harness(fromModel({ v: 1, lets: [], plots: [{ name: "Line 1", onPrice: false, expr: price("C") }] }), { check });
  const host = await mount(h.node);
  await flush();
  expect(check).toHaveBeenCalledWith('Plot(C, "Line 1");');
  expect(host.textContent).toContain("The engine accepts these blocks");
  await drag(pal(host, "price:O"), () => q(host, '[data-fb-slot="p0"]'));
  await drop();
  await flush();
  expect(q(host, '[role="alert"]').textContent).toContain("line 1, col 6: bad price here");
  expect(q(host, '[data-fb-block="p0"]').getAttribute("aria-invalid")).toBe("true");
  // with an empty slot, the engine is not asked; the problems list says what to finish
  check.mockClear();
  expect(q<HTMLButtonElement>(host, 'button[aria-label="Remove line 1"]').disabled).toBe(true); // the last line stays
  await focus(q(host, '[data-fb-block="p0"]'));
  await key(q(host, '[data-fb-block="p0"]'), "Backspace");
  await flush();
  expect(check).not.toHaveBeenCalled();
  expect(host.textContent).toContain("Line 1: the line value slot is empty.");
});

it("formula-only indicators: Convert to blocks, or say why and go back to the Formula tab", async () => {
  const h = harness({ formula: 'x = EMA(C, 9); // smooth\nPlot(x, "A")', blocks: null });
  const host = await mount(h.node);
  expect(host.textContent).toContain("comments are not kept");
  await click([...host.querySelectorAll("button")].find((b) => b.textContent === "Convert to blocks")!);
  expect(h.calls[0].formula).toBe('x = EMA(C, 9);\nPlot(x, "A");');
  expect(host.querySelector('[data-fb-block="l0"]')).not.toBeNull();
  await unmount();
  const showFormula = vi.fn();
  const bad = harness({ formula: 'Plot(FOO(C), "A")', blocks: null }, { showFormula });
  const host2 = await mount(bad.node);
  expect(q(host2, '[role="alert"]').textContent).toContain("line 1, col 6: unknown function FOO");
  await click([...host2.querySelectorAll("button")].find((b) => b.textContent === "Edit on the Formula tab")!);
  expect(showFormula).toHaveBeenCalled();
  // a stale layout (formula edited elsewhere) is not trusted
  await unmount();
  const stale = harness({ formula: 'Plot(O, "A");', blocks: emptyBlocks() });
  const host3 = await mount(stale.node);
  expect(host3.textContent).toContain("Convert to blocks");
});

it("editor: a new indicator opens on Blocks and saves the formula with its blocks", async () => {
  const create = vi.fn(async (d: { name: string; formula: string; blocks: unknown }) => ({ id: 1, version: 1, createdAt: "", updatedAt: "", ...d }));
  const host = await mount(<FormulaEditor row={null} functions={[]} canEdit check={async () => ({ ...info, params: [{ key: "Periods", default: 14, min: 2, max: 200, step: 1 }], plots: ["Osc"] })}
    api={{ create, update: vi.fn(), remove: vi.fn() }} onSaved={vi.fn()} onDeleted={vi.fn()} onClose={vi.fn()}
    blocksEditor={(p) => <FormulaBlocks {...p} indicators={inds} />} />);
  expect(host.querySelector('[role="tab"][aria-selected="true"]')!.textContent).toBe("Blocks");
  expect(q(host, '[aria-label="Generated formula"]').textContent).toBe('N = Param("Periods", 14, 2, 200, 1);\nPlot(C - MA(C, N), "Osc");');
  await typeText(q<HTMLInputElement>(host, ".lab-formula-name input"), "Osc");
  await click([...host.querySelectorAll("button")].find((b) => b.textContent === "Save")!);
  expect(create).toHaveBeenCalledWith({ name: "Osc", formula: 'N = Param("Periods", 14, 2, 200, 1);\nPlot(C - MA(C, N), "Osc");', blocks: expect.objectContaining({ v: 1 }) });
  // typing on the Formula tab detaches the layout; Blocks then offers to convert
  await click(q(host, '[role="tab"]:not([aria-selected="true"])'));
  await act(async () => { const t = q<HTMLTextAreaElement>(host, "textarea"); Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!.call(t, 'Plot(H, "Hi")'); t.dispatchEvent(new Event("input", { bubbles: true })); });
  await click(q(host, '[role="tab"]:not([aria-selected="true"])'));
  expect(host.textContent).toContain("This indicator was written on the Formula tab.");
});
