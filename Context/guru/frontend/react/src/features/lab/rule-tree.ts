/** Phase 7 rule tree (rules.rs grammar): kinds, value-based edits, the walker that follows the engine's optimize paths,
 *  the engine's comparison/node budget, location labels and UX pre-checks. The engine stays the authority. */
import { COPY, EXPAND_FIRST_REFUSED, OPERAND_KIND_LABELS } from "./labels";
import { candleCountNode, getAt, isCond, isSeries, isValue, OPERATOR_LABELS, operandLabel } from "./spec-model";
import { isFormulaOperand } from "./formula";
import type { Catalogue, Cond, Operand, RuleNode, Spec } from "./types";

export const ARITH = ["add", "sub", "mul", "div", "min", "max"] as const;
export const WINDOWS = ["sum", "avg", "highest", "lowest"] as const;
/** rules.rs EXPRESSIONS, same order. */
export const EXPR_KEYS = [...ARITH, "pct", "abs", "ref", ...WINDOWS, "change", "bars_since", "value_when", "highest_since", "lowest_since", "count", "compress"] as const;
export type ExprKey = (typeof EXPR_KEYS)[number];
export const MAX_WHOLE = 1000, MAX_NTH = 20, MAX_STEPS = 4, MAX_DEPTH = 16;
const RANGE_OPS = ["between", "outside"], TREND_OPS = ["rising", "falling"];
type Obj = Record<string, unknown>;
const isObj = (v: unknown): v is Obj => typeof v === "object" && v !== null && !Array.isArray(v);

export type NodeKind = "cmp" | "range" | "trend" | "all" | "any" | "not" | "at_least" | "then" | "setup" | "unknown";
export function nodeKind(n: unknown): NodeKind {
  if (!isObj(n)) return "unknown";
  if ("lhs" in n) return RANGE_OPS.includes(n.op as string) ? "range" : TREND_OPS.includes(n.op as string) ? "trend" : isCond(n) ? "cmp" : "unknown";
  if (Array.isArray(n.all)) return "all";
  if (Array.isArray(n.any)) return "any";
  if ("not" in n) return "not";
  if ("at_least" in n) return "at_least";
  if ("setup" in n && "trigger" in n) return "setup";
  return Array.isArray(n.then) ? "then" : "unknown";
}
export const isLeaf = (k: NodeKind) => k === "cmp" || k === "range" || k === "trend";
/** The first expression key, as rules.rs `expr()` finds it. */
export const exprKey = (o: unknown): ExprKey | undefined => (isObj(o) && !("since_entry" in o) ? (Object.keys(o).find((k) => (EXPR_KEYS as readonly string[]).includes(k)) as ExprKey | undefined) : undefined);

export type OperandKind = "number" | "price" | "indicator" | "calendar" | "bars_held" | "since_entry" | "arith" | "pct" | "abs" | "ref" | "window" | "change"
  | "compress" | "bars_since" | "value_when" | "since" | "count" | "formula" | "unknown";
export const CALENDAR_IDS = ["calendar", "session", "sessions_left", "period_end"];
/** rules.rs SINCE_KINDS: a value of the open position, top-level exit comparisons only (like bars_held). */
export const SINCE_ENTRY_KINDS = ["highest", "lowest", "count", "entry_price"] as const;
export type SinceEntryKind = (typeof SINCE_ENTRY_KINDS)[number];
export const isSinceEntry = (o: unknown): boolean => isObj(o) && "since_entry" in o;
/** The default fields of a since-entry value of kind `k`, keeping an existing inner value or condition where it fits. */
export function sinceEntry(k: SinceEntryKind, from: Obj = {}): Obj {
  const keep = Object.fromEntries(Object.entries(from).filter(([key]) => !["since_entry", "of", "when"].includes(key)));
  if (k === "highest" || k === "lowest") return { ...keep, since_entry: k, of: from.of ?? { s: "close" } };
  if (k === "count") return { ...keep, since_entry: k, when: from.when ?? { lhs: { s: "close" }, op: ">", rhs: { s: "open" } } };
  return { ...keep, since_entry: k };
}
const kindOfKey = (k: ExprKey): OperandKind =>
  (ARITH as readonly string[]).includes(k) ? "arith" : (WINDOWS as readonly string[]).includes(k) ? "window" : k === "highest_since" || k === "lowest_since" ? "since" : (k as OperandKind);
