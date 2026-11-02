import { afterEach, expect, it, vi } from "vitest";
vi.mock("../src/features/lab/client", async (orig) => ({ ...(await orig<typeof import("../src/features/lab/client")>()),
  // the market's instrument list the Reference symbol box offers: indices first, then other symbols
  tickers: vi.fn(async (m: string) => (m === "us" ? [{ ticker: "SPX" }, { ticker: "DJI" }, { ticker: "SPY", native: true, asset_type: "etf", symbol: "SPY" }]
    : [{ ticker: "NIFTY 50" }, { ticker: "NIFTYBANK" }])),
  labInstruments: vi.fn(async () => ({ instruments: [] })) }));
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { COPY } from "../src/features/lab/labels";
import { defaultSpec, setAt, setExecution } from "../src/features/lab/spec-model";
import { DEFAULT_SHORT_SIDE, withDirection, withEntryOrder, withTakeProfits } from "../src/features/lab/execution-model";
import type { Catalogue, Spec } from "../src/features/lab/types";
import { catalogue7Fixture, catalogueFixture } from "./fixtures/lab";
import { MARGIN_NOTE, PAD_NOTE, RiskPanel, TICK_NOTE } from "../src/features/lab/RiskPanel";
import { BacktestParams, NEEDS_ENGINE } from "../src/features/lab/settings/BacktestParams";
import { GeneralTab } from "../src/features/lab/settings/GeneralTab";
import { FUTURES_NOTE, MONEY_BASIS_HINT, MONEY_BLOCKED } from "../src/features/lab/labels";
import { withMode, type InstrumentContext } from "../src/features/lab/execution-model";

const usEtf: InstrumentContext = { kind: "etf", market: "us" };
const moneyCat = { ...catalogue7Fixture, execution: { ...catalogue7Fixture.execution!, sizing: ["pct_equity", "risk_pct", "units", "value"] } };

const active: { root: Root; host: HTMLElement }[] = [];
afterEach(async () => { for (const { root, host } of active) { await act(async () => root.unmount()); host.remove(); } active.length = 0; });
async function setup(spec: Spec, instrument: InstrumentContext = usEtf, catalogue: Catalogue = moneyCat, tab = false) {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host); active.push({ root, host });
  const onChange = vi.fn();
  // `tab`: AmiBroker's General tab, where the commissions and position limits are
  const rerender = async (s: Spec) => { await act(async () => root.render(tab ? <GeneralTab spec={s} catalogue={catalogue} instrument={instrument} onChange={onChange} /> : <><BacktestParams spec={s} catalogue={catalogue} instrument={instrument} onChange={onChange} unitCosts={null} orders={null} /><RiskPanel spec={s} catalogue={catalogue} instrument={instrument} onChange={onChange} /></>)); };
  await rerender(spec);
  return { host, last: () => onChange.mock.calls.at(-1)![0] as Spec, rerender };
}
const click = async (el: Element) => act(async () => (el as HTMLElement).click());
const choose = async (el: Element, value: string) => act(async () => { (el as HTMLSelectElement).value = value; el.dispatchEvent(new Event("change", { bubbles: true })); });
const type = async (el: Element, value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(el, value);
  el.dispatchEvent(new Event("input", { bubbles: true }));
});
const risk = setup, general = (spec: Spec, instrument: InstrumentContext = usEtf) => setup(spec, instrument, moneyCat, true);

it("is absent until the engine lists execution options; folded unless something is set", async () => {
  expect((await risk(defaultSpec(), usEtf, catalogueFixture)).host.innerHTML).toBe("");
  const { host } = await risk(defaultSpec(), usEtf);
  expect(host.querySelector<HTMLDetailsElement>("details[data-lab-risk]")!.open).toBe(false);
  expect(host.querySelector("summary")!.textContent).toContain("Position size, re-entry and risk");
  const set = await risk(setExecution(defaultSpec(), { cooldown: { bars: 3 } }), usEtf);
  expect(set.host.querySelector<HTMLDetailsElement>("details[data-lab-risk]")!.open).toBe(true);
  expect(set.host.querySelector("summary")!.textContent).toContain("cooldown 3 bars");
});

