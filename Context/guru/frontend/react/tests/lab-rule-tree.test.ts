import { expect, it } from "vitest";
import * as t from "../src/features/lab/rule-tree";
import { EXPAND_FIRST_REFUSED } from "../src/features/lab/labels";
import { getAt } from "../src/features/lab/spec-model";
import type { Cond, Operand, RuleNode, Spec } from "../src/features/lab/types";
import { catalogueFixture as cat, catalogue7Fixture as cat7 } from "./fixtures/lab";

const C: Cond = { lhs: { s: "close" }, op: ">", rhs: { s: "ema", p: { length: 50 } } };
const spec = (patch: Partial<Spec>): Spec => ({ v: 1, entry: { all: [C] }, exit: { any: [{ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } }] }, ...patch });

it("classifies nodes and operands", () => {
  expect(t.nodeKind(C)).toBe("cmp");
  expect(t.nodeKind({ lhs: { s: "rsi" }, op: "between", lo: { value: 30 }, hi: { value: 70 } })).toBe("range");
  expect(t.nodeKind({ lhs: { s: "close" }, op: "falling", bars: 3 })).toBe("trend");
  expect((["all", "any", "not", "at_least", "then"] as const).map((k) => t.nodeKind(t.newNode(k)))).toEqual(["all", "any", "not", "at_least", "then"]);
  expect(t.nodeKind({ fancy: 1 } as never)).toBe("unknown");
  const k = (o: Operand, c = cat7) => t.operandKind(o, c);
  expect([k({ value: 3 }), k({ s: "close" }), k({ s: "rsi" }), k({ s: "bars_held" }), k({ s: "session", out: "tdom" }), k({ s: "nope" })])
    .toEqual(["number", "price", "indicator", "bars_held", "calendar", "unknown"]);
  expect([k({ sub: [{ s: "close" }, { value: 1 }] }), k({ pct: [{ s: "close" }, { s: "open" }] }), k({ avg: { s: "close" }, bars: 5 }),
    k({ change: { s: "close" }, bars: 1 }), k({ ref: { s: "close" }, bars: 1 }), k({ highest_since: C, of: { s: "high" } }),
    k({ count: C, bars: 5 }), k({ compress: { s: "close" }, tf: "W" })]).toEqual(["arith", "pct", "window", "change", "ref", "since", "count", "compress"]);
});

it("budget counts like the engine (comparisons, groups, expressions; both sides; at levels)", () => {
  const s: Spec = { v: 1, direction: "both",
    entry: { all: [{ any: [C, { not: C }] }, { lhs: { ref: { s: "close" }, bars: 1 }, op: "<", rhs: { value: 5 } }], stop: { at: { highest: { s: "high" }, bars: 5 } } },
    exit: { any: [{ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 3 } }] },
    short_side: { entry: { all: [{ at_least: 2, of: 3, when: C }] }, exit: { any: [{ lhs: { bars_since: C }, op: ">", rhs: { value: 4 } }] } } };
  expect(t.budget(s)).toMatchObject({ leaves: 7, nodes: 13, top: { long: 3, short: 2 } });
  expect(t.budget(spec({ entry: { all: [{ lhs: { s: "close", tf: "W", expand: "point" }, op: ">", rhs: { value: 1 } }] } }))).toMatchObject({ leaves: 2, nodes: 2 });
  expect(t.limits(cat)).toEqual({ leaves: 40, nodes: 120, top: 12 });
});

