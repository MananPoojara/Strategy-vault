import { COPY } from "./labels";
import { formulaLabel, isFormulaOperand } from "./formula";
import type { Catalogue, Cond, Dist, IndicatorParam, Operand, OptimizeDim, RuleNode, SeriesOperand, Spec, ValueOperand } from "./types";

export type Side = "entry" | "exit";
/** Operands that are not catalogue series. Phase 7 adds entries here (or the catalogue starts listing them). */
export const SPECIAL_OPERANDS: { s: string; label: string; sides: Side[] }[] = [{ s: "bars_held", label: "Bars held", sides: ["exit"] }];
export interface PriceOrder { key: "limit" | "stop_loss" | "target" | "trailing"; side: Side; label: string; pct: [number, number]; zeroOk: boolean }
/** Price orders the engine accepts (rules.rs bounds). Phase 7 order types are added here. */
export const PRICE_ORDERS: PriceOrder[] = [
  { key: "limit", side: "entry", label: "Limit entry", pct: [0, 50], zeroOk: true },
  { key: "stop_loss", side: "exit", label: "Stop-loss", pct: [0, 99], zeroOk: false },
  { key: "target", side: "exit", label: "Target", pct: [0, 1000], zeroOk: false },
  { key: "trailing", side: "exit", label: "Trailing stop", pct: [0, 99], zeroOk: false },
];
export const OPERATOR_LABELS: Record<string, string> = {
  "=": "equals", "!=": "is not", crosses: "crosses", between: "is between", outside: "is outside", rising: "is rising for", falling: "is falling for",
  crosses_above: "crosses above", crosses_below: "crosses below", ">": "is above", "<": "is below", ">=": "is at least", "<=": "is at most",
};

export const defaultSpec = (): Spec => ({
  v: 1,
  entry: { all: [{ lhs: { s: "close" }, op: ">", rhs: { s: "ema", p: { length: 50 } } }] },
  exit: { any: [{ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } }] },
});

export const isValue = (o: unknown): o is ValueOperand =>
  typeof o === "object" && o !== null && Object.keys(o).length === 1 && typeof (o as ValueOperand).value === "number";
export const isSeries = (o: unknown): o is SeriesOperand => typeof o === "object" && o !== null && typeof (o as SeriesOperand).s === "string";

export const isCond = (n: unknown): n is Cond => typeof n === "object" && n !== null && "lhs" in n && "rhs" in n &&
  typeof (n as Cond).op === "string" && !["between", "outside", "rising", "falling"].includes((n as Cond).op);

export const conds = (spec: Spec, side: Side): RuleNode[] => (side === "entry" ? spec.entry.all : spec.exit.any);
const withConds = (spec: Spec, side: Side, list: RuleNode[]): Spec =>
  side === "entry" ? { ...spec, entry: { ...spec.entry, all: list } } : { ...spec, exit: { ...spec.exit, any: list } };
export const setCond = (spec: Spec, side: Side, i: number, cond: RuleNode) => withConds(spec, side, conds(spec, side).map((c, j) => (j === i ? cond : c)));
export const addCond = (spec: Spec, side: Side, cond: RuleNode) => withConds(spec, side, [...conds(spec, side), cond]);
export const removeCond = (spec: Spec, side: Side, i: number) => withConds(spec, side, conds(spec, side).filter((_, j) => j !== i));
export const conditionCount = (spec: Spec) => spec.entry.all.length + spec.exit.any.length;

const without = <T extends object>(obj: T, key: string): T => {
  const copy = { ...obj } as Record<string, unknown>;
  delete copy[key];
  return copy as T;
};
export const orderOf = (spec: Spec, key: PriceOrder["key"]): Dist | undefined => {
  const d = key === "limit" ? spec.entry.limit : spec.exit[key];
  return d && !("at" in d) ? d : undefined;
};

export function setExecution(spec: Spec, patch: Partial<NonNullable<Spec["execution"]>>): Spec {
  let execution: Record<string, unknown> = { ...(spec.execution ?? {}) };
  for (const [k, v] of Object.entries(patch)) execution = v === undefined ? without(execution, k) : { ...execution, [k]: v };
  return Object.keys(execution).length ? { ...spec, execution } : without(spec, "execution");
}

