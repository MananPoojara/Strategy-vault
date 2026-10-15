import { COPY } from "./labels";
import type { Catalogue, Cond, Dist, Operand, OptimizeDim, SeriesOperand, Spec, ValueOperand } from "./types";

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

export const conds = (spec: Spec, side: Side): Cond[] => (side === "entry" ? spec.entry.all : spec.exit.any);
const withConds = (spec: Spec, side: Side, list: Cond[]): Spec =>
  side === "entry" ? { ...spec, entry: { ...spec.entry, all: list } } : { ...spec, exit: { ...spec.exit, any: list } };
export const setCond = (spec: Spec, side: Side, i: number, cond: Cond) => withConds(spec, side, conds(spec, side).map((c, j) => (j === i ? cond : c)));
export const addCond = (spec: Spec, side: Side, cond: Cond) => withConds(spec, side, [...conds(spec, side), cond]);
export const removeCond = (spec: Spec, side: Side, i: number) => withConds(spec, side, conds(spec, side).filter((_, j) => j !== i));
export const conditionCount = (spec: Spec) => spec.entry.all.length + spec.exit.any.length;

const without = <T extends object>(obj: T, key: string): T => {
  const copy = { ...obj } as Record<string, unknown>;
  delete copy[key];
  return copy as T;
};
export const orderOf = (spec: Spec, key: PriceOrder["key"]): Dist | undefined => (key === "limit" ? spec.entry.limit : spec.exit[key]);

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
export const setDirection = (spec: Spec, dir: "long" | "short"): Spec => (dir === "short" ? { ...spec, direction: "short" } : without(spec, "direction"));

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

export interface OptimizableSetting { path: string; label: string; current: number; min: number; max: number; step: number; integer: boolean }
const orderPath = (key: PriceOrder["key"]) => (key === "limit" ? "entry.limit" : `exit.${key}`);
export function optimizablePaths(spec: Spec, cat: Catalogue): OptimizableSetting[] {
  const out: OptimizableSetting[] = [];
  for (const side of ["entry", "exit"] as const)
    conds(spec, side).forEach((cond, i) => {
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
    if (!d) continue;
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
const MAX_BYTES = 7500, MAX_ENCODED = 15000;

function distProblems(label: string, d: Dist, [lo, hi]: [number, number], zeroOk: boolean): string[] {
  if ("pct" in d)
    return (zeroOk ? d.pct >= lo : d.pct > lo) && d.pct <= hi ? [] : [`${label}: % must be ${zeroOk ? `from ${lo}` : `above ${lo}`} and at most ${hi}.`];
  const out: string[] = [];
  if (!(d.atr > 0 && d.atr <= 50)) out.push(`${label}: ATR multiplier must be above 0 and at most 50.`);
  if (d.atr_length !== undefined && !(Number.isInteger(d.atr_length) && d.atr_length >= 1 && d.atr_length <= 500))
    out.push(`${label}: ATR length must be a whole number from 1 to 500.`);
  return out;
}

/** UX pre-checks only; the engine stays the authority (unknown series ids are left to it). */
export function specProblems(spec: Spec, cat: Catalogue): string[] {
  const out: string[] = [];
  if (!spec.entry.all.length) out.push(COPY.emptyEntry);
  const priceExit = ["stop_loss", "target", "trailing"].some((k) => spec.exit[k] !== undefined);
  if (!spec.exit.any.length && !priceExit) out.push("exit.any needs at least one condition (or a stop_loss, target or trailing exit)");
  if (conditionCount(spec) > cat.max_conditions) out.push(`Use at most ${cat.max_conditions} conditions in total.`);
  for (const side of ["entry", "exit"] as const)
    conds(spec, side).forEach((c, i) => {
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
  const json = JSON.stringify(spec);
  if (specBytes(spec) > MAX_BYTES || new URLSearchParams({ spec: json }).toString().length > MAX_ENCODED) out.push(COPY.tooLong);
  return out;
}

/** The engine refuses a cost query that differs from execution.cost_bps, so only one is ever sent. */
export function runQuery(spec: Spec, fees: string, slippage: string): { cost?: number; slippage?: number } {
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
  const order = /^exit\.(stop_loss|target|trailing)( \(gap at the open\))?$/.exec(reason);
  if (order) return ({ stop_loss: "Stop-loss", target: "Target", trailing: "Trailing stop" } as const)[order[1] as "stop_loss"] + (order[2] ?? "");
  const rule = /^exit\.any\[(\d+)\]: (.*)$/.exec(reason);
  return rule ? `Exit rule ${Number(rule[1]) + 1}: ${rule[2]}` : reason;
}

export const canonicalJson = (value: unknown): string =>
  JSON.stringify(value, (_key, x) =>
    x && typeof x === "object" && !Array.isArray(x) ? Object.fromEntries(Object.entries(x).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))) : x);

export function operandLabel(o: Operand, cat: Catalogue): string {
  if (isValue(o)) return String(o.value);
  if (!isSeries(o)) return "kept as is";
  const special = SPECIAL_OPERANDS.find((x) => x.s === o.s);
  if (special) return special.label;
  const ind = cat.indicators.find((x) => x.id === o.s);
  const params = ind && o.p ? `(${ind.params.filter((p) => o.p![p.key] !== undefined).map((p) => o.p![p.key]).join(", ")})` : "";
  return [`${ind?.name ?? o.s}${params}${o.out ? `.${o.out}` : ""}`, o.tf && o.tf !== "D" ? o.tf : "",
    o.offset ? `${o.offset} bar${o.offset === 1 ? "" : "s"} ago` : ""].filter(Boolean).join(" · ");
}
