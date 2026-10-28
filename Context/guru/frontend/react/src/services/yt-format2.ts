/**
 * YT backtest result workbook, "Format 2": the team's Tradesheet Format (sheets Rules, 2008-2011 Summary, YOY Summary,
 * one trade sheet laid out like NF_En_BFR_0d_EX_Ncandle-9, and All Index Explore). Every computed cell carries the
 * template's Excel formula and the value calculated here, so the file opens with correct numbers and still recalculates
 * when a trade is edited. The 2008 and 2011 blocks start on 1 January of the year of the first trade entered on or after
 * those years (e.g. 1/1/2015 when the first such trade is in 2015); the summary keeps the 2000 / 2008 / 2011 labels.
 */

export type F2Side = "Long" | "Short";
export interface F2Trade { symbol: string; side: F2Side; entry: string; entryPx: number; exit: string; exitPx: number }
export interface F2Bars { ticker: string; d: string[]; o: (number | null)[]; h: (number | null)[]; l: (number | null)[]; c: (number | null)[] }
export type F2Error = "#N/A" | "#DIV/0!";
export interface F2Cell { v?: number | string | null; e?: F2Error; f?: string; array?: boolean; fmt?: "pct" | "date" | "num" | "int"; bold?: boolean }
export interface F2Sheet { name: string; cells: Record<string, F2Cell>; widths?: number[] }
export interface F2Rules { timeframe: string; index: string; aflPath: string; doneBy: string; verifiedBy: string; strategy: string; position: string; filter: string; setup: string; entry: string; exit: string[] }
export interface F2Input { sheetName: string; trades: F2Trade[]; bars: F2Bars; rules: F2Rules; impact?: number; rollLong?: number; rollShort?: number }
/** Several trade sheets in one workbook (e.g. one per exit and index); `index` in rules may list one line per index. */
export interface F2BookInput { runs: { sheetName: string; ticker: string; trades: F2Trade[] }[]; bars: F2Bars[]; rules: Omit<F2Rules, "index"> & { index: string | string[] }; impact?: number; rollLong?: number; rollShort?: number }
interface F2SheetInput { sheetName: string; trades: F2Trade[]; bars: F2Bars; exploreRows: number; lastYear: number; impact: number; rollLong: number; rollShort: number }

type Val = number | F2Error;
const isErr = (v: Val | undefined | null): v is F2Error => typeof v === "string";

/** Excel serial day number of an ISO date (1900 date system). */
export function serial(iso: string): number {
  const [y, m, d] = iso.slice(0, 10).split("-").map(Number);
  return Math.round((Date.UTC(y, m - 1, d) - Date.UTC(1899, 11, 30)) / 86400000);
}
const yearOf = (iso: string) => +iso.slice(0, 4);

/** Column letters for a 0-based index. */
export function col(i: number): string {
  let s = "";
  for (let n = i + 1; n > 0; n = Math.floor((n - 1) / 26)) s = String.fromCharCode(65 + ((n - 1) % 26)) + s;
  return s;
}

const avg = (xs: number[]): Val => (xs.length ? xs.reduce((a, b) => a + b, 0) / xs.length : "#DIV/0!");
const sum = (xs: number[]) => xs.reduce((a, b) => a + b, 0);
const min0 = (xs: number[]) => (xs.length ? Math.min(...xs) : 0); // Excel MIN/MAX of nothing is 0
const max0 = (xs: number[]) => (xs.length ? Math.max(...xs) : 0);
const div = (a: Val, b: Val): Val => (isErr(a) ? a : isErr(b) ? b : b === 0 ? "#DIV/0!" : a / b);
const op = (f: (...x: number[]) => number, ...xs: Val[]): Val => { const e = xs.find(isErr); return e ? (e as F2Error) : f(...(xs as number[])); };

/** Excel-safe sheet name (31 characters, no []:*?/\). */
export function safeSheetName(name: string): string {
  return name.replace(/[[\]:*?/\\]/g, "-").slice(0, 31) || "Trades";
}

/** Team-style short trade sheet name, e.g. NF_En_BFR_3d_EX_NC-7 or BN_En_BFR_0d_EX_ATR5,2_HSL-L. */
export function format2SheetName(ticker: string, strategyId: string, variant: string, p: Record<string, number>): string {
  const code = ({ "NIFTY 50": "NF", NIFTYBANK: "BN", NIFTYMIDCAP100: "MD" } as Record<string, string>)[ticker] || ticker.replace(/[^A-Za-z0-9]/g, "").slice(0, 6);
  if (!strategyId.startsWith("ythol_")) return safeSheetName(`${code}_${variant}`);
  const exits: string[] = [];
  if (p.nc_n > 0) exits.push(`NC-${p.nc_n}`);
  if (p.atr_on) exits.push(`ATR${p.atr_n},${p.atr_mult}`);
  if (p.hsl === 1) exits.push("HSL-L");
  if (p.hsl === 2) exits.push("HSL-C");
  if (p.st_on) exits.push(`STR${p.st_factor}-${p.st_running ? "R" : "F"}`);
  const entry = `${p.after ? "AFT" : "BFR"}_${p.days ?? 0}d`, ex = (xs: string[]) => (xs.length ? `_EX_${xs.join("_")}` : "");
  // Excel allows 31 characters: compact only when needed (no hyphens inside exits, then no "En_")
  const tries = [`${code}_En_${entry}${ex(exits)}`, `${code}_En_${entry}${ex(exits.map(x => x.replace(/-/g, "")))}`, `${code}_${entry}${ex(exits.map(x => x.replace(/-/g, "")))}`];
  return safeSheetName(tries.find(x => x.length <= 31) || tries[2]);
}

