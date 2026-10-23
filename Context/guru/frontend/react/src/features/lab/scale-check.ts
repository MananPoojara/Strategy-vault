// What an indicator is compared with: its own signal line, its usual level, the price, or its typical value; and the
// 0-trade check for two sides that never meet (Ulcer Index 0–25 "crosses above" EMA 50 of the price ≈ 25,000).
import { series } from "./client";
import { DEFAULT_LEVELS } from "./Workbench";
import { describeOperand } from "./rule-tree";
import { isSeries, isValue } from "./spec-model";
import type { Catalogue, Cond, Operand, SavedMarket, SeriesOperand, Spec } from "./types";

export interface Range { min: number; max: number; typical: number }

/** The usual right side for an indicator picked on the left; undefined = its typical value on the instrument (fetched). */
export function defaultRhs(o: SeriesOperand, cat: Catalogue): Operand | undefined {
  const ind = cat.indicators.find((i) => i.id === o.s);
  if (!ind) return undefined;
  const signal = ind.outputs.find((x) => x === "signal" || x === "trigger");
  if (signal && signal !== (o.out ?? ind.outputs[0])) return { ...o, out: signal }; // MACD crosses its signal line
  const level = DEFAULT_LEVELS[o.s]?.[0];
  if (level !== undefined) return { value: level }; // RSI 70, A/D ratio 1
  if (ind.placement === "price") return { s: "close", ...(o.tf ? { tf: o.tf } : {}) }; // a moving average or band against the price
  return undefined;
}

/** Plain series only: another symbol, pair or formula is not fetched here. */
const plain = (o: Operand): o is SeriesOperand => isSeries(o) && !o.symbol && !o.vs;
/** Two significant digits: 4.237 → 4.2, 23817 → 24000. */
export const round2 = (v: number) => Number(v.toPrecision(2));

export async function seriesRange(m: SavedMarket, ticker: string, o: SeriesOperand): Promise<Range | undefined> {
  const r = await series(m, { id: o.s, ticker, tf: o.tf, p: o.p });
  const v = (r.series[o.out ?? r.outputs[0]] ?? []).filter((x): x is number => typeof x === "number" && Number.isFinite(x)).sort((a, b) => a - b);
  return v.length ? { min: v[0], max: v[v.length - 1], typical: v[Math.floor(v.length / 2)] } : undefined;
}

const fmt = (v: number) => v.toLocaleString("en-IN", { maximumSignificantDigits: 4 });
const span = (r: Range) => r.min === r.max ? fmt(r.min) : `${fmt(r.min)} to ${fmt(r.max)}`;

/** Whether `op` can ever hold between two ranges (a number is a range of one value); undefined = not judged. */
export function canMeet(op: string, a: Range, b: Range): boolean | undefined {
  if (op.startsWith("crosses")) return a.max >= b.min && a.min <= b.max && !(a.min === a.max && b.min === b.max);
  if (op === ">" || op === ">=") return op === ">" ? a.max > b.min : a.max >= b.min;
  if (op === "<" || op === "<=") return op === "<" ? a.min < b.max : a.min <= b.max;
  return undefined;
}

/** Plain-language notes for the top-level buy rules that can never be true on this instrument (a run with 0 trades). */
export async function neverMeets(spec: Spec, cat: Catalogue, m: SavedMarket, ticker: string): Promise<string[]> {
  const cache = new Map<string, Promise<Range | undefined>>();
  const range = (o: Operand): Promise<Range | undefined> => {
    if (isValue(o)) return Promise.resolve({ min: o.value, max: o.value, typical: o.value });
    if (!plain(o)) return Promise.resolve(undefined);
    const key = JSON.stringify(o);
    if (!cache.has(key)) cache.set(key, seriesRange(m, ticker, o).catch(() => undefined));
    return cache.get(key)!;
  };
  const notes = await Promise.all(spec.entry.all.map(async (n, i) => {
    const c = n as Cond;
    if (c.off === true || !("lhs" in c) || !("rhs" in c)) return undefined;
    const [a, b] = await Promise.all([range(c.lhs), range(c.rhs)]);
    if (!a || !b || canMeet(c.op, a, b) !== false) return undefined;
    const side = (o: Operand, r: Range) => isValue(o) ? fmt(o.value) : `${describeOperand(o, cat)} (${span(r)} on ${ticker})`;
    const fix = isValue(c.rhs) ? `use a number between ${fmt(a.min)} and ${fmt(a.max)}, e.g. ${fmt(round2(a.typical))}` : `compare it with a number, e.g. ${fmt(round2(a.typical))}`;
    return `Buy rule ${i + 1} is never true: ${side(c.lhs, a)} never reaches ${side(c.rhs, b)}. To fix it, ${fix}.`;
  }));
  return notes.filter((x): x is string => !!x);
}
