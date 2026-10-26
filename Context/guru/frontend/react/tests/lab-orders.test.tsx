import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { COPY } from "../src/features/lab/labels";
import { defaultSpec, setAt, setExecution } from "../src/features/lab/spec-model";
import { DEFAULT_SHORT_SIDE, withDirection, withEntryOrder, withTakeProfits } from "../src/features/lab/execution-model";
import type { Catalogue, Spec } from "../src/features/lab/types";
import { catalogue7Fixture, catalogueFixture } from "./fixtures/lab";
import { OrdersPanel } from "../src/features/lab/OrdersPanel";

const active: { root: Root; host: HTMLElement }[] = [];
afterEach(async () => { for (const { root, host } of active) { await act(async () => root.unmount()); host.remove(); } active.length = 0; });
async function setup(spec: Spec, pre: "" | "short_side." = "", catalogue: Catalogue = catalogue7Fixture) {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host); active.push({ root, host });
  const onChange = vi.fn();
  const rerender = async (s: Spec) => { await act(async () => root.render(<OrdersPanel spec={s} catalogue={catalogue} pre={pre} onChange={onChange} />)); };
  await rerender(spec);
  return { host, last: () => onChange.mock.calls.at(-1)![0] as Spec, rerender };
}
const click = async (el: Element) => act(async () => (el as HTMLElement).click());
const clickIn = async (host: HTMLElement, group: string, text: string) => {
  const b = [...host.querySelectorAll<HTMLButtonElement>(`${group} button`)].find((b) => b.textContent === text);
  expect(b, `${group} ${text}`).toBeDefined(); await click(b!);
};
const choose = async (el: Element, value: string) => act(async () => { (el as HTMLSelectElement).value = value; el.dispatchEvent(new Event("change", { bubbles: true })); });
const type = async (el: Element, value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(el, value);
  el.dispatchEvent(new Event("input", { bubbles: true }));
});
const orders = setup;

it("entry orders: stop, At level with an operand editor, stop-limit hint, order duration", async () => {
  const { host, last, rerender } = await orders(setExecution(defaultSpec(), { fill: "close" }));
  await clickIn(host, '[aria-label="Long entry order"]', "Stop");
  expect(last().entry.stop).toEqual({ pct: 1 });
  expect(last().execution).toEqual({ exit_fill: "close" });
  await rerender(last());
  await choose(host.querySelector('select[aria-label="Stop unit"]')!, "at");
  expect(last().entry.stop).toEqual({ at: { highest: { s: "high" }, bars: 5 } });
  await rerender(withEntryOrder(last(), "", "stop_limit"));
  expect(host.textContent).toContain(COPY.stopLimitHint);
  await type(host.querySelector('input[aria-label="Order stays (bars)"]')!, "3");
  expect(last().entry.valid_bars).toBe(3);
});

it("AmiBroker stops: four fixed groups in order, Disabled removes, unsupported options are not selectable", async () => {
  const { host, last, rerender } = await orders(defaultSpec(), "", catalogue7Fixture);
  expect([...host.querySelectorAll(".lab-stops > fieldset > legend")].map((l) => l.textContent)).toEqual(["Maximum loss stop", "Profit target", "Trailing stop", "N-bar stop"]);
  const unit = () => host.querySelector<HTMLSelectElement>('[data-lab-exit="stop_loss"] select[aria-label="Stop-loss unit"]')!;
  expect(unit().value).toBe("off");
  await choose(unit(), "pct");
  expect(last().exit.stop_loss).toEqual({ pct: 5 });
  await rerender(last());
  await choose(unit(), "off");
  expect(last().exit).not.toHaveProperty("stop_loss");
  // exit modes, N-bar priority and re-entry delays are single-instrument settings shown with `reentry`
  expect(host.querySelector('select[aria-label="Target exit mode"]')).toBeNull();
  expect(host.querySelector('[data-lab-exit="n_bar"] input[type=checkbox]')).toBeNull();
  expect(host.querySelector('input[aria-label="Re-entry delay (bars)"]')).toBeNull();
  expect(host.textContent).not.toContain("not modelled");
});

