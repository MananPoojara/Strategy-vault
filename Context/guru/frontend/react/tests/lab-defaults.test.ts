import { afterEach, expect, it } from "vitest";
import { changedSettings, stopsOn, DEFAULT_EXIT, defaultExitNodes, defaultExitWords, firstRule, loadDefaultExit, ownExit, storeDefaultExit, tradeBlockers, usesDefaultExit, validDefaultExit, withDefaultExit } from "../src/features/lab/defaults";
import { INDICATOR_RULES } from "../src/features/lab/indicator-rules";
import { candleCount, candleCountNode, ordinal, ruleWords, specSummary, starterSpec } from "../src/features/lab/spec-model";
import type { Indicator, RuleNode, Spec } from "../src/features/lab/types";
import { catalogue7Fixture as cat } from "./fixtures/lab";

afterEach(() => localStorage.clear());
const rsi = { lhs: { s: "rsi", p: { length: 14 } }, op: "<", rhs: { value: 30 } } as RuleNode;
const up = (n: number) => candleCountNode("up", n), down = (n: number) => candleCountNode("down", n);
const spec = (patch: Partial<Spec> = {}): Spec => ({ ...starterSpec(), entry: { all: [rsi] }, ...patch });

it("a side with no sell rule switched on gets the default exit; anything that closes trades keeps the spec as it is", () => {
  expect(withDefaultExit(spec(), DEFAULT_EXIT).exit.any).toEqual([up(1), down(4)]);
  const offOnly = spec({ exit: { any: [{ lhs: { s: "rsi" }, op: ">", rhs: { value: 70 }, off: true } as RuleNode] } });
  expect(withDefaultExit(offOnly, { up: 2, down: 5 }).exit.any.slice(1)).toEqual([up(2), down(5)]); // the switched-off row stays
  for (const kept of [
    spec({ exit: { any: [rsi] } }),
    spec({ exit: { any: [], stop_loss: { pct: 5 } } }),
    spec({ entry: { all: [rsi, { ...rsi, filter: true, filter_exit: true } as RuleNode] } }),
    spec({ direction: "both", short_side: { entry: { all: [rsi] }, exit: { any: [] } } }), // reversing, always in the market
    spec({ portfolio: { mode: "signals", universe: { basket: "nifty50" } } as Spec["portfolio"] }), // no since-entry values in a portfolio
  ]) expect(withDefaultExit(kept, DEFAULT_EXIT)).toBe(kept);
  // Dates never replace the sell rule: the default exit still applies, closing at the period end or not
  expect(usesDefaultExit(spec({ date_filter: { windows: [["2020-01-01", "2021-01-01"]] } }))).toBe(true);
  expect(usesDefaultExit(spec({ date_filter: { windows: [["2020-01-01", "2021-01-01"]], exit_at_change: false } }))).toBe(true);
});

it("a short position mirrors the default exit: a down close is its profit", () => {
  expect(withDefaultExit(spec({ direction: "short" }), DEFAULT_EXIT).exit.any).toEqual([down(1), up(4)]);
  const both = spec({ direction: "both", execution: { fill: "close", opposite: "exit" }, short_side: { entry: { all: [rsi] }, exit: { any: [] } } });
  const out = withDefaultExit(both, DEFAULT_EXIT);
  expect([out.exit.any, out.short_side!.exit.any]).toEqual([[up(1), down(4)], [down(1), up(4)]]);
  expect(defaultExitNodes(DEFAULT_EXIT, true)).toEqual([down(1), up(4)]);
});

