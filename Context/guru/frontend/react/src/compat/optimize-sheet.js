// "Download optimization sheet": the research team's AmiBroker-style optimization workbook (handoff section 4.4, reference 04_optimization_sheet.xlsx).
// Sheet "Rules" (form) + sheet "Optimization": No., the 36 AmiBroker result-list columns, the optimized settings, then derived columns written as live
// Excel formulas on the same row. Sorted on CAR/MDD (blanks last), AutoFilter on, row 1 frozen, 28 columns hidden, recalculated on load.
// The file is written here (stored ZIP + the reference's own styles.xml) because the Excel library used elsewhere cannot freeze rows or hide gridlines.

export const AM_COLUMNS = ['Net Profit', 'Net % Profit', 'Exposure %', 'CAR', 'RAR', 'Max. Trade Drawdown', 'Max. Trade % Drawdown', 'Max. Sys Drawdown', 'Max. Sys % Drawdown',
  'Recovery Factor', 'CAR/MDD', 'RAR/MDD', 'Profit Factor', 'Payoff Ratio', 'Standard Error', 'RRR', 'Ulcer Index', 'Ulcer Perf. Index', 'Sharpe Ratio', 'K-Ratio', '# Trades',
  'Avg Profit/Loss', 'Avg % Profit/Loss', 'Avg Bars Held', '# of winners', '% of Winners', 'W. Tot. Profit', 'W. Avg. Profit', 'W. Avg % Profit', 'W. Avg. Bars Held',
  '# of losers', '% of Losers', 'L. Tot. Loss', 'L. Avg. Loss', 'L. Avg % Loss', 'L. Avg. Bars Held'];
const HIDDEN = new Set(['No.', 'Net Profit', 'Net % Profit', 'Exposure %', 'RAR', 'Max. Trade Drawdown', 'Max. Trade % Drawdown', 'Max. Sys Drawdown', 'Recovery Factor', 'RAR/MDD', 'Profit Factor', 'Payoff Ratio', 'Standard Error', 'RRR', 'Ulcer Index', 'Ulcer Perf. Index', 'Sharpe Ratio', 'K-Ratio', 'Avg Profit/Loss', '# of winners', 'W. Tot. Profit', 'W. Avg. Profit', 'W. Avg. Bars Held', '# of losers', '% of Losers', 'L. Tot. Loss', 'L. Avg. Loss', 'L. Avg. Bars Held']);
const INT_COLUMNS = new Set(['No.', '# Trades', '# of winners', '# of losers']);
export const DERIVED_FULL = ['Monthly Thickness', 'Expectancy Ratio', 'Final Avg % Profit/Loss', 'Final W Avg', 'Final L Avg', 'Final M. Thk', 'Final Expectancy Ratio', 'Total Profit', 'Total Net Profit', 'Holding Days'];
export const DERIVED_BOTH = ['Monthly Thickness', 'Expectancy Ratio', 'Total Profit', 'Holding Days'];

const colName = n => { let s = ''; for (n++; n > 0; n = Math.floor((n - 1) / 26)) s = String.fromCharCode(65 + ((n - 1) % 26)) + s; return s; };

/** the formulas of the derived columns for row r; `at(name)` gives the column letters of an AmiBroker, settings or derived column */
export function derivedFormulas(position, at, r) {
  const a = n => at(n) + r;
  const final = (avg, bars) => (position === 'Short' ? `${avg}+(((7/5*(${bars}-1))*0.0166)-0.3)` : `${avg}-(((7/5*(${bars}-1))*0.033)+0.3)`);
  const mt = `${a('Avg % Profit/Loss')}/(${a('Avg Bars Held')}-1)*21`;
  const ex = `((${a('W. Avg % Profit')}/ABS(${a('L. Avg % Loss')}))*${a('% of Winners')}-${a('% of Losers')})/100`;
  if (position === 'Long and short') return [mt, ex, `${a('# Trades')}*${a('Avg % Profit/Loss')}`, `${a('# Trades')}*(${a('Avg Bars Held')}-1)*7/5`];
  return [mt, ex,
    final(a('Avg % Profit/Loss'), a('Avg Bars Held')), final(a('W. Avg % Profit'), a('W. Avg. Bars Held')), final(a('L. Avg % Loss'), a('L. Avg. Bars Held')),
    `(${a('Final Avg % Profit/Loss')}/(${a('Avg Bars Held')}-1))*21`,
    `((${a('Final W Avg')}*(${a('% of Winners')}/100))-(ABS(${a('Final L Avg')})*(${a('% of Losers')}/100)))/(ABS(${a('Final L Avg')}))`,
    `${a('# Trades')}*${a('Avg % Profit/Loss')}`, `${a('# Trades')}*${a('Final Avg % Profit/Loss')}`, `${a('# Trades')}*(${a('Avg Bars Held')}-1)*7/5`];
}

