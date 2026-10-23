import { isPair } from "./foreign-model";
import type { Indicator, Market, Timeframe } from "./types";

export const MAX_PRICE_OVERLAYS = 4;
/** AmiBroker's own defaults as SV parameters (AFL function signatures; the @AB columns of AllIndicators_AB_vs_SV_Exploration.afl):
 *  only the keys that differ from SV's defaults or pick the AmiBroker variant. Supertrend mode 1 is the user's AmiBroker AFL. */
export const AB_DEFAULTS: Record<string, Record<string, number>> = {
  rsi: { length: 14, variant: 2 }, stoch: { k: 14, smooth_k: 3, d: 3, variant: 0 }, cci: { length: 14 }, roc: { length: 12, variant: 0 },
  trix: { length: 9, variant: 1 }, dpo: { length: 20, variant: 0 }, bb: { length: 15, mult: 2, source: 0 },
  kc: { length: 15, mult: 2, atr_length: 15, basis_type: 1, source: 0, range: 0 }, envelope: { length: 15, percent: 2, exponential: 0 },
  mass_index: { ema: 9, length: 25 }, chaikin_osc: { fast: 9, slow: 14 }, asi: { limit: 100 }, swing_index: { limit: 100 }, tvi: { min_tick: 0.1 },
  beta: { length: 21, returns: 0 }, price_osc: { fast: 9, slow: 18, exponential: 1 }, apo: { fast: 9, slow: 18, exponential: 1 },
  eom: { variant: 1 }, percentrank: { variant: 1 }, supertrend: { mode: 1 },
};
export const MAX_PANES = 3;
/** Short ids read as the familiar abbreviation (SMA, RSI, MACD); longer ids keep the full name. */
export const shortName = (ind: Indicator) => (/^[a-z0-9]{2,6}$/.test(ind.id) ? ind.id.toUpperCase() : ind.name);

/** Matches name, id, TradingView and AmiBroker names; an empty query returns the list unchanged. */
export function searchIndicators(list: Indicator[], query: string): Indicator[] {
  const q = query.trim().toLowerCase();
  if (!q) return list;
  return list.filter((i) => [i.name, i.id, i.tv, i.ab].some((s) => s.toLowerCase().includes(q)));
}

/** Groups in catalogue (first-appearance) order. */
export function groupByCategory(list: Indicator[]): [string, Indicator[]][] {
  const groups = new Map<string, Indicator[]>();
  for (const i of list) groups.set(i.category, [...(groups.get(i.category) ?? []), i]);
  return [...groups];
}

/** First matching rule wins (plan Task 7). */
export function availability(ind: Indicator, { market, hasVolume, tf, maxForeign }: { market: Market; hasVolume: boolean; tf: Timeframe; maxForeign?: number }): { ok: boolean; reason?: string } {
  const n = ind.needs;
  if (!ind.computed) return { ok: false, reason: ind.note || "Listed only; the engine does not compute it yet." };
  if (n.volume && !hasVolume) return { ok: false, reason: "Needs volume; this instrument has no volume in its data." };
  if (n.breadth && (market === "gl" || tf !== "D")) return { ok: false, reason: "Breadth data exists only for India and US daily bars." };
  if (n.second_symbol) {
    if (!isPair(ind.id) || !Number.isInteger(maxForeign) || !(maxForeign! > 0)) return { ok: false, reason: ind.note || "Comparison instruments are not advertised by this engine yet." };
    if (tf !== "D") return { ok: false, reason: "Pair indicators use daily (D) values only." };
    return { ok: true, reason: "Choose a comparison instrument on the chart." };
  }
  if (n.intraday || n.fundamentals || n.derivatives || n.onchain)
    return { ok: false, reason: ind.note || "Needs data Strategy Vault does not have yet." };
  return { ok: true };
}