export function operandKind(o: Operand, cat: Catalogue): OperandKind {
  if (isValue(o)) return "number";
  if (isSinceEntry(o)) return (SINCE_ENTRY_KINDS as readonly unknown[]).includes((o as Obj).since_entry) ? "since_entry" : "unknown";
  if (isFormulaOperand(o)) return "formula";
  const k = exprKey(o);
  if (k) return kindOfKey(k);
  if (!isSeries(o)) return "unknown";
  if (o.s === "bars_held") return "bars_held";
  if (cat.price_sources.includes(o.s)) return "price";
  if (CALENDAR_IDS.includes(o.s)) return "calendar";
  return cat.indicators.some((i) => i.id === o.s) ? "indicator" : "unknown";
}

/** + Add condition: an indicator against a threshold, the usual first rule (every part can be changed in the card). */
export const DEFAULT_COND: Cond = { lhs: { s: "rsi", p: { length: 14 } }, op: "crosses_above", rhs: { value: 30 } };
export const DEFAULT_STATE_COND: Cond = { lhs: { s: "close" }, op: "crosses_above", rhs: { s: "ema", p: { length: 50 } } };
/** A new Indicator filter row: a trend state that holds over days (a crossing lasts one candle), close above its 200-day EMA. */
export const newFilter = (): RuleNode => ({ lhs: { s: "close" }, op: ">", rhs: { s: "ema", p: { length: 200 } }, filter: true } as RuleNode);
export function newNode(kind: "cond" | "all" | "any" | "not" | "at_least" | "then"): RuleNode {
  const c = structuredClone(DEFAULT_COND);
  switch (kind) {
    case "cond": return c;
    case "all": return { all: [c] };
    case "any": return { any: [c] };
    case "not": return { not: c };
    case "at_least": return { at_least: 2, of: 3, when: { lhs: { s: "close" }, op: ">", rhs: { ref: { s: "close" }, bars: 1 } } };
    case "then": return { then: [{ lhs: { s: "close" }, op: "crosses_above", rhs: { s: "ema", p: { length: 20 } } }, c], within: 5 };
  }
}
export function withOp(n: RuleNode, op: string): RuleNode {
  const x = n as Obj;
  const rest = Object.fromEntries(Object.entries(x).filter(([k]) => !["lhs", "op", "rhs", "lo", "hi", "bars"].includes(k)));
  const lhs = (x.lhs ?? { s: "close" }) as Operand;
  if (RANGE_OPS.includes(op)) {
    const lo = (x.lo ?? x.rhs ?? { value: 0 }) as Operand;
    return { ...rest, lhs, op: op as "between" | "outside", lo, hi: (x.hi ?? { value: isValue(lo) ? lo.value + 10 : 100 }) as Operand };
  }
  if (TREND_OPS.includes(op)) return { ...rest, lhs, op: op as "rising" | "falling", bars: (x.bars ?? 3) as number };
  return { ...rest, lhs, op, rhs: (x.rhs ?? x.lo ?? { value: 0 }) as Operand };
}
export const wrap = (n: RuleNode, k: "all" | "any" | "not"): RuleNode => k === "not" ? { not: n } : { [k]: [n] } as RuleNode;
export function unwrap(n: RuleNode): RuleNode | undefined {
  const x = n as Obj, kind = nodeKind(n);
  if (kind === "not") return x.not as RuleNode;
  if (kind === "at_least") return x.when as RuleNode;
  const kids = x[kind];
  return (kind === "all" || kind === "any") && Array.isArray(kids) && kids.length === 1 ? kids[0] as RuleNode : undefined;
}
export function move<T>(list: T[], i: number, d: number): T[] {
  const j = i + d;
  if (!Number.isInteger(i) || !Number.isInteger(j) || i < 0 || j < 0 || i >= list.length || j >= list.length) return list;
  const next = [...list];
  [next[i], next[j]] = [next[j], next[i]];
  return next;
}
export function innerOperand(o: Operand): Operand | undefined {
  if (isSinceEntry(o)) return (o as Obj).of as Operand | undefined;
  const k = exprKey(o);
  if (!k) return undefined;
  const x = o as Obj;
  if ((ARITH as readonly string[]).includes(k) || k === "pct") return Array.isArray(x[k]) ? x[k][0] as Operand : undefined;
  if (["value_when", "highest_since", "lowest_since"].includes(k)) return x.of as Operand;
  return ["abs", "ref", ...WINDOWS, "change", "compress"].includes(k) ? x[k] as Operand : undefined;
}
export function convertOperand(o: Operand, to: OperandKind, cat: Catalogue): Operand {
  if (operandKind(o, cat) === to) return o;
  const timing = isSeries(o) ? Object.fromEntries(Object.entries(o).filter(([k]) => ["tf", "offset", "expand"].includes(k))) : {};
  const base = innerOperand(o) ?? o;
  // a position value (bars held, since entry) cannot sit inside a formula: start the formula from the close instead
  const series: Operand = isValue(base) || isSinceEntry(base) || (isSeries(base) && base.s === "bars_held") ? { s: "close" } : base;
  const convert = (): Operand => { switch (to) {
    case "number": return { value: 0 };
    case "price": return { s: "close", ...timing };
    case "indicator": return { s: cat.indicators.find((i) => i.computed && !CALENDAR_IDS.includes(i.id))?.id ?? "ema", ...timing };
    case "calendar": return cat.indicators.some((i) => i.id === "session" && i.computed) ? { s: "session", out: "tdom" } : { s: "calendar", out: "dow" };
    case "bars_held": return { s: "bars_held" };
    case "since_entry": return sinceEntry("highest", { of: series });
    case "arith": return { sub: isValue(base) ? [{ s: "close" }, base] : [series, { value: 0 }] };
    case "pct": return { pct: [series, { ref: series, bars: 1 }] };
    case "abs": return { abs: series };
    case "ref": return { ref: series, bars: 1 };
    case "window": return { avg: series, bars: 10 };
    case "change": return { change: series, bars: 1 };
    case "bars_since": return { bars_since: structuredClone(DEFAULT_STATE_COND) };
    case "value_when": return { value_when: structuredClone(DEFAULT_STATE_COND), of: series, nth: 1 };
    case "since": return { highest_since: structuredClone(DEFAULT_STATE_COND), of: series };
    case "count": return { count: structuredClone(DEFAULT_STATE_COND), bars: 20 };
    case "compress": return { compress: series, tf: "W", mode: "last" };
    default: return o;
  } };
  const known = ["s", "formula", "name", "p", "tf", "out", "offset", "expand", "value", "bars", "of", "nth", "mode", "since_entry", "when", "mul", "add", ...EXPR_KEYS];
  const extras = exprKey(o) || isSinceEntry(o) || ["number", "price", "indicator", "calendar", "bars_held", "since_entry"].includes(to)
    ? Object.fromEntries(Object.entries(o).filter(([k]) => !known.includes(k))) : {};
  return { ...extras, ...convert() };
}