export function setOrder(spec: Spec, key: PriceOrder["key"], dist: Dist | undefined): Spec {
  if (key === "limit") {
    const entry = dist ? { ...spec.entry, limit: dist } : without(spec.entry, "limit");
    // The engine refuses execution.fill with a limit entry; fill also set exit fills, so keep that as exit_fill.
    const ex = spec.execution;
    return dist ? setExecution({ ...spec, entry }, { fill: undefined, ...(ex?.fill !== undefined && ex.exit_fill === undefined ? { exit_fill: ex.fill } : {}) }) : { ...spec, entry };
  }
  return { ...spec, exit: dist ? { ...spec.exit, [key]: dist } : without(spec.exit, key) };
}
export const setDirection = (spec: Spec, dir: NonNullable<Spec["direction"]>): Spec => (dir === "long" ? without(spec, "direction") : { ...spec, direction: dir });

function parsePath(path: string): (string | number)[] {
  return path.split(".").flatMap((part) => {
    const match = /^([^[\]]+)((?:\[\d+\])*)$/.exec(part);
    if (!match) throw new Error(`Unreadable setting path ${path}`);
    return [match[1], ...[...match[2].matchAll(/\[(\d+)\]/g)].map((x) => Number(x[1]))];
  });
}
export const getAt = (obj: unknown, path: string): unknown =>
  parsePath(path).reduce<unknown>((o, k) => (o == null ? undefined : (o as Record<string | number, unknown>)[k]), obj);
export function setAt<T>(obj: T, path: string, value: unknown): T {
  const keys = parsePath(path);
  const go = (o: unknown, i: number): unknown => {
    if (i === keys.length) return value;
    const copy = (Array.isArray(o) ? [...o] : { ...((o as object) ?? {}) }) as Record<string | number, unknown>;
    copy[keys[i]] = go((o as Record<string | number, unknown> | undefined)?.[keys[i]], i + 1);
    return copy;
  };
  return go(obj, 0) as T;
}

export function removeAt<T>(obj: T, path: string): T {
  const keys = parsePath(path);
  const go = (o: unknown, i: number): unknown => {
    const key = keys[i];
    if (!o || typeof o !== "object" || !Object.hasOwn(o, key)) return o;
    if (Array.isArray(o)) {
      if (typeof key !== "number") return o;
      const copy = [...o];
      if (i === keys.length - 1) copy.splice(key, 1);
      else {
        const next = go(o[key], i + 1);
        if (next === o[key]) return o;
        copy[key] = next;
      }
      return copy;
    }
    const record = o as Record<string | number, unknown>;
    const copy = { ...record };
    if (i === keys.length - 1) delete copy[key];
    else {
      const next = go(record[key], i + 1);
      if (next === record[key]) return o;
      copy[key] = next;
    }
    return copy;
  };
  return go(obj, 0) as T;
}
export const putAt = <T,>(obj: T, path: string, value: unknown): T => value === undefined ? removeAt(obj, path) : setAt(obj, path, value);

export interface OptimizableSetting { path: string; label: string; current: number; min: number; max: number; step: number; integer: boolean }
const orderPath = (key: PriceOrder["key"]) => (key === "limit" ? "entry.limit" : `exit.${key}`);
export function optimizablePaths(spec: Spec, cat: Catalogue): OptimizableSetting[] {
  const out: OptimizableSetting[] = [];
  for (const side of ["entry", "exit"] as const)
    conds(spec, side).forEach((cond, i) => {
      if (!isCond(cond)) return;
      for (const slot of ["lhs", "rhs"] as const) {
        const o = cond[slot];
        const base = `${side === "entry" ? "entry.all" : "exit.any"}[${i}].${slot}`;
        const tag = `${side === "entry" ? "Entry" : "Exit"} ${i + 1}`;
        if (isValue(o)) {
          const span = Math.max(Math.abs(o.value) * 0.5, 1);
          out.push({ path: `${base}.value`, label: `${tag} · number`, current: o.value, min: o.value - span, max: o.value + span, step: 1, integer: Number.isInteger(o.value) });
        } else if (isSeries(o)) {
          const ind = cat.indicators.find((x) => x.id === o.s);
          for (const p of ind?.params ?? [])
            out.push({ path: `${base}.p.${p.key}`, label: `${tag} · ${ind!.name} ${p.label}`, current: o.p?.[p.key] ?? p.default,
              min: p.min, max: p.max, step: p.step ?? (p.integer ? 1 : (p.max - p.min) / 100), integer: p.integer });
        }
      }
    });
  for (const order of PRICE_ORDERS) {
    const d = orderOf(spec, order.key);
    if (!d || !("pct" in d || "atr" in d)) continue;
    const isPct = "pct" in d;
    out.push({ path: `${orderPath(order.key)}.${isPct ? "pct" : "atr"}`, label: `${order.label} · ${isPct ? "%" : "ATR ×"}`,
      current: isPct ? d.pct : d.atr, min: isPct ? (order.zeroOk ? order.pct[0] : 0.5) : 0.25, max: isPct ? order.pct[1] : 50, step: isPct ? 0.5 : 0.25, integer: false });
  }
  return out;
}
export const valuesCount = (d: OptimizeDim) => (d.step > 0 && d.to >= d.from ? Math.floor((d.to - d.from) / d.step + 1e-9) + 1 : 0);
export const comboCount = (dims: OptimizeDim[]) => dims.reduce((n, d) => n * valuesCount(d), dims.length ? 1 : 0);
export const withOptimize = (spec: Spec, dims: OptimizeDim[]): Spec => (dims.length ? { ...spec, optimize: dims } : without(spec, "optimize"));

