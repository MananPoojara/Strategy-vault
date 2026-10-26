import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { ForeignProvider } from "../src/features/lab/ForeignFields";
import { OperandEditor } from "../src/features/lab/OperandEditor";
import { DEFAULT_STATE_COND } from "../src/features/lab/rule-tree";
import { EXPAND_FIRST_REFUSED } from "../src/features/lab/labels";
import type { Operand } from "../src/features/lab/types";
import { catalogue7Fixture, mount, unmount } from "./fixtures/lab";

afterEach(unmount);
const setup = async (operand: Operand, extra: Partial<Parameters<typeof OperandEditor>[0]> = {}) => {
  await unmount();
  const onChange = vi.fn();
  const host = await mount(<OperandEditor operand={operand} side="entry" catalogue={catalogue7Fixture} onChange={onChange} label="Entry 1 left" {...extra} />);
  return { host, onChange, last: () => onChange.mock.calls.at(-1)?.[0] as Operand };
};
const choose = async (el: HTMLSelectElement, value: string) => act(async () => { el.value = value; el.dispatchEvent(new Event("change", { bubbles: true })); });
const type = async (el: HTMLInputElement, value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(el, value);
  el.dispatchEvent(new Event("input", { bubbles: true }));
});
const click = async (el: HTMLElement) => act(async () => el.click());

it("offers grouped kinds; bars held only when allowed; formula kinds disabled at the cap", async () => {
  const { host } = await setup({ s: "close" }, { side: "entry" });
  const kind = host.querySelector<HTMLSelectElement>('select[aria-label="Operand type"]')!;
  expect([...kind.querySelectorAll("optgroup")].map((g) => g.label)).toEqual(["Formula", "Since a condition"]);
  expect(kind.querySelector('option[value="bars_held"]')).toBeNull();
  const held = await setup({ s: "close" }, { side: "exit", heldOk: true });
  expect(held.host.querySelector('option[value="bars_held"]')).not.toBeNull();
  const full = await setup({ s: "close" }, { nodesFull: true });
  expect(full.host.querySelector<HTMLOptionElement>('option[value="ref"]')!.disabled).toBe(true);
});

it("wraps into N bars ago, swaps to a rolling window keeping the inner value, edits bars, and unwraps", async () => {
  const { host, last } = await setup({ s: "rsi", p: { length: 2 } });
  await choose(host.querySelector('select[aria-label="Operand type"]')!, "ref");
  expect(last()).toEqual({ ref: { s: "rsi", p: { length: 2 } }, bars: 1 });
  const ref = await setup({ ref: { s: "rsi", p: { length: 2 } }, bars: 1 });
  expect(ref.host.querySelector(".lab-operand")!.getAttribute("data-lab-operand-kind")).toBe("ref");
  await choose(ref.host.querySelector('select[aria-label="Operand type"]')!, "window");
  expect(ref.last()).toEqual({ avg: { s: "rsi", p: { length: 2 } }, bars: 10 });
  const win = await setup({ avg: { s: "close" }, bars: 10 });
  await choose(win.host.querySelector('select[aria-label="Window"]')!, "highest");
  expect(win.last()).toEqual({ highest: { s: "close" }, bars: 10 });
  await type(win.host.querySelector('input[aria-label="bars"]')!, "20");
  expect(win.last()).toEqual({ avg: { s: "close" }, bars: 20 });
  await click(win.host.querySelector('button[aria-label="Remove formula"]')!);
  expect(win.last()).toEqual({ s: "close" });
});

it("arithmetic: operator select and two nested operands", async () => {
  const { host, last } = await setup({ sub: [{ s: "close" }, { value: 1 }] });
  await choose(host.querySelector('select[aria-label="Arithmetic"]')!, "div");
  expect(last()).toEqual({ div: [{ s: "close" }, { value: 1 }] });
  expect(host.querySelectorAll(".lab-operand .lab-operand")).toHaveLength(2);
});