/**
 * the same derived columns as numbers (what the page shows), keyed by column name; `am` is a row of the 36 AmiBroker columns
 * @returns {Record<string, number>}
 */
export function derivedValues(position, am) {
  const g = n => am[AM_COLUMNS.indexOf(n)];
  const nz = v => (v === null || v === undefined ? NaN : v);
  const avg = nz(g('Avg % Profit/Loss')), bars = nz(g('Avg Bars Held')), nT = nz(g('# Trades')), pw = nz(g('% of Winners')), pl = nz(g('% of Losers'));
  const wAvg = nz(g('W. Avg % Profit')), wBars = nz(g('W. Avg. Bars Held')), lAvg = nz(g('L. Avg % Loss')), lBars = nz(g('L. Avg. Bars Held'));
  const final = (a, b) => (position === 'Short' ? a + (((7 / 5 * (b - 1)) * 0.0166) - 0.3) : a - (((7 / 5 * (b - 1)) * 0.033) + 0.3));
  const out = {'Monthly Thickness': avg / (bars - 1) * 21, 'Expectancy Ratio': ((wAvg / Math.abs(lAvg)) * pw - pl) / 100, 'Total Profit': nT * avg, 'Holding Days': nT * (bars - 1) * 7 / 5};
  if (position !== 'Long and short') {
    const fAvg = final(avg, bars), fW = final(wAvg, wBars), fL = final(lAvg, lBars);
    Object.assign(out, {'Final Avg % Profit/Loss': fAvg, 'Final W Avg': fW, 'Final L Avg': fL, 'Final M. Thk': (fAvg / (bars - 1)) * 21,
      'Final Expectancy Ratio': ((fW * (pw / 100)) - (Math.abs(fL) * (pl / 100))) / Math.abs(fL), 'Total Net Profit': nT * fAvg});
  }
  return out;
}

