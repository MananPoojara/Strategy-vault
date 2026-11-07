// The lab's own Excel look for every table download except Download TS (handoff section 4, "xlsx.ts"): Calibri 10, slate header, navy block titles,
// thin pale borders, every other data row shaded, cells typed from their displayed text ("12.50%" -> 0.125 shown as 0.00%, "1,234.50" -> 1234.5).
// Same styles.xml as the optimization sheet, written without the Excel library (it cannot freeze rows or hide gridlines).
import {STYLES, XML, NS, RELS, zipStored, esc, text, num} from './optimize-sheet.js';

const REL = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships';
const S_TITLE = 2, S_HEAD = 1, S_TEXT = 4, S_INT = 5, S_NUM = 6, S_PCT = 7, S_THOU = 8;   // + 5 = the shaded twin

/** a displayed cell -> {s: base style, n: number} or {s: S_TEXT, t: text}; null/undefined/'' = blank */
export function parseCell(v) {
  if (v === null || v === undefined || v === '') return null;
  const intStyle = n => (Math.abs(n) >= 10000 ? S_THOU : S_INT);
  if (typeof v === 'number') return Number.isFinite(v) ? {s: Number.isInteger(v) ? intStyle(v) : S_NUM, n: v} : null;
  const t = String(v);
  if (/^-?\d[\d,]*(\.\d+)?%$/.test(t)) return {s: S_PCT, n: Number(t.slice(0, -1).replace(/,/g, '')) / 100};
  if (/^-?\d[\d,]*(\.\d+)?$/.test(t)) { const n = Number(t.replace(/,/g, '')); return t.includes('.') ? {s: S_NUM, n} : {s: intStyle(n), n}; }
  return {s: S_TEXT, t};
}

const letters = i => { let s = ''; for (i++; i > 0; i = Math.floor((i - 1) / 26)) s = String.fromCharCode(65 + ((i - 1) % 26)) + s; return s; };
const grp = (n, d) => n.toLocaleString('en-US', {minimumFractionDigits: d, maximumFractionDigits: d});
/** characters a cell needs: text and percentages as shown, other numbers without their sign and with the decimal point counting double (fits the lab's own file) */
function shown(c) {
  if (!c) return 0;
  if (c.t !== undefined) return c.t.length;
  const a = Math.abs(c.n);
  if (c.s === S_PCT) return (c.n < 0 ? 1 : 0) + (grp(a * 100, 2) + '%').length;
  const d = c.s === S_NUM ? grp(a, 2) : c.s === S_THOU ? grp(a, 0) : String(Math.round(a));
  return d.length + (d.includes('.') ? 1 : 0);
}

/** rows: arrays of displayed cells; headRows: indexes of header rows (their longest word sets a minimum width); titleRows: block titles */
function sheetXml({rows, titleRows = new Set(), headRows = new Set(), blocks, freeze}) {
  const parsed = rows.map(r => (r || []).map(parseCell));
  const wide = Math.max(1, ...rows.map(r => (r || []).length));
  const width = Array.from({length: wide}, (_, c) => {
    let w = 0;
    rows.forEach((r, i) => {
      if (titleRows.has(i)) return;
      if (headRows.has(i)) { String((r || [])[c] ?? '').split(/\s+/).forEach(x => { w = Math.max(w, x.length); }); return; }
      w = Math.max(w, shown(parsed[i][c]));
    });
    return Math.max(8, Math.min(48, Math.floor(w + 3)));
  });
  let band = 0, body = '';
  rows.forEach((r, i) => {
    const rr = i + 1;
    if (titleRows.has(i)) { body += `<row r="${rr}" ht="22" customHeight="1">${text('A' + rr, S_TITLE, r[0])}</row>`; band = 0; return; }
    if (headRows.has(i)) { body += `<row r="${rr}" ht="34" customHeight="1">${(r || []).map((h, c) => (h === '' ? '' : text(letters(c) + rr, S_HEAD, h))).join('')}</row>`; band = 0; return; }
    if (!(r || []).length) { body += `<row r="${rr}"></row>`; return; }
    const shade = band++ % 2 === 1 ? 5 : 0;
    body += `<row r="${rr}">${parsed[i].map((c, k) => (c ? (c.t !== undefined ? text(letters(k) + rr, c.s + shade, c.t) : num(letters(k) + rr, c.s + shade, c.n)) : '')).join('')}</row>`;
  });
  const view = freeze ? '<sheetViews><sheetView workbookViewId="0"><pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/></sheetView></sheetViews>'
    : '<sheetViews><sheetView workbookViewId="0" showGridLines="0"></sheetView></sheetViews>';
  return XML + `<worksheet ${NS}>${view}<sheetFormatPr defaultRowHeight="18" customHeight="1"/><cols>${width.map((w, c) => `<col min="${c + 1}" max="${c + 1}" width="${w}" customWidth="1"/>`).join('')}</cols><sheetData>${body}</sheetData></worksheet>`;
}

/** layout 'blocks': blocks = [{title, header: ['', 'All trades', ...], rows: [[label, ...]]}], each followed by an empty row.
 *  layout 'table':  header = [...], rows = [[...]]  (first row frozen).  */
export function labSheet(spec) {
  if (spec.layout === 'table') return sheetXml({rows: [spec.header, ...spec.rows], headRows: new Set([0]), freeze: true});
  const rows = [], titleRows = new Set(), headRows = new Set();
  for (const b of spec.blocks) {
    titleRows.add(rows.length); rows.push([b.title]);
    headRows.add(rows.length); rows.push(b.header);
    b.rows.forEach(r => rows.push(r)); rows.push([]);
  }
  return sheetXml({rows, titleRows, headRows});
}

/** sheets: [{name, layout, ...}] -> the .xlsx bytes */
export function buildLabXlsx(sheets) {
  const n = sheets.length;
  const names = sheets.map(s => String(s.name).replace(/[\[\]:*?/\\]/g, ' ').slice(0, 31));
  const ct = XML + '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>'
    + sheets.map((_, i) => `<Override PartName="/xl/worksheets/sheet${i + 1}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>`).join('') + '</Types>';
  const wb = XML + `<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="${REL}"><sheets>${names.map((nm, i) => `<sheet name="${esc(nm)}" sheetId="${i + 1}" r:id="rId${i + 1}"/>`).join('')}</sheets></workbook>`;
  const rels = XML + '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
    + sheets.map((_, i) => `<Relationship Id="rId${i + 1}" Type="${REL}/worksheet" Target="worksheets/sheet${i + 1}.xml"/>`).join('') + `<Relationship Id="rId${n + 1}" Type="${REL}/styles" Target="styles.xml"/></Relationships>`;
  return zipStored([['[Content_Types].xml', ct], ['_rels/.rels', RELS], ['xl/workbook.xml', wb], ['xl/_rels/workbook.xml.rels', rels], ['xl/styles.xml', STYLES],
    ...sheets.map((s, i) => [`xl/worksheets/sheet${i + 1}.xml`, labSheet(s)])]);
}