it("an index runs in money from the initial equity like a stock; India shorts stay blocked", async () => {
  const { host } = await risk(withMode(defaultSpec(), "money"), { kind: "index", market: "in" });
  const equity = host.querySelector<HTMLInputElement>('input[aria-label="Initial equity"]')!;
  expect(equity.disabled).toBe(false);
  expect(equity.value).toBe("1000000"); // the AFL's Initial Equity
  expect(host.querySelector("[data-lab-money-blocked]")).toBeNull(); // no index note: the page says the run is simulated
  const short = await risk(withDirection(defaultSpec(), "short"), { kind: "individual_stock", market: "in" });
  expect(short.host.querySelector("[data-lab-money-blocked]")!.textContent).toBe(MONEY_BLOCKED.indiaShort);
});

it("money mode on a US ETF: capital, sizing in shares, costs, taxes, borrow for shorts", async () => {
  // the page puts an ETF in money mode (IndicatorsPage); the AFLs' Initial Equity is the default
  const { host, last } = await risk(withMode(setExecution(defaultSpec(), { cost_bps: 10 }), "money"), usEtf);
  expect(withMode(defaultSpec(), "money").execution).toEqual({ capital: 1_000_000 });
  expect(host.querySelector<HTMLInputElement>('input[aria-label="Initial equity"]')!.value).toBe("1000000");
  expect(host.textContent).not.toContain(MONEY_BASIS_HINT); // the cost disclaimer is not shown
  await choose(host.querySelector('select[aria-label="Position size per"]')!, "units");
  expect(last().execution?.sizing).toEqual({ units: 100 });
  await type(host.querySelector('input[aria-label="Slippage (bps)"]')!, "20");
  expect(last().execution?.costs).toEqual({ slippage: { bps: 20 } });
  await click(host.querySelector('button[aria-label="Add tax"]')!);
  expect(last().execution?.costs?.taxes).toEqual([{ name: "Tax", pct: 0.1, side: "both" }]);
  expect(host.querySelector('input[aria-label="Borrow % per year"]')).toBeNull();
  const short = await risk(withMode(withDirection(defaultSpec(), "short"), "money"), usEtf);
  expect(short.host.querySelector('input[aria-label="Borrow % per year"]')).not.toBeNull();
});

it("unit mode offers % of equity only; Share and Fixed equity need money mode; risk % only when the spec uses it", async () => {
  const { host } = await risk(defaultSpec(), usEtf);
  const per = host.querySelector<HTMLSelectElement>('select[aria-label="Position size per"]')!;
  expect([...per.options].map((o) => [o.value, o.disabled])).toEqual([["equity", false], ["units", true]]); // AFL Share needs money
  const mode = host.querySelector<HTMLSelectElement>('select[aria-label="Equity mode"]')!;
  expect([...mode.options].map((o) => [o.textContent, o.disabled])).toEqual([["% of equity", false], ["Fixed equity", true]]);
  expect(host.querySelector<HTMLInputElement>('input[aria-label="% of equity"]')!.value).toBe("100"); // AFL default: 100 % of equity
  const risky = await risk(setExecution(defaultSpec(), { sizing: { risk_pct: 2 } }), usEtf);
  expect([...risky.host.querySelector<HTMLSelectElement>('select[aria-label="Position size per"]')!.options].map((o) => o.value)).toEqual(["equity", "units", "risk_pct"]);
  expect(risky.host.querySelector<HTMLInputElement>('input[aria-label="Risk %"]')!.value).toBe("2");
  expect(risky.host.querySelector('select[aria-label="Equity mode"]')).toBeNull();
});

it("the AFL backtest parameter rows, in its order, with its defaults", async () => {
  const { host } = await risk(withMode(defaultSpec(), "money"), { kind: "index", market: "in" });
  const block = host.querySelector('[aria-label="Backtest parameters"]')!;
  const labels = [...block.querySelectorAll("input[aria-label], select[aria-label]")].map((e) => e.getAttribute("aria-label"));
  expect(labels.slice(0, 13)).toEqual(["Initial equity", "Position size per", "Equity mode", "% of equity", "Equity allocation", "Brokerage (%)", "Slippage (bps)",
    "Maximum position", "Dead stock backtest", "Penny stock price", "Turnover filter", "Aux1", "Aux2"]);
  const val = (l: string) => block.querySelector<HTMLInputElement | HTMLSelectElement>(`[aria-label="${l}"]`)!;
  expect(["Initial equity", "% of equity", "Equity allocation", "Maximum position", "Penny stock price"].map((l) => val(l).value)).toEqual(["1000000", "100", "100000", "1", "0"]);
  expect([...block.querySelectorAll<HTMLSelectElement>("select")].slice(0, 6).map((s) => s.selectedOptions[0].textContent))
    .toEqual(["Equity", "% of equity", "No", "No", "Off", "Off"]);
  expect(val("Equity allocation").disabled).toBe(true); // only for Fixed equity
});