export const LIST_PATHS = ["entry.all", "exit.any", "short_side.entry.all", "short_side.exit.any"] as const;
export type ListPath = (typeof LIST_PATHS)[number];
const LEVEL_ROOTS = ["entry.limit.at", "entry.stop.at", "short_side.entry.limit.at", "short_side.entry.stop.at"];
export type WalkItem =
  | { t: "node"; path: string; root: string; node: RuleNode; kind: NodeKind; top: boolean }
  | { t: "operand"; path: string; root: string; operand: Operand; held: boolean }
  | { t: "knob"; path: string; root: string; value: number; whole: boolean; min: number; max: number };
type Emit = (x: WalkItem) => void;

export function walkSpec(spec: Spec, f: Emit): void {
  for (const pre of ["", "short_side."]) {
    for (const suffix of ["entry.all", "exit.any"]) {
      const root = pre + suffix, list = getAt(spec, root);
      if (Array.isArray(list)) list.forEach((n, i) => walkNode(n as RuleNode, `${root}[${i}]`, root, true, f));
    }
    for (const root of LEVEL_ROOTS.filter((r) => r.startsWith("short_side.") === !!pre)) {
      const o = getAt(spec, root);
      if (o !== undefined) walkOperand(o as Operand, root, root, false, f);
    }
  }
  if(spec.portfolio?.score!==undefined)walkOperand(spec.portfolio.score,"portfolio.score","portfolio.score",false,f);
}
const knob = (f: Emit, path: string, root: string, v: unknown, min: number, max = MAX_WHOLE) =>
  f({ t: "knob", path, root, value: typeof v === "number" ? v : NaN, whole: true, min, max });
