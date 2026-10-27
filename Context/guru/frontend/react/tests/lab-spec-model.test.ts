import { expect, it } from "vitest";
import * as m from "../src/features/lab/spec-model";
import type { Spec } from "../src/features/lab/types";
import { catalogueFixture as cat, catalogue7Fixture as cat7, templateFixture } from "./fixtures/lab";

const variant = (): Spec => (templateFixture.status === "available" ? structuredClone(templateFixture.template.spec) : m.defaultSpec());

it("default spec is close > EMA(50) entry and a 5-bar time exit", () => {
  const s = m.defaultSpec();
  expect(s.entry.all[0]).toEqual({ lhs: { s: "close" }, op: ">", rhs: { s: "ema", p: { length: 50 } } });
  expect(s.exit.any[0]).toEqual({ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } });
  expect(m.specProblems(s, cat)).toEqual([]);
});

it("edits keep origin and unknown keys and never mutate the input", () => {
  const s = { ...variant(), future_rule: { q: 1 } } as Spec;
  const before = JSON.stringify(s);
  let t = m.setCond(s, "entry", 0, { lhs: { s: "rsi", p: { length: 3 } }, op: "<", rhs: { value: 10 }, tag: "x" });
  t = m.addCond(t, "exit", { lhs: { s: "bars_held" }, op: ">=", rhs: { value: 3 } });
  t = m.setOrder(t, "stop_loss", { atr: 2, atr_length: 14 });
  t = m.setDirection(t, "short");
  t = m.setAt(t, "entry.all[0].rhs.value", 12);
  expect(JSON.stringify(s)).toBe(before);
  expect(t.origin).toBeDefined();
  expect(t.origin).toBe(s.origin);
  expect(t.future_rule).toEqual({ q: 1 });
  expect(t.entry.all[0]).toMatchObject({ tag: "x", rhs: { value: 12 } });
  expect(JSON.parse(JSON.stringify(t))).toEqual(t);
  expect(m.setDirection(t, "long").direction).toBeUndefined();
  expect(m.removeCond(t, "exit", 1).exit.any).toHaveLength(1);
});

it("a limit entry removes execution.fill; removing an order deletes the key", () => {
  const s = m.setExecution(m.defaultSpec(), { fill: "close", exit_fill: "close" });
  const limited = m.setOrder(s, "limit", { pct: 1 });
  expect(limited.entry.limit).toEqual({ pct: 1 });
  expect(limited.execution).toEqual({ exit_fill: "close" });
  expect("limit" in m.setOrder(limited, "limit", undefined).entry).toBe(false);
  expect(m.setExecution(m.defaultSpec(), { fill: undefined }).execution).toBeUndefined();
  // fill also set the exit fill, so adding a limit must not move exits to the next open.
  expect(m.setOrder(m.setExecution(m.defaultSpec(), { fill: "close" }), "limit", { pct: 1 }).execution).toEqual({ exit_fill: "close" });
  expect(m.setOrder(m.setExecution(m.defaultSpec(), { fill: "close", exit_fill: "next_open" }), "limit", { pct: 1 }).execution)
    .toEqual({ exit_fill: "next_open" });
  expect(m.setOrder(m.defaultSpec(), "limit", { pct: 1 }).execution).toBeUndefined();
});

it("getAt/setAt follow the engine's optimize path grammar", () => {
  const s = m.defaultSpec();
  expect(m.getAt(s, "entry.all[0].rhs.p.length")).toBe(50);
  expect(m.getAt(m.setAt(s, "exit.stop_loss.pct", 3), "exit.stop_loss.pct")).toBe(3);
  expect(() => m.setAt(s, "entry.all[x]", 1)).toThrow();
});

it("lists optimizable settings for params, numbers and price orders", () => {
  const s = m.setOrder(m.defaultSpec(), "target", { pct: 5 });
  const paths = m.optimizablePaths(s, cat);
  expect(paths.map((p) => p.path)).toEqual(["entry.all[0].rhs.p.length", "exit.any[0].rhs.value", "exit.target.pct"]);
  expect(paths[0]).toMatchObject({ label: "Entry 1 · Exponential Moving Average Length", current: 50, min: 1, max: 5000, step: 1, integer: true });
  expect(paths[2]).toMatchObject({ min: 0.5, max: 1000 });
  const more = m.optimizablePaths(m.setOrder(m.setOrder(s, "limit", { pct: 1 }), "stop_loss", { atr: 2 }), cat);
  expect(more.find((p) => p.path === "entry.limit.pct")).toMatchObject({ min: 0, max: 50 });
  expect(more.find((p) => p.path === "exit.stop_loss.atr")).toMatchObject({ min: 0.25, max: 50 });
});

