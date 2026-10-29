// Excel tradesheet in the research team's own template layout, for one instrument's trade list.
//
// Layout, formulas, number formats, fonts, fills, borders, column widths and row heights are copied from the two
// templates on the research share (Ritik Jain): "#Tradesheet Template Long & Short - For TS from 2008.xlsx" (format
// "2008", trade sheet "1") and "Template & Rule & Others/TS_ From 2000.xlsx" (format "2000", trade sheet "4"); the data
// block at the end of this file was extracted from those files by script, shared formulas expanded. Every formula cell is
// written with the template's formula text AND a value computed here by a small evaluator of exactly that text, so the
// file shows numbers before Excel recalculates. Excel recomputes everything on open; the evaluator only fills the cache.
//
// Sheets kept: 'All Index Explore' (the bars the MAE formula reads), 'All Sheets Name' (the list the summaries' INDIRECT
// reads; this one trade sheet is its B5), 'Rules', '2008-2011 Summary', 'YOY Summary', then the trade sheet. Dropped:
// 'Sheet_List' and 'Macro for sheet name' (a VBA listing and its output; no formula reads them), the empty
// "Template (n)" sheets and the sample-data sheets. The 2000 template has no Rules sheet; both formats use the 2008 one
// (the filled example on the share has it in the same place). Not reproduced, because the vendored SheetJS writer has
// no support for them: frozen panes, hidden gridlines, zoom, tab colours and the 2008 sheet's conditional formats.

export type TsFormat = "2000" | "2008";
export const defaultTsFormat = (from: string): TsFormat => (from < "2008-01-01" ? "2000" : "2008");
/** The templates' C1, D1, E1: impact cost per trade, rollover cost per 30 days long and short. */
export const TS_COSTS = { impact: 0.003, rollLong: 0.01, rollShort: -0.005 };

export interface TsInput {
  format: TsFormat;
  sheetName: string;
  symbol: string;
  trades: { dir: 1 | -1; entry: string; exit: string; entry_px: number; exit_px: number; open?: boolean }[];
  ohlc: { date: string; open: number; high: number; low: number; close: number }[];
  costs: { impact: number; rollLong: number; rollShort: number };
  rules: { timeframe: string; index: string; period: string; strategy: string; position: string; filter: string; setup: string; entry: string[]; exit: string[] };
  /** Several trade sheets in one book (same entry, one sheet per exit variant), in this order; replaces sheetName/trades. */
  sheets?: { sheetName: string; trades: TsInput["trades"]; exitLabel?: string }[];
}
/** The YOY Summary has 20 sheet columns. */
export const TS_MAX_SHEETS = 20;

const EXPLORE = "All Index Explore", NAMES = "All Sheets Name", RULES = "Rules", SUMMARY = "2008-2011 Summary", YOY = "YOY Summary";
/** Last trade column (X / W) and the column that holds the trade sheet's own Entry/Exit/Filter lines (rows 10-12). */
const LAYOUT: Record<TsFormat, { lastCol: number; ruleCol: number }> = { "2008": { lastCol: 24, ruleCol: 16 }, "2000": { lastCol: 23, ruleCol: 25 } };