it("walker yields the engine's optimize paths in engine order, as JSON paths", () => {
  const s = spec({
    entry: { all: [
      { lhs: { s: "close" }, op: "rising", bars: 3 },
      { at_least: 2, of: 3, when: { lhs: { avg: { s: "close" }, bars: 10 }, op: ">", rhs: { value: 1 } } },
      { then: [C, { lhs: { s: "rsi", p: { length: 2 } }, op: "<", rhs: { ref: { s: "rsi" }, bars: 0 } }], within: 5 },
    ] },
    exit: { any: [{ lhs: { value_when: C, of: { s: "close" }, nth: 2 }, op: "<", rhs: { s: "close" } }] },
  });
  const knobs: t.WalkItem[] = [];
  t.walkSpec(s, (x) => x.t === "knob" && knobs.push(x));
  expect(knobs.map((x) => x.path)).toEqual(["entry.all[0].bars", "entry.all[1].at_least", "entry.all[1].of", "entry.all[1].when.lhs.bars",
    "entry.all[1].when.rhs.value", "entry.all[2].then[1].rhs.bars", "entry.all[2].within", "exit.any[0].lhs.nth"]);
  for (const x of knobs) if (x.t === "knob") expect(getAt(s, x.path)).toBe(x.value);
  expect(knobs.find((x) => x.path.endsWith("rhs.bars"))).toMatchObject({ min: 0, max: 1000, whole: true });
  expect(knobs.find((x) => x.path.endsWith(".nth"))).toMatchObject({ min: 1, max: 20 });
  const held: string[] = [];
  t.walkSpec(spec({ exit: { any: [{ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } }, { not: { lhs: { s: "bars_held" }, op: ">", rhs: { value: 1 } } }] } }),
    (x) => x.t === "operand" && x.held && held.push(x.path));
  expect(held).toEqual(["exit.any[0].lhs", "exit.any[0].rhs"]);
});

it("labels engine paths in plain words", () => {
  expect(t.pathLabel("entry.all[0]", false)).toBe("Entry 1");
  expect(t.pathLabel("entry.all[2].any[0].rhs.value", false)).toBe("Entry 3 › any 1 › right › number");
  expect(t.pathLabel("exit.any[0]", true)).toBe("Long exit 1");
  expect(t.pathLabel("short_side.exit.any[1].lhs.bars_since", true)).toBe("Short exit 2 › left › bars since");
  expect(t.pathLabel("entry.stop.at.bars", false)).toBe("Stop level › bars");
  expect(t.pathLabel("entry.all[1].when.lhs.bars", false)).toBe("Entry 2 › when › left › bars");
  expect(t.pathLabel("entry.all[0].lhs.p.length", false)).toBe("Entry 1 › left › length");
});

