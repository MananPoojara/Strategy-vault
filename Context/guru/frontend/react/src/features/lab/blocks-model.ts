/** Block builder model for custom indicators (design §5): a versioned JSON tree, typed slots, formula generation (the engine only
 *  ever sees the text) and a parser of the same language (design §1) for "Convert to blocks". Pure: no React, no I/O.
 *  Locations are keys: "l2" = named result 3, "p0" = Line 1, then ".i" per argument ("p0.1.0"). */
import type { Indicator } from "./types";

export type SlotType = "series" | "number" | "whole" | "condition";
export type PriceName = "O" | "H" | "L" | "C" | "V" | "Avg" | "hl2" | "hlc3" | "ohlc4";
export type Slot = BlockNode | null;
export type BlockNode =
  | { k: "num"; v: number }
  | { k: "bool"; v: boolean }
  | { k: "price"; f: PriceName }
  | { k: "param"; label: string; def: number; min: number; max: number; step: number }
  | { k: "ref"; name: string }
  | { k: "fn"; f: FnId; a: Slot[] }
  | { k: "ago"; n: number; a: Slot[] }
  | { k: "ind"; id: string; out?: string; a: Slot[] };
export interface LetRow { name: string; expr: Slot }
export interface PlotRow { name: string; onPrice: boolean; expr: Slot }
export interface Blocks { v: 1; lets: LetRow[]; plots: PlotRow[] }

export const MAX_PLOTS = 8, MAX_PARAMS = 16, MAX_AGO = 2000;

interface Arg { label: string; type: SlotType; optional?: true }
interface FnDef {
  label: string; group: string; text: string; args: Arg[]; out: SlotType | "arg0"; desc: string;
  /** Binary operator precedence (or 1 … ^ 8); absent = function call. */
  infix?: number; prefix?: true;
  /** How the block reads on the canvas: words and argument indexes. */
  show?: (string | number)[];
}
const S = (label: string, optional?: true): Arg => ({ label, type: "series", ...(optional && { optional }) });
const W = (label: string, optional?: true): Arg => ({ label, type: "whole", ...(optional && { optional }) });
const N = (label: string, optional?: true): Arg => ({ label, type: "number", ...(optional && { optional }) });
const B = (label: string): Arg => ({ label, type: "condition" });
const bin = (label: string, text: string, infix: number, group: string, out: SlotType, desc: string, args: Arg[] = [S("a"), S("b")]): FnDef =>
  ({ label, text, infix, group, out, desc, args, show: [0, label, 1] });
const call = (label: string, text: string, group: string, args: Arg[], desc: string, out: SlotType | "arg0" = "series"): FnDef => ({ label, text, group, args, out, desc });
const win = (label: string, text: string, desc: string) => call(label, text, "Smoothing & windows", [S("of"), W("length")], desc);

export const FNS = {
  add: bin("+", "+", 5, "Math", "series", "add"), sub: bin("−", "-", 5, "Math", "series", "subtract"),
  mul: bin("×", "*", 6, "Math", "series", "multiply"), div: bin("÷", "/", 6, "Math", "series", "divide"),
  pow: bin("^", "^", 8, "Math", "series", "power"),
  neg: { label: "negate", text: "-", group: "Math", args: [S("x")], out: "series", prefix: true, desc: "minus x", show: ["−", 0] } as FnDef,
  abs: call("Abs", "Abs", "Math", [S("x")], "absolute value"), sqrt: call("Sqrt", "Sqrt", "Math", [S("x")], "square root"),
  log: call("Log", "Log", "Math", [S("x")], "natural logarithm"), exp: call("Exp", "Exp", "Math", [S("x")], "e to the power x"),
  max: call("Max", "Max", "Math", [S("a"), S("b")], "larger of two"), min: call("Min", "Min", "Math", [S("a"), S("b")], "smaller of two"),
  ma: win("MA", "MA", "simple moving average"), ema: win("EMA", "EMA", "exponential moving average"),
  wma: win("WMA", "WMA", "weighted moving average"), wilders: win("Wilders", "Wilders", "Wilder's smoothing (RMA)"),
  stdev: win("StDev", "StDev", "standard deviation"), sum: win("Sum", "Sum", "sum over the window"),
  hhv: win("Highest", "HHV", "highest value in the window"), llv: win("Lowest", "LLV", "lowest value in the window"),
  roc: win("ROC", "ROC", "rate of change, %"), change: win("Change", "Change", "difference from n bars ago (momentum)"),
  linreg: win("LinReg slope", "LinRegSlope", "slope of the linear regression"),
  ama: call("AMA", "AMA", "Smoothing & windows", [S("of"), N("alpha")], "adaptive moving average with smoothing factor alpha"),
  gt: bin(">", ">", 4, "Compare", "condition", "greater than"), lt: bin("<", "<", 4, "Compare", "condition", "less than"),
  ge: bin("≥", ">=", 4, "Compare", "condition", "greater than or equal"), le: bin("≤", "<=", 4, "Compare", "condition", "less than or equal"),
  eq: bin("=", "==", 4, "Compare", "condition", "equal"), ne: bin("≠", "!=", 4, "Compare", "condition", "not equal"),
  cross_above: { ...call("crosses above", "Cross", "Compare", [S("a"), S("b")], "a crosses above b on this bar", "condition"), show: [0, "crosses above", 1] },
  cross_below: { ...call("crosses below", "Cross", "Compare", [S("a"), S("b")], "a crosses below b on this bar", "condition"), show: [0, "crosses below", 1] },
  and: bin("and", "AND", 2, "Logic", "condition", "both true", [B("a"), B("b")]),
  or: bin("or", "OR", 1, "Logic", "condition", "either true", [B("a"), B("b")]),
  not: { label: "not", text: "NOT", group: "Logic", args: [B("condition")], out: "condition", prefix: true, desc: "true when false", show: ["not", 0] } as FnDef,
  if: { ...call("if … then … else", "IIf", "Logic", [B("if"), S("then"), S("else")], "pick a value by a condition"), show: ["if", 0, "then", 1, "else", 2] },
  barssince: call("Bars since", "BarsSince", "State", [B("condition")], "bars since the condition was last true"),
  valuewhen: call("Value when", "ValueWhen", "State", [B("when"), S("value"), W("nth", true)], "value on the bar the condition was last (nth) true"),
  nz: call("Nz", "Nz", "State", [S("value"), N("else", true)], "replace missing (warm-up) values", "arg0"),
} satisfies Record<string, FnDef>;
export type FnId = keyof typeof FNS;
export const fnDef = (f: FnId): FnDef => FNS[f];