it("Exit at stop? per stop and N-bar Has priority, with AmiBroker wording and disclosures", async () => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host); active.push({ root, host });
  const onChange = vi.fn();
  const render = async (s: Spec) => act(async () => root.render(<OrdersPanel spec={s} catalogue={catalogue7Fixture} reentry onChange={onChange} />));
  await render(defaultSpec());
  const mode = (label: string) => host.querySelector<HTMLSelectElement>(`select[aria-label="${label} exit mode"]`)!;
  expect(["Stop-loss", "Target", "Trailing stop"].map((l) => mode(l).value)).toEqual(["1", "1", "1"]);
  expect(host.querySelector('select[aria-label="N-bar stop exit mode"]')).toBeNull(); // AmiBroker's N-bar stop has only Has priority
  expect([...mode("Target").options].map((o) => [o.value, o.textContent, o.disabled])).toEqual([
    ["0", "Exit at trade price only (0)", false], ["1", "Exit intraday at stop (1)", false], ["2", "Exit next bar at trade price (2)", false]]);
  await choose(mode("Target"), "2");
  expect(onChange.mock.calls.at(-1)![0].execution.exit_at_stop).toEqual({ target: 2 });
  await render(setExecution(defaultSpec(), { exit_at_stop: { target: 2, bars: 0 } }));
  expect(mode("Target").value).toBe("2");
  await choose(mode("Target"), "1"); // back to the default: not stored
  expect(onChange.mock.calls.at(-1)![0].execution.exit_at_stop).toEqual({ bars: 0 });
  const priority = host.querySelector<HTMLInputElement>('[data-lab-exit="n_bar"] input[type=checkbox]')!;
  expect([priority.checked, priority.disabled, priority.parentElement!.textContent]).toEqual([false, false, "Has priority"]);
  await act(async () => priority.click());
  expect(onChange.mock.calls.at(-1)![0].execution.nbar_has_priority).toBe(true);
  expect(host.querySelector("[data-lab-stop-modes]")!.textContent).toContain("next bar's open");
  expect(host.textContent).toContain("The N-bar stop has no stop price");
});

it("each stop carries its own re-entry delay when asked", async () => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host); active.push({ root, host });
  const onChange = vi.fn();
  await act(async () => root.render(<OrdersPanel spec={defaultSpec()} catalogue={catalogue7Fixture} reentry onChange={onChange} />));
  expect(host.querySelectorAll('input[aria-label="Re-entry delay (bars)"]')).toHaveLength(4);
  await type(host.querySelector('[data-lab-exit="trailing"] input[aria-label="Re-entry delay (bars)"]')!, "3");
  expect(onChange.mock.calls.at(-1)![0].execution.reentry).toEqual({ trailing: 3 });
});

it("exits: add menu lists only unset extra exits; trailing activation; breakeven; take-profits in % of the position; min bars", async () => {
  const { host, last, rerender } = await orders(defaultSpec());
  const add = () => host.querySelector<HTMLSelectElement>('select[aria-label="Add exit (long)"]')!;
  expect([...add().options].map((o) => o.value)).toEqual(["", "breakeven", "take_profits", "min_bars", "dual_atr", "dual_line"]);
  await choose(host.querySelector('[data-lab-exit="trailing"] select[aria-label="Trailing stop unit"]')!, "pct");
  await rerender(last());
  await choose(host.querySelector('[data-lab-exit="trailing"] select[aria-label="Trailing activation unit"]')!, "pct");
  expect(last().exit.trailing).toEqual({ pct: 5, activate: { pct: 2 } });
  await choose(add(), "breakeven");
  expect(last().exit.breakeven).toEqual({ after: { pct: 2 } });
  await rerender(withTakeProfits(defaultSpec(), "", [{ pct: 5, size: 0.5 }]));
  expect(host.querySelector<HTMLInputElement>('[data-lab-exit="take_profits"] input[aria-label="Take-profit 1 size (%)"]')!.value).toBe("50");
  await type(host.querySelector('[data-lab-exit="take_profits"] input[aria-label="Take-profit 1 size (%)"]')!, "30");
  expect(last().exit.take_profits).toEqual([{ pct: 5, size: 0.3 }]);
  await rerender(withTakeProfits(defaultSpec(), "", [{ pct: 1, size: 0.2 }, { pct: 2, size: 0.2 }, { pct: 3, size: 0.2 }]));
  expect(host.querySelector('[data-lab-exit="take_profits"] button[aria-label="Add take-profit"]')).toBeNull();
  await choose(add(), "min_bars");
  expect(last().exit.min_bars).toBe(1);
  await rerender(last());
  expect(host.textContent).toContain(COPY.minBarsHint);
  await click(host.querySelector('[data-lab-exit="min_bars"] button[aria-label="Remove Minimum bars"]')!);
  expect(last().exit).not.toHaveProperty("min_bars");
});

