import { act } from "react";
import { expect, it, vi } from "vitest";
import { nBarStop, ReentryDelay, toTradePrices, tradePriceProblems, TradePrices, withExitAtStop, withNBarStop } from "../src/features/lab/AmiSettings";
import { amiProblems, executionProblems, withDirection, withEntryOrder } from "../src/features/lab/execution-model";
import { defaultSpec, setExecution } from "../src/features/lab/spec-model";
import type { Spec } from "../src/features/lab/types";
import { catalogueFixture, mount } from "./fixtures/lab";

it("converts today's fills to the same AmiBroker prices and delays, for the sides the strategy trades", () => {
  expect(toTradePrices(defaultSpec()).execution).toEqual({ prices: { buy: "open", sell: "open" }, delays: { buy: 1, sell: 1 } });
  const close = toTradePrices({ ...defaultSpec(), execution: { fill: "close", exit_fill: "next_open", delay_bars: 2 } });
  expect(close.execution).toEqual({ prices: { buy: "close", sell: "open" }, delays: { buy: 0, sell: 2 } });
  expect(toTradePrices({ ...defaultSpec(), direction: "short" }).execution).toEqual({ prices: { short: "open", cover: "open" }, delays: { short: 1, cover: 1 } });
  const both = toTradePrices(withDirection(defaultSpec(), "both"));
  expect(Object.keys(both.execution!.prices!)).toEqual(["buy", "sell", "short", "cover"]);
  // a limit entry fills by its order: no buy key (the engine refuses one)
  expect(toTradePrices(withEntryOrder(defaultSpec(), "", "limit")).execution).toEqual({ prices: { sell: "open" }, delays: { sell: 1 } });
});

const ex = (execution: Spec["execution"], direction?: Spec["direction"]): Spec => ({ ...defaultSpec(), ...(direction ? { direction } : {}), execution });
it("flags what the engine refuses: look-ahead on the signal bar, delays out of range, keys of sides not traded", () => {
  expect(tradePriceProblems(ex({ prices: { buy: "close" }, delays: { buy: 0 } }))).toEqual([]);
  expect(tradePriceProblems(ex({ prices: { buy: "average" }, delays: { buy: 0 } }))[0]).toContain("Buy price average with delay 0");
  expect(tradePriceProblems(ex({ delays: { sell: 0 } }))[0]).toContain("Sell price open with delay 0"); // a missing price is open
  expect(tradePriceProblems(ex(undefined))).toEqual([]);
  expect(tradePriceProblems(ex({ delays: { buy: 1.5 } }))[0]).toBe("Buy delay must be a whole number from 0 to 20.");
  expect(tradePriceProblems(ex({ delays: { sell: 25 } }))[0]).toBe("Sell delay must be a whole number from 0 to 20.");
  expect(tradePriceProblems(ex({ prices: { short: "low" } }))[0]).toContain("Short price and delay apply to short trades");
  expect(tradePriceProblems(ex({ prices: { buy: "high" } }, "short"))[0]).toContain("Buy price and delay apply to long trades");
  const limit = withEntryOrder(defaultSpec(), "", "limit");
  expect(tradePriceProblems({ ...limit, execution: { prices: { buy: "close" }, delays: { buy: 0 } } })[0]).toContain("Buy price and delay apply to market entries");
});

it("the Run gate (executionProblems) carries the trade price, re-entry and limit checks", () => {
  const problems = (s: Spec) => executionProblems(s, catalogueFixture, { kind: "etf", market: "us" });
  expect(problems(ex({ prices: { buy: "high" }, delays: { buy: 0 } }))).toContain("Buy price high with delay 0 would use prices before the signal; use Close or a delay of 1 or more.");
  expect(amiProblems(ex({ reentry: { stop_loss: -2 } }))).toEqual(["Re-entry delay (stop-loss) must be a whole number of bars from 0 to 1000."]);
  expect(amiProblems(ex({ reentry: { bars: 2.5 } }))).toHaveLength(1);
  expect(amiProblems(ex({ capital: 1e6, limits: { round_lot: 2.5 } }))).toEqual(["Round lot size must be a whole number from 1 to 1,000,000,000."]);
  expect(amiProblems(ex({ capital: 1e6, limits: { max_volume_pct: 120 } }))).toHaveLength(1);
  expect(amiProblems(ex({ capital: 1e6, limits: { min_value: 500, max_value: 100 } }))).toEqual(["Min. pos. value must not exceed Max. pos. value."]);
  expect(amiProblems(ex({ capital: 1e6, limits: { min_value: 500, max_value: 0 } }))).toEqual([]); // 0 = no maximum, as in the engine
});

it("the Run gate checks account margin, tick size and the report rates as rules.rs does, and refuses report rates in a portfolio", () => {
  expect(amiProblems(ex({ capital: 1e6, limits: { margin_pct: 50 }, tick_size: 0.05, report: { rf_sharpe: 0, rf_upi: 3 } }))).toEqual([]);
  expect(amiProblems(ex({ capital: 1e6, limits: { margin_pct: 0 } }))).toEqual(["Account margin must be from 1% to 100% (100 = no margin)."]);
  expect(amiProblems(ex({ capital: 1e6, limits: { margin_pct: 101 } }))).toHaveLength(1);
  expect(amiProblems(ex({ tick_size: -1 }))).toEqual(["Tick size must be from 0 (no rounding) to 1,000,000."]);
  expect(amiProblems(ex({ report: { rf_sharpe: 101, rf_upi: -1 } }))).toEqual(["Risk-free rate for Sharpe ratio must be from 0% to 100%.", "Risk-free rate for UPI must be from 0% to 100%."]);
  expect(amiProblems({ ...ex({ report: { rf_sharpe: 3 } }), portfolio: { universe: { tickers: [] } } } as Spec)).toEqual(["The risk-free rates (Settings › Report) apply to single-instrument runs only; remove them on the General tab."]);
});

