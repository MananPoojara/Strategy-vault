import { afterEach, expect, it, vi } from "vitest";
import { act, useState } from "react";
import { RuleBuilder } from "../src/features/lab/RuleBuilder";
import { defaultSpec, addCond } from "../src/features/lab/spec-model";
import type { Spec } from "../src/features/lab/types";
import { catalogueFixture as cat, catalogue7Fixture, mount, templateFixture, unmount } from "./fixtures/lab";

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
  await act(async () => host.querySelector<HTMLButtonElement>("[data-lab-timing]")!.click());
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
  expect(host.textContent).toContain("Higher-timeframe values are used only after that period closes (unless a timing option says otherwise).");
  const lhs = host.querySelector<HTMLElement>('[aria-label="Entry 1 left"]')!, rhs = host.querySelector<HTMLElement>('[aria-label="Entry 1 right"]')!;
  await act(async () => lhs.querySelector<HTMLButtonElement>("[data-lab-timing]")!.click());
  await choose(lhs.querySelector<HTMLSelectElement>('select[aria-label="Timeframe"]')!, "D");
  expect(last().entry.all[0].lhs).toEqual({ s: "close", offset: 2 });
  expect(last().entry.all[0].note).toBe("x");
  await typeIn(lhs.querySelector<HTMLInputElement>('input[aria-label="periods ago"]')!, "0");
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
  await act(async () => host.querySelector<HTMLButtonElement>('[aria-label="Remove Entry 1"]')!.click());
  expect(last().entry.all).toEqual([]);
  await unmount();
  const limited = await setup({ ...defaultSpec(), entry: { ...defaultSpec().entry, limit: { pct: 1 } } });
  expect(limited.host.textContent).toContain("fill at the limit price");
  expect(limited.host.textContent).not.toContain("Entry fill");
});

it("clearing the fees box keeps cost_bps and the same focused input", async () => {
  const onChange = vi.fn();
  const Host = () => {
    const [spec, setSpec] = useState<Spec>({ ...defaultSpec(), execution: { cost_bps: 10 } });
    return <RuleBuilder spec={spec} catalogue={cat} onChange={(s) => { onChange(s); setSpec(s); }}
      settings={settings} onSettings={() => {}} problems={[]} running={false} onRun={() => {}} />;
  };
  const host = await mount(<Host />);
  const fees = host.querySelector<HTMLInputElement>('input[aria-label="Fees (bps / side)"]')!;
  fees.focus();
  await typeIn(fees, "");
  expect(onChange.mock.calls.every(([s]) => (s as Spec).execution?.cost_bps !== undefined)).toBe(true);
  expect(host.querySelector('input[aria-label="Fees (bps / side)"]')).toBe(fees);
  expect(document.activeElement).toBe(fees);
  await typeIn(fees, "7");
  expect((onChange.mock.calls.at(-1)![0] as Spec).execution?.cost_bps).toBe(7);
});

it("each remove button and order input names its own row; repeated problems all render", async () => {
  const spec = addCond({ ...defaultSpec(), exit: { any: defaultSpec().exit.any, stop_loss: { pct: 5 }, target: { atr: 2, atr_length: 14 } } }, "entry", defaultSpec().entry.all[0]);
  const { host } = await setup(spec, { problems: ["same", "same"] });
  expect(host.querySelector('[aria-label="Remove Entry 2"]')).not.toBeNull();
  const names = [...host.querySelectorAll('button[aria-label^="Remove"]')].map((b) => b.getAttribute("aria-label"));
  expect(new Set(names).size).toBe(names.length);
  expect(host.querySelector('[role="group"][aria-label="Stop-loss"] input[aria-label="%"]')).not.toBeNull();
  expect(host.querySelector('[role="group"][aria-label="Target"] input[aria-label="ATR length"]')).not.toBeNull();
  expect(host.querySelectorAll(".rnote li")).toHaveLength(2);
  expect(host.textContent).toContain("Changing the entry fill also moves exits");
});

it("bars ago ignores fractions; a listed indicator with no computed ones keeps a disabled Indicator kind", async () => {
  const spec = { ...defaultSpec(), entry: { all: [{ lhs: { s: "close" }, op: ">", rhs: { s: "or_range" } }] } } as Spec;
  const { host, onChange } = await setup(spec, { catalogue: { ...cat, indicators: cat.indicators.map((x) => ({ ...x, computed: false })) } });
  const rhsKind = host.querySelector<HTMLElement>('[aria-label="Entry 1 right"] select[aria-label="Operand type"]') as HTMLSelectElement;
  expect(rhsKind.value).toBe("indicator");
  expect(rhsKind.querySelector<HTMLOptionElement>('option[value="indicator"]')!.disabled).toBe(true);
  await act(async () => host.querySelector<HTMLButtonElement>('[aria-label="Entry 1 left"] [data-lab-timing]')!.click());
  await typeIn(host.querySelector<HTMLInputElement>('[aria-label="Entry 1 left"] input[aria-label="bars ago"]')!, "1.5");
  expect(onChange).not.toHaveBeenCalled();
});

it("noncomparison nodes stay read-only and survive editing another rule", async () => {
  const node = { all: [{ lhs: { s: "rsi" }, op: "<", rhs: { value: 30 } }], future_tag: "keep" };
  const spec = { ...defaultSpec(), entry: { all: [node] }, future_rule: { enabled: true } } as Spec;
  const before = JSON.stringify(spec);
  const { host, last } = await setup(spec);
  expect(host.querySelector(".lab-cond .lab-kept")?.textContent).toContain(JSON.stringify(node));
  const exit = host.querySelectorAll<HTMLElement>(".lab-side")[1];
  await choose(exit.querySelector<HTMLSelectElement>('select[aria-label="Operator"]')!, "<=");
  expect(last().entry.all[0]).toEqual(node);
  expect(last().future_rule).toEqual({ enabled: true });
  expect(JSON.stringify(spec)).toBe(before);
  await act(async () => host.querySelector<HTMLButtonElement>('[aria-label="Remove Entry 1"]')!.click());
  expect(last().entry.all).toEqual([]);
});


it("Phase7 Both exposes independent long and short rule lists and a compact caps line", async () => {
  const spec = { ...defaultSpec(), direction: "both", short_side: { entry: defaultSpec().entry, exit: defaultSpec().exit } } as Spec;
  const { host } = await setup(spec, { catalogue: catalogue7Fixture });
  expect([...host.querySelectorAll("[data-lab-side]")].map((e) => e.getAttribute("data-lab-side"))).toEqual(["long", "short"]);
  expect(host.querySelector("[data-lab-list='short_side.entry.all']")).not.toBeNull();
  expect(host.querySelector("[data-lab-caps]")!.textContent).toContain("comparisons 4/40");
  expect(host.querySelector("details[data-lab-risk]")).not.toBeNull();
});
it("Phase7 money uses declared costs instead of sending legacy fee fields", async () => {
  const spec = { ...defaultSpec(), execution: { capital: 100000 } } as Spec;
  const { host } = await setup(spec, { catalogue: catalogue7Fixture });
  expect(host.querySelector('input[aria-label="Fees (bps / side)"]')).toBeNull();
  expect(host.querySelector('input[aria-label="Slippage (bps / side)"]')).toBeNull();
});