interface Row { t: F2Trade; c: number; e: number; g: number; days: number; imp: number; roll: number; net: number; L: number; O: number; Q: number; T: number; mae: number }

function tradeRows(trades: F2Trade[], bars: F2Bars, impact: number, rollLong: number, rollShort: number): Row[] {
  let L = 100, M = 100, Q = 100, R = 100;
  return trades.map(t => {
    const long = t.side === "Long", c = serial(t.entry), e = serial(t.exit);
    const g = long ? t.exitPx / t.entryPx - 1 : -(t.exitPx / t.entryPx - 1), days = e - c;
    const roll = (days / 30) * (long ? rollLong : rollShort), net = g - (impact + roll);
    L *= 1 + g; M = Math.max(L, M); Q *= 1 + net; R = Math.max(Q, R);
    const span: number[] = [];
    bars.d.forEach((d, i) => { if (d > t.entry && d <= t.exit) { const x = long ? bars.l[i] : bars.h[i]; if (x != null) span.push(x); } });
    const mae = long ? min0(span) / t.entryPx - 1 : -(max0(span) / t.entryPx - 1);
    return { t, c, e, g, days, imp: impact, roll, net, L, O: M > L ? (L - M) / M : 0, Q, T: R > Q ? (Q - R) / R : 0, mae };
  });
}

/** The template's stats block over the trades entered on or after `from` (0 = every trade). */
function block(rows: Row[], from: number, netFrom: number) {
  const r = rows.filter(x => x.c >= from), g = r.map(x => x.g), n = r.map(x => x.net), d = r.map(x => x.days);
  const wr = (xs: number[]) => div(xs.filter(x => x > 0).length, xs.length);
  const w = (xs: number[]) => avg(xs.filter(x => x > 0)), l = (xs: number[]) => avg(xs.filter(x => x < 0));
  const exp = (WR: Val, W: Val, LO: Val) => op((a, b, c) => (a * b - (1 - a) * Math.abs(c)) / Math.abs(c), WR, W, LO);
  const expSafe = (WR: Val, W: Val, LO: Val) => (!isErr(LO) && LO === 0 ? "#DIV/0!" : exp(WR, W, LO));
  const avgDays = avg(d);
  return {
    gross: { total: sum(g), avg: avg(g), count: g.length, wr: wr(g), w: w(g), l: l(g), max: max0(g), min: min0(g) },
    net: { total: sum(n), avg: avg(n), count: n.length, wr: wr(n), w: w(n), l: l(n), max: max0(n), min: min0(n) },
    impact: sum(r.map(x => x.imp)), roll: sum(r.map(x => x.roll)), days: sum(d), avgDays, months: div(avgDays, 30.5),
    exp: { g: expSafe(wr(g), w(g), l(g)), n: expSafe(wr(n), w(n), l(n)) },
    thick: { g: op(x => x * 30, div(avg(g), avgDays)), n: op(x => x * 30, div(avg(n), avgDays)) },
    rr: { g: op((a, b) => Math.abs(a / b), w(g), l(g)), n: op((a, b) => Math.abs(a / b), w(n), l(n)) },
    ddGross: min0(r.map(x => x.O)), ddNet: min0(rows.filter(x => x.c >= netFrom).map(x => x.T)),
  };
}

/** One workbook with a single trade sheet (the app's Format 2 download). */
export function buildFormat2(input: F2Input): F2Sheet[] {
  return buildFormat2Book({ runs: [{ sheetName: input.sheetName, ticker: input.bars.ticker, trades: input.trades }], bars: [input.bars], rules: input.rules, impact: input.impact, rollLong: input.rollLong, rollShort: input.rollShort });
}

