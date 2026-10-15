import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { RuleBuilder } from "../src/features/lab/RuleBuilder";
import { defaultSpec, addCond } from "../src/features/lab/spec-model";
import type { Spec } from "../src/features/lab/types";
import { catalogueFixture as cat, mount, templateFixture, unmount } from "./fixtures/lab";

afterEach(unmount);
const settings = { from: "", to: "", fees: "", slippage: "" };
const setup = async (spec: Spec, extra: Partial<Parameters<typeof RuleBuilder>[0]> = {}) => {
  const onChange = vi.fn(), onRun = vi.fn(), onSettings = vi.fn();
  const host = await mount(<RuleBuilder spec={spec} catalogue={{ ...cat, timeframes: ["D", "W", "M", "Q"] }} onChange={onChange}
    settings={settings} onSettings={onSettings} problems={[]} running={false} onRun={onRun} {...extra} />);
  return { host, onChange, onRun, onSettings, last: () => onChange.mock.calls.at(-1)![0] as Spec };
};
const choose = async (el: HTMLSelectElement, value: string) => act(async () => { el.value = value; el.dispatchEvent(new Event("change", { bubbles: true })); });

it("shows both sides, catalogue operators incl. 'is at most' and catalogue timeframes", async () => {
  const { host } = await setup(defaultSpec());
  expect(host.textContent).toContain("Entry · all of these");
  expect(host.textContent).toContain("Exit · any of these");
  expect([...host.querySelectorAll("option")].map((o) => o.textContent)).toEqual(expect.arrayContaining(["is at most", "crosses above"]));
  expect([...host.querySelectorAll("option")].some((o) => o.value === "Q")).toBe(true);
});

it("bars held only on the exit side; listed indicators are never offered", async () => {
  const { host } = await setup(defaultSpec());
  const [entry, exit] = host.querySelectorAll<HTMLElement>(".lab-side");
  expect(entry.textContent).not.toContain("Bars held");
  expect(exit.textContent).toContain("Bars held");
  expect(host.querySelector('option[value="or_range"]')).toBeNull();
});

it("editing keeps origin and unknown keys", async () => {
  const spec = { ...(templateFixture.status === "available" ? templateFixture.template.spec : defaultSpec()), future_rule: 1 } as Spec;
  const { host, last } = await setup(spec);
  await choose(host.querySelector<HTMLSelectElement>(".lab-cond select[aria-label='Operator']")!, "<=");
  expect(last().entry.all[0].op).toBe("<=");
  expect(last().origin).toEqual(spec.origin);
  expect(last().future_rule).toBe(1);
});

it("unknown operand shapes are kept and shown read-only", async () => {
  const spec = { ...defaultSpec(), entry: { all: [{ lhs: { s: "close" }, op: ">", rhs: { fancy: { k: 1 } } as never }] } } as Spec;
  const { host } = await setup(spec);
  expect(host.querySelector(".lab-kept")?.textContent).toContain('{"fancy":{"k":1}}');
});

it("add condition stops at the catalogue limit; problems block the run", async () => {
  let spec = defaultSpec();
  for (let i = 0; i < 10; i++) spec = addCond(spec, "entry", spec.entry.all[0]);
  const { host } = await setup(spec, { problems: ["Entry 1: a condition cannot compare two numbers; one side must be a series."] });
  expect([...host.querySelectorAll<HTMLButtonElement>("button")].filter((b) => b.textContent === "+ Add condition").every((b) => b.disabled)).toBe(true);
  expect(host.querySelector<HTMLButtonElement>('[data-lab-action="run"]')!.disabled).toBe(true);
  expect(host.textContent).toContain("one side must be a series");
});

