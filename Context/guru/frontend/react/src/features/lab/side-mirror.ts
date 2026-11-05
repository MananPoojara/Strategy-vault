/** Long ↔ short: the rules the lab's presets build for one side (defaults.ts firstRule, the Exit card's Supertrend and Indicator
 *  exits, rule-tree.ts EXIT_PRESETS) mirrored for the other, so switching Positions keeps every preset on its side. Only these
 *  shapes are mirrored; any other rule (ATR > 50, volume, bars held, a candle count) means the same for both sides and is kept.
 *  Mirroring twice gives the rule back. */
import { DEFAULT_LEVELS } from "./Workbench";
import type { Band } from "./BandRule";
import type { RuleNode } from "./types";

type O = Record<string, unknown>;
const FLIP: Record<string, string> = { crosses_above: "crosses_below", crosses_below: "crosses_above", ">": "<", "<": ">", ">=": "<=", "<=": ">=", rising: "falling", falling: "rising" };
const BAND: Record<string, string> = { upper: "lower", lower: "upper" };
/** ±1 trend-state outputs (Supertrend, Chandelier, Volatility stop … direction; NRTR, PP Supertrend trend). */
const STATE = new Set(["direction", "trend"]);
const SIGNAL = new Set(["signal", "trigger"]);
const isSeries = (o: unknown): o is O => !!o && typeof o === "object" && typeof (o as O).s === "string";
const isClose = (o: unknown) => isSeries(o) && o.s === "close" && !o.out;
const band = (o: O) => (typeof o.out === "string" && BAND[o.out] ? { ...o, out: BAND[o.out] } : o);
const num = (v: number) => Math.round(v * 1e9) / 1e9;

export function mirrorRule(node: RuleNode): RuleNode {
  const x = node as O;
  if (Array.isArray(x.all)) return { ...x, all: (x.all as RuleNode[]).map(mirrorRule) } as RuleNode;
  if (Array.isArray(x.any)) return { ...x, any: (x.any as RuleNode[]).map(mirrorRule) } as RuleNode;
  const op = typeof x.op === "string" ? FLIP[x.op] : undefined, l = x.lhs, r = x.rhs as O | undefined;
  if (!op || !isSeries(l)) return node;
  const flipped = { ...x, op } as O;
  if (x.op === "rising" || x.op === "falling") return flipped as RuleNode; // the indicator rising ↔ falling for N bars
  if (!r) return node;
  // since-entry exits: a trail from the highest ↔ lowest value, a stop or target on the entry price (×0.95 ↔ ×1.05, −10 ↔ +10)
  if (typeof r.since_entry === "string") {
    const se = r.since_entry === "highest" ? "lowest" : r.since_entry === "lowest" ? "highest" : r.since_entry;
    return { ...flipped, rhs: { ...r, since_entry: se, ...(typeof r.add === "number" ? { add: -r.add } : {}), ...(typeof r.mul === "number" ? { mul: num(2 - r.mul) } : {}) } } as unknown as RuleNode;
  }
  if (STATE.has(String(l.out)) && r.value === 0) return flipped as RuleNode; // a trend state against 0
  if (isSeries(r) && r.s === l.s && SIGNAL.has(String(r.out))) return flipped as RuleNode; // a line against its signal line
  const levels = DEFAULT_LEVELS[l.s as string];
  if (levels?.length && typeof r.value === "number") // an oscillator against its level: RSI 30 ↔ 70, CCI −100 ↔ 100
    return { ...flipped, rhs: { ...r, value: num(Math.min(...levels) + Math.max(...levels) - r.value) } } as RuleNode;
  if (isClose(l) && isSeries(r)) return { ...flipped, rhs: band(r) } as RuleNode; // the close against a price line or a channel side
  if (isClose(r)) return { ...flipped, lhs: band(l) } as RuleNode; // a price line against the close
  return node;
}

/** The comparison for an indicator just picked on a rule's left side, set from the side it serves: `bull` for a long entry or a
 *  short exit, otherwise bear. `rhs` is its usual comparison (scale-check.ts defaultRhs); the rule keeps its style (a cross
 *  stays a cross). A price line against the close: bull = the line crosses below the close (the price rises above it); an
 *  oscillator: bull = its low level (crosses above 30 / < 30), bear = its high one; a line and its signal line: bull = crosses above.
 *  Picking used to keep the previous rule's condition: RSI "crosses above 30" became "Supertrend crosses above the close" on a
 *  long, a sell signal (Ritik, TS_NF_En_ST 1,5_EX_ST 5,1). */
export function pickedComparison(o: { s: string }, rhs: unknown, op: string | undefined, bull: boolean): { op?: string; rhs?: unknown } {
  const cross = !op || op.startsWith("crosses");
  const way = (up: boolean) => (cross ? (up ? "crosses_above" : "crosses_below") : up ? ">" : "<");
  if (!rhs || typeof rhs !== "object") return { rhs };
  const r = rhs as O;
  if (isSeries(r) && r.s === o.s && SIGNAL.has(String(r.out))) return { op: way(bull), rhs };
  const levels = DEFAULT_LEVELS[o.s];
  if (levels?.length && typeof r.value === "number") {
    const low = Math.min(...levels), high = Math.max(...levels);
    return { op: cross ? way(bull) : (bull ? "<" : ">"), rhs: { ...r, value: bull ? low : high } };
  }
  if (isClose(r)) return { op: way(!bull), rhs }; // the line below the price is bullish
  return { rhs };
}

/** A band setup (BandRule) for the other side: below the lower band then back up ↔ above the upper band then back down; the
 *  trigger candle, the cancel line and Bollinger %B's level (0 ↔ 1) swap with it; its extra conditions are mirrored too. */
export function mirrorBand(b: Band): Band {
  const swap = <T extends string>(v: T): T => (v === "upper" ? "lower" : v === "lower" ? "upper" : v) as T;
  return { ...b, from: swap(b.from), dir: b.dir === "below" ? "above" : "below", candle: b.candle === "green" ? "red" : b.candle === "red" ? "green" : b.candle,
    cancel: swap(b.cancel), extra: b.extra.map(mirrorRule), ...(b.pct !== undefined ? { pct: num(1 - b.pct) } : {}) };
}
