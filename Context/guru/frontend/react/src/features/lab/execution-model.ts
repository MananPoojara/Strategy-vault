import { COPY, FUTURES_NOTE, MODEL_NOTE, MONEY_BLOCKED } from "./labels";
import { getAt, putAt, removeAt, setAt, setExecution } from "./spec-model";
import type { InstrumentOption } from "./instruments";
import type { Catalogue, EntryLevel, EntrySpec, ExitSpec, Market, ShortSide, Sizing, Spec, TakeProfit } from "./types";

export type Direction = "long" | "short" | "both";
export type SidePre = "" | "short_side.";
export const directionOf = (s: Spec): Direction => s.direction ?? "long";
export const hasShorts = (s: Spec) => directionOf(s) !== "long";
export const DEFAULT_SHORT_SIDE: ShortSide = {
  entry: { all: [{ lhs: { s: "close" }, op: "<", rhs: { s: "ema", p: { length: 50 } } }] },
  exit: { any: [{ lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } }] },
};
const omit = <T extends object>(o: T, ...keys: string[]): T => { const c = { ...o } as Record<string, unknown>; for (const k of keys) delete c[k]; return c as T; };
export function withDirection(spec: Spec, dir: Direction, kept?: ShortSide): Spec {
  let s = omit(spec, "direction", "short_side");
  if (dir !== "both") s = setExecution(s, { opposite: undefined });
  if (dir === "short") return { ...s, direction: "short" };
  if (dir === "both") return { ...s, direction: "both", short_side: structuredClone(spec.short_side ?? kept ?? DEFAULT_SHORT_SIDE) };
  return s;
}
export const sideOf = (s: Spec, pre: SidePre): { entry: EntrySpec; exit: ExitSpec } | undefined => (pre ? s.short_side : s);
export type EntryOrderType = "market" | "limit" | "stop" | "stop_limit";
export const entryOrderType = (e: EntrySpec): EntryOrderType => (e.limit && e.stop ? "stop_limit" : e.stop ? "stop" : e.limit ? "limit" : "market");
export function withEntryOrder(spec: Spec, pre: SidePre, type: EntryOrderType): Spec {
  const side = sideOf(spec, pre);
  if (!side) return spec;
  const e = side.entry;
  let entry = omit(e, "limit", "stop", "valid_bars");
  if (type === "limit" || type === "stop_limit") entry = { ...entry, limit: e.limit ?? { pct: 1 } };
  if (type === "stop" || type === "stop_limit") entry = { ...entry, stop: e.stop ?? { pct: 1 } };
  if (type !== "market" && e.valid_bars !== undefined) entry = { ...entry, valid_bars: e.valid_bars };
  let s = setAt(spec, `${pre}entry`, entry);
  if (type !== "market") { // the engine refuses execution.fill with an order; fill also set exits, so keep it as exit_fill
    const ex = s.execution;
    s = setExecution(s, { fill: undefined, ...(ex?.fill !== undefined && ex.exit_fill === undefined ? { exit_fill: ex.fill } : {}) });
  }
  return s;
}
export type LevelSlot = "entry.limit" | "entry.stop" | "exit.stop_loss" | "exit.target" | "exit.trailing" | "exit.trailing.activate" | "exit.breakeven.after" | "exit.breakeven.plus";
export const LEVEL_SLOTS: LevelSlot[] = ["entry.limit", "entry.stop", "exit.stop_loss", "exit.target", "exit.trailing", "exit.trailing.activate", "exit.breakeven.after", "exit.breakeven.plus"];
export interface LevelBounds { label: string; maxPct: number; zeroOk: boolean; at: boolean; pct: number }
/** rules.rs SLOTS and dist() bounds: entry orders may sit at the signal close (0); exits need > 0. */
export const LEVELS: Record<LevelSlot, LevelBounds> = {
  "entry.limit": { label: "Limit", maxPct: 50, zeroOk: true, at: true, pct: 1 },
  "entry.stop": { label: "Stop", maxPct: 50, zeroOk: true, at: true, pct: 1 },
  "exit.stop_loss": { label: "Stop-loss", maxPct: 99, zeroOk: false, at: false, pct: 5 },
  "exit.target": { label: "Target", maxPct: 1000, zeroOk: false, at: false, pct: 10 },
  "exit.trailing": { label: "Trailing stop", maxPct: 99, zeroOk: false, at: false, pct: 5 },
  "exit.trailing.activate": { label: "Trailing activation", maxPct: 1000, zeroOk: false, at: false, pct: 2 },
  "exit.breakeven.after": { label: "Breakeven after", maxPct: 1000, zeroOk: false, at: false, pct: 2 },
  "exit.breakeven.plus": { label: "Breakeven plus", maxPct: 1000, zeroOk: false, at: false, pct: 0.1 },
};
export const TP_BOUNDS: LevelBounds = { label: "Take-profit", maxPct: 1000, zeroOk: false, at: false, pct: 5 };
export type LevelForm = "pct" | "atr" | "points" | "at";
export const levelForm = (d: unknown): LevelForm | undefined =>
  typeof d === "object" && d !== null ? (["at", "pct", "atr", "points"] as const).find((k) => k in d) : undefined;