// ---------------------------------------------------------------- the file (OOXML parts copied from the reference workbook)
export const STYLES = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><fonts count="4"><font><sz val="10"/><color rgb="FF000000"/><name val="Calibri"/></font><font><b/><sz val="10"/><color rgb="FFFFFFFF"/><name val="Calibri"/></font><font><b/><sz val="11"/><color rgb="FFFFFFFF"/><name val="Calibri"/></font><font><b/><sz val="10"/><color rgb="FF2C3E50"/><name val="Calibri"/></font></fonts><fills count="6"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill><fill><patternFill patternType="solid"><fgColor rgb="FF34495E"/><bgColor indexed="64"/></patternFill></fill><fill><patternFill patternType="solid"><fgColor rgb="FF1F3864"/><bgColor indexed="64"/></patternFill></fill><fill><patternFill patternType="solid"><fgColor rgb="FFF2F6FA"/><bgColor indexed="64"/></patternFill></fill><fill><patternFill patternType="solid"><fgColor rgb="FFF9FBFD"/><bgColor indexed="64"/></patternFill></fill></fills><borders count="2"><border><left/><right/><top/><bottom/><diagonal/></border><border><left style="thin"><color rgb="FFB0C4D8"/></left><right style="thin"><color rgb="FFB0C4D8"/></right><top style="thin"><color rgb="FFB0C4D8"/></top><bottom style="thin"><color rgb="FFB0C4D8"/></bottom><diagonal/></border></borders><cellStyleXfs count="1"><xf/></cellStyleXfs><cellXfs count="15"><xf fontId="0"/><xf fontId="1" fillId="2" borderId="1" applyFont="1" applyFill="1" applyBorder="1" applyAlignment="1"><alignment horizontal="center" vertical="center" wrapText="1"/></xf><xf fontId="2" fillId="3" applyFont="1" applyFill="1" applyAlignment="1"><alignment vertical="center"/></xf><xf fontId="3" fillId="4" borderId="1" applyFont="1" applyFill="1" applyBorder="1" applyAlignment="1"><alignment vertical="center"/></xf><xf fontId="0" fillId="0" borderId="1" numFmtId="0" applyFont="1" applyFill="1" applyBorder="1" applyNumberFormat="1" applyAlignment="1"><alignment horizontal="center" vertical="center" wrapText="1"/></xf><xf fontId="0" fillId="0" borderId="1" numFmtId="1" applyFont="1" applyFill="1" applyBorder="1" applyNumberFormat="1" applyAlignment="1"><alignment horizontal="center" vertical="center"/></xf><xf fontId="0" fillId="0" borderId="1" numFmtId="4" applyFont="1" applyFill="1" applyBorder="1" applyNumberFormat="1" applyAlignment="1"><alignment horizontal="center" vertical="center"/></xf><xf fontId="0" fillId="0" borderId="1" numFmtId="10" applyFont="1" applyFill="1" applyBorder="1" applyNumberFormat="1" applyAlignment="1"><alignment horizontal="center" vertical="center"/></xf><xf fontId="0" fillId="0" borderId="1" numFmtId="3" applyFont="1" applyFill="1" applyBorder="1" applyNumberFormat="1" applyAlignment="1"><alignment horizontal="center" vertical="center"/></xf><xf fontId="0" fillId="5" borderId="1" numFmtId="0" applyFont="1" applyFill="1" applyBorder="1" applyNumberFormat="1" applyAlignment="1"><alignment horizontal="center" vertical="center" wrapText="1"/></xf><xf fontId="0" fillId="5" borderId="1" numFmtId="1" applyFont="1" applyFill="1" applyBorder="1" applyNumberFormat="1" applyAlignment="1"><alignment horizontal="center" vertical="center"/></xf><xf fontId="0" fillId="5" borderId="1" numFmtId="4" applyFont="1" applyFill="1" applyBorder="1" applyNumberFormat="1" applyAlignment="1"><alignment horizontal="center" vertical="center"/></xf><xf fontId="0" fillId="5" borderId="1" numFmtId="10" applyFont="1" applyFill="1" applyBorder="1" applyNumberFormat="1" applyAlignment="1"><alignment horizontal="center" vertical="center"/></xf><xf fontId="0" fillId="5" borderId="1" numFmtId="3" applyFont="1" applyFill="1" applyBorder="1" applyNumberFormat="1" applyAlignment="1"><alignment horizontal="center" vertical="center"/></xf><xf fontId="0" borderId="1" applyFont="1" applyBorder="1" applyAlignment="1"><alignment horizontal="left" vertical="center" indent="1" wrapText="1"/></xf></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>';
export const XML = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n';
export const NS = 'xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"';
const CONTENT_TYPES = XML + '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/worksheets/sheet2.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>';
export const RELS = XML + '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>';
const WORKBOOK = XML + '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Rules" sheetId="1" r:id="rId1"/><sheet name="Optimization" sheetId="2" r:id="rId2"/></sheets><calcPr fullCalcOnLoad="1"/></workbook>';
const WB_RELS = XML + '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>';

export const esc = s => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
export const text = (a, s, v) => `<c r="${a}" s="${s}" t="inlineStr"><is><t xml:space="preserve">${esc(v)}</t></is></c>`;
export const num = (a, s, v) => `<c r="${a}" s="${s}"><v>${v}</v></c>`;
const formula = (a, s, f) => `<c r="${a}" s="${s}"><f>${esc(f)}</f></c>`;
const MONEY = new Set(['Net Profit', 'Max. Trade Drawdown', 'Max. Sys Drawdown', 'Standard Error', 'Avg Profit/Loss', 'W. Tot. Profit', 'W. Avg. Profit', 'L. Tot. Loss', 'L. Avg. Loss']);  // a whole-number amount shows as #,##0
const S_HEAD = 1, S_LABEL = 3, S_VALUE = 14, S_INT = 5, S_NUM = 6;

