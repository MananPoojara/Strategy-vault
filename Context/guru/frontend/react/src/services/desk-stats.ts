// Strategy-report figures named and ordered as the desk's Signals page (user 20 Nov 2026). Display only:
// engine statistics are shown as returned; the few page-level figures (MAE sums, monthly ratios, drawdown periods,
// time in market) are derived from the engine's own trades, equity and dates, never from prices.
export type Kind = "percent" | "ratio" | "count" | "days" | "text";
type Stats = Record<string, unknown>;
interface Trade {open?: boolean; entry?: string; exit?: string; accounting?: {mae_ret_observed?: unknown; days_held?: unknown}}
export interface DeskRun {stats: Stats; gross_stats?: Stats; trades?: Trade[]; dates?: string[]; equity?: number[]}

const num = (v: unknown): number | null => typeof v === "number" && Number.isFinite(v) ? v : null;
const mean = (xs: number[]) => xs.length ? xs.reduce((a, b) => a + b, 0) / xs.length : null;
const ratio = (a: number | null, b: number | null) => a == null || b == null || b === 0 ? null : a / b;

export function maeFigures(trades: Trade[] = []) {
  const xs = trades.filter(t => !t.open).map(t => num(t.accounting?.mae_ret_observed)).filter((v): v is number => v !== null);
  return {avg: mean(xs), worst: xs.length ? Math.min(...xs) : null, total: xs.length ? xs.reduce((a, b) => a + b, 0) : null};
}

/** Distinct observed daily bars with a position, entry and exit bars included. */
export function barsInMarket(dates: string[] = [], trades: Trade[] = []): number | null {
  const ok = (x?: string) => /^\d{4}-\d{2}-\d{2}$/.test(x || "");
  if (!dates.length || trades.some(t => !ok(t.entry) || (!t.open && !ok(t.exit)))) return null;
  const last = dates[dates.length - 1];
  return dates.filter(d => trades.some(t => d >= t.entry! && d <= (t.open ? last : t.exit!))).length;
}

/** Strategy Metrics rows: [label, gross, net, kind]; a single-basis figure fills both cells. */
export function strategyMetricRows(d: DeskRun): Array<[string, number | string | null, number | string | null, Kind]> {
  const g = d.gross_stats || {}, n = d.stats || {}, both = (k: string) => [num(g[k]), num(n[k])] as const;
  const mae = maeFigures(d.trades), bars = barsInMarket(d.dates, d.trades);
  const gainPain = (s: Stats) => ratio(num(s.net_profit), mae.total == null ? null : Math.abs(mae.total));
  const row = (label: string, k: string, kind: Kind): [string, number | null, number | null, Kind] => [label, ...both(k), kind];
  return [
    row("Total Return %", "net_profit", "percent"),
    [ "Avg. Return %", num(g.avg_trade), num(n.avg_net_pnl ?? n.avg_trade), "percent"],
    row("Avg. Days Held", "avg_hold_days", "days"),
    row("Total Trades", "trades", "count"),
    row("Win %", "win_rate", "percent"),
    row("Win Avg. %", "avg_win", "percent"),
    row("Loss Avg. %", "avg_loss", "percent"),
    row("Max Win", "best", "percent"),
    row("Max Loss", "worst", "percent"),
    row("Expectancy", "expectancy_ratio", "ratio"),
    row("Monthly Thickness %", "monthly_thickness", "percent"),
    row("Risk Reward", "payoff", "ratio"),
    ["Gain / Pain", gainPain(g), gainPain(n), "ratio"],
    row("CAGR %", "cagr", "percent"),
    row("Max Drawdown %", "max_dd", "percent"),
    [ "CAR/MDD", num(g.car_mdd ?? g.mar), num(n.car_mdd ?? n.mar), "ratio"],
    ["Average Adverse Move", mae.avg, mae.avg, "percent"],
    ["Worst Adverse Move", mae.worst, mae.worst, "percent"],
    ["Total Adverse Move", mae.total, mae.total, "percent"],
    ["Time In Market", bars == null ? null : `${bars} bars (${(bars / 252).toFixed(1)} years)`, bars == null ? null : `${bars} bars (${(bars / 252).toFixed(1)} years)`, "text"],
  ];
}

/** Month-end returns of the engine's equity curve (first month measured from the first equity point). */
export function monthlyReturns(dates: string[] = [], equity: number[] = []): number[] {
  const ends: number[] = [];
  let base = num(equity[0]);
  for (let i = 0; i < dates.length; i++) if (i === dates.length - 1 || dates[i].slice(0, 7) !== dates[i + 1].slice(0, 7)) {
    const e = num(equity[i]);
    if (e != null && base) ends.push(e / base - 1);
    if (e != null) base = e;
  }
  return ends;
}

const quantile = (sorted: number[], q: number) => {
  const p = (sorted.length - 1) * q, lo = Math.floor(p), hi = Math.ceil(p);
  return sorted[lo] + (sorted[hi] - sorted[lo]) * (p - lo);
};

