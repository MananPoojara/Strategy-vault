import {sourceAccountCopy,type SourceAccount} from "./source-account";
import {unitExposureDisplay} from './unit-exposure-presenter';
import type {UnitExposureView} from './unit-exposure-report';
export type ReturnDisplayMode = "fixed" | "compound";

export interface ReturnDisplay {
  mode: ReturnDisplayMode;
  label: string;
  /** Fractional return; percentage formatting belongs to the presenter. */
  value: number | null;
  closedTrades: number | null;
  description: string;
  reportMode: string;
  unavailableReason: string | null;
}

/** Read the existing net, position-weighted trade returns without changing
 * execution, sizing, other statistics or open-position equity marks. */
export function returnDisplay(data: unknown, mode: ReturnDisplayMode = "fixed", account: SourceAccount | null = null, unit: UnitExposureView | null = null): ReturnDisplay {
  if (unit) return unitExposureDisplay(data,mode,unit);
  const result: ReturnDisplay = {
    mode,
    label: "Net Profit",
    value: null,
    closedTrades: null,
    description: mode === "compound"
      ? "Product of net closed-trade return factors. Open-position marks are excluded. No share quantity supplied."
      : "Additive trade-return view. No share quantity supplied. Open-position marks are excluded.",
    reportMode: mode === "compound" ? "Compounded closed-trade returns" : "Additive closed-trade returns",
    unavailableReason: null,
  };
  if (data && typeof data === 'object' && (Object.hasOwn(data,'unit_exposure_report')
    || ((data as {id?:unknown}).id==='hbqts_dema16_8' && (data as {book?:unknown}).book==='hbqts')))
    return {...result,unavailableReason:'A validated unit-exposure report is required; legacy returns cannot replace it.'};
  if (account) {
    result.label = mode === "compound" ? "Closed compounding" : "Closed return sum";
    result.reportMode = mode === "compound" ? "Hypothetical · weighted closed returns" : "Sum · weighted closed returns";
    result.description = (mode === "compound"
      ? "Hypothetical product of accepted closed net fractions of actual entry equity."
      : "Sum of accepted closed net fractions of actual entry equity; equals unrounded average on that same basis × closed trades.")
      + sourceAccountCopy(account).displayNote;
  }
  const unavailable = (reason: string): ReturnDisplay => ({...result, unavailableReason: reason});
  // Lane E6: a run over the engine's trade cap arrives summary-only (trades_page); its trade list is not here, so the
  // additive view is the engine's own closed-trade return sum, and the compounded view (not computed by the engine) is withheld.
  const page = data && typeof data === "object" ? (data as {trades_page?: unknown}).trades_page : undefined;
  if (page !== undefined && !account) {
    const stats = (data as {stats?: {net_profit?: unknown; trades?: unknown}}).stats;
    if (mode === "compound") return unavailable("This run's trades load page by page; the compounded closed-trade view needs every trade at once.");
    if (typeof stats?.net_profit !== "number" || !Number.isFinite(stats.net_profit) || typeof stats.trades !== "number")
      return unavailable("Closed-trade returns are unavailable.");
    return {...result, value: stats.net_profit, closedTrades: stats.trades, description: result.description + " Summed by the engine over every closed trade (six decimals)."};
  }
  if (!data || typeof data !== "object" || !Array.isArray((data as {trades?: unknown}).trades)) {
    return unavailable("Closed-trade returns are unavailable.");
  }
  const trades = (data as {trades: unknown[]}).trades;
  const closed: Record<string, unknown>[] = [];
  for (const trade of trades) {
    if (!trade || typeof trade !== "object" || Array.isArray(trade)
      || typeof (trade as {open?: unknown}).open !== "boolean") {
      return unavailable("Trade closure status is unavailable; the closed-trade display cannot be calculated.");
    }
    if (!(trade as {open: boolean}).open) closed.push(trade as Record<string, unknown>);
  }
  result.closedTrades = closed.length;
  let value = mode === "compound" ? 1 : 0;
  for (const trade of closed) {
    // Current engines expose an unrounded reporting value alongside the legacy
    // six-decimal field. Older saved ledgers retain their recorded precision.
    const ret = Object.hasOwn(trade, "ret_full_precision") ? trade.ret_full_precision : trade.ret;
    if (typeof ret !== "number" || !Number.isFinite(ret)) {
      return unavailable("A closed trade has no finite net return; the display cannot be calculated.");
    }
    if (mode === "compound") {
      const factor = 1 + ret;
      if (factor <= 0) return unavailable("Compounding requires a positive return factor for every closed trade.");
      value *= factor;
      if (!Number.isFinite(value) || value <= 0) {
        return unavailable("The compounded return is outside finite positive arithmetic bounds.");
      }
    } else {
      value += ret;
      if (!Number.isFinite(value)) return unavailable("The additive return is outside finite arithmetic bounds.");
    }
  }
  result.value = mode === "compound" ? value - 1 : value;
  return result;
}