it("counts combinations like the engine", () => {
  expect(m.valuesCount({ path: "a", from: 10, to: 30, step: 10 })).toBe(3);
  expect(m.valuesCount({ path: "a", from: 0.1, to: 0.3, step: 0.1 })).toBe(3);
  expect(m.valuesCount({ path: "a", from: 5, to: 1, step: 1 })).toBe(0);
  expect(m.comboCount([{ path: "a", from: 1, to: 3, step: 1 }, { path: "b", from: 1, to: 2, step: 1 }])).toBe(6);
  expect(m.comboCount([])).toBe(0);
  expect(m.withOptimize(m.defaultSpec(), []).optimize).toBeUndefined();
});

it("pre-checks: two numbers, empty exit without a price exit, listed indicators, bounds, condition cap", () => {
  const s = m.defaultSpec();
  expect(m.specProblems(m.setCond(s, "entry", 0, { lhs: { value: 1 }, op: ">", rhs: { value: 2 } }), cat))
    .toContain("Entry 1: a condition cannot compare two numbers; one side must be a series.");
  const noExit = m.removeCond(s, "exit", 0);
  expect(m.specProblems(noExit, cat)).toContain("exit.any needs at least one condition (or a stop_loss, target, trailing, breakeven or take-profit exit)");
  expect(m.specProblems(m.setOrder(noExit, "trailing", { pct: 5 }), cat)).toEqual([]);
  expect(m.specProblems(m.setCond(s, "entry", 0, { lhs: { s: "or_range" }, op: ">", rhs: { value: 1 } }), cat)[0])
    .toBe("Entry 1: Opening Range is listed only and cannot be used in rules. Needs intraday (minute) bars; not available yet.");
  expect(m.specProblems(m.setOrder(s, "stop_loss", { pct: 0 }), cat)).toContain("Stop-loss: % must be above 0 and at most 99.");
  expect(m.specProblems(m.setOrder(s, "limit", { pct: 0 }), cat)).toEqual([]);
  for (const atr of [0, 51]) expect(m.specProblems(m.setOrder(s, "target", { atr }), cat)).toContain("Target: ATR multiplier must be above 0 and at most 50.");
  expect(m.specProblems(m.setOrder(s, "target", { atr: 50 }), cat)).toEqual([]);
  expect(m.specProblems(m.setCond(s, "entry", 0, { lhs: { s: "bars_held" }, op: ">", rhs: { value: 1 } }), cat))
    .toContain("Entry 1: Bars held can only be used in exit conditions.");
  expect(m.specProblems(m.setOrder(s, "target", { atr: 2, atr_length: 600 }), cat)).toContain("Target: ATR length must be a whole number from 1 to 500.");
  let many = s;
  for (let i = 0; i < 11; i++) many = m.addCond(many, "entry", s.entry.all[0]);
  expect(m.specProblems(many, cat)).toContain("Use at most 12 conditions in total.");
  expect(m.specProblems(m.setCond(s, "entry", 0, { lhs: { s: "phase7_thing" }, op: ">", rhs: { value: 1 } }), cat)).toEqual([]);
});

it("rejects specs that would exceed the gateway query cap", () => {
  let s = m.defaultSpec();
  s = { ...s, note_padding: "x".repeat(7600) } as Spec;
  expect(m.specProblems(s, cat)).toContain("These rules are too long to send. Remove a condition or shorten settings.");
  const encoded = { ...m.defaultSpec(), pad: "{}".repeat(3500) } as Spec; // 7 KB raw, ~21 KB URL-encoded
  expect(m.specBytes(encoded)).toBeLessThan(7500);
  expect(m.specProblems(encoded, cat)).toContain("These rules are too long to send. Remove a condition or shorten settings.");
});

it("sends cost from the fees box only when the spec has no cost_bps", () => {
  expect(m.runQuery(m.defaultSpec(), "5", "2")).toEqual({ cost: 5, slippage: 2 });
  expect(m.runQuery(m.defaultSpec(), "", "")).toEqual({});
  expect(m.runQuery(m.setExecution(m.defaultSpec(), { cost_bps: 10 }), "5", "")).toEqual({});
});