/** Formula names (case-insensitive) → block function; aliases from design §1. */
const FN_NAMES: Record<string, FnId> = {
  abs: "abs", sqrt: "sqrt", log: "log", exp: "exp", max: "max", min: "min", ma: "ma", sma: "ma", ema: "ema", wma: "wma", wilders: "wilders", rma: "wilders",
  stdev: "stdev", sum: "sum", hhv: "hhv", highest: "hhv", llv: "llv", lowest: "llv", roc: "roc", change: "change", mom: "change",
  linregslope: "linreg", ama: "ama", iif: "if", cross: "cross_above", barssince: "barssince", valuewhen: "valuewhen", nz: "nz",
};
const PRICE_NAMES: Record<string, PriceName> = {
  o: "O", open: "O", h: "H", high: "H", l: "L", low: "L", c: "C", close: "C", v: "V", volume: "V", avg: "Avg", hl2: "hl2", hlc3: "hlc3", ohlc4: "ohlc4",
};
export const PRICES: { f: PriceName; label: string }[] = [
  { f: "O", label: "Open" }, { f: "H", label: "High" }, { f: "L", label: "Low" }, { f: "C", label: "Close" }, { f: "V", label: "Volume" },
  { f: "Avg", label: "Avg (H+L+C)/3" }, { f: "hl2", label: "hl2" }, { f: "hlc3", label: "hlc3" }, { f: "ohlc4", label: "ohlc4" },
];
const priceLabel = (f: PriceName) => PRICES.find((p) => p.f === f)!.label.split(" ")[0];
const KEYWORDS = ["and", "or", "not", "true", "false", "param", "plot", "ref"];
const IDENT = /^[A-Za-z_][A-Za-z0-9_]*$/;
/** Names a result may not take: the language's own words, prices, functions and (when given) catalogue ids. */
export function reservedName(name: string, indicators: Indicator[] = []): boolean {
  const n = name.toLowerCase();
  return KEYWORDS.includes(n) || n in PRICE_NAMES || n in FN_NAMES || indicators.some((i) => i.id.toLowerCase() === n);
}
/** Catalogue indicators a formula can call by id: computed, id is an identifier and not shadowed by a built-in function or price. */
export const formulaIndicators = (list: Indicator[]) =>
  list.filter((i) => i.computed && !i.formula && !i.needs.second_symbol && IDENT.test(i.id) && !reservedName(i.id));

// ---------- locations ----------
export interface Loc { row: "l" | "p"; i: number; path: number[] }
export const parseKey = (key: string): Loc => {
  const [head, ...rest] = key.split(".");
  return { row: head[0] as "l" | "p", i: Number(head.slice(1)), path: rest.map(Number) };
};
export const childKey = (key: string, i: number) => `${key}.${i}`;
const parentKey = (key: string) => key.slice(0, key.lastIndexOf("."));
const rowOf = (m: Blocks, l: Loc) => (l.row === "l" ? m.lets[l.i] : m.plots[l.i]);
export const childrenOf = (n: Slot): Slot[] => (n && "a" in n ? n.a : []);

export function getAt(m: Blocks, key: string): Slot {
  const l = parseKey(key);
  let n: Slot = rowOf(m, l)?.expr ?? null;
  for (const i of l.path) n = childrenOf(n)[i] ?? null;
  return n;
}
export function setAt(m: Blocks, key: string, node: Slot): Blocks {
  const l = parseKey(key);
  const put = (n: Slot, path: number[]): Slot => {
    if (!path.length) return node;
    if (!n || !("a" in n)) throw new Error(`No slot at ${key}`);
    const a = [...n.a];
    a[path[0]] = put(a[path[0]] ?? null, path.slice(1));
    return { ...n, a } as BlockNode;
  };
  const rows = l.row === "l" ? [...m.lets] : [...m.plots];
  if (!rows[l.i]) throw new Error(`No row ${key}`);
  rows[l.i] = { ...rows[l.i], expr: put(rows[l.i].expr, l.path) } as never;
  return l.row === "l" ? { ...m, lets: rows as LetRow[] } : { ...m, plots: rows as PlotRow[] };
}
/** Every location in reading order (rows: named results first, then lines), with its content. */
export function walk(m: Blocks): { key: string; node: Slot }[] {
  const out: { key: string; node: Slot }[] = [];
  const go = (key: string, n: Slot) => { out.push({ key, node: n }); childrenOf(n).forEach((c, i) => go(childKey(key, i), c)); };
  m.lets.forEach((r, i) => go(`l${i}`, r.expr));
  m.plots.forEach((r, i) => go(`p${i}`, r.expr));
  return out;
}