it("candle-count rows are recognised, read as words and edited by count", () => {
  expect(candleCount(up(3))).toEqual({ kind: "up", n: 3 });
  expect(candleCount({ ...down(4), off: true } as RuleNode)).toEqual({ kind: "down", n: 4 });
  expect(candleCount({ ...up(1), rhs: { value: 1.5 } } as RuleNode)).toBeUndefined();
  expect(candleCount({ lhs: { since_entry: "count", when: { lhs: { s: "close" }, op: ">", rhs: { s: "close", offset: 2 } } }, op: ">=", rhs: { value: 1 } })).toBeUndefined();
  expect([1, 2, 3, 4, 11, 12, 13, 21, 22, 101].map(ordinal)).toEqual(["1st", "2nd", "3rd", "4th", "11th", "12th", "13th", "21st", "22nd", "101st"]);
  expect(ruleWords(up(1), cat)).toEqual({ text: "at the 1st up close after entry", when: false });
  expect(specSummary(withDefaultExit(spec(), DEFAULT_EXIT), cat)).toBe("Buy when RSI(14) is below 30. Sell at the 1st up close after entry or at the 4th down close after entry.");
});

it("the user's default exit numbers are kept per browser; a broken value falls back", () => {
  expect(loadDefaultExit()).toEqual({ up: 1, down: 4 });
  storeDefaultExit({ up: 2, down: 6 });
  expect(loadDefaultExit()).toEqual({ up: 2, down: 6 });
  localStorage.setItem("lab.defaultExit", JSON.stringify({ up: 0, down: 6 }));
  expect(loadDefaultExit()).toEqual(DEFAULT_EXIT);
});

it("each default exit part can be switched on or off: candle counts, days in the trade and % stops", () => {
  const d = { up: 1, down: 4, downOff: true, days: 10, stopPct: 5, targetPct: 10, trailPct: 5 };
  const out = withDefaultExit(spec(), d);
  expect(out.exit.any).toEqual([up(1), { lhs: { s: "bars_held" }, op: ">=", rhs: { value: 10 } }]);
  expect([out.exit.stop_loss, out.exit.target, out.exit.trailing]).toEqual([{ pct: 5 }, { pct: 10 }, { pct: 5 }]);
  expect(defaultExitWords(d)).toBe("sell at the 1st up close after entry, or after 10 days in the trade, or at a 5% stop-loss, or at a 10% profit target, or with a 5% trailing stop");
  expect(defaultExitNodes({ up: 1, down: 4, upOff: true }, true)).toEqual([up(4)]); // a short's losing streak is up closes
  expect(defaultExitWords(DEFAULT_EXIT)).toBe("sell at the 1st up close after entry, or at the 4th down close");
  // at least one part stays on; bad numbers are refused and a stored bad value falls back
  expect(validDefaultExit({ up: 1, down: 4, upOff: true, downOff: true })).toBe(false);
  expect(validDefaultExit({ up: 1, down: 4, upOff: true, downOff: true, stopPct: 3 })).toBe(true);
  expect(validDefaultExit({ up: 1, down: 4, stopPct: 0 })).toBe(false);
  expect(validDefaultExit({ up: 1, down: 4, days: 2.5 })).toBe(false);
  storeDefaultExit(d);
  expect(loadDefaultExit()).toEqual(d);
  localStorage.setItem("lab.defaultExit", JSON.stringify({ up: 1, down: 4, upOff: true, downOff: true }));
  expect(loadDefaultExit()).toEqual(DEFAULT_EXIT);
});