/** Excel sheet name: ≤31 chars, none of []:*?/\ and no apostrophes at all — the summaries' INDIRECT("'"&name&"'!E3") cannot quote them. */
export function tsSheetName(raw: string): string {
  let n = raw.replace(/[[\]:*?/\\']/g, " ").replace(/\s+/g, " ").trim().slice(0, 31).trim() || "Trades";
  if ([EXPLORE, NAMES, RULES, SUMMARY, YOY].some((f) => f.toLowerCase() === n.toLowerCase())) n = `TS ${n}`.slice(0, 31);
  return n;
}
/** Sanitised names, made unique (case-insensitively) with a " (2)", " (3)" … suffix. */
export function tsSheetNames(raw: string[]): string[] {
  const seen = new Set<string>();
  return raw.map((r) => {
    let n = tsSheetName(r);
    for (let i = 2; seen.has(n.toLowerCase()); i++) n = `${tsSheetName(r).slice(0, 31 - ` (${i})`.length).trimEnd()} (${i})`;
    seen.add(n.toLowerCase());
    return n;
  });
}
export const tsFileName = (input: TsInput) => `TS_${tsSheetName(input.sheetName)}.xlsx`.replace(/[\\/:*?"<>|]+/g, " ");

let xlsxLoad: Promise<any> | undefined;
/** The app's vendored SheetJS-with-styles, loaded once on demand (global XLSX). */
export function loadXlsxStyle(): Promise<any> {
  const w = window as unknown as { XLSX?: any };
  if (w.XLSX?.utils) return Promise.resolve(w.XLSX);
  return (xlsxLoad ??= new Promise((ok, fail) => {
    const s = document.createElement("script");
    s.src = "vendor/xlsx-js-style-1.2.0.min.js";
    s.onload = () => (w.XLSX?.utils ? ok(w.XLSX) : fail(new Error("The Excel library did not load.")));
    s.onerror = () => { xlsxLoad = undefined; fail(new Error("Could not load the Excel library.")); };
    document.head.appendChild(s);
  }));
}

const EPOCH = Date.UTC(1899, 11, 30);
const serial = (iso: string) => Math.round((Date.UTC(+iso.slice(0, 4), +iso.slice(5, 7) - 1, +iso.slice(8, 10)) - EPOCH) / 864e5);
const colNum = (c: string) => [...c.toUpperCase()].reduce((n, ch) => n * 26 + ch.charCodeAt(0) - 64, 0);
const colName = (n: number): string => (n > 26 ? colName(Math.floor((n - 1) / 26)) : "") + String.fromCharCode(65 + ((n - 1) % 26));
/** Moves a formula's relative references by dr rows and dc columns; text in quotes (INDIRECT targets) stays. */
const shiftRefs = (f: string, dr: number, dc: number) => f.split(/("[^"]*")/).map((part, i) => i % 2 ? part
  : part.replace(/(\$?)([A-Z]{1,3})(\$?)(\d+)(?![\w(])/g, (_, ca, c, ra, r) => `${ca}${ca ? c : colName(colNum(c) + dc)}${ra}${ra ? r : +r + dr}`)).join("");
const addr = (ref: string) => { const m = /^\$?([A-Za-z]{1,3})\$?(\d+)$/.exec(ref)!; return { r: +m[2], c: colNum(m[1]) }; };

export function buildTsWorkbook(XLSX: any, input: TsInput): any {
  const list = input.sheets?.length ? input.sheets : [{ sheetName: input.sheetName, trades: input.trades, exitLabel: undefined }];
  if (list.length > TS_MAX_SHEETS) throw new Error(`A tradesheet workbook holds at most ${TS_MAX_SHEETS} trade sheets (the YOY Summary has ${TS_MAX_SHEETS} columns); ${list.length} were given.`);
  const fmt = FORMATS[input.format], lay = LAYOUT[input.format], names = tsSheetNames(list.map((s) => s.sheetName));
  const book = new Book();
  const styleAt = (spec: SSheet, ref: string) => spec.cells.find((c) => c[0] === ref)?.[1] ?? -1;

  const ex = book.add(EXPLORE, fmt.explore, true);
  ex.place(fmt.explore.cells.filter((c) => addr(c[0]).r === 1));
  const exStyles = ["A2", "B2", "C2", "D2", "E2", "F2"].map((ref) => styleAt(fmt.explore, ref));
  input.ohlc.forEach((b, i) => [input.symbol, serial(b.date), b.open, b.high, b.low, b.close].forEach((v, c) => ex.put(i + 2, c + 1, v, exStyles[c])));

  book.add(NAMES, fmt.names).place([fmt.names.cells[0], ...names.map((n, i): SCell => [`B${5 + i}`, styleAt(fmt.names, "B5"), n])]);

  const rules = book.add(RULES, fmt.rules), label = styleAt(fmt.rules, "B2"), valueStyle = styleAt(fmt.rules, "C2"), r = input.rules;
  let row = 2;
  for (const [l, vals] of [["Timeframe:", [r.timeframe]], ["Index:", [r.index]], ["Period:", [r.period]], ["AFL Path:", []], ["Done By:", []], ["Verified By:", []],
    ["", []], ["Strategy:", [r.strategy]], ["Position:", [r.position]], ["Filter:", [r.filter]], ["Setup:", [r.setup]], ["Entry:", r.entry],
    ["Exit:", list.length > 1 ? list.map((s, i) => (s.exitLabel ? `${names[i]}: ${s.exitLabel}` : names[i])) : r.exit]] as [string, string[]][]) {
    rules.put(row, 2, l || null, label);
    (vals.length ? vals : [null]).forEach((v, i) => rules.put(row + i, 3, v, valueStyle));
    row += Math.max(1, vals.length);
  }

  // Summary: one row block per sheet (2 rows in 2008, 3 in 2000). Block 1 is the template's; blocks 2..n repeat its block 2
  // moved down, with the C formula's offset (-2, -4 … / -3, -6 …) following the template's pattern.
  const summary = book.add(SUMMARY, fmt.summary), size = input.format === "2008" ? 2 : 3;
  summary.place(fmt.summary.cells.filter((c) => addr(c[0]).r < 4));
  names.forEach((_, i) => {
    const dr = i ? (i - 1) * size : 0, from = i ? 4 + size : 4;
    summary.place(fmt.summary.cells.filter((c) => addr(c[0]).r >= from && addr(c[0]).r < from + size).map(([ref, s, v, f, a]): SCell => {
      const { r: row, c } = addr(ref);
      const ff = f === undefined ? f : row === from && c === 3 && i ? shiftRefs(f, dr, 0).replace(/-\d+\)$/, `-${size * i})`) : shiftRefs(f, dr, 0);
      return [`${colName(c)}${row + dr}`, s, c === 2 && v !== undefined && v !== null ? i + 1 : v, ff, a];
    }));
  });
  // YOY: column C reads sheet 1; sheet k gets column C's formulas moved right, with the template's own column styles.
  const yoy = book.add(YOY, fmt.yoy), colStyles = fmt.yoy.colStyles!;
  yoy.place(fmt.yoy.cells.filter((c) => !c[0].startsWith("C")));
  names.forEach((_, i) => {
    for (const [ref, s0, v, f, a] of fmt.yoy.cells.filter((c) => /^C\d/.test(c[0]))) {
      const { r: row } = addr(ref), s = i ? colStyles[Math.min(i - 1, colStyles.length - 1)][row - 1] : s0;
      yoy.put(row, 3 + i, row === 2 ? i + 1 : v ?? null, s, f === undefined ? f : shiftRefs(f, 0, i), a === 1);
    }
  });

  list.forEach((sheet, n) => {
    // The last trade may still be open: it is written like the others (Long/Short, which the templates' formulas know) with its
    // exit at the last bar's close, so the sheet values the open position; the Rules sheet says so.
    const trades = sheet.trades, first = fmt.trade.dataRow, last = first + trades.length - 1;
    // Keep the template's ranges unless the data outgrows them: then the range is stretched to the last trade / bar.
    const fit = (f: string) => f.replace(/('All Index Explore'!)?(\$?[A-Z]{1,3}\$?)(\d+):(\$?[A-Z]{1,3}\$?)(\d+)/g, (m, exp, a, r0, b, r1) => {
      const need = exp ? input.ohlc.length + 1 : colNum(a.replace(/\$/g, "")) <= lay.lastCol && +r0 >= first - 2 ? last : 0;
      return +r1 < need ? `${exp ?? ""}${a}${r0}:${b}${need}` : m;
    });
    const ts = book.add(names[n], fmt.trade);
    ts.place(fmt.trade.cells.map(([ref, s, v, f, a]) => [ref, s, v, f === undefined ? f : fit(f), a] as SCell));
    [input.costs.impact, input.costs.rollLong, input.costs.rollShort].forEach((v, i) => ts.put(1, 3 + i, v, styleAt(fmt.trade, "C1")));
    [r.entry.join(", "), sheet.exitLabel ?? r.exit.join(", "), r.filter].forEach((v, i) => ts.put(10 + i, lay.ruleCol, v, ts.cells.get(key(10 + i, lay.ruleCol))?.s ?? -1));
    trades.forEach((t, i) => {
      const at = first + i, tpl = i ? fmt.trade.next : fmt.trade.first;
      const vals = [input.symbol, t.dir === 1 ? "Long" : "Short", serial(t.entry), t.entry_px, serial(t.exit), t.exit_px];
      vals.forEach((v, c) => ts.put(at, c + 1, v, tpl[String.fromCharCode(65 + c)]?.[0] ?? -1));
      for (const [col, [s, f, a]] of Object.entries(tpl)) {
        if (f !== undefined) ts.put(at, colNum(col), null, s, fit(f.replace(/\{r\}/g, String(at)).replace(/\{p\}/g, String(at - 1))), a === 1);
        else if (colNum(col) > 6) ts.put(at, colNum(col), null, s);
      }
    });
    // Rows first, top-down, so the running CUMM/Peak chains are memoised before any header formula jumps to their end.
    for (const [k, c] of [...ts.cells].sort((x, y) => x[0] - y[0])) if (Math.floor(k / COLS) >= first) book.value(ts, Math.floor(k / COLS), k % COLS, c);
  });
  for (const g of book.sheets) for (const [k, c] of g.cells) book.value(g, Math.floor(k / COLS), k % COLS, c);

  const wb = XLSX.utils.book_new();
  for (const g of book.sheets) {
    const ws: Record<string, any> = {};
    for (const [k, c] of g.cells) {
      const a = XLSX.utils.encode_cell({ r: Math.floor(k / COLS) - 1, c: (k % COLS) - 1 }), v = c.v;
      if (v === null && c.s < 0) continue;
      // The writer drops empty cells, so a styled blank (fill, border, format) is written as empty text to keep its look.
      const o: Record<string, any> = v instanceof XErr ? { t: "e", v: ERR_CODE[v.code], w: v.code }
        : typeof v === "number" ? { t: "n", v } : typeof v === "string" ? { t: "s", v } : typeof v === "boolean" ? { t: "b", v } : { t: "s", v: "" };
      if (c.f !== undefined) o.f = c.f;
      if (c.arr) o.F = a;
      if (c.s >= 0) o.s = STYLES[c.s];
      ws[a] = o;
    }
    ws["!ref"] = XLSX.utils.encode_range({ s: { r: 0, c: 0 }, e: { r: Math.max(g.maxRow, 1) - 1, c: Math.max(g.maxCol, 1) - 1 } });
    ws["!cols"] = g.spec.cols.map((w) => (w ? { width: w } : undefined));
    const rows: ({ hpt: number } | undefined)[] = [];
    for (const [n, h] of Object.entries(g.spec.rows ?? {})) rows[+n - 1] = { hpt: h };
    if (rows.length) ws["!rows"] = rows;
    XLSX.utils.book_append_sheet(wb, ws, g.name);
  }
  return wb;
}

// ---- A small evaluator for the Excel formulas the templates use ----
// Scalars, 1-column ranges, array (CSE) arithmetic, and SUM AVERAGE COUNT MIN MAX COUNTIF AVERAGEIF SUMIF IF IFERROR
// INDEX MATCH(exact) ABS YEAR ROW NA INDIRECT. Ranges are clipped to a sheet's last used row (the rest is empty).
class XErr { constructor(readonly code: string) {} }
const ERR_CODE: Record<string, number> = { "#NULL!": 0, "#DIV/0!": 7, "#VALUE!": 15, "#REF!": 23, "#NAME?": 29, "#NUM!": 36, "#N/A": 42 };
const NA = new XErr("#N/A"), DIV0 = new XErr("#DIV/0!"), VALUE = new XErr("#VALUE!"), REF = new XErr("#REF!"), NUM = new XErr("#NUM!");
type S = number | string | boolean | null | XErr;
interface Ref { g: Grid; r0: number; c0: number; r1: number; c1: number }
type V = S | S[] | Ref;
interface Cell { v: S; f?: string; arr?: boolean; s: number; st?: 1 | 2 }
const COLS = 16384, key = (r: number, c: number) => r * COLS + c;

class Grid {
  cells = new Map<number, Cell>(); maxRow = 0; maxCol = 0;
  constructor(readonly name: string, readonly spec: SSheet, readonly cache?: Map<string, S[]>) {}
  put(r: number, c: number, v: S, s = -1, f?: string, arr = false) {
    this.cells.set(key(r, c), { v, s, f, arr });
    this.maxRow = Math.max(this.maxRow, r); this.maxCol = Math.max(this.maxCol, c);
  }
  place(cells: SCell[]) { for (const [ref, s, v, f, a] of cells) { const { r, c } = addr(ref); this.put(r, c, v ?? null, s, f, a === 1); } return this; }
}

type Node = { k: "v"; v: S } | { k: "r"; sheet?: string; r0: number; c0: number; r1: number; c1: number } | { k: "f"; name: string; a: Node[] } | { k: "neg"; a: Node } | { k: "op"; op: string; a: Node; b: Node };
const TOKEN = /\s*(?:("(?:[^"]|"")*")|('(?:[^']|'')+'!|[A-Za-z_][\w.]*!)?(\$?[A-Za-z]{1,3}\$?\d+(?::\$?[A-Za-z]{1,3}\$?\d+)?)(?![\w(])|(\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)|([A-Za-z_][\w.]*)(?=\s*\()|(TRUE|FALSE)\b|(<=|>=|<>|[-+*/^&=<>(),]))/y;

function parse(src: string): Node {
  const toks: { t: string; v: string; sheet?: string }[] = [];
  for (TOKEN.lastIndex = 0; TOKEN.lastIndex < src.length && !/^\s*$/.test(src.slice(TOKEN.lastIndex));) {
    const at = TOKEN.lastIndex, m = TOKEN.exec(src);
    if (!m) throw new Error(`Formula parse error at ${at}: ${src}`);
    if (m[1]) toks.push({ t: "s", v: m[1].slice(1, -1).replace(/""/g, '"') });
    else if (m[3]) toks.push({ t: "r", v: m[3], sheet: m[2] && m[2].slice(0, -1).replace(/^'(.*)'$/, "$1").replace(/''/g, "'") });
    else if (m[4]) toks.push({ t: "n", v: m[4] });
    else if (m[5]) toks.push({ t: "f", v: m[5].toUpperCase() });
    else if (m[6]) toks.push({ t: "b", v: m[6] });
    else toks.push({ t: "o", v: m[7] });
  }
  let i = 0;
  const peek = () => toks[i]?.t === "o" ? toks[i].v : "", eat = (v: string) => { if (peek() !== v) throw new Error(`Expected ${v} in ${src}`); i++; };
  const level = (ops: string[], next: () => Node) => (): Node => {
    let a = next();
    while (ops.includes(peek())) { const op = toks[i++].v; a = { k: "op", op, a, b: next() }; }
    return a;
  };
  const primary = (): Node => {
    const t = toks[i++];
    if (!t) throw new Error(`Unexpected end of ${src}`);
    if (t.t === "o" && (t.v === "-" || t.v === "+")) { const a = primary(); return t.v === "-" ? { k: "neg", a } : a; }
    if (t.t === "o" && t.v === "(") { const a = cmp(); eat(")"); return a; }
    if (t.t === "n") return { k: "v", v: +t.v };
    if (t.t === "s") return { k: "v", v: t.v };
    if (t.t === "b") return { k: "v", v: t.v === "TRUE" };
    if (t.t === "r") { const [p, q = p] = t.v.split(":").map(addr); return { k: "r", sheet: t.sheet, r0: p.r, c0: p.c, r1: q.r, c1: q.c }; }
    if (t.t === "f") {
      eat("("); const a: Node[] = [];
      while (peek() !== ")") { a.push(cmp()); if (peek() === ",") i++; }
      eat(")"); return { k: "f", name: t.v, a };
    }
    throw new Error(`Unexpected ${t.v} in ${src}`);
  };
  const pow = level(["^"], primary), mul = level(["*", "/"], pow), add = level(["+", "-"], mul), cat = level(["&"], add);
  const cmp = level(["=", "<>", "<", ">", "<=", ">="], cat);
  const root = cmp();
  if (i < toks.length) throw new Error(`Trailing tokens in ${src}`);
  return root;
}

const isRef = (v: V): v is Ref => typeof v === "object" && v !== null && !(v instanceof XErr) && !Array.isArray(v);
const toNum = (x: S): number | XErr => x === null ? 0 : typeof x === "number" ? x : typeof x === "boolean" ? +x : x instanceof XErr ? x : x.trim() !== "" && !isNaN(+x) ? +x : VALUE;
const truth = (x: S): boolean | XErr => x instanceof XErr ? x : typeof x === "string" ? (/^(true|false)$/i.test(x) ? /^t/i.test(x) : VALUE) : !!x;
const rank = (x: S) => typeof x === "number" ? 0 : typeof x === "string" ? 1 : 2;
function compare(x: S, y: S): number {
  if (x === null) x = typeof y === "string" ? "" : typeof y === "boolean" ? false : 0;
  if (y === null) y = typeof x === "string" ? "" : typeof x === "boolean" ? false : 0;
  if (x === y) return 0;
  if (rank(x) !== rank(y)) return rank(x) - rank(y);
  if (typeof x === "string") { const a = x.toLowerCase(), b = (y as string).toLowerCase(); return a < b ? -1 : a > b ? 1 : 0; }
  return +x - +(y as number);
}
const text = (x: S) => x === null ? "" : typeof x === "boolean" ? (x ? "TRUE" : "FALSE") : String(x);
function lift(x: S | S[], y: S | S[], fn: (a: S, b: S) => S): S | S[] {
  if (!Array.isArray(x) && !Array.isArray(y)) return fn(x, y);
  const n = Math.max(Array.isArray(x) ? x.length : 0, Array.isArray(y) ? y.length : 0), out: S[] = new Array(n);
  for (let i = 0; i < n; i++) out[i] = fn(Array.isArray(x) ? (i < x.length ? x[i] : NA) : x, Array.isArray(y) ? (i < y.length ? y[i] : NA) : y);
  return out;
}
const OPS: Record<string, (a: S, b: S) => S> = {
  "&": (a, b) => a instanceof XErr ? a : b instanceof XErr ? b : text(a) + text(b),
  ...Object.fromEntries(["=", "<>", "<", ">", "<=", ">="].map((op) => [op, (a: S, b: S): S => {
    if (a instanceof XErr) return a; if (b instanceof XErr) return b;
    const d = compare(a, b);
    return op === "=" ? d === 0 : op === "<>" ? d !== 0 : op === "<" ? d < 0 : op === ">" ? d > 0 : op === "<=" ? d <= 0 : d >= 0;
  }])),
  ...Object.fromEntries(["+", "-", "*", "/", "^"].map((op) => [op, (x: S, y: S): S => {
    const a = toNum(x), b = toNum(y);
    if (a instanceof XErr) return a; if (b instanceof XErr) return b;
    if (op === "/" && b === 0) return DIV0;
    const v = op === "+" ? a + b : op === "-" ? a - b : op === "*" ? a * b : op === "/" ? a / b : Math.pow(a, b);
    return Number.isFinite(v) ? v : NUM;
  }])),
};
function criterion(c: S): (x: S) => boolean {
  if (typeof c !== "string") return (x) => x !== null && !(x instanceof XErr) && compare(x, c) === 0;
  const [, op = "=", rhs] = /^(<=|>=|<>|<|>|=)?(.*)$/s.exec(c)!, n = rhs.trim() !== "" && !isNaN(+rhs) ? +rhs : rhs;
  return (x) => {
    if (x === null || x instanceof XErr || (typeof n === "number") !== (typeof x === "number")) return op === "<>";
    const d = compare(x, n);
    return op === "=" ? d === 0 : op === "<>" ? d !== 0 : op === "<" ? d < 0 : op === ">" ? d > 0 : op === "<=" ? d <= 0 : d >= 0;
  };
}

class Book {
  sheets: Grid[] = [];
  add(name: string, spec: SSheet, constant = false) { const g = new Grid(name, spec, constant ? new Map() : undefined); this.sheets.push(g); return g; }
  find(name: string) { return this.sheets.find((g) => g.name.toLowerCase() === name.toLowerCase()); }

  value(g: Grid, r: number, c: number, cell = g.cells.get(key(r, c))): S {
    if (!cell) return null;
    if (cell.f === undefined || cell.st === 2) return cell.v;
    if (cell.st === 1) return REF; // circular
    cell.st = 1;
    const v = this.ev(parse(cell.f), g, r);
    cell.v = this.scalar(v);
    cell.st = 2;
    return cell.v;
  }
  /** A formula's result as one cell shows it: a reference reads its cell (empty shows 0), an array its first item. */
  scalar(v: V): S {
    if (isRef(v)) v = this.value(v.g, v.r0, v.c0);
    if (Array.isArray(v)) v = v.length ? v[0] : VALUE;
    return v === null ? 0 : v;
  }
  values(x: Ref): S[] {
    const k = `${x.r0},${x.c0},${x.r1},${x.c1}`, hit = x.g.cache?.get(k);
    if (hit) return hit;
    const out: S[] = [];
    for (let r = x.r0; r <= Math.min(x.r1, x.g.maxRow); r++) for (let c = x.c0; c <= x.c1; c++) out.push(this.value(x.g, r, c));
    x.g.cache?.set(k, out);
    return out;
  }
  deref(v: V): S | S[] { return !isRef(v) ? v : v.r0 === v.r1 && v.c0 === v.c1 ? this.value(v.g, v.r0, v.c0) : this.values(v); }

  ev(n: Node, g: Grid, row: number): V {
    switch (n.k) {
      case "v": return n.v;
      case "r": { const s = n.sheet === undefined ? g : this.find(n.sheet); return s ? { g: s, r0: n.r0, c0: n.c0, r1: n.r1, c1: n.c1 } : REF; }
      case "neg": return lift(this.deref(this.ev(n.a, g, row)), 0, (a) => OPS["-"](0, a));
      case "op": return lift(this.deref(this.ev(n.a, g, row)), this.deref(this.ev(n.b, g, row)), OPS[n.op]);
      case "f": return this.call(n.name, n.a, g, row);
    }
  }
  /** Numbers of SUM/AVERAGE/MIN/MAX arguments: from ranges and arrays numbers only, a typed-in scalar is coerced. */
  numbers(args: V[], skipErrors = false): number[] | XErr {
    const out: number[] = [];
    for (const a of args) {
      const v = this.deref(a);
      for (const x of Array.isArray(v) || isRef(a) ? ([] as S[]).concat(v) : []) {
        if (x instanceof XErr) { if (!skipErrors) return x; } else if (typeof x === "number") out.push(x);
      }
      if (!Array.isArray(v) && !isRef(a) && v !== null) { const x = toNum(v); if (x instanceof XErr) { if (!skipErrors) return x; } else out.push(x); }
    }
    return out;
  }
  call(name: string, a: Node[], g: Grid, row: number): V {
    const arg = (i: number) => this.ev(a[i], g, row), val = (i: number) => this.deref(arg(i));
    const one = (i: number): S => { const v = val(i); return Array.isArray(v) ? (v.length ? v[0] : VALUE) : v; };
    const agg = (f: (xs: number[]) => S) => { const xs = this.numbers(a.map((_, i) => arg(i))); return xs instanceof XErr ? xs : f(xs); };
    const list = (v: V): S[] => isRef(v) ? this.values(v) : ([] as S[]).concat(this.deref(v));
    switch (name) {
      case "SUM": return agg((xs) => xs.reduce((s, x) => s + x, 0));
      case "AVERAGE": return agg((xs) => xs.length ? xs.reduce((s, x) => s + x, 0) / xs.length : DIV0);
      case "MIN": return agg((xs) => xs.length ? xs.reduce((m, x) => Math.min(m, x)) : 0);
      case "MAX": return agg((xs) => xs.length ? xs.reduce((m, x) => Math.max(m, x)) : 0);
      case "COUNT": { const xs = this.numbers(a.map((_, i) => arg(i)), true); return (xs as number[]).length; }
      case "COUNTIF": { const ok = criterion(one(1)); return list(arg(0)).filter(ok).length; }
      case "SUMIF": case "AVERAGEIF": {
        const ok = criterion(one(1)), keys = list(arg(0)), vals = a[2] ? list(arg(2)) : keys, hit: number[] = [];
        for (let i = 0; i < keys.length; i++) if (ok(keys[i])) { const x = vals[i] ?? null; if (x instanceof XErr) return x; if (typeof x === "number") hit.push(x); }
        const sum = hit.reduce((s, x) => s + x, 0);
        return name === "SUMIF" ? sum : hit.length ? sum / hit.length : DIV0;
      }
      case "IF": {
        const c = val(0);
        if (!Array.isArray(c)) { const t = truth(c); return t instanceof XErr ? t : t ? arg(1) : a[2] ? arg(2) : false; }
        const yes = val(1), no = a[2] ? val(2) : false;
        return c.map((x, i) => { const t = truth(x); return t instanceof XErr ? t : t ? (Array.isArray(yes) ? yes[i] ?? NA : yes) : (Array.isArray(no) ? no[i] ?? NA : no); });
      }
      case "IFERROR": {
        const v = val(0);
        if (Array.isArray(v)) { const alt = val(1); return v.map((x, i) => x instanceof XErr ? (Array.isArray(alt) ? alt[i] ?? NA : alt) : x); }
        return v instanceof XErr ? arg(1) : v;
      }
      case "INDEX": {
        const src = arg(0), r = toNum(one(1)), c = a[2] ? toNum(one(2)) : 1;
        if (r instanceof XErr) return r; if (c instanceof XErr) return c;
        if (isRef(src)) return r < 1 || c < 1 || r > src.r1 - src.r0 + 1 || c > src.c1 - src.c0 + 1 ? REF : this.value(src.g, src.r0 + Math.floor(r) - 1, src.c0 + Math.floor(c) - 1);
        const xs = list(src); return r < 1 || r > xs.length ? REF : xs[Math.floor(r) - 1];
      }
      case "MATCH": {
        const x = one(0); if (x instanceof XErr) return x;
        const i = list(arg(1)).findIndex((y) => y !== null && !(y instanceof XErr) && rank(y) === rank(x) && compare(x, y) === 0);
        return i < 0 ? NA : i + 1;
      }
      case "ABS": return lift(val(0), 0, (x) => { const n = toNum(x); return n instanceof XErr ? n : Math.abs(n); });
      case "YEAR": return lift(val(0), 0, (x) => { const n = toNum(x); return n instanceof XErr ? n : new Date(EPOCH + Math.floor(n) * 864e5).getUTCFullYear(); });
      case "ROW": { const x = a[0] ? arg(0) : null; return x && isRef(x) ? x.r0 : row; }
      case "NA": return NA;
      case "INDIRECT": {
        const t = one(0); if (t instanceof XErr) return t;
        const m = /^(?:'((?:[^']|'')+)'!|([^'!]+)!)?(\$?[A-Za-z]{1,3}\$?\d+)(?::(\$?[A-Za-z]{1,3}\$?\d+))?$/.exec(text(t).trim());
        const sheet = m && (m[1] ?? m[2]), s = m && (sheet ? this.find(sheet.replace(/''/g, "'")) : g);
        if (!m || !s) return REF;
        const p = addr(m[3]), q = m[4] ? addr(m[4]) : p;
        return { g: s, r0: p.r, c0: p.c, r1: q.r, c1: q.c };
      }
    }
    return new XErr("#NAME?");
  }
}

type Style = Record<string, unknown>;
/** Template cell: address, index into STYLES (-1: none), constant value, formula text, 1 = array (CSE) formula. */
type SCell = [ref: string, style: number, value?: string | number | null, formula?: string, array?: 1];
/** Trade-row template per column: style, formula with {r} = this row and {p} = previous row, 1 = array formula. */
type RowCell = [style: number, formula?: string, array?: 1];
interface SSheet { cols: number[]; rows?: Record<string, number>; cells: SCell[]; colStyles?: number[][] }
interface Fmt {
  trade: SSheet & { dataRow: number; first: Record<string, RowCell>; next: Record<string, RowCell> };
  summary: SSheet; yoy: SSheet; names: SSheet; explore: SSheet; rules: SSheet;
}

