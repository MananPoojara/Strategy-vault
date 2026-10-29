/** What a new strategy starts with and what a test uses when the user leaves something out: an empty buy list (never a
 *  hidden example row), the default exit when no sell rule is on, first buy rules for an indicator, and the settings that
 *  can stop a trade from opening. */
import { DEFAULT_LEVELS } from "./Workbench";
import { candleCountNode, ordinal, setExecution } from "./spec-model";
import { entryOrderType } from "./execution-model";
import { INDICATOR_RULES } from "./indicator-rules";
import { BUILT_IN } from "./builtin-periods";
import { chosenPhase } from "./date-sets";
import type { Indicator, RuleNode, SeriesOperand, Spec } from "./types";

/** The exit used when a side has no sell rule switched on, for every indicator. Each part can be on or off: the Nth up close
 *  after entry, the Mth down close (a losing streak), N days in the trade, a stop-loss, a profit target and a trailing stop
 *  (% of the entry price). A short mirrors the closes (a down close is its profit). */
export interface DefaultExit { up: number; down: number; upOff?: boolean; downOff?: boolean; days?: number; stopPct?: number; targetPct?: number; trailPct?: number }
export const DEFAULT_EXIT: DefaultExit = { up: 1, down: 4 };
export const MAX_DEFAULT_COUNT = 100;
/** A new test starts on this date (the user can clear it to test all data). */
export const DEFAULT_FROM = "2000-01-01";
const KEY = "lab.defaultExit";
const count = (n: unknown) => typeof n === "number" && Number.isInteger(n) && n >= 1 && n <= MAX_DEFAULT_COUNT;
const pctOk = (n: unknown, max: number) => n === undefined || (typeof n === "number" && n > 0 && n <= max);
/** Valid numbers, and at least one part switched on. */
export function validDefaultExit(d: Partial<DefaultExit> | null | undefined): d is DefaultExit {
  return !!d && count(d.up) && count(d.down) && (d.days === undefined || (Number.isInteger(d.days) && d.days >= 1 && d.days <= 1000))
    && pctOk(d.stopPct, 99) && pctOk(d.targetPct, 1000) && pctOk(d.trailPct, 99)
    && (!d.upOff || !d.downOff || !!d.days || !!d.stopPct || !!d.targetPct || !!d.trailPct);
}
export function loadDefaultExit(): DefaultExit {
  try { const d = JSON.parse(localStorage.getItem(KEY) ?? "null") as Partial<DefaultExit> | null; if (validDefaultExit(d)) return { ...d }; } catch { /* private window or old value */ }
  return DEFAULT_EXIT;
}
export function storeDefaultExit(d: DefaultExit) {
  try { localStorage.setItem(KEY, JSON.stringify(d)); } catch { /* private window: kept for this visit */ }
}
export function defaultExitNodes(d: DefaultExit, short: boolean): RuleNode[] {
  const [win, lose] = short ? (["down", "up"] as const) : (["up", "down"] as const);
  return [...(d.upOff ? [] : [candleCountNode(win, d.up)]), ...(d.downOff ? [] : [candleCountNode(lose, d.down)]),
    ...(d.days ? [{ lhs: { s: "bars_held" }, op: ">=", rhs: { value: d.days } } as RuleNode] : [])];
}
/** The default exit's stops, as the exit's own stop-loss, target and trailing (a % distance; the engine mirrors a short). */
const defaultExitStops = (d: DefaultExit) => ({ ...(d.stopPct ? { stop_loss: { pct: d.stopPct } } : {}), ...(d.targetPct ? { target: { pct: d.targetPct } } : {}),
  ...(d.trailPct ? { trailing: { pct: d.trailPct } } : {}) });
export const defaultExitParts = (d: DefaultExit, short = false): string[] => [
  ...(d.upOff ? [] : [`at the ${ordinal(d.up)} ${short ? "down" : "up"} close after entry`]), ...(d.downOff ? [] : [`at the ${ordinal(d.down)} ${short ? "up" : "down"} close`]),
  ...(d.days ? [`after ${d.days} day${d.days === 1 ? "" : "s"} in the trade`] : []), ...(d.stopPct ? [`at a ${d.stopPct}% stop-loss`] : []),
  ...(d.targetPct ? [`at a ${d.targetPct}% profit target`] : []), ...(d.trailPct ? [`with a ${d.trailPct}% trailing stop`] : [])];