it("a first rule per indicator comes from the TradingView-style table, with the user's parameters; its own exit too", () => {
  const find = (id: string) => cat.indicators.find((i) => i.id === id)!;
  expect(firstRule(find("rsi"), { length: 14 })).toEqual({ lhs: { s: "rsi", p: { length: 14 } }, op: "crosses_above", rhs: { value: 30 } });
  expect(ownExit(find("rsi"), { length: 14 })).toEqual({ lhs: { s: "rsi", p: { length: 14 } }, op: "crosses_below", rhs: { value: 70 } });
  expect(firstRule(find("macd"), {})).toEqual({ lhs: { s: "macd" }, op: "crosses_above", rhs: { s: "macd", out: "signal" } });
  expect(firstRule(find("ema"), { length: 50 })).toEqual({ lhs: { s: "close" }, op: "crosses_above", rhs: { s: "ema", p: { length: 50 } } });
  expect(INDICATOR_RULES.rsi.tv).toContain("RSI Strategy");
  // a short-sale or an indicator missing from the table falls back to the generic reading
  expect(firstRule(find("rsi"), { length: 14 }, true)).toEqual({ lhs: { s: "rsi", p: { length: 14 } }, op: ">", rhs: { value: 70 } });
  expect(ownExit(find("rsi"), { length: 14 }, true)).toBeUndefined();
  const unknown = (patch: Partial<Indicator>) => ({ ...find("ema"), id: "zz_unknown", params: [], ...patch }) as Indicator;
  expect(firstRule(unknown({ outputs: ["line", "signal"], placement: "pane" }), {})).toEqual({ lhs: { s: "zz_unknown" }, op: "crosses_above", rhs: { s: "zz_unknown", out: "signal" } });
  expect(firstRule(unknown({ placement: "price" }), { length: 5 })).toEqual({ lhs: { s: "close" }, op: "crosses_above", rhs: { s: "zz_unknown", p: { length: 5 } } });
  expect(firstRule(unknown({ placement: "pane" }), {})).toEqual({ lhs: { s: "zz_unknown" }, op: "rising", bars: 3 });
});

it("every computed indicator in the table has a rule the lab can read: no stored parameters, an entry, a TradingView note", () => {
  for (const [id, r] of Object.entries(INDICATOR_RULES)) {
    expect(JSON.stringify(r), id).not.toMatch(new RegExp(`"s":"${id}","p"|"p":\\{[^}]*\\},"s":"${id}"`));
    expect(r.entry, id).toBeTypeOf("object");
    expect(r.tv.length, id).toBeGreaterThan(10);
  }
  expect(Object.keys(INDICATOR_RULES).length).toBeGreaterThan(500);
});

it("settings that can stop a trade are listed and switch off in one step", () => {
  const busy = spec({
    entry: { all: [rsi, { ...rsi, filter: true } as RuleNode] }, date_filter: { windows: [["2020-01-01", "2021-01-01"]], set: { name: "Base 2" } },
    execution: { fill: "close", shadow: { loss_pct: 5 }, cooldown: { bars: 3, after: "loss" }, max_trades: { count: 2, per: "M" }, risk: { max_consecutive_losses: 3, pause_bars: 10, max_drawdown_pct: 20 } },
  });
  const list = tradeBlockers(busy, { from: "2020-01-01" });
  expect(list.map((b) => b.id)).toEqual(["filters", "date_filter", "shadow", "loss_pause", "dd_stop", "cooldown", "max_trades", "period"]);
  expect(list.find((b) => b.id === "date_filter")!.text).toContain("“Base 2”");
  let s = busy;
  for (const b of list) if (b.off) s = b.off(s);
  expect(tradeBlockers(s).map((b) => b.id)).toEqual([]);
  expect(s.entry.all[1]).toEqual({ ...rsi, filter: true, off: true });
  expect(s.execution).toEqual({ fill: "close" });
});

it("More settings names every setting that differs from its default", () => {
  expect(changedSettings(spec())).toEqual([]); // fill at the close is a basic, not a More setting
  expect(changedSettings(spec({ exit: { any: [], stop_loss: { pct: 5 } }, execution: { fill: "close", cooldown: { bars: 3 }, allow_same_bar_exit: false } })))
    .toEqual(["cooldown", "no same-bar exit and entry"]); // stops and targets are set under the sell rules
  expect(stopsOn({ stop_loss: { pct: 5 }, trailing: { atr: 2, atr_length: 14 }, take_profits: [] }, 10)).toEqual(["stop-loss 5%", "trailing stop 2 × ATR", "N-bar stop 10 bars"]);
});
