import { expect, it } from "vitest";
import { reasonLabeler, rulesSheet } from "../src/features/lab/trade-explain";
import { withDefaultExit } from "../src/features/lab/defaults";
import { starterSpec } from "../src/features/lab/spec-model";
import type { RuleNode, Spec } from "../src/features/lab/types";
import { catalogue7Fixture as cat, runFixture } from "./fixtures/lab";

const rsi = (op: string, v: number, off = false) => ({ lhs: { s: "rsi", p: { length: 14 } }, op, rhs: { value: v }, ...(off ? { off: true } : {}) }) as RuleNode;
// the screenshot's test: buy RSI crosses above 60, the only sell row switched off, default exit = 15th up close or 10 days
const own: Spec = { ...starterSpec(), entry: { all: [rsi("crosses_above", 60)] }, exit: { any: [rsi("crosses_above", 70, true)] } };
const spec = withDefaultExit(own, { up: 15, down: 4, downOff: true, days: 10 });

it("exit reasons are short names, as the Exit card calls them", () => {
  const label = reasonLabeler({ spec, own, catalogue: cat });
  expect(label("exit.any[2]: bars_held >= 10")).toBe("N candle");
  expect(label("exit.any[1]: since_entry.count(close > close[1]) >= 15")).toBe("N candle");
  const mine: Spec = { ...own, exit: { any: [rsi("crosses_below", 70)], stop_loss: { atr: 1, atr_length: 5 } } };
  const mineLabel = reasonLabeler({ spec: mine, own: mine, catalogue: cat });
  expect(mineLabel("exit.any[0]: rsi(length=14) crosses_below 70")).toBe("Exit rule 1");
  expect(mineLabel("exit.stop_loss")).toBe("HSL");
  expect(mineLabel("exit.target (gap at the open)")).toBe("Profit booking (gap)");
  expect(mineLabel("short_side.exit.any[0]: x")).toBe("Short: Exit rule 1");
  expect(label("exit.any[0]: setup(%B < 0) then trigger(…)")).toBe("Exit rule 1"); // the switched-off RSI row is row 1
  expect(reasonLabeler({ spec: { ...own, exit: { any: [{ setup: rsi("<", 30), trigger: rsi(">", 30) } as unknown as RuleNode] } }, own, catalogue: cat })("exit.any[0]: setup(…)")).toBe("Band setup");
  expect(label("date_filter: window ended")).toBe("Date window ended");
  expect(label("exit.dual_atr")).toBe("ATR");
  expect(label("open")).toBe("Open");
});

it("the Rules sheet is a title, then Item | Value under Strategy, Entry and Exit; no explanations", () => {
  const rows = rulesSheet({ spec, own, catalogue: cat, fees: 5, slippage: 0 }, runFixture);
  const text = rows.map((r) => r.join(" | "));
  expect(rows[0]).toEqual(["STRATEGY RULES"]);
  expect(text.filter((t) => ["Strategy", "Entry", "Exit"].includes(t))).toEqual(["Strategy", "Entry", "Exit"]);
  expect(rows.every((r) => r.length <= 2)).toBe(true);
  expect(text).toContain("Entry 1 | RSI(14) crosses above 60");
  expect(text).toContain("Exit rule 1 (off) | RSI(14) crosses above 70");
  expect(text).toContain("N candle | after 10 days");
  expect(text).toContain("Fees | 5 bps per side");
  expect(text).toContain("Filter | Without filter");
  expect(text.some((t) => /^(Buy and sell at|Slippage|Results in) \|/.test(t))).toBe(false);
  // with a date filter and an indicator filter: one Filter row names both; the filter row is not repeated under Entry
  const filtered: Spec = { ...spec, entry: { all: [...spec.entry.all, { ...rsi(">", 50), filter: true } as RuleNode] },
    date_filter: { windows: [["2020-01-01", "2020-06-30"], ["2021-01-01", "2021-03-31"]], set: { name: "Base 1" } } };
  const withF = rulesSheet({ spec: filtered, own: filtered, catalogue: cat }, runFixture).map((r) => r.join(" | "));
  expect(withF).toContain("Filter | With filter — Dates: Base 1 · 2 periods; Indicator: RSI(14) is above 50");
  expect(withF.filter((t) => t.startsWith("Entry "))).toEqual(["Entry 1 | RSI(14) crosses above 60"]);
  expect(text.some((t) => /How it works|What it is|Columns/.test(t))).toBe(false);
});

it("the tradesheet tab is named like the team's (index code, entry, exit) and the Rules fields are filled", async () => {
  const { tsTabName, tsRules } = await import("../src/features/lab/trade-explain");
  const st = (p: Record<string, number>) => ({ s: "supertrend", p, out: "supertrend" });
  const s: Spec = { ...own, entry: { all: [{ lhs: { s: "close" }, op: "crosses_above", rhs: { ...st({ atr_length: 5, factor: 1, mode: 1 }), offset: 1 } } as unknown as RuleNode] },
    exit: { any: [{ lhs: { since_entry: "count", when: { lhs: { s: "close" }, op: ">", rhs: { s: "open" } } }, op: ">=", rhs: { value: 9 } } as unknown as RuleNode] } };
  const r = { ...runFixture, instrument: { ...runFixture.instrument, ticker: "NIFTY 50" }, from: "2000-01-03", to: "2026-10-09" };
  expect(tsTabName({ spec: s, own: s, catalogue: cat }, r)).toBe("NF_En_ST 5,1_EX_Ncandle-9");
  const rules = tsRules({ spec: s, own: s, catalogue: cat }, r);
  expect(rules).toMatchObject({ timeframe: "Daily", index: "NIFTY 50 (03-01-2000 to 09-10-2026)", position: "Long", filter: "Without filter" });
  expect(rules.exit[0]).toMatch(/^N candle: /);
});