export const defaultExitWords = (d: DefaultExit, short = false) => `${short ? "cover" : "sell"} ${defaultExitParts(d, short).join(", or ")}`;

type Obj = Record<string, unknown>;
const on = (n: RuleNode) => (n as Obj).off !== true;
const PRICE_EXITS = ["stop_loss", "target", "trailing", "breakeven", "dual_atr", "dual_line"];
/** Long and short both, reversing, no exit rule anywhere: always in the market, each signal flips the position (specProblems). */
export const alwaysIn = (spec: Spec) => spec.direction === "both" && (spec.execution?.opposite ?? "reverse") === "reverse" && !spec.exit.any.length && !spec.short_side?.exit.any.length;
/** Whether a side (the main rules, or short_side) trades with the default exit: no sell rule is on and nothing else closes a
 *  trade. One instrument only: a portfolio cannot read since-entry values (portfolio-model.ts), so it keeps its own exits. */
export function usesDefaultExit(spec: Spec, shortSide = false): boolean {
  const side = shortSide ? spec.short_side : spec;
  if (!side || spec.portfolio || alwaysIn(spec)) return false;
  const exit = side.exit as unknown as Obj;
  if ((side.exit.any ?? []).some(on) || PRICE_EXITS.some((k) => exit[k] !== undefined) || (side.exit.take_profits?.length ?? 0) > 0) return false;
  return !side.entry.all.some((n) => (n as Obj).filter_exit === true && on(n));
}
/** The spec a test runs: each side without a sell rule gets the default exit; otherwise the same object. */
export function withDefaultExit(spec: Spec, d: DefaultExit): Spec {
  const main = usesDefaultExit(spec), other = usesDefaultExit(spec, true);
  let out = spec;
  if (main) out = { ...out, exit: { ...out.exit, ...defaultExitStops(d), any: [...(out.exit.any ?? []), ...defaultExitNodes(d, spec.direction === "short")] } };
  if (other && out.short_side) out = { ...out, short_side: { ...out.short_side, exit: { ...out.short_side.exit, ...defaultExitStops(d), any: [...(out.short_side.exit.any ?? []), ...defaultExitNodes(d, true)] } } };
  return out;
}

/** The stored rule with the user's parameter values on every operand of indicator `id` (stored rules carry none). */
function withParams(node: RuleNode, id: string, p: Record<string, number>): RuleNode {
  if (!Object.keys(p).length) return structuredClone(node);
  const walk = (x: unknown): unknown => Array.isArray(x) ? x.map(walk)
    : x && typeof x === "object" ? { ...Object.fromEntries(Object.entries(x as Obj).map(([k, v]) => [k, walk(v)])), ...((x as Obj).s === id ? { p } : {}) } : x;
  return walk(node) as RuleNode;
}
/** The indicator's own exit (TradingView style) with the user's parameters; undefined when it has none or for a short. */
export const ownExit = (ind: Pick<Indicator, "id">, p: Record<string, number>, short = false): RuleNode | undefined => {
  const exit = short ? undefined : INDICATOR_RULES[ind.id]?.exit;
  return exit ? withParams(exit, ind.id, p) : undefined;
};
/** A first buy (or short-sale) rule for an indicator: its TradingView-style rule from the table, else the way analysts
 *  usually read it: a line crossing its signal line,
 *  an oscillator past its low (high) level, the close crossing a price line (a channel's previous top or bottom), else
 *  the indicator rising (falling) for 3 bars. */
export function firstRule(ind: Indicator, p: Record<string, number>, short = false): RuleNode {
  const own = short ? undefined : INDICATOR_RULES[ind.id];
  if (own) return withParams(own.entry, ind.id, p);
  const o: SeriesOperand = { s: ind.id, ...(Object.keys(p).length ? { p } : {}) };
  const signal = ind.outputs.find((x) => x === "signal" || x === "trigger");
  if (signal && ind.outputs[0] !== signal) return { lhs: o, op: short ? "crosses_below" : "crosses_above", rhs: { ...o, out: signal } };
  const levels = DEFAULT_LEVELS[ind.id];
  if (levels?.length) return { lhs: o, op: short ? ">" : "<", rhs: { value: short ? Math.max(...levels) : Math.min(...levels) } };
  if (ind.placement === "price") {
    // a channel of highs and lows contains today's close: a breakout compares with the previous bar's value
    const prior = ["donchian", "highest", "lowest"].includes(ind.id);
    const rhs: SeriesOperand = prior ? { ...o, ...(ind.outputs.includes("upper") ? { out: short ? "lower" : "upper" } : {}), offset: 1 } : o;
    return { lhs: { s: "close" }, op: short ? "crosses_below" : "crosses_above", rhs };
  }
  return { lhs: o, op: short ? "falling" : "rising", bars: 3 } as RuleNode;
}