// ---- stored (uncompressed) ZIP
const CRC = (() => { const t = new Uint32Array(256); for (let n = 0; n < 256; n++) { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xEDB88320 ^ (c >>> 1) : c >>> 1; t[n] = c >>> 0; } return t; })();
const crc32 = b => { let c = 0xFFFFFFFF; for (let i = 0; i < b.length; i++) c = CRC[(c ^ b[i]) & 255] ^ (c >>> 8); return (c ^ 0xFFFFFFFF) >>> 0; };
export function zipStored(files) {
  const enc = new TextEncoder(), parts = [], central = [];
  let offset = 0;
  const u16 = n => [n & 255, (n >>> 8) & 255], u32 = n => [n & 255, (n >>> 8) & 255, (n >>> 16) & 255, (n >>> 24) & 255];
  for (const [name, content] of files) {
    const nm = enc.encode(name), data = typeof content === 'string' ? enc.encode(content) : content, crc = crc32(data);
    const local = Uint8Array.from([0x50, 0x4B, 3, 4, ...u16(20), ...u16(0x0800), ...u16(0), ...u16(0), ...u16(0x21), ...u32(crc), ...u32(data.length), ...u32(data.length), ...u16(nm.length), ...u16(0)]);
    parts.push(local, nm, data);
    central.push(Uint8Array.from([0x50, 0x4B, 1, 2, ...u16(20), ...u16(20), ...u16(0x0800), ...u16(0), ...u16(0), ...u16(0x21), ...u32(crc), ...u32(data.length), ...u32(data.length), ...u16(nm.length), ...u16(0), ...u16(0), ...u16(0), ...u16(0), ...u32(0), ...u32(offset)]), nm);
    offset += local.length + nm.length + data.length;
  }
  const cdSize = central.reduce((n, c) => n + c.length, 0);
  const end = Uint8Array.from([0x50, 0x4B, 5, 6, ...u16(0), ...u16(0), ...u16(files.length), ...u16(files.length), ...u32(cdSize), ...u32(offset), ...u16(0)]);
  const all = [...parts, ...central, end], out = new Uint8Array(all.reduce((n, p) => n + p.length, 0));
  let o = 0; for (const p of all) { out.set(p, o); o += p.length; }
  return out;
}

/** deflated ZIP (as Excel writes its own files), with the browser's native deflate; the files are [name, string | bytes] */
export async function zipDeflate(files) {
  const enc = new TextEncoder(), parts = [], central = [];
  let offset = 0;
  const u16 = n => [n & 255, (n >>> 8) & 255], u32 = n => [n & 255, (n >>> 8) & 255, (n >>> 16) & 255, (n >>> 24) & 255];
  for (const [name, content] of files) {
    const nm = enc.encode(name), data = typeof content === 'string' ? enc.encode(content) : content, crc = crc32(data);
    const z = new Uint8Array(await new Response(new ReadableStream({start(c) { c.enqueue(data); c.close(); }}).pipeThrough(new CompressionStream('deflate-raw'))).arrayBuffer());
    const local = Uint8Array.from([0x50, 0x4B, 3, 4, ...u16(20), ...u16(0x0800), ...u16(8), ...u16(0), ...u16(0x21), ...u32(crc), ...u32(z.length), ...u32(data.length), ...u16(nm.length), ...u16(0)]);
    parts.push(local, nm, z);
    central.push(Uint8Array.from([0x50, 0x4B, 1, 2, ...u16(20), ...u16(20), ...u16(0x0800), ...u16(8), ...u16(0), ...u16(0x21), ...u32(crc), ...u32(z.length), ...u32(data.length), ...u16(nm.length), ...u16(0), ...u16(0), ...u16(0), ...u16(0), ...u32(0), ...u32(offset)]), nm);
    offset += local.length + nm.length + z.length;
  }
  const cdSize = central.reduce((n, c) => n + c.length, 0);
  const end = Uint8Array.from([0x50, 0x4B, 5, 6, ...u16(0), ...u16(0), ...u16(files.length), ...u16(files.length), ...u32(cdSize), ...u32(offset), ...u16(0)]);
  const all = [...parts, ...central, end], out = new Uint8Array(all.reduce((n, p) => n + p.length, 0));
  let o = 0; for (const p of all) { out.set(p, o); o += p.length; }
  return out;
}

