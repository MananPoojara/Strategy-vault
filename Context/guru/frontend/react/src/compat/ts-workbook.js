// "Download TS": the research team's Excel tradesheet (format "From 2000"), built in the browser with SheetJS (xlsx-js-style).
// Every number is a live Excel formula, so changing the costs in C1:E1 recalculates the sheet. Layout follows ts-template.js.
import TPL from './ts-template.js';
import {evaluateWorkbook} from './ts-eval.js';
import {zipDeflate} from './optimize-sheet.js';

const REF_NAME = 'NF_En_RSI 14_EX_ATR 5-2,4';           // sheet name inside the template (replaced by the real tab name)
const serial = iso => (Date.UTC(+iso.slice(0, 4), +iso.slice(5, 7) - 1, +iso.slice(8, 10)) - Date.UTC(1899, 11, 30)) / 86400000;
const STYLES = TPL.styles.map(t => {
  const s = {font: {name: t.nm || 'Calibri', sz: t.sz || 11, bold: !!t.b, italic: !!t.i}};
  if (t.col) s.font.color = {rgb: t.col.slice(-6)};
  if (t.fill) s.fill = {patternType: 'solid', fgColor: {rgb: t.fill.slice(-6)}};
  if (t.z && t.z !== 'General') s.numFmt = t.z;
  if (t.h || t.w) s.alignment = {horizontal: t.h || undefined, wrapText: !!t.w};
  const edge = k => t[k] ? {style: t[k], color: {rgb: 'B0B0B0'}} : undefined;
  if (t.bt || t.bb || t.bl || t.br) s.border = {top: edge('bt'), bottom: edge('bb'), left: edge('bl'), right: edge('br')};
  return s;
});
const PAD = 5 / 6;                                       // the library writes width = wch + 5/6, so subtract it to get the template's exact width
const colN = c => c.split('').reduce((n, ch) => n * 26 + ch.charCodeAt(0) - 64, 0);

// a formula written for trade row 22 (previous row 21) moved to row r (previous row r-1)
const moveRow = (f, r) => f.replace(/(?<![$A-Z0-9])([A-Z]{1,2})22(?![0-9])/g, '$1@R@').replace(/(?<![$A-Z0-9])([A-Z]{1,2})21(?![0-9])/g, '$1' + (r - 1)).replace(/@R@/g, String(r));
// ranges the data outgrows are stretched to the last trade / bar (the template ranges stay when they already cover it)
const stretch = (f, lastTrade, lastBar) => f
  .replace(/\$(\d{2,3})\b/g, (m, n) => (['250', '374'].includes(n) ? '$' + Math.max(+n, lastTrade) : m))
  .replace(/\$52322\b/g, '$' + Math.max(52322, lastBar))
  .replace(/(:[A-Z]{1,2})501\b/g, (m, c) => c + Math.max(501, lastTrade));      // Max Winner / Max Looser (net) read K21:K501 in the team's template

function put(ws, c, extra = {}) {
  const o = {...c, ...extra};
  const cell = {s: o.va ? {...STYLES[o.s], alignment: {...STYLES[o.s].alignment, vertical: 'center'}} : STYLES[o.s]};
  if (o.f !== undefined) { cell.t = 'n'; cell.f = o.f; if (o.F) cell.F = o.F; }
  else if (o.d !== undefined) { cell.t = 'n'; cell.v = serial(o.d); }
  else if (typeof o.v === 'number') { cell.t = 'n'; cell.v = o.v; }
  else if (o.v !== undefined) { cell.t = 's'; cell.v = String(o.v); }
  else if (cell.s.font.bold || cell.s.fill || cell.s.border) { cell.t = 's'; cell.v = ''; }   // the team's file keeps its blank formatted cells as empty text
  else cell.t = 'z';
  ws[o.a] = cell;
}
function extent(ws) {
  let maxR = 0, maxC = 0;
  for (const k of Object.keys(ws)) {
    if (k[0] === '!') continue;
    const m = /^([A-Z]+)(\d+)$/.exec(k); maxR = Math.max(maxR, +m[2]); maxC = Math.max(maxC, colN(m[1]));
  }
  return maxR ? {s: {r: 0, c: 0}, e: {r: maxR - 1, c: maxC - 1}} : null;
}
const widths = obj => Object.entries(obj).map(([k, w]) => [colN(k) - 1, w]).reduce((a, [i, w]) => { a[i] = {wch: w - PAD}; return a; }, []);