function walkNode(n: RuleNode, path: string, root: string, top: boolean, f: Emit) {
  const kind = nodeKind(n);
  f({ t: "node", path, root, node: n, kind, top });
  if (!isObj(n)) return;
  const x = n as unknown as Obj, kids = (k: string) => (Array.isArray(x[k]) ? (x[k] as RuleNode[]) : []);
  switch (kind) {
    case "cmp": { const held = top && root.endsWith("exit.any"); walkOperand(x.lhs as Operand, `${path}.lhs`, root, held, f); walkOperand(x.rhs as Operand, `${path}.rhs`, root, held, f); break; }
    case "range": for (const k of ["lhs", "lo", "hi"]) walkOperand(x[k] as Operand, `${path}.${k}`, root, false, f); break;
    case "trend": walkOperand(x.lhs as Operand, `${path}.lhs`, root, false, f); knob(f, `${path}.bars`, root, x.bars, 1); break;
    case "all": case "any": kids(kind).forEach((c, i) => walkNode(c, `${path}.${kind}[${i}]`, root, false, f)); break;
    case "not": walkNode(x.not as RuleNode, `${path}.not`, root, false, f); break;
    case "at_least": knob(f, `${path}.at_least`, root, x.at_least, 1); knob(f, `${path}.of`, root, x.of, 1); walkNode(x.when as RuleNode, `${path}.when`, root, false, f); break;
    case "then": kids("then").forEach((c, i) => walkNode(c, `${path}.then[${i}]`, root, false, f)); knob(f, `${path}.within`, root, x.within, 1); break;
    case "setup": for (const k of ["setup", "trigger", "cancel"]) if (x[k] !== undefined) walkNode(x[k] as RuleNode, `${path}.${k}`, root, false, f); break;
  }
}
function walkOperand(o: Operand | undefined, path: string, root: string, held: boolean, f: Emit) {
  if (o === undefined) return;
  f({ t: "operand", path, root, operand: o, held });
  if (isValue(o)) { f({ t: "knob", path: `${path}.value`, root, value: o.value, whole: false, min: -Infinity, max: Infinity }); return; }
  if (isSinceEntry(o)) {
    const s = o as Obj;
    if (s.of !== undefined) walkOperand(s.of as Operand, `${path}.of`, root, false, f);
    if (s.when !== undefined) walkNode(s.when as RuleNode, `${path}.when`, root, false, f);
    for (const key of ["mul", "add"]) if (s[key] !== undefined) f({ t: "knob", path: `${path}.${key}`, root, value: typeof s[key] === "number" ? s[key] as number : NaN, whole: false, min: -Infinity, max: Infinity });
    return;
  }
  const k = exprKey(o);
  if (!k) return;
  const x = o as Obj;
  const sub = (key: string) => walkOperand(x[key] as Operand, `${path}.${key}`, root, false, f);
  const cond = (key: string) => walkNode(x[key] as RuleNode, `${path}.${key}`, root, false, f);
  if ((ARITH as readonly string[]).includes(k) || k === "pct") {
    if (Array.isArray(x[k])) (x[k] as Operand[]).forEach((c, i) => walkOperand(c, `${path}.${k}[${i}]`, root, false, f));
    return;
  }
  switch (k) {
    case "abs": sub("abs"); break;
    case "ref": sub("ref"); knob(f, `${path}.bars`, root, x.bars, 0); break;
    case "sum": case "avg": case "highest": case "lowest": case "change": sub(k); knob(f, `${path}.bars`, root, x.bars, 1); break;
    case "bars_since": cond("bars_since"); break;
    case "count": cond("count"); knob(f, `${path}.bars`, root, x.bars, 1); break;
    case "value_when": cond("value_when"); sub("of"); knob(f, `${path}.nth`, root, x.nth ?? 1, 1, MAX_NTH); break;
    case "highest_since": case "lowest_since": cond(k); sub("of"); break;
    case "compress": sub("compress"); break;
  }
}