it("tree problems mirror rules.rs in plain words", () => {
  expect(t.treeProblems(spec({}), cat7)).toEqual([]);
  const p = (s: Spec) => t.treeProblems(s, cat7);
  expect(p(spec({ entry: { all: [{ any: [{ lhs: { value: 1 }, op: ">", rhs: { value: 2 } }] }] } })))
    .toContain("Entry 1 › any 1: a condition cannot compare two numbers; one side must be a series.");
  expect(p(spec({ entry: { all: [{ at_least: 4, of: 3, when: C }] } }))).toContain('Entry 1: "at least" (4) cannot exceed the number of bars (3).');
  expect(p(spec({ entry: { all: [{ then: [C], within: 5 }] } }))).toContain("Entry 1: a sequence needs 2 to 4 steps.");
  expect(p(spec({ entry: { all: [{ then: [C, C, C], within: 1 }] } }))).toContain("Entry 1: within (1) is shorter than the 3 steps need (2).");
  expect(p(spec({ entry: { all: [{ all: [] }] } }))).toContain("Entry 1: add at least one condition to this group.");
  expect(p(spec({ entry: { all: [{ lhs: { s: "close" }, op: "between", lo: { value: 9 }, hi: { value: 1 } }] } }))).toContain("Entry 1: the low bound is above the high bound.");
  expect(p(spec({ entry: { all: [{ lhs: { s: "bars_held" }, op: ">", rhs: { value: 1 } }] } })))
    .toContain("Entry 1 › left: Bars held works only in a top-level exit condition (not inside a group, range, trend or formula).");
  expect(p(spec({ exit: { any: [{ not: { lhs: { s: "bars_held" }, op: ">", rhs: { value: 1 } } }] } })))
    .toContain("Exit 1 › not › left: Bars held works only in a top-level exit condition (not inside a group, range, trend or formula).");
  expect(p(spec({ entry: { all: [{ lhs: { s: "or_range" }, op: ">", rhs: { value: 1 } }] } })))
    .toContain("Entry 1 › left: Opening Range is listed only and cannot be used in rules. Needs intraday (minute) bars; not available yet.");
  expect(p(spec({ entry: { all: [{ lhs: { s: "close", tf: "W", expand: "first" }, op: ">", rhs: { value: 1 } }] } }))).toContain(`Entry 1 › left: ${EXPAND_FIRST_REFUSED}`);
  expect(p(spec({ entry: { all: [{ lhs: { s: "open", tf: "W", expand: "first" }, op: ">", rhs: { value: 1 } }] } }))).toEqual([]);
  expect(p(spec({ entry: { all: [{ lhs: { compress: { s: "high" }, tf: "M", mode: "first", expand: "first" }, op: ">", rhs: { value: 1 } }] } }))).toEqual([]);
  expect(p(spec({ entry: { all: [{ lhs: { s: "close", expand: "point" }, op: ">", rhs: { value: 1 } }] } })))
    .toContain("Entry 1 › left: “Use the period value” applies to W, M, Q, Y and N-session values only.");
  expect(p(spec({ entry: { all: [{ lhs: { s: "close", tf: "70D" }, op: ">", rhs: { value: 1 } }] } })))
    .toContain("Entry 1 › left: timeframe must be D, W, M, Q, Y or N sessions (2 to 63).");
  expect(p(spec({ entry: { all: [{ lhs: { ref: { value: 3 }, bars: 1 }, op: ">", rhs: { s: "close" } }] } }))).toContain("Entry 1 › left: N bars ago needs a series, not a number.");
  expect(p(spec({ entry: { all: [{ lhs: { add: [{ value: 1 }, { value: 2 }] }, op: ">", rhs: { s: "close" } }] } })))
    .toContain("Entry 1 › left: a formula cannot combine two numbers; use at least one series.");
  expect(p(spec({ entry: { all: [{ lhs: { avg: { s: "close" }, bars: 0 }, op: ">", rhs: { s: "close" } }] } }))).toContain("Entry 1 › left › bars must be a whole number from 1 to 1000.");
  const big = spec({ entry: { all: Array.from({ length: 12 }, () => ({ all: [C, C, C, C] })) } });
  expect(p(big)).toContain("More than 40 comparisons in the rules; the limit is 40.");
  let deep: RuleNode = C;
  for (let i = 0; i < 12; i++) deep = { not: deep };
  expect(p(spec({ entry: { all: [deep] } }))).toContain("These rules are nested too deeply (the engine reads at most 16 levels). Ungroup a group or remove a formula.");
});

it("converts operands: wraps, swaps formulas keeping the inner value, replaces plain kinds", () => {
  const rsi = { s: "rsi", p: { length: 2 } };
  expect(t.convertOperand(rsi, "ref", cat7)).toEqual({ ref: rsi, bars: 1 });
  expect(t.convertOperand({ ref: rsi, bars: 1 }, "window", cat7)).toEqual({ avg: rsi, bars: 10 });
  expect(t.convertOperand({ value: 5 }, "abs", cat7)).toEqual({ abs: { s: "close" } });
  expect(t.convertOperand({ value: 5 }, "arith", cat7)).toEqual({ sub: [{ s: "close" }, { value: 5 }] });
  expect(t.convertOperand(rsi, "arith", cat7)).toEqual({ sub: [rsi, { value: 0 }] });
  expect(t.convertOperand(rsi, "pct", cat7)).toEqual({ pct: [rsi, { ref: rsi, bars: 1 }] });
  expect(t.convertOperand(rsi, "bars_since", cat7)).toEqual({ bars_since: t.DEFAULT_STATE_COND });
  expect(t.convertOperand(rsi, "value_when", cat7)).toEqual({ value_when: t.DEFAULT_STATE_COND, of: rsi, nth: 1 });
  expect(t.convertOperand(rsi, "since", cat7)).toEqual({ highest_since: t.DEFAULT_STATE_COND, of: rsi });
  expect(t.convertOperand(rsi, "count", cat7)).toEqual({ count: t.DEFAULT_STATE_COND, bars: 20 });
  expect(t.convertOperand(rsi, "compress", cat7)).toEqual({ compress: rsi, tf: "W", mode: "last" });
  expect(t.convertOperand({ s: "ema", p: { length: 9 }, tf: "W", offset: 1 }, "price", cat7)).toEqual({ s: "close", tf: "W", offset: 1 });
  expect(t.convertOperand(rsi, "calendar", cat7)).toEqual({ s: "session", out: "tdom" });
  expect(t.convertOperand(rsi, "number", cat7)).toEqual({ value: 0 });
  expect(t.innerOperand({ sub: [rsi, { value: 1 }] })).toEqual(rsi);
  expect(t.innerOperand({ value_when: C, of: rsi })).toEqual(rsi);
  expect(t.innerOperand({ bars_since: C })).toBeUndefined();
});