it("state operands render their condition through renderNode, or read-only without it", async () => {
  const renderNode = vi.fn(() => <span data-testid="node" />);
  const { host } = await setup({ bars_since: DEFAULT_STATE_COND }, { renderNode });
  expect(renderNode).toHaveBeenCalledWith(DEFAULT_STATE_COND, expect.any(Function), expect.stringContaining("condition"));
  expect(host.querySelector('[data-testid="node"]')).not.toBeNull();
  const plain = await setup({ bars_since: DEFAULT_STATE_COND });
  expect(plain.host.textContent).toContain("close crosses above Exponential Moving Average(50)");
});

it("timing stays folded until needed; N sessions; expand first is offered only where the engine accepts it", async () => {
  const { host, last } = await setup({ s: "close" });
  expect(host.querySelector('select[aria-label="Timeframe"]')).toBeNull();
  await click(host.querySelector("button[data-lab-timing]")!);
  await choose(host.querySelector('select[aria-label="Timeframe"]')!, "nD");
  expect(last()).toEqual({ s: "close", tf: "5D" });
  const w = await setup({ s: "close", tf: "W" });
  expect(w.host.querySelector("button[data-lab-timing]")!.textContent).toContain("Weekly");
  await click(w.host.querySelector("button[data-lab-timing]")!);
  const expand = w.host.querySelector<HTMLSelectElement>('select[aria-label="Use the period value"]')!;
  expect(expand.querySelector<HTMLOptionElement>('option[value="first"]')!.disabled).toBe(true);
  expect(w.host.textContent).toContain(EXPAND_FIRST_REFUSED);
  const open = await setup({ s: "open", tf: "W" });
  await click(open.host.querySelector("button[data-lab-timing]")!);
  expect(open.host.querySelector<HTMLOptionElement>('option[value="first"]')!.disabled).toBe(false);
  const shifted = await setup({ s: "close", tf: "W", offset: 1, expand: "first" });
  await click(shifted.host.querySelector("button[data-lab-timing]")!);
  await type(shifted.host.querySelector('input[aria-label="periods ago"]')!, "0");
  expect(shifted.last()).toEqual({ s: "close", tf: "W" }); // first is not valid at offset 0, so it is dropped
});

it("calendar: one select with plain labels writes id and output", async () => {
  const { host, last } = await setup({ s: "close" });
  await choose(host.querySelector('select[aria-label="Operand type"]')!, "calendar");
  expect(last()).toEqual({ s: "session", out: "tdom" });
  const cal = await setup({ s: "session", out: "tdom" });
  const sel = cal.host.querySelector<HTMLSelectElement>('select[aria-label="Calendar value"]')!;
  expect([...sel.options].map((o) => o.textContent)).toContain("Sessions left in the month · future dates");
  await choose(sel, "sessions_left.m");
  expect(cal.last()).toEqual({ s: "sessions_left", out: "m" });
});

it("calendar: Bar number shows as itself; a value missing from the list never shows as the first option", async () => {
  const bar = await setup({ s: "calendar", out: "bar_index" });
  const sel = bar.host.querySelector<HTMLSelectElement>('select[aria-label="Calendar value"]')!;
  expect(sel.selectedOptions[0].textContent).toBe("Bar number (BarIndex, 0 = first bar)");
  await unmount();
  const odd = await setup({ s: "calendar", out: "future_out" });
  expect(odd.host.querySelector<HTMLSelectElement>('select[aria-label="Calendar value"]')!.selectedOptions[0].textContent).toBe("calendar.future_out");
});

it("compress: timeframe without D, mode, and first-expand only with mode first or offset", async () => {
  const { host, last } = await setup({ compress: { s: "high" }, tf: "W", mode: "last" });
  const tf = host.querySelector<HTMLSelectElement>('select[aria-label="Aggregate timeframe"]')!;
  expect([...tf.options].some((o) => o.value === "D")).toBe(false);
  await choose(host.querySelector('select[aria-label="Aggregate"]')!, "first");
  expect(last()).toEqual({ compress: { s: "high" }, tf: "W", mode: "first" });
});