// ---------- types ----------
const whole = (...xs: number[]) => xs.every(Number.isInteger);
export const TYPE_WORDS: Record<SlotType, string> = {
  series: "a series", number: "a number (a constant or a Parameter)", whole: "a whole number (like 14, or a whole-number Parameter)", condition: "a condition (true/false)",
};
/** What a block produces. Refs take their result's type; `[n]` and Nz keep a condition a condition. */
export function outType(n: BlockNode, m: Blocks, depth = 0): SlotType {
  switch (n.k) {
    case "num": return Number.isInteger(n.v) ? "whole" : "number";
    case "bool": return "condition";
    case "param": return whole(n.def, n.min, n.max, n.step) ? "whole" : "number";
    case "ref": {
      const r = m.lets.find((x) => x.name.toLowerCase() === n.name.toLowerCase());
      return r?.expr && depth < 50 ? outType(r.expr, m, depth + 1) : "series";
    }
    case "ago": return n.a[0] && outType(n.a[0], m, depth + 1) === "condition" ? "condition" : "series";
    case "fn": {
      const out = FNS[n.f].out;
      if (out !== "arg0") return out;
      return n.a[0] && outType(n.a[0], m, depth + 1) === "condition" ? "condition" : "series";
    }
    default: return "series";
  }
}
export const accepts = (slot: SlotType, t: SlotType) =>
  slot === "series" || slot === t || (slot === "number" && t === "whole");

/** The type and label of the slot at `key` (row roots take anything). */
export function slotInfo(m: Blocks, key: string, indicators: Indicator[]): { type: SlotType; label: string; optional?: boolean } {
  const l = parseKey(key);
  if (!l.path.length) return { type: "series", label: l.row === "l" ? "value" : "line value" };
  const p = getAt(m, parentKey(key)), i = l.path[l.path.length - 1];
  if (p?.k === "fn") { const a = FNS[p.f].args[i]; return { type: a.type, label: a.label, optional: a.optional }; }
  if (p?.k === "ind") {
    const par = indicators.find((x) => x.id === p.id)?.params[i];
    return { type: par && !par.integer ? "number" : "whole", label: par?.label ?? `argument ${i + 1}` };
  }
  return { type: "series", label: "series" };
}

export function blockLabel(n: BlockNode, indicators: Indicator[] = []): string {
  switch (n.k) {
    case "num": return String(n.v);
    case "bool": return n.v ? "true" : "false";
    case "price": return priceLabel(n.f);
    case "param": return `Parameter “${n.label}”`;
    case "ref": return n.name;
    case "ago": return "bars ago";
    case "fn": return FNS[n.f].label;
    case "ind": { const i = indicators.find((x) => x.id === n.id); return i ? indLabel(i) : n.id; }
  }
}
export const indLabel = (i: Indicator) => (/^[a-z0-9]{2,6}$/.test(i.id) ? i.id.toUpperCase() : i.name);

// ---------- problems ----------
export interface Problem { key: string; kind: "empty" | "type" | "other"; message: string }
const rowName = (key: string, m: Blocks) => { const l = parseKey(key); return l.row === "l" ? (m.lets[l.i]?.name || `result ${l.i + 1}`) : `Line ${l.i + 1}`; };

/** Everything that would stop the engine or that the slots refuse. `type` problems are the ones edits may not add. */
export function problems(m: Blocks, indicators: Indicator[] = []): Problem[] {
  const out: Problem[] = [];
  const params = new Map<string, string>();
  for (const { key, node } of walk(m)) {
    const l = parseKey(key), slot = slotInfo(m, key, indicators), where = rowName(key, m);
    if (!node) { if (!slot.optional) out.push({ key, kind: "empty", message: `${where}: the ${slot.label} slot is empty.` }); continue; }
    const t = outType(node, m);
    if (!accepts(slot.type, t)) out.push({ key, kind: "type", message: `${where}: ${slot.label} needs ${TYPE_WORDS[slot.type]}; ${blockLabel(node, indicators)} gives ${TYPE_WORDS[t]}.` });
    if (node.k === "ref") {
      const at = m.lets.findIndex((x) => x.name.toLowerCase() === node.name.toLowerCase());
      if (at < 0) out.push({ key, kind: "type", message: `${where}: “${node.name}” is not a named result.` });
      else if (l.row === "l" && at >= l.i) out.push({ key, kind: "type", message: `${where}: “${node.name}” is defined ${at === l.i ? "by this row" : "below this row"}; a result can use only results above it.` });
    } else if (node.k === "num" && !Number.isFinite(node.v)) out.push({ key, kind: "other", message: `${where}: the number is not valid.` });
    else if (node.k === "ago" && !(Number.isInteger(node.n) && node.n >= 0 && node.n <= MAX_AGO)) out.push({ key, kind: "other", message: `${where}: bars ago must be a whole number from 0 to ${MAX_AGO}.` });
    else if (node.k === "ind") {
      const ind = indicators.find((x) => x.id === node.id);
      if (!ind) out.push({ key, kind: "other", message: `${where}: the catalogue has no indicator “${node.id}”.` });
      else if (node.out && !ind.outputs.includes(node.out)) out.push({ key, kind: "other", message: `${where}: ${indLabel(ind)} has no output “${node.out}”.` });
    } else if (node.k === "param") {
      const sig = `${node.def},${node.min},${node.max},${node.step}`, lower = node.label.trim().toLowerCase();
      if (!node.label.trim()) out.push({ key, kind: "other", message: `${where}: a Parameter needs a label.` });
      else if (params.has(lower) && params.get(lower) !== sig) out.push({ key, kind: "other", message: `${where}: another Parameter is already called “${node.label}”.` });
      params.set(lower, sig);
      if (![node.def, node.min, node.max, node.step].every(Number.isFinite) || node.min > node.max || node.def < node.min || node.def > node.max || node.step <= 0)
        out.push({ key, kind: "other", message: `${where}: Parameter “${node.label}” needs min ≤ default ≤ max and a step above 0.` });
    }
  }
  const seen = new Set<string>();
  m.lets.forEach((r, i) => {
    const key = `l${i}:name`, lower = r.name.toLowerCase();
    if (!IDENT.test(r.name)) out.push({ key, kind: "other", message: `Result ${i + 1}: a name starts with a letter and uses letters, digits or _ only.` });
    else if (reservedName(r.name, indicators)) out.push({ key, kind: "other", message: `Result ${i + 1}: “${r.name}” is a built-in name; choose another.` });
    else if (seen.has(lower)) out.push({ key, kind: "other", message: `Result ${i + 1}: “${r.name}” is already used.` });
    seen.add(lower);
  });
  const plotNames = new Set<string>();
  m.plots.forEach((r, i) => {
    const key = `p${i}:name`, lower = r.name.trim().toLowerCase();
    if (!lower) out.push({ key, kind: "other", message: `Line ${i + 1}: give the line a name.` });
    else if (plotNames.has(lower)) out.push({ key, kind: "other", message: `Line ${i + 1}: another line is already called “${r.name}”.` });
    plotNames.add(lower);
  });
  if (!m.plots.length) out.push({ key: "model", kind: "other", message: "Add at least one line to draw." });
  if (m.plots.length > MAX_PLOTS) out.push({ key: "model", kind: "other", message: `At most ${MAX_PLOTS} lines.` });
  if (params.size > MAX_PARAMS) out.push({ key: "model", kind: "other", message: `At most ${MAX_PARAMS} Parameters.` });
  return out;
}