it("tree transforms keep unknown keys", () => {
  const tagged = { ...C, tag: "x" } as Cond;
  expect(t.withOp(tagged, "between")).toEqual({ lhs: C.lhs, op: "between", lo: C.rhs, hi: { value: 100 }, tag: "x" });
  expect(t.withOp({ lhs: { s: "rsi" }, op: "between", lo: { value: 30 }, hi: { value: 70 } } as RuleNode, "rising")).toEqual({ lhs: { s: "rsi" }, op: "rising", bars: 3 });
  expect(t.withOp({ lhs: { s: "rsi" }, op: "between", lo: { value: 30 }, hi: { value: 70 } } as RuleNode, "outside")).toMatchObject({ lo: { value: 30 }, hi: { value: 70 } });
  expect(t.withOp({ lhs: { s: "close" }, op: "rising", bars: 3 } as RuleNode, ">")).toEqual({ lhs: { s: "close" }, op: ">", rhs: { value: 0 } });
  expect(t.withOp({ lhs: { s: "rsi" }, op: "<", rhs: { value: 30 } }, "between")).toMatchObject({ lo: { value: 30 }, hi: { value: 40 } });
  expect(t.wrap(C, "not")).toEqual({ not: C });
  expect(t.unwrap({ all: [C] })).toEqual(C);
  expect(t.unwrap({ all: [C, C] })).toBeUndefined();
  expect(t.unwrap({ at_least: 2, of: 3, when: C })).toEqual(C);
  expect(t.move(["a", "b", "c"], 0, 1)).toEqual(["b", "a", "c"]);
  expect(t.move(["a", "b", "c"], 2, 1)).toEqual(["a", "b", "c"]);
});

it("detects session-date readers, higher timeframes and describes nodes", () => {
  expect(t.readsSessionDates(spec({ exit: { any: [{ lhs: { s: "sessions_left", out: "m" }, op: "<", rhs: { value: 2 } }] } }))).toBe(true);
  expect(t.readsSessionDates(spec({}))).toBe(false);
  expect(t.usesHigherTimeframes(spec({ entry: { all: [{ lhs: { compress: { s: "close" }, tf: "Q" }, op: ">", rhs: { value: 1 } }] } }))).toBe(true);
  expect(t.firstAllowed({ s: "close", tf: "W" })).toBe(false);
  expect(t.firstAllowed({ s: "close", tf: "W", offset: 1 })).toBe(true);
  expect(t.firstAllowed({ s: "open", tf: "W" })).toBe(true);
  expect(t.describeNode({ any: [C, { lhs: { ref: { s: "close" }, bars: 1 }, op: "<", rhs: { value: 5 } }] }, cat))
    .toBe("Any of 2: close is above Exponential Moving Average(50); close · 1 bar ago is below 5");
  expect(t.describeNode({ at_least: 2, of: 3, when: C }, cat)).toBe("At least 2 of 3 bars: close is above Exponential Moving Average(50)");
  expect(t.describeOperand({ avg: { s: "close" }, bars: 10 }, cat)).toBe("average(close, 10)");
});