it("equity mode and share sizing map to the spec as the AFL's SetPositionSize", async () => {
  const { host, last, rerender } = await risk(withMode(defaultSpec(), "money"), usEtf);
  await choose(host.querySelector('select[aria-label="Equity mode"]')!, "value");
  expect(last().execution?.sizing).toEqual({ value: 100_000 }); // spsValue, EQUITY ALLOCATION
  await rerender(last());
  expect(host.querySelector<HTMLInputElement>('input[aria-label="Equity allocation"]')!.disabled).toBe(false);
  expect(host.querySelector<HTMLInputElement>('input[aria-label="% of equity"]')!.disabled).toBe(true);
  await type(host.querySelector('input[aria-label="Equity allocation"]')!, "250000");
  expect(last().execution?.sizing).toEqual({ value: 250_000 });
  await choose(host.querySelector('select[aria-label="Equity mode"]')!, "full");
  expect(last().execution?.sizing).toBeUndefined(); // 100 % of equity = no sizing field
  await rerender(setExecution(last(), { sizing: { units: 100 } }));
  expect(host.querySelector('select[aria-label="Equity mode"]')).toBeNull(); // Share has no equity mode
  await type(host.querySelector('input[aria-label="No. of shares"]')!, "250");
  expect(last().execution?.sizing).toEqual({ units: 250 }); // spsShares
});

const gateCat = { ...moneyCat, execution: { ...moneyCat.execution, entry_gates: ["dead_stock", "penny_price", "turnover_filter", "aux1", "aux2"] } };
it("each entry gate writes its execution field and removes it when off", async () => {
  const { host, last, rerender } = await setup(withMode(defaultSpec(), "money"), { ...usEtf, hasVolume: true }, gateCat);
  for (const [label, field] of [["Dead stock backtest", "dead_stock"], ["Turnover filter", "turnover_filter"], ["Aux1", "aux1"], ["Aux2", "aux2"]] as const) {
    await choose(host.querySelector(`select[aria-label="${label}"]`)!, "on");
    expect(last().execution?.[field], label).toBe(true);
    await rerender(last());
    await choose(host.querySelector(`select[aria-label="${label}"]`)!, "off");
    expect(last().execution, label).not.toHaveProperty(field);
    await rerender(last());
  }
  await type(host.querySelector('input[aria-label="Penny stock price"]')!, "5");
  expect(last().execution?.penny_price).toBe(5);
  await rerender(last());
  await type(host.querySelector('input[aria-label="Penny stock price"]')!, "0");
  expect(last().execution).not.toHaveProperty("penny_price");
  expect(last().execution).toEqual({ capital: 1_000_000 }); // nothing left behind when every gate is off
});

it("entry gates are greyed until the engine lists them; turnover needs volume", async () => {
  const { host } = await risk(withMode(defaultSpec(), "money"), usEtf);
  for (const l of ["Dead stock backtest", "Penny stock price", "Turnover filter", "Aux1", "Aux2"]) {
    const el = host.querySelector<HTMLInputElement | HTMLSelectElement>(`[aria-label="${l}"]`)!;
    expect(el.disabled, l).toBe(true);
    expect(el.closest("[title]")!.getAttribute("title"), l).toBe(NEEDS_ENGINE);
  }
  const noVol = await setup(withMode(defaultSpec(), "money"), { ...usEtf, hasVolume: false }, gateCat);
  const turnover = noVol.host.querySelector<HTMLSelectElement>('select[aria-label="Turnover filter"]')!;
  expect(turnover.disabled).toBe(true);
  expect(turnover.closest("[title]")!.getAttribute("title")).toContain("volume");
  expect(noVol.host.querySelector<HTMLSelectElement>('select[aria-label="Aux1"]')!.disabled).toBe(false);
  // the engine refuses the gates on a portfolio: off ones are greyed, one already on stays enabled to turn it off
  const port = withMode({ ...defaultSpec(), portfolio: { universe: { basket: "nifty50" }, max_open: 5 } } as Spec, "money");
  const p = await setup(setExecution(port, { aux1: true }), { ...usEtf, hasVolume: true }, gateCat);
  expect(p.host.querySelector<HTMLSelectElement>('select[aria-label="Aux2"]')!.disabled).toBe(true);
  expect(p.host.querySelector<HTMLSelectElement>('select[aria-label="Aux1"]')!.disabled).toBe(false);
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
  expect(host.querySelector<HTMLInputElement>('input[aria-label="Initial equity"]')!.disabled).toBe(true);
  expect(host.querySelector("[data-lab-money-blocked]")?.textContent).toBe("Money mode is not supported by this engine yet.");
});