// ---- Template data: extracted from the two research templates (see file header). Do not edit by hand. ----
const STYLES: Style[] = [
  {"numFmt":"dd-mm-yyyy"},
  {"numFmt":"0.0%"},
  {"numFmt":"0"},
  {"numFmt":"0.00%"},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"BDD7EE"}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"alignment":{"horizontal":"right"}},
  {"numFmt":"0.0%","font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0","font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D0CECE"}},"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center","wrapText":true}},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D0CECE"}},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center","wrapText":true}},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D0CECE"}},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"left","vertical":"center","wrapText":true}},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D0CECE"}},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"left","vertical":"center","wrapText":true}},
  {"font":{"bold":true,"sz":12.0}},
  {"numFmt":"0.00%","font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center"}},
  {"numFmt":"dd-mm-yyyy","border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center"}},
  {"numFmt":"0","border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0","border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"alignment":{"horizontal":"right"}},
  {"numFmt":"0","alignment":{"horizontal":"center","vertical":"center"}},
  {"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.00%","alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","alignment":{"horizontal":"center","vertical":"center"}},
  {"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"medium","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center"}},
  {"numFmt":"dd-mm-yyyy","border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"medium","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center"}},
  {"numFmt":"0","border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"medium","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"medium","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0","border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"medium","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"font":{"sz":12.0}},
  {"font":{"bold":true,"italic":true,"sz":14.0,"color":{"rgb":"FF0000"}},"alignment":{"horizontal":"right"}},
  {"font":{"bold":true,"sz":11.5},"alignment":{"horizontal":"right"}},
  {"alignment":{"horizontal":"center"}},
  {"numFmt":"0.00","font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.00"},
  {"numFmt":"0.00","font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"alignment":{"horizontal":"right"}},
  {"font":{"bold":true,"sz":14.0},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.00","font":{"bold":true}},
  {"border":{"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"right"}},
  {"numFmt":"0.00%","border":{"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.00","border":{"bottom":{"style":"thin","color":{"rgb":"000000"}}}},
  {"numFmt":"0.00","border":{"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.00","border":{"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"right"}},
  {"numFmt":"0.000000000000000%"},
  {"numFmt":"0","font":{"bold":true},"alignment":{"horizontal":"center","vertical":"center"}},
  {"font":{"bold":true}},
  {"font":{"bold":true},"alignment":{"horizontal":"right"}},
  {"numFmt":"0.00%","alignment":{"horizontal":"center"}},
  {"numFmt":"0.0%","alignment":{"horizontal":"center"}},
  {"font":{"bold":true},"alignment":{"horizontal":"center"}},
  {"font":{"bold":true,"sz":11.5},"fill":{"patternType":"solid","fgColor":{"rgb":"BDD7EE"}},"alignment":{"horizontal":"center"}},
  {"font":{"bold":true,"sz":11.5},"fill":{"patternType":"solid","fgColor":{"rgb":"BDD7EE"}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"font":{"bold":true,"color":{"rgb":"FFFFFF"}},"fill":{"patternType":"solid","fgColor":{"rgb":"5B9BD5"}},"border":{"left":{"style":"thin","color":{"rgb":"9DC3E6"}},"top":{"style":"thin","color":{"rgb":"9DC3E6"}}}},
  {"font":{"bold":true,"color":{"rgb":"FFFFFF"}},"fill":{"patternType":"solid","fgColor":{"rgb":"5B9BD5"}},"border":{"top":{"style":"thin","color":{"rgb":"9DC3E6"}}},"alignment":{"horizontal":"center"}},
  {"numFmt":"0","fill":{"patternType":"solid","fgColor":{"rgb":"DEEBF7"}},"border":{"left":{"style":"thin","color":{"rgb":"9DC3E6"}},"top":{"style":"thin","color":{"rgb":"9DC3E6"}}},"alignment":{"horizontal":"center"}},
  {"numFmt":"0.0%","fill":{"patternType":"solid","fgColor":{"rgb":"DEEBF7"}},"border":{"top":{"style":"thin","color":{"rgb":"9DC3E6"}}},"alignment":{"horizontal":"center"}},
  {"numFmt":"0","border":{"left":{"style":"thin","color":{"rgb":"9DC3E6"}},"top":{"style":"thin","color":{"rgb":"9DC3E6"}}},"alignment":{"horizontal":"center"}},
  {"numFmt":"0.0%","border":{"top":{"style":"thin","color":{"rgb":"9DC3E6"}}},"alignment":{"horizontal":"center"}},
  {"fill":{"patternType":"solid","fgColor":{"rgb":"DEEBF7"}},"border":{"left":{"style":"thin","color":{"rgb":"9DC3E6"}},"top":{"style":"thin","color":{"rgb":"9DC3E6"}}},"alignment":{"horizontal":"center"}},
  {"numFmt":"0","alignment":{"horizontal":"center"}},
  {"font":{"bold":true},"alignment":{"horizontal":"center","vertical":"center"}},
  {"font":{"bold":true},"alignment":{"horizontal":"right","vertical":"center"}},
  {"numFmt":"0.00%","font":{"bold":true},"alignment":{"horizontal":"center","vertical":"center"}},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"BDD7EE"}},"border":{"left":{"style":"thin","color":{"rgb":"A6A6A6"}},"right":{"style":"thin","color":{"rgb":"A6A6A6"}},"top":{"style":"thin","color":{"rgb":"A6A6A6"}},"bottom":{"style":"thin","color":{"rgb":"A6A6A6"}}},"alignment":{"horizontal":"center","vertical":"center","wrapText":true}},
  {"numFmt":"0","font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"BDD7EE"}},"border":{"left":{"style":"thin","color":{"rgb":"A6A6A6"}},"right":{"style":"thin","color":{"rgb":"A6A6A6"}},"top":{"style":"thin","color":{"rgb":"A6A6A6"}},"bottom":{"style":"thin","color":{"rgb":"A6A6A6"}}},"alignment":{"horizontal":"center","vertical":"center","wrapText":true}},
  {"numFmt":"0","border":{"left":{"style":"thin","color":{"rgb":"A6A6A6"}},"right":{"style":"thin","color":{"rgb":"A6A6A6"}},"top":{"style":"thin","color":{"rgb":"A6A6A6"}},"bottom":{"style":"thin","color":{"rgb":"A6A6A6"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"border":{"left":{"style":"thin","color":{"rgb":"A6A6A6"}},"right":{"style":"thin","color":{"rgb":"A6A6A6"}},"top":{"style":"thin","color":{"rgb":"A6A6A6"}},"bottom":{"style":"thin","color":{"rgb":"A6A6A6"}}},"alignment":{"horizontal":"left","vertical":"center","wrapText":true}},
  {"numFmt":"0","fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"border":{"left":{"style":"thin","color":{"rgb":"A6A6A6"}},"right":{"style":"thin","color":{"rgb":"A6A6A6"}},"top":{"style":"thin","color":{"rgb":"A6A6A6"}},"bottom":{"style":"thin","color":{"rgb":"A6A6A6"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","font":{"bold":true},"border":{"left":{"style":"thin","color":{"rgb":"A6A6A6"}},"right":{"style":"thin","color":{"rgb":"A6A6A6"}},"top":{"style":"thin","color":{"rgb":"A6A6A6"}},"bottom":{"style":"thin","color":{"rgb":"A6A6A6"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0","font":{"bold":true},"border":{"left":{"style":"thin","color":{"rgb":"A6A6A6"}},"right":{"style":"thin","color":{"rgb":"A6A6A6"}},"top":{"style":"thin","color":{"rgb":"A6A6A6"}},"bottom":{"style":"thin","color":{"rgb":"A6A6A6"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.00","font":{"bold":true},"border":{"left":{"style":"thin","color":{"rgb":"A6A6A6"}},"right":{"style":"thin","color":{"rgb":"A6A6A6"}},"top":{"style":"thin","color":{"rgb":"A6A6A6"}},"bottom":{"style":"thin","color":{"rgb":"A6A6A6"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"border":{"left":{"style":"thin","color":{"rgb":"A6A6A6"}},"right":{"style":"thin","color":{"rgb":"A6A6A6"}},"top":{"style":"thin","color":{"rgb":"A6A6A6"}},"bottom":{"style":"thin","color":{"rgb":"A6A6A6"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0","border":{"left":{"style":"thin","color":{"rgb":"A6A6A6"}},"right":{"style":"thin","color":{"rgb":"A6A6A6"}},"top":{"style":"thin","color":{"rgb":"A6A6A6"}},"bottom":{"style":"thin","color":{"rgb":"A6A6A6"}}},"alignment":{"horizontal":"left","vertical":"center","wrapText":true}},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"BDD7EE"}}},
  {"numFmt":"0.0%","font":{"bold":true},"alignment":{"horizontal":"left","vertical":"center"}},
  {"numFmt":"0.0%","font":{"bold":true},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0","font":{"bold":true},"alignment":{"horizontal":"left","vertical":"center"}},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"alignment":{"horizontal":"left"}},
  {"alignment":{"horizontal":"left"}},
  {"numFmt":"0.00","alignment":{"horizontal":"center","vertical":"center"}},
  {"font":{"bold":true,"sz":12.0,"color":{"rgb":"2F5597"}}},
  {"numFmt":"dd-mm-yyyy"},
  {"numFmt":1},
  {"numFmt":10},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"alignment":{"vertical":"center"}},
  {"numFmt":1,"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"alignment":{"vertical":"center"}},
  {"numFmt":10,"alignment":{"vertical":"center"}},
  {"font":{"bold":true,"sz":12.0},"fill":{"patternType":"solid","fgColor":{"rgb":"AFABAB"}},"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"left","vertical":"center","wrapText":true}},
  {"font":{"bold":true,"sz":12.0},"fill":{"patternType":"solid","fgColor":{"rgb":"D0CECE"}},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"left","vertical":"center","wrapText":true}},
  {"font":{"bold":true,"sz":12.0},"fill":{"patternType":"solid","fgColor":{"rgb":"D0CECE"}},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"left","vertical":"center","wrapText":true}},
  {"font":{"bold":true,"sz":12.0},"alignment":{"vertical":"center"}},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}}},
  {"font":{"sz":12.0},"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}}},
  {"numFmt":"dd-mm-yyyy","border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}}},
  {"numFmt":1,"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":1,"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":1,"alignment":{"horizontal":"center","vertical":"center"}},
  {"font":{"sz":12.0},"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"medium","color":{"rgb":"000000"}}}},
  {"numFmt":"dd-mm-yyyy","border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"medium","color":{"rgb":"000000"}}}},
  {"numFmt":1,"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"medium","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":1,"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"medium","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"font":{"bold":true,"italic":true,"sz":14.0,"color":{"rgb":"FF0000"}}},
  {"numFmt":1,"font":{"bold":true}},
  {"numFmt":10,"font":{"bold":true}},
  {"numFmt":1,"fill":{"patternType":"solid","fgColor":{"rgb":"DEEBF7"}},"border":{"left":{"style":"thin","color":{"rgb":"9DC3E6"}},"top":{"style":"thin","color":{"rgb":"9DC3E6"}}},"alignment":{"horizontal":"center"}},
  {"numFmt":1,"border":{"left":{"style":"thin","color":{"rgb":"9DC3E6"}},"top":{"style":"thin","color":{"rgb":"9DC3E6"}}},"alignment":{"horizontal":"center"}},
  {"numFmt":1,"alignment":{"horizontal":"center"}},
  {"numFmt":10,"font":{"bold":true},"alignment":{"horizontal":"center","vertical":"center"}},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"DEEBF7"}},"border":{"left":{"style":"thin","color":{"rgb":"A6A6A6"}},"right":{"style":"thin","color":{"rgb":"A6A6A6"}},"top":{"style":"thin","color":{"rgb":"A6A6A6"}},"bottom":{"style":"thin","color":{"rgb":"A6A6A6"}}},"alignment":{"horizontal":"center","vertical":"center","wrapText":true}},
  {"numFmt":1,"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"DEEBF7"}},"border":{"left":{"style":"thin","color":{"rgb":"A6A6A6"}},"right":{"style":"thin","color":{"rgb":"A6A6A6"}},"top":{"style":"thin","color":{"rgb":"A6A6A6"}},"bottom":{"style":"thin","color":{"rgb":"A6A6A6"}}},"alignment":{"horizontal":"center","vertical":"center","wrapText":true}},
  {"numFmt":1,"border":{"left":{"style":"thin","color":{"rgb":"A6A6A6"}},"right":{"style":"thin","color":{"rgb":"A6A6A6"}},"top":{"style":"thin","color":{"rgb":"A6A6A6"}},"bottom":{"style":"thin","color":{"rgb":"A6A6A6"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":2,"font":{"bold":true},"border":{"left":{"style":"thin","color":{"rgb":"A6A6A6"}},"right":{"style":"thin","color":{"rgb":"A6A6A6"}},"top":{"style":"thin","color":{"rgb":"A6A6A6"}},"bottom":{"style":"thin","color":{"rgb":"A6A6A6"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":1,"font":{"bold":true},"border":{"left":{"style":"thin","color":{"rgb":"A6A6A6"}},"right":{"style":"thin","color":{"rgb":"A6A6A6"}},"top":{"style":"thin","color":{"rgb":"A6A6A6"}},"bottom":{"style":"thin","color":{"rgb":"A6A6A6"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":9,"font":{"bold":true},"border":{"left":{"style":"thin","color":{"rgb":"A6A6A6"}},"right":{"style":"thin","color":{"rgb":"A6A6A6"}},"top":{"style":"thin","color":{"rgb":"A6A6A6"}},"bottom":{"style":"thin","color":{"rgb":"A6A6A6"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":10,"font":{"bold":true},"border":{"left":{"style":"thin","color":{"rgb":"A6A6A6"}},"right":{"style":"thin","color":{"rgb":"A6A6A6"}},"top":{"style":"thin","color":{"rgb":"A6A6A6"}},"bottom":{"style":"thin","color":{"rgb":"A6A6A6"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":1,"fill":{"patternType":"solid","fgColor":{"rgb":"E7E6E6"}},"border":{"left":{"style":"thin","color":{"rgb":"A6A6A6"}},"right":{"style":"thin","color":{"rgb":"A6A6A6"}},"top":{"style":"thin","color":{"rgb":"A6A6A6"}},"bottom":{"style":"thin","color":{"rgb":"A6A6A6"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"wrapText":true}},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"BDD7EE"}},"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","wrapText":true}},
  {"numFmt":"0.0%","font":{"bold":true},"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"left","vertical":"center"}},
  {"numFmt":"0.0%","font":{"bold":true},"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":1,"font":{"bold":true},"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"left","vertical":"center"}},
  {"numFmt":1,"font":{"bold":true},"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"left","vertical":"center"}},
  {"numFmt":"0.0%","font":{"bold":true},"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","font":{"bold":true},"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"medium","color":{"rgb":"000000"}}},"alignment":{"horizontal":"left","vertical":"center"}},
  {"numFmt":"0.0%","font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"92D050"}},"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"medium","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"left"}},
  {"numFmt":"0.0%","font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"left"}},
  {"numFmt":1,"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"left"}},
  {"numFmt":1,"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center"}},
  {"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"medium","color":{"rgb":"000000"}}},"alignment":{"horizontal":"left"}},
  {"numFmt":"0.0%","border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"medium","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center"}},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"BDD7EE"}},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","wrapText":true}},
  {"numFmt":"0.0%","font":{"bold":true},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","font":{"bold":true},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","font":{"bold":true},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"medium","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":1,"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0","font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0","border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center"}},
  {"numFmt":"0.0%","border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"medium","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center"}},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"BDD7EE"}},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","wrapText":true}},
  {"numFmt":"0.0%","font":{"bold":true},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","font":{"bold":true},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","font":{"bold":true},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"medium","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":1,"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0","font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0","border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0%","border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center"}},
  {"numFmt":"0.0%","border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"medium","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"medium","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center"}},
  {"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0","font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"D9D9D9"}},"border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"numFmt":"0.0","border":{"left":{"style":"medium","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"thin","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center"}},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"92D050"}},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","wrapText":true}},
  {"font":{"bold":true},"fill":{"patternType":"solid","fgColor":{"rgb":"BDD7EE"}},"border":{"left":{"style":"thin","color":{"rgb":"000000"}},"right":{"style":"thin","color":{"rgb":"000000"}},"top":{"style":"medium","color":{"rgb":"000000"}},"bottom":{"style":"thin","color":{"rgb":"000000"}}},"alignment":{"horizontal":"center","vertical":"center","wrapText":true}},
];
const FORMATS: Record<TsFormat, Fmt> = {
  "2008": {
    trade: {
      cols: [18.14,11.28,13.14,12.72,13.14,9.57,0,13.86,13.0,14.29,9.71,12.29,12.29,12.72,20.72,11.28,12.14,9.29,10.57,13.29,12.43,14.14,11.43,13.43,10.57,8.43,7.15,8.43,6.0,9.29,8.86,6.57,7.28,6.57,6.57,6.57,6.0,11.28,11.28,7.0],
      rows: {"3":33.0},
      dataRow: 23,
      first: {"C":[0],"E":[0],"G":[48,"IF((B{r}=\"Long\")+(B{r}=\"Open Long\"),(F{r}/D{r}-1),-(F{r}/D{r}-1))",1],"H":[60,"E{r}-C{r}"],"I":[49,"C$1"],"J":[49,"IF((B{r}=\"Long\")+(B{r}=\"Open Long\"),H{r}/30*D$1,H{r}/30*E$1)"],"K":[49,"G{r}-SUM(I{r},J{r})"],"L":[48,"IF((B{r}=\"Long\")+(B{r}=\"Open Long\"),MIN(IF((A{r}='All Index Explore'!$A$2:$A$61022)*(C{r}<'All Index Explore'!$B$2:$B$61022)*(E{r}>='All Index Explore'!$B$2:$B$61022),'All Index Explore'!$E$2:$E$61022,\"\"))/D{r}-1,-(MAX(IF((A{r}='All Index Explore'!$A$2:$A$61022)*(C{r}<'All Index Explore'!$B$2:$B$61022)*(E{r}>='All Index Explore'!$B$2:$B$61022),'All Index Explore'!$D$2:$D$61022,\"\"))/D{r}-1))",1],"M":[48,"IF((B{r}=\"Long\")+(B{r}=\"Open Long\"),MAX(IF((A{r}='All Index Explore'!$A$2:$A$61022)*(C{r}<'All Index Explore'!$B$2:$B$61022)*(E{r}>='All Index Explore'!$B$2:$B$61022),'All Index Explore'!$D$2:$D$61022,\"\"))/D{r}-1,-(MIN(IF((A{r}='All Index Explore'!$A$2:$A$61022)*(C{r}<'All Index Explore'!$B$2:$B$61022)*(E{r}>='All Index Explore'!$B$2:$B$61022),'All Index Explore'!$E$2:$E$61022,\"\"))/D{r}-1))",1],"N":[60,"N{p}*(1+G{r})"],"O":[60,"MAX(N{r},O{p})"],"P":[60,"IF(O{r}>N{r},N{r}-O{r},\"\")"],"Q":[49,"IFERROR(P{r}/O{r},0)"],"R":[33],"S":[60,"S{p}*(1+K{r})"],"T":[60,"MAX(S{r},T{p})"],"U":[60,"IF(T{r}>S{r},S{r}-T{r},\"\")"],"V":[49,"IFERROR(U{r}/T{r},0)"],"W":[60,"YEAR(E{r})"],"X":[60,"YEAR(C{r})"]},
      next: {"C":[0],"E":[0],"G":[48,"IF((B{r}=\"Long\")+(B{r}=\"Open Long\"),(F{r}/D{r}-1),-(F{r}/D{r}-1))"],"H":[60,"E{r}-C{r}"],"I":[49,"C$1"],"J":[49,"IF((B{r}=\"Long\")+(B{r}=\"Open Long\"),H{r}/30*D$1,H{r}/30*E$1)"],"K":[49,"G{r}-SUM(I{r},J{r})"],"L":[48,"IF((B{r}=\"Long\")+(B{r}=\"Open Long\"),MIN(IF((A{r}='All Index Explore'!$A$2:$A$61022)*(C{r}<'All Index Explore'!$B$2:$B$61022)*(E{r}>='All Index Explore'!$B$2:$B$61022),'All Index Explore'!$E$2:$E$61022,\"\"))/D{r}-1,-(MAX(IF((A{r}='All Index Explore'!$A$2:$A$61022)*(C{r}<'All Index Explore'!$B$2:$B$61022)*(E{r}>='All Index Explore'!$B$2:$B$61022),'All Index Explore'!$D$2:$D$61022,\"\"))/D{r}-1))",1],"M":[48,"IF((B{r}=\"Long\")+(B{r}=\"Open Long\"),MAX(IF((A{r}='All Index Explore'!$A$2:$A$61022)*(C{r}<'All Index Explore'!$B$2:$B$61022)*(E{r}>='All Index Explore'!$B$2:$B$61022),'All Index Explore'!$D$2:$D$61022,\"\"))/D{r}-1,-(MIN(IF((A{r}='All Index Explore'!$A$2:$A$61022)*(C{r}<'All Index Explore'!$B$2:$B$61022)*(E{r}>='All Index Explore'!$B$2:$B$61022),'All Index Explore'!$E$2:$E$61022,\"\"))/D{r}-1))",1],"N":[60,"N{p}*(1+G{r})"],"O":[60,"MAX(N{r},O{p})"],"P":[60,"IF(O{r}>N{r},N{r}-O{r},\"\")"],"Q":[49,"IFERROR(P{r}/O{r},0)"],"R":[33],"S":[60,"S{p}*(1+K{r})"],"T":[60,"MAX(S{r},T{p})"],"U":[60,"IF(T{r}>S{r},S{r}-T{r},\"\")"],"V":[49,"IFERROR(U{r}/T{r},0)"],"W":[60,"YEAR(E{r})"],"X":[60,"YEAR(C{r})"]},
      cells: [
        ["A1",0,"Overall"],["C1",1,0.003],["D1",1,0.01],["E1",1,-0.005],["H1",0,40544],["I1",2,null,"YEAR(H1)"],["J1",0,null,"INDEX(C23:C1048576,MATCH(I1,X23:X1048576,0),1)",1],["K1",3],["L1",0,null,"MAX(E23:E1048576)"],
        ["B2",4,"Gross %"],["C2",4,"Impact Cost"],["D2",4,"Rollover Cost"],["E2",4,"Net %"],["F2",4,"Days"],["I2",4,"Gross %"],["J2",4,"Impact Cost"],["K2",4,"Rollover Cost"],["L2",4,"Net %"],["M2",4,"Days"],["N2",3],
        ["A3",5,"Total Profit %"],["B3",6,null,"SUM(G23:G5000)"],["C3",6,null,"SUM(I23:I5000)"],["D3",6,null,"SUM(J23:J5000)"],["E3",6,null,"SUM(K23:K5000)"],["F3",7,null,"SUM(H23:H5000)"],["H3",5,"Total Profit %"],["I3",6,null,"SUM(IF(H$1<=C23:C5000,G23:G5000,\"\"))",1],["J3",6,null,"SUM(IF(H$1<=C23:C5000,I23:I5000,\"\"))",1],["K3",6,null,"SUM(IF(H$1<=C23:C5000,J23:J5000,\"\"))",1],["L3",6,null,"SUM(IF(H$1<=C23:C5000,K23:K5000,\"\"))",1],["M3",7,null,"SUM(IF(H$1<=C23:C5000,H23:H5000,\"\"))",1],["N3",3],["O3",8,"Dates"],["P3",9,"Start Date"],["Q3",9,"End Date"],["R3",9,"Trades"],["S3",10,"Total Gross Profit"],["T3",10,"Average Gross Profit"],["U3",10,"Gross Month. Thk."],["V3",10,"Total Net Profit"],["W3",10,"Average Net Profit"],["X3",10,"Net Month. Thk."],["Y3",10,"Average Days Held"],["Z3",11,"Months Held"],["AA3",12],
        ["A4",5,"Avg %"],["B4",13,null,"AVERAGE(G23:G5000)"],["C4",14],["D4",14],["E4",6,null,"AVERAGE(K23:K5000)"],["F4",7,null,"AVERAGE(H23:H5000)"],["H4",5,"Avg %"],["I4",6,null,"AVERAGE(IF(H$1<=C23:C5000,G23:G5000,\"\"))",1],["J4",14],["K4",14],["L4",6,null,"AVERAGE(IF(H$1<=C23:C5000,K23:K5000,\"\"))",1],["M4",7,null,"AVERAGE(IF(H$1<=C23:C5000,H23:H5000,\"\"))",1],["N4",3],["O4",15," 2008 - 2026"],["P4",16,39447],["Q4",16,46387],["R4",17,null,"COUNT(IF((P4<=$C$23:$C$5000)*(Q4>=$E$23:$E$5000),$G$23:$G$5000,\"\"))",1],["S4",18,null,"SUM(IF((P4<=$C$23:$C$5000)*(Q4>=$E$23:$E$5000),$G$23:$G$5000,\"\"))",1],["T4",18,null,"AVERAGE(IF((P4<=$C$23:$C$5000)*(Q4>=$E$23:$E$5000),$G$23:$G$5000,\"\"))",1],["U4",18,null,"T4/Z4"],["V4",18,null,"SUM(IF((P4<=$C$23:$C$5000)*(Q4>=$E$23:$E$5000),$K$23:$K$5000,\"\"))",1],["W4",18,null,"AVERAGE(IF((P4<=$C$23:$C$5000)*(Q4>=$E$23:$E$5000),$K$23:$K$5000,\"\"))",1],["X4",18,null,"W4/Z4"],["Y4",17,null,"AVERAGE(IF((P4<=$C$23:$C$5000)*(Q4>=$E$23:$E$5000),$H$23:$H$5000,\"\"))",1],["Z4",19,null,"Y4/30.5"],
        ["A5",20,"Total Trades"],["B5",21,null,"COUNT(G23:G5000)"],["C5",22],["D5",22],["E5",21,null,"COUNT(K23:K5000)"],["F5",21,null,"F4/30.5"],["H5",20,"Total Trades"],["I5",21,null,"COUNT(IF(H$1<=C23:C5000,G23:G5000,\"\"))",1],["J5",22],["K5",22],["L5",21,null,"COUNT(IF(H$1<=C23:C5000,K23:K5000,\"\"))",1],["M5",21,null,"M4/30.5"],["N5",3],["O5",15," 2008 - 2010"],["P5",16,39447],["Q5",16,40543],["R5",17,null,"COUNT(IF((P5<=$C$23:$C$5000)*(Q5>=$E$23:$E$5000),$G$23:$G$5000,\"\"))",1],["S5",18,null,"SUM(IF((P5<=$C$23:$C$5000)*(Q5>=$E$23:$E$5000),$G$23:$G$5000,\"\"))",1],["T5",18,null,"AVERAGE(IF((P5<=$C$23:$C$5000)*(Q5>=$E$23:$E$5000),$G$23:$G$5000,\"\"))",1],["U5",18,null,"T5/Z5"],["V5",18,null,"SUM(IF((P5<=$C$23:$C$5000)*(Q5>=$E$23:$E$5000),$K$23:$K$5000,\"\"))",1],["W5",18,null,"AVERAGE(IF((P5<=$C$23:$C$5000)*(Q5>=$E$23:$E$5000),$K$23:$K$5000,\"\"))",1],["X5",18,null,"W5/Z5"],["Y5",17,null,"AVERAGE(IF((P5<=$C$23:$C$5000)*(Q5>=$E$23:$E$5000),$H$23:$H$5000,\"\"))",1],["Z5",19,null,"Y5/30.5"],
        ["A6",5,"Win Ratio"],["B6",13,null,"COUNTIF(G23:G5000,\">0\")/B5"],["C6",14],["D6",14],["E6",6,null,"COUNTIF(K23:K5000,\">0\")/E5"],["F6",14],["H6",5,"Win Ratio"],["I6",6,null,"COUNT(IF((H$1<=C23:C5000)*(0<G23:G5000),G23:G5000,\"\"))/I5",1],["J6",14],["K6",14],["L6",6,null,"COUNT(IF((H$1<=C23:C5000)*(0<K23:K5000),K23:K5000,\"\"))/L5",1],["M6",14],["N6",3],["O6",15," 2011 - 2019"],["P6",16,40543],["Q6",16,43830],["R6",17,null,"COUNT(IF((P6<=$C$23:$C$5000)*(Q6>=$E$23:$E$5000),$G$23:$G$5000,\"\"))",1],["S6",18,null,"SUM(IF((P6<=$C$23:$C$5000)*(Q6>=$E$23:$E$5000),$G$23:$G$5000,\"\"))",1],["T6",18,null,"AVERAGE(IF((P6<=$C$23:$C$5000)*(Q6>=$E$23:$E$5000),$G$23:$G$5000,\"\"))",1],["U6",18,null,"T6/Z6"],["V6",18,null,"SUM(IF((P6<=$C$23:$C$5000)*(Q6>=$E$23:$E$5000),$K$23:$K$5000,\"\"))",1],["W6",18,null,"AVERAGE(IF((P6<=$C$23:$C$5000)*(Q6>=$E$23:$E$5000),$K$23:$K$5000,\"\"))",1],["X6",18,null,"W6/Z6"],["Y6",17,null,"AVERAGE(IF((P6<=$C$23:$C$5000)*(Q6>=$E$23:$E$5000),$H$23:$H$5000,\"\"))",1],["Z6",19,null,"Y6/30.5"],
        ["A7",20,"Win Avg"],["B7",23,null,"AVERAGEIF(G23:G5000,\">0\",G23:G5000)"],["C7",22],["D7",22],["E7",24,null,"AVERAGEIF(K23:K5000,\">0\",K23:K5000)"],["F7",22],["H7",20,"Win Avg"],["I7",24,null,"AVERAGE(IF((H$1<=C23:C5000)*(0<G23:G5000),G23:G5000,\"\"))",1],["J7",22],["K7",22],["L7",24,null,"AVERAGE(IF((H$1<=C23:C5000)*(0<K23:K5000),K23:K5000,\"\"))",1],["M7",22],["N7",3],["O7",25," 2011 - 2025"],["P7",26,40543],["Q7",26,46387],["R7",27,null,"COUNT(IF((P7<=$C$23:$C$5000)*(Q7>=$E$23:$E$5000),$G$23:$G$5000,\"\"))",1],["S7",28,null,"SUM(IF((P7<=$C$23:$C$5000)*(Q7>=$E$23:$E$5000),$G$23:$G$5000,\"\"))",1],["T7",28,null,"AVERAGE(IF((P7<=$C$23:$C$5000)*(Q7>=$E$23:$E$5000),$G$23:$G$5000,\"\"))",1],["U7",28,null,"T7/Z7"],["V7",28,null,"SUM(IF((P7<=$C$23:$C$5000)*(Q7>=$E$23:$E$5000),$K$23:$K$5000,\"\"))",1],["W7",28,null,"AVERAGE(IF((P7<=$C$23:$C$5000)*(Q7>=$E$23:$E$5000),$K$23:$K$5000,\"\"))",1],["X7",28,null,"W7/Z7"],["Y7",27,null,"AVERAGE(IF((P7<=$C$23:$C$5000)*(Q7>=$E$23:$E$5000),$H$23:$H$5000,\"\"))",1],["Z7",29,null,"Y7/30.5"],
        ["A8",20,"Loss Avg"],["B8",23,null,"AVERAGEIF(G23:G5000,\"<0\",G23:G5000)"],["C8",22],["D8",22],["E8",24,null,"AVERAGEIF(K23:K5000,\"<0\",K23:K5000)"],["F8",22],["H8",20,"Loss Avg"],["I8",24,null,"AVERAGE(IF((H$1<=C23:C5000)*(0>G23:G5000),G23:G5000,\"\"))",1],["J8",22],["K8",22],["L8",24,null,"AVERAGE(IF((H$1<=C23:C5000)*(0>K23:K5000),K23:K5000,\"\"))",1],["M8",22],["N8",3],["O8",30],
        ["A9",20,"Max Winner"],["B9",23,null,"MAX(G23:G5000)"],["C9",22],["D9",22],["E9",24,null,"MAX(K23:K503)"],["F9",22],["H9",20,"Max Winner"],["I9",24,null,"MAX(IF(H$1<=C23:C5000,G23:G5000,\"\"))",1],["J9",22],["K9",22],["L9",24,null,"MAX(IF(H$1<=C23:C5000,K23:K5000,\"\"))",1],["M9",22],["N9",3],["O9",31,"Rules:"],
        ["A10",20,"Max Looser"],["B10",23,null,"MIN(G23:G5000)"],["C10",22],["D10",22],["E10",24,null,"MIN(K23:K503)"],["F10",22],["H10",20,"Max Looser"],["I10",24,null,"MIN(IF(H$1<=C23:C5000,G23:G5000,\"\"))",1],["J10",22],["K10",22],["L10",24,null,"MIN(IF(H$1<=C23:C5000,K23:K5000,\"\"))",1],["M10",22],["N10",3],["O10",32,"Entry:"],["P10",33],
        ["A11",5,"Expectancy Ratio"],["B11",34,null,"((B6*B7)-((1-B6)*ABS(B8)))/(ABS(B8))"],["C11",34],["D11",34],["E11",34,null,"((E6*E7)-((1-E6)*ABS(E8)))/(ABS(E8))"],["F11",34],["G11",35],["H11",36,"Expectancy Ratio"],["I11",34,null,"((I6*I7)-((1-I6)*ABS(I8)))/(ABS(I8))"],["J11",34],["K11",34],["L11",34,null,"((L6*L7)-((1-L6)*ABS(L8)))/(ABS(L8))"],["M11",34],["N11",3],["O11",32,"Exit:"],["P11",33],
        ["A12",20,"Monthly Thickness"],["B12",23,null,"B4/F4*30"],["C12",22],["D12",22],["E12",24,null,"E4/F4*30"],["F12",22],["H12",20,"Monthly Thickness"],["I12",24,null,"I4/M4*30"],["J12",22],["K12",22],["L12",24,null,"L4/M4*30"],["M12",22],["N12",3],["O12",32,"Filter:"],
        ["A13",20,"Risk Reward"],["B13",21,null,"ABS(B7/B8)"],["C13",22],["D13",22],["E13",21,null,"ABS(E7/E8)"],["F13",22],["H13",20,"Risk Reward"],["I13",21,null,"ABS(I7/I8)"],["J13",22],["K13",22],["L13",21,null,"ABS(L7/L8)"],["M13",22],["N13",3],["O13",12],
        ["A14",20,"CAGR"],["B14",23,null,"(INDEX($N$22:$N$5001,MATCH(MAX($E$22:$E$5000),$E$22:$E$5000,0),1)/100)^(1/(((MAX(E22:E5000)-MIN(C22:C5000))/365)))-1",1],["C14",22],["D14",22],["E14",24,null,"(INDEX($S$22:$S$5001,MATCH(MAX($E$22:$E$5000),$E$22:$E$5001,0),1)/100)^(1/(((MAX(E22:E5001)-MIN(C22:C5000))/365)))-1",1],["F14",22],["H14",20,"CAGR"],["I14",24,null,"(INDEX(N22:N5000,MATCH(L1,E22:E5000,0),1)/INDEX(N22:N5000,MATCH(J1,C22:C5000,0)-1,1))^(1/(((MAX(E22:E5000)-J1)/365)))-1",1],["J14",22],["K14",22],["L14",24,null,"(INDEX(S22:S5000,MATCH(L1,E22:E5000,0),1)/INDEX(S22:S5000,MATCH(J1,C22:C5000,0)-1,1))^(1/(((MAX(E22:E5000)-J1)/365)))-1",1],["M14",22],["N14",3],["O14",37],["P14",38],
        ["A15",20,"DD"],["B15",23,null,"MIN($Q$21:$Q$5000)"],["E15",24,null,"MIN($V$21:$V$5000)"],["H15",20,"DD"],["I15",24,null,"MIN(IF(H$1<=C23:C5000,Q23:Q5000,\"\"))",1],["L15",24,null,"MIN(IF(J$1<=C23:C5000,V23:V5000,\"\"))",1],["N15",3],
        ["A16",39,"CAR/MDD"],["B16",40,null,"ABS(B14/B15)"],["C16",41],["D16",41],["E16",42,null,"ABS(E14/E15)"],["F16",41],["G16",41],["H16",43,"CAR/MDD"],["I16",42,null,"ABS(I14/I15)"],["J16",41],["K16",41],["L16",42,null,"ABS(L14/L15)"],["M16",41],["N16",3],["O16",44],
        ["B17",45,"MAE"],["C17",46],["D17",46],["E17",45,"MFE"],["I17",45,"MAE"],["J17",46],["K17",46],["L17",45,"MFE"],["N17",3],
        ["A18",47,"Sum"],["B18",24,null,"SUM(L23:L5000)"],["C18",33],["D18",33],["E18",24,null,"SUM(M23:M5000)"],["H18",47,"Sum"],["I18",24,null,"SUM(IF(H$1<=C23:C5000,L23:L5000,\"\"))",1],["J18",33],["K18",33],["L18",24,null,"SUM(IF(H$1<=C23:C5000,M23:M5000,\"\"))",1],["N18",3],
        ["A19",47,"Max"],["B19",48,null,"MIN(L23:L500)"],["C19",33],["D19",33],["E19",48,null,"MAX(M23:M500)"],["G19",0],["H19",47,"Max"],["I19",49,null,"MIN(IF(H$1<=C23:C5000,L23:L5000,\"\"))",1],["J19",33],["K19",33],["L19",49,null,"MAX(IF(H$1<=C23:C5000,M23:M5000,\"\"))",1],["O19",3],["P19",3],
        ["A20",47,"Avg."],["B20",48,null,"AVERAGE(L23:L5000)"],["C20",33],["D20",33],["E20",48,null,"AVERAGE(M23:M5000)"],["G20",0],["H20",47,"Avg."],["I20",49,null,"AVERAGE(IF(H$1<=C23:C5000,L23:L5000,\"\"))",1],["J20",33],["K20",33],["L20",49,null,"AVERAGE(IF(H$1<=C23:C5000,M23:M5000,\"\"))",1],["M20",46],["N20",46],["O20",46],["P20",46],["Q20",46],["S20",46],
        ["C21",0],["E21",0],["G21",3],["N21",50,"Gross CUMM"],["O21",50,"Peak"],["P21",50,"DD"],["Q21",50,"Gross %DD"],["R21",50],["S21",50,"Net CUMM"],["T21",50,"Peak"],["U21",50,"DD"],["V21",50,"Net %DD"],
        ["A22",51,"Symbol"],["B22",51,"Trade"],["C22",51,"En. date"],["D22",51,"En. Price"],["E22",51,"Ex. date"],["F22",51,"Ex. Price"],["G22",51,"% chg"],["H22",51,"Days"],["I22",52,"Impact cost"],["J22",51,"Rollover Cost"],["K22",51,"Net Profit"],["L22",51,"MAE"],["M22",51,"MFE"],["N22",33,100],["O22",33,100],["S22",33,100],["T22",33,100],["W22",50,"Exit Year"],["X22",50,"Entry Year"],["AD22",53,"Year"],["AE22",54,"Net Return"],
        ["AD23",55,2008],["AE23",56,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD23,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD23,$K$23:$K$1075)),\"\")",1],
        ["AD24",57,2009],["AE24",58,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD24,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD24,$K$23:$K$1075)),\"\")",1],
        ["AD25",55,2010],["AE25",56,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD25,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD25,$K$23:$K$1075)),\"\")",1],
        ["AD26",57,2011],["AE26",58,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD26,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD26,$K$23:$K$1075)),\"\")",1],
        ["AD27",55,2012],["AE27",56,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD27,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD27,$K$23:$K$1075)),\"\")",1],
        ["AD28",57,2013],["AE28",58,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD28,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD28,$K$23:$K$1075)),\"\")",1],
        ["AD29",55,2014],["AE29",56,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD29,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD29,$K$23:$K$1075)),\"\")",1],
        ["AD30",57,2015],["AE30",58,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD30,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD30,$K$23:$K$1075)),\"\")",1],
        ["AD31",55,2016],["AE31",56,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD31,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD31,$K$23:$K$1075)),\"\")",1],
        ["AD32",57,2017],["AE32",58,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD32,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD32,$K$23:$K$1075)),\"\")",1],
        ["AD33",55,2018],["AE33",56,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD33,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD33,$K$23:$K$1075)),\"\")",1],
        ["AD34",57,2019],["AE34",58,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD34,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD34,$K$23:$K$1075)),\"\")",1],
        ["AD35",55,2020],["AE35",56,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD35,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD35,$K$23:$K$1075)),\"\")",1],
        ["AD36",57,2021],["AE36",58,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD36,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD36,$K$23:$K$1075)),\"\")",1],
        ["AD37",55,2022],["AE37",56,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD37,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD37,$K$23:$K$1075)),\"\")",1],
        ["AD38",57,2023],["AE38",58,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD38,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD38,$K$23:$K$1075)),\"\")",1],
        ["AD39",55,2024],["AE39",56,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD39,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD39,$K$23:$K$1075)),\"\")",1],
        ["AD40",57,2025],["AE40",58,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD40,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD40,$K$23:$K$1075)),\"\")",1],
        ["AD41",55,2026],["AE41",58,null,"IFERROR(IF(SUMIF($W$23:$W$1075,AD41,$K$23:$K$1075)=0,NA(),SUMIF($W$23:$W$1075,AD41,$K$23:$K$1075)),\"\")",1],
        ["AD42",59,"Total Net Return"],["AE42",1,null,"SUM($AE$23:$AE$41)"],
      ],
    },
    summary: {
      cols: [3.57,0,22.14,0,12.0,10.57,0,0,0,0,0,0,0,11.43,10.71,0,0,0,11.0,10.0,10.43],
      cells: [
        ["D1",61],["E1",61],
        ["C2",62],["D2",63],["E2",63],
        ["B3",64,"Sr. No."],["C3",64,"All Sheets Name"],["D3",64,"From"],["E3",64,"Total Net Profit %"],["F3",64,"Net Avg %"],["G3",65,"Avarage Days"],["H3",64,"Total Trades"],["I3",64,"Win Ratio"],["J3",64,"Win Avg"],["K3",64,"Loss Avg"],["L3",64,"Max Winner"],["M3",64,"Max Looser"],["N3",64,"Expectancy Ratio"],["O3",64,"Monthly Thickness"],["P3",64,"Risk Reward"],["Q3",64,"CAGR"],["R3",64,"DD"],["S3",64,"CAR/MDD"],["T3",64,"Sum MAE"],["U3",64,"Max MAE"],["V3",64,"Avg MAE"],
        ["B4",66,1],["C4",67,null,"INDIRECT(\"'\"&$C$3&\"'!B\"&ROW()+B4)"],["D4",68,2008],["E4",69,null,"IFERROR(IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!E3\")),\"\")"],["F4",69,null,"IFERROR(IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!E4\")),\"\")"],["G4",70,null,"IFERROR(IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!F4\")),\"\")"],["H4",70,null,"IFERROR(IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!E5\")),\"\")"],["I4",69,null,"IFERROR(IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!E6\")),\"\")"],["J4",69,null,"IFERROR(IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!E7\")),\"\")"],["K4",69,null,"IFERROR(IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!E8\")),\"\")"],["L4",69,null,"IFERROR(IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!E9\")),\"\")"],["M4",69,null,"IFERROR(IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!E10\")),\"\")"],["N4",71,null,"IFERROR(IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!E11\")),\"\")"],["O4",69,null,"IFERROR(IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!E12\")),\"\")"],["P4",71,null,"IFERROR(IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!E13\")),\"\")"],["Q4",69,null,"IFERROR(IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!E14\")),\"\")"],["R4",69,null,"IFERROR(IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!E15\")),\"\")"],["S4",71,null,"IFERROR(IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!E16\")),\"\")"],["T4",69,null,"IFERROR(IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!B18\")),\"\")"],["U4",69,null,"IFERROR(IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!B19\")),\"\")"],["V4",69,null,"IFERROR(IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!B20\")),\"\")"],
        ["B5",72],["C5",67,null,"C4"],["D5",66,2011],["E5",69,null,"IFERROR(IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!L3\")),\"\")"],["F5",69,null,"IFERROR(IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!L4\")),\"\")"],["G5",70,null,"IFERROR(IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!M4\")),\"\")"],["H5",70,null,"IFERROR(IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!L5\")),\"\")"],["I5",69,null,"IFERROR(IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!L6\")),\"\")"],["J5",69,null,"IFERROR(IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!L7\")),\"\")"],["K5",69,null,"IFERROR(IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!L8\")),\"\")"],["L5",69,null,"IFERROR(IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!L9\")),\"\")"],["M5",69,null,"IFERROR(IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!L10\")),\"\")"],["N5",71,null,"IFERROR(IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!L11\")),\"\")"],["O5",69,null,"IFERROR(IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!L12\")),\"\")"],["P5",71,null,"IFERROR(IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!L13\")),\"\")"],["Q5",69,null,"IFERROR(IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!L14\")),\"\")"],["R5",69,null,"IFERROR(IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!L15\")),\"\")"],["S5",71,null,"IFERROR(IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!L16\")),\"\")"],["T5",69,null,"IFERROR(IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!I18\")),\"\")"],["U5",69,null,"IFERROR(IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!I19\")),\"\")"],["V5",69,null,"IFERROR(IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!I20\")),\"\")"],
        ["B6",66,2],["C6",73,null,"INDIRECT(\"'\"&$C$3&\"'!B\"&ROW()+B6-2)"],["D6",68,2008],["E6",69,null,"IFERROR(IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!E3\")),\"\")"],["F6",69,null,"IFERROR(IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!E4\")),\"\")"],["G6",70,null,"IFERROR(IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!F4\")),\"\")"],["H6",70,null,"IFERROR(IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!E5\")),\"\")"],["I6",69,null,"IFERROR(IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!E6\")),\"\")"],["J6",69,null,"IFERROR(IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!E7\")),\"\")"],["K6",69,null,"IFERROR(IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!E8\")),\"\")"],["L6",69,null,"IFERROR(IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!E9\")),\"\")"],["M6",69,null,"IFERROR(IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!E10\")),\"\")"],["N6",71,null,"IFERROR(IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!E11\")),\"\")"],["O6",69,null,"IFERROR(IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!E12\")),\"\")"],["P6",71,null,"IFERROR(IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!E13\")),\"\")"],["Q6",69,null,"IFERROR(IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!E14\")),\"\")"],["R6",69,null,"IFERROR(IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!E15\")),\"\")"],["S6",71,null,"IFERROR(IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!E16\")),\"\")"],["T6",69,null,"IFERROR(IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!B18\")),\"\")"],["U6",69,null,"IFERROR(IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!B19\")),\"\")"],["V6",69,null,"IFERROR(IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!B20\")),\"\")"],
        ["B7",72],["C7",73,null,"C6"],["D7",66,2011],["E7",69,null,"IFERROR(IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!L3\")),\"\")"],["F7",69,null,"IFERROR(IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!L4\")),\"\")"],["G7",70,null,"IFERROR(IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!M4\")),\"\")"],["H7",70,null,"IFERROR(IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!L5\")),\"\")"],["I7",69,null,"IFERROR(IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!L6\")),\"\")"],["J7",69,null,"IFERROR(IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!L7\")),\"\")"],["K7",69,null,"IFERROR(IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!L8\")),\"\")"],["L7",69,null,"IFERROR(IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!L9\")),\"\")"],["M7",69,null,"IFERROR(IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!L10\")),\"\")"],["N7",71,null,"IFERROR(IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!L11\")),\"\")"],["O7",69,null,"IFERROR(IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!L12\")),\"\")"],["P7",71,null,"IFERROR(IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!L13\")),\"\")"],["Q7",69,null,"IFERROR(IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!L14\")),\"\")"],["R7",69,null,"IFERROR(IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!L15\")),\"\")"],["S7",71,null,"IFERROR(IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!L16\")),\"\")"],["T7",69,null,"IFERROR(IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!I18\")),\"\")"],["U7",69,null,"IFERROR(IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!I19\")),\"\")"],["V7",69,null,"IFERROR(IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!I20\")),\"\")"],
      ],
    },
    yoy: {
      cols: [0,15.72,10.86],
      rows: {"2":18.75,"3":13.5,"4":17.25,"5":2.25},
      colStyles: [
        [-1,60,-1,74,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,6,7,21,6,24,34,24,21,24,24,80,24,24,24,-1],
        [-1,60,-1,74,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,6,7,21,6,24,34,24,21,24,24,80,24,24,24,-1],
        [-1,60,-1,74,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,6,7,21,6,24,34,24,21,24,24,80,24,24,24,-1],
        [-1,60,-1,74,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,6,7,21,6,24,34,24,21,24,24,80,24,24,24,-1],
        [-1,60,-1,74,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,6,7,21,6,24,34,24,21,24,24,80,24,24,24,-1],
        [-1,60,-1,74,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,6,7,21,6,24,34,24,21,24,24,80,24,24,24,-1],
        [-1,60,-1,74,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,6,7,21,6,24,34,24,21,24,24,80,24,24,24,-1],
        [-1,60,-1,74,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,6,7,21,6,24,34,24,21,24,24,80,24,24,24,-1],
        [-1,60,-1,74,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,6,7,21,6,24,34,24,21,24,24,80,24,24,24,-1],
        [-1,60,-1,74,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,6,7,21,6,24,34,24,21,24,24,80,24,24,24,-1],
        [-1,60,-1,74,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,6,7,21,6,24,34,24,21,24,24,80,24,24,24,-1],
        [-1,60,-1,74,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,6,7,21,6,24,34,24,21,24,24,80,24,24,24,-1],
        [-1,60,-1,74,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,6,7,21,6,24,34,24,21,24,24,80,24,24,24,-1],
        [-1,60,-1,74,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,76,6,7,21,6,24,34,24,21,24,24,80,24,24,24,-1],
      ],
      cells: [
        ["B1",12,"Note: The YOY Returns are on net basis and from exit date (From 2008)"],
        ["C2",60,1],
        ["C3",-1,"All Sheets Name"],
        ["C4",74,null,"IFERROR(INDIRECT(\"'\"&$C$3&\"'!B\"&ROW()+C2),\"\")"],
        ["B5",75,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C5",76,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B6",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C6",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B7",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C7",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B8",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C8",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B9",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C9",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B10",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C10",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B11",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C11",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B12",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C12",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B13",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C13",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B14",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C14",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B15",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C15",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B16",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C16",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B17",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C17",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B18",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C18",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B19",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C19",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B20",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C20",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B21",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C21",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B22",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C22",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B23",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C23",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B24",77,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C24",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B25",75,null,"INDIRECT(\"'\"&C$4&\"'!AD\"&ROW()+17)"],["C25",76,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!AE\"&ROW()+17),\"\")"],
        ["B26",78,"Net Avg %"],["C26",6,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!e\"&ROW()-22),\"\")"],
        ["B27",78,"Avg. Days"],["C27",7,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!f\"&ROW()-23),\"\")"],
        ["B28",79,"Total Trades"],["C28",21,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!e\"&ROW()-23),\"\")"],
        ["B29",78,"Win Ratio"],["C29",6,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!e\"&ROW()-23),\"\")"],
        ["B30",79,"Win Avg"],["C30",24,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!e\"&ROW()-23),\"\")"],
        ["B31",78,"Expectancy Ratio"],["C31",34,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!e\"&ROW()-20),\"\")"],
        ["B32",79,"Monthly Thickness"],["C32",24,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!e\"&ROW()-20),\"\")"],
        ["B33",79,"Risk Reward"],["C33",21,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!e\"&ROW()-20),\"\")"],
        ["B34",79,"CAGR"],["C34",24,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!e\"&ROW()-20),\"\")"],
        ["B35",79,"DD"],["C35",24,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!e\"&ROW()-20),\"\")"],
        ["B36",79,"CAR/MDD"],["C36",80,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!e\"&ROW()-20),\"\")"],
        ["B37",79,"Sum MAE"],["C37",24,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!b\"&ROW()-19),\"\")"],
        ["B38",79,"Max MAE"],["C38",24,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!b\"&ROW()-19),\"\")"],
        ["B39",79,"Avg MAE"],["C39",23,null,"IFERROR(INDIRECT(\"'\"&C$4&\"'!b\"&ROW()-19),\"\")"],
      ],
    },
    names: {
      cols: [],
      cells: [
        ["B4",12,"All Sheets Name"],
        ["B5",-1,1],
      ],
    },
    explore: {
      cols: [16.29,10.43],
      cells: [
        ["A1",-1,"Ticker"],["B1",-1,"Date/Time"],["C1",-1,"Open"],["D1",-1,"High"],["E1",-1,"Low"],["F1",-1,"Close"],
        ["A2",-1,"NIFTY 50"],["B2",0,36528],["C2",2,1482.15],["D2",2,1592.9],["E2",2,1482.15],["F2",2,1592.2],
      ],
    },
    rules: {
      cols: [6.14,22.86,27.72],
      cells: [
        ["B2",81,"Timeframe:"],["C2",12],
        ["B3",81,"Index:"],["C3",12],
        ["B4",81,"Period:"],["C4",12],
        ["B5",81,"AFL Path:"],["C5",12],
        ["B6",81,"Done By:"],["C6",12],
        ["B7",81,"Verified By:"],["C7",12],
        ["B8",81],["C8",12],
        ["B9",81,"Strategy:"],["C9",12],
        ["B10",81,"Position:"],["C10",12],
        ["B11",81,"Filter:"],["C11",12],
        ["B12",81,"Setup:"],["C12",12],
        ["B13",81,"Entry:"],["C13",12],["D13",50],
        ["B14",81,"Exit:"],["C14",12],["D14",50],
      ],
    },
  },
  "2000": {
    trade: {
      cols: [8.7109375,8.7109375,10.5703125,9.28515625,10.5703125,9.28515625,7.140625,12.140625,11.0,12.7109375,9.7109375,12.28515625,12.28515625,12.85546875,11.0,10.7109375,12.28515625,12.28515625,12.85546875,7.85546875,5.140625,5.140625,13.0,11.28515625,10.5703125,7.85546875,7.140625,9.28515625,8.7109375,10.28515625,10.5703125,8.7109375,12.0,11.28515625,7.140625,8.42578125,6.0,6.5703125,6.0,6.5703125],
      dataRow: 21,
      first: {"C":[82],"E":[82],"G":[1,"IF(B{r}=\"Long\",(F{r}/D{r}-1),-(F{r}/D{r}-1))"],"H":[108,"E{r}-C{r}"],"I":[1,"C$1"],"J":[1,"IF(B{r}=\"Long\",H{r}/30*D$1,H{r}/30*E$1)"],"K":[1,"G{r}-SUM(I{r},J{r})"],"L":[83,"L{p}*(1+G{r})"],"M":[83,"MAX(L{r},M{p})"],"N":[83,"IF(M{r}>L{r},L{r}-M{r},\"\")"],"O":[1,"IFERROR(N{r}/M{r},0)"],"Q":[83,"Q{p}*(1+K{r})"],"R":[83,"MAX(Q{r},R{p})"],"S":[83,"IF(R{r}>Q{r},Q{r}-R{r},\"\")"],"T":[1,"IFERROR(S{r}/R{r},0)"],"U":[83,"YEAR(E{r})"],"V":[83,"YEAR(C{r})"],"W":[1,"IF(B{r}=\"Long\",MIN(IF((A{r}='All Index Explore'!$A$2:$A$52322)*(C{r}<'All Index Explore'!$B$2:$B$52322)*(E{r}>='All Index Explore'!$B$2:$B$52322),'All Index Explore'!$E$2:$E$52322,\"\"))/D{r}-1,-(MAX(IF((A{r}='All Index Explore'!$A$2:$A$52322)*(C{r}<'All Index Explore'!$B$2:$B$52322)*(E{r}>='All Index Explore'!$B$2:$B$52322),'All Index Explore'!$D$2:$D$52322,\"\"))/D{r}-1))",1]},
      next: {"C":[82],"E":[82],"G":[1,"IF(B{r}=\"Long\",(F{r}/D{r}-1),-(F{r}/D{r}-1))"],"H":[108,"E{r}-C{r}"],"I":[1,"C$1"],"J":[1,"IF(B{r}=\"Long\",H{r}/30*D$1,H{r}/30*E$1)"],"K":[1,"G{r}-SUM(I{r},J{r})"],"L":[83,"L{p}*(1+G{r})"],"M":[83,"MAX(L{r},M{p})"],"N":[83,"IF(M{r}>L{r},L{r}-M{r},\"\")"],"O":[1,"IFERROR(N{r}/M{r},0)"],"Q":[83,"Q{p}*(1+K{r})"],"R":[83,"MAX(Q{r},R{p})"],"S":[83,"IF(R{r}>Q{r},Q{r}-R{r},\"\")"],"T":[1,"IFERROR(S{r}/R{r},0)"],"U":[83,"YEAR(E{r})"],"V":[83,"YEAR(C{r})"],"W":[1,"IF(B{r}=\"Long\",MIN(IF((A{r}='All Index Explore'!$A$2:$A$52322)*(C{r}<'All Index Explore'!$B$2:$B$52322)*(E{r}>='All Index Explore'!$B$2:$B$52322),'All Index Explore'!$E$2:$E$52322,\"\"))/D{r}-1,-(MAX(IF((A{r}='All Index Explore'!$A$2:$A$52322)*(C{r}<'All Index Explore'!$B$2:$B$52322)*(E{r}>='All Index Explore'!$B$2:$B$52322),'All Index Explore'!$D$2:$D$52322,\"\"))/D{r}-1))",1]},
      cells: [
        ["A1",82,"Overall"],["C1",1,0.003],["D1",1,0.01],["E1",1,-0.005],["H1",82,39448],["I1",83,null,"YEAR(H1)"],["J1",82,null,"INDEX(C21:C1048576,MATCH(I1,V21:V1048575,0),1)"],["K1",84],["L1",82,null,"MAX(E21:E1048576)"],["O1",82,40544],["P1",83,null,"YEAR(O1)"],["Q1",82,null,"INDEX(C21:C1048576,MATCH(P1,V21:V1048575,0),1)"],["R1",84],["S1",82,null,"MAX(E21:E1048576)"],
        ["B2",4,"Gross %"],["C2",4,"Impact Cost"],["D2",4,"Rollover Cost"],["E2",4,"Net %"],["F2",4,"Days"],["I2",4,"Gross %"],["J2",4,"Impact Cost"],["K2",4,"Rollover Cost"],["L2",4,"Net %"],["M2",4,"Days"],["N2",84],["P2",4,"Gross %"],["Q2",4,"Impact Cost"],["R2",4,"Rollover Cost"],["S2",4,"Net %"],["T2",4,"Days"],["U2",84],["V2",84],
        ["A3",85,"Total Profit %"],["B3",6,null,"SUM(G21:G1048576)"],["C3",6,null,"SUM(I21:I1048576)"],["D3",6,null,"SUM(J21:J1048576)"],["E3",6,null,"SUM(K21:K1048576)"],["F3",86,null,"SUM(H21:H1048576)"],["G3",87],["H3",85,"Total Profit %"],["I3",6,null,"SUM(IF(H$1<=C21:C1048576,G21:G1048576,\"\"))",1],["J3",6,null,"SUM(IF(H$1<=C21:C1048576,I21:I1048576,\"\"))",1],["K3",6,null,"SUM(IF(H$1<=C21:C1048576,J21:J1048576,\"\"))",1],["L3",6,null,"SUM(IF(H$1<=C21:C1048576,K21:K1048576,\"\"))",1],["M3",86,null,"SUM(IF(H$1<=C21:C1048576,H21:H1048576,\"\"))",1],["N3",88],["O3",85,"Total Profit %"],["P3",6,null,"SUM(IF(O$1<=C21:C1048576,G21:G1048576,\"\"))",1],["Q3",6,null,"SUM(IF(O$1<=C21:C1048576,I21:I1048576,\"\"))",1],["R3",6,null,"SUM(IF(O$1<=C21:C1048576,J21:J1048576,\"\"))",1],["S3",6,null,"SUM(IF(O$1<=C21:C1048576,K21:K1048576,\"\"))",1],["T3",86,null,"SUM(IF(O$1<=C21:C1048576,H21:H1048576,\"\"))",1],["U3",88],["V3",88],["W3",89,"Dates"],["X3",90,"Start Date"],["Y3",90,"End Date"],["Z3",90,"Trades"],["AA3",90,"Total Gross Profit"],["AB3",90,"Average Gross Profit"],["AC3",90,"Gross Month. Thk."],["AD3",90,"Total Net Profit"],["AE3",90,"Average Net Profit"],["AF3",90,"Net Month. Thk."],["AG3",90,"Average Days Held"],["AH3",91,"Months Held"],["AI3",92],
        ["A4",93,"Avg %"],["B4",6,null,"AVERAGE(G21:G1048576)"],["C4",14],["D4",14],["E4",6,null,"AVERAGE(K21:K1048576)"],["F4",86,null,"AVERAGE(H21:H1048576)"],["H4",93,"Avg %"],["I4",6,null,"AVERAGE(IF(H$1<=C21:C1048576,G21:G1048576,\"\"))",1],["J4",14],["K4",14],["L4",6,null,"AVERAGE(IF(H$1<=C21:C1048576,K21:K1048576,\"\"))",1],["M4",86,null,"AVERAGE(IF(H$1<=C21:C1048576,H21:H1048576,\"\"))",1],["N4",84],["O4",93,"Avg %"],["P4",6,null,"AVERAGE(IF(O$1<=C21:C1048576,G21:G1048576,\"\"))",1],["Q4",14],["R4",14],["S4",6,null,"AVERAGE(IF(O$1<=C21:C1048576,K21:K1048576,\"\"))",1],["T4",86,null,"AVERAGE(IF(O$1<=C21:C1048576,H21:H1048576,\"\"))",1],["U4",84],["V4",84],["W4",94," 2008 - 2026"],["X4",95,39447],["Y4",95,46387],["Z4",96,null,"COUNT(IF((X4<=$C$21:$C$1048576)*(Y4>=$E$21:$E$1048576),$G$21:$G$1048576,\"\"))",1],["AA4",18,null,"SUM(IF((X4<=$C$21:$C$1048576)*(Y4>=$E$21:$E$1048576),$G$21:$G$1048576,\"\"))",1],["AB4",18,null,"AVERAGE(IF((X4<=$C$21:$C$1048576)*(Y4>=$E$21:$E$1048576),$G$21:$G$1048576,\"\"))",1],["AC4",18,null,"AB4/AH4"],["AD4",18,null,"SUM(IF((X4<=$C$21:$C$1048576)*(Y4>=$E$21:$E$1048576),$K$21:$K$1048576,\"\"))",1],["AE4",18,null,"AVERAGE(IF((X4<=$C$21:$C$1048576)*(Y4>=$E$21:$E$1048576),$K$21:$K$1048576,\"\"))",1],["AF4",18,null,"AE4/AH4"],["AG4",96,null,"AVERAGE(IF((X4<=$C$21:$C$1048576)*(Y4>=$E$21:$E$1048576),$H$21:$H$1048576,\"\"))",1],["AH4",97,null,"AG4/30.5"],
        ["A5",-1,"Total Trades"],["B5",98,null,"COUNT(G21:G1048576)"],["C5",22],["D5",22],["E5",98,null,"COUNT(K21:K1048576)"],["F5",98,null,"F4/30.5"],["H5",-1,"Total Trades"],["I5",98,null,"COUNT(IF(H$1<=C21:C1048576,G21:G1048576,\"\"))",1],["J5",22],["K5",22],["L5",98,null,"COUNT(IF(H$1<=C21:C1048576,K21:K1048576,\"\"))",1],["M5",98,null,"M4/30.5"],["N5",84],["O5",-1,"Total Trades"],["P5",98,null,"COUNT(IF(O$1<=C21:C1048576,G21:G1048576,\"\"))",1],["Q5",22],["R5",22],["S5",98,null,"COUNT(IF(O$1<=C21:C1048576,K21:K1048576,\"\"))",1],["T5",98,null,"T4/30.5"],["U5",84],["V5",84],["W5",94," 2008 - 2010"],["X5",95,39447],["Y5",95,40543],["Z5",96,null,"COUNT(IF((X5<=$C$21:$C$1048576)*(Y5>=$E$21:$E$1048576),$G$21:$G$1048576,\"\"))",1],["AA5",18,null,"SUM(IF((X5<=$C$21:$C$1048576)*(Y5>=$E$21:$E$1048576),$G$21:$G$1048576,\"\"))",1],["AB5",18,null,"AVERAGE(IF((X5<=$C$21:$C$1048576)*(Y5>=$E$21:$E$1048576),$G$21:$G$1048576,\"\"))",1],["AC5",18,null,"AB5/AH5"],["AD5",18,null,"SUM(IF((X5<=$C$21:$C$1048576)*(Y5>=$E$21:$E$1048576),$K$21:$K$1048576,\"\"))",1],["AE5",18,null,"AVERAGE(IF((X5<=$C$21:$C$1048576)*(Y5>=$E$21:$E$1048576),$K$21:$K$1048576,\"\"))",1],["AF5",18,null,"AE5/AH5"],["AG5",96,null,"AVERAGE(IF((X5<=$C$21:$C$1048576)*(Y5>=$E$21:$E$1048576),$H$21:$H$1048576,\"\"))",1],["AH5",97,null,"AG5/30.5"],
        ["A6",93,"Win Ratio"],["B6",6,null,"COUNTIF(G21:G1048576,\">0\")/B5"],["C6",14],["D6",14],["E6",6,null,"COUNTIF(K21:K1048576,\">0\")/E5"],["F6",14],["H6",93,"Win Ratio"],["I6",6,null,"COUNT(IF((H$1<=C21:C1048576)*(0<G21:G1048576),G21:G1048576,\"\"))/I5",1],["J6",14],["K6",14],["L6",6,null,"COUNT(IF((H$1<=C21:C1048576)*(0<K21:K1048576),K21:K1048576,\"\"))/L5",1],["M6",14],["N6",84],["O6",93,"Win Ratio"],["P6",6,null,"COUNT(IF((O$1<=C21:C1048576)*(0<G21:G1048576),G21:G1048576,\"\"))/P5",1],["Q6",14],["R6",14],["S6",6,null,"COUNT(IF((O$1<=C21:C1048576)*(0<K21:K1048576),K21:K1048576,\"\"))/S5",1],["T6",14],["U6",84],["V6",84],["W6",94," 2011 - 2019"],["X6",95,40543],["Y6",95,43830],["Z6",96,null,"COUNT(IF((X6<=$C$21:$C$1048576)*(Y6>=$E$21:$E$1048576),$G$21:$G$1048576,\"\"))",1],["AA6",18,null,"SUM(IF((X6<=$C$21:$C$1048576)*(Y6>=$E$21:$E$1048576),$G$21:$G$1048576,\"\"))",1],["AB6",18,null,"AVERAGE(IF((X6<=$C$21:$C$1048576)*(Y6>=$E$21:$E$1048576),$G$21:$G$1048576,\"\"))",1],["AC6",18,null,"AB6/AH6"],["AD6",18,null,"SUM(IF((X6<=$C$21:$C$1048576)*(Y6>=$E$21:$E$1048576),$K$21:$K$1048576,\"\"))",1],["AE6",18,null,"AVERAGE(IF((X6<=$C$21:$C$1048576)*(Y6>=$E$21:$E$1048576),$K$21:$K$1048576,\"\"))",1],["AF6",18,null,"AE6/AH6"],["AG6",96,null,"AVERAGE(IF((X6<=$C$21:$C$1048576)*(Y6>=$E$21:$E$1048576),$H$21:$H$1048576,\"\"))",1],["AH6",97,null,"AG6/30.5"],
        ["A7",-1,"Win Avg"],["B7",24,null,"AVERAGEIF(G21:G1048576,\">0\",G21:G1048576)"],["C7",22],["D7",22],["E7",24,null,"AVERAGEIF(K21:K1048576,\">0\",K21:K1048576)"],["F7",22],["H7",-1,"Win Avg"],["I7",24,null,"AVERAGE(IF((H$1<=C21:C1048576)*(0<G21:G1048576),G21:G1048576,\"\"))",1],["J7",22],["K7",22],["L7",24,null,"AVERAGE(IF((H$1<=C21:C1048576)*(0<K21:K1048576),K21:K1048576,\"\"))",1],["M7",22],["N7",84],["O7",-1,"Win Avg"],["P7",24,null,"AVERAGE(IF((O$1<=C21:C1048576)*(0<G21:G1048576),G21:G1048576,\"\"))",1],["Q7",22],["R7",22],["S7",24,null,"AVERAGE(IF((O$1<=C21:C1048576)*(0<K21:K1048576),K21:K1048576,\"\"))",1],["T7",22],["U7",84],["V7",84],["W7",99," 2011 - 2026"],["X7",100,40543],["Y7",100,46387],["Z7",101,null,"COUNT(IF((X7<=$C$21:$C$1048576)*(Y7>=$E$21:$E$1048576),$G$21:$G$1048576,\"\"))",1],["AA7",28,null,"SUM(IF((X7<=$C$21:$C$1048576)*(Y7>=$E$21:$E$1048576),$G$21:$G$1048576,\"\"))",1],["AB7",28,null,"AVERAGE(IF((X7<=$C$21:$C$1048576)*(Y7>=$E$21:$E$1048576),$G$21:$G$1048576,\"\"))",1],["AC7",28,null,"AB7/AH7"],["AD7",28,null,"SUM(IF((X7<=$C$21:$C$1048576)*(Y7>=$E$21:$E$1048576),$K$21:$K$1048576,\"\"))",1],["AE7",28,null,"AVERAGE(IF((X7<=$C$21:$C$1048576)*(Y7>=$E$21:$E$1048576),$K$21:$K$1048576,\"\"))",1],["AF7",28,null,"AE7/AH7"],["AG7",101,null,"AVERAGE(IF((X7<=$C$21:$C$1048576)*(Y7>=$E$21:$E$1048576),$H$21:$H$1048576,\"\"))",1],["AH7",102,null,"AG7/30.5"],
        ["A8",-1,"Loss Avg"],["B8",24,null,"AVERAGEIF(G21:G1048576,\"<0\",G21:G1048576)"],["C8",22],["D8",22],["E8",24,null,"AVERAGEIF(K21:K1048576,\"<0\",K21:K1048576)"],["F8",22],["H8",-1,"Loss Avg"],["I8",24,null,"AVERAGE(IF((H$1<=C21:C1048576)*(0>G21:G1048576),G21:G1048576,\"\"))",1],["J8",22],["K8",22],["L8",24,null,"AVERAGE(IF((H$1<=C21:C1048576)*(0>K21:K1048576),K21:K1048576,\"\"))",1],["M8",22],["N8",84],["O8",-1,"Loss Avg"],["P8",24,null,"AVERAGE(IF((O$1<=C21:C1048576)*(0>G21:G1048576),G21:G1048576,\"\"))",1],["Q8",22],["R8",22],["S8",24,null,"AVERAGE(IF((O$1<=C21:C1048576)*(0>K21:K1048576),K21:K1048576,\"\"))",1],["T8",22],["U8",84],["V8",84],["W8",30],
        ["A9",-1,"Max Winner"],["B9",24,null,"MAX(G21:G1048576)"],["C9",22],["D9",22],["E9",24,null,"MAX(K21:K501)"],["F9",22],["H9",-1,"Max Winner"],["I9",24,null,"MAX(IF(H$1<=C21:C1048576,G21:G1048576,\"\"))",1],["J9",22],["K9",22],["L9",24,null,"MAX(IF(H$1<=C21:C1048576,K21:K1048576,\"\"))",1],["M9",22],["N9",84],["O9",-1,"Max Winner"],["P9",24,null,"MAX(IF(O$1<=C21:C1048576,G21:G1048576,\"\"))",1],["Q9",22],["R9",22],["S9",24,null,"MAX(IF(O$1<=C21:C1048576,K21:K1048576,\"\"))",1],["T9",22],["U9",84],["V9",84],
        ["A10",-1,"Max Looser"],["B10",24,null,"MIN(G21:G1048576)"],["C10",22],["D10",22],["E10",24,null,"MIN(K21:K501)"],["F10",22],["H10",-1,"Max Looser"],["I10",24,null,"MIN(IF(H$1<=C21:C1048576,G21:G1048576,\"\"))",1],["J10",22],["K10",22],["L10",24,null,"MIN(IF(H$1<=C21:C1048576,K21:K1048576,\"\"))",1],["M10",22],["N10",84],["O10",-1,"Max Looser"],["P10",24,null,"MIN(IF(O$1<=C21:C1048576,G21:G1048576,\"\"))",1],["Q10",22],["R10",22],["S10",24,null,"MIN(IF(O$1<=C21:C1048576,K21:K1048576,\"\"))",1],["T10",22],["U10",84],["V10",84],["W10",103,"Rules:"],["X10",32,"Entry:"],["Y10",-1,"Before 0 Days"],
        ["A11",93,"Expectancy Ratio"],["B11",86,null,"((B6*B7)-((1-B6)*ABS(B8)))/(ABS(B8))"],["C11",14],["D11",14],["E11",86,null,"((E6*E7)-((1-E6)*ABS(E8)))/(ABS(E8))"],["F11",14],["H11",93,"Expectancy Ratio"],["I11",86,null,"((I6*I7)-((1-I6)*ABS(I8)))/(ABS(I8))"],["J11",14],["K11",14],["L11",86,null,"((L6*L7)-((1-L6)*ABS(L8)))/(ABS(L8))"],["M11",14],["N11",84],["O11",93,"Expectancy Ratio"],["P11",86,null,"((P6*P7)-((1-P6)*ABS(P8)))/(ABS(P8))"],["Q11",14],["R11",14],["S11",86,null,"((S6*S7)-((1-S6)*ABS(S8)))/(ABS(S8))"],["T11",14],["U11",84],["V11",84],["X11",32,"Exit:"],["Y11",-1,"ATR - 5- 1 With HSL close"],
        ["A12",-1,"Monthly Thickness"],["B12",24,null,"B4/F4*30"],["C12",22],["D12",22],["E12",24,null,"E4/F4*30"],["F12",22],["H12",-1,"Monthly Thickness"],["I12",24,null,"I4/M4*30"],["J12",22],["K12",22],["L12",24,null,"L4/M4*30"],["M12",22],["N12",84],["O12",-1,"Monthly Thickness"],["P12",24,null,"P4/T4*30"],["Q12",22],["R12",22],["S12",24,null,"S4/T4*30"],["T12",22],["U12",84],["V12",84],["W12",12],["X12",32,"Filter:"],["Y12",-1,"WOF"],
        ["A13",-1,"Risk Reward"],["B13",98,null,"ABS(B7/B8)"],["C13",22],["D13",22],["E13",98,null,"ABS(E7/E8)"],["F13",22],["H13",-1,"Risk Reward"],["I13",98,null,"ABS(I7/I8)"],["J13",22],["K13",22],["L13",98,null,"ABS(L7/L8)"],["M13",22],["N13",84],["O13",-1,"Risk Reward"],["P13",98,null,"ABS(P7/P8)"],["Q13",22],["R13",22],["S13",98,null,"ABS(S7/S8)"],["T13",22],["U13",84],["V13",84],["W13",12],
        ["A14",-1,"CAGR"],["B14",24,null,"(INDEX($L$20:$L$1048576,MATCH(MAX($E$20:$E$1048576),$E$20:$E$1048576,0),1)/100)^(1/(((MAX(E20:E1048573)-MIN(C20:C1048576))/365)))-1"],["C14",22],["D14",22],["E14",24,null,"(INDEX($Q$20:$Q$1048576,MATCH(MAX($E$20:$E$1048576),$E$20:$E$1048576,0),1)/100)^(1/(((MAX(E20:E1048576)-MIN(C20:C1048576))/365)))-1"],["F14",22],["H14",-1,"CAGR"],["I14",24,null,"(INDEX(L20:L1048573,MATCH(L1,E20:E1048573,0),1)/INDEX(L20:L1048573,MATCH(J1,C20:C1048573,0)-1,1))^(1/(((MAX(E20:E1048573)-J1)/365)))-1"],["J14",22],["K14",22],["L14",24,null,"(INDEX(Q20:Q1048573,MATCH(L1,E20:E1048573,0),1)/INDEX(Q20:Q1048573,MATCH(J1,C20:C1048573,0)-1,1))^(1/(((MAX(E20:E1048573)-J1)/365)))-1"],["M14",22],["N14",84],["O14",-1,"CAGR"],["P14",24,null,"(INDEX(L20:L1048573,MATCH(S1,E20:E1048573,0),1)/INDEX(L20:L1048573,MATCH(Q1,C20:C1048573,0)-1,1))^(1/(((MAX(E20:E1048573)-Q1)/365)))-1"],["Q14",22],["R14",22],["S14",24,null,"(INDEX(Q20:Q1048573,MATCH(S1,E20:E1048573,0),1)/INDEX(Q20:Q1048573,MATCH(Q1,C20:C1048573,0)-1,1))^(1/(((MAX(E20:E1048573)-Q1)/365)))-1"],["T14",22],["U14",84],["V14",84],
        ["A15",-1,"DD"],["B15",24,null,"MIN($O$20:$O$1048576)"],["E15",24,null,"MIN($T$20:$T$1048576)"],["H15",-1,"DD"],["I15",24,null,"MIN(IF(H$1<=C21:C1048576,O21:O1048576,\"\"))",1],["L15",24,null,"MIN(IF(J$1<=C21:C1048576,T21:T1048576,\"\"))",1],["N15",84],["O15",-1,"DD"],["P15",24,null,"MIN(IF(O$1<=C21:C1048576,O21:O1048576,\"\"))",1],["S15",24,null,"MIN(IF(Q$1<=C21:C1048576,T21:T1048576,\"\"))",1],["U15",84],["V15",84],
        ["A16",-1,"CAR/MDD"],["B16",98,null,"ABS(B14/B15)"],["E16",98,null,"ABS(E14/E15)"],["H16",-1,"CAR/MDD"],["I16",98,null,"ABS(I14/I15)"],["L16",98,null,"ABS(L14/L15)"],["N16",84],["O16",-1,"CAR/MDD"],["P16",98,null,"ABS(P14/P15)"],["S16",98,null,"ABS(S14/S15)"],["U16",84],["V16",84],
        ["A17",-1,"Max MAE"],["E17",49,null,"MIN($W$21:$W$374)"],["G17",82],["H17",-1,"Max MAE"],["I17",84],["K17",84],["L17",49,null,"MIN(IF(H1<=$E$21:$E$374,$W$21:$W$374,\"\"))",1],["O17",-1,"Max MAE"],["P17",84],["R17",84],["S17",49,null,"MIN(IF(O1<=$E$21:$E$374,$W$21:$W$374,\"\"))",1],["W17",84],["X17",84],
        ["A18",-1,"Avg MAE"],["E18",49,null,"AVERAGE($W$21:$W$374)"],["G18",82],["H18",-1,"Avg MAE"],["I18",84],["L18",49,null,"AVERAGE(IF(H1<=$E$21:$E$374,$W$21:$W$374,\"\"))",1],["M18",-1,"Gross"],["O18",-1,"Avg MAE"],["P18",84],["S18",49,null,"AVERAGE(IF(O1<=$E$21:$E$374,$W$21:$W$374,\"\"))",1],["T18",-1,"Gross"],
        ["C19",82],["E19",82],["G19",84],["L19",-1,"CUMM"],["M19",-1,"Peak"],["N19",-1,"DD"],["O19",-1,"%DD"],["Q19",-1,"CUMM"],["R19",-1,"Peak"],["S19",-1,"DD"],["T19",-1,"%DD"],["U19",-1,"Year by exit Date"],["V19",-1,"Year by entry Date"],
        ["A20",46,"Symbol"],["B20",46,"Trade"],["C20",46,"En. date"],["D20",46,"En. Price"],["E20",46,"Ex. date"],["F20",46,"Ex. Price"],["G20",46,"% chg"],["H20",50,"Days"],["I20",61,"Impact cost"],["J20",46,"Rollover Cost"],["K20",46,"Net Profit"],["L20",104,100],["M20",104,null,"MAX(L20,M19)"],["N20",46,null,"IF(M20>L20,L20-M20,\"\")"],["O20",105],["P20",46],["Q20",104,100],["R20",104,null,"MAX(Q20,R19)"],["S20",46,null,"IF(R20>Q20,Q20-R20,\"\")"],["T20",105],["U20",46],["V20",46],["W20",46,"MAE"],["Z20",53,"Year"],["AA20",54,"Net Return"],
        ["Z21",106,2008],["AA21",56,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z21,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z21,$K$21:$K$250)),\"\")"],
        ["Z22",107,2009],["AA22",58,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z22,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z22,$K$21:$K$250)),\"\")"],
        ["Z23",106,2010],["AA23",56,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z23,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z23,$K$21:$K$250)),\"\")"],
        ["Z24",107,2011],["AA24",58,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z24,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z24,$K$21:$K$250)),\"\")"],
        ["Z25",106,2012],["AA25",56,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z25,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z25,$K$21:$K$250)),\"\")"],
        ["Z26",107,2013],["AA26",58,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z26,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z26,$K$21:$K$250)),\"\")"],
        ["Z27",106,2014],["AA27",56,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z27,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z27,$K$21:$K$250)),\"\")"],
        ["Z28",107,2015],["AA28",58,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z28,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z28,$K$21:$K$250)),\"\")"],
        ["Z29",106,2016],["AA29",56,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z29,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z29,$K$21:$K$250)),\"\")"],
        ["Z30",107,2017],["AA30",58,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z30,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z30,$K$21:$K$250)),\"\")"],
        ["Z31",106,2018],["AA31",56,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z31,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z31,$K$21:$K$250)),\"\")"],
        ["Z32",107,2019],["AA32",58,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z32,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z32,$K$21:$K$250)),\"\")"],
        ["Z33",106,2020],["AA33",56,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z33,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z33,$K$21:$K$250)),\"\")"],
        ["Z34",107,2021],["AA34",58,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z34,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z34,$K$21:$K$250)),\"\")"],
        ["Z35",106,2022],["AA35",56,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z35,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z35,$K$21:$K$250)),\"\")"],
        ["Z36",107,2023],["AA36",58,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z36,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z36,$K$21:$K$250)),\"\")"],
        ["Z37",106,2024],["AA37",56,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z37,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z37,$K$21:$K$250)),\"\")"],
        ["Z38",107,2025],["AA38",58,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z38,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z38,$K$21:$K$250)),\"\")"],
        ["Z39",106,2026],["AA39",58,null,"IFERROR(IF(SUMIF($U$21:$U$250,Z39,$K$21:$K$250)=0,NA(),SUMIF($U$21:$U$250,Z39,$K$21:$K$250)),\"\")"],
        ["Z40",59,"Total Net Return"],["AA40",1,null,"SUM($AA$21:$AA$39)"],
      ],
    },
    summary: {
      cols: [3.85546875,0,29.0,0,12.0,10.28515625,0,0,0,0,0,0,0,11.42578125,10.85546875,0,0,0,11.0,11.7109375],
      cells: [
        ["D1",61],["E1",61],
        ["C2",62],["D2",109],["E2",109],
        ["B3",110,"Sr. No."],["C3",110,"All Sheets Name"],["D3",110,"From"],["E3",110,"Total Net Profit %"],["F3",110,"Net Avg %"],["G3",111,"Avarage Days"],["H3",110,"Total Trades"],["I3",110,"Win Ratio"],["J3",110,"Win Avg"],["K3",110,"Loss Avg"],["L3",110,"Max Winner"],["M3",110,"Max Looser"],["N3",110,"Expectancy Ratio"],["O3",110,"Monthly Thickness"],["P3",110,"Risk Reward"],["Q3",110,"CAGR"],["R3",110,"DD"],["S3",110,"CAR/MDD"],["T3",110,"Max MAE"],["U3",110,"Avg MAE"],
        ["B4",112,1],["C4",67,null,"INDIRECT(\"'\"&$C$3&\"'!B\"&ROW()+B4)"],["D4",112,2000],["E4",69,null,"IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!E3\"))"],["F4",69,null,"IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!E4\"))"],["G4",113,null,"IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!F4\"))"],["H4",114,null,"IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!E5\"))"],["I4",69,null,"IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!E6\"))"],["J4",69,null,"IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!E7\"))"],["K4",69,null,"IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!E8\"))"],["L4",69,null,"IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!E9\"))"],["M4",69,null,"IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!E10\"))"],["N4",113,null,"IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!E11\"))"],["O4",69,null,"IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!E12\"))"],["P4",113,null,"IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!E13\"))"],["Q4",69,null,"IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!E14\"))"],["R4",115,null,"IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!E15\"))"],["S4",113,null,"IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!E16\"))"],["T4",116,null,"IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!e17\"))"],["U4",116,null,"IF($C5=\"\",\"\",INDIRECT(\"'\" & $C5 & \"'!e18\"))"],
        ["B5",72],["C5",67,null,"C4"],["D5",112,2008],["E5",69,null,"IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!L3\"))"],["F5",69,null,"IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!L4\"))"],["G5",113,null,"IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!M4\"))"],["H5",114,null,"IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!L5\"))"],["I5",69,null,"IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!L6\"))"],["J5",69,null,"IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!L7\"))"],["K5",69,null,"IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!L8\"))"],["L5",69,null,"IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!L9\"))"],["M5",69,null,"IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!L10\"))"],["N5",113,null,"IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!L11\"))"],["O5",69,null,"IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!L12\"))"],["P5",113,null,"IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!L13\"))"],["Q5",69,null,"IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!L14\"))"],["R5",115,null,"IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!L15\"))"],["S5",113,null,"IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!L16\"))"],["T5",116,null,"IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!L17\"))"],["U5",116,null,"IF($C6=\"\",\"\",INDIRECT(\"'\" & $C6 & \"'!L18\"))"],
        ["B6",72],["C6",67,null,"C5"],["D6",117,2011],["E6",69,null,"IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!S3\"))"],["F6",69,null,"IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!S4\"))"],["G6",113,null,"IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!T4\"))"],["H6",114,null,"IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!S5\"))"],["I6",69,null,"IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!S6\"))"],["J6",69,null,"IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!S7\"))"],["K6",69,null,"IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!S8\"))"],["L6",69,null,"IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!S9\"))"],["M6",69,null,"IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!S10\"))"],["N6",113,null,"IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!S11\"))"],["O6",69,null,"IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!S12\"))"],["P6",113,null,"IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!S13\"))"],["Q6",69,null,"IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!S14\"))"],["R6",115,null,"IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!S15\"))"],["S6",113,null,"IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!S16\"))"],["T6",116,null,"IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!S17\"))"],["U6",116,null,"IF($C4=\"\",\"\",INDIRECT(\"'\" & $C4 & \"'!S18\"))"],
        ["B7",112,2],["C7",67,null,"INDIRECT(\"'\"&$C$3&\"'!B\"&ROW()+B7-3)"],["D7",112,2000],["E7",69,null,"IF($C8=\"\",\"\",INDIRECT(\"'\" & $C8 & \"'!E3\"))"],["F7",69,null,"IF($C8=\"\",\"\",INDIRECT(\"'\" & $C8 & \"'!E4\"))"],["G7",113,null,"IF($C8=\"\",\"\",INDIRECT(\"'\" & $C8 & \"'!F4\"))"],["H7",114,null,"IF($C8=\"\",\"\",INDIRECT(\"'\" & $C8 & \"'!E5\"))"],["I7",69,null,"IF($C8=\"\",\"\",INDIRECT(\"'\" & $C8 & \"'!E6\"))"],["J7",69,null,"IF($C8=\"\",\"\",INDIRECT(\"'\" & $C8 & \"'!E7\"))"],["K7",69,null,"IF($C8=\"\",\"\",INDIRECT(\"'\" & $C8 & \"'!E8\"))"],["L7",69,null,"IF($C8=\"\",\"\",INDIRECT(\"'\" & $C8 & \"'!E9\"))"],["M7",69,null,"IF($C8=\"\",\"\",INDIRECT(\"'\" & $C8 & \"'!E10\"))"],["N7",113,null,"IF($C8=\"\",\"\",INDIRECT(\"'\" & $C8 & \"'!E11\"))"],["O7",69,null,"IF($C8=\"\",\"\",INDIRECT(\"'\" & $C8 & \"'!E12\"))"],["P7",113,null,"IF($C8=\"\",\"\",INDIRECT(\"'\" & $C8 & \"'!E13\"))"],["Q7",69,null,"IF($C8=\"\",\"\",INDIRECT(\"'\" & $C8 & \"'!E14\"))"],["R7",115,null,"IF($C8=\"\",\"\",INDIRECT(\"'\" & $C8 & \"'!E15\"))"],["S7",113,null,"IF($C8=\"\",\"\",INDIRECT(\"'\" & $C8 & \"'!E16\"))"],["T7",116,null,"IF($C8=\"\",\"\",INDIRECT(\"'\" & $C8 & \"'!e17\"))"],["U7",116,null,"IF($C8=\"\",\"\",INDIRECT(\"'\" & $C8 & \"'!e18\"))"],
        ["B8",72],["C8",67,null,"C7"],["D8",112,2008],["E8",69,null,"IF($C9=\"\",\"\",INDIRECT(\"'\" & $C9 & \"'!L3\"))"],["F8",69,null,"IF($C9=\"\",\"\",INDIRECT(\"'\" & $C9 & \"'!L4\"))"],["G8",113,null,"IF($C9=\"\",\"\",INDIRECT(\"'\" & $C9 & \"'!M4\"))"],["H8",114,null,"IF($C9=\"\",\"\",INDIRECT(\"'\" & $C9 & \"'!L5\"))"],["I8",69,null,"IF($C9=\"\",\"\",INDIRECT(\"'\" & $C9 & \"'!L6\"))"],["J8",69,null,"IF($C9=\"\",\"\",INDIRECT(\"'\" & $C9 & \"'!L7\"))"],["K8",69,null,"IF($C9=\"\",\"\",INDIRECT(\"'\" & $C9 & \"'!L8\"))"],["L8",69,null,"IF($C9=\"\",\"\",INDIRECT(\"'\" & $C9 & \"'!L9\"))"],["M8",69,null,"IF($C9=\"\",\"\",INDIRECT(\"'\" & $C9 & \"'!L10\"))"],["N8",113,null,"IF($C9=\"\",\"\",INDIRECT(\"'\" & $C9 & \"'!L11\"))"],["O8",69,null,"IF($C9=\"\",\"\",INDIRECT(\"'\" & $C9 & \"'!L12\"))"],["P8",113,null,"IF($C9=\"\",\"\",INDIRECT(\"'\" & $C9 & \"'!L13\"))"],["Q8",69,null,"IF($C9=\"\",\"\",INDIRECT(\"'\" & $C9 & \"'!L14\"))"],["R8",115,null,"IF($C9=\"\",\"\",INDIRECT(\"'\" & $C9 & \"'!L15\"))"],["S8",113,null,"IF($C9=\"\",\"\",INDIRECT(\"'\" & $C9 & \"'!L16\"))"],["T8",116,null,"IF($C9=\"\",\"\",INDIRECT(\"'\" & $C9 & \"'!L17\"))"],["U8",116,null,"IF($C9=\"\",\"\",INDIRECT(\"'\" & $C9 & \"'!L18\"))"],
        ["B9",72],["C9",67,null,"C8"],["D9",117,2011],["E9",69,null,"IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!S3\"))"],["F9",69,null,"IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!S4\"))"],["G9",113,null,"IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!T4\"))"],["H9",114,null,"IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!S5\"))"],["I9",69,null,"IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!S6\"))"],["J9",69,null,"IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!S7\"))"],["K9",69,null,"IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!S8\"))"],["L9",69,null,"IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!S9\"))"],["M9",69,null,"IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!S10\"))"],["N9",113,null,"IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!S11\"))"],["O9",69,null,"IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!S12\"))"],["P9",113,null,"IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!S13\"))"],["Q9",69,null,"IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!S14\"))"],["R9",115,null,"IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!S15\"))"],["S9",113,null,"IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!S16\"))"],["T9",116,null,"IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!S17\"))"],["U9",116,null,"IF($C7=\"\",\"\",INDIRECT(\"'\" & $C7 & \"'!S18\"))"],
      ],
    },
    yoy: {
      cols: [0,15.7109375,12.28515625],
      rows: {"2":18.75,"3":13.5,"4":63.75,"5":2.25},
      colStyles: [
        [-1,83,-1,138,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,140,141,142,143,144,145,18,146,18,147,18,18,147,148,149,-1,-1],
        [-1,83,-1,138,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,140,141,142,143,144,145,18,146,18,147,18,18,147,148,149,-1,-1],
        [-1,83,-1,138,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,140,141,142,143,144,145,18,146,18,147,18,18,147,148,149,-1,-1],
        [-1,83,-1,138,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,140,141,142,143,144,145,18,146,18,147,18,18,147,148,149,-1,-1],
        [-1,83,-1,138,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,140,141,142,143,144,145,18,146,18,147,18,18,147,148,149,-1,-1],
        [-1,83,-1,150,151,151,151,151,151,151,151,151,151,151,151,151,151,151,151,151,151,151,151,152,153,154,155,156,157,158,159,158,160,158,158,160,161,162,-1,-1],
        [-1,83,-1,119,121,121,121,121,121,121,121,121,121,121,121,121,121,121,121,121,121,121,121,124,126,128,130,163,133,134,164,134,165,134,134,165,135,137,-1,-1],
        [-1,83,-1,138,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,140,141,142,143,144,145,18,146,18,147,18,18,147,148,149,-1,-1],
        [-1,83,-1,138,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,140,141,142,143,144,145,18,146,18,147,18,18,147,148,149,-1,-1],
        [-1,83,-1,138,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,140,141,142,143,144,145,18,146,18,147,18,18,147,148,149,-1,-1],
        [-1,83,-1,166,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,140,141,142,143,144,145,18,146,18,147,18,18,147,148,149,-1,-1],
        [-1,83,-1,138,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,140,141,142,143,144,145,18,146,18,147,18,18,147,148,149,-1,-1],
        [-1,83,-1,150,151,151,151,151,151,151,151,151,151,151,151,151,151,151,151,151,151,151,151,152,153,154,155,156,157,158,159,158,160,158,158,160,161,162,-1,-1],
        [-1,83,-1,119,121,121,121,121,121,121,121,121,121,121,121,121,121,121,121,121,121,121,121,124,126,128,130,163,133,134,164,134,165,134,134,165,135,137,-1,-1],
        [-1,83,-1,167,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,140,141,142,143,144,145,18,146,18,147,18,18,147,148,149,-1,-1],
        [-1,83,-1,138,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,140,141,142,143,144,145,18,146,18,147,18,18,147,148,149,-1,-1],
        [-1,83,-1,138,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,140,141,142,143,144,145,18,146,18,147,18,18,147,148,149,-1,-1],
        [-1,83,-1,167,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,139,140,141,142,143,144,145,18,146,18,147,18,18,147,148,149,-1,-1],
        [-1,83,-1,150,151,151,151,151,151,151,151,151,151,151,151,151,151,151,151,151,151,151,151,152,153,154,155,156,157,158,159,158,160,158,158,160,161,162,-1,-1],
      ],
      cells: [
        ["B1",12,"Note: The YOY Returns are on net basis and from exit date (From 2008)"],
        ["C2",83,1],
        ["C3",-1,"All Sheets Name"],
        ["B4",118],["C4",119,null,"INDIRECT(\"'\"&$C$3&\"'!B\"&ROW()+C2)"],
        ["B5",120,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C5",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B6",122,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C6",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B7",122,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C7",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B8",122,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C8",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B9",122,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C9",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B10",122,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C10",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B11",122,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C11",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B12",122,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C12",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B13",122,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C13",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B14",122,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C14",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B15",122,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C15",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B16",122,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C16",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B17",122,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C17",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B18",122,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C18",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B19",122,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C19",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B20",122,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C20",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B21",122,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C21",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B22",122,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C22",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B23",122,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C23",121,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B24",123,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C24",124,null,"INDIRECT(\"'\"&C$4&\"'!aa\"&ROW()+15)"],
        ["B25",125,null,"INDIRECT(\"'\"&C$4&\"'!z\"&ROW()+15)"],["C25",126,null,"INDIRECT(\"'\"&C$4&\"'!l\"&ROW()-22)"],
        ["B26",127,"Net Avg %"],["C26",128,null,"INDIRECT(\"'\"&C$4&\"'!l\"&ROW()-22)"],
        ["B27",129,"Avg. Days"],["C27",130,null,"INDIRECT(\"'\"&C$4&\"'!m\"&ROW()-23)"],
        ["B28",131,"Total Trades"],["C28",132,null,"INDIRECT(\"'\"&C$4&\"'!l\"&ROW()-23)"],
        ["B29",129,"Win Ratio"],["C29",133,null,"INDIRECT(\"'\"&C$4&\"'!l\"&ROW()-23)"],
        ["B30",131,"Win Avg"],["C30",134,null,"INDIRECT(\"'\"&C$4&\"'!l\"&ROW()-23)"],
        ["B31",129,"Expectancy Ratio"],["C31",130,null,"INDIRECT(\"'\"&C$4&\"'!l\"&ROW()-20)"],
        ["B32",131,"Monthly Thickness"],["C32",134,null,"INDIRECT(\"'\"&C$4&\"'!l\"&ROW()-20)"],
        ["B33",131,"Risk Reward"],["C33",132,null,"INDIRECT(\"'\"&C$4&\"'!l\"&ROW()-20)"],
        ["B34",131,"CAGR"],["C34",134,null,"INDIRECT(\"'\"&C$4&\"'!l\"&ROW()-20)"],
        ["B35",131,"DD"],["C35",134,null,"INDIRECT(\"'\"&C$4&\"'!l\"&ROW()-20)"],
        ["B36",131,"CAR/MDD"],["C36",132,null,"INDIRECT(\"'\"&C$4&\"'!l\"&ROW()-20)"],
        ["B37",131,"Max MAE"],["C37",135,null,"INDIRECT(\"'\"&C$4&\"'!l\"&ROW()-20)"],
        ["B38",136,"Avg MAE"],["C38",137,null,"INDIRECT(\"'\"&C$4&\"'!l\"&ROW()-20)"],
      ],
    },
    names: {
      cols: [],
      cells: [
        ["B4",12,"All Sheets Name"],
        ["B5",-1,1],
      ],
    },
    explore: {
      cols: [8.42578125,10.42578125],
      cells: [
        ["A1",-1,"Ticker"],["B1",-1,"Date/Time"],["C1",-1,"Open"],["D1",-1,"High"],["E1",-1,"Low"],["F1",-1,"Close"],
        ["A2",-1,"NIFTY 50"],["B2",82,36528],["C2",83,1482.15],["D2",83,1592.9],["E2",83,1482.15],["F2",83,1592.2],
      ],
    },
    rules: {
      cols: [6.14,22.86,27.72],
      cells: [
        ["B2",81,"Timeframe:"],["C2",12],
        ["B3",81,"Index:"],["C3",12],
        ["B4",81,"Period:"],["C4",12],
        ["B5",81,"AFL Path:"],["C5",12],
        ["B6",81,"Done By:"],["C6",12],
        ["B7",81,"Verified By:"],["C7",12],
        ["B8",81],["C8",12],
        ["B9",81,"Strategy:"],["C9",12],
        ["B10",81,"Position:"],["C10",12],
        ["B11",81,"Filter:"],["C11",12],
        ["B12",81,"Setup:"],["C12",12],
        ["B13",81,"Entry:"],["C13",12],["D13",50],
        ["B14",81,"Exit:"],["C14",12],["D14",50],
      ],
    },
  },
};