it("preserves unrecognized metadata when changing plain kinds or replacing formulas", () => {
  const tagged: Operand = { s: "rsi", p: { length: 2 }, tf: "W", future_setting: { enabled: true } };
  const before = structuredClone(tagged);
  expect(t.convertOperand(tagged, "price", cat7)).toEqual({ s: "close", tf: "W", future_setting: { enabled: true } });
  expect(t.convertOperand({ ref: { s: "close" }, bars: 2, future_setting: "kept" }, "window", cat7))
    .toEqual({ avg: { s: "close" }, bars: 10, future_setting: "kept" });
  expect(tagged).toEqual(before);
});

it("walks every expression and group, including every short-side and order-level location", () => {
  const future: Operand = { s: "sessions_left", out: "m" };
  const watched: Cond = { lhs: future, op: "<", rhs: { value: 5 } };
  for (const key of t.EXPR_KEYS) {
    const expression: Operand = (t.ARITH as readonly string[]).includes(key) || key === "pct" ? { [key]: [future, { value: 1 }] }
      : key === "bars_since" || key === "count" ? { [key]: watched, ...(key === "count" ? { bars: 5 } : {}) }
      : key === "value_when" || key.endsWith("_since") ? { [key]: C, of: future, ...(key === "value_when" ? { nth: 1 } : {}) }
      : key === "compress" ? { compress: future, tf: "Q", mode: "high" }
      : { [key]: future, ...(key === "abs" ? {} : { bars: 1 }) };
    expect(t.readsSessionDates(spec({ entry: { all: [{ lhs: expression, op: ">", rhs: { value: 0 } }] } })), key).toBe(true);
  }
  const nodes: RuleNode[] = [{ all: [watched] }, { any: [watched] }, { not: watched }, { at_least: 1, of: 2, when: watched }, { then: [C, watched], within: 2 },
    { lhs: { s: "close" }, op: "between", lo: future, hi: { value: 10 } }, { lhs: future, op: "rising", bars: 2 }];
  for (const node of nodes) expect(t.readsSessionDates(spec({ entry: { all: [node] } })), t.nodeKind(node)).toBe(true);
  for (const path of ["entry.all[0].lhs", "exit.any[0].lhs", "short_side.entry.all[0].lhs", "short_side.exit.any[0].lhs", "entry.limit.at", "entry.stop.at", "short_side.entry.limit.at", "short_side.entry.stop.at"]) {
    const s = spec({ direction: "both", short_side: { entry: { all: [C] }, exit: { any: [C] } } });
    const keys = path.replaceAll(/\[(\d+)\]/g, ".$1").split(".");
    let parent = s as unknown as Record<string, unknown>;
    keys.slice(0, -1).forEach((key) => parent = (parent[key] ??= {}) as Record<string, unknown>);
    parent[keys.at(-1)!] = future;
    expect(t.readsSessionDates(s), path).toBe(true);
    expect(t.treeProblems(s, cat7).some((p) => p.includes("Bars held")), path).toBe(false);
  }
});

it("visits order levels before the short side, matching rules.rs, without mutating the spec", () => {
  const s = spec({ direction: "both", entry: { all: [C], limit: { at: { value: 17 } }, stop: { at: { value: 18 } } },
    short_side: { entry: { all: [{ ...C, rhs: { value: 19 } }], limit: { at: { value: 20 } }, stop: { at: { value: 21 } } }, exit: { any: [] } } });
  const before = structuredClone(s), paths: string[] = [];
  t.walkSpec(s, (x) => { if (x.t === "knob") paths.push(x.path); });
  expect(paths).toEqual(["exit.any[0].rhs.value", "entry.limit.at.value", "entry.stop.at.value", "short_side.entry.all[0].rhs.value", "short_side.entry.limit.at.value", "short_side.entry.stop.at.value"]);
  expect(s).toEqual(before);
  const first = t.newNode("cond") as Cond, second = t.newNode("cond") as Cond;
  (first.lhs as { p: { length: number } }).p.length = 7;
  expect((second.lhs as { p: { length: number } }).p.length).toBe(14);
  const list = [C, second];
  expect(t.move(list, 0, -1)).toBe(list);
  expect(t.move(list, 0, 1)).toEqual([second, C]);
  expect(list).toEqual([C, second]);
});