export interface Budget { leaves: number; nodes: number; top: { long: number; short: number } }
/** rules.rs Budget::take: every leaf (comparison, range, trend) and every group and expression counts once, over both sides and `at` levels. */
export function budget(spec: Spec): Budget {
  let leaves = 0, nodes = spec.portfolio?.score!==undefined ? 1 : 0;
  walkSpec(spec, (x) => {
    if (x.t === "node") { nodes++; if (isLeaf(x.kind)) leaves++; }
    else if (x.t === "operand" && exprKey(x.operand)) nodes++;
  });
  const side = (e?: { all: unknown[] }, x?: { any: unknown[] }) => (e?.all.length ?? 0) + (x?.any.length ?? 0);
  return { leaves, nodes, top: { long: side(spec.entry, spec.exit), short: side(spec.short_side?.entry, spec.short_side?.exit) } };
}
export const limits = (cat: Catalogue) => ({ leaves: cat.max_leaves ?? 40, nodes: cat.max_nodes ?? 120, top: cat.max_conditions });

const SEGMENT: Record<string, string> = {
  lhs: "left", rhs: "right", lo: "low", hi: "high", all: "all", any: "any", not: "not", when: "when", then: "step", within: "within",
  at_least: "at least", of: "of", bars: "bars", nth: "occurrence", value: "number", p: "", ref: "bars ago", abs: "absolute", pct: "% difference",
  add: "plus", sub: "minus", mul: "times", div: "divided by", min: "min", max: "max", sum: "sum", avg: "average", highest: "highest",
  lowest: "lowest", change: "change", bars_since: "bars since", value_when: "value when", highest_since: "highest since",
  lowest_since: "lowest since", count: "count", compress: "aggregate",
};
export function pathLabel(path: string, both: boolean): string {
  const match = /^(short_side\.)?(entry\.all|exit\.any|entry\.limit\.at|entry\.stop\.at|portfolio\.score)(?:\[(\d+)\])?(.*)$/.exec(path);
  if (!match) return path;
  const [, short, root, index, rest] = match;
  const word = ({ "entry.all": "entry", "exit.any": "exit", "entry.limit.at": "limit level", "entry.stop.at": "stop level", "portfolio.score":"rank score" } as Record<string, string>)[root];
  const title = short ? `Short ${word}` : both ? `Long ${word}` : word[0].toUpperCase() + word.slice(1);
  const parts = [title + (index !== undefined ? ` ${Number(index) + 1}` : "")];
  for (const m of rest.matchAll(/\.([^.[\]]+)(?:\[(\d+)\])?/g)) {
    const label = SEGMENT[m[1]] ?? m[1].replaceAll("_", " ");
    if (label) parts.push(label + (m[2] !== undefined ? ` ${Number(m[2]) + 1}` : ""));
  }
  return parts.join(" › ");
}
export const tfOk = (tf: string, cat: Catalogue) => tf === "D" || cat.timeframes.includes(tf) ||
  (/^\d+D$/.test(tf) && Number(tf.slice(0, -1)) >= 2 && Number(tf.slice(0, -1)) <= (cat.custom_days?.[1] ?? 63));