export const specBytes = (spec: Spec) => new TextEncoder().encode(JSON.stringify(spec)).length;
// Engine cap 8192 bytes; the gateway caps the URL-encoded query at 16384, so keep margins for ticker/dates.
export const SPEC_BYTE_BUDGET = 7500;
const MAX_ENCODED = 15000;
export const specTooLong = (spec: Spec) => specBytes(spec) > SPEC_BYTE_BUDGET || new URLSearchParams({ spec: JSON.stringify(spec) }).toString().length > MAX_ENCODED;

function distProblems(label: string, d: Dist, [lo, hi]: [number, number], zeroOk: boolean): string[] {
  if ("pct" in d)
    return (zeroOk ? d.pct >= lo : d.pct > lo) && d.pct <= hi ? [] : [`${label}: % must be ${zeroOk ? `from ${lo}` : `above ${lo}`} and at most ${hi}.`];
  if ("points" in d) return Number.isFinite(d.points) && (zeroOk ? d.points >= 0 : d.points > 0) && d.points <= 1e9
    ? [] : [`${label}: points must be ${zeroOk ? "from" : "above"} 0 and at most 1000000000.`];
  if ("profit_pct" in d) return d.profit_pct > 0 && d.profit_pct < 100 ? [] : [`${label}: % of profit must be above 0 and below 100.`];
  const out: string[] = [];
  if (!(d.atr > 0 && d.atr <= 50)) out.push(`${label}: ATR multiplier must be above 0 and at most 50.`);
  if (d.atr_length !== undefined && !(Number.isInteger(d.atr_length) && d.atr_length >= 1 && d.atr_length <= 500))
    out.push(`${label}: ATR length must be a whole number from 1 to 500.`);
  return out;
}

