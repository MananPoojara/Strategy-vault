// Draws the AmiBroker-style report (cards + charts) and writes its Excel files. Pure functions: the page passes in what they need.

const fin = v => typeof v === 'number' && Number.isFinite(v);
const grp = (n, d) => n.toLocaleString('en-US', {minimumFractionDigits: d, maximumFractionDigits: d});
export const fPct = (v, d = 2) => (fin(v) ? `${(v * 100).toFixed(d)}%` : '—');
export const fNum = (v, d = 2) => (fin(v) ? grp(v, d) : '—');
const fInt = v => (fin(v) ? grp(Math.round(v), 0) : '—');
const dmy = s => String(s).slice(0, 10).split('-').reverse().join('-');
const esc = s => String(s).replace(/[&<>"]/g, c => ({'&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;'}[c]));

/** [title, head row or null, rows]; every row = [label, kind, all, long, short, buyhold] with raw values (null = no value) */
export function reportBlocks(rep) {
  const S = rep.system, B = rep.buy_hold || {}, A = rep.all, L = rep.long, Sh = rep.short, H = rep.buyhold || {winners: {}, losers: {}};
  const sys = (k, bh = true) => [S[k], null, null, bh ? B[k] : null];
  const side = f => [f(A), f(L), f(Sh), f(H)];
  const win = (key, word, big) => [
    [`Total ${word} (× initial equity)`, 'n3', ...side(s => s[key].profit)],
    ['Sum of trade %', 'p1', ...side(s => s[key].total)],
    [`Avg. ${word} %`, 'p2', ...side(s => s[key].avg)],
    ['Avg. Bars Held', 'n1', ...side(s => s[key].avg_bars)],
    ['Max. Consecutive', 'i', ...side(s => s[key].max_consecutive)],
    [`Largest ${big} %`, 'p2', ...side(s => s[key].largest)],
    [`# bars in largest ${big}`, 'i', ...side(s => s[key].bars_in_largest)],
  ];
  const net = S.net_profit, cap = rep.capital || 1000000;
  const blocks = [
    ['Returns', null, [
      ['Initial capital', 'm', cap, cap, cap, cap],
      ['Ending capital', 'm', cap * (1 + net), null, null, cap * (1 + B.net_profit)],
      ['Net Profit', 'm', cap * net, null, null, cap * B.net_profit],
      ['Net Profit %', 'p2', ...sys('net_profit')],
      ['Exposure %', 'p2', ...sys('exposure')],
      ['Net Risk Adjusted Return %', 'p2', ...sys('net_rar')],
      ['Annual Return %', 'p2', ...sys('cagr')],
      ['Risk Adjusted Return %', 'p2', ...sys('rar')],
      ['Transaction costs', 'm', tradeMoney(rep).costs, null, null, null],
    ]],
    ['Trades', ['All trades', 'i', ...side(s => s.trades)], [
      ['Avg. Profit/Loss %', 'p2', ...side(s => s.avg)],
      ['Avg. Bars Held', 'n1', ...side(s => s.avg_bars)],
    ]],
    ['Winners', ['Winners', 'wl', ...side(s => [s.winners.count, s.winners.pct])], win('winners', 'Profit', 'win')],
    ['Losers', ['Losers', 'wl', ...side(s => [s.losers.count, s.losers.pct])], win('losers', 'Loss', 'loss')],
    ['Risk & ratios', null, [
      ['Max. trade % drawdown', 'p2', S.max_trade_dd, null, null, null],
      ['Max. system drawdown (× initial equity)', 'n3', ...sys('max_sys_dd_abs')],
      ['Max. system % drawdown', 'p2', ...sys('max_sys_dd')],
      ['Recovery Factor', 'n2', ...sys('recovery_factor')],
      ['CAR/MaxDD', 'n2', ...sys('car_mdd')],
      ['RAR/MaxDD', 'n2', ...sys('rar_mdd')],
      ['Profit Factor', 'n2', ...side(s => s.profit_factor)],
      ['Payoff Ratio', 'n2', ...side(s => s.payoff)],
      ['Standard Error (× initial equity)', 'n3', ...sys('std_error')],
      ['Risk-Reward Ratio', 'n2', ...sys('rrr')],
      ['Ulcer Index', 'n2', ...sys('ulcer_index')],
      ['Ulcer Performance Index', 'n2', ...sys('upi')],
      ['Sharpe Ratio of trades', 'n2', S.sharpe_trades, null, null, null],
      ['K-Ratio', 'n3', ...sys('k_ratio')],
    ]],
  ];
  if (rep.buy_hold) return blocks;
  // no index for this basket: the Buy&Hold column stays empty
  const blank = r => (r ? [...r.slice(0, 5), null] : r);
  return blocks.map(([title, head, rows]) => [title, blank(head), rows.map(blank)]);
}

const money = v => (Number.isInteger(Math.round(v * 100) / 100) ? grp(Math.round(v), 0) : grp(v, 2));   // the lab shows whole amounts without decimals
const text = (v, kind) => {
  if (kind === 'wl') return Array.isArray(v) ? `${fInt(v[0])} (${v[1] == null ? '—' : fPct(v[1])})` : fInt(v);
  if (v === null || v === undefined) return '—';
  return kind === 'm' ? money(v) : kind === 'p1' ? fPct(v, 1) : kind === 'p2' ? fPct(v, 2) : kind === 'n1' ? fNum(v, 1) : kind === 'n2' ? fNum(v, 2) : kind === 'n3' ? fNum(v, 3) : fInt(v);
};
const SIGNED = /Profit|Loss|Return|drawdown|Largest|Recovery|CAR\/MaxDD|RAR\/MaxDD/;
const tone = (label, s) => (SIGNED.test(label) ? (/^[1-9]|^0\.0*[1-9]/.test(s) ? ' pos' : /^[-−]\d/.test(s) ? ' neg' : '') : '');
const heads = tk => ['All trades', 'Long trades', 'Short trades', `Buy&Hold (${tk})`];

function card(title, head, rows, tk) {
  const cell = (label, kind, vals) => vals.map(v => { const s = text(v, kind); return `<td class="n${tone(label, s)}">${s}</td>`; }).join('');
  return `<table class="tbl rep-card"><caption>${esc(title)}</caption><thead><tr><th></th>${heads(tk).map(h => `<th>${esc(h)}</th>`).join('')}</tr></thead><tbody>`
    + (head ? `<tr><td>${esc(head[0])}</td>${cell(head[0], head[1], head.slice(2))}</tr>` : '')
    + rows.map(([l, k, ...v]) => `<tr><td${head ? ' class="sub"' : ''}>${esc(l)}</td>${cell(l, k, v)}</tr>`).join('')
    + '</tbody></table>';
}

// ---- charts: one figure = caption, y labels (top / bottom), a 100 x 60 SVG that stretches to its box, x labels (start / title / end)
const RED = '#b63b35', GREEN = '#3a7d5a';
const figure = (cls, title, sub, yTop, yBot, svg, x0, xt, x1, attrs = '') =>
  `<figure class="rep-chart ${cls}"${attrs}><figcaption>${esc(title)}<small>${esc(sub)}</small></figcaption><div class="rep-plot"><div class="rep-y"><span>${yTop}</span><span>${yBot}</span></div>${svg}</div><div class="rep-x"><span>${x0}</span><span>${esc(xt)}</span><span>${x1}</span></div></figure>`;
const none = (cls, title) => `<figure class="rep-chart ${cls}"><figcaption>${esc(title)}</figcaption><p class="rnote">No trades.</p></figure>`;

/** histogram of values in 5 % buckets (empty buckets between the first and the last are drawn too, so every bar has the same width) */
function histogram(values, title, xTitle, cls) {
  const v = values.filter(fin);
  if (!v.length) return none(cls, title);
  const idx = v.map(r => Math.floor(r / 0.05)), lo = Math.min(...idx), hi = Math.max(...idx), nb = hi - lo + 1;
  const count = new Array(nb).fill(0); idx.forEach(i => count[i - lo]++);
  const top = Math.max(...count), bw = 96 / nb;
  const bars = count.map((c, k) => (c ? `<rect x="${(2 + k * bw + bw * 0.07).toFixed(2)}" y="${(58 - c / top * 54).toFixed(2)}" width="${(bw * 0.86).toFixed(2)}" height="${(c / top * 54).toFixed(2)}" fill="${lo + k >= 0 ? GREEN : RED}"><title>${fPct((lo + k) * 0.05, 0)} to ${fPct((lo + k) * 0.05 + 0.05, 0)}: ${c} trades</title></rect>` : '')).join('');
  const zero = lo <= 0 && hi >= 0 ? `<line class="axis" x1="${(2 - lo * bw).toFixed(2)}" x2="${(2 - lo * bw).toFixed(2)}" y1="2" y2="58"/>` : '';
  return figure(cls, title, 'trades per 5 % bucket', top, 0, `<svg viewBox="0 0 100 60" preserveAspectRatio="none" role="img" aria-label="${esc(title)}">${bars}${zero}</svg>`, fPct(lo * 0.05, 0), xTitle, fPct((hi + 1) * 0.05, 0));
}

/** one dot per trade (x = MAE or MFE, y = trade %), red for a loss and green for a win; the vertical axis marks 0 % */
function scatter(trades, key, title, xTitle, cls) {
  const pts = trades.filter(t => fin(t[key]) && fin(t.ret));
  if (!pts.length) return none(cls, title);
  const xs = pts.map(t => t[key]), ys = pts.map(t => t.ret), x0 = Math.min(...xs, 0), x1 = Math.max(...xs, 0), y0 = Math.min(...ys, 0), y1 = Math.max(...ys, 0);
  const X = v => 2 + (v - x0) / ((x1 - x0) || 1) * 96, Y = v => 58 - (v - y0) / ((y1 - y0) || 1) * 56;
  const dots = pts.map(t => { const x = X(t[key]).toFixed(2), y = Y(t.ret).toFixed(2); return `<line x1="${x}" x2="${x}" y1="${y}" y2="${y}" stroke="${t.ret > 0 ? GREEN : RED}" stroke-width="5" stroke-linecap="round" vector-effect="non-scaling-stroke"><title>${xTitle} ${fPct(t[key])} · trade ${fPct(t.ret)}</title></line>`; }).join('');
  const zeroY = Y(0).toFixed(2), zeroX = X(0).toFixed(2);
  return figure(cls, title, 'y: trade %', fPct(y1, 0), fPct(y0, 0),
    `<svg viewBox="0 0 100 60" preserveAspectRatio="none" role="img" aria-label="${esc(title)} chart"><line class="axis" x1="${zeroX}" x2="${zeroX}" y1="2" y2="58"/><line class="axis" x1="2" x2="98" y1="${zeroY}" y2="${zeroY}"/>${dots}</svg>`,
    fPct(x0, 0), xTitle, fPct(x1, 0), ` data-rep-excursion="${key}"`);
}

export function reportHTML(rep, tk) {
  const cards = reportBlocks(rep).map(([title, h, r]) => card(title, h, r, tk)).join('');
  return `<section class="box" id="adr-report"><div class="rep-head"><div class="rep-title">Report <small>AmiBroker-style statistics · simulated · initial capital 10,00,000</small></div><button class="dbtn primary" id="adr-report-xlsx">Download report</button></div>
  <div class="rep-cards">${cards}</div>
  <div class="rep-toggle"><span class="join sm"><button class="join-item active" data-rep-mode="scatter">MAE/MFE vs trade %</button><button class="join-item" data-rep-mode="dist">MAE/MFE distribution</button></span></div>
  <div class="rep-charts">${histogram(rep.trades.map(t => t.ret), 'Profit distribution', '% Profit', 'rep-always')}${scatter(rep.trades, 'mae', 'Max. Adverse Excursion', 'MAE %', 'rep-scatter')}${scatter(rep.trades, 'mfe', 'Max. Favourable Excursion', 'MFE %', 'rep-scatter')}${histogram(rep.trades.map(t => t.mae), 'Max. Adverse Excursion distribution', 'MAE %', 'rep-hist')}${histogram(rep.trades.map(t => t.mfe), 'Max. Favourable Excursion distribution', 'MFE %', 'rep-hist')}</div>
  <p class="rnote">Definitions: 365-day years · drawdown peak starts at the first bar · an open trade is included and marked at the last close · exposure counts bars from entry up to the bar before the exit · risk-free rate 5 % (Sharpe of trades) and 5.4 % (Ulcer performance index) · profit = equity before the trade × its return · Buy&amp;Hold = one trade over the run.</p></section>`;
}

// ---------------------------------------------------------------- Excel (written by lab-xlsx.js, the lab's table look; cells are the text shown on the page)
const DEC = {p1: [fPct, 1], p2: [fPct, 2], n1: [fNum, 1], n2: [fNum, 2], n3: [fNum, 3], i: [fInt], m: [money]};
const shownCell = (kind, v) => { const f = DEC[kind]; return fin(v) ? f[0](v, f[1]) : '—'; };

/** the "Statistics" sheet as blocks for labSheet/buildLabXlsx: a title, a header row, the rows; a dash where a column has no value */
export function statisticsBlocks(rep, tk) {
  return reportBlocks(rep).map(([title, head, rows]) => {
    const row = (label, kind, vals) => [label, ...vals.map(v => (kind === 'wl'
      ? (Array.isArray(v) ? `${fInt(v[0])} (${v[1] == null ? '—' : fPct(v[1])})` : fInt(v))
      : shownCell(kind, v)))];
    return {title, header: ['', ...heads(tk)], rows: [...(head ? [row(head[0], head[1], head.slice(2))] : []), ...rows.map(([l, k, ...v]) => row(l, k, v))]};
  });
}

export const TRADE_COLUMNS = ['Symbol', 'Trade type', 'Entry date', 'Entry price', 'Exit date', 'Exit price', '% Change', 'Bars held', 'Days', 'Net profit', 'MAE', 'MFE', 'DD %', 'Allocation', 'Shares', 'Position value', 'Costs', 'Exit reason'];

/** the lab's money mode for a run: shares = whole shares the equity before the trade buys at the entry price, position value = shares x entry price,
 *  net profit = equity before x the trade's return (so it adds up to the Returns card), costs = both sides on the position value */
export function tradeMoney(rep) {
  let eq = rep.capital || 1000000, costs = 0;
  const rows = rep.trades.map(t => {
    const fees = (value) => value * (rep.side || 0) * (1 + t.exitPx / t.entryPx);
    if (rep.portfolio) {            // a stock universe: the engine already sized and priced every trade
      const value = t.shares * t.entryPx, fee = fees(value);
      costs += fee;
      return {profit: t.pnl, weight: t.weight, shares: t.shares, value, fee};
    }
    const before = eq, shares = Math.floor(before / t.entryPx), value = shares * t.entryPx, fee = fees(value);
    eq += before * t.ret; costs += fee;
    return {profit: before * t.ret, weight: value / before, shares, value, fee};
  });
  return {rows, costs};
}

/** the "Trades" sheet rows (without the header): the trade table plus the money columns and the exit reason */
export function tradeRows(rep, symbol) {
  const m = tradeMoney(rep).rows;
  return rep.trades.map((t, i) => {
    const days = Math.round((Date.parse(t.exit) - Date.parse(t.entry)) / 86400000);
    return [t.symbol || symbol, t.dir > 0 ? 'Long' : 'Short', dmy(t.entry), fNum(t.entryPx), t.open ? 'Open' : dmy(t.exit), fNum(t.exitPx), fPct(t.ret), String(t.bars), String(days),
      money(m[i].profit), fPct(t.mae), fPct(t.mfe), fPct(t.accountDd), fPct(m[i].weight, 1), String(m[i].shares), money(m[i].value), money(m[i].fee), t.open ? 'Still open at the end' : (t.reason || '—')];
  });
}

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const pctUnits = v => (fin(v) ? Math.round(v * 10000) / 100 : null);     // 12.34 means 12.34 %, a plain number
/** the YOY-Monthly sheet rows: Year, Jan ... Dec, Total, then a Total and an Avg row */
export function yoyAoa(y) {
  return [['Year', ...MONTHS, 'Total'],
    ...y.rows.map(r => [r.year, ...r.months.map(pctUnits), pctUnits(r.total)]),
    ['Total', ...y.totals.map(pctUnits), pctUnits(y.grand)],
    ['Avg', ...y.avgs.map(pctUnits), pctUnits(y.avgOfTotals)]];
}