it("unknown operand shapes stay read-only", async () => {
  const { host } = await setup({ fancy: { k: 1 } } as never);
  expect(host.querySelector(".lab-kept")?.textContent).toContain('{"fancy":{"k":1}}');
});

it("changing the period open to a close or indicator removes unsafe expand-first", async () => {
  const price = await setup({ s: "open", tf: "W", expand: "first", future_setting: "kept" });
  await choose(price.host.querySelector('select[aria-label="Price source"]')!, "close");
  expect(price.last()).toEqual({ s: "close", tf: "W", future_setting: "kept" });
  const kind = await setup({ s: "open", tf: "W", expand: "first" });
  await choose(kind.host.querySelector('select[aria-label="Operand type"]')!, "indicator");
  expect(kind.last()).toEqual({ s: "ema", tf: "W" });
});

it("nested edits keep unknown keys, and nested exit operands never offer Bars held", async () => {
  const operand: Operand = { ref: { s: "close", future_series_setting: true }, bars: 2, future_formula_setting: { x: 1 } };
  const before = structuredClone(operand), nested = await setup(operand, { side: "exit" });
  const child = nested.host.querySelector<HTMLElement>('.lab-expr > .lab-operand')!;
  expect(child.querySelector('option[value="bars_held"]')).toBeNull();
  await choose(child.querySelector('select[aria-label="Price source"]')!, "high");
  expect(nested.last()).toEqual({ ...operand, ref: { s: "high", future_series_setting: true } });
  expect(operand).toEqual(before);
  const current = await setup({ ref: { s: "close" }, bars: 2 }, { nodesFull: true });
  expect(current.host.querySelector<HTMLOptionElement>('select[aria-label="Operand type"] > optgroup option[value="ref"]')!.disabled).toBe(false);
  expect(current.host.querySelector<HTMLOptionElement>('select[aria-label="Operand type"] > optgroup option[value="abs"]')!.disabled).toBe(true);
});

it("state value and condition edits are immutable and occurrence/window bounds are enforced", async () => {
  const operand: Operand = { value_when: DEFAULT_STATE_COND, of: { s: "close" }, nth: 1, future_state_setting: 7 };
  const before = structuredClone(operand);
  const state = await setup(operand, { renderNode: (n, onChange) => <button data-test-condition onClick={() => onChange({ not: n })}>Change condition</button> });
  await click(state.host.querySelector('[data-test-condition]')!);
  expect(state.last()).toEqual({ ...operand, value_when: { not: DEFAULT_STATE_COND } });
  await type(state.host.querySelector('input[aria-label="occurrence"]')!, "20");
  expect(state.last()).toEqual({ ...operand, nth: 20 });
  state.onChange.mockClear();
  await type(state.host.querySelector('input[aria-label="occurrence"]')!, "21");
  expect(state.onChange).not.toHaveBeenCalled();
  expect(operand).toEqual(before);
  const since = await setup({ highest_since: DEFAULT_STATE_COND, of: { s: "low" }, future_setting: "kept" });
  await choose(since.host.querySelector('select[aria-label="Highest or lowest"]')!, "lowest_since");
  expect(since.last()).toEqual({ lowest_since: DEFAULT_STATE_COND, of: { s: "low" }, future_setting: "kept" });
  const count = await setup({ count: DEFAULT_STATE_COND, bars: 5 });
  await type(count.host.querySelector('input[aria-label="bars"]')!, "0");
  await type(count.host.querySelector('input[aria-label="bars"]')!, "1.5");
  expect(count.onChange).not.toHaveBeenCalled();
});