export function defaultLevel(slot: LevelSlot | "take_profit", form: LevelForm, short: boolean): EntryLevel {
  if (form === "atr") return { atr: 2, atr_length: 14 };
  if (form === "points") return { points: 10 };
  if (form === "at") return (slot === "entry.stop") !== short ? { at: { highest: { s: "high" }, bars: 5 } } : { at: { lowest: { s: "low" }, bars: 5 } };
  return { pct: slot === "take_profit" ? TP_BOUNDS.pct : LEVELS[slot].pct };
}
/** The level at a slot; the trailing distance is returned without its activation. */
export function levelAt(s: Spec, pre: SidePre, slot: LevelSlot): EntryLevel | undefined {
  const v = getAt(s, pre + slot) as Record<string, unknown> | undefined;
  return (slot === "exit.trailing" && v ? omit(v, "activate") : v) as EntryLevel | undefined;
}
export function withLevel(s: Spec, pre: SidePre, slot: LevelSlot, d: EntryLevel | undefined): Spec {
  const path = pre + slot;
  if (slot === "exit.breakeven.after" && !d) return removeAt(s, `${pre}exit.breakeven`);
  if (!d) return removeAt(s, path);
  const old = getAt(s, path);
  const extra = old && typeof old === "object" ? omit(old, "pct", "atr", "atr_length", "points", "at") : {};
  return setAt(s, path, { ...extra, ...d });
}
export const takeProfits = (s: Spec, pre: SidePre): TakeProfit[] => {
  const list = getAt(s, `${pre}exit.take_profits`);
  return Array.isArray(list) ? list : [];
};
export const withTakeProfits = (s: Spec, pre: SidePre, list: TakeProfit[]) => putAt(s, `${pre}exit.take_profits`, list.length ? list : undefined);
export type ResultMode = "unit" | "money";
export const modeOf = (s: Spec): ResultMode => (s.execution?.capital !== undefined ? "money" : "unit");
export const DEFAULT_CAPITAL = 1_000_000;
export function withMode(s: Spec, mode: ResultMode): Spec {
  if (mode === "money") return setExecution(s, { capital: s.execution?.capital ?? DEFAULT_CAPITAL, cost_bps: undefined });
  const z = s.execution?.sizing;
  return setExecution(s, { capital: undefined, costs: undefined, sizing: z && typeof z === "object" && ("units" in z || "value" in z) ? undefined : z });
}
export type SizingKind = "full" | "pct_equity" | "risk_pct" | "units" | "value";
export const sizingKind = (s: Spec): SizingKind => { const z = s.execution?.sizing; return z && typeof z === "object" ? ((["pct_equity", "risk_pct", "units", "value"] as const).find((k) => k in z) ?? "full") : "full"; };
const SIZING_DEFAULT: Record<Exclude<SizingKind, "full">, Sizing> = { pct_equity: { pct_equity: 100 }, risk_pct: { risk_pct: 1 }, units: { units: 100 }, value: { value: 100_000 } };
export const withSizingKind = (s: Spec, k: SizingKind) => setExecution(s, { sizing: k === "full" ? undefined : {
  ...omit(s.execution?.sizing ?? {}, "pct_equity", "risk_pct", "units", "value"), ...SIZING_DEFAULT[k],
} });
export interface InstrumentContext { kind: string; market: Market }
/** in/us rows without a native asset type are indices (instruments.ts); gl rows carry their kind (index | cfd | crypto). */
export const instrumentContext = (market: Market, option?: InstrumentOption): InstrumentContext => ({ market, kind: option?.type ?? (market === "gl" ? "unknown" : "index") });
export function moneyBlocked(ctx: InstrumentContext, spec: Spec): string | null {
  if (ctx.kind === "index") return MONEY_BLOCKED.index;
  if (ctx.kind === "crypto") return MONEY_BLOCKED.crypto;
  if (ctx.kind === "cfd") return MONEY_BLOCKED.cfd;
  if (!["individual_stock", "etf"].includes(ctx.kind)) return "Money mode needs an admitted stock or ETF.";
  return ctx.market === "in" && hasShorts(spec) ? MONEY_BLOCKED.indiaShort : null;
}
/** Pre-run mirror of lab/model.rs model(); the response's execution_model replaces it after a run. */
export function instrumentNotes(ctx: InstrumentContext, spec: Spec): string[] {
  const shorts = hasShorts(spec);
  const notes = ctx.kind === "index" ? [MODEL_NOTE.index] : ctx.kind === "crypto" ?
    shorts ? [MODEL_NOTE.crypto, MODEL_NOTE.cryptoShort] : [MODEL_NOTE.crypto] : ctx.kind === "cfd" ? [MODEL_NOTE.cfd] :
    shorts && ctx.market === "in" ? [MODEL_NOTE.indiaShort] : shorts && modeOf(spec) === "unit" ? [MODEL_NOTE.usShortUnit] : [];
  if (modeOf(spec) === "money" && (spec.execution?.cooldown || spec.execution?.risk))
    return [...notes, "Risk pauses and cooldowns use the simulator's unit returns, before money rounding and declared costs."];
  return notes;
}

