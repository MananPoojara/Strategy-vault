// The team's AmiBroker optimization workbook for a YT optimization (the Indicators lab's optimize-sheet.ts, branch Nandini):
// a "Rules" sheet and the optimization result list with the team's derived formula columns.
import { col, type CellValue, type Sheet } from "./xlsx";

export type Dir = "long" | "short" | "both";
/** The engine's YT /optimize response with sheet=1 and table=1. */
export interface YtOptimizeResult {
  from: string; to: string; combos: number; ms: number; threads: number;
  dims: { key: string; values: number[] }[]; cols: string[]; book: (number | null)[];
  rank: { eligible: number; distinct: number; total: number; min_trades: number; per_year: number; top3: { row: number; score: number; pct: number[] }[] };
  picked: Record<string, (number | null)[]>; rows: (number | null)[][];
  sheet_cols?: string[]; sheet?: (number | null)[][]; table_cols?: string[]; table?: (number | null)[][];
  costs?: { fees_bps?: number; slippage_bps?: number };
  /** The combination number (1-based, grid order) of each sheet row, when rows were dropped; else the row's place. */
  sheetNo?: number[];
}
export interface SheetContext {
  dir: Dir; label(key: string): string; group(key: string): string; instrumentLabel: string; strategy: string; aflPath: string; capital: number;
  /** The From,To,Step the user set for each optimized setting (as typed; a dropdown shows its names). */
  range(key: string): string;
  /** Every rule of the strategy, as on its Rules card: all are listed, optimized or not. */
  rules: Record<"filter" | "setup" | "entry" | "exit", string[]>;
}

const POSITIONS = { long: "Long", short: "Short", both: "Long and short" } as const;
const dayFirst = (d: string) => d.split("-").reverse().join("-");

/** The team's derived columns (row-relative; each reads the AmiBroker columns by name). The cost adjustment differs by side:
 *  long as in "Long Daily.xlsx", short as in "Short Daily Optims.xlsx"; long-and-short has no team template, so only the side-free columns. */
const final = (avg: string, bars: string, short: boolean) => short ? `{${avg}}+(((7/5*({${bars}}-1))*0.0166)-0.3)` : `{${avg}}-(((7/5*({${bars}}-1))*0.033)+0.3)`;
const derivedFor = (dir: Dir): [string, string][] => {
  const common: [string, string][] = [
    ["Monthly Thickness", "{Avg % Profit/Loss}/({Avg Bars Held}-1)*21"],
    ["Expectancy Ratio", "(({W. Avg % Profit}/ABS({L. Avg % Loss}))*{% of Winners}-{% of Losers})/100"],
  ];
  if (dir === "both") return [...common, ["Total Profit", "{# Trades}*{Avg % Profit/Loss}"], ["Holding Days", "{# Trades}*({Avg Bars Held}-1)*7/5"]];
  const short = dir === "short";
  return [...common,
    ["Final Avg % Profit/Loss", final("Avg % Profit/Loss", "Avg Bars Held", short)],
    ["Final W Avg", final("W. Avg % Profit", "W. Avg. Bars Held", short)],
    ["Final L Avg", final("L. Avg % Loss", "L. Avg. Bars Held", short)],
    ["Final M. Thk", "({Final Avg % Profit/Loss}/({Avg Bars Held}-1))*21"],
    ["Final Expectancy Ratio", "(({Final W Avg}*({% of Winners}/100))-(ABS({Final L Avg})*({% of Losers}/100)))/(ABS({Final L Avg}))"],
    ["Total Profit", "{# Trades}*{Avg % Profit/Loss}"],
    ["Total Net Profit", "{# Trades}*{Final Avg % Profit/Loss}"],
    ["Holding Days", "{# Trades}*({Avg Bars Held}-1)*7/5"],
  ];
};

/** The page shows the optimized settings first, then only the team's screen columns, in sheet order. */
const SCREEN_SHOWN = new Set(["CAR", "Max. Sys % Drawdown", "CAR/MDD", "# Trades", "Avg % Profit/Loss", "Avg Bars Held", "% of Winners",
  "Expectancy Ratio", "Final Avg % Profit/Loss", "Final M. Thk", "Final Expectancy Ratio", "Total Net Profit"]);