it("orders: a limit hides the entry fill and removes it; ATR stops; short direction", async () => {
  const { host, last } = await setup({ ...defaultSpec(), execution: { fill: "close" } });
  const row = (label: string) => [...host.querySelectorAll<HTMLElement>(".lab-order")].find((r) => r.textContent!.startsWith(label))!;
  await act(async () => [...row("Limit entry").querySelectorAll<HTMLButtonElement>(".join-item")].find((b) => b.textContent === "%")!.click());
  expect(last().entry.limit).toBeDefined();
  // fill also set the exit fill, so the model keeps it as exit_fill (Task 4 review fix).
  expect(last().execution).toEqual({ exit_fill: "close" });
  await act(async () => [...row("Stop-loss").querySelectorAll<HTMLButtonElement>(".join-item")].find((b) => b.textContent === "ATR")!.click());
  expect(last().exit.stop_loss).toEqual({ atr: 2, atr_length: 14 });
  await act(async () => [...host.querySelectorAll<HTMLButtonElement>(".join-item")].find((b) => b.textContent === "Short")!.click());
  expect(last().direction).toBe("short");
});

it("the fees box edits cost_bps when the spec carries it", async () => {
  const { host, last, onSettings } = await setup({ ...defaultSpec(), execution: { cost_bps: 10 } });
  const fees = host.querySelector<HTMLInputElement>('input[aria-label="Fees (bps / side)"]')!;
  expect(fees.value).toBe("10");
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(fees, "12");
    fees.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(last().execution?.cost_bps).toBe(12);
  expect(onSettings).not.toHaveBeenCalledWith(expect.objectContaining({ fees: "12" }));
});

const typeIn = async (el: HTMLInputElement, value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(el, value);
  el.dispatchEvent(new Event("input", { bubbles: true }));
});

it("operand edits: D deletes tf, 0 deletes offset, a new indicator drops params, price keeps tf", async () => {
  const spec = { ...defaultSpec(), entry: { all: [{ lhs: { s: "close", tf: "W", offset: 2 }, op: ">", rhs: { s: "macd", p: { fast: 5 }, out: "signal", tf: "W" }, note: "x" }] } } as Spec;
  const { host, last } = await setup(spec);
  expect(host.textContent).toContain("Weekly and monthly values are used only after that bar closes.");
  const lhs = host.querySelector<HTMLElement>('[aria-label="Entry 1 left"]')!, rhs = host.querySelector<HTMLElement>('[aria-label="Entry 1 right"]')!;
  await choose(lhs.querySelector<HTMLSelectElement>('select[aria-label="Timeframe"]')!, "D");
  expect(last().entry.all[0].lhs).toEqual({ s: "close", offset: 2 });
  expect(last().entry.all[0].note).toBe("x");
  await typeIn(lhs.querySelector<HTMLInputElement>('input[aria-label="bars ago"]')!, "0");
  expect(last().entry.all[0].lhs).toEqual({ s: "close", tf: "W" });
  expect(rhs.querySelector<HTMLSelectElement>('select[aria-label="Output"]')!.value).toBe("signal");
  await choose(rhs.querySelector<HTMLSelectElement>('select[aria-label="Indicator"]')!, "rsi");
  expect(last().entry.all[0].rhs).toEqual({ s: "rsi", tf: "W" });
  await choose(rhs.querySelector<HTMLSelectElement>('select[aria-label="Operand type"]')!, "price");
  expect(last().entry.all[0].rhs).toEqual({ s: "close", tf: "W" });
});

it("orders Off removes the key; exec line follows fill; remove condition", async () => {
  const { host, last } = await setup({ ...defaultSpec(), exit: { any: defaultSpec().exit.any, target: { pct: 10 } } });
  expect(host.textContent).toContain("fill at the next open");
  const target = [...host.querySelectorAll<HTMLElement>(".lab-order")].find((r) => r.textContent!.startsWith("Target"))!;
  await act(async () => [...target.querySelectorAll<HTMLButtonElement>(".join-item")].find((b) => b.textContent === "Off")!.click());
  expect("target" in last().exit).toBe(false);
  await act(async () => host.querySelector<HTMLButtonElement>('[aria-label="Remove condition"]')!.click());
  expect(last().entry.all).toEqual([]);
  await unmount();
  const limited = await setup({ ...defaultSpec(), entry: { ...defaultSpec().entry, limit: { pct: 1 } } });
  expect(limited.host.textContent).toContain("fill at the limit price");
  expect(limited.host.textContent).not.toContain("Entry fill");
});
