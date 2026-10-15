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

it("exits: add menu lists only unset exits; trailing activation; breakeven; take-profits in % of the position; min bars", async () => {
  const { host, last, rerender } = await orders(defaultSpec());
  const add = () => host.querySelector<HTMLSelectElement>('select[aria-label="Add exit (long)"]')!;
  await choose(add(), "trailing");
  await rerender(last());
  expect([...add().options].map((o) => o.value)).not.toContain("trailing");
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
  await choose(host.querySelector('select[aria-label="Add exit (short)"]')!, "stop_loss");
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