// ---------- edits ----------
export type Edit = { ok: true; model: Blocks } | { ok: false; reason: string };
/** An edit is refused when it adds a type problem (wrong kind of block in a slot, or a result used above its definition). */
export function guard(before: Blocks, after: Blocks, indicators: Indicator[]): Edit {
  const old = new Set(problems(before, indicators).filter((p) => p.kind === "type").map((p) => p.message));
  const added = problems(after, indicators).filter((p) => p.kind === "type" && !old.has(p.message));
  return added.length ? { ok: false, reason: added[0].message } : { ok: true, model: after };
}
const isInside = (key: string, of: string) => key === of || key.startsWith(`${of}.`);

/** Put `node` into the slot at `key` (replacing what is there); with `from`, the block moves from that location. */
export function place(m: Blocks, key: string, node: BlockNode, indicators: Indicator[], from?: string): Edit {
  if (from && isInside(key, from)) return { ok: false, reason: "A block cannot go inside itself." };
  const slot = slotInfo(m, key, indicators), t = outType(node, m);
  if (!accepts(slot.type, t)) return { ok: false, reason: `The ${slot.label} slot needs ${TYPE_WORDS[slot.type]}; ${blockLabel(node, indicators)} gives ${TYPE_WORDS[t]}.` };
  const l = parseKey(key);
  if (node.k === "ref" && l.row === "l") {
    const at = m.lets.findIndex((x) => x.name.toLowerCase() === node.name.toLowerCase());
    if (at >= l.i) return { ok: false, reason: `“${node.name}” is defined ${at === l.i ? "by this row" : "below this row"}; a result can use only results above it.` };
  }
  return guard(m, setAt(from ? setAt(m, from, null) : m, key, node), indicators);
}
export const removeAt = (m: Blocks, key: string): Blocks => setAt(m, key, null);

export function renameLet(m: Blocks, i: number, name: string): Blocks {
  const old = m.lets[i].name.toLowerCase();
  const fix = (n: Slot): Slot => {
    if (!n) return n;
    if (n.k === "ref" && n.name.toLowerCase() === old) return { ...n, name };
    return "a" in n ? { ...n, a: n.a.map(fix) } as BlockNode : n;
  };
  return {
    ...m,
    lets: m.lets.map((r, j) => ({ name: j === i ? name : r.name, expr: fix(r.expr) })),
    plots: m.plots.map((r) => ({ ...r, expr: fix(r.expr) })),
  };
}
/** Required empty slots in reading order. */
export const emptyKeys = (m: Blocks, indicators: Indicator[] = []) =>
  walk(m).filter(({ key, node }) => !node && !slotInfo(m, key, indicators).optional).map((x) => x.key);

// ---------- generation ----------
type Ctx = { kind: "top" } | { kind: "bin"; prec: number; right: boolean; pow: boolean } | { kind: "neg" } | { kind: "not" };
const TOP: Ctx = { kind: "top" };
const isNegLike = (n: BlockNode) => (n.k === "num" && (n.v < 0 || Object.is(n.v, -0))) || (n.k === "fn" && n.f === "neg");
function needsParens(n: BlockNode, c: Ctx): boolean {
  if (c.kind === "top") return false;
  if (isNegLike(n)) return c.kind === "neg" || (c.kind === "bin" && (c.right || c.pow)); // -x + y reads the same everywhere; a ^ / right side does not
  if (n.k !== "fn") return false;
  if (n.f === "not") return c.kind === "neg" || (c.kind === "bin" && c.prec >= 3);
  const p = FNS[n.f].infix;
  if (p === undefined) return false;
  if (c.kind !== "bin") return true; // -(a + b), NOT (a > b)
  // comparisons do not chain and ^ operands are always grouped: the result reads the same whichever way the engine associates them
  return c.pow || p < c.prec || (p === c.prec && (c.right || p === 4));
}
export const quote = (s: string) => `"${s.replace(/[\\"]/g, (ch) => `\\${ch}`).replace(/[\r\n\t]/g, " ")}"`;
const fmt = (v: number) => (Number.isFinite(v) ? String(v) : "0");