export const supportsMoney = (cat: Catalogue): boolean =>
  !!cat.execution?.sizing?.includes("units") && !!cat.execution.sizing?.includes("value");

const finite = (n: unknown): n is number => typeof n === "number" && Number.isFinite(n);
const bounded = (n: unknown, lo: number, hi: number, whole = false) => finite(n) && n >= lo && n <= hi && (!whole || Number.isInteger(n));
function levelProblems(label: string, d: unknown, b: LevelBounds): string[] {
  if (!d || typeof d !== "object") return [`${label}: choose %, ATR or points.`];
  const v = d as Record<string, unknown>;
  if ("ticks" in v) return [`${label}: ticks need a futures contract, and futures contract data is not admitted.`];
  const forms = ["at", "pct", "atr", "points"].filter((k) => k in v);
  if (forms.length !== 1 || ("atr_length" in v && !("atr" in v))) return [`${label}: choose exactly one of %, ATR, points or an entry At level.`];
  if ("at" in v) return b.at ? [] : [`${label}: an At level works only for entry orders.`];
  const valid = (n: unknown, max: number) => bounded(n, 0, max) && (b.zeroOk || (n as number) > 0);
  if ("pct" in v) return valid(v.pct, b.maxPct) ? [] : [`${label}: % must be ${b.zeroOk ? "from 0" : "above 0"} and at most ${b.maxPct}.`];
  if ("points" in v) return valid(v.points, 1e9) ? [] : [`${label}: points must be ${b.zeroOk ? "from 0" : "above 0"} and at most 1,000,000,000.`];
  const out: string[] = [];
  if (!bounded(v.atr, Number.MIN_VALUE, 50)) out.push(`${label}: ATR multiplier must be above 0 and at most 50.`);
  if (v.atr_length !== undefined && !bounded(v.atr_length, 1, 500, true)) out.push(`${label}: ATR length must be a whole number from 1 to 500.`);
  return out;
}
function comparable(d: unknown): [string, number] | undefined {
  if (!d || typeof d !== "object") return;
  const v = d as Record<string, unknown>;
  if (finite(v.pct)) return ["pct", v.pct];
  if (finite(v.points)) return ["points", v.points];
  if (finite(v.atr)) return [`atr:${v.atr_length ?? 14}`, v.atr];
}

