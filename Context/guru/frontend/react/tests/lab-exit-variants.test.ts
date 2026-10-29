import { expect, it } from "vitest";
import { exitVariants, type ExitPlan } from "../src/features/lab/exit-variants";
import { defaultSpec } from "../src/features/lab/spec-model";
import type { RuleNode, Spec } from "../src/features/lab/types";

const rsiExit = { lhs: { s: "rsi", p: { length: 14 } }, op: "crosses_above", rhs: { value: 60 } } as unknown as RuleNode;
const base: Spec = { ...defaultSpec(), exit: { any: [rsiExit, { lhs: { s: "bars_held" }, op: ">=", rhs: { value: 10 } } as unknown as RuleNode], stop_loss: { pct: 3 } } };
const off = { on: false, pct: 5, mode: "close" as const };
const plan = (p: Partial<ExitPlan>): ExitPlan => ({ ncandle: "", atr: "", supertrend: "", indicator: "", hsl: off, target: off, ...p });

it("one variant per typed exit, the entry kept and the other exits cleared", () => {
  const v = exitVariants(base, plan({ ncandle: "3, 5, 7", atr: "5-1,1; 5-2,2", supertrend: "5-1; 5-1,2", indicator: "55, 70" }));
  expect(v.map((x) => x.tab)).toEqual(["Ncandle-3", "Ncandle-5", "Ncandle-7", "ATR 5-1,1", "ATR 5-2,2", "ST 5-1", "ST 5-1,2", "IND 55", "IND 70"]);
  expect(v.every((x) => JSON.stringify(x.spec.entry) === JSON.stringify(base.entry))).toBe(true);
  expect(v[0].spec.exit).toEqual({ any: [{ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 3 } }] }); // the strategy's stop-loss is not carried
  expect(v[3].spec.exit).toEqual({ any: [], dual_atr: { initial: { period: 5, mult: 1, price: "hl2" }, favour: { period: 5, mult: 1, price: "hl2" } } });
  expect(v[5].spec.exit.any).toEqual([{ lhs: { s: "supertrend", p: { atr_length: 5, factor: 1, mode: 1, price: 0 }, out: "direction" }, op: "crosses_above", rhs: { value: 0 } }]);
  expect((v[6].spec.exit as Record<string, unknown>).dual_line).toMatchObject({ initial: { p: { factor: 1 } }, favour: { p: { factor: 2 } } });
  expect(v[8].spec.exit.any).toEqual([{ ...rsiExit, rhs: { value: 70 } }]); // only the indicator row, threshold replaced
});

it("HSL and Profit booking ride on every variant; Both gives a live and a close sheet each", () => {
  const v = exitVariants(base, plan({ ncandle: "5", hsl: { on: true, pct: 5, mode: "both" }, target: { on: true, pct: 10, mode: "live" } }));
  expect(v.map((x) => x.tab)).toEqual(["Ncandle-5 HSL5L PB10", "Ncandle-5 HSL5C PB10"]);
  expect(v[0].spec.exit).toMatchObject({ stop_loss: { pct: 5 }, target: { pct: 10 } });
  expect(v[0].spec.execution?.exit_at_stop?.stop_loss).toBeUndefined(); // live = the default intraday check
  expect(v[1].spec.execution?.exit_at_stop?.stop_loss).toBe(0); // close
  expect(() => exitVariants({ ...base, short_side: base as never }, plan({ ncandle: "5" }))).toThrow(/one side/);
});

it("an exit type switched off makes no sheets but keeps what was typed", () => {
  const p = plan({ ncandle: "3, 5", atr: "5-1,1", off: { ncandle: true } });
  expect(exitVariants(base, p).map((x) => x.tab)).toEqual(["ATR 5-1,1"]);
  expect(p.ncandle).toBe("3, 5");
});

it("variants keep the Exit card's settings: a weekly ATR stays weekly, a weekly Supertrend stays weekly", () => {
  const wk: Spec = { ...base, exit: { any: [{ lhs: { s: "supertrend", p: { atr_length: 5, factor: 1, mode: 1, price: 0 }, out: "direction", tf: "W" }, op: "crosses_above", rhs: { value: 0 } } as unknown as RuleNode],
    dual_atr: { initial: { period: 5, mult: 2, price: "hl2" }, favour: { period: 5, mult: 4, price: "hl2" }, tf: "W" } } as Spec["exit"] };
  const v = exitVariants(wk, plan({ atr: "5-1,1", supertrend: "5-2" }));
  expect(v.map((x) => x.tab)).toEqual(["ATR 5-1,1 W", "ST 5-2 W"]);
  expect((v[0].spec.exit as Record<string, unknown>).dual_atr).toMatchObject({ tf: "W", initial: { period: 5, mult: 1 } });
  expect((v[1].spec.exit.any[0] as { lhs: { tf?: string } }).lhs.tf).toBe("W");
});

it("N candle on a weekly / monthly timeframe: the engine's period_end count, read back, worded, and kept by the variants", async () => {
  const { nCandleNode, nCandleOf } = await import("../src/features/lab/ExitChoice");
  const { words } = await import("../src/features/lab/PlainRules");
  const w3 = nCandleNode("any", 3, "W");
  expect(w3).toEqual({ lhs: { since_entry: "count", when: { lhs: { s: "period_end", out: "w" }, op: ">", rhs: { value: 0 } } }, op: ">=", rhs: { value: 3 } });
  const up = nCandleNode("up", 2, "M") as unknown as { lhs: { when: { all: { rhs: unknown }[] } } };
  expect(up.lhs.when.all[1].rhs).toEqual({ s: "close", tf: "M", offset: 1 }); // the previous month's close on a month-end bar
  for (const [k, n, tf] of [["any", 3, "W"], ["up", 2, "M"], ["down", 4, "W"], ["any", 10, "D"], ["up", 5, "D"]] as const) expect(nCandleOf(nCandleNode(k, n, tf))).toEqual({ kind: k, n, tf });
  expect(words(w3, { indicators: [] } as never).text).toBe("after 3 weekly candles");
  expect(words(up as never, { indicators: [] } as never).text).toBe("at the 2nd monthly up close after entry");
  const v = exitVariants({ ...base, exit: { any: [w3] } }, plan({ ncandle: "5, 7" }));
  expect(v.map((x) => x.tab)).toEqual(["Ncandle-5 W", "Ncandle-7 W"]);
  expect(v[0].spec.exit.any[0]).toEqual(nCandleNode("any", 5, "W"));
});