/** UX pre-checks only; the engine stays the authority (unknown series ids are left to it). */
export function specProblems(spec: Spec, cat: Catalogue): string[] {
  const out: string[] = [];
  if (!spec.entry.all.length && spec.portfolio?.mode!=="rotation") out.push(COPY.emptyEntry);
  if (spec.portfolio && spec.date_filter && !spec.date_filter.off) out.push("A date filter works on a single instrument only; switch it off (Settings → Date filter) for a portfolio.");
  const alwaysIn = spec.direction === "both" && (spec.execution?.opposite ?? "reverse") === "reverse" &&
    !spec.exit.any.length && !spec.short_side?.exit.any.length;
  const sides = [{ side: spec, prefix: "", title: "" },
    ...(spec.short_side ? [{ side: spec.short_side, prefix: "short_side.", title: "Short side: " }] : [])];
  for (const { side, prefix, title } of sides) {
    const dated = !!spec.date_filter && !spec.date_filter.off && spec.date_filter.exit_at_change !== false; // rules.rs: the window-change exit closes trades
    const filterExit = side.entry.all.some((n) => { const x = n as Record<string, unknown>; return x.filter_exit === true && x.off !== true; }); // rules.rs filter_exits
    const priceExit = ["stop_loss", "target", "trailing", "breakeven", "dual_atr"].some((k) => side.exit[k] !== undefined) || !!side.exit.take_profits?.length || dated || filterExit;
    if (spec.portfolio?.mode!=="rotation" && !alwaysIn && !side.exit.any.length && !priceExit)
      out.push(`${prefix}exit.any needs at least one condition (or a stop_loss, target, trailing, breakeven or take-profit exit)`);
    if (side.entry.all.length + side.exit.any.length > cat.max_conditions)
      out.push(`${title}${title ? "use" : "Use"} at most ${cat.max_conditions} conditions in total.`);
  }
  for (const side of ["entry", "exit"] as const)
    conds(spec, side).forEach((c, i) => {
      if (!isCond(c)) return;
      const tag = `${side === "entry" ? "Entry" : "Exit"} ${i + 1}`;
      if (isValue(c.lhs) && isValue(c.rhs)) out.push(`${tag}: a condition cannot compare two numbers; one side must be a series.`);
      for (const o of [c.lhs, c.rhs]) {
        const special = isSeries(o) ? SPECIAL_OPERANDS.find((x) => x.s === o.s && !x.sides.includes(side)) : undefined;
        if (special) out.push(`${tag}: ${special.label} can only be used in ${special.sides.join(" or ")} conditions.`);
        const ind = isSeries(o) ? cat.indicators.find((x) => x.id === o.s && !x.computed) : undefined;
        if (ind) out.push(`${tag}: ${ind.name} is listed only and cannot be used in rules. ${ind.note}`.trim());
      }
    });
  for (const order of PRICE_ORDERS) {
    const d = orderOf(spec, order.key);
    if (d) out.push(...distProblems(order.label, d, order.pct, order.zeroOk));
  }
  if (specTooLong(spec)) out.push(COPY.tooLong);
  return out;
}

/** The engine refuses a cost query that differs from execution.cost_bps, so only one is ever sent. */
export function runQuery(spec: Spec, fees: string, slippage: string): { cost?: number; slippage?: number } {
  if (spec.execution?.capital !== undefined) return {};
  const q: { cost?: number; slippage?: number } = {};
  if (spec.execution?.cost_bps === undefined && fees.trim() !== "" && Number.isFinite(Number(fees))) q.cost = Number(fees);
  if (slippage.trim() !== "" && Number.isFinite(Number(slippage))) q.slippage = Number(slippage);
  return q;
}

/** The engine needs more than 20 bars on each side of the split. */
export function splitDate(dates: string[], heldBackPct: number): string | null {
  if (heldBackPct <= 0) return null;
  const i = Math.floor(dates.length * (1 - heldBackPct / 100));
  return i > 20 && dates.length - i > 20 ? dates[i] : null;
}

export function friendlyReason(reason: string): string {
  if (reason === "open") return "Still open at the end";
  const order = /^exit\.(stop_loss|target|trailing|breakeven)( \(gap at the open\))?$/.exec(reason);
  if (order) return ({ stop_loss: "Stop-loss", target: "Target", trailing: "Trailing stop", breakeven: "Breakeven stop" } as const)[order[1] as "stop_loss"] + (order[2] ?? "");
  const rule = /^(short_side\.)?exit\.any\[(\d+)\]: (.*)$/.exec(reason);
  if (rule) return `${rule[1] ? "Short exit" : "Exit"} rule ${Number(rule[2]) + 1}: ${rule[3]}`;
  const fixed: Record<string, string> = { "reverse to long": "Reversed to long", "reverse to short": "Reversed to short", "opposite entry": "Opposite signal",
    expiry: "Contract expiry", margin_call: "Margin call", roll: "Contract roll", "exit.dual_atr": "Dual ATR stop" };
  return fixed[reason] ?? reason;
}

export const canonicalJson = (value: unknown): string =>
  JSON.stringify(value, (_key, x) =>
    x && typeof x === "object" && !Array.isArray(x) ? Object.fromEntries(Object.entries(x).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))) : x);