/** The price exits that are on, with their size: "stop-loss 5%", "N-bar stop 10 bars". */
export function stopsOn(exit: Record<string, unknown>, nbar: number | undefined): string[] {
  const size = (d: unknown) => { const x = (d ?? {}) as Record<string, number>; return x.pct !== undefined ? ` ${x.pct}%` : x.points !== undefined ? ` ${x.points} points` : x.atr !== undefined ? ` ${x.atr} × ATR` : ""; };
  const out = ([["stop_loss", "stop-loss"], ["target", "target"], ["trailing", "trailing stop"]] as const).filter(([k]) => exit[k] !== undefined).map(([k, name]) => name + size(exit[k]));
  if (nbar !== undefined) out.push(`N-bar stop ${nbar} bars`);
  // a two-line Supertrend reads as its periods and multipliers, "Supertrend (5, 1 → 2) stop"
  const dl = exit.dual_line as { initial?: { s?: string; p?: Record<string, number> }; favour?: { p?: Record<string, number> } } | undefined;
  const TFW: Record<string, string> = { W: " weekly", M: " monthly" };
  const stLine = dl?.initial?.s === "supertrend" ? `Supertrend (${dl.initial.p?.atr_length ?? 10}, ${dl.initial.p?.factor ?? 3} → ${dl.favour?.p?.factor ?? "?"})${TFW[(dl.initial as { tf?: string }).tf ?? ""] ?? ""} stop` : "two-line exit";
  // the Dual ATR stop with its numbers and timeframe, "ATR (5, 2) (5, 4) weekly", so a tradesheet says which ATR ran
  const da = exit.dual_atr as { initial?: { period?: number; mult?: number }; favour?: { period?: number; mult?: number }; tf?: string } | undefined;
  const atrName = da ? `ATR (${da.initial?.period}, ${da.initial?.mult}) (${da.favour?.period}, ${da.favour?.mult})${TFW[da.tf ?? ""] ?? ""}` : "two ATR stops";
  for (const [k, name] of [["breakeven", "breakeven"], ["take_profits", "take-profits"], ["min_bars", "minimum bars"], ["dual_atr", atrName], ["dual_line", stLine]] as const)
    if (exit[k] !== undefined && !(Array.isArray(exit[k]) && !(exit[k] as unknown[]).length)) out.push(name);
  return out;
}

/** The More settings that differ from their defaults, by name ("cooldown", "date filter"), for the folded box's heading.
 *  Stops, targets and the entry order are set with the rules in Your idea, so they are not listed here. */
export function changedSettings(spec: Spec): string[] {
  const ex = (spec.execution ?? {}) as Record<string, unknown>, out: string[] = [];
  const named: [string, unknown][] = [["position size", ex.sizing], ["cooldown", ex.cooldown], ["trades per period", ex.max_trades], ["loss-streak pause or drawdown stop", ex.risk],
    ["trade after a losing streak", ex.shadow], ["trade prices", ex.prices], ["delays", ex.delays ?? ex.delay_bars], ["re-entry delays", ex.reentry], ["position limits", ex.limits],
    ["stop exit modes", ex.exit_at_stop], ["stops from the next bar", ex.activate_stops_immediately === false || undefined], ["no same-bar exit and entry", ex.allow_same_bar_exit === false || undefined],
    ["re-enter on the exit bar", ex.reenter_on_exit_bar], ["hold the exit on a circuit close", ex.defer_exit_on_circuit], ["tick size", ex.tick_size || undefined],
    ["pad and align", ex.pad_align], ["report rates", ex.report], ["entry on", ex.entry_on], ["same-bar order", ex.intrabar], ["costs (cash)", ex.costs],
    ["portfolio", spec.portfolio], ["date filter", spec.date_filter && !spec.date_filter.off ? spec.date_filter : undefined]];
  for (const [name, v] of named) if (v !== undefined && v !== false) out.push(name);
  return out;
}