/**
 * trades: [{dir:'Long'|'Short', entry:'YYYY-MM-DD', entryPx, exit:'YYYY-MM-DD', exitPx, sym?}]  (an open trade uses the last bar as its exit; sym for a stock basket, else ticker)
 * bars:   {d:[YYYY-MM-DD], o, h, l, c, s?}   daily bars of the instrument (the MAE formulas read them); s = the symbol of each bar for a stock basket
 * mae:    optional [value per trade]: the MAE results worked out by the caller (maeOf), so a basket's thousands of MAE array formulas need not be evaluated here
 * rules:  {timeframe, index, period, aflPath, doneBy, verifiedBy, position, filter, setup, entry, exit}
 * costs:  {impact, rollLong, rollShort}  fractions (0.003 = 0.3 %)
 */
export function buildTsWorkbook(XLSX, {ticker, tabName, trades, bars, rules, costs, mae = /** @type {number[] | null} */ (null)}) {
  const first = 21, last = first + trades.length - 1, nb = bars.d.length;
  const book = XLSX.utils.book_new();

  // All Index Explore
  const ix = {};
  TPL.idx_hdr.forEach(c => put(ix, c));
  const sRow = TPL.idx_row.reduce((m, c) => (m[c.a.replace(/\d+/, '')] = c.s, m), {});
  bars.d.forEach((d, i) => {
    const r = i + 2;
    put(ix, {a: 'A' + r, v: bars.s ? bars.s[i] : ticker, s: sRow.A}); put(ix, {a: 'B' + r, v: undefined, s: sRow.B}); ix['B' + r] = {t: 'n', v: serial(d), s: STYLES[sRow.B]};
    ['o', 'h', 'l', 'c'].forEach((k, j) => { const col = 'CDEF'[j]; ix[col + r] = {t: 'n', v: bars[k][i], s: STYLES[sRow[col]]}; });
  });
  ix['!ref'] = XLSX.utils.encode_range(extent(ix));
  ix['!cols'] = widths({A: 8.42578125, B: 10.42578125});

  // Rules (label in B, value in C)
  const rl = {};
  const rv = {2: rules.timeframe, 3: rules.index, 4: rules.period, 5: rules.aflPath || '', 6: rules.doneBy || '', 7: rules.verifiedBy || '', 9: tabName, 10: rules.position, 11: rules.filter, 12: rules.setup, 13: rules.entry, 14: rules.exit};
  TPL.rules.forEach(c => { const r = +c.a.replace(/\D/g, ''); put(rl, c, c.a[0] === 'C' && r in rv ? {v: rv[r] === '' ? undefined : rv[r]} : {}); });
  rl['!ref'] = XLSX.utils.encode_range(extent(rl)); rl['!cols'] = widths(TPL.rules_w);

  // the two summary sheets only carry the tab name
  const named = (cells, w) => { const s = {}; cells.forEach(c => put(s, c, c.v === REF_NAME ? {v: tabName} : {})); s['!ref'] = XLSX.utils.encode_range(extent(s)); s['!cols'] = widths(w); return s; };
  const sm = named(TPL.sum, TPL.sum_w), yy = named(TPL.yoy, TPL.yoy_w);
  yy['!rows'] = [undefined, {hpt: 18.75}, {hpt: 13.5}, {hpt: 63.75}, {hpt: 2.25}];

  // the trade sheet
  const ts = {};
  const tops = {C1: costs.impact, D1: costs.rollLong, E1: costs.rollShort};
  TPL.top.forEach(c => {
    const extra = {};
    if (c.a in tops) extra.v = tops[c.a];
    if (c.a === 'Y10') extra.v = rules.entry; if (c.a === 'Y11') extra.v = rules.exit; if (c.a === 'Y12') extra.v = rules.filter;
    put(ts, c, c.f ? {f: stretch(c.f, last, nb + 1)} : extra);
  });
  trades.forEach((t, i) => {
    const r = first + i;
    for (const [col, c] of Object.entries(TPL.row)) {
      if (col === 'A') put(ts, {...c, a: 'A' + r, v: t.sym ?? ticker});
      else if (col === 'B') put(ts, {...c, a: 'B' + r, v: t.dir});
      else if (col === 'C') put(ts, {...c, a: 'C' + r, v: undefined, d: t.entry});
      else if (col === 'D') put(ts, {...c, a: 'D' + r, v: t.entryPx});
      else if (col === 'E') put(ts, {...c, a: 'E' + r, v: undefined, d: t.exit});
      else if (col === 'F') put(ts, {...c, a: 'F' + r, v: t.exitPx});
      else put(ts, {...c, a: col + r, f: stretch(moveRow(c.f, r), last, nb + 1), F: c.F ? col + r : undefined});
    }
    if (mae) ts['W' + r].pre = mae[i];
  });
  TPL.year.forEach(c => put(ts, c, c.f ? {f: stretch(c.f, last, nb + 1)} : {}));
  ts['!ref'] = XLSX.utils.encode_range(extent(ts)); ts['!cols'] = widths(TPL.colw);

  XLSX.utils.book_append_sheet(book, ix, 'All Index Explore');
  XLSX.utils.book_append_sheet(book, rl, 'Rules');
  XLSX.utils.book_append_sheet(book, sm, '2008-2011 Summary');
  XLSX.utils.book_append_sheet(book, yy, 'YOY Summary');
  XLSX.utils.book_append_sheet(book, ts, tabName.slice(0, 31));
  evaluateWorkbook(book);                                  // store each formula's result next to it, as the team's file does
  book.Workbook = {CalcPr: {fullCalcOnLoad: true}};
  return book;
}

