// AmiBroker-style report statistics for one run (ADR results page): headline tiles, the All / Long / Short / Buy&Hold report,
// per-trade DD % and bars held, and the YOY-Monthly grid. Definitions follow the Results-feature handoff (section 4.3, "report" set):
// 365-day years, the drawdown peak starts at the first bar, an open trade is included and marked at the last close, a trade's
// exposure runs from its entry bar to the bar before its exit, bars held for the report count both the entry and the exit bar.
// Statistics are per 1 of capital (a trade's profit is the equity before the trade times its return); `capital` only scales the money amounts shown.

const sum = a => a.reduce((s, x) => s + x, 0);
const mean = a => (a.length ? sum(a) / a.length : null);
const RF_SHARPE = 5, RF_UPI = 5.4;                          // AmiBroker defaults (percent)

function curveStats(eq, years, exposure) {
  const n = eq.length, net = eq[n - 1] / eq[0] - 1, cagr = years > 0 ? Math.pow(eq[n - 1] / eq[0], 1 / years) - 1 : null;
  let peak = eq[0], maxDd = 0, maxAbs = 0, u2 = 0;
  for (let k = 0; k < n; k++) {
    peak = Math.max(peak, eq[k]);
    const dd = eq[k] / peak - 1;
    maxDd = Math.min(maxDd, dd); maxAbs = Math.max(maxAbs, peak - eq[k]); u2 += (100 * dd) ** 2;
  }
  // least squares of equity on the bar number
  const xm = (n - 1) / 2, ym = mean(eq);
  let sxx = 0, sxy = 0;
  for (let k = 0; k < n; k++) { sxx += (k - xm) ** 2; sxy += (k - xm) * (eq[k] - ym); }
  const slope = sxy / sxx, icpt = ym - slope * xm;
  let sse = 0;
  for (let k = 0; k < n; k++) sse += (eq[k] - (icpt + slope * k)) ** 2;
  const se = n > 2 ? Math.sqrt(sse / (n - 2)) : null, barsPerYear = years > 0 ? n / years : null;
  const ulcer = Math.sqrt(u2 / n), abs = Math.abs(maxDd);
  const rar = exposure > 0 && cagr != null ? cagr / exposure : null;
  return {
    net_profit: net, cagr, exposure, max_sys_dd: maxDd, max_sys_dd_abs: -maxAbs,
    net_rar: exposure > 0 ? net / exposure : null, rar,
    recovery_factor: maxAbs > 0 ? net / maxAbs : null,
    car_mdd: abs > 0 && cagr != null ? cagr / abs : null, rar_mdd: abs > 0 && rar != null ? rar / abs : null,
    std_error: se, rrr: se && barsPerYear ? slope * barsPerYear / se : null,
    ulcer_index: ulcer, upi: ulcer > 0 && cagr != null ? (100 * cagr - RF_UPI) / ulcer : null,
    k_ratio: se ? slope * Math.sqrt(sxx) / se / n : null, bars_per_year: barsPerYear,
  };
}

function sideBlock(list, total) {
  const win = list.filter(t => t.ret > 0), loss = list.filter(t => t.ret <= 0);
  const run = pred => { let best = 0, cur = 0; for (const t of list) { cur = pred(t) ? cur + 1 : 0; best = Math.max(best, cur); } return best; };
  const part = (g, pred, pick) => {
    const big = g.length ? g.reduce((a, b) => (pick(b.ret, a.ret) ? b : a)) : null;
    return {
      count: g.length, pct: total ? g.length / total : null, profit: sum(g.map(t => t.profit)), total: sum(g.map(t => t.ret)),
      avg: g.length ? sum(g.map(t => t.ret)) / g.length : null, avg_bars: mean(g.map(t => t.held)),
      max_consecutive: run(pred), largest: big ? big.ret : null, bars_in_largest: big ? big.held : null,
    };
  };
  const winP = sum(win.map(t => t.profit)), lossP = sum(loss.map(t => t.profit));
  return {
    trades: list.length, avg: mean(list.map(t => t.ret)), avg_bars: mean(list.map(t => t.held)),
    winners: part(win, t => t.ret > 0, (a, b) => a > b), losers: part(loss, t => t.ret <= 0, (a, b) => Math.abs(a) > Math.abs(b)),
    profit_factor: lossP < 0 ? winP / -lossP : null,
    payoff: win.length && loss.length && lossP < 0 ? (winP / win.length) / (-lossP / loss.length) : null,
  };
}

