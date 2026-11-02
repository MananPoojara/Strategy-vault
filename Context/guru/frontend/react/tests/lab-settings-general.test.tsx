import { afterEach, expect, it, vi } from "vitest";
vi.mock("../src/features/lab/client", async (orig) => ({ ...(await orig<typeof import("../src/features/lab/client")>()),
  // the market's instrument list the Reference symbol box offers: indices first
  tickers: vi.fn(async () => [{ ticker: "NIFTY 50" }, { ticker: "NIFTYBANK" }]),
  labInstruments: vi.fn(async () => ({ instruments: [] })) }));
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { defaultSpec } from "../src/features/lab/spec-model";
import { withDirection, withMode, type InstrumentContext } from "../src/features/lab/execution-model";
import { GeneralTab } from "../src/features/lab/settings/GeneralTab";
import { CASH_BLOCKED, CASH_ONLY, NOT_IN_LAB, ONE_INSTRUMENT } from "../src/features/lab/settings/common";
import type { Catalogue, Spec } from "../src/features/lab/types";
import { catalogue7Fixture, catalogueFixture } from "./fixtures/lab";

const usEtf: InstrumentContext = { kind: "etf", market: "us" };
const moneyCat = { ...catalogue7Fixture, execution: { ...catalogue7Fixture.execution!, sizing: ["pct_equity", "risk_pct", "units", "value"] } };

const active: { root: Root; host: HTMLElement }[] = [];
afterEach(async () => { for (const { root, host } of active) { await act(async () => root.unmount()); host.remove(); } active.length = 0; });
async function setup(spec: Spec, instrument: InstrumentContext = usEtf, catalogue: Catalogue = moneyCat) {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host); active.push({ root, host });
  const onChange = vi.fn();
  await act(async () => root.render(<GeneralTab spec={spec} catalogue={catalogue} instrument={instrument} onChange={onChange} />));
  return { host, last: () => onChange.mock.calls.at(-1)![0] as Spec };
}
const input = (host: HTMLElement, label: string) => host.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!;
const type = async (el: Element, value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(el, value);
  el.dispatchEvent(new Event("input", { bubbles: true }));
});
const reasons = (host: HTMLElement) => [...host.querySelectorAll("[data-lab-why]")].map((p) => p.textContent);

it("is absent until the engine lists execution options; AmiBroker's sections in order, pad & align first", async () => {
  expect((await setup(defaultSpec(), usEtf, catalogueFixture)).host.innerHTML).toBe("");
  const { host } = await setup(defaultSpec());
  expect([...host.querySelectorAll(".lab-sgroup > h4")].map((h) => h.textContent)).toEqual(["Data", "Backtest", "Commissions & rates"]);
  expect(host.querySelector("input[type=checkbox]")!.getAttribute("aria-label")).toBe("Pad and align all data to reference symbol");
  // options the lab cannot do are left out, and Initial equity is only under Starting money
  for (const label of ["Periodicity", "Use QuickAFL", "commission table", "Dynamic interest symbol", "Futures mode", "Initial equity"]) expect(host.querySelector(`[aria-label="${label}"]`)).toBeNull();
});

it("money-only fields stay visible but disabled with the reason in % returns, and the block's reason on crypto", async () => {
  const etf = await setup(defaultSpec());
  for (const label of ["Min. shares", "Round lot size", "percent", "$ per trade", "Account margin", "Allow position size shrinking"]) expect(input(etf.host, label).disabled).toBe(true);
  expect(reasons(etf.host)).toContain(CASH_ONLY);
  // said once per group (Backtest, Commissions & rates), not beside every greyed row
  expect(reasons(etf.host).filter((r) => r === CASH_ONLY)).toHaveLength(2);
  const crypto = await setup(defaultSpec(), { kind: "crypto", market: "gl" }); // an index now runs in money; crypto is still blocked
  expect(reasons(crypto.host)).toContain(CASH_BLOCKED);
  expect(reasons(crypto.host)).not.toContain(CASH_ONLY);
});

it("in money mode writes the AmiBroker limits, commissions and rates; zero round lot means none", async () => {
  const { host, last } = await setup(withMode(defaultSpec(), "money"));
  expect(reasons(host)).not.toContain(CASH_ONLY);
  await type(input(host, "Min. shares"), "0.1");
  expect(last().execution?.limits).toEqual({ min_shares: 0.1 });
  await type(input(host, "Round lot size"), "0");
  expect(last().execution?.limits).toBeUndefined();
  await type(input(host, "percent"), "0.05");
  expect(last().execution?.costs).toEqual({ commission: { pct: 0.05 } });
  await type(input(host, "Margin rate [%]"), "8");
  expect(last().execution?.limits).toEqual({ margin_rate_pct: 8 });
  await act(async () => input(host, "Allow position size shrinking").click());
  expect(last().execution?.limits).toEqual({ shrink: true });
});

it("Reverse entry signal forces exit: disabled long-only, unticked = ignore the opposite signal", async () => {
  const long = await setup(defaultSpec());
  expect(input(long.host, "Reverse entry signal forces exit").disabled).toBe(true);
  expect(input(long.host, "Reverse entry signal forces exit").closest(".lab-row")!.textContent).toContain("Only with Long and short positions.");
  const both = await setup(withDirection(defaultSpec(), "both"));
  expect(input(both.host, "Reverse entry signal forces exit").checked).toBe(true);
  await act(async () => input(both.host, "Reverse entry signal forces exit").click());
  expect(both.last().execution?.opposite).toBe("ignore");
});

it("a portfolio keeps the single-instrument options visible, disabled, with the reason", async () => {
  const { host } = await setup({ ...withMode(defaultSpec(), "money"), portfolio: { universe: { basket: "nifty50" } } } as Spec, { kind: "portfolio", market: "in" });
  for (const label of ["Pad and align all data to reference symbol", "Tick size", "Min. shares", "Fixed annual interest [%]"]) expect(input(host, label).disabled).toBe(true);
  expect(input(host, "$ per trade").disabled).toBe(false);
  expect(reasons(host)).toContain(ONE_INSTRUMENT);
});