it("aggregate changes drop invalid first expansion, custom-session bounds hold, and future dates are disclosed", async () => {
  const aggregate = await setup({ compress: { s: "high" }, tf: "W", mode: "first", expand: "first", future_setting: true });
  await choose(aggregate.host.querySelector('select[aria-label="Aggregate"]')!, "high");
  expect(aggregate.last()).toEqual({ compress: { s: "high" }, tf: "W", mode: "high", future_setting: true });
  const block = await setup({ compress: { s: "close" }, tf: "5D" });
  await type(block.host.querySelector('input[aria-label="sessions"]')!, "63");
  expect(block.last()).toEqual({ compress: { s: "close" }, tf: "63D" });
  block.onChange.mockClear();
  for (const value of ["1", "64", "2.5"]) await type(block.host.querySelector('input[aria-label="sessions"]')!, value);
  expect(block.onChange).not.toHaveBeenCalled();
  const calendar = await setup({ s: "sessions_left", out: "m" });
  expect(calendar.host.textContent).toContain("future session dates");
});

it("formula displays and unwrapping preserve the inner series across every formula kind", async () => {
  const inner: Operand = { s: "rsi", p: { length: 3 }, future_setting: "keep" };
  const operands: Operand[] = [
    { pct: [inner, { s: "close" }] }, { abs: inner }, { change: inner, bars: 2 }, { avg: inner, bars: 4 },
    { ref: inner, bars: 1 }, { compress: inner, tf: "M" }, { value_when: DEFAULT_STATE_COND, of: inner },
    { highest_since: DEFAULT_STATE_COND, of: inner }, { bars_since: DEFAULT_STATE_COND }, { count: DEFAULT_STATE_COND, bars: 3 },
  ];
  for (const operand of operands) {
    const row = await setup(operand);
    expect(row.host.querySelector('.lab-expr')).not.toBeNull();
    expect(row.host.querySelector('select[aria-label="Operand type"]')).not.toBeNull();
    await click(row.host.querySelector('button[aria-label="Remove formula"]')!);
    expect(row.last()).toEqual("bars_since" in operand || "count" in operand ? { s: "close" } : inner);
  }
});

it("shares foreign controls recursively through formula operands only when advertised", async () => {
  const onChange=vi.fn();const catalogue={...catalogue7Fixture,max_foreign:4};
  const host=await mount(<ForeignProvider value={{catalogue,market:"us",ticker:"SPY",options:[{ticker:"SPX",label:"S&P 500",disabled:false,hasVolume:false}]}}>
    <OperandEditor operand={{abs:{s:"rsi",p:{length:2},future:"keep"}}} side="entry" catalogue={catalogue} onChange={onChange} label="Entry left" />
  </ForeignProvider>);
  const select=host.querySelector<HTMLSelectElement>('[aria-label="Entry left › abs · Read from"]'); expect(select).not.toBeNull();
  await choose(select!,"SPX");expect(onChange).toHaveBeenCalledWith({abs:{s:"rsi",p:{length:2},future:"keep",symbol:"SPX"}});
});

it("AmiBroker defaults sets the indicator's AmiBroker parameters over the current ones; indicators without them show no button", async () => {
  const { host, last } = await setup({ s: "stoch", p: { k: 20 } });
  await click(host.querySelector<HTMLButtonElement>("[data-lab-ab-defaults]")!);
  expect(last()).toEqual({ s: "stoch", p: { k: 14, smooth_k: 3, d: 3, variant: 0 } }); // AmiBroker StochK/StochD = variant 0 (AmiBroker 6.40 export)
  const sma = await setup({ s: "sma", p: { length: 20 } });
  expect(sma.host.querySelector("[data-lab-ab-defaults]")).toBeNull();
});

it("a platform variant is picked by name from a dropdown, not typed as 0/1", async () => {
  const { host, last } = await setup({ s: "stoch", p: { length: 14 } });
  const variant = host.querySelector<HTMLSelectElement>('select[aria-label^="Variant"]')!;
  expect(variant.closest("label")!.firstChild!.textContent).toBe("Variant");
  expect([...variant.options].map((o) => o.text)).toEqual(["TradingView", "AmiBroker ratio of sums"]);
  await choose(variant, "1");
  expect(last()).toEqual({ s: "stoch", p: { length: 14, variant: 1 } });
});