export function operandLabel(o: Operand, cat: Catalogue): string {
  if (isValue(o)) return String(o.value);
  if (isFormulaOperand(o)) return [formulaLabel(o), o.tf && o.tf !== "D" ? o.tf : "", o.offset ? `${o.offset} bar${o.offset === 1 ? "" : "s"} ago` : ""].filter(Boolean).join(" · ");
  if (!isSeries(o)) return "kept as is";
  const special = SPECIAL_OPERANDS.find((x) => x.s === o.s);
  if (special) return special.label;
  const ind = cat.indicators.find((x) => x.id === o.s);
  const params = ind && o.p ? `(${ind.params.filter((p) => o.p![p.key] !== undefined).map((p) => o.p![p.key]).join(", ")})` : "";
  return [`${ind?.name ?? o.s}${params}${o.out ? `.${o.out}` : ""}`, o.tf && o.tf !== "D" ? o.tf : "",
    o.offset ? `${o.offset} bar${o.offset === 1 ? "" : "s"} ago` : ""].filter(Boolean).join(" · ");
}

// ---- plain-language rules (Customize panel). One model for the summary text and the inline number inputs.
/** An editable number inside a phrase; `path` is a setAt path into the spec. */
export interface NumSlot { path: string; value: number; label: string; min?: number; max?: number; step?: number; integer?: boolean }
export type Phrase = (string | NumSlot)[];
export const phraseText = (p: Phrase) => p.map((x) => (typeof x === "string" ? x : String(x.value))).join("");

const UNIT: Record<string, string> = { D: "day", W: "week", M: "month" };
const PERIOD: Record<string, string> = { W: "weekly ", M: "monthly " };
const plural = (word: string, n: number) => (n === 1 ? word : `${word}s`);
const SHORT: Record<string, string> = { bbpct: "Bollinger %B" };
const OUT_SHORT: Record<string, string> = { plus_di: "+DI", minus_di: "−DI" };
const shortName = (id: string, name?: string) => SHORT[id] ?? (id.length <= 4 || !name ? id.toUpperCase() : name);

function operandPhrase(o: Operand, base: string, cat: Catalogue): Phrase {
  if (isValue(o)) return [{ path: `${base}.value`, value: o.value, label: "Threshold" }];
  if (!isSeries(o)) return ["a value kept as is"];
  const tf = o.tf ?? "D";
  const unit = UNIT[tf] ?? "bar";
  const ago: Phrase = o.offset
    ? [" ", { path: `${base}.offset`, value: o.offset, label: `${plural(unit, 2)} ago`, min: 1, step: 1, integer: true }, ` ${plural(unit, o.offset)} ago`] : [];
  if (cat.price_sources.includes(o.s)) return [`the ${PERIOD[tf] ?? ""}${o.s}`, ...ago];
  const ind = cat.indicators.find((x) => x.id === o.s);
  const params = ind ? ind.params.filter((p) => !o.p || o.p[p.key] !== undefined) : Object.keys(o.p ?? {}).map((key) => ({ key, label: key }));
  const slots: NumSlot[] = params.map((p) => {
    const meta = p as Partial<IndicatorParam>;
    return { path: `${base}.p.${p.key}`, value: o.p?.[p.key] ?? meta.default ?? 0, label: `${ind?.name ?? o.s} ${p.label}`,
      min: meta.min, max: meta.max, step: meta.step ?? (meta.integer ? 1 : undefined), integer: meta.integer };
  });
  const name = shortName(o.s, ind?.name);
  if (slots.length === 1 && params[0].key === "length" && (ind?.category === "Moving averages" || o.s === "lowest" || o.s === "highest")) {
    const n = slots[0].value;
    if (o.s === "lowest" || o.s === "highest") return [`the ${o.s} ${o.s === "lowest" ? "low" : "high"} of the last `, slots[0], ` ${plural(unit, n)}`, ...ago];
    return ["the ", slots[0], `-${unit} ${name}`, ...ago];
  }
  const label = o.out && OUT_SHORT[o.out] ? OUT_SHORT[o.out] : o.out && ind && ind.outputs.length > 1 ? `${name} ${o.out.replace(/_/g, " ")}` : name;
  return [`${PERIOD[tf] ?? ""}${label}`, ...(slots.length ? ["(", ...slots.flatMap((s, i) => (i ? [", ", s] : [s])), ")"] : []), ...ago];
}