it("the short side writes to short_side paths and labels itself", async () => {
  const both = withDirection(defaultSpec(), "both");
  const { host, last } = await orders(both, "short_side.");
  expect(host.querySelector('[data-lab-orders="short"]')).not.toBeNull();
  await choose(host.querySelector('[data-lab-exit="stop_loss"] select[aria-label="Stop-loss unit"]')!, "pct");
  expect(last().short_side?.exit.stop_loss).toEqual({ pct: 5 });
  expect(last().exit).not.toHaveProperty("stop_loss");
});

it("scale-in shows only with the engine's execution options and refuses take-profits", async () => {
  const none = await orders(defaultSpec(), "", catalogueFixture);
  expect(none.host.querySelector('input[aria-label="Scale-in legs (% of full size)"]')).toBeNull();
  const { host, last, rerender } = await orders(defaultSpec());
  await click(host.querySelector('input[type=checkbox][aria-label="Scale in"]')!);
  expect(last().entry.scale_in).toEqual({ legs: [0.5, 0.5] });
  await rerender(last());
  await type(host.querySelector('input[aria-label="Scale-in legs (% of full size)"]')!, "50, 25, 25");
  expect(last().entry.scale_in).toEqual({ legs: [0.5, 0.25, 0.25] });
  const tp = await orders(withTakeProfits(defaultSpec(), "", [{ pct: 5, size: 0.5 }]));
  expect(tp.host.querySelector<HTMLInputElement>('input[type=checkbox][aria-label="Scale in"]')!.disabled).toBe(true);
  expect(tp.host.textContent).toContain(COPY.scaleInWithTp);
});

it("exit level edits preserve origin and unknown fields without mixing distance forms", async () => {
  const spec = setAt(setAt(defaultSpec(), "origin", { book: "stc9", method: "f3a", template_version: 1 }), "exit.take_profits", [{ pct: 5, size: 0.5, future_note: "keep" }]);
  const before = JSON.stringify(spec);
  const { host, last } = await orders(spec);
  await choose(host.querySelector('select[aria-label="Take-profit 1 unit"]')!, "atr");
  expect(last().exit.take_profits).toEqual([{ atr: 2, atr_length: 14, size: 0.5, future_note: "keep" }]);
  expect(last().origin).toEqual(spec.origin);
  expect(JSON.stringify(spec)).toBe(before);
});

it("scale-in prevents adding take-profits and malformed legs stay visible without a crash", async () => {
  const scaled = setAt(defaultSpec(), "entry.scale_in", { legs: [0.5, 0.5] });
  const { host } = await orders(scaled);
  expect(host.querySelector<HTMLOptionElement>('select[aria-label="Add exit (long)"] option[value="take_profits"]')!.disabled).toBe(true);
  expect(host.textContent).toContain(COPY.scaleInWithTp);
  const malformed = await orders(setAt(defaultSpec(), "entry.scale_in", { legs: "future" }));
  expect(malformed.host.querySelector(".lab-kept")?.textContent).toContain('"legs":"future"');
});