it("pins the engine comparison, node, timing and JSON-depth boundaries", () => {
  const comparisons = Array.from({ length: 39 }, () => ({ not: { not: C } }));
  const atLimit = spec({ entry: { all: [{ not: { any: comparisons } }] } });
  expect(t.budget(atLimit)).toMatchObject({ leaves: 40, nodes: 120 });
  expect(t.treeProblems(atLimit, cat7).some((p) => p.startsWith("More than"))).toBe(false);
  const over = spec({ entry: { all: [{ not: { not: { any: comparisons } } }] } });
  expect(t.treeProblems(over, cat7)).toContain("More than 120 comparisons, groups and formulas in the rules; the limit is 120.");
  expect(t.budget(spec({ entry: { all: [{ lhs: { compress: { s: "close" }, tf: "Q" }, op: ">", rhs: { value: 1 } }] } }))).toMatchObject({ leaves: 2, nodes: 3 });
  let empty: unknown = {};
  for (let i = 1; i < 16; i++) empty = [empty];
  expect(t.jsonDepth(empty)).toBe(16);
  let scalar: unknown = 0;
  for (let i = 0; i < 16; i++) scalar = [scalar];
  expect(t.jsonDepth(scalar)).toBe(17);
  for (const tf of ["D", "W", "M", "Q", "Y", "2D", "63D"]) expect(t.tfOk(tf, cat7), tf).toBe(true);
  for (const tf of ["1D", "64D", "-2D", "2.5D", "week"]) expect(t.tfOk(tf, cat7), tf).toBe(false);
  expect(t.timingProblem({ s: "close", tf: "Q", offset: -1 }, cat7)).toContain("offset");
  expect(t.timingProblem({ compress: { s: "close" }, tf: "D" }, cat7, true)).toContain("aggregate needs");
  expect(t.timingProblem({ compress: { s: "close" }, tf: "5D", mode: "high", expand: "first" }, cat7, true)).toBe(EXPAND_FIRST_REFUSED);
  expect(t.timingProblem({ compress: { s: "close" }, tf: "5D", mode: "high", offset: 1, expand: "first" }, cat7, true)).toBeUndefined();
  expect(t.treeProblems(spec({ entry: { all: [{ lhs: { value_when: C, of: { s: "close" }, nth: 21 }, op: ">", rhs: { value: 0 } }] } }), cat7))
    .toContain("Entry 1 › left › occurrence must be a whole number from 1 to 20.");
});

it("since-entry values: kind, defaults, description, optimizable factor and top-level-only check", () => {
  const trail: Operand = { since_entry: "highest", of: { s: "rsi", p: { length: 14 } }, add: -10 };
  expect(t.operandKind(trail, cat7)).toBe("since_entry");
  expect(t.operandKind({ since_entry: "median" }, cat7)).toBe("unknown");
  expect(t.describeOperand(trail, cat7)).toMatch(/^highest .*since entry − 10$/);
  expect(t.describeOperand({ since_entry: "entry_price", mul: 0.95 }, cat7)).toBe("entry price × 0.95");
  expect(t.sinceEntry("count", { since_entry: "highest", of: { s: "close" }, mul: 2 })).toEqual({ mul: 2, since_entry: "count", when: { lhs: { s: "close" }, op: ">", rhs: { s: "open" } } });
  expect(t.sinceEntry("entry_price", { since_entry: "count", when: C, add: 1 })).toEqual({ add: 1, since_entry: "entry_price" });
  expect(t.convertOperand({ s: "close" }, "since_entry", cat7)).toEqual({ since_entry: "highest", of: { s: "close" } });
  const top = spec({ exit: { any: [{ lhs: { s: "rsi", p: { length: 14 } }, op: "<", rhs: trail }] } });
  const knobs: string[] = [];
  t.walkSpec(top, (x) => { if (x.t === "knob") knobs.push(x.path); });
  expect(knobs).toContain("exit.any[0].rhs.add");
  expect(t.treeProblems(top, cat7)).toEqual([]);
  const nested = spec({ exit: { any: [{ any: [{ lhs: { s: "close" }, op: "<", rhs: { since_entry: "entry_price", mul: 0 } }] }] } });
  const problems = t.treeProblems(nested, cat7).join(" ");
  expect(problems).toContain("Since-entry values work only in a top-level exit condition");
  expect(problems).toContain("the factor must not be 0");
});