function condPhrase(c: RuleNode, base: string, cat: Catalogue): { phrase: Phrase; when: boolean } {
  if (!isCond(c)) return { when: true, phrase: ["a rule shown only in Strategy rules"] };
  if (isSeries(c.lhs) && c.lhs.s === "bars_held" && isValue(c.rhs) && c.op === ">=")
    return { when: false, phrase: ["after ", { path: `${base}.rhs.value`, value: c.rhs.value, label: "Days held", min: 1, step: 1, integer: true }, ` ${plural("day", c.rhs.value)}`] };
  const side = (o: Operand, slot: string) => (isSeries(o) && o.s === "bars_held" ? ["the days held"] : operandPhrase(o, `${base}.${slot}`, cat));
  return { when: true, phrase: [...side(c.lhs, "lhs"), ` ${OPERATOR_LABELS[c.op] ?? c.op} `, ...side(c.rhs, "rhs")] };
}

const ORDER_WORDS: Record<PriceOrder["key"], string> = { limit: "limit order", stop_loss: "stop-loss", target: "profit target", trailing: "trailing stop" };
function orderPhrase(spec: Spec, order: PriceOrder): Phrase | null {
  const d = orderOf(spec, order.key);
  if (!d) return null;
  const path = orderPath(order.key);
  const size: Phrase = "pct" in d
    ? [{ path: `${path}.pct`, value: d.pct, label: `${order.label} %`, min: order.pct[0], max: order.pct[1], step: 0.5 }, "%"]
    : "points" in d ? [{ path: `${path}.points`, value: d.points, label: `${order.label} points`, min: 0, step: 1 }, " points"]
    : "profit_pct" in d ? [{ path: `${path}.profit_pct`, value: d.profit_pct, label: `${order.label} % of profit`, min: 1, max: 99, step: 1 }, "% of open profit"]
    : [{ path: `${path}.atr`, value: d.atr, label: `${order.label} ATR multiple`, min: 0.25, max: 50, step: 0.25 }, " × ATR(",
       { path: `${path}.atr_length`, value: d.atr_length ?? 14, label: `${order.label} ATR length`, min: 1, max: 500, step: 1, integer: true }, ")"];
  if (order.key === "limit") return ["using a limit order ", ...size, ` ${spec.direction === "short" ? "above" : "below"} the signal close`];
  return ["at a ", ...size, ` ${ORDER_WORDS[order.key]}`];
}

const tagged = (tag: string, p: Phrase): Phrase => p.map((x) => (typeof x === "string" ? x : { ...x, label: `${tag} · ${x.label}` }));
export interface SpecPhrases { buy: string; sell: string; entry: Phrase[]; entryOrder: Phrase | null; exitWhen: Phrase[]; exitOther: Phrase[] }
export function describeSpec(spec: Spec, cat: Catalogue): SpecPhrases {
  const short = spec.direction === "short";
  const exits = conds(spec, "exit").map((c, i) => { const x = condPhrase(c, `exit.any[${i}]`, cat); return { ...x, phrase: tagged(`Exit ${i + 1}`, x.phrase) }; });
  return {
    buy: short ? "Sell short" : "Buy", sell: short ? "Cover" : "Sell",
    entry: conds(spec, "entry").map((c, i) => tagged(`Entry ${i + 1}`, condPhrase(c, `entry.all[${i}]`, cat).phrase)),
    entryOrder: orderPhrase(spec, PRICE_ORDERS[0]),
    exitWhen: exits.filter((x) => x.when).map((x) => x.phrase),
    exitOther: [...exits.filter((x) => !x.when).map((x) => x.phrase),
      ...PRICE_ORDERS.slice(1).map((o) => orderPhrase(spec, o)).filter((p): p is Phrase => p !== null)],
  };
}

const joinList = (items: string[], word: string) => (items.length < 2 ? items.join("") : `${items.slice(0, -1).join(", ")} ${word} ${items[items.length - 1]}`);
/** e.g. "Buy when the close is above the 200-day SMA and RSI(2) is below 4. Sell when RSI(2) is above 65." */
export function specSummary(spec: Spec, cat: Catalogue): string {
  const d = describeSpec(spec, cat);
  const entry = d.entry.length ? `${d.buy} when ${joinList(d.entry.map(phraseText), "and")}` : `${d.buy} (no entry rule yet)`;
  const exitParts = [...(d.exitWhen.length ? [`when ${joinList(d.exitWhen.map(phraseText), "or")}`] : []), ...d.exitOther.map(phraseText)];
  return `${entry}${d.entryOrder ? `, ${phraseText(d.entryOrder)}` : ""}. ${exitParts.length ? `${d.sell} ${exitParts.join(" or ")}.` : "No exit rule yet."}`;
}