/** The download keeps every column but opens showing only these and the optimized settings (the team's view). */
const EXCEL_SHOWN = new Set([...SCREEN_SHOWN, "W. Avg % Profit", "L. Avg % Loss", "Monthly Thickness", "Final W Avg", "Final L Avg", "Total Profit", "Holding Days"]);
const hiddenAt = (head: string[], settingsAt: number, nd: number, shown: Set<string>) =>
  head.flatMap((h, i) => ((i < settingsAt || i >= settingsAt + nd) && !shown.has(h) ? [i] : []));

/** The team's Dual ATR rule: Initial ATR × must be at most Favour ATR ×. The engine leaves those combinations unranked;
 *  this drops them from the lists, the sheet and the tradesheet downloads too. */
export function withoutAtrOutOfOrder(r: YtOptimizeResult, now: (key: string) => number): YtOptimizeResult & { skipped: number } {
  const [i, f] = [r.dims.findIndex((d) => d.key === "atr_mult"), r.dims.findIndex((d) => d.key === "fav_mult")];
  if (i < 0 && f < 0) return { ...r, skipped: 0 };
  const ok = (row: readonly (number | null)[]) => (i < 0 ? now("atr_mult") : row[i]!) <= (f < 0 ? now("fav_mult") : row[f]!);
  const table = r.table?.filter(ok), kept = (r.sheet ?? []).flatMap((row, k) => (ok(row) ? [k] : []));
  return { ...r, rows: r.rows.filter(ok), table, sheet: kept.map((k) => r.sheet![k]), sheetNo: kept.map((k) => r.sheetNo?.[k] ?? k + 1),
    skipped: r.table && table ? r.table.length - table.length : 0 };
}

export const sheetFileName = (c: SheetContext) =>
  `${c.instrumentLabel.replace(/[\\/:*?"<>|]+/g, " ").replace(/\s+/g, " ").trim()} ${{ long: "Long", short: "Short", both: "Long Short" }[c.dir]} Daily Optims.xlsx`;

/** Evaluates one derived-column template (numbers, + - * /, parentheses, ABS) with the row's values; null when an input is
 *  blank or the result is not finite, as an empty Excel cell would show. Used for the on-page list; the workbook keeps formulas. */
export function evalTemplate(f: string, get: (name: string) => number | null): number | null {
  const src = f.replace(/\{([^}]+)\}/g, (_m, n: string) => { const v = get(n); return v === null ? "NaN" : `(${v})`; });
  let i = 0;
  const peek = () => src[i], eat = (c: string) => { if (src[i] !== c) throw new Error(`expected ${c} in ${f}`); i++; };
  const atom = (): number => {
    if (peek() === "-") { i++; return -atom(); }
    if (src.startsWith("ABS(", i)) { i += 3; return Math.abs(atom()); }
    if (peek() === "(") { i++; const v = sum(); eat(")"); return v; }
    const m = /^(NaN|\d+(\.\d+)?([eE][-+]?\d+)?)/.exec(src.slice(i));
    if (!m) throw new Error(`bad term in ${f}`);
    i += m[0].length; return Number(m[0]);
  };
  const product = (): number => { let v = atom(); while (peek() === "*" || peek() === "/") v = src[i++] === "*" ? v * atom() : v / atom(); return v; };
  const sum = (): number => { let v = product(); while (peek() === "+" || peek() === "-") v = src[i++] === "+" ? v + product() : v - product(); return v; };
  const v = sum();
  return Number.isFinite(v) ? v : null;
}

/** The workbook's Optimization sheet as values for the page: the same header, "No.", order (CAR/MDD, best first) and derived columns;
 *  `shown`: the column indices the team's screen layout keeps. */
export function optimizationList(r: YtOptimizeResult, dir: Dir, label: (key: string) => string): { head: string[]; rows: (number | null)[][]; settingsAt: number; shown: number[] } | null {
  const { sheet, sheet_cols: names } = r;
  if (!sheet || !names) return null;
  const nd = r.dims.length, derived = derivedFor(dir), carMdd = names.indexOf("CAR/MDD");
  const rows = sheet.map((row, i) => {
    const vals = new Map<string, number | null>(names.map((n, j) => [n, row[nd + j] ?? null]));
    const out: (number | null)[] = [r.sheetNo?.[i] ?? i + 1, ...names.map((n) => vals.get(n) ?? null), ...row.slice(0, nd)];
    for (const [n, f] of derived) { const v = evalTemplate(f, (k) => vals.get(k) ?? null); vals.set(n, v); out.push(v); }
    return out;
  });
  const key = (row: (number | null)[]) => (carMdd < 0 ? null : row[1 + carMdd]);
  rows.sort((a, b) => { const x = key(a), y = key(b); return x === y ? (a[0] as number) - (b[0] as number) : x === null ? 1 : y === null ? -1 : y - x; });
  const head = ["No.", ...names, ...r.dims.map((d) => label(d.key)), ...derived.map(([n]) => n)], settingsAt = 1 + names.length;
  const isSetting = (i: number) => i >= settingsAt && i < settingsAt + nd, all = head.map((_, i) => i);
  return { head, rows, settingsAt, shown: [...all.filter(isSetting), ...all.filter((i) => !isSetting(i) && SCREEN_SHOWN.has(head[i]))] };
}