export interface RiskCell {label: string; value: number | null; kind: Kind; tip: string}
export function extendedRisk(d: DeskRun): RiskCell[] {
  const s = d.stats || {}, m = monthlyReturns(d.dates, d.equity), sorted = [...m].sort((a, b) => a - b);
  const pos = m.filter(x => x > 0).reduce((a, b) => a + b, 0), neg = m.filter(x => x < 0).reduce((a, b) => a + b, 0);
  const avg = mean(m), sd = avg == null || m.length < 2 ? null : Math.sqrt(m.reduce((a, x) => a + (x - avg) ** 2, 0) / (m.length - 1));
  const moment = (k: number) => avg == null || !sd || m.length < 3 ? null : m.reduce((a, x) => a + ((x - avg) / sd) ** k, 0) / m.length;
  const tail = m.length >= 20 ? ratio(quantile(sorted, 0.95), Math.abs(quantile(sorted, 0.05))) : null;
  const pf = num(s.profit_factor), win = num(s.win_rate), payoff = num(s.payoff), dd = num(s.max_dd);
  const k4 = moment(4);
  return [
    {label: "Sortino", value: num(s.sortino), kind: "ratio", tip: "Like Sharpe, but it only punishes downside moves. Higher means better returns for the downside risk taken. Engine figure (daily returns)."},
    {label: "Calmar", value: num(s.car_mdd ?? s.mar), kind: "ratio", tip: "Annual growth (CAGR) divided by the worst drawdown."},
    {label: "Vol Ann.", value: num(s.vol), kind: "percent", tip: "Annualised volatility of the strategy's daily returns (engine figure)."},
    {label: "Profit Factor", value: pf, kind: "ratio", tip: "Sum of the winning trades' returns divided by the absolute sum of the losing trades' returns."},
    {label: "Payoff", value: payoff, kind: "ratio", tip: "Average winning trade divided by the absolute average losing trade (the same figure as Risk Reward)."},
    {label: "Recovery", value: ratio(num(s.compounded_return ?? s.total_return), dd == null ? null : Math.abs(dd)), kind: "ratio", tip: "Compounded total return divided by the worst drawdown."},
    {label: "Common Sense Ratio", value: tail == null || !neg ? null : pos / Math.abs(neg) * tail, kind: "ratio", tip: "Monthly profit factor (winning months over the absolute losing months) times the tail ratio."},
    {label: "CPC Index", value: pf == null || win == null || payoff == null ? null : pf * win * payoff, kind: "ratio", tip: "Profit factor × win rate × payoff."},
    {label: "Tail Ratio", value: tail, kind: "ratio", tip: "95th-percentile month divided by the absolute 5th-percentile month (needs 20+ months)."},
    {label: "Gain/Pain (Monthly)", value: neg ? m.reduce((a, b) => a + b, 0) / Math.abs(neg) : null, kind: "ratio", tip: "Sum of all monthly returns divided by the absolute sum of the losing months (Schwager)."},
    {label: "Kelly", value: win == null || !payoff ? null : win - (1 - win) / payoff, kind: "percent", tip: "Win rate minus (1 − win rate) divided by payoff: a position-sizing guide."},
    {label: "Avg Return", value: avg, kind: "percent", tip: "Average monthly return of the equity curve in the selected range."},
    {label: "Avg Win", value: num(s.avg_win), kind: "percent", tip: "Average return of the winning trades."},
    {label: "Avg Loss", value: num(s.avg_loss), kind: "percent", tip: "Average return of the losing trades."},
    {label: "Skew", value: moment(3), kind: "ratio", tip: "Lean of the monthly returns: positive leans to large gains, negative to large losses."},
    {label: "Kurtosis", value: k4 == null ? null : k4 - 3, kind: "ratio", tip: "Excess kurtosis of the monthly returns (a normal distribution is 0); higher means fatter tails."},
  ];
}

export interface DrawdownPeriod {started: string; trough: string; recovered: string | null; recoveryDays: number | null; drawdown: number; days: number}
const dayGap = (a: string, b: string) => Math.round((Date.parse(b) - Date.parse(a)) / 864e5);

/** The n deepest peak-to-trough declines of the equity curve, each from its peak to recovery (or the last bar). */
export function worstDrawdowns(dates: string[] = [], equity: number[] = [], n = 5): DrawdownPeriod[] {
  const out: DrawdownPeriod[] = [];
  let peak = 0, trough = -1;
  const close = (end: number | null) => {
    if (trough < 0) return;
    const dd = equity[trough] / equity[peak] - 1, stop = end ?? dates.length - 1;
    out.push({started: dates[peak], trough: dates[trough], recovered: end == null ? null : dates[end],
      recoveryDays: end == null ? null : dayGap(dates[trough], dates[end]), drawdown: dd, days: dayGap(dates[peak], dates[stop])});
  };
  for (let i = 1; i < equity.length; i++) {
    if (!(equity[i] > 0)) continue;
    if (equity[i] >= equity[peak]) { close(trough >= 0 ? i : null); peak = i; trough = -1; continue; }
    if (trough < 0 || equity[i] < equity[trough]) trough = i;
  }
  if (trough >= 0) close(null);
  return out.sort((a, b) => a.drawdown - b.drawdown).slice(0, n);
}

/** Compare Strategies rows, Return % to CAR/MDD: [label, strategy, buy & hold, kind]; trade statistics have no hold value. */
export function compareRows(s: Stats, b: Stats | null): Array<[string, number | null, number | null, Kind]> {
  const bh = (k: string) => b ? num(b[k]) : null;
  return [
    ["Return %", num(s.total_return), bh("total_return"), "percent"],
    ["CAGR", num(s.cagr), bh("cagr"), "percent"],
    ["Trades", num(s.trades), null, "count"],
    ["Win Rate", num(s.win_rate), null, "percent"],
    ["Expectancy", num(s.expectancy_ratio), null, "ratio"],
    ["Monthly Thickness", num(s.monthly_thickness), null, "percent"],
    ["Risk Reward", num(s.payoff), null, "ratio"],
    ["Max DD", num(s.max_dd), bh("max_dd"), "percent"],
    ["CAR/MDD", num(s.car_mdd ?? s.mar), bh("mar"), "ratio"],
  ];
}