/** One trade sheet laid out like the template's NF_En_BFR_0d_EX_Ncandle-9. */
function tradeSheet(o: F2SheetInput) {
  const { trades, bars, impact, rollLong, rollShort, lastYear } = o;
  const S = safeSheetName(o.sheetName), ref = (cell: string) => `'${S.replace(/'/g, "''")}'!${cell}`;
  const rows = tradeRows(trades, bars, impact, rollLong, rollShort), n = rows.length, first = 21, last = Math.max(first + n + 200, 1000);
  const rng = (c: string, from = first) => `${c}${from}:${c}${last}`, abs = (c: string, from = first) => `$${c}$${from}:$${c}$${last}`;
  const cells: Record<string, F2Cell> = {};
  const put = (a: string, v: Val | string | null | undefined, extra: Partial<F2Cell> = {}) => {
    cells[a] = isErr(v as Val) && (v === "#N/A" || v === "#DIV/0!") ? { e: v as F2Error, ...extra } : { v: v as number | string | null, ...extra };
  };
  const pct = { fmt: "pct" as const }, date = { fmt: "date" as const }, bold = { bold: true };

  // block start dates: 1 January of the year of the first trade entered on or after 2008 / 2011
  const startYear = (y: number) => { const t = rows.find(x => yearOf(x.t.entry) >= y); return t ? yearOf(t.t.entry) : y; };
  const y08 = startYear(2008), y11 = startYear(2011), h1 = serial(`${y08}-01-01`), o1 = serial(`${y11}-01-01`), maxExit = max0(rows.map(x => x.e));
  const firstIn = (y: number): Val => rows.find(x => yearOf(x.t.entry) === y)?.c ?? "#N/A";
  const j1 = firstIn(y08), q1 = firstIn(y11);
  const all = block(rows, -Infinity, -Infinity), b08 = block(rows, h1, isErr(j1) ? Infinity : j1), b11 = block(rows, o1, isErr(q1) ? Infinity : q1);

  // row 1: costs and block headers
  put("A1", "Overall", bold); put("C1", impact); put("D1", rollLong); put("E1", rollShort);
  put("H1", h1, date); put("I1", y08, { f: "YEAR(H1)" });
  put("J1", j1, { ...date, f: `INDEX(${rng("C")},MATCH(I1,${rng("V")},0),1)` }); put("L1", maxExit, { ...date, f: `MAX(${rng("E")})` });
  put("O1", o1, date); put("P1", y11, { f: "YEAR(O1)" });
  put("Q1", q1, { ...date, f: `INDEX(${rng("C")},MATCH(P1,${rng("V")},0),1)` }); put("S1", maxExit, { ...date, f: `MAX(${rng("E")})` });
  for (const [g, i, r, nCol, d] of [["B", "C", "D", "E", "F"], ["I", "J", "K", "L", "M"], ["P", "Q", "R", "S", "T"]]) {
    put(`${g}2`, "Gross %", bold); put(`${i}2`, "Impact Cost", bold); put(`${r}2`, "Rollover Cost", bold); put(`${nCol}2`, "Net %", bold); put(`${d}2`, "Days", bold);
  }

  // stats rows 3-18: overall (A:F), from 2008 (H:M), from 2011 (O:T)
  const labels = ["Total Profit %", "Avg %", "Total Trades", "Win Ratio", "Win Avg", "Loss Avg", "Max Winner", "Max Looser", "Expectancy Ratio", "Monthly Thickness", "Risk Reward", "CAGR", "DD", "CAR/MDD", "Max MAE", "Avg MAE"];
  labels.forEach((t, k) => { put(`A${k + 3}`, t, bold); put(`H${k + 3}`, t, bold); put(`O${k + 3}`, t, bold); });
  const cagrAll = (cumCol: "L" | "Q"): Val => {
    if (!n) return "#N/A";
    const at = rows.find(x => x.e === maxExit)!, start = Math.min(...rows.map(x => x.c));
    return Math.pow((cumCol === "L" ? at.L : at.Q) / 100, 1 / ((maxExit - start) / 365)) - 1;
  };
  const cagrFrom = (cumCol: "L" | "Q", jd: Val): Val => {
    if (isErr(jd) || !n) return "#N/A";
    const end = rows.find(x => x.e === maxExit)!, i = rows.findIndex(x => x.c === jd), before = i > 0 ? rows[i - 1] : null;
    const a = cumCol === "L" ? end.L : end.Q, b = before ? (cumCol === "L" ? before.L : before.Q) : 100;
    return Math.pow(a / b, 1 / ((max0(rows.map(x => x.e)) - (jd as number)) / 365)) - 1;
  };
  const maeAll = rows.map(x => x.mae), maeFrom = (s: number) => rows.filter(x => x.e >= s).map(x => x.mae);
  // overall
  put("B3", all.gross.total, { ...pct, f: `SUM(${rng("G")})` }); put("C3", all.impact, { ...pct, f: `SUM(${rng("I")})` }); put("D3", all.roll, { ...pct, f: `SUM(${rng("J")})` });
  put("E3", all.net.total, { ...pct, f: `SUM(${rng("K")})` }); put("F3", all.days, { f: `SUM(${rng("H")})` });
  put("B4", all.gross.avg, { ...pct, f: `AVERAGE(${rng("G")})` }); put("E4", all.net.avg, { ...pct, f: `AVERAGE(${rng("K")})` }); put("F4", all.avgDays, { fmt: "num", f: `AVERAGE(${rng("H")})` });
  put("B5", all.gross.count, { f: `COUNT(${rng("G")})` }); put("E5", all.net.count, { f: `COUNT(${rng("K")})` }); put("F5", all.months, { fmt: "num", f: "F4/30.5" });
  put("B6", all.gross.wr, { ...pct, f: `COUNTIF(${rng("G")},">0")/B5` }); put("E6", all.net.wr, { ...pct, f: `COUNTIF(${rng("K")},">0")/E5` });
  put("B7", all.gross.w, { ...pct, f: `AVERAGEIF(${rng("G")},">0",${rng("G")})` }); put("E7", all.net.w, { ...pct, f: `AVERAGEIF(${rng("K")},">0",${rng("K")})` });
  put("B8", all.gross.l, { ...pct, f: `AVERAGEIF(${rng("G")},"<0",${rng("G")})` }); put("E8", all.net.l, { ...pct, f: `AVERAGEIF(${rng("K")},"<0",${rng("K")})` });
  put("B9", all.gross.max, { ...pct, f: `MAX(${rng("G")})` }); put("E9", all.net.max, { ...pct, f: `MAX(${rng("K")})` });
  put("B10", all.gross.min, { ...pct, f: `MIN(${rng("G")})` }); put("E10", all.net.min, { ...pct, f: `MIN(${rng("K")})` });
  put("B11", all.exp.g, { fmt: "num", f: "((B6*B7)-((1-B6)*ABS(B8)))/(ABS(B8))" }); put("E11", all.exp.n, { fmt: "num", f: "((E6*E7)-((1-E6)*ABS(E8)))/(ABS(E8))" });
  put("B12", all.thick.g, { ...pct, f: "B4/F4*30" }); put("E12", all.thick.n, { ...pct, f: "E4/F4*30" });
  put("B13", all.rr.g, { fmt: "num", f: "ABS(B7/B8)" }); put("E13", all.rr.n, { fmt: "num", f: "ABS(E7/E8)" });
  const cagrG = cagrAll("L"), cagrN = cagrAll("Q");
  put("B14", cagrG, { ...pct, f: `(INDEX(${abs("L", 20)},MATCH(MAX(${abs("E", 20)}),${abs("E", 20)},0),1)/100)^(1/(((MAX(${rng("E", 20)})-MIN(${rng("C", 20)}))/365)))-1` });
  put("E14", cagrN, { ...pct, f: `(INDEX(${abs("Q", 20)},MATCH(MAX(${abs("E", 20)}),${abs("E", 20)},0),1)/100)^(1/(((MAX(${rng("E", 20)})-MIN(${rng("C", 20)}))/365)))-1` });
  put("B15", all.ddGross, { ...pct, f: `MIN(${abs("O", 20)})` }); put("E15", all.ddNet, { ...pct, f: `MIN(${abs("T", 20)})` });
  put("B16", op((a, b) => Math.abs(a / b), cagrG, all.ddGross === 0 ? "#DIV/0!" : all.ddGross), { fmt: "num", f: "ABS(B14/B15)" });
  put("E16", op((a, b) => Math.abs(a / b), cagrN, all.ddNet === 0 ? "#DIV/0!" : all.ddNet), { fmt: "num", f: "ABS(E14/E15)" });
  put("E17", min0(maeAll), { ...pct, f: `MIN(${abs("W")})` }); put("E18", avg(maeAll), { ...pct, f: `AVERAGE(${abs("W")})` });
  // from-year blocks (array formulas, as in the template)
  const fromBlock = (s: string, cols: [string, string, string, string, string], b: ReturnType<typeof block>, jRef: string, jd: Val, start: number) => {
    const [g, i, r, nc, d] = cols, cond = `${s}$1<=${rng("C")}`, A = { array: true };
    put(`${g}3`, b.gross.total, { ...pct, ...A, f: `SUM(IF(${cond},${rng("G")},""))` }); put(`${i}3`, b.impact, { ...pct, ...A, f: `SUM(IF(${cond},${rng("I")},""))` });
    put(`${r}3`, b.roll, { ...pct, ...A, f: `SUM(IF(${cond},${rng("J")},""))` }); put(`${nc}3`, b.net.total, { ...pct, ...A, f: `SUM(IF(${cond},${rng("K")},""))` });
    put(`${d}3`, b.days, { ...A, f: `SUM(IF(${cond},${rng("H")},""))` });
    put(`${g}4`, b.gross.avg, { ...pct, ...A, f: `AVERAGE(IF(${cond},${rng("G")},""))` }); put(`${nc}4`, b.net.avg, { ...pct, ...A, f: `AVERAGE(IF(${cond},${rng("K")},""))` });
    put(`${d}4`, b.avgDays, { fmt: "num", ...A, f: `AVERAGE(IF(${cond},${rng("H")},""))` });
    put(`${g}5`, b.gross.count, { ...A, f: `COUNT(IF(${cond},${rng("G")},""))` }); put(`${nc}5`, b.net.count, { ...A, f: `COUNT(IF(${cond},${rng("K")},""))` });
    put(`${d}5`, b.months, { fmt: "num", f: `${d}4/30.5` });
    const both = (x: string, sign: string) => `(${s}$1<=${rng("C")})*(0${sign}${rng(x)})`;
    put(`${g}6`, b.gross.wr, { ...pct, ...A, f: `COUNT(IF(${both("G", "<")},${rng("G")},""))/${g}5` }); put(`${nc}6`, b.net.wr, { ...pct, ...A, f: `COUNT(IF(${both("K", "<")},${rng("K")},""))/${nc}5` });
    put(`${g}7`, b.gross.w, { ...pct, ...A, f: `AVERAGE(IF(${both("G", "<")},${rng("G")},""))` }); put(`${nc}7`, b.net.w, { ...pct, ...A, f: `AVERAGE(IF(${both("K", "<")},${rng("K")},""))` });
    put(`${g}8`, b.gross.l, { ...pct, ...A, f: `AVERAGE(IF(${both("G", ">")},${rng("G")},""))` }); put(`${nc}8`, b.net.l, { ...pct, ...A, f: `AVERAGE(IF(${both("K", ">")},${rng("K")},""))` });
    put(`${g}9`, b.gross.max, { ...pct, ...A, f: `MAX(IF(${cond},${rng("G")},""))` }); put(`${nc}9`, b.net.max, { ...pct, ...A, f: `MAX(IF(${cond},${rng("K")},""))` });
    put(`${g}10`, b.gross.min, { ...pct, ...A, f: `MIN(IF(${cond},${rng("G")},""))` }); put(`${nc}10`, b.net.min, { ...pct, ...A, f: `MIN(IF(${cond},${rng("K")},""))` });
    put(`${g}11`, b.exp.g, { fmt: "num", f: `((${g}6*${g}7)-((1-${g}6)*ABS(${g}8)))/(ABS(${g}8))` }); put(`${nc}11`, b.exp.n, { fmt: "num", f: `((${nc}6*${nc}7)-((1-${nc}6)*ABS(${nc}8)))/(ABS(${nc}8))` });
    put(`${g}12`, b.thick.g, { ...pct, f: `${g}4/${d}4*30` }); put(`${nc}12`, b.thick.n, { ...pct, f: `${nc}4/${d}4*30` });
    put(`${g}13`, b.rr.g, { fmt: "num", f: `ABS(${g}7/${g}8)` }); put(`${nc}13`, b.rr.n, { fmt: "num", f: `ABS(${nc}7/${nc}8)` });
    const endRef = s === "H" ? "L1" : "S1", cg = cagrFrom("L", jd), cn = cagrFrom("Q", jd);
    put(`${g}14`, cg, { ...pct, f: `(INDEX(${rng("L", 20)},MATCH(${endRef},${rng("E", 20)},0),1)/INDEX(${rng("L", 20)},MATCH(${jRef},${rng("C", 20)},0)-1,1))^(1/(((MAX(${rng("E", 20)})-${jRef})/365)))-1` });
    put(`${nc}14`, cn, { ...pct, f: `(INDEX(${rng("Q", 20)},MATCH(${endRef},${rng("E", 20)},0),1)/INDEX(${rng("Q", 20)},MATCH(${jRef},${rng("C", 20)},0)-1,1))^(1/(((MAX(${rng("E", 20)})-${jRef})/365)))-1` });
    put(`${g}15`, b.ddGross, { ...pct, ...A, f: `MIN(IF(${cond},${rng("O")},""))` });
    put(`${nc}15`, isErr(jd) ? "#N/A" : b.ddNet, { ...pct, ...A, f: `MIN(IF(${jRef.replace(/\d+$/, "")}$1<=${rng("C")},${rng("T")},""))` });
    put(`${g}16`, op((a, x) => Math.abs(a / x), cg, b.ddGross === 0 ? "#DIV/0!" : b.ddGross), { fmt: "num", f: `ABS(${g}14/${g}15)` });
    put(`${nc}16`, op((a, x) => Math.abs(a / x), cn, isErr(jd) ? "#N/A" : b.ddNet === 0 ? "#DIV/0!" : b.ddNet), { fmt: "num", f: `ABS(${nc}14/${nc}15)` });
    const mae = maeFrom(start);
    put(`${nc}17`, min0(mae), { ...pct, ...A, f: `MIN(IF(${s}1<=${abs("E")},${abs("W")},""))` }); put(`${nc}18`, avg(mae), { ...pct, ...A, f: `AVERAGE(IF(${s}1<=${abs("E")},${abs("W")},""))` });
    put(`${d}18`, "Gross", bold);
  };
  fromBlock("H", ["I", "J", "K", "L", "M"], b08, "J1", j1, h1);
  fromBlock("O", ["P", "Q", "R", "S", "T"], b11, "Q1", q1, o1);

  // period table (template's fixed periods)
  const periods: [string, string, string][] = [[` 2008 - ${lastYear}`, "2007-12-31", `${lastYear}-12-31`], [" 2008 - 2010", "2007-12-31", "2010-12-31"], [" 2011 - 2019", "2010-12-31", "2019-12-31"], [` 2011 - ${lastYear}`, "2010-12-31", `${lastYear}-12-31`]];
  ["Dates", "Start Date", "End Date", "Trades", "Total Gross Profit", "Average Gross Profit", "Gross Month. Thk.", "Total Net Profit", "Average Net Profit", "Net Month. Thk.", "Average Days Held", "Months Held"]
    .forEach((t, k) => put(`${col(22 + k)}3`, t, bold));
  periods.forEach(([label, a, z], k) => {
    const r = 4 + k, x = serial(a), y = serial(z), sel = rows.filter(t => x <= t.c && y >= t.e), cond = `(X${r}<=${abs("C")})*(Y${r}>=${abs("E")})`, A = { array: true };
    const ag = avg(sel.map(t => t.g)), an = avg(sel.map(t => t.net)), ad = avg(sel.map(t => t.days)), mh = div(ad, 30.5);
    put(`W${r}`, label); put(`X${r}`, x, date); put(`Y${r}`, y, date);
    put(`Z${r}`, sel.length, { ...A, f: `COUNT(IF(${cond},${abs("G")},""))` }); put(`AA${r}`, sum(sel.map(t => t.g)), { ...pct, ...A, f: `SUM(IF(${cond},${abs("G")},""))` });
    put(`AB${r}`, ag, { ...pct, ...A, f: `AVERAGE(IF(${cond},${abs("G")},""))` }); put(`AC${r}`, div(ag, mh), { ...pct, f: `AB${r}/AH${r}` });
    put(`AD${r}`, sum(sel.map(t => t.net)), { ...pct, ...A, f: `SUM(IF(${cond},${abs("K")},""))` }); put(`AE${r}`, an, { ...pct, ...A, f: `AVERAGE(IF(${cond},${abs("K")},""))` });
    put(`AF${r}`, div(an, mh), { ...pct, f: `AE${r}/AH${r}` }); put(`AG${r}`, ad, { fmt: "num", ...A, f: `AVERAGE(IF(${cond},${abs("H")},""))` }); put(`AH${r}`, mh, { fmt: "num", f: `AG${r}/30.5` });
  });

  // trade table
  ["L", "Q"].forEach(c => put(`${c}19`, "CUMM", bold)); ["M", "R"].forEach(c => put(`${c}19`, "Peak", bold)); ["N", "S"].forEach(c => put(`${c}19`, "DD", bold));
  ["O", "T"].forEach(c => put(`${c}19`, "%DD", bold)); put("U19", "Year by exit Date", bold); put("V19", "Year by entry Date", bold);
  ["Symbol", "Trade", "En. date", "En. Price", "Ex. date", "Ex. Price", "% chg", "Days", "Impact cost", "Rollover Cost", "Net Profit"].forEach((t, k) => put(`${col(k)}20`, t, bold));
  put("L20", 100); put("M20", 100, { f: "MAX(L20,M19)" }); put("N20", "", { f: 'IF(M20>L20,L20-M20,"")' }); put("Q20", 100); put("R20", 100, { f: "MAX(Q20,R19)" }); put("S20", "", { f: 'IF(R20>Q20,Q20-R20,"")' }); put("W20", "MAE", bold); put("Z20", "Year", bold); put("AA20", "Net Return", bold);
  const ix = o.exploreRows + 1, ia = (c: string) => `'All Index Explore'!$${c}$2:$${c}$${ix}`;
  rows.forEach((x, k) => {
    const r = first + k, p = r - 1;
    put(`A${r}`, x.t.symbol); put(`B${r}`, x.t.side); put(`C${r}`, x.c, date); put(`D${r}`, x.t.entryPx, { fmt: "num" }); put(`E${r}`, x.e, date); put(`F${r}`, x.t.exitPx, { fmt: "num" });
    put(`G${r}`, x.g, { ...pct, f: `IF(B${r}="Long",(F${r}/D${r}-1),-(F${r}/D${r}-1))` }); put(`H${r}`, x.days, { f: `E${r}-C${r}` }); put(`I${r}`, x.imp, { ...pct, f: "C$1" });
    put(`J${r}`, x.roll, { ...pct, f: `IF(B${r}="Long",H${r}/30*D$1,H${r}/30*E$1)` }); put(`K${r}`, x.net, { ...pct, f: `G${r}-SUM(I${r},J${r})` });
    const prevL = k ? rows[k - 1].L : 100, prevQ = k ? rows[k - 1].Q : 100, peakL = Math.max(...rows.slice(0, k + 1).map(y => y.L), 100), peakQ = Math.max(...rows.slice(0, k + 1).map(y => y.Q), 100);
    put(`L${r}`, prevL * (1 + x.g), { fmt: "num", f: `L${p}*(1+G${r})` }); put(`M${r}`, peakL, { fmt: "num", f: `MAX(L${r},M${p})` });
    put(`N${r}`, peakL > x.L ? x.L - peakL : "", { fmt: "num", f: `IF(M${r}>L${r},L${r}-M${r},"")` }); put(`O${r}`, x.O, { ...pct, f: `IFERROR(N${r}/M${r},0)` });
    put(`Q${r}`, prevQ * (1 + x.net), { fmt: "num", f: `Q${p}*(1+K${r})` }); put(`R${r}`, peakQ, { fmt: "num", f: `MAX(Q${r},R${p})` });
    put(`S${r}`, peakQ > x.Q ? x.Q - peakQ : "", { fmt: "num", f: `IF(R${r}>Q${r},Q${r}-R${r},"")` }); put(`T${r}`, x.T, { ...pct, f: `IFERROR(S${r}/R${r},0)` });
    put(`U${r}`, yearOf(x.t.exit), { f: `YEAR(E${r})` }); put(`V${r}`, yearOf(x.t.entry), { f: `YEAR(C${r})` });
    put(`W${r}`, x.mae, { ...pct, array: true, f: `IF(B${r}="Long",MIN(IF((A${r}=${ia("A")})*(C${r}<${ia("B")})*(E${r}>=${ia("B")}),${ia("E")},""))/D${r}-1,-(MAX(IF((A${r}=${ia("A")})*(C${r}<${ia("B")})*(E${r}>=${ia("B")}),${ia("D")},""))/D${r}-1))` });
  });
  // year table (net return by exit year, from 2008) and its total
  let yr = first, yearTotal = 0;
  for (let y = 2008; y <= lastYear; y++, yr++) {
    const s = sum(rows.filter(x => yearOf(x.t.exit) === y).map(x => x.net));
    yearTotal += s;
    put(`Z${yr}`, y); put(`AA${yr}`, s === 0 ? "" : s, { ...pct, f: `IFERROR(IF(SUMIF(${abs("U")},Z${yr},${abs("K")})=0,NA(),SUMIF(${abs("U")},Z${yr},${abs("K")})),"")` });
  }
  put(`Z${yr}`, "Total Net Return", bold); put(`AA${yr}`, yearTotal, { ...pct, f: `SUM($AA$${first}:$AA$${yr - 1})` });
  const trade: F2Sheet = { name: S, cells, widths: [16, 7, 11, 11, 11, 11, 10, 7, 11, 13, 11, 10, 10, 9, 9, 10, 10, 10, 9, 9, 10, 10, 10, 6, 11, 11, 8, 12, 12, 12, 12, 12, 12, 12] };

  return { sheet: trade, cells, S, ref, first };
}