/** column width: the reference sizes each column to its header words and its longest value (min 8, max 14) */
function widthOf(name, maxLen, derivedIndex) {
  if (derivedIndex !== undefined) return name === 'Final Avg % Profit/Loss' ? 14 : 13;
  const OVER = {'Net Profit': 14, 'Max. Sys Drawdown': 16, 'Standard Error': 14, 'W. Tot. Profit': 16, 'W. Avg. Profit': 14, 'L. Tot. Loss': 16, 'L. Avg. Loss': 13, 'Ulcer Index': 9, 'Avg Bars Held': 9, 'W. Avg. Bars Held': 9, 'L. Avg. Bars Held': 9};
  if (OVER[name]) return OVER[name];
  const word = Math.max(...name.split(/\s+/).map(w => w.length));
  return Math.max(8, Math.min(14, Math.max(maxLen + 3, word + 3)));
}
const shown = (v, int) => (int ? String(Math.round(v)) : Number.isInteger(v) ? String(v) : v.toLocaleString('en-US', {minimumFractionDigits: 2, maximumFractionDigits: 2}));

/**
 * rules:   {timeframe, index, period, brokerage, openPosition, equity, aflPath, strategy, position, filter, setup, entry:[lines], exit:[lines]}
 * labels:  names of the optimized settings (one column each, after the AmiBroker columns)
 * rows:    [{no, am: 36 numbers (null = blank) in AM_COLUMNS order, settings: numbers in `labels` order}]
 * returns  the .xlsx file as a Uint8Array
 */