it("nested edits preserve unknown metadata and clear empty cost objects", async () => {
  const source = setExecution(defaultSpec(), { capital: 100000, costs: { slippage: { bps: 10 }, future_cost: true } as never, future_execution: { keep: true } });
  const before = JSON.stringify(source);
  const { host, last, rerender } = await risk(source);
  await type(host.querySelector('input[aria-label="Slippage (bps)"]')!, "");
  expect(last().execution?.costs).toEqual({ future_cost: true });
  expect(last().execution?.future_execution).toEqual({ keep: true });
  expect(JSON.stringify(source)).toBe(before);
  await rerender(setExecution(source, { costs: { slippage: { bps: 11 } } }));
  await type(host.querySelector('input[aria-label="Slippage (bps)"]')!, "");
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
  await type(host.querySelector('input[aria-label="Initial equity"]')!, "200000");
  expect(last().execution?.costs?.taxes).toEqual([null, { future_tax: true }]);
});

it("position limits and commissions (AmiBroker General tab) edit in money mode only, edit execution.limits and clear when emptied", async () => {
  expect((await general(defaultSpec(), usEtf)).host.querySelector<HTMLInputElement>('input[aria-label="Round lot size"]')!.disabled).toBe(true); // visible, greyed in % returns
  expect((await risk(withMode(defaultSpec(), "money"), usEtf)).host.querySelector('input[aria-label="Round lot size"]')).toBeNull(); // not repeated under position size & risk
  const { host, last, rerender } = await general(withMode(defaultSpec(), "money"), usEtf);
  expect(host.querySelector('input[aria-label^="Limit trade size"]')).toBeNull(); // on the Portfolio tab
  await type(host.querySelector('input[aria-label="$ per trade"]')!, "20");
  expect(last().execution?.costs).toEqual({ commission: { per_order: 20 } });
  await type(host.querySelector('input[aria-label="Round lot size"]')!, "10");
  expect(last().execution?.limits).toEqual({ round_lot: 10 });
  await rerender(last());
  await click(host.querySelector('input[aria-label="Allow position size shrinking"]')!);
  expect(last().execution?.limits).toEqual({ round_lot: 10, shrink: true });
  await rerender(last());
  await type(host.querySelector('input[aria-label="Fixed annual interest [%]"]')!, "4");
  expect(last().execution?.limits).toEqual({ round_lot: 10, shrink: true, interest_pct: 4 });
  const cleared = await general(setExecution(withMode(defaultSpec(), "money"), { limits: { round_lot: 10 } }), usEtf);
  await type(cleared.host.querySelector('input[aria-label="Round lot size"]')!, "");
  expect(cleared.last().execution).not.toHaveProperty("limits");
  expect(withMode(last(), "unit").execution?.limits).toBeUndefined();
});

