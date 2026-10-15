import type { Indicator, Market, Timeframe } from "./types";

export const MAX_PRICE_OVERLAYS = 4;
export const MAX_PANES = 3;

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
export function availability(ind: Indicator, { market, hasVolume, tf }: { market: Market; hasVolume: boolean; tf: Timeframe }): { ok: boolean; reason?: string } {
  const n = ind.needs;
  if (!ind.computed) return { ok: false, reason: ind.note || "Listed only; the engine does not compute it yet." };
  if (n.volume && !hasVolume) return { ok: false, reason: "Needs volume; this instrument has no volume in its data." };
  if (n.breadth && (market === "gl" || tf !== "D")) return { ok: false, reason: "Breadth data exists only for India and US daily bars." };
  if (n.intraday || n.second_symbol || n.fundamentals || n.derivatives || n.onchain)
    return { ok: false, reason: ind.note || "Needs data Strategy Vault does not have yet." };
  return { ok: true };
}