export interface Generated { text: string; spans: Record<string, [number, number]>; holes: string[] }
/** Deterministic formula text: `name = expr;` per named result, then `Plot(expr, "Name"[, "price"]);` per line. Empty slots read `?`. */
export function generate(m: Blocks): Generated {
  let s = "";
  const spans: Record<string, [number, number]> = {}, holes: string[] = [];
  const w = (t: string) => { s += t; };
  const emit = (n: Slot, key: string, c: Ctx) => {
    const wrap = !!n && needsParens(n, c);
    if (wrap) w("(");
    const start = s.length;
    if (!n) { holes.push(key); w("?"); }
    else switch (n.k) {
      case "num": w(fmt(n.v)); break;
      case "bool": w(n.v ? "true" : "false"); break;
      case "price": w(n.f); break;
      case "ref": w(n.name); break;
      case "param": w(`Param(${quote(n.label)}, ${fmt(n.def)}, ${fmt(n.min)}, ${fmt(n.max)}, ${fmt(n.step)})`); break;
      case "ago": {
        const x = n.a[0] ?? null;
        if (x && (x.k === "price" || x.k === "ref")) { emit(x, childKey(key, 0), TOP); w(`[${fmt(n.n)}]`); }
        else { w("Ref("); emit(x, childKey(key, 0), TOP); w(`, ${n.n ? `-${fmt(n.n)}` : "0"})`); }
        break;
      }
      case "ind": {
        w(`${n.id}(`);
        n.a.forEach((x, i) => { if (i) w(", "); emit(x, childKey(key, i), TOP); });
        w(")");
        if (n.out) w(`.${n.out}`);
        break;
      }
      case "fn": {
        const d = FNS[n.f];
        if (d.infix !== undefined) {
          const pow = n.f === "pow";
          emit(n.a[0] ?? null, childKey(key, 0), { kind: "bin", prec: d.infix, right: false, pow });
          w(` ${d.text} `);
          emit(n.a[1] ?? null, childKey(key, 1), { kind: "bin", prec: d.infix, right: true, pow });
        } else if (d.prefix) {
          w(n.f === "not" ? "NOT " : "-");
          emit(n.a[0] ?? null, childKey(key, 0), { kind: n.f === "not" ? "not" : "neg" });
        } else {
          let last = n.a.length; // optional trailing arguments left empty are omitted
          while (last > 0 && !n.a[last - 1] && d.args[last - 1]?.optional) last--;
          const order = n.f === "cross_below" ? [1, 0] : [...Array(last).keys()];
          w(`${d.text}(`);
          order.forEach((i, j) => { if (j) w(", "); emit(n.a[i] ?? null, childKey(key, i), TOP); });
          w(")");
        }
      }
    }
    spans[key] = [start, s.length];
    if (wrap) w(")");
  };
  const lines: string[] = [];
  const row = (key: string, body: () => void) => {
    const start = s.length;
    body();
    spans[`${key}:row`] = [start, s.length];
    lines.push(key);
    w("\n");
  };
  m.lets.forEach((r, i) => row(`l${i}`, () => { w(`${r.name} = `); emit(r.expr, `l${i}`, TOP); w(";"); }));
  m.plots.forEach((r, i) => row(`p${i}`, () => { w("Plot("); emit(r.expr, `p${i}`, TOP); w(`, ${quote(r.name)}${r.onPrice ? ', "price"' : ""});`); }));
  return { text: s.replace(/\n$/, ""), spans, holes };
}
/** The block (innermost) or row an engine error position points at. */
export function keyAt(g: Generated, pos: { line: number; col: number }): string | undefined {
  const lines = g.text.split("\n");
  if (pos.line < 1 || pos.line > lines.length) return undefined;
  const at = lines.slice(0, pos.line - 1).reduce((k, l) => k + l.length + 1, 0) + pos.col - 1;
  let best: string | undefined, size = Infinity;
  for (const [key, [a, b]] of Object.entries(g.spans)) {
    if (at >= a && (at < b || (a === b && at === a)) && b - a < size) { best = key; size = b - a; }
  }
  return best;
}

// ---------- factories ----------
export const num = (v: number): BlockNode => ({ k: "num", v });
export const fn = (f: FnId, ...a: Slot[]): BlockNode => ({ k: "fn", f, a: FNS[f].args.map((_, i) => a[i] ?? null) });
export const price = (f: PriceName): BlockNode => ({ k: "price", f });
export const ref = (name: string): BlockNode => ({ k: "ref", name });
export const ago = (x: Slot, n = 1): BlockNode => ({ k: "ago", n, a: [x] });
export const param = (label: string, def = 14, min = 1, max = 200, step = 1): BlockNode => ({ k: "param", label, def, min, max, step });
export const indBlock = (i: Indicator, out?: string): BlockNode =>
  ({ k: "ind", id: i.id, ...(out && out !== i.outputs[0] && { out }), a: i.params.map((p) => num(p.default)) });
export const emptyBlocks = (): Blocks => ({ v: 1, lets: [], plots: [{ name: "Line 1", onPrice: false, expr: null }] });
/** What a new indicator starts with (same as the Formula tab's starter). */
export const starterBlocks = (): Blocks => ({
  v: 1, lets: [{ name: "N", expr: param("Periods", 14, 2, 200, 1) }],
  plots: [{ name: "Osc", onPrice: false, expr: fn("sub", price("C"), fn("ma", price("C"), ref("N"))) }],
});
export const uniqueName = (taken: string[], base: string) => {
  const low = new Set(taken.map((t) => t.toLowerCase()));
  if (!low.has(base.toLowerCase())) return base;
  for (let k = 2; ; k++) if (!low.has(`${base}${k}`.toLowerCase())) return `${base}${k}`;
};
export const paramLabels = (m: Blocks) => walk(m).flatMap(({ node }) => (node?.k === "param" ? [node.label] : []));