it("General: Pad and align (a reference symbol from the market's own list, first) and Tick size, with their disclosures", async () => {
  const { host, last, rerender } = await general(defaultSpec(), usEtf);
  await act(async () => { await Promise.resolve(); });
  const ref = () => host.querySelector<HTMLSelectElement>('select[aria-label="Reference symbol"]')!;
  expect(ref().value).toBe("SPX");
  expect([...ref().querySelectorAll("optgroup")].map((g) => [g.label, [...g.querySelectorAll("option")].map((o) => o.value)])).toEqual([["Indices", ["SPX", "DJI"]], ["Other symbols", ["SPY"]]]);
  expect(host.querySelector("[data-lab-tick-note]")).toBeNull();
  await type(host.querySelector('input[aria-label="Tick size"]')!, "0.05");
  expect(last().execution?.tick_size).toBe(0.05);
  await rerender(last());
  expect(host.querySelector("[data-lab-tick-note]")!.textContent).toBe(TICK_NOTE);
  await act(async () => { ref().value = "DJI"; ref().dispatchEvent(new Event("change", { bubbles: true })); }); // picked while off: remembered
  await click(host.querySelector('input[aria-label="Pad and align all data to reference symbol"]')!);
  expect(last().execution).toEqual({ tick_size: 0.05, pad_align: "DJI" });
  await rerender(last());
  expect(host.querySelector("[data-lab-pad-note]")!.textContent).toBe(PAD_NOTE);
  await act(async () => { ref().value = "SPY"; ref().dispatchEvent(new Event("change", { bubbles: true })); });
  expect(last().execution).toEqual({ tick_size: 0.05, pad_align: "SPY" });
  await rerender(last());
  await type(host.querySelector('input[aria-label="Tick size"]')!, "0");
  expect(last().execution).toEqual({ pad_align: "SPY" }); // AmiBroker: 0 = no rounding, so the field is removed
  const india = await general(defaultSpec(), { kind: "individual_stock", market: "in" });
  await act(async () => { await Promise.resolve(); });
  expect(india.host.querySelector<HTMLSelectElement>('select[aria-label="Reference symbol"]')!.value).toBe("NIFTY 50");
  const fut = await general(setExecution(defaultSpec(), { instrument: { kind: "futures" } }), usEtf);
  expect((fut.host.querySelector('input[aria-label="Tick size"]') as HTMLInputElement).disabled).toBe(true);
  const port = await general({ ...withMode(defaultSpec(), "money"), portfolio: { universe: { basket: "nifty50" } } } as Spec, usEtf);
  expect(port.host.querySelector("[data-lab-general-data]")).toBeNull();
});

it("Account margin and Margin rate edit execution.limits and disclose the loan interest", async () => {
  const { host, last, rerender } = await general(withMode(defaultSpec(), "money"), usEtf);
  expect(host.querySelector("[data-lab-margin-note]")).toBeNull();
  await type(host.querySelector('input[aria-label="Account margin"]')!, "50");
  expect(last().execution?.limits).toEqual({ margin_pct: 50 });
  await rerender(last());
  expect(host.querySelector("[data-lab-margin-note]")!.textContent).toBe(MARGIN_NOTE);
  expect(MARGIN_NOTE).toContain("Margin rate [%] is charged");
  await type(host.querySelector('input[aria-label="Margin rate [%]"]')!, "9");
  expect(last().execution?.limits).toEqual({ margin_pct: 50, margin_rate_pct: 9 });
  expect((await general(defaultSpec(), usEtf)).host.querySelector<HTMLInputElement>('input[aria-label="Account margin"]')!.disabled).toBe(true); // money mode only
});

it("trade after a losing streak: switching it on sets the AFL defaults and close fills; the target follows the chosen mode", async () => {
  const { host, last, rerender } = await risk(defaultSpec());
  const box = () => host.querySelector<HTMLInputElement>("[data-lab-shadow] input[type=checkbox]")!;
  await click(box());
  expect(last().execution).toMatchObject({ shadow: { loss_pct: 5, min_bars: 5, until: "trade_profit", profit_pct: 3 }, fill: "close" });
  await rerender(last());
  await type(host.querySelector('input[aria-label="Paper loss (%)"]')!, "7");
  expect(last().execution!.shadow!.loss_pct).toBe(7);
  await rerender(last());
  await choose(host.querySelector('select[aria-label="Back to paper when"]')!, "trades");
  expect(last().execution!.shadow).toMatchObject({ until: "trades", trades: 1 });
  expect(last().execution!.shadow!.profit_pct).toBeUndefined();
  await rerender(last());
  expect(host.querySelector('input[aria-label="Real trades"]')).not.toBeNull();
  expect(host.querySelector("summary")!.textContent).toContain("trade after a 7% paper loss");
  await click(box());
  expect(last().execution?.shadow).toBeUndefined();
});
