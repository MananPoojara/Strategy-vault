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
  await clickIn(host, '[aria-label="Direction"]', "Both");
  expect(last().short_side).toEqual(DEFAULT_SHORT_SIDE);
  await rerender(setAt(last(), "short_side.entry.all[0].rhs.p.length", 20));
  expect(host.querySelector('[aria-label="When the other side signals"]')).not.toBeNull();
  expect(host.textContent).toContain(COPY.oppositeHelp.reverse);
  expect(host.textContent).toContain(COPY.bothFlat);
  await clickIn(host, '[aria-label="When the other side signals"]', "Exit only");
  expect(last().execution).toEqual({ opposite: "exit" });
  await clickIn(host, '[aria-label="Direction"]', "Long");
  expect(last()).not.toHaveProperty("short_side");
  await rerender(last());
  await clickIn(host, '[aria-label="Direction"]', "Both");
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