it("split date leaves more than 20 bars each side", () => {
  const dates = Array.from({ length: 200 }, (_, i) => `d${String(i).padStart(3, "0")}`);
  expect(m.splitDate(dates, 30)).toBe("d140");
  expect(m.splitDate(dates.slice(0, 60), 30)).toBeNull();
  expect(m.splitDate(dates, 0)).toBeNull();
  expect(m.splitDate(dates.slice(0, 42), 50)).toBe("d021"); // exactly 21 bars each side
  expect(m.splitDate(dates.slice(0, 41), 50)).toBeNull(); // 20 bars before the split
  expect(m.splitDate(dates.slice(0, 41), 48)).toBeNull(); // 20 bars after the split
});

it("maps exit reasons to friendly text and keeps unknown text", () => {
  expect(["open", "exit.stop_loss", "exit.target (gap at the open)", "exit.trailing", "exit.any[1]: rsi > 70", "entry.all[2]: filter turned false: close > ema(length=50)", "something new"].map(m.friendlyReason))
    .toEqual(["Still open at the end", "Stop-loss", "Target (gap at the open)", "Trailing stop", "Exit rule 2: rsi > 70", "Filter turned false: close > ema(length=50)", "something new"]);
});

it("canonical JSON ignores key order (jsonb does not keep it)", () => {
  expect(m.canonicalJson({ b: 1, a: { d: 2, c: [3, { z: 1, y: 2 }] } })).toBe(m.canonicalJson({ a: { c: [3, { y: 2, z: 1 }], d: 2 }, b: 1 }));
});

it("labels operands for summaries", () => {
  expect(m.operandLabel({ s: "ema", p: { length: 50 }, tf: "W", offset: 1 }, cat)).toBe("Exponential Moving Average(50) · W · 1 bar ago");
  expect(m.operandLabel({ s: "macd", out: "signal" }, cat)).toBe("MACD.signal");
  expect(m.operandLabel({ value: 30 }, cat)).toBe("30");
  expect(m.operandLabel({ s: "bars_held" }, cat)).toBe("Bars held");
  expect(m.conditionCount(m.defaultSpec())).toBe(2);
});

it("isCond tells v1 comparisons from Phase 7 nodes", () => {
  expect(m.isCond({ lhs: { s: "close" }, op: ">", rhs: { value: 1 } })).toBe(true);
  expect(m.isCond({ lhs: { s: "close" }, op: "between", lo: { value: 1 }, hi: { value: 2 } })).toBe(false);
  expect(m.isCond({ lhs: { s: "close" }, op: "rising", bars: 3 })).toBe(false);
  expect(m.isCond({ all: [] })).toBe(false);
});

it("removeAt and putAt edit by engine path and never mutate", () => {
  const s = m.setAt(m.defaultSpec(), "exit.breakeven.after.pct", 2);
  const before = JSON.stringify(s);
  expect(m.removeAt(s, "exit.breakeven").exit).not.toHaveProperty("breakeven");
  expect(m.putAt(s, "exit.min_bars", 3).exit.min_bars).toBe(3);
  expect(m.putAt(s, "exit.min_bars", undefined).exit).not.toHaveProperty("min_bars");
  expect(m.removeAt(s, "exit.nothing.here")).toEqual(s);
  expect(JSON.stringify(s)).toBe(before);
});

it("pre-checks: entry, exit (price exits incl. breakeven and take-profits; always-in), per-side top-level cap, size", () => {
  const s = m.defaultSpec();
  const noExit = m.removeCond(s, "exit", 0);
  expect(m.specProblems(noExit, cat)).toContain("exit.any needs at least one condition (or a stop_loss, target, trailing, breakeven or take-profit exit)");
  expect(m.specProblems(m.setAt(noExit, "exit.breakeven.after.pct", 2), cat)).toEqual([]);
  expect(m.specProblems(m.setAt(noExit, "exit.take_profits", [{ pct: 2, size: 0.5 }]), cat)).toEqual([]);
  const always = { ...noExit, direction: "both", short_side: { entry: { all: [s.entry.all[0]] }, exit: { any: [] } } } as Spec;
  expect(m.specProblems(always, cat)).toEqual([]);
  expect(m.specProblems(m.setExecution(always, { opposite: "exit" }), cat)).toContain("exit.any needs at least one condition (or a stop_loss, target, trailing, breakeven or take-profit exit)");
  let many = s;
  for (let i = 0; i < 11; i++) many = m.addCond(many, "entry", s.entry.all[0]);
  expect(m.specProblems(many, cat)).toContain("Use at most 12 conditions in total.");
  const shortMany = { ...s, direction: "both", short_side: { entry: { all: Array(12).fill(s.entry.all[0]) }, exit: { any: [s.exit.any[0]] } } } as Spec;
  expect(m.specProblems(shortMany, cat)).toContain("Short side: use at most 12 conditions in total.");
  expect(m.specProblems({ ...s, entry: { all: [] } }, cat)).toContain("Add a buy rule first: pick a suggestion under “Buy when…” in Your idea, or press + Add condition.");
});