/** The workbook: Rules, 2008-2011 Summary (3 rows per trade sheet), YOY Summary (a column per trade sheet), the trade
 * sheets and All Index Explore (every index's daily OHLC, which the MAE formulas read). */
export function buildFormat2Book(input: F2BookInput): F2Sheet[] {
  const { rules } = input, impact = input.impact ?? 0.003, rollLong = input.rollLong ?? 0.01, rollShort = input.rollShort ?? -0.005;
  const exploreRows = input.bars.reduce((a, b) => a + b.d.length, 0);
  const lastYear = Math.max(2026, ...input.bars.map(b => (b.d.length ? yearOf(b.d[b.d.length - 1]) : 2026)), ...input.runs.flatMap(r => r.trades.map(t => yearOf(t.exit))));
  const empty: F2Bars = { ticker: "", d: [], o: [], h: [], l: [], c: [] };
  const used = new Set<string>();
  const sheets = input.runs.map(run => {
    let name = safeSheetName(run.sheetName);
    for (let k = 2; used.has(name); k++) name = safeSheetName(`${run.sheetName.slice(0, 27)}_${k}`); // unique sheet names
    used.add(name);
    return tradeSheet({ sheetName: name, trades: run.trades, bars: input.bars.find(b => b.ticker === run.ticker) || empty, exploreRows, lastYear, impact, rollLong, rollShort });
  });

  // 2008-2011 Summary: three rows per trade sheet; the 2000 / 2008 / 2011 labels stay as in the template
  const sum08: Record<string, F2Cell> = {}, bold = { bold: true };
  ["Sr. No.", "All Sheets Name", "From", "Total Net Profit %", "Net Avg %", "Avarage Days", "Total Trades", "Win Ratio", "Win Avg", "Loss Avg", "Max Winner", "Max Looser", "Expectancy Ratio", "Monthly Thickness", "Risk Reward", "CAGR", "DD", "CAR/MDD", "Max MAE", "Avg MAE"]
    .forEach((t, k) => { sum08[`${col(1 + k)}3`] = { v: t, bold: true }; });
  const map: [string, string, string][] = [["E", "E3", "pct"], ["F", "E4", "pct"], ["G", "F4", "num"], ["H", "E5", "int"], ["I", "E6", "pct"], ["J", "E7", "pct"], ["K", "E8", "pct"], ["L", "E9", "pct"], ["M", "E10", "pct"], ["N", "E11", "num"], ["O", "E12", "pct"], ["P", "E13", "num"], ["Q", "E14", "pct"], ["R", "E15", "pct"], ["S", "E16", "num"], ["T", "E17", "pct"], ["U", "E18", "pct"]];
  const blockCell = (src: string, k: number) => k === 0 ? src : src === "F4" ? (k === 1 ? "M4" : "T4") : `${k === 1 ? "L" : "S"}${src.slice(1)}`;
  const copy = (cell: F2Cell | undefined, f: string, fmt: string): F2Cell => ({ ...(cell?.e ? { e: cell.e } : { v: cell?.v ?? null }), f, fmt: fmt as F2Cell["fmt"] });
  sheets.forEach((t, i) => [2000, 2008, 2011].forEach((label, k) => {
    const r = 4 + i * 3 + k;
    if (k === 0) sum08[`B${r}`] = { v: i + 1 };
    sum08[`C${r}`] = { v: t.S }; sum08[`D${r}`] = { v: label };
    map.forEach(([c, src, fmt]) => { const at = blockCell(src, k); sum08[`${c}${r}`] = copy(t.cells[at], t.ref(at), fmt); });
  }));
  const summary: F2Sheet = { name: "2008-2011 Summary", cells: sum08, widths: [3, 7, 32, 7, 12, 10, 10, 9, 9, 9, 9, 10, 10, 12, 12, 10, 9, 9, 9, 9, 9] };

  // YOY Summary (net, by exit year, from 2008): one column per trade sheet
  const yoy: Record<string, F2Cell> = {};
  yoy.B1 = { v: "Note: The YOY Returns are on net basis and from exit date (From 2008)", bold: true };
  yoy.B5 = { v: "Year", bold: true };
  sheets.forEach((t, i) => {
    const c = col(2 + i);
    yoy[`${c}2`] = { v: i + 1 }; yoy[`${c}3`] = { v: "All Sheets Name", bold: true }; yoy[`${c}4`] = { v: t.S, bold: true }; yoy[`${c}5`] = { v: "Net Return", bold: true };
    let r = 6;
    for (let y = 2008, src = t.first; y <= lastYear; y++, r++, src++) { yoy[`B${r}`] = { v: y }; yoy[`${c}${r}`] = { v: t.cells[`AA${src}`]?.v ?? "", f: t.ref(`AA${src}`), fmt: "pct" }; }
    for (const [label, src, fmt] of [["Total Net Return", "L3", "pct"], ["Net Avg %", "L4", "pct"], ["Avg. Days", "M4", "num"], ["Total Trades", "L5", "int"], ["Win Ratio", "L6", "pct"], ["Win Avg", "L7", "pct"], ["Expectancy Ratio", "L11", "num"], ["Monthly Thickness", "L12", "pct"], ["Risk Reward", "L13", "num"], ["CAGR", "L14", "pct"], ["DD", "L15", "pct"], ["CAR/MDD", "L16", "num"], ["Max MAE", "L17", "pct"], ["Avg MAE", "L18", "pct"]] as const) {
      yoy[`B${r}`] = { v: label, bold: true }; yoy[`${c}${r}`] = copy(t.cells[src], t.ref(src), fmt); r++;
    }
  });
  const yoySheet: F2Sheet = { name: "YOY Summary", cells: yoy, widths: [3, 20, ...sheets.map(() => 30)] };

  // Rules
  const rc: Record<string, F2Cell> = {};
  const rr = (row: number, label: string, value: string) => { if (label) rc[`B${row}`] = { v: label, bold: true }; rc[`C${row}`] = { v: value }; };
  rr(2, "Timeframe:", rules.timeframe);
  (Array.isArray(rules.index) ? rules.index : [rules.index]).slice(0, 3).forEach((x, k) => rr(3 + k, k ? "" : "Index:", x));
  rr(6, "AFL Path:", rules.aflPath); rr(7, "Done By:", rules.doneBy); rr(8, "Verified By:", rules.verifiedBy);
  rr(10, "Strategy:", rules.strategy); rr(11, "Position:", rules.position); rr(12, "Filter:", rules.filter); rr(13, "Setup:", rules.setup); rr(14, "Entry:", rules.entry);
  rules.exit.forEach((x, k) => rr(15 + k, k ? "" : "Exit:", x));
  const rulesSheet: F2Sheet = { name: "Rules", cells: rc, widths: [3, 14, 90] };

  // All Index Explore: each index's daily OHLC, one after another (the MAE formulas filter by the Index column)
  const ie: Record<string, F2Cell> = {};
  ["Index", "Date", "Open", "High", "Low", "Close"].forEach((t, k) => { ie[`${col(k)}1`] = { v: t, bold: true }; });
  let row = 2;
  for (const bars of input.bars) bars.d.forEach((d, i) => {
    ie[`A${row}`] = { v: bars.ticker }; ie[`B${row}`] = { v: serial(d), fmt: "date" };
    ie[`C${row}`] = { v: bars.o[i] }; ie[`D${row}`] = { v: bars.h[i] }; ie[`E${row}`] = { v: bars.l[i] }; ie[`F${row}`] = { v: bars.c[i] };
    row++;
  });
  const explore: F2Sheet = { name: "All Index Explore", cells: ie, widths: [16, 11, 10, 10, 10, 10] };

  return [rulesSheet, summary, yoySheet, ...sheets.map(t => t.sheet), explore];
}

