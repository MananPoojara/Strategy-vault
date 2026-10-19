/** AmiBroker-style Monte Carlo over a closed-trade list: resample with replacement, rebuild equity from 1.0. Pure and seeded. */

import type { LedgerRow, Trade } from "./types";

export const PERCENTILES = [1, 5, 10, 25, 50, 75, 90, 95, 99] as const;
export const MAX_RUNS = 5000;
/** runs × trades bound: keeps the work and memory of one Monte Carlo small enough for the browser's main thread. */
export const MAX_WORK = 5_000_000;
/** Envelope columns kept (the chart draws at most this many points). */
export const ENVELOPE_POINTS = 200;

/** "fixed": rets are each trade's P&L in units of the initial equity, added (not compounded): AmiBroker fixed size / fixed value. */
export type Sizing = { kind: "none" } | { kind: "fraction"; pct: number } | { kind: "fixed" };
export interface McInput { rets: number[]; runs: number; sizing: Sizing; seed: number; years: number | null }
/** maxDd and maxDdAbs ≤ 0 (AmiBroker sign): the 1% row is the worst drawdown. maxDdAbs is in units of the initial equity. */
export interface McPath { final: number; cagr: number | null; maxDd: number; maxDdAbs: number; lowest: number }
export interface McRow { p: number; final: number; cagr: number | null; maxDd: number; maxDdAbs: number; lowest: number }
export interface McResult {
  /** runs actually made; below the requested runs when `capped` (runs × trades > MAX_WORK). */
  runs: number; trades: number; rows: McRow[]; capped: boolean;
  /** Sorted ascending, one value per run (the percentile charts). */
  finals: number[]; drawdowns: number[]; drawdownsAbs: number[]; lowests: number[];
  /** Equity after trade at[j] (0 is the start, 1.0) across runs, at up to ENVELOPE_POINTS trade numbers. */
  envelope: { at: number[]; min: number[]; median: number[]; max: number[] };
}

/** mulberry32: small deterministic PRNG returning [0, 1). */
export function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Linear-interpolated percentile (p in 0..100) of an ascending array. */
export function percentile(sorted: ArrayLike<number>, p: number): number {
  const n = sorted.length;
  if (!n) return NaN;
  const h = (n - 1) * (p / 100), lo = Math.floor(h), hi = Math.min(n - 1, lo + 1);
  return sorted[lo] + (h - lo) * (sorted[hi] - sorted[lo]);
}

/** Years between two ISO dates (backtest span), null when unknown or empty. */
export function spanYears(from: string, to: string): number | null {
  const y = (Date.parse(to) - Date.parse(from)) / (365.25 * 86400e3);
  return Number.isFinite(y) && y > 0 ? y : null;
}

/**
 * The per-trade equity returns to resample (closed trades only). Unit mode: the trade's return, which is already its
 * effect on equity. Money mode (ledger aligned with the trades): P&L over the cash equity before the trade, so a trade
 * that used 1% of equity moves equity by about 1% of its return, as in the backtest.
 */
export function tradeReturns(trades: Trade[], ledger?: LedgerRow[], capital?: number): number[] {
  const money = !!capital && !!ledger && ledger.length === trades.length && ledger.every((r, i) => r.entry === trades[i].entry);
  let equity = capital ?? 1;
  return trades.flatMap((t, i) => {
    if (t.open || typeof t.ret !== "number" || !Number.isFinite(t.ret)) return [];
    if (!money) return [t.ret];
    const r = ledger![i].pnl / equity;
    equity += ledger![i].pnl;
    return Number.isFinite(r) ? [r] : [];
  });
}

/**
 * AmiBroker Monte Carlo "Fixed size" (shares per trade) and "Fixed value" (money per trade): each closed trade's P&L over
 * `capital`, as shares × entry price × trade % or value × trade %, so the trade % (costs included) drives the result.
 */