/** Client-side hints only; the engine validates every field again before computing a result. */
export function executionProblems(spec: Spec, cat: Catalogue, ctx: InstrumentContext): string[] {
  const out: string[] = [];
  const caps = cat.execution;
  const sides: [SidePre, { entry: EntrySpec; exit: ExitSpec }][] = [["", spec]];
  if (directionOf(spec) === "both" && spec.short_side) sides.push(["short_side.", spec.short_side]);
  if (directionOf(spec) === "both" && !spec.short_side) out.push("Both directions need short-side entry and exit rules.");
  if (directionOf(spec) !== "both" && spec.short_side) out.push("Short-side rules apply to Both directions only.");
  for (const [pre, side] of sides) {
    const prefix = pre ? "Short " : directionOf(spec) === "both" ? "Long " : "";
    for (const slot of LEVEL_SLOTS) {
      const d = levelAt(spec, pre, slot);
      if (d !== undefined) out.push(...levelProblems(prefix + LEVELS[slot].label, d, LEVELS[slot]));
    }
    const tps = takeProfits(spec, pre);
    if (side.exit.take_profits !== undefined && !Array.isArray(side.exit.take_profits)) out.push(`${prefix}Take-profits: use a list of levels.`);
    if (side.exit.take_profits && !tps.length) out.push(`${prefix}Take-profits: add at least one level or remove the exit.`);
    if (tps.length > (caps?.max_take_profits ?? 3)) out.push(`${prefix}Take-profits: use at most ${caps?.max_take_profits ?? 3} levels.`);
    tps.forEach((tp, i) => {
      if (!tp || typeof tp !== "object") { out.push(`${prefix}Take-profit ${i + 1}: choose a level and size.`); return; }
      if (!bounded(tp.size, Number.MIN_VALUE, 1) || tp.size === 1) out.push(`${prefix}Take-profit ${i + 1}: size must be above 0% and below 100% of the initial position.`);
      out.push(...levelProblems(`${prefix}Take-profit ${i + 1}`, omit(tp, "size"), TP_BOUNDS));
    });
    if (tps.reduce((n, p) => n + (finite(p?.size) ? p.size : 0), 0) >= 1) out.push(`${prefix}Take-profits: sizes must add up to less than 100%; the rest exits by the other rules.`);
    const be = side.exit.breakeven;
    if (be && !be.after) out.push(`${prefix}Breakeven: set the activation distance.`);
    const after = comparable(be?.after), plus = comparable(be?.plus);
    if (after && plus && after[0] === plus[0] && plus[1] >= after[1]) out.push(`${prefix}Breakeven: "plus" must be below "after".`);
    const maxValid = caps?.max_valid_bars ?? 20;
    if (side.entry.valid_bars !== undefined) {
      if (entryOrderType(side.entry) === "market") out.push(`${prefix}Order duration applies to limit and stop orders only.`);
      if (!bounded(side.entry.valid_bars, 1, maxValid, true)) out.push(`${prefix}Order duration must be a whole number of bars from 1 to ${maxValid}.`);
    }
    if (side.exit.min_bars !== undefined && !bounded(side.exit.min_bars, 0, 1000, true)) out.push(`${prefix}Minimum bars must be a whole number from 0 to 1000.`);
    const si = side.entry.scale_in;
    if (si) {
      const max = caps?.max_legs ?? 5;
      if (!Array.isArray(si.legs) || si.legs.length < 2 || si.legs.length > max || si.legs.some((n) => !bounded(n, Number.MIN_VALUE, 1)) || si.legs.reduce((a, b) => a + b, 0) > 1 + 1e-9)
        out.push(`${prefix}Scale-in: use 2 to ${max} legs above 0% that add up to at most 100%.`);
      if (si.min_bars !== undefined && !bounded(si.min_bars, 1, 1000, true)) out.push(`${prefix}Scale-in: bars between legs must be a whole number from 1 to 1000.`);
      if (si.price !== undefined && !["any", "better", "worse"].includes(si.price)) out.push(`${prefix}Scale-in: choose any, better or worse prices.`);
      if (tps.length) out.push(COPY.scaleInWithTp);
    }
  }
  const ex = spec.execution ?? {};
  if (!["long", "short", "both"].includes(directionOf(spec))) out.push("Choose Long, Short or Both directions.");
  for (const [key, values] of [
    ["fill", caps?.fill ?? ["next_open", "close"]], ["exit_fill", caps?.fill ?? ["next_open", "close"]],
    ["entry_on", caps?.entry_on ?? ["level", "edge"]], ["intrabar", caps?.intrabar ?? ["stop_first", "ohlc_path"]],
    ["opposite", caps?.opposite ?? ["reverse", "exit", "ignore"]],
  ] as const) if (ex[key] !== undefined && !values.includes(ex[key]!)) out.push(`Choose a supported ${key.replaceAll("_", " ")}.`);
  if (sides.some(([, s]) => entryOrderType(s.entry) !== "market") && ex.fill !== undefined)
    out.push("Remove the entry fill: limit and stop orders fill on the bars after the signal.");
  if (ex.delay_bars !== undefined) {
    if ((ex.fill ?? "next_open") !== "next_open" || (ex.exit_fill ?? ex.fill ?? "next_open") !== "next_open") out.push(COPY.delayOnlyNextOpen);
    if (!bounded(ex.delay_bars, 1, caps?.max_delay_bars ?? 10, true)) out.push(`Fill delay must be a whole number of bars from 1 to ${caps?.max_delay_bars ?? 10}.`);
  }
  if (ex.opposite !== undefined && directionOf(spec) !== "both") out.push("Opposite-signal behavior applies to Both directions only.");
  const z = ex.sizing;
  if (z !== undefined && (!z || typeof z !== "object" || Array.isArray(z))) out.push("Position size: choose one sizing method.");
  else if (z) {
    if (["pct_equity", "risk_pct", "units", "value"].filter((k) => k in z).length !== 1) out.push("Position size: choose exactly one sizing method.");
    if ("pct_equity" in z && !bounded(z.pct_equity, Number.MIN_VALUE, 100)) out.push("% of equity must be above 0 and at most 100.");
    if ("risk_pct" in z) {
      if (!bounded(z.risk_pct, Number.MIN_VALUE, 100)) out.push("Risk per trade must be above 0% and at most 100%.");
      if (sides.some(([, s]) => !s.exit.stop_loss)) out.push(COPY.riskNeedsStop);
    }
    if (("units" in z || "value" in z) && modeOf(spec) !== "money") out.push(COPY.needsMoney);
    if ("units" in z && !bounded(z.units, 1, 1e9, true)) out.push("Shares must be a whole number from 1 to 1,000,000,000.");
    if ("value" in z && !bounded(z.value, Number.MIN_VALUE, 1e12)) out.push("Position value must be above 0 and at most 1,000,000,000,000.");
  }
  if (modeOf(spec) === "money") {
    if (!supportsMoney(cat)) out.push("Money mode is not supported by this engine yet.");
    if (directionOf(spec) === "both" && (ex.opposite ?? "reverse") === "reverse" && sides.some(([, s]) => s.entry.limit !== undefined))
      out.push("Money mode cannot classify a resting limit reversal's fill costs; use unit mode or opposite Exit/Ignore.");
    const blocked = moneyBlocked(ctx, spec);
    if (blocked) out.push(blocked);
    if (!bounded(ex.capital, Number.MIN_VALUE, 1e12)) out.push("Capital must be above 0 and at most 1,000,000,000,000.");
    if (ex.cost_bps !== undefined) out.push("Money mode takes its costs from the money settings; remove the spec's fee (cost_bps).");
    const c = ex.costs ?? {};
    const checkCost = (n: unknown, label: string, hi: number, max: string) => {
      if (n !== undefined && !bounded(n, 0, hi)) out.push(`${label}: must be from 0 to ${max}.`);
    };
    checkCost(c.commission?.per_order, "Commission per order", 1e9, "1,000,000,000");
    checkCost(c.commission?.per_unit, "Commission per share", 1e6, "1,000,000");
    checkCost(c.commission?.pct, "Commission %", 10, "10");
    checkCost(c.slippage?.bps, "Slippage: bps", 1000, "1000");
    if (c.slippage && (typeof c.slippage !== "object" || "ticks" in c.slippage || !("bps" in c.slippage))) out.push("Slippage in ticks needs admitted futures contract data.");
    const taxes = Array.isArray(c.taxes) ? c.taxes : [];
    if (c.taxes !== undefined && !Array.isArray(c.taxes)) out.push("Taxes must be a list.");
    if (taxes.length > 6) out.push("Use at most 6 taxes.");
    taxes.forEach((t, i) => {
      if (!t || typeof t !== "object") { out.push(`Tax ${i + 1}: set a name, percentage and side.`); return; }
      if (typeof t.name !== "string" || !/^[A-Za-z0-9 _-]{1,40}$/.test(t.name)) out.push(`Tax ${i + 1}: the name must be 1 to 40 letters, digits, spaces, _ or -.`);
      if (!bounded(t.pct, 0, 10)) out.push(`Tax ${i + 1}: % must be from 0 to 10.`);
      if (!["buy", "sell", "both"].includes(t.side)) out.push(`Tax ${i + 1}: choose buy, sell or both.`);
    });
    if (ctx.market === "us" && hasShorts(spec) && c.borrow_pct_year === undefined) out.push(COPY.borrowRequired);
    if (c.borrow_pct_year !== undefined && !bounded(c.borrow_pct_year, 0, 100)) out.push("Borrow cost must be from 0% to 100% a year.");
  } else if (ex.costs !== undefined) out.push("Money costs apply in money mode only.");
  const cd = ex.cooldown;
  if (cd) {
    const max = caps?.max_cooldown_bars ?? 1000;
    if (!bounded(cd.bars, 1, max, true)) out.push(`Cooldown must be a whole number of bars from 1 to ${max}.`);
    if (cd.after !== undefined && !["any", "loss", "win"].includes(cd.after)) out.push("Cooldown: choose after any trade, a loss or a win.");
  }
  const mt = ex.max_trades;
  if (mt) {
    const max = caps?.max_trades_count ?? 1000;
    if (!bounded(mt.count, 1, max, true)) out.push(`Max entries must be a whole number from 1 to ${max}.`);
    if (!(caps?.max_trades_per ?? ["W", "M", "Q", "Y"]).includes(mt.per)) out.push("Max entries: choose week, month, quarter or year.");
  }
  const risk = ex.risk;
  if (risk) {
    const losses = risk.max_consecutive_losses, pause = risk.pause_bars;
    if ((losses !== undefined) !== (pause !== undefined)) out.push("Loss-streak pause needs both the number of losses and the pause in bars.");
    if ((losses !== undefined && !bounded(losses, 1, caps?.max_consecutive_losses ?? 1000, true)) ||
        (pause !== undefined && !bounded(pause, 1, caps?.max_pause_bars ?? 100000, true)))
      out.push(`Loss-streak pause: losses 1 to ${caps?.max_consecutive_losses ?? 1000}, pause 1 to ${caps?.max_pause_bars ?? 100000} bars.`);
    if (risk.max_drawdown_pct !== undefined && (!bounded(risk.max_drawdown_pct, Number.MIN_VALUE, caps?.max_drawdown_pct_below ?? 100) || risk.max_drawdown_pct >= (caps?.max_drawdown_pct_below ?? 100)))
      out.push("Drawdown stop must be above 0% and below 100%.");
  }
  if (getAt(spec, "execution.instrument.kind") === "futures") out.push(FUTURES_NOTE);
  return out;
}
