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

it("exit reasons name the rule that closed the trade: own rows by their number, default-exit rows as the default exit", () => {
  const label = reasonLabeler({ spec, own, catalogue: cat });
  expect(label("exit.any[2]: bars_held >= 10")).toBe("Default exit: after 10 days");
  expect(label("exit.any[1]: since_entry.count(close > close[1]) >= 15")).toBe("Default exit: at the 15th up close after entry");
  const mine: Spec = { ...own, exit: { any: [rsi("crosses_below", 70)], stop_loss: { atr: 1, atr_length: 5 } } };
  const mineLabel = reasonLabeler({ spec: mine, own: mine, catalogue: cat });
  expect(mineLabel("exit.any[0]: rsi(length=14) crosses_below 70")).toBe("Sell rule 1: RSI(14) crosses below 70");
  expect(mineLabel("exit.stop_loss")).toBe("Stop-loss 1 × ATR(5)");
  expect(label("open")).toBe("Still open at the end of the test");
});

it("the Rules sheet lists the settings, the entry and exit rules as numbered in the exit reasons, and how a trade works", () => {
  const rows = rulesSheet({ spec, own, catalogue: cat, fees: 5, slippage: 0 }, runFixture);
  const text = rows.map((r) => r.join(" | "));
  expect(rows[0]).toEqual(["Section", "Item", "What it is"]);
  expect(text).toContain("Entry | Buy rule 1 | RSI(14) crosses above 60");
  expect(text).toContain("Exit | Sell rule 1 (switched off, not used) | RSI(14) crosses above 70");
  expect(text.some((t) => t.startsWith("Exit | Default exit | after 10 days"))).toBe(true);
  expect(text.some((t) => t.startsWith("Test | Costs | fees 5 bps"))).toBe(true);
  expect(text.some((t) => t.startsWith("How it works | crosses above / below"))).toBe(true);
});