export function fixedReturns(trades: Trade[], mode: "shares" | "value", amount: number, capital: number): number[] {
  return trades.flatMap((t) => {
    if (t.open || typeof t.ret !== "number" || !Number.isFinite(t.ret)) return [];
    const size = mode === "value" ? amount : typeof t.entry_px === "number" ? amount * t.entry_px : NaN, r = (size * t.ret) / capital;
    return Number.isFinite(r) ? [r] : [];
  });
}

/** "Don't change" compounds each trade % as-is; fixed fraction applies the trade % to that share of equity; fixed adds each P&L. Equity floors at 0 (ruin). */
export function equityPath(rets: ArrayLike<number>, sizing: Sizing, years: number | null = null, out?: Float64Array): McPath {
  const f = sizing.kind === "fraction" ? sizing.pct / 100 : 1;
  let eq = 1, peak = 1, maxDd = 0, maxDdAbs = 0, lowest = 1;
  if (out) out[0] = 1;
  for (let i = 0; i < rets.length; i++) {
    if (eq > 0) eq = Math.max(0, sizing.kind === "fixed" ? eq + rets[i] : eq * (1 + f * rets[i])); // ruined equity takes no more trades
    if (out) out[i + 1] = eq;
    if (eq > peak) peak = eq;
    else {
      if ((eq - peak) / peak < maxDd) maxDd = (eq - peak) / peak;
      if (eq - peak < maxDdAbs) maxDdAbs = eq - peak;
    }
    if (eq < lowest) lowest = eq;
  }
  return { final: eq, cagr: years ? (eq > 0 ? eq ** (1 / years) - 1 : -1) : null, maxDd, maxDdAbs, lowest };
}

export function runMonteCarlo({ rets, runs, sizing, seed, years }: McInput): McResult {
  const n = rets.length, asked = Math.max(1, Math.min(MAX_RUNS, Math.floor(runs)));
  const R = Math.max(1, Math.min(asked, Math.floor(MAX_WORK / Math.max(1, n))));
  const rand = mulberry32(seed), sample = new Float64Array(n), path = new Float64Array(n + 1);
  // only the sampled trade numbers are kept: ENVELOPE_POINTS columns, not n + 1
  const at = n + 1 <= ENVELOPE_POINTS ? Array.from({ length: n + 1 }, (_, k) => k) : Array.from({ length: ENVELOPE_POINTS }, (_, j) => Math.round((j * n) / (ENVELOPE_POINTS - 1)));
  const cols = at.map(() => new Float64Array(R));
  const finals = new Float64Array(R), dds = new Float64Array(R), ddAbs = new Float64Array(R), lows = new Float64Array(R), cagrs = new Float64Array(R);
  for (let r = 0; r < R; r++) {
    for (let i = 0; i < n; i++) sample[i] = rets[Math.floor(rand() * n)];
    const res = equityPath(sample, sizing, years, path);
    at.forEach((k, j) => { cols[j][r] = path[k]; });
    finals[r] = res.final; dds[r] = res.maxDd; ddAbs[r] = res.maxDdAbs; lows[r] = res.lowest; cagrs[r] = res.cagr ?? NaN;
  }
  [finals, dds, ddAbs, lows, cagrs].forEach((a) => a.sort());
  const min: number[] = [], median: number[] = [], max: number[] = [];
  for (const col of cols) { col.sort(); min.push(col[0]); median.push(percentile(col, 50)); max.push(col[R - 1]); }
  // Each column is its own distribution (AmiBroker's Monte Carlo table), so a row is not one run.
  const rows = PERCENTILES.map((p) => ({ p, final: percentile(finals, p), cagr: years ? percentile(cagrs, p) : null, maxDd: percentile(dds, p), maxDdAbs: percentile(ddAbs, p), lowest: percentile(lows, p) }));
  return { runs: R, trades: n, rows, capped: R < asked, finals: [...finals], drawdowns: [...dds], drawdownsAbs: [...ddAbs], lowests: [...lows], envelope: { at, min, median, max } };
}
