// The lab's optimizer results, drawn for our YT pages (handoff sections 9 and Appendix C section 5): the "Now / #1 / #2 / #3" table with Stats and Tradesheet
// buttons, the metric picker with the isometric 3D surface, the tradesheet and sheet downloads, and the sortable "Optimization result list".
// Numbers come from the engine's `sheet` rows (36 AmiBroker columns per combination, see backend/src/amsheet.rs) plus the derived columns of the sheet.
import {AM_COLUMNS, derivedValues} from './optimize-sheet.js';

const fin = v => typeof v === 'number' && Number.isFinite(v);
const esc = s => String(s).replace(/[&<>"]/g, c => ({'&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;'}[c]));
const grp = (n, d) => n.toLocaleString('en-US', {minimumFractionDigits: d, maximumFractionDigits: d});
const f2 = v => (fin(v) ? (Math.abs(v) < 0.005 ? '0.00' : grp(v, 2)) : '—');
const f0 = v => (fin(v) ? grp(Math.round(v), 0) : '—');

/** the twelve columns the page shows, in the lab's order: [title, source ('am' = AmiBroker column, 'd' = derived), name] */
export const SCREEN = [['CAR', 'am', 'CAR'], ['Max. Sys % Drawdown', 'am', 'Max. Sys % Drawdown'], ['CAR/MDD', 'am', 'CAR/MDD'], ['# Trades', 'am', '# Trades'],
  ['Avg % Profit/Loss', 'am', 'Avg % Profit/Loss'], ['Avg Bars Held', 'am', 'Avg Bars Held'], ['% of Winners', 'am', '% of Winners'], ['Expectancy Ratio', 'd', 'Expectancy Ratio'],
  ['Final Avg % Profit/Loss', 'd', 'Final Avg % Profit/Loss'], ['Final M. Thk', 'd', 'Final M. Thk'], ['Final Expectancy Ratio', 'd', 'Final Expectancy Ratio'], ['Total Net Profit', 'd', 'Total Net Profit']];
/** metrics of the 3D surface (handoff 5.2) */
export const SURFACE_METRICS = ['Net % Profit', 'Exposure %', 'CAR', 'Max. Sys % Drawdown', 'CAR/MDD', '# Trades', '% of Winners', 'Profit Factor', 'Avg % Profit/Loss', 'Sharpe Ratio'];

/** one entry per combination (engine order): its settings, the 36 columns, the derived values and the screen values */
export function labRows(res, position, defaults = {}) {
  const nd = res.dims.length;
  const make = (am, settings, k) => {
    const d = derivedValues(position, am);
    const screen = SCREEN.map(([, src, name]) => (src === 'am' ? am[AM_COLUMNS.indexOf(name)] : d[name]));
    return {k, no: k + 1, settings, am, d, screen};
  };
  const rows = (res.sheet || []).map((am, k) => make(am, res.rows[k].slice(0, nd), k));
  const now = (res.now_sheet || []).length ? make(res.now_sheet, res.dims.map(d => res.fixed?.[d.key] ?? defaults[d.key] ?? NaN), -1) : null;
  return {rows, now};
}

const sign = (title, v) => (title === '# Trades' || !fin(v) ? '' : v > 0 ? ' pos' : v < 0 ? ' neg' : '');
const cell = (title, v) => `<td class="n${sign(title, v)}">${title === '# Trades' ? f0(v) : f2(v)}</td>`;
const setCell = v => `<td class="oset-v">${fin(v) ? (Number.isInteger(v) ? v : grp(v, 2)) : '—'}</td>`;
const setTxt = v => (fin(v) ? (Number.isInteger(v) ? String(v) : grp(v, 2)) : '—');
/** the hover text of a Tradesheet button, as in the lab: every optimized setting with its value */
const tsTip = (labels, r) => esc(`Download the tradesheet of ${labels.map((l, i) => `${l} ${setTxt(r.settings[i])}`).join(' · ')} as an Excel file`);
const head = (labels, extra = '') => `<tr><th></th>${labels.map(l => `<th>${esc(l)}</th>`).join('')}${SCREEN.map(([t]) => `<th>${esc(t)}</th>`).join('')}${extra}</tr>`;

/** the Now / #1 / #2 / #3 table; `top` = the engine's top3 (row indexes), buttons carry data-stats / data-ts with the combination number */
export function topTableHTML(labels, lab, top) {
  const body = [];
  if (lab.now) body.push(`<tr class="now"><td class="rk">Now</td>${lab.now.settings.map(setCell).join('')}${lab.now.screen.map((v, i) => cell(SCREEN[i][0], v)).join('')}<td></td></tr>`);
  top.forEach((k, i) => {
    const r = lab.rows[k]; if (!r) return;
    body.push(`<tr class="${i === 0 ? 'pick' : ''}"><td class="rk">#${i + 1}</td>${r.settings.map(setCell).join('')}${r.screen.map((v, j) => cell(SCREEN[j][0], v)).join('')}<td class="act"><button class="olab" data-stats="${r.k}">Stats</button><button class="olab plain" data-tsrow="${r.k}" title="${tsTip(labels, r)}">Tradesheet</button></td></tr>`);
  });
  return `<div class="tblwrap"><table class="tbl lab-ocmp"><thead>${head(labels, '<th></th>')}</thead><tbody>${body.join('')}</tbody></table></div>`;
}

/** the sortable result list; sort = {col: index into labels+SCREEN or -1 for the engine order, dir: 1 | -1} */
export function listHTML(labels, lab, sort) {
  const nl = labels.length;
  const key = (r, c) => (c < nl ? r.settings[c] : r.screen[c - nl]);
  let rows = [...lab.rows];
  if (sort && sort.col >= 0) rows.sort((a, b) => { const x = key(a, sort.col), y = key(b, sort.col), xn = fin(x), yn = fin(y); if (xn && yn) return (x - y) * sort.dir || a.k - b.k; return xn ? -1 : yn ? 1 : a.k - b.k; });
  const th = [...labels, ...SCREEN.map(s => s[0])].map((t, c) => {
    const on = sort && sort.col === c;
    return `<th data-sort="${c}" aria-sort="${on ? (sort.dir > 0 ? 'ascending' : 'descending') : 'none'}">${esc(t)}${on ? (sort.dir > 0 ? ' ▲' : ' ▼') : ''}</th>`;
  }).join('');
  const shown = rows.slice(0, sort?.all ? rows.length : 100);
  const more = rows.length > shown.length ? `<button class="olab" data-showall="1">Show all ${f0(rows.length)}</button>` : '';
  return `<div class="tblwrap lab-opt-all"><table class="tbl lab-ocmp lab-opt-all-table"><thead><tr>${th}<th class="tsh">Tradesheet</th></tr></thead><tbody>${
    shown.map(r => `<tr>${r.settings.map(setCell).join('')}${r.screen.map((v, j) => cell(SCREEN[j][0], v)).join('')}<td class="act"><button class="olab plain" data-tsrow="${r.k}" title="${tsTip(labels, r)}">Tradesheet</button></td></tr>`).join('')}</tbody></table></div>${more}`;
}

/** isometric surface of one metric over the first two optimized settings: (u, v) are the grid positions, the height is the normalised metric */
export function surfaceHTML(res, labels, lab, metric) {
  const nx = res.dims[0].values.length, ny = res.dims[1].values.length;
  const idx = AM_COLUMNS.indexOf(metric);
  const z = Array.from({length: nx}, () => new Array(ny).fill(null));
  lab.rows.forEach(r => { const i = res.dims[0].values.indexOf(r.settings[0]), j = res.dims[1].values.indexOf(r.settings[1]); if (i >= 0 && j >= 0 && fin(r.am[idx])) z[i][j] = r.am[idx]; });
  const vals = z.flat().filter(fin);
  if (!vals.length) return `<p class="rnote">No values of ${esc(metric)} in this grid.</p>`;
  const lo = Math.min(...vals), hi = Math.max(...vals), t = v => (hi > lo ? (v - lo) / (hi - lo) : 0.5);
  const cos = Math.cos(Math.PI / 6), W = 560, H = 448, ox = 70, oy = 150;
  const P = (i, j, h) => { const u = i / (nx - 1), v = j / (ny - 1); return [ox + 180 + (u - v) * cos * 200, oy + (u + v) * 0.5 * 200 - h * 120]; };
  const quads = [];
  for (let i = 0; i < nx - 1; i++) for (let j = 0; j < ny - 1; j++) {
    const c = [[i, j], [i + 1, j], [i + 1, j + 1], [i, j + 1]];
    if (c.some(([a, b]) => !fin(z[a][b]))) continue;
    const tt = c.reduce((s, [a, b]) => s + t(z[a][b]), 0) / 4;
    const pts = c.map(([a, b]) => P(a, b, t(z[a][b])).map(n => n.toFixed(1)).join(',')).join(' ');
    quads.push({order: i + j, html: `<polygon points="${pts}" style="fill:color-mix(in oklab, var(--blue) ${(tt * 100).toFixed(0)}%, var(--red));stroke:var(--card);stroke-width:1" ><title>${esc(labels[0])} ${res.dims[0].values[i]} · ${esc(labels[1])} ${res.dims[1].values[j]} · ${esc(metric)} ${f2(z[i][j])}</title></polygon>`});
  }
  quads.sort((a, b) => a.order - b.order);
  const floor = [[0, 0], [nx - 1, 0], [nx - 1, ny - 1], [0, ny - 1]].map(([a, b]) => P(a, b, 0).map(n => n.toFixed(1)).join(',')).join(' ');
  // as in the lab: the vertical axis stands on the left corner, the first setting runs along the lower-left edge and the second along the lower-right edge
  const [lx, ly] = P(0, ny - 1, 0), [tx, ty] = P(0, ny - 1, 1), [bx, by] = P(nx - 1, ny - 1, 0), [rx, ry] = P(nx - 1, 0, 0);
  const deg = (xa, ya, xb, yb) => Math.atan2(yb - ya, xb - xa) * 180 / Math.PI;
  const unit = metric.includes('%') || ['CAR', 'Exposure %'].includes(metric) ? '%' : '';
  const label = (x, y, rot, s) => `<text x="${x.toFixed(1)}" y="${y.toFixed(1)}" transform="rotate(${rot.toFixed(1)} ${x.toFixed(1)} ${y.toFixed(1)})" text-anchor="middle" class="sv-t">${esc(s)}</text>`;
  const tick = (x, y, s, anchor) => `<text x="${x.toFixed(1)}" y="${y.toFixed(1)}" text-anchor="${anchor}" class="sv-t">${esc(s)}</text>`;
  const d0 = res.dims[0].values, d1 = res.dims[1].values;
  const svg = `<svg viewBox="0 0 ${W} ${H}" width="${W}" height="${H}" role="img" aria-label="${esc(metric)} surface"><polygon points="${floor}" class="sv-floor"/><line x1="${lx.toFixed(1)}" y1="${ly.toFixed(1)}" x2="${tx.toFixed(1)}" y2="${ty.toFixed(1)}" class="sv-axis"/>${quads.map(q => q.html).join('')}
    ${tick(tx - 8, ty + 4, f2(hi) + unit, 'end')}${tick(lx - 8, ly + 4, f2(lo) + unit, 'end')}
    ${label((lx + bx) / 2 - 22, (ly + by) / 2 + 34, deg(lx, ly, bx, by), labels[0])}${label((bx + rx) / 2 + 22, (by + ry) / 2 + 34, deg(bx, by, rx, ry), labels[1])}
    ${tick(lx - 6, ly + 20, d0[0], 'end')}${tick(bx, by + 18, `${d0[nx - 1]} · ${d1[ny - 1]}`, 'middle')}${tick(rx + 8, ry + 6, d1[0], 'start')}</svg>`;
  const legend = `<div class="sv-legend"><span>${f2(lo)}${unit}</span><i></i><span>${f2(hi)}${unit}</span><span class="sv-m">${esc(metric.replace(' % Drawdown', '').replace('Net % Profit', 'Net %'))}</span></div>`;
  return svg + legend;
}

/** the whole results block of the lab (without the ranges table) */
export function resultsHTML(res, labels, lab, top, metric, sort, extras = true) {
  return `${topTableHTML(labels, lab, top)}
    <div class="lab-dl"><label>Tradesheets <select id="adr-ts-mode"><option value="zip">Separate files (ZIP)</option><option value="one">One file</option></select></label><button class="olab" id="adr-ts-all">Download all tradesheets</button></div>
    <div class="lab-dl"><button class="olab" id="adr-opt-sheet" title="AmiBroker-style optimization workbook: Rules and every combination, sorted on CAR/MDD">Download optimization sheet</button>${extras ? '<button class="olab plain" id="adr-concl">Conclusion workbook</button><button class="olab plain" id="adr-gpt">GPT analysis of this grid</button>' : ''}</div>
    <div class="lab-list-h"><b>Optimization result list</b> <span>${f0(lab.rows.length)} combinations</span></div><div id="adr-list">${listHTML(labels, lab, sort)}</div>`;
}