/** the template's MAE (column W) of one trade, as Excel works it out from the explore sheet: the lowest Low (Long) or highest High (Short) of the symbol's bars after the entry date up to the exit date, against the entry price; no bar gives MIN/MAX 0.
 *  rows = the trade's own bars [[YYYY-MM-DD, o, h, l, c]] (they cover its whole window, and the symbol's other rows on those dates are the same bars, so the result is the sheet's) */
export function maeOf(t, rows) {
  let lo = Infinity, hi = -Infinity, any = false;
  for (const [d, , h, l] of rows) { if (!(d > t.entry) || d > t.exit) continue; any = true; if (l < lo) lo = l; if (h > hi) hi = h; }
  return t.dir === 'Long' ? (any ? lo : 0) / t.entryPx - 1 : -((any ? hi : 0) / t.entryPx - 1);
}

/** the workbook as .xlsx bytes, deflated like Excel's own files (SheetJS's built-in compression leaves them about 3x bigger; without CompressionStream it is the fallback) */
export async function tsBytes(XLSX, book) {
  const opts = {bookType: 'xlsx', type: 'array', cellStyles: true, sheetStubs: true};
  if (typeof CompressionStream === 'undefined' || !XLSX.CFB) return XLSX.write(book, {...opts, compression: true});
  const cfb = XLSX.CFB.read(new Uint8Array(XLSX.write(book, opts)), {type: 'array'});
  const files = cfb.FileIndex.map((f, i) => [cfb.FullPaths[i].replace(/^Root Entry\//, ''), f]).filter(([n, f]) => f.type === 2 && f.content && !n.startsWith('\u0001'))
    .map(([n, f]) => [n, f.content instanceof Uint8Array ? f.content : Uint8Array.from(f.content)]);
  files.sort((a, b) => (b[0] === '[Content_Types].xml') - (a[0] === '[Content_Types].xml'));
  return zipDeflate(files);
}

/** a stock basket's All Index Explore: per trade only the bar its MAE formula picks (lowest Low after entry up to exit for Long, highest High for Short).
 *  The formula takes MIN/MAX over the symbol's rows in the trade's window, and every other row there is a bar of the same window, so the result is the same as with all bars. */
export function maeBars(trades) {
  const bars = {d: [], o: [], h: [], l: [], c: [], s: []};
  for (const t of trades) {
    let k = null;
    for (const b of t.bars) { if (!(b[0] > t.entry) || b[0] > t.exit) continue; if (!k || (t.dir === 'Long' ? b[3] < k[3] : b[2] > k[2])) k = b; }
    if (k) { bars.d.push(k[0]); bars.o.push(k[1]); bars.h.push(k[2]); bars.l.push(k[3]); bars.c.push(k[4]); bars.s.push(t.sym); }
  }
  return bars;
}