export function firstAllowed(o: Operand, compress = false): boolean {
  const x = o as Obj;
  return Number(x.offset ?? 0) >= 1 || (compress ? (x.mode ?? "last") === "first" : x.s === "open");
}
export function timingProblem(o: Operand, cat: Catalogue, compress = false): string | undefined {
  const x = o as Obj, tf = typeof x.tf === "string" ? x.tf : "D";
  if ((x.tf !== undefined && typeof x.tf !== "string") || !tfOk(tf, cat) || (compress && tf === "D"))
    return compress ? "the aggregate needs a timeframe of W, M, Q, Y or N sessions (2 to 63)." : "timeframe must be D, W, M, Q, Y or N sessions (2 to 63).";
  if (x.offset !== undefined && (!Number.isInteger(x.offset) || Number(x.offset) < 0 || Number(x.offset) > MAX_WHOLE))
    return `offset must be a whole number from 0 to ${MAX_WHOLE} (past bars only).`;
  if (x.expand !== undefined && !["last", "first", "point"].includes(x.expand as string)) return "“Use the period value” must be last, first or point.";
  if (x.expand !== undefined && x.expand !== "last" && tf === "D") return "“Use the period value” applies to W, M, Q, Y and N-session values only.";
  if (x.expand === "first" && !firstAllowed(o, compress)) return EXPAND_FIRST_REFUSED;
  return undefined;
}
export function jsonDepth(v: unknown): number {
  const children = Array.isArray(v) ? v : isObj(v) ? Object.values(v) : undefined;
  return children ? 1 + children.reduce<number>((max, child) => Math.max(max, jsonDepth(child)), 0) : 1;
}
export function treeProblems(spec: Spec, cat: Catalogue): string[] {
  const out: string[] = [], b = budget(spec), lim = limits(cat), both = spec.direction === "both";
  if (b.leaves > lim.leaves) out.push(`More than ${lim.leaves} comparisons in the rules; the limit is ${lim.leaves}.`);
  if (b.nodes > lim.nodes) out.push(`More than ${lim.nodes} comparisons, groups and formulas in the rules; the limit is ${lim.nodes}.`);
  for (const side of ["long", "short"] as const) if (b.top[side] > lim.top)
    out.push(`${side === "short" ? "Short side" : both ? "Long side" : "Rules"}: use at most ${lim.top} top-level entry and exit conditions in total.`);
  if (jsonDepth(spec) > MAX_DEPTH) out.push(COPY.tooDeep);
  for (const pre of ["", "short_side."] as const) {
    const entry = getAt(spec, `${pre}entry.all`);
    if (Array.isArray(entry) && entry.length && entry.every((c) => (c as Obj).off === true)) out.push(`${pre ? "Short entry" : both ? "Long entry" : "Entry"}: every rule is switched off; switch one on.`);
  }
  walkSpec(spec, (item) => {
    const label = pathLabel(item.path, both), add = (message: string) => out.push(`${label}: ${message}`);
    if (item.t === "knob") {
      if (item.whole && (!Number.isInteger(item.value) || item.value < item.min || item.value > item.max))
        out.push(`${label} must be a whole number from ${item.min} to ${item.max}.`);
      else if (!item.whole && !Number.isFinite(item.value)) out.push(`${label} must be a finite number.`);
      return;
    }
    if (item.t === "node") {
      const x = item.node as Obj;
      if (x.off !== undefined && (x.off !== true || !(item.top || /\.(all|any)\[\d+\]$/.test(item.path)))) add("only a top-level rule or a condition of an all/any group can be switched off.");
      if ((item.kind === "all" || item.kind === "any") && (x[item.kind] as RuleNode[]).length && (x[item.kind] as RuleNode[]).every((c) => (c as Obj).off === true)) add("every condition of this group is switched off; switch one on or remove the group.");
      if (item.kind === "cmp" && isValue(x.lhs) && isValue(x.rhs)) add("a condition cannot compare two numbers; one side must be a series.");
      if (item.kind === "range" && isValue(x.lo) && isValue(x.hi) && x.lo.value > x.hi.value) add("the low bound is above the high bound.");
      if ((item.kind === "all" || item.kind === "any") && !(x[item.kind] as RuleNode[]).length) add("add at least one condition to this group.");
      if (item.kind === "at_least" && Number(x.at_least) > Number(x.of)) add(`"at least" (${x.at_least}) cannot exceed the number of bars (${x.of}).`);
      if (item.kind === "then") {
        const count = (x.then as RuleNode[]).length;
        if (count < 2 || count > MAX_STEPS) add(`a sequence needs 2 to ${MAX_STEPS} steps.`);
        if (Number(x.within) < count - 1) add(`within (${x.within}) is shorter than the ${count} steps need (${count - 1}).`);
      }
      return;
    }
    const o = item.operand, k = exprKey(o), x = o as Obj;
    if (isSinceEntry(o)) {
      if (!item.held) add(COPY.sinceTopOnly);
      const kind = x.since_entry;
      if (!(SINCE_ENTRY_KINDS as readonly unknown[]).includes(kind)) add(`since entry must be one of ${SINCE_ENTRY_KINDS.join(", ")}.`);
      if ((kind === "highest" || kind === "lowest") && (x.of === undefined || isValue(x.of))) add("the highest or lowest since entry needs a series, not a number.");
      if (kind === "count" && x.when === undefined) add("the count since entry needs a condition.");
      if (x.mul === 0) add("the factor must not be 0.");
      return;
    }
    if (isFormulaOperand(o)) {
      const issue = timingProblem(o, cat);
      if (issue) add(issue);
      if (!o.formula.trim()) add("the custom indicator has no formula.");
    }
    if (isSeries(o)) {
      if (o.s === "bars_held") {
        if (!item.held) add(COPY.heldTopOnly);
        else if (["p", "tf", "out", "offset", "expand"].some((key) => key in o)) add("Bars held takes no settings, timeframe, output, offset or expansion.");
        return;
      }
      const indicator = cat.indicators.find((i) => i.id === o.s);
      if (indicator && !indicator.computed) add(`${indicator.name} is listed only and cannot be used in rules. ${indicator.note}`.trim());
      const issue = timingProblem(o, cat);
      if (issue) add(issue);
    }
    if (!k) return;
    const pair = x[k];
    if ((ARITH as readonly string[]).includes(k) || k === "pct") {
      if (!Array.isArray(pair) || pair.length !== 2) add("a formula needs exactly two operands.");
      else if (pair.every(isValue)) add("a formula cannot combine two numbers; use at least one series.");
    } else {
      const inner = innerOperand(o);
      if (inner !== undefined && isValue(inner)) add(`${OPERAND_KIND_LABELS[kindOfKey(k)]} needs a series, not a number.`);
    }
    if (k === "compress") {
      const issue = timingProblem(o, cat, true);
      if (issue) add(issue);
      if (x.mode !== undefined && !["last", "first", "high", "low", "sum"].includes(x.mode as string)) add("aggregate mode must be last, first, high, low or sum.");
    }
  });
  return out;
}
export function readsSessionDates(spec: Spec): boolean {
  let found = false;
  walkSpec(spec, (x) => { if (x.t === "operand" && isSeries(x.operand) && x.operand.s === "sessions_left") found = true; });
  return found;
}
export function usesHigherTimeframes(spec: Spec): boolean {
  let found = false;
  walkSpec(spec, (x) => {
    if (x.t === "operand" && ((isSeries(x.operand) && (x.operand.tf ?? "D") !== "D") || exprKey(x.operand) === "compress")) found = true;
  });
  return found;
}
const ARITH_SYMBOL: Record<string, string> = { add: "+", sub: "−", mul: "×", div: "÷", min: "min", max: "max" };
const num = (v: number) => String(Number(v.toFixed(6)));
export function describeOperand(o: Operand, cat: Catalogue): string {
  if (isSinceEntry(o)) {
    const s = o as Obj;
    const base = s.since_entry === "highest" || s.since_entry === "lowest" ? `${s.since_entry} ${describeOperand(s.of as Operand, cat)} since entry`
      : s.since_entry === "count" ? `bars since entry where (${describeNode(s.when as RuleNode, cat)})` : s.since_entry === "entry_price" ? "entry price" : "kept as is";
    const mul = typeof s.mul === "number" && s.mul !== 1 ? ` × ${num(s.mul)}` : "", add = typeof s.add === "number" && s.add !== 0 ? ` ${s.add < 0 ? "−" : "+"} ${num(Math.abs(s.add))}` : "";
    return base + mul + add;
  }
  if (isValue(o) || isSeries(o) || isFormulaOperand(o)) return operandLabel(o, cat);
  const k = exprKey(o);
  if (!k) return "kept as is";
  const x = o as Obj, inner = (key: string) => describeOperand(x[key] as Operand, cat), cond = (key: string) => describeNode(x[key] as RuleNode, cat);
  if ((ARITH as readonly string[]).includes(k) || k === "pct") {
    if (!Array.isArray(x[k]) || x[k].length !== 2) return "kept as is";
    const [a, b] = (x[k] as Operand[]).map((v) => describeOperand(v, cat));
    return k === "pct" ? `% diff(${a}, ${b})` : `(${a} ${ARITH_SYMBOL[k]} ${b})`;
  }
  switch (k) {
    case "ref": return `${inner(k)} · ${x.bars} bar${x.bars === 1 ? "" : "s"} ago`;
    case "sum": case "avg": case "highest": case "lowest": return `${k === "avg" ? "average" : k}(${inner(k)}, ${x.bars})`;
    case "abs": return `|${inner(k)}|`;
    case "change": return `change(${inner(k)}, ${x.bars})`;
    case "bars_since": return `bars since (${cond(k)})`;
    case "value_when": return `${inner("of")} when (${cond(k)})`;
    case "highest_since": case "lowest_since": return `${k === "highest_since" ? "highest" : "lowest"} of ${inner("of")} since (${cond(k)})`;
    case "count": return `count(${cond(k)}, ${x.bars})`;
    case "compress": return `${inner(k)} as ${x.tf} ${x.mode ?? "last"}`;
    default: return "kept as is";
  }
}
export function describeNode(n: RuleNode, cat: Catalogue): string {
  const x = n as Obj;
  if (x.off === true) return `(off) ${describeNode({ ...x, off: undefined } as unknown as RuleNode, cat)}`;
  const operand = (key: string) => describeOperand(x[key] as Operand, cat), kind = nodeKind(n);
  switch (kind) {
    case "cmp": return `${operand("lhs")} ${OPERATOR_LABELS[x.op as string] ?? x.op} ${operand("rhs")}`;
    case "range": return `${operand("lhs")} is ${x.op} ${operand("lo")} and ${operand("hi")}`;
    case "trend": return `${operand("lhs")} is ${x.op === "rising" ? "rising" : "falling"} for ${x.bars} bars`;
    case "all": case "any": { const kids = x[kind] as RuleNode[]; return `${kind === "all" ? "All" : "Any"} of ${kids.length}: ${kids.map((c) => describeNode(c, cat)).join("; ")}`; }
    case "not": return `not (${describeNode(x.not as RuleNode, cat)})`;
    case "at_least": return `At least ${x.at_least} of ${x.of} bars: ${describeNode(x.when as RuleNode, cat)}`;
    case "then": return `${(x.then as RuleNode[]).map((c) => describeNode(c, cat)).join(" then ")} within ${x.within} bars`;
    case "setup": return `after ${describeNode(x.setup as RuleNode, cat)}, ${describeNode(x.trigger as RuleNode, cat)}${x.cancel ? `; cancelled by ${describeNode(x.cancel as RuleNode, cat)}` : ""}${x.period_end ? ` (at ${x.period_end === "W" ? "week" : "month"} end)` : ""}`;
    default: return "kept as is";
  }
}