it("switching to a limit entry or another direction drops the keys the engine would refuse", () => {
  const prices = toTradePrices(withDirection(defaultSpec(), "both"));
  expect(withEntryOrder(prices, "", "limit").execution!.prices).toEqual({ sell: "open", short: "open", cover: "open" });
  expect(withEntryOrder(prices, "short_side.", "stop").execution!.delays).toEqual({ buy: 1, sell: 1, cover: 1 });
  expect(withDirection(prices, "long").execution).toEqual({ prices: { buy: "open", sell: "open" }, delays: { buy: 1, sell: 1 } });
  const shortOnly = withDirection(prices, "short");
  expect(shortOnly.execution).toEqual({ prices: { short: "open", cover: "open" }, delays: { short: 1, cover: 1 } });
  expect(tradePriceProblems(shortOnly)).toEqual([]);
});

it("shows long and short groups by direction and edits one side", async () => {
  const onChange = vi.fn();
  const spec: Spec = toTradePrices({ ...defaultSpec(), direction: "short" });
  const host = await mount(<TradePrices spec={spec} onChange={onChange} />);
  expect([...host.querySelectorAll("legend")].map((l) => l.textContent)).toEqual(["Short trades"]);
  const select = host.querySelector<HTMLSelectElement>('select[aria-label="Cover price"]')!;
  await act(async () => { select.value = "average"; select.dispatchEvent(new Event("change", { bubbles: true })); });
  expect(onChange.mock.calls.at(-1)![0].execution.prices.cover).toBe("average");
});

it("re-entry delays are stored per stop and removed when cleared", async () => {
  const onChange = vi.fn();
  const host = await mount(<ReentryDelay spec={defaultSpec()} stop="stop_loss" onChange={onChange} />);
  const input = host.querySelector<HTMLInputElement>('input[aria-label="Re-entry delay (bars)"]')!;
  const type = (v: string) => act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, v); input.dispatchEvent(new Event("input", { bubbles: true })); });
  await type("3");
  expect(onChange.mock.calls.at(-1)![0].execution.reentry).toEqual({ stop_loss: 3 });
  await type("0");
  expect(onChange.mock.calls.at(-1)![0].execution?.reentry).toBeUndefined();
});

it("a preset shows its fills as a greyed price and delay grid", async () => {
  const host = await mount(<TradePrices spec={setExecution(defaultSpec(), { fill: "close" })} preset onChange={vi.fn()} />);
  expect([...host.querySelectorAll("fieldset")].every((f) => f.disabled)).toBe(true);
  expect(host.querySelector<HTMLSelectElement>('select[aria-label="Buy price"]')!.value).toBe("close");
  expect(host.querySelector<HTMLInputElement>('input[aria-label="Buy delay"]')!.value).toBe("0");
  expect(host.textContent).toContain("Set by the scenario");
});

it("Trades hides the entry row of a side that enters by a limit or stop order", async () => {
  const spec = toTradePrices(withDirection(defaultSpec(), "both"));
  const host = await mount(<TradePrices spec={withEntryOrder(spec, "", "limit")} onChange={vi.fn()} />);
  expect([...host.querySelectorAll("select")].map((x) => x.getAttribute("aria-label"))).toEqual(["Sell price", "Short price", "Cover price"]);
});

it("the N-bar stop reads and writes a top-level bars_held exit rule; Off removes it", () => {
  const plain: Spec = { ...defaultSpec(), exit: { any: [] } };
  const rule = (value: number) => ({ lhs: { s: "bars_held" }, op: ">=", rhs: { value } });
  const s = withNBarStop(plain, "", 5);
  expect(s.exit.any).toEqual([rule(5)]);
  expect(nBarStop(defaultSpec(), "")).toBe(5); // today's default exit is such a rule
  expect(withNBarStop(s, "", 8).exit.any).toEqual([rule(8)]);
  expect(withNBarStop(s, "", undefined).exit.any).toEqual([]);
});

it("AmiBroker stop settings: withExitAtStop stores only non-default modes; a portfolio flags them for the Run gate", () => {
  let s = withExitAtStop(defaultSpec(), "stop_loss", 2);
  expect(s.execution).toEqual({ exit_at_stop: { stop_loss: 2 } });
  s = withExitAtStop(withExitAtStop(s, "bars", 0), "stop_loss", 1);
  expect(s.execution).toEqual({ exit_at_stop: { bars: 0 } });
  expect(withExitAtStop(s, "bars", 1)).not.toHaveProperty("execution");
  expect(amiProblems(ex({ exit_at_stop: { target: 0 }, allow_same_bar_exit: false }))).toEqual([]);
  const port = { ...ex({ capital: 1e6, nbar_has_priority: true }), portfolio: { universe: { basket: "nifty50" } } } as Spec;
  expect(amiProblems(port)).toEqual(["Exit at stop?, Activate stops immediately, N-bar Has priority and Allow same bar exit / entry signal apply to one instrument only; remove them on the Trades tab."]);
});
