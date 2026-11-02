import { expect, it } from "vitest";
import { exitOrder } from "../src/features/lab/ExitChoice";
import { defaultSpec } from "../src/features/lab/spec-model";
import type { RuleNode, Spec } from "../src/features/lab/types";

const rsi = { lhs: { s: "rsi", p: { length: 14 } }, op: "crosses_above", rhs: { value: 60 } } as unknown as RuleNode;
const nBars = { lhs: { s: "bars_held" }, op: ">=", rhs: { value: 10 } } as unknown as RuleNode;
const spec = (exit: object, execution: object = {}): Spec => ({ ...defaultSpec(), exit: { any: [], ...exit }, execution } as Spec);

it("shows nothing for fewer than two exits", () => {
  expect(exitOrder(spec({ any: [rsi] }), "")).toEqual([]);
  expect(exitOrder(spec({ stop_loss: { pct: 5 } }), "")).toEqual([]);
});

it("default settings: stops inside the bar before profit booking, then rule exits, Dual ATR and two-line Supertrend at the close", () => {
  expect(exitOrder(spec({ any: [rsi, nBars], stop_loss: { pct: 5 }, trailing: { atr: 3, atr_length: 14 }, target: { pct: 10 },
    dual_atr: { initial: { period: 5, mult: 2 }, favour: { period: 5, mult: 4 } }, dual_line: { initial: { s: "supertrend" }, favour: { s: "supertrend" } } }), "")).toEqual([
    "Inside the bar: the tightest of HSL and Trailing ATR stop before Profit booking; a level the open gaps through fills first, at the open",
    "At the close: Indicator and N candle (in list order), then ATR stop (Dual ATR), then Supertrend (two lines)",
  ]);
});

it("follows the settings: trade-price stops by the sell price, OHLC path and N-bar priority", () => {
  const exit = { any: [nBars], stop_loss: { pct: 5 }, target: { pct: 10 } };
  // HSL checked on the close (ExitAtStop 0) with the default next-open sell: tested on the open, before anything else
  expect(exitOrder(spec(exit, { exit_at_stop: { stop_loss: 0 } }), "")).toEqual([
    "HSL at the open (checked on the trade price)", "Inside the bar: Profit booking", "At the close: N candle"]);
  // selling at the close: tested after the stops inside the bar
  expect(exitOrder(spec(exit, { exit_at_stop: { stop_loss: 0 }, prices: { sell: "close" }, delays: { sell: 0 } }), "")[1]).toBe("HSL at the sell price (checked on the trade price)");
  expect(exitOrder(spec(exit, { intrabar: "ohlc_path" }), "")[0]).toBe("Inside the bar: HSL and Profit booking, whichever price the bar reaches first (open, then the nearer of high and low)");
  expect(exitOrder(spec(exit, { nbar_has_priority: true }), "")[0]).toBe("N candle has priority: on the bar it is due, the stops and targets are not checked");
  // a short side reads its own cover price
  const both = { ...spec(exit, { exit_at_stop: { stop_loss: 0 }, prices: { sell: "close", cover: "open" }, delays: { sell: 0 } }), direction: "both", short_side: { entry: defaultSpec().entry, exit } } as Spec;
  expect(exitOrder(both, "short_side.")[0]).toBe("HSL at the open (checked on the trade price)");
  // switched-off rule rows do not count
  expect(exitOrder(spec({ any: [{ ...rsi, off: true } as RuleNode], stop_loss: { pct: 5 } }), "")).toEqual([]);
});