export function buildOptimizationXlsx({rules, labels, rows}) {
  const position = rules.position === 'Short' ? 'Short' : rules.position === 'Long and short' ? 'Long and short' : 'Long';
  const derived = position === 'Long and short' ? DERIVED_BOTH : DERIVED_FULL;
  const header = ['No.', ...AM_COLUMNS, ...labels, ...derived];
  const letters = {}; header.forEach((h, i) => { letters[h] = colName(i); });
  const at = n => letters[n];
  const nAm = AM_COLUMNS.length;

  // ---- Rules sheet
  const R = {};
  const items = [[2, 'Timeframe:', rules.timeframe], [3, 'Index:', rules.index], [4, 'Period:', rules.period], [5, 'Brokerage:', rules.brokerage ?? 0], [6, 'Open Position:', rules.openPosition ?? 1],
    [7, '%Equity:', rules.equity], [8, 'AFL Path:', rules.aflPath], [9, 'AFL Final check by:'], [10, 'Done By:'], [11, 'Verified By:'], [13, 'Strategy:', rules.strategy], [14, 'Position:', rules.position], [15, 'Filter:', rules.filter], [16, 'Setup:', rules.setup]];
  for (const [r, label, v] of items) {
    R[r] = text('B' + r, S_LABEL, label);
    if (v !== undefined && v !== null && v !== '') R[r] += typeof v === 'number' ? num('C' + r, S_INT, v) : text('C' + r, S_VALUE, v);
  }
  let r = 17;
  const block = (label, lines) => (lines || []).forEach((line, i) => { R[r] = (i === 0 ? text('B' + r, S_LABEL, label) : '') + text('C' + r, S_VALUE, line); r++; });
  if (!(rules.entry || []).length) { R[17] = text('B17', S_LABEL, 'Entry:'); r = 18; } else block('Entry:', rules.entry);
  if (!(rules.exit || []).length) { R[r] = text('B' + r, S_LABEL, 'Exit:'); r++; } else block('Exit:', rules.exit);
  let rulesRows = '';
  for (let i = 1; i < r; i++) rulesRows += `<row r="${i}">${R[i] || ''}</row>`;
  const sheet1 = XML + `<worksheet ${NS}><sheetViews><sheetView workbookViewId="0" showGridLines="0"></sheetView></sheetViews><sheetFormatPr defaultRowHeight="18" customHeight="1"/><cols><col min="1" max="1" width="8" customWidth="1"/><col min="2" max="2" width="22" customWidth="1"/><col min="3" max="3" width="50" customWidth="1"/></cols><sheetData>${rulesRows}</sheetData></worksheet>`;

  // ---- Optimization sheet: sorted on CAR/MDD, blanks last, ties in the original order
  const ci = AM_COLUMNS.indexOf('CAR/MDD');
  const order = rows.map((x, i) => [x, i]).sort((p, q) => {
    const a = p[0].am[ci], b = q[0].am[ci], an = Number.isFinite(a), bn = Number.isFinite(b);
    if (an && bn) return b - a || p[1] - q[1];
    return an ? -1 : bn ? 1 : p[1] - q[1];
  }).map(x => x[0]);
  const maxLen = header.map(() => 0);
  const body = [];
  order.forEach((row, i) => {
    const rr = i + 2, band = rr % 2 === 1 ? 5 : 0;
    let cells = '';
    const add = (c, name, v) => {
      if (v === null || v === undefined || !Number.isFinite(v)) return;
      v = Math.round(v * 1e6) / 1e6;                                       // the lab's file keeps 6 decimals
      const int = INT_COLUMNS.has(name) || Number.isInteger(v);          // a whole number gets the integer format wherever it stands
      cells += num(colName(c) + rr, (int ? (MONEY.has(name) ? 8 : S_INT) : S_NUM) + band, v);
      maxLen[c] = Math.max(maxLen[c], shown(v, int).length);
    };
    add(0, 'No.', row.no);
    row.am.forEach((v, k) => add(1 + k, AM_COLUMNS[k], v));
    row.settings.forEach((v, k) => add(1 + nAm + k, labels[k], v));
    derivedFormulas(position, at, rr).forEach((f, k) => { cells += formula(colName(1 + nAm + labels.length + k) + rr, S_NUM + band, f); });
    body.push(`<row r="${rr}">${cells}</row>`);
  });
  const last = colName(header.length - 1), ref = `A1:${last}${order.length + 1}`;
  const cols = header.map((h, i) => {
    const w = widthOf(h, maxLen[i], i > nAm + labels.length ? i : undefined);
    return `<col min="${i + 1}" max="${i + 1}" width="${w}"${HIDDEN.has(h) ? ' hidden="1"' : ''} customWidth="1"/>`;
  }).join('');
  const head = header.map((h, i) => text(colName(i) + '1', S_HEAD, h)).join('');
  const sheet2 = XML + `<worksheet ${NS}><sheetViews><sheetView workbookViewId="0"><pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/></sheetView></sheetViews><sheetFormatPr defaultRowHeight="18" customHeight="1"/><cols>${cols}</cols><sheetData><row r="1" ht="62" customHeight="1">${head}</row>${body.join('')}</sheetData><autoFilter ref="${ref}"/></worksheet>`;

  return zipStored([['[Content_Types].xml', CONTENT_TYPES], ['_rels/.rels', RELS], ['xl/workbook.xml', WORKBOOK], ['xl/_rels/workbook.xml.rels', WB_RELS], ['xl/styles.xml', STYLES],
    ['xl/worksheets/sheet1.xml', sheet1], ['xl/worksheets/sheet2.xml', sheet2]]);
}

/** the Rules lines of an optimized setting: "<label> - from,to,step" */
export const rangeLine = (label, from, to, step) => `${label} - ${from},${to},${step}`;
/** file name: "<instrument>[ <filter>] <Long|Short|Long Short> Daily Optims.xlsx" */
export const optimsFileName = (instrument, position, filter) =>
  `${[instrument, filter, position === 'Long and short' ? 'Long Short' : position, 'Daily Optims'].filter(Boolean).join(' ').replace(/[\\/:*?"<>|]/g, ' ').replace(/\s+/g, ' ').trim()}.xlsx`;