// ---------- palette ----------
export interface PaletteItem { id: string; group: string; sub?: string; label: string; hint: string; words: string; type: SlotType; make(): BlockNode }
export const PALETTE_GROUPS = ["Price", "Number", "Parameter", "My results", "Indicators", "Math", "Smoothing & windows", "Lookback", "Compare", "Logic", "State"];
export function paletteItems(m: Blocks, indicators: Indicator[]): PaletteItem[] {
  const items: PaletteItem[] = [];
  const add = (it: Omit<PaletteItem, "type">) => items.push({ ...it, type: outType(it.make(), m) });
  for (const p of PRICES) add({ id: `price:${p.f}`, group: "Price", label: p.label, hint: p.f, words: `${p.f} price`, make: () => price(p.f) });
  add({ id: "num", group: "Number", label: "Number", hint: "1", words: "constant value", make: () => num(1) });
  add({ id: "param", group: "Parameter", label: "Parameter", hint: "Param(…)", words: "tunable optimize input",
    make: () => param(uniqueName(paramLabels(m), "Periods")) });
  for (const r of m.lets) if (r.name) add({ id: `ref:${r.name}`, group: "My results", label: r.name, hint: "named result", words: "result variable series", make: () => ref(r.name) });
  for (const i of formulaIndicators(indicators)) add({
    id: `ind:${i.id}`, group: "Indicators", sub: i.category, label: indLabel(i), hint: i.name,
    words: [i.id, i.name, i.tv, i.ab, i.category].join(" "), make: () => indBlock(i),
  });
  const fnItem = (f: FnId, group = FNS[f].group) => add({ id: `fn:${f}`, group, label: FNS[f].label, hint: FNS[f].desc, words: `${FNS[f].text} ${f} ${FNS[f].desc}`, make: () => fn(f) });
  (["add", "sub", "mul", "div", "pow", "abs", "sqrt", "log", "exp", "max", "min", "neg"] as FnId[]).forEach((f) => fnItem(f));
  (["ma", "ema", "wma", "wilders", "stdev", "sum", "hhv", "llv", "roc", "change", "linreg", "ama"] as FnId[]).forEach((f) => fnItem(f));
  add({ id: "ago", group: "Lookback", label: "n bars ago", hint: "x[1]", words: "ref previous lookback", make: () => ago(null, 1) });
  (["gt", "lt", "ge", "le", "eq", "ne", "cross_above", "cross_below"] as FnId[]).forEach((f) => fnItem(f));
  (["and", "or", "not", "if"] as FnId[]).forEach((f) => fnItem(f));
  add({ id: "bool:true", group: "Logic", label: "true", hint: "always", words: "yes", make: () => ({ k: "bool", v: true }) });
  add({ id: "bool:false", group: "Logic", label: "false", hint: "never", words: "no", make: () => ({ k: "bool", v: false }) });
  (["barssince", "valuewhen", "nz"] as FnId[]).forEach((f) => fnItem(f));
  return items;
}
/** Matches label, group, formula name and keywords; an empty query returns the list unchanged. */
export function searchPalette(items: PaletteItem[], query: string): PaletteItem[] {
  const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return items;
  return items.filter((it) => { const hay = `${it.label} ${it.group} ${it.sub ?? ""} ${it.hint} ${it.words}`.toLowerCase(); return words.every((w) => hay.includes(w)); });
}

// ---------- reading saved layouts ----------
const isNum = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
function readNode(x: unknown): Slot | undefined {
  if (x === null) return null;
  if (typeof x !== "object") return undefined;
  const n = x as Record<string, unknown>;
  const kids = (len?: number): Slot[] | undefined => {
    if (!Array.isArray(n.a) || (len !== undefined && n.a.length !== len)) return undefined;
    const a = n.a.map(readNode);
    return a.every((c) => c !== undefined) ? (a as Slot[]) : undefined;
  };
  switch (n.k) {
    case "num": return isNum(n.v) ? { k: "num", v: n.v } : undefined;
    case "bool": return typeof n.v === "boolean" ? { k: "bool", v: n.v } : undefined;
    case "price": return PRICES.some((p) => p.f === n.f) ? { k: "price", f: n.f as PriceName } : undefined;
    case "ref": return typeof n.name === "string" ? { k: "ref", name: n.name } : undefined;
    case "param": return typeof n.label === "string" && [n.def, n.min, n.max, n.step].every(isNum)
      ? { k: "param", label: n.label, def: n.def as number, min: n.min as number, max: n.max as number, step: n.step as number } : undefined;
    case "ago": { const a = kids(1); return a && isNum(n.n) ? { k: "ago", n: n.n, a } : undefined; }
    case "ind": { const a = kids(); return a && typeof n.id === "string" ? { k: "ind", id: n.id, ...(typeof n.out === "string" && { out: n.out }), a } : undefined; }
    case "fn": { if (typeof n.f !== "string" || !(n.f in FNS)) return undefined; const a = kids(FNS[n.f as FnId].args.length); return a && { k: "fn", f: n.f as FnId, a }; }
    default: return undefined;
  }
}
/** A saved `blocks` value as a model, or null when it is missing, from another version or malformed. */
export function readBlocks(x: unknown): Blocks | null {
  const b = (x ?? {}) as Record<string, unknown>;
  if (b.v !== 1 || !Array.isArray(b.lets) || !Array.isArray(b.plots)) return null;
  const lets: LetRow[] = [], plots: PlotRow[] = [];
  for (const r of b.lets as Record<string, unknown>[]) {
    const expr = readNode(r?.expr ?? null);
    if (typeof r?.name !== "string" || expr === undefined) return null;
    lets.push({ name: r.name, expr });
  }
  for (const r of b.plots as Record<string, unknown>[]) {
    const expr = readNode(r?.expr ?? null);
    if (typeof r?.name !== "string" || expr === undefined) return null;
    plots.push({ name: r.name, onPrice: r.onPrice === true, expr });
  }
  return { v: 1, lets, plots };
}