it("ready-made exits are valid top-level since-entry conditions", () => {
  for (const p of t.EXIT_PRESETS) {
    for (const short of [false, true]) {
      const s = spec({ ...(short ? { direction: "short" } : {}), exit: { any: [p.node(short)] } });
      expect(t.treeProblems(s, cat7), p.id).toEqual([]);
      expect(t.describeNode(p.node(short), cat7)).toMatch(/since entry|entry price/);
    }
  }
  expect(t.EXIT_PRESETS.map((p) => p.id)).toEqual(["green", "upclose", "red", "downclose", "rsi_trail", "close_drop", "close_stop", "close_target"]);
});

it("switched-off rules: allowed places, a group or entry list never all off, described as off", () => {
  const C: Cond = { lhs: { s: "close" }, op: ">", rhs: { value: 1 } };
  const off = { ...C, off: true } as RuleNode;
  expect(t.treeProblems(spec({ entry: { all: [C, off] } }), cat7)).toEqual([]);
  expect(t.treeProblems(spec({ entry: { all: [{ any: [C, off] }] } }), cat7)).toEqual([]);
  expect(t.treeProblems(spec({ entry: { all: [{ not: off }] } }), cat7).join(" ")).toContain("only a top-level rule or a condition of an all/any group can be switched off");
  expect(t.treeProblems(spec({ entry: { all: [{ all: [off] }] } }), cat7).join(" ")).toContain("every condition of this group is switched off");
  expect(t.treeProblems(spec({ entry: { all: [off] } }), cat7).join(" ")).toContain("Entry: every rule is switched off");
  expect(t.describeNode(off, cat7)).toMatch(/^\(off\) close/);
});

it("review fixes: since-entry factors are not formulas; position values convert from the close; presets mirror for shorts", () => {
  const trail: Operand = { since_entry: "highest", of: { s: "close" }, mul: 0.95 };
  expect(t.exprKey(trail)).toBeUndefined();
  const before = t.budget(spec({ exit: { any: [{ lhs: { s: "close" }, op: "<", rhs: { since_entry: "highest", of: { s: "close" } } }] } })).nodes;
  expect(t.budget(spec({ exit: { any: [{ lhs: { s: "close" }, op: "<", rhs: trail }] } })).nodes).toBe(before);
  expect(t.convertOperand({ since_entry: "count", when: C }, "arith", cat7)).toEqual({ sub: [{ s: "close" }, { value: 0 }] });
  expect(t.convertOperand({ s: "bars_held" }, "abs", cat7)).toEqual({ abs: { s: "close" } });
  const stop = t.EXIT_PRESETS.find((p) => p.id === "close_stop")!;
  expect(stop.node(true)).toEqual({ lhs: { s: "close" }, op: ">", rhs: { since_entry: "entry_price", mul: 1.05 } });
  expect(stop.label(true)).toContain("above the entry price");
  expect(t.EXIT_PRESETS.find((p) => p.id === "rsi_trail")!.node(true)).toEqual({ lhs: { s: "rsi", p: { length: 14 } }, op: ">", rhs: { since_entry: "lowest", of: { s: "rsi", p: { length: 14 } }, add: 10 } });
});