it("money mode sends no cost or slippage query; otherwise as before", () => {
  expect(m.runQuery(m.defaultSpec(), "5", "2")).toEqual({ cost: 5, slippage: 2 });
  expect(m.runQuery(m.defaultSpec(), "", "")).toEqual({});
  expect(m.runQuery(m.setExecution(m.defaultSpec(), { cost_bps: 10 }), "5", "")).toEqual({});
  expect(m.runQuery(m.setExecution(m.defaultSpec(), { capital: 100000 }), "5", "2")).toEqual({});
});

it("maps Phase 7 exit reasons to friendly text and keeps unknown text", () => {
  expect(["open", "exit.stop_loss", "exit.target (gap at the open)", "exit.trailing", "exit.breakeven", "exit.breakeven (gap at the open)",
    "exit.any[1]: rsi > 70", "short_side.exit.any[0]: rsi < 30", "reverse to short", "reverse to long", "opposite entry", "expiry", "margin_call", "roll", "something new"]
    .map(m.friendlyReason)).toEqual(["Still open at the end", "Stop-loss", "Target (gap at the open)", "Trailing stop", "Breakeven stop",
    "Breakeven stop (gap at the open)", "Exit rule 2: rsi > 70", "Short exit rule 1: rsi < 30", "Reversed to short", "Reversed to long",
    "Opposite signal", "Contract expiry", "Margin call", "Contract roll", "something new"]);
});

it("operator labels cover the Phase 7 operators", () => {
  expect(cat7.operators.every((op) => m.OPERATOR_LABELS[op])).toBe(true);
});

it("new nodes and point distances keep Customize summaries usable", () => {
  const s = m.setAt(m.defaultSpec(), "entry.all[0]", { all: [{ lhs: { s: "close" }, op: ">", rhs: { value: 1 } }] });
  expect(() => m.specProblems(s, cat)).not.toThrow();
  expect(m.specSummary(s, cat)).toContain("a rule shown only in Strategy rules");
  const points = m.setOrder(m.defaultSpec(), "target", { points: 4 });
  expect(m.specSummary(points, cat)).toContain("4 points profit target");
  expect(m.optimizablePaths(points, cat).some((x) => x.path === "exit.target.atr")).toBe(false);
  expect(m.describeSpec(variant(), cat).entry[0]).toContainEqual(expect.objectContaining({ path: "entry.all[0].rhs.value", value: 4 }));
});

it("removeAt splices arrays and preserves unknown keys and missing paths", () => {
  const s = m.setAt(m.defaultSpec(), "future", { note: "keep" });
  expect(m.removeAt(s, "entry.all[0]").entry.all).toEqual([]);
  expect(m.removeAt(s, "missing.path")).toBe(s);
  expect(m.removeAt(s, "entry.all[0]").future).toEqual({ note: "keep" });
});

it("point-distance prechecks keep zero-price orders and reject invalid protective distances", () => {
  expect(m.specProblems(m.setOrder(m.defaultSpec(), "limit", { points: 0 }), cat)).toEqual([]);
  for (const points of [0, -1, Infinity, 1000000001])
    expect(m.specProblems(m.setOrder(m.defaultSpec(), "stop_loss", { points }), cat).length).toBeGreaterThan(0);
});

it("extended catalogues defer entry At levels to execution validation", () => {
  const spec = m.setAt(m.defaultSpec(), "entry.limit", { at: { s: "close" } });
  expect(m.specProblems(spec, cat7).some((p) => p.includes("ATR multiplier"))).toBe(false);
});