// ---------- parser (design §1) ----------
export class FormulaParseError extends Error {
  constructor(message: string, readonly line: number, readonly col: number) { super(`line ${line}, col ${col}: ${message}`); }
}
type Tok = { t: "num" | "id" | "str" | "op" | "nl" | "eof"; v: string; pos: number };
const OPS2 = [">=", "<=", "==", "!=", "&&", "||"];
function tokenize(src: string, fail: (msg: string, pos: number) => never): Tok[] {
  const out: Tok[] = [];
  let i = 0;
  while (i < src.length) {
    const ch = src[i];
    if (ch === "\n") { out.push({ t: "nl", v: "\n", pos: i++ }); continue; }
    if (/\s/.test(ch)) { i++; continue; }
    if (src.startsWith("//", i)) { while (i < src.length && src[i] !== "\n") i++; continue; }
    const numM = /^(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?/.exec(src.slice(i));
    if (numM) { out.push({ t: "num", v: numM[0], pos: i }); i += numM[0].length; continue; }
    const idM = /^[A-Za-z_][A-Za-z0-9_]*/.exec(src.slice(i));
    if (idM) { out.push({ t: "id", v: idM[0], pos: i }); i += idM[0].length; continue; }
    if (ch === '"') {
      let j = i + 1, v = "";
      while (j < src.length && src[j] !== '"' && src[j] !== "\n") { if (src[j] === "\\" && j + 1 < src.length) j++; v += src[j++]; }
      if (src[j] !== '"') fail("the text has no closing quote", i);
      out.push({ t: "str", v, pos: i }); i = j + 1; continue;
    }
    const two = src.slice(i, i + 2);
    if (OPS2.includes(two)) { out.push({ t: "op", v: two, pos: i }); i += 2; continue; }
    if ("+-*/^<>!=()[],;.".includes(ch)) { out.push({ t: "op", v: ch, pos: i++ }); continue; }
    fail(`unexpected character “${ch}”`, i);
  }
  out.push({ t: "eof", v: "", pos: src.length });
  return out;
}

/** Formula text → block layout. Throws FormulaParseError (line/col) for anything blocks cannot show. Comments are not kept. */
export function parseFormula(src: string, indicators: Indicator[] = []): Blocks {
  const fail = (msg: string, pos: number): never => {
    const before = src.slice(0, pos).split("\n");
    throw new FormulaParseError(msg, before.length, before[before.length - 1].length + 1);
  };
  const toks = tokenize(src, fail);
  const known = formulaIndicators(indicators);
  let p = 0, depth = 0;
  const lets: LetRow[] = [], plots: PlotRow[] = [];
  const peek = (): Tok => { if (depth > 0) while (toks[p].t === "nl") p++; return toks[p]; };
  const next = () => { const t = peek(); p++; return t; };
  const isOp = (v: string) => { const t = peek(); return t.t === "op" && t.v === v; };
  const isWord = (w: string) => { const t = peek(); return t.t === "id" && t.v.toLowerCase() === w; };
  const expect = (v: string) => { const t = next(); if (t.t !== "op" || t.v !== v) fail(`expected “${v}”`, t.pos); return t; };
  const findLet = (name: string) => lets.find((r) => r.name.toLowerCase() === name.toLowerCase());

  const binary = (sub: () => BlockNode, ops: Record<string, FnId>, words: Record<string, FnId> = {}) => (): BlockNode => {
    let a = sub();
    for (;;) {
      const t = peek(), f = t.t === "op" ? ops[t.v] : t.t === "id" ? words[t.v.toLowerCase()] : undefined;
      if (!f) return a;
      next();
      a = fn(f, a, sub());
    }
  };
  const unary = (): BlockNode => {
    if (isOp("-")) {
      next();
      const x = unary();
      return x.k === "num" ? num(-x.v) : fn("neg", x);
    }
    if (isOp("+")) { next(); return unary(); }
    return pow();
  };
  const pow = (): BlockNode => {
    let a = postfix();
    while (isOp("^")) { next(); a = fn("pow", a, unary()); }
    return a;
  };
  const mul = binary(unary, { "*": "mul", "/": "div" });
  const add = binary(mul, { "+": "add", "-": "sub" });
  const rel = binary(add, { ">": "gt", "<": "lt", ">=": "ge", "<=": "le" });
  const eq = binary(rel, { "==": "eq", "!=": "ne" });
  const not = (): BlockNode => {
    if (isOp("!") || isWord("not")) { next(); return fn("not", not()); }
    return eq();
  };
  const and = binary(not, { "&&": "and" }, { and: "and" });
  const or = binary(and, { "||": "or" }, { or: "or" });
  const expr = or;

  const postfix = (): BlockNode => {
    let a = primary();
    for (;;) {
      if (isOp("[")) {
        next();
        depth++;
        const n = next();
        if (n.t !== "num" || !/^\d+$/.test(n.v) || Number(n.v) > MAX_AGO) fail(`bars ago must be a whole number from 0 to ${MAX_AGO}`, n.pos);
        expect("]"); depth--;
        a = ago(a, Number(n.v));
      } else if (isOp(".")) {
        const dot = next(), o = next();
        if (a.k !== "ind" || o.t !== "id") return fail("only a catalogue indicator has outputs (.name)", dot.pos);
        const id = a.id, ind = known.find((i) => i.id === id)!;
        const out = ind.outputs.find((x) => x.toLowerCase() === o.v.toLowerCase());
        if (!out) fail(`${indLabel(ind)} has no output “${o.v}”`, o.pos);
        a = { k: "ind", id, a: a.a, ...(out !== ind.outputs[0] && { out }) };
      } else return a;
    }
  };
  type Arg = { name?: string; node?: BlockNode; str?: string; pos: number };
  const args = (): Arg[] => {
    const list: Arg[] = [];
    expect("("); depth++;
    if (!isOp(")")) for (;;) {
      const t = peek();
      if (t.t === "str") { next(); list.push({ str: t.v, pos: t.pos }); }
      else if (t.t === "id" && toks[p + 1]?.t === "op" && toks[p + 1].v === "=") { next(); next(); list.push({ name: t.v, node: expr(), pos: t.pos }); }
      else list.push({ node: expr(), pos: t.pos });
      if (!isOp(",")) break;
      next();
    }
    depth--; expect(")");
    return list;
  };
  const exprArg = (a: Arg, what: string): BlockNode => {
    if (!a.node || a.name) fail(a.str !== undefined ? `text is allowed only in Param and Plot` : `${what} takes no named arguments`, a.pos);
    return a.node!;
  };
  const primary = (): BlockNode => {
    const t = next();
    if (t.t === "num") return num(Number(t.v));
    if (t.t === "op" && t.v === "(") { depth++; const x = expr(); depth--; expect(")"); return x; }
    if (t.t === "str") fail("text is allowed only in Param and Plot", t.pos);
    if (t.t !== "id") fail(t.t === "eof" || t.t === "nl" ? "the statement ends too early" : `unexpected “${t.v}”`, t.pos);
    const low = t.v.toLowerCase();
    if (isOp("(")) {
      const list = args();
      if (low === "param") {
        const [label, ...nums] = list;
        if (list.length !== 5 || label.str === undefined || !nums.every((a) => a.node?.k === "num"))
          fail(`Param needs a label and four numbers: Param("Label", default, min, max, step)`, t.pos);
        const [def, min, max, step] = nums.map((a) => (a.node as { v: number }).v);
        return param(label.str!, def, min, max, step);
      }
      if (low === "ref") {
        if (list.length !== 2) fail("Ref needs a series and a bar offset: Ref(x, -n)", t.pos);
        const off = list[1].node;
        if (off?.k !== "num" || !Number.isInteger(off.v)) fail("Ref's offset must be a whole number like -1", list[1].pos);
        const n = (off as { v: number }).v;
        if (n > 0) fail("Ref with a positive offset looks into the future; use a negative offset", list[1].pos);
        if (-n > MAX_AGO) fail(`Ref can look back at most ${MAX_AGO} bars`, list[1].pos);
        return ago(exprArg(list[0], "Ref"), -n);
      }
      if (low === "plot") fail("Plot(…) must be its own statement", t.pos);
      const f = FN_NAMES[low];
      if (f) {
        const d = FNS[f], need = d.args.filter((a) => !a.optional).length;
        if (list.length < need || list.length > d.args.length)
          fail(`${t.v} takes ${need === d.args.length ? need : `${need} to ${d.args.length}`} argument${d.args.length === 1 ? "" : "s"}`, t.pos);
        return fn(f, ...list.map((a) => exprArg(a, t.v)));
      }
      const ind = known.find((i) => i.id.toLowerCase() === low);
      if (ind) {
        const a: Slot[] = ind.params.map((pr) => num(pr.default));
        let named = false;
        list.forEach((x, i) => {
          if (x.str !== undefined) fail("text is allowed only in Param and Plot", x.pos);
          let at = i;
          if (x.name) { named = true; at = ind.params.findIndex((pr) => pr.key.toLowerCase() === x.name!.toLowerCase()); if (at < 0) fail(`${indLabel(ind)} has no parameter “${x.name}”`, x.pos); }
          else if (named) fail("positional arguments cannot follow named ones", x.pos);
          if (at >= ind.params.length) fail(`${indLabel(ind)} takes ${ind.params.length} parameter${ind.params.length === 1 ? "" : "s"}`, x.pos);
          const v = x.node!;
          if (!(v.k === "num" || v.k === "param" || v.k === "ref")) fail(`${indLabel(ind)}'s parameters must be numbers or Parameters`, x.pos);
          a[at] = v;
        });
        return { k: "ind", id: ind.id, a };
      }
      fail(indicators.length || !/^[a-z]/i.test(t.v) ? `unknown function ${t.v}` : `unknown function ${t.v} (the indicator catalogue is not loaded)`, t.pos);
    }
    if (low === "true" || low === "false") return { k: "bool", v: low === "true" };
    if (low in PRICE_NAMES) return price(PRICE_NAMES[low]);
    const r = findLet(t.v);
    if (r) return ref(r.name);
    return fail(`unknown name ${t.v}${FN_NAMES[low] || known.some((i) => i.id.toLowerCase() === low) ? " (it needs arguments in parentheses)" : ""}`, t.pos);
  };

  const end = () => {
    const t = peek();
    if (t.t === "op" && t.v === ";") { next(); return; }
    if (t.t === "nl" || t.t === "eof") return;
    fail(`expected “;” or a new line before “${t.v}”`, t.pos);
  };
  for (;;) {
    const t = peek();
    if (t.t === "eof") break;
    if (t.t === "nl" || (t.t === "op" && t.v === ";")) { next(); continue; }
    if (t.t === "id" && t.v.toLowerCase() === "plot" && toks[p + 1]?.v === "(") {
      next();
      const list = args();
      if (list.length < 2 || list.length > 3) fail(`Plot needs a value and a name: Plot(value, "Name")`, t.pos);
      const name = list[1].str, place3 = list[2]?.str;
      if (name === undefined) fail(`Plot's name must be text in quotes`, list[1].pos);
      if (list[2] && place3?.toLowerCase() !== "price") fail(`Plot's third argument can only be "price"`, list[2].pos);
      if (plots.length >= MAX_PLOTS) fail(`at most ${MAX_PLOTS} Plots`, t.pos);
      plots.push({ name: name!, onPrice: !!list[2], expr: exprArg(list[0], "Plot") });
    } else if (t.t === "id" && toks[p + 1]?.t === "op" && toks[p + 1].v === "=") {
      next(); next();
      if (findLet(t.v)) fail(`${t.v} is already defined; a name is assigned once`, t.pos);
      if (reservedName(t.v, known)) fail(`${t.v} is a built-in name`, t.pos);
      lets.push({ name: t.v, expr: expr() });
    } else fail(`blocks can show only “name = …” and Plot(…) statements`, t.pos);
    end();
  }
  if (!plots.length) fail("the formula has no Plot(…)", src.length);
  return { v: 1, lets, plots };
}
