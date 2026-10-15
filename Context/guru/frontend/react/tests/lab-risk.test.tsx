import { afterEach, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { COPY } from "../src/features/lab/labels";
import { defaultSpec, setAt, setExecution } from "../src/features/lab/spec-model";
import { DEFAULT_SHORT_SIDE, withDirection, withEntryOrder, withTakeProfits } from "../src/features/lab/execution-model";
import type { Catalogue, Spec } from "../src/features/lab/types";
import { catalogue7Fixture, catalogueFixture } from "./fixtures/lab";
import { RiskPanel } from "../src/features/lab/RiskPanel";
import { FUTURES_NOTE, MONEY_BASIS_HINT, MONEY_BLOCKED } from "../src/features/lab/labels";
import { withMode, type InstrumentContext } from "../src/features/lab/execution-model";

const usEtf: InstrumentContext = { kind: "etf", market: "us" };
const moneyCat = { ...catalogue7Fixture, execution: { ...catalogue7Fixture.execution!, sizing: ["pct_equity", "risk_pct", "units", "value"] } };

const active: { root: Root; host: HTMLElement }[] = [];
afterEach(async () => { for (const { root, host } of active) { await act(async () => root.unmount()); host.remove(); } active.length = 0; });
async function setup(spec: Spec, instrument: InstrumentContext = usEtf, catalogue: Catalogue = moneyCat) {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host); active.push({ root, host });
  const onChange = vi.fn();
  const rerender = async (s: Spec) => { await act(async () => root.render(<RiskPanel spec={s} catalogue={catalogue} instrument={instrument} onChange={onChange} />)); };
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
const risk = setup;

it("is absent until the engine lists execution options; folded unless something is set", async () => {
  expect((await risk(defaultSpec(), usEtf, catalogueFixture)).host.innerHTML).toBe("");
  const { host } = await risk(defaultSpec(), usEtf);
  expect(host.querySelector<HTMLDetailsElement>("details[data-lab-risk]")!.open).toBe(false);
  expect(host.querySelector("summary")!.textContent).toContain("Position size, re-entry and risk");
  const set = await risk(setExecution(defaultSpec(), { cooldown: { bars: 3 } }), usEtf);
  expect(set.host.querySelector<HTMLDetailsElement>("details[data-lab-risk]")!.open).toBe(true);
  expect(set.host.querySelector("summary")!.textContent).toContain("cooldown 3 bars");
});

it("money mode is blocked with the reason on an index and for India shorts", async () => {
  const { host } = await risk(defaultSpec(), { kind: "index", market: "in" });
  const money = [...host.querySelectorAll<HTMLButtonElement>('[aria-label="Results in"] button')].find((b) => b.textContent === "Money")!;
  expect(money.disabled).toBe(true);
  expect(host.querySelector("[data-lab-money-blocked]")!.textContent).toBe(MONEY_BLOCKED.index);
  const short = await risk(withDirection(defaultSpec(), "short"), { kind: "individual_stock", market: "in" });
  expect(short.host.querySelector("[data-lab-money-blocked]")!.textContent).toBe(MONEY_BLOCKED.indiaShort);
});

it("money mode on a US ETF: capital, sizing in shares, costs, taxes, borrow for shorts", async () => {
  const { host, last, rerender } = await risk(setExecution(defaultSpec(), { cost_bps: 10 }), usEtf);
  await clickIn(host, '[aria-label="Results in"]', "Money");
  expect(last().execution).toEqual({ capital: 1_000_000 });
  await rerender(last());
  expect(host.textContent).toContain(MONEY_BASIS_HINT);
  await choose(host.querySelector('select[aria-label="Size"]')!, "units");
  expect(last().execution?.sizing).toEqual({ units: 100 });
  await type(host.querySelector('input[aria-label="Commission per order"]')!, "20");
  expect(last().execution?.costs).toEqual({ commission: { per_order: 20 } });
  await click(host.querySelector('button[aria-label="Add tax"]')!);
  expect(last().execution?.costs?.taxes).toEqual([{ name: "Tax", pct: 0.1, side: "both" }]);
  expect(host.querySelector('input[aria-label="Borrow % per year"]')).toBeNull();
  const short = await risk(withMode(withDirection(defaultSpec(), "short"), "money"), usEtf);
  expect(short.host.querySelector('input[aria-label="Borrow % per year"]')).not.toBeNull();
});

it("unit mode offers full, % of equity and risk %; shares and value need money mode", async () => {
  const { host, last } = await risk(defaultSpec(), usEtf);
  const size = host.querySelector<HTMLSelectElement>('select[aria-label="Size"]')!;
  expect([...size.options].filter((o) => o.disabled).map((o) => o.value)).toEqual(["units", "value"]);
  await choose(size, "risk_pct");
  expect(last().execution).toEqual({ sizing: { risk_pct: 1 } });
});

it("re-entry and risk fields: empty means off", async () => {
  const { host, last, rerender } = await risk(defaultSpec(), usEtf);
  await type(host.querySelector('input[aria-label="Cooldown (bars)"]')!, "3");
  expect(last().execution).toEqual({ cooldown: { bars: 3 } });
  await rerender(last());
  await choose(host.querySelector('select[aria-label="Cooldown after"]')!, "loss");
  expect(last().execution?.cooldown).toEqual({ bars: 3, after: "loss" });
  await type(host.querySelector('input[aria-label="Max entries"]')!, "2");
  expect(last().execution?.max_trades).toEqual({ count: 2, per: "M" });
  await type(host.querySelector('input[aria-label="Losses in a row"]')!, "4");
  expect(last().execution?.risk).toEqual({ max_consecutive_losses: 4, pause_bars: 20 });
  await type(host.querySelector('input[aria-label="Stop new entries at drawdown (%)"]')!, "25");
  expect(last().execution?.risk).toMatchObject({ max_drawdown_pct: 25 });
  await type(host.querySelector('input[aria-label="Cooldown (bars)"]')!, "");
  expect(last().execution?.cooldown).toBeUndefined();
});

it("shows the futures note", async () => {
  expect((await risk(defaultSpec(), usEtf)).host.textContent).toContain(FUTURES_NOTE);
});


it("money is disabled when the engine has only unit sizing capabilities", async () => {
  const { host } = await risk(defaultSpec(), usEtf, catalogue7Fixture);
  const button = [...host.querySelectorAll<HTMLButtonElement>('[aria-label="Results in"] button')].find((b) => b.textContent === "Money")!;
  expect(button.disabled).toBe(true);
  expect(host.querySelector("[data-lab-money-blocked]")?.textContent).toBe("Money mode is not supported by this engine yet.");
});

it("nested edits preserve unknown metadata and clear empty cost objects", async () => {
  const source = setExecution(defaultSpec(), { capital: 100000, costs: { commission: { per_order: 10 }, future_cost: true } as never, future_execution: { keep: true } });
  const before = JSON.stringify(source);
  const { host, last, rerender } = await risk(source);
  await type(host.querySelector('input[aria-label="Commission per order"]')!, "");
  expect(last().execution?.costs).toEqual({ future_cost: true });
  expect(last().execution?.future_execution).toEqual({ keep: true });
  expect(JSON.stringify(source)).toBe(before);
  await rerender(setExecution(source, { costs: { commission: { per_order: 11 } } }));
  await type(host.querySelector('input[aria-label="Commission per order"]')!, "");
  expect(last().execution).not.toHaveProperty("costs");
});

it("money risk notes disclose unit-return timing and borrow zero is kept", async () => {
  const source = setExecution(withMode(withDirection(defaultSpec(), "short"), "money"), { cooldown: { bars: 2 } });
  const { host, last } = await risk(source);
  expect(host.textContent).toContain("Risk pauses and cooldowns use the simulator's unit returns");
  await type(host.querySelector('input[aria-label="Borrow % per year"]')!, "0");
  expect(last().execution?.costs?.borrow_pct_year).toBe(0);
});


it("unknown tax row shapes stay visible and other money edits preserve them", async () => {
  const source = setAt(withMode(defaultSpec(), "money"), "execution.costs.taxes", [null, { future_tax: true }]);
  const { host, last } = await risk(source);
  expect(host.querySelector(".lab-kept")?.textContent).toContain("future_tax");
  await type(host.querySelector('input[aria-label="Capital"]')!, "200000");
  expect(last().execution?.costs?.taxes).toEqual([null, { future_tax: true }]);
});
