// The YT run as the Indicators lab's results components read it (the lab's RunResult / LabReport, trimmed to what YT has).

export interface ReportGroup { count: number; pct: number | null; total: number; profit: number; avg: number | null; avg_bars: number | null;
  largest: number | null; bars_in_largest: number | null; max_consecutive: number }
export interface ReportSide { trades: number; avg: number | null; avg_bars: number | null; profit_factor: number | null; payoff: number | null; winners: ReportGroup; losers: ReportGroup }
export interface LabReport {
  all: ReportSide; long: ReportSide; short: ReportSide; buyhold?: ReportSide;
  system: Record<string, number | null>; buy_hold?: Record<string, number | null>;
  rates?: { sharpe: number; upi: number }; notes: string[];
}
export interface Trade {
  entry: string; exit: string; entry_px: number; exit_px: number; ret: number; bars: number; open: boolean; dir: number; reason: string;
  mae?: number | null; mfe?: number | null;
}
/** One row of the whole-unit account per trade (the lab's money ledger). */
export interface LedgerRow { entry: string; qty: number; notional: number; costs: number; pnl: number }
export interface MoneySummary { capital: number; final_equity: number; costs_total: number; currency?: string }

export interface YtRunResult {
  from: string; to: string; dates: string[]; equity: number[]; bh: number[];
  trades: Trade[]; ledger?: LedgerRow[]; money?: MoneySummary; report?: LabReport;
  /** The engine's headline stats (closed trades, 365.25-day years): the tradesheet summaries' Sharpe and Buy & hold CAGR. */
  stats?: Record<string, number | null>;
  instrument: { ticker: string; name?: string };
  /** The page's Rollover setting, % per 30 days held: the tradesheet's D1 (long, +x%) and E1 (short, -x%). */
  rollover?: number;
}

/** What a tradesheet / report needs to say about the strategy (the Rules sheet). */
export interface YtRules {
  strategy: string; position: "Long" | "Short" | "Long & Short"; filter: string; setup: string; entry: string[]; exit: string[];
  /** Settings as shown on the page: [name, value]. */
  settings: [string, string][];
  /** Tab name: index code, entry and exit ("NF_En_Monday Reversal_EX_ATR"). */
  tab: string;
}

/** The engine's YT run JSON, as much of it as the adapter reads. */
interface EngineTrade {
  dir: number; legs: [number, number, number][]; entry: string; exit: string; exit_px: number; ret: number; bars: number; open: boolean; reason: string;
  accounting?: { mae_ret_observed?: number | null; mfe_ret_observed?: number | null };
  units?: number; position_value?: number; pnl?: number; costs_paid?: number;
}
export interface EngineRun { dates: string[]; equity: number[]; bh: number[]; trades: EngineTrade[]; report?: LabReport; money?: MoneySummary; stats?: Record<string, number | null> }

/** The engine's YT run as the lab's RunResult: trade list with AmiBroker MAE/MFE, the whole-unit ledger and the report. */
export function ytRunResult(d: EngineRun, ticker: string, name?: string, rollover?: number): YtRunResult {
  const trades: Trade[] = d.trades.map((t) => ({
    entry: t.entry, exit: t.exit, entry_px: t.legs[0][1], exit_px: t.exit_px, ret: t.ret, bars: t.bars, open: t.open, dir: t.dir, reason: t.reason,
    mae: t.accounting?.mae_ret_observed ?? null, mfe: t.accounting?.mfe_ret_observed ?? null,
  }));
  const ledger = d.money && d.trades.every((t) => typeof t.pnl === "number")
    ? d.trades.map((t) => ({ entry: t.entry, qty: t.units ?? 0, notional: t.position_value ?? 0, costs: t.costs_paid ?? 0, pnl: t.pnl ?? 0 })) : undefined;
  return { from: d.dates[0], to: d.dates[d.dates.length - 1], dates: d.dates, equity: d.equity, bh: d.bh, trades, ledger, money: d.money, report: d.report, stats: d.stats,
    instrument: { ticker, name }, ...(rollover === undefined ? {} : { rollover }) };
}