/** Rules + Optimization sheets; null when the response carries no sheet. */
export function optimizationWorkbook(r: YtOptimizeResult, c: SheetContext): Sheet[] | null {
  const { sheet, sheet_cols: names } = r;
  if (!sheet || !names) return null;
  const nd = r.dims.length;
  // each optimized setting on its group's line ("Exit: … · Initial ATR multiplier - 1,4,1")
  const optimized = (re: RegExp) => r.dims.filter((d) => re.test(c.group(d.key))).map((d) => `${c.label(d.key)} - ${c.range(d.key)}`);
  const filters = [...c.rules.filter, ...optimized(/filter/i)], exits = [...c.rules.exit, ...optimized(/exit|stop/i)], entries = [...c.rules.entry, ...optimized(/^entry/i)];
  const setup = [...c.rules.setup, ...optimized(/^(?!.*(filter|exit|stop|entry)).*/i)];
  const list = (label: string, items: string[]): CellValue[][] => (items.length ? items : [""]).map((v, i) => [null, i ? null : label, v]);
  const brokerage = r.costs?.fees_bps ? `${r.costs.fees_bps} bps per side${r.costs.slippage_bps ? `; slippage ${r.costs.slippage_bps} bps per side` : ""}` : 0;
  const rules: Sheet = { name: "Rules", layout: "form", rows: [
    [],
    [null, "Timeframe:", "Daily"], [null, "Index:", c.instrumentLabel],
    [null, "Period:", `${dayFirst(r.from)} to ${dayFirst(r.to)}`],
    [null, "Brokerage:", brokerage], [null, "Open Position:", 1], [null, "%Equity:", 100], [null, "Initial equity:", c.capital],
    [null, "AFL Path:", c.aflPath], [null, "AFL Final check by:"], [null, "Done By:", "Strategy Vault (YT optimization)"], [null, "Verified By:"],
    [],
    [null, "Strategy:", c.strategy], [null, "Position:", POSITIONS[c.dir]],
    ...list("Filter:", filters), ...list("Setup:", setup), ...list("Entry:", entries), ...list("Exit:", exits),
  ] };
  const at = new Map(names.map((n, i) => [n, col(1 + i)]));
  const DERIVED = derivedFor(c.dir);
  DERIVED.forEach(([n], i) => at.set(n, col(1 + names.length + nd + i)));
  const derived = DERIVED.every(([, f]) => [...f.matchAll(/\{([^}]+)\}/g)].every((m) => at.has(m[1]))) ? DERIVED : [];
  const carMdd = names.indexOf("CAR/MDD");
  const key = (row: (number | null)[]) => (carMdd < 0 ? null : row[nd + carMdd]);
  const order = sheet.map((_, i) => i).sort((a, b) => {
    const x = key(sheet[a]), y = key(sheet[b]);
    return x === y ? a - b : x === null ? 1 : y === null ? -1 : y - x;
  });
  const rows: CellValue[][] = order.map((i, k) => {
    const row = sheet[i], n = k + 2;
    return [r.sheetNo?.[i] ?? i + 1, ...names.map((_, j) => row[nd + j] ?? null), ...row.slice(0, nd),
      ...derived.map(([, f]) => ({ f: f.replace(/\{([^}]+)\}/g, (_m, name: string) => `${at.get(name)}${n}`) }))];
  });
  const head = ["No.", ...names, ...r.dims.map((d) => c.label(d.key)), ...derived.map(([n]) => n)];
  const optimization: Sheet = { name: "Optimization", autoFilter: true, hidden: hiddenAt(head, 1 + names.length, nd, EXCEL_SHOWN), rows: [head, ...rows] };
  return [rules, optimization];
}