/** Ready-made since-entry exits (the AmiBroker AFL exits users ask for by name), added to an exit list as normal conditions. */
const RSI14 = { s: "rsi", p: { length: 14 } };
/** Each preset for a long position and its mirror for a short one (trails run from the lowest value, stops sit above). */
export const EXIT_PRESETS: { id: string; label(short: boolean): string; node(short: boolean): RuleNode }[] = [
  { id: "green", label: () => "Nth green candle since entry", node: () => candleCountNode("green", 3) },
  { id: "upclose", label: () => "Nth up-close since entry", node: () => candleCountNode("up", 3) },
  { id: "red", label: () => "Nth red candle since entry", node: () => candleCountNode("red", 3) },
  { id: "downclose", label: () => "Nth down-close since entry", node: () => candleCountNode("down", 3) },
  { id: "rsi_trail", label: (short) => short ? "RSI rises 10 above its lowest since entry" : "RSI falls 10 below its highest since entry",
    node: (short) => short ? { lhs: { ...RSI14 }, op: ">", rhs: { since_entry: "lowest", of: { ...RSI14 }, add: 10 } } : { lhs: { ...RSI14 }, op: "<", rhs: { since_entry: "highest", of: { ...RSI14 }, add: -10 } } },
  { id: "close_drop", label: (short) => short ? "Close 5 % above its lowest since entry" : "Close 5 % below its highest since entry",
    node: (short) => short ? { lhs: { s: "close" }, op: ">", rhs: { since_entry: "lowest", of: { s: "close" }, mul: 1.05 } } : { lhs: { s: "close" }, op: "<", rhs: { since_entry: "highest", of: { s: "close" }, mul: 0.95 } } },
  { id: "close_stop", label: (short) => `Close 5 % ${short ? "above" : "below"} the entry price (stop on close)`,
    node: (short) => ({ lhs: { s: "close" }, op: short ? ">" : "<", rhs: { since_entry: "entry_price", mul: short ? 1.05 : 0.95 } }) },
  { id: "close_target", label: (short) => `Close 10 % ${short ? "below" : "above"} the entry price (target on close)`,
    node: (short) => ({ lhs: { s: "close" }, op: short ? "<" : ">", rhs: { since_entry: "entry_price", mul: short ? 0.9 : 1.1 } }) },
];