/** A setting that can keep trades from opening (or close them early), with how to switch it off where that is one step. */
export interface Blocker { id: string; text: string; off?: (spec: Spec) => Spec }
export function tradeBlockers(spec: Spec, period?: { from?: string; to?: string; latest?: string }): Blocker[] {
  const out: Blocker[] = [], ex = spec.execution ?? {}, df = spec.date_filter;
  const filters = spec.entry.all.filter((n) => (n as Obj).filter === true && on(n)).length;
  if (filters) out.push({ id: "filters", text: `${filters} filter${filters === 1 ? "" : "s"}: a buy also needs ${filters === 1 ? "it" : "each of them"} to be true`,
    off: (s) => ({ ...s, entry: { ...s.entry, all: s.entry.all.map((n) => (n as Obj).filter === true ? { ...n, off: true } as RuleNode : n) } }) });
  if (df && !df.off) {
    const built = BUILT_IN.find((b) => b.name === df.set?.name && df.set?.id === undefined);
    const phase = df.mode === "outside" ? "outside its" : built ? `its ${(chosenPhase(df.windows, built.windows) ?? "Bull").toLowerCase()}` : "its chosen";
    out.push({ id: "date_filter", text: `Dates${df.set?.name ? ` “${df.set.name}”` : ""}: trades only in ${phase} periods`, off: (s) => { const { date_filter: _, ...rest } = s; return rest as Spec; } });
  }
  if (ex.shadow) out.push({ id: "shadow", text: `Trade after a losing streak: real trades start only after the paper trades lose ${ex.shadow.loss_pct}%`, off: (s) => setExecution(s, { shadow: undefined }) });
  if (ex.risk?.max_consecutive_losses) out.push({ id: "loss_pause", text: `Loss-streak pause: no new trade for ${ex.risk.pause_bars} bars after ${ex.risk.max_consecutive_losses} losses in a row`,
    off: (s) => setExecution(s, { risk: dropKeys(s.execution?.risk, "max_consecutive_losses", "pause_bars") }) });
  if (ex.risk?.max_drawdown_pct) out.push({ id: "dd_stop", text: `Drawdown stop: no new trade after a ${ex.risk.max_drawdown_pct}% drawdown`, off: (s) => setExecution(s, { risk: dropKeys(s.execution?.risk, "max_drawdown_pct") }) });
  if (ex.cooldown) out.push({ id: "cooldown", text: `Cooldown: waits ${ex.cooldown.bars} bars after ${ex.cooldown.after === "loss" ? "a losing" : ex.cooldown.after === "win" ? "a winning" : "every"} trade`, off: (s) => setExecution(s, { cooldown: undefined }) });
  if (ex.max_trades) out.push({ id: "max_trades", text: `At most ${ex.max_trades.count} trade${ex.max_trades.count === 1 ? "" : "s"} per ${{ W: "week", M: "month", Q: "quarter", Y: "year" }[ex.max_trades.per]}`, off: (s) => setExecution(s, { max_trades: undefined }) });
  if (ex.delay_bars) out.push({ id: "delay", text: `Orders wait ${ex.delay_bars} bar${ex.delay_bars === 1 ? "" : "s"} after the signal`, off: (s) => setExecution(s, { delay_bars: undefined }) });
  const order = entryOrderType(spec.entry);
  if (order !== "market") out.push({ id: "entry_order", text: `Entry by ${order.replace("_", "-")} order: a trade opens only if the price reaches the order level` });
  // the default period (01-01-2000 to the latest data) is not a blocker; a period the user narrowed is
  if ((period?.from && period.from !== DEFAULT_FROM) || (period?.to && period.to !== period.latest)) out.push({ id: "period", text: `Test period ${period.from || "first bar"} → ${period.to || "last bar"} (Settings)` });
  return out;
}
const dropKeys = <T extends object>(o: T | undefined, ...keys: string[]): T | undefined => {
  if (!o) return undefined;
  const rest = Object.fromEntries(Object.entries(o).filter(([k]) => !keys.includes(k))) as T;
  return Object.keys(rest).length ? rest : undefined;
};