it("future take-profit row shapes stay read-only and survive other order edits", async () => {
  const source = setAt(defaultSpec(), "exit.take_profits", [null, { future_level: 3 }]);
  const { host, last } = await orders(source);
  expect(host.querySelector('[data-lab-exit="take_profits"] .lab-kept')?.textContent).toContain("future_level");
  await clickIn(host, '[aria-label="Long entry order"]', "Stop");
  expect(last().exit.take_profits).toEqual([null, { future_level: 3 }]);
});

it("trailing stop: choosing % of profit writes exit.trailing { profit_pct: 40 }", async () => {
  const { host, last } = await orders(setAt(defaultSpec(), "exit.trailing", { pct: 5 }));
  await choose(host.querySelector('[data-lab-exit="trailing"] select[aria-label="Trailing stop unit"]')!, "profit_pct");
  expect(last().exit.trailing).toEqual({ profit_pct: 40 });
  expect([...host.querySelectorAll<HTMLOptionElement>('select[aria-label="Trailing stop unit"] option')].map((o) => o.value)).toContain("profit_pct");
});

it("Two-line exit: added from More exits with the MultiTF Supertrend lines, each line edited as an operand, removable", async () => {
  const { host, last, rerender } = await orders(defaultSpec());
  await choose(host.querySelector('select[aria-label="Add exit (long)"]')!, "dual_line");
  expect(last().exit.dual_line).toEqual({ initial: { s: "supertrend", p: { atr_length: 5, factor: 1, mode: 1 }, out: "supertrend" },
    favour: { s: "supertrend", p: { atr_length: 5, factor: 4, mode: 1 }, out: "supertrend" } });
  await rerender(last());
  const favour = host.querySelector('[role="group"][aria-label="Favour line"]')!;
  expect(favour).not.toBeNull();
  expect(host.querySelector('[role="group"][aria-label="Initial line"]')).not.toBeNull();
  expect(host.textContent).toContain(COPY.dualLineHint);
  await click(host.querySelector('button[aria-label="Remove Two-line exit (initial → favour)"]')!);
  expect(last().exit.dual_line).toBeUndefined();
});
it("Dual ATR exit: added from More exits, bands, timeframe and the period-end test; not in a portfolio", async () => {
  const { host, last, rerender } = await orders(defaultSpec());
  await choose(host.querySelector('select[aria-label="Add exit (long)"]')!, "dual_atr");
  expect(last().exit.dual_atr).toEqual({ initial: { period: 5, mult: 2, price: "hl2" }, favour: { period: 5, mult: 4, price: "hl2" } });
  await rerender(last());
  await type(host.querySelector('input[aria-label="Favour ATR ×"]')!, "3");
  expect(last().exit.dual_atr!.favour).toEqual({ period: 5, mult: 3, price: "hl2" });
  await rerender(last());
  await choose(host.querySelector('select[aria-label="Initial price"]')!, "open");
  expect(last().exit.dual_atr!.initial.price).toBe("open");
  await rerender(last());
  expect(host.textContent).not.toContain("last bar");
  await clickIn(host, '[aria-label="Dual ATR timeframe"]', "Monthly");
  expect(last().exit.dual_atr!.tf).toBe("M");
  await rerender(last());
  await click([...host.querySelectorAll("label.lab-check")].find((l) => l.textContent === "Exit only on the month's last bar")!.querySelector("input")!);
  expect(last().exit.dual_atr!.period_end_only).toBe(true);
  await rerender(last());
  await clickIn(host, '[aria-label="Dual ATR timeframe"]', "Daily");
  expect(last().exit.dual_atr).toEqual({ initial: { period: 5, mult: 2, price: "open" }, favour: { period: 5, mult: 3, price: "hl2" } });
  await click(host.querySelector('button[aria-label="Remove Dual ATR exit"]')!);
  expect(last().exit.dual_atr).toBeUndefined();
  const port = await orders({ ...defaultSpec(), execution: { capital: 1e6 }, portfolio: { universe: { basket: "nifty50" } } } as Spec);
  expect(port.host.querySelector<HTMLOptionElement>('select[aria-label="Add exit (long)"] option[value="dual_atr"]')!.disabled).toBe(true);
});
