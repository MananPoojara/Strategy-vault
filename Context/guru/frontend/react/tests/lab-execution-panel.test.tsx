import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { COPY } from "../src/features/lab/labels";
import { defaultSpec, setAt, setExecution } from "../src/features/lab/spec-model";
import { DEFAULT_SHORT_SIDE, withDirection, withEntryOrder, withTakeProfits } from "../src/features/lab/execution-model";
import type { Catalogue, Spec } from "../src/features/lab/types";
import { catalogue7Fixture, catalogueFixture } from "./fixtures/lab";
import { ExecutionPanel } from "../src/features/lab/ExecutionPanel";

const active: { root: Root; host: HTMLElement }[] = [];
afterEach(async () => { for (const { root, host } of active) { await act(async () => root.unmount()); host.remove(); } active.length = 0; });
async function setup(spec: Spec, pre: "" | "short_side." = "", catalogue: Catalogue = catalogue7Fixture) {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host); active.push({ root, host });
  const onChange = vi.fn();
  const rerender = async (s: Spec) => { await act(async () => root.render(<ExecutionPanel spec={s} catalogue={catalogue} onChange={onChange} />)); };
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
const exec = setup;

it("Both adds the short side and the opposite setting with help; Long removes them; Both again restores the short side", async () => {
  const { host, last, rerender } = await exec(defaultSpec());
  await clickIn(host, '[aria-label="Positions"]', "Long and short");
  expect(last().short_side).toEqual(DEFAULT_SHORT_SIDE);
  await rerender(setAt(last(), "short_side.entry.all[0].rhs.p.length", 20));
  expect(host.querySelector('[aria-label="When the other side signals"]')).not.toBeNull();
  expect(host.textContent).toContain(COPY.oppositeHelp.reverse);
  expect(host.textContent).toContain(COPY.bothFlat);
  await clickIn(host, '[aria-label="When the other side signals"]', "Exit only");
  expect(last().execution).toEqual({ opposite: "exit" });
  await clickIn(host, '[aria-label="Positions"]', "Long");
  expect(last()).not.toHaveProperty("short_side");
  await rerender(last());
  await clickIn(host, '[aria-label="Positions"]', "Long and short");
  expect(last().short_side?.entry.all[0]).toMatchObject({ rhs: { p: { length: 20 } } });
});

it("fills: entry fill hidden while any side has an order; same close drops the fill delay", async () => {
  const ordered = await exec(withEntryOrder(withDirection(defaultSpec(), "both"), "short_side.", "stop"));
  expect(ordered.host.querySelector('select[aria-label="Entry fill"]')).toBeNull();
  const { host, last } = await exec(setExecution(defaultSpec(), { delay_bars: 2 }));
  await choose(host.querySelector('select[aria-label="Exit fill"]')!, "close");
  expect(last().execution).toEqual({ exit_fill: "close" });
});

it("timing options fold away until set; OHLC path shows its explanation; edge entries", async () => {
  const { host, last } = await exec(defaultSpec());
  expect(host.querySelector('[aria-label="Same-bar order"]')).toBeNull();
  await click(host.querySelector("button[data-lab-timing-options]")!);
  await clickIn(host, '[aria-label="Same-bar order"]', "OHLC path");
  expect(last().execution).toEqual({ intrabar: "ohlc_path" });
  const set = await exec(setExecution(defaultSpec(), { intrabar: "ohlc_path" }));
  expect(set.host.textContent).toContain(COPY.intrabarHelp.ohlc_path);
  await clickIn(set.host, '[aria-label="Enter on"]', "Only the bar they turn true");
  expect(set.last().execution).toMatchObject({ entry_on: "edge" });
});


it("Allow same bar exit / entry signal: checked keeps today's behaviour; unchecked is stored and explained; portfolios keep it fixed", async () => {
  const { host, last, rerender } = await exec(defaultSpec());
  const box = () => [...host.querySelectorAll<HTMLLabelElement>("label.lab-check")].find((l) => l.textContent === "Allow same bar exit / entry signal")!.querySelector("input")!;
  expect([box().checked, box().disabled]).toEqual([true, false]);
  expect(host.querySelector("[data-lab-same-bar]")!.textContent).toContain("Checked (default)");
  await click(box());
  expect(last().execution).toEqual({ allow_same_bar_exit: false });
  await rerender(last());
  expect(box().checked).toBe(false);
  expect(host.querySelector("[data-lab-same-bar]")!.textContent).toContain("no position exits on its entry bar");
  await click(box());
  expect(last()).not.toHaveProperty("execution");
  await rerender({ ...defaultSpec(), execution: { capital: 1e6 }, portfolio: { universe: { basket: "nifty50" } } } as Spec);
  expect(box().disabled).toBe(true);
  expect(host.textContent).not.toContain("not modelled");
});

it("Re-enter on the exit bar is off by default, stores true when ticked and is fixed off in a portfolio", async () => {
  const { host, last, rerender } = await exec(defaultSpec());
  const box = () => [...host.querySelectorAll<HTMLLabelElement>("label.lab-check")].find((l) => l.textContent === "Re-enter on the exit bar")!.querySelector("input")!;
  expect([box().checked, box().disabled]).toEqual([false, false]);
  await click(box());
  expect(last().execution).toEqual({ reenter_on_exit_bar: true });
  await rerender(last());
  await click(box());
  expect(last()).not.toHaveProperty("execution");
  await rerender({ ...defaultSpec(), execution: { capital: 1e6 }, portfolio: { universe: { basket: "nifty50" } } } as Spec);
  expect(box().disabled).toBe(true);
});

it("Count bars held from the signal bar stores bars_held_from and clears it when unchecked", async () => {
  const { host, last, rerender } = await exec(defaultSpec());
  const box = () => [...host.querySelectorAll<HTMLLabelElement>("label.lab-check")].find((l) => l.textContent === "Count bars held from the signal bar")!.querySelector("input")!;
  expect(box().checked).toBe(false);
  await click(box());
  expect(last().execution).toEqual({ bars_held_from: "signal" });
  await rerender(last());
  expect(box().checked).toBe(true);
  await click(box());
  expect(last()).not.toHaveProperty("execution");
});