/** run = {dates, close, equity, bh, trades}; bars = {d,h,l} (daily prices of the traded instrument, for MAE / MFE); bh = null: the basket has no index, so no Buy&Hold */
export function buildReport(run, bars, money = {}) {
  const {dates, close, equity, bh} = run, n = equity.length, port = !!run.portfolio;   // portfolio = a stock-universe run: many positions at once, real money per trade
  const years = (Date.parse(dates[n - 1]) - Date.parse(dates[0])) / 86400000 / 365;
  const px = new Map();
  if (bars?.d) bars.d.forEach((d, i) => px.set(String(d).slice(0, 10), i));
  let growth = 1;
  const trades = (run.trades || []).filter(t => t.legs?.length && Number.isFinite(t.ret_full_precision ?? t.ret)).map(t => {
    const i = t.legs[0][0], x = t.exit_i, ret = t.ret_full_precision ?? t.ret, dir = t.dir === -1 ? -1 : 1, profit = port ? t.pnl / (money.capital || 1000000) : growth * ret;
    if (!port) growth *= 1 + ret;
    // worst peak-to-valley of the trade: entry fill, the closes from the entry bar to the bar before the exit, the exit fill
    const mark = p => (dir > 0 ? p / t.avg_px : 2 - p / t.avg_px), path = [mark(t.avg_px)];
    if (close) for (let k = i; k < x; k++) path.push(mark(close[k]));        // a stock universe has no single close series: the two fills only
    path.push(mark(t.exit_px));
    let pk = path[0], dd = 0;
    for (const m of path) { pk = Math.max(pk, m); dd = Math.min(dd, m / pk - 1); }
    let mae = null, mfe = null;
    const a = px.get(String(t.entry).slice(0, 10)), b = px.get(String(t.exit).slice(0, 10));
    if (a != null && b != null && bars.h && bars.l) {
      let lo = Infinity, hi = -Infinity;
      for (let k = a; k <= b; k++) { lo = Math.min(lo, bars.l[k]); hi = Math.max(hi, bars.h[k]); }
      const lowRet = lo / t.avg_px - 1, highRet = hi / t.avg_px - 1;
      mae = dir > 0 ? Math.min(0, lowRet) : Math.min(0, -highRet); mfe = dir > 0 ? Math.max(0, highRet) : Math.max(0, -lowRet);
    }
    return {dir, i, x, ret, profit, held: x - i + 1, bars: x - i, open: !!t.open, reason: t.reason, entry: t.entry, exit: t.exit, entryPx: t.avg_px, exitPx: t.exit_px, maxTradeDd: dd, mae, mfe, equityAfter: port ? equity[Math.min(x, n - 1)] / equity[0] : growth,
      symbol: t.symbol, pnl: t.pnl, shares: t.shares, weight: t.weight};
  });
  // running account drawdown after each trade (the trade table's "DD %")
  let pk = 1;
  trades.forEach(t => { pk = Math.max(pk, t.equityAfter); t.accountDd = t.equityAfter / pk - 1; });

  const exposure = sum(trades.map(t => (t.x - t.i) * (port ? t.weight || 0 : 1))) / n;      // a portfolio's exposure = the average share of equity in positions
  const system = {...curveStats(equity, years, exposure), max_trade_dd: trades.length ? Math.min(...trades.map(t => t.maxTradeDd)) : null};
  const rets = trades.map(t => t.ret * 100), held = mean(trades.map(t => t.held)), sd = rets.length ? Math.sqrt(mean(rets.map(r => (r - mean(rets)) ** 2))) : null;
  if (rets.length && system.bars_per_year && held && sd > 0) { const f = system.bars_per_year / held; system.sharpe_trades = (mean(rets) * f - RF_SHARPE) / (sd * Math.sqrt(f)); } else system.sharpe_trades = null;
  const buy_hold = bh ? curveStats(bh, years, 1) : null;
  const bhNet = bh ? bh[n - 1] / bh[0] - 1 : NaN;
  const buyhold = !bh ? null : {
    trades: 1, avg: bhNet, avg_bars: n, profit_factor: null, payoff: null,
    winners: {count: bhNet > 0 ? 1 : 0, pct: bhNet > 0 ? 1 : 0, profit: bhNet > 0 ? bhNet : 0, total: bhNet > 0 ? bhNet : 0, avg: bhNet > 0 ? bhNet : null, avg_bars: bhNet > 0 ? n : null, max_consecutive: bhNet > 0 ? 1 : 0, largest: bhNet > 0 ? bhNet : null, bars_in_largest: bhNet > 0 ? n : null},
    losers: {count: bhNet > 0 ? 0 : 1, pct: bhNet > 0 ? 0 : 1, profit: bhNet > 0 ? 0 : bhNet, total: bhNet > 0 ? 0 : bhNet, avg: bhNet > 0 ? null : bhNet, avg_bars: bhNet > 0 ? null : n, max_consecutive: bhNet > 0 ? 0 : 1, largest: bhNet > 0 ? null : bhNet, bars_in_largest: bhNet > 0 ? null : n},
  };
  return {
    years, trades, system, buy_hold, capital: money.capital || 1000000, side: money.side || 0, portfolio: port,   // money mode: the amounts of the Returns card and the Trades sheet scale with `capital`
    all: sideBlock(trades, trades.length), long: sideBlock(trades.filter(t => t.dir > 0), trades.length), short: sideBlock(trades.filter(t => t.dir < 0), trades.length), buyhold,
  };
}

/** YOY-Monthly grid: monthly return = equity at the last bar of the month / equity at the last bar of the previous month - 1 */
export function yoyMonthly(dates, equity) {
  const last = new Map();                                    // 'YYYY-MM' -> equity at the last bar of that month
  dates.forEach((d, i) => last.set(String(d).slice(0, 7), equity[i]));
  const keys = [...last.keys()], byYear = new Map();
  let prev = 1;
  for (const k of keys) {
    const v = last.get(k), r = v / prev - 1; prev = v;
    const y = +k.slice(0, 4), m = +k.slice(5, 7) - 1;
    if (!byYear.has(y)) byYear.set(y, new Array(12).fill(null));
    byYear.get(y)[m] = Math.abs(r) <= 1e-12 ? null : r;
  }
  const rows = [...byYear.entries()].sort((a, b) => a[0] - b[0]).map(([year, months]) => ({year, months, total: sum(months.map(v => v ?? 0))}));
  const totals = Array.from({length: 12}, (_, m) => sum(rows.map(r => r.months[m] ?? 0)));
  const avgs = Array.from({length: 12}, (_, m) => { const v = rows.map(r => r.months[m]).filter(x => x != null); return v.length ? mean(v) : null; });
  return {rows, totals, grand: sum(totals), avgs, avgOfTotals: rows.length ? mean(rows.map(r => r.total)) : null};
}