/** Excel bytes for the sheets, using the SheetJS-compatible library the caller loaded (xlsx-js-style). */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function writeFormat2Xlsx(XLSX: any, sheets: F2Sheet[]): Uint8Array {
  const book = XLSX.utils.book_new(), fmt = { pct: "0.00%", date: "dd-mm-yyyy", num: "0.00", int: "0" }, errs = { "#DIV/0!": 7, "#N/A": 42 };
  for (const sh of sheets) {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const ws: Record<string, any> = {};
    let maxR = 0, maxC = 0;
    for (const [a, c] of Object.entries(sh.cells)) {
      const m = a.match(/^([A-Z]+)(\d+)$/)!, ci = [...m[1]].reduce((n, ch) => n * 26 + ch.charCodeAt(0) - 64, 0) - 1, ri = +m[2] - 1;
      maxR = Math.max(maxR, ri); maxC = Math.max(maxC, ci);
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const cell: Record<string, any> = c.e ? { t: "e", v: errs[c.e], w: c.e } : typeof c.v === "number" ? { t: "n", v: c.v } : { t: "s", v: c.v ?? "" };
      if (c.f) { cell.f = c.f; if (c.array) cell.F = `${a}:${a}`; }
      if (c.fmt) cell.z = fmt[c.fmt];
      if (c.bold || c.fmt) cell.s = { ...(c.bold ? { font: { bold: true } } : {}), ...(c.fmt ? { numFmt: fmt[c.fmt] } : {}) };
      ws[a] = cell;
    }
    ws["!ref"] = XLSX.utils.encode_range({ s: { r: 0, c: 0 }, e: { r: maxR, c: maxC } });
    if (sh.widths) ws["!cols"] = sh.widths.map(w => ({ wch: w }));
    XLSX.utils.book_append_sheet(book, ws, sh.name);
  }
  return new Uint8Array(XLSX.write(book, { bookType: "xlsx", type: "array", cellStyles: true, compression: true }));
}
