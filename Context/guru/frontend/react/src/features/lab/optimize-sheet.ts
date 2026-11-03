// The team's AmiBroker optimization workbook: a "Rules" sheet and the optimization result list with their derived columns.
import { directionOf } from "./execution-model";
import { col, type CellValue, type Sheet } from "./xlsx";
import type { OptimizeResult, Spec } from "./types";
import { dateFilterName } from "./date-sets";
import { getAt } from "./spec-model";

const POSITIONS = { long: "Long", short: "Short", both: "Long and short" } as const;
const dayFirst = (d: string) => d.split("-").reverse().join("-");

/** The team's derived columns (row-relative; each reads the AmiBroker columns by name). The cost adjustment differs by side:
 *  long as in "Long Daily.xlsx", short as in "Short Daily Optims.xlsx"; long-and-short has no team template, so only the side-free columns. */
const final = (avg: string, bars: string, short: boolean) => short ? `{${avg}}+(((7/5*({${bars}}-1))*0.0166)-0.3)` : `{${avg}}-(((7/5*({${bars}}-1))*0.033)+0.3)`;
const derivedFor = (dir: "long" | "short" | "both"): [string, string][] => {
  const common: [string, string][] = [
    ["Monthly Thickness", "{Avg % Profit/Loss}/({Avg Bars Held}-1)*21"],
    ["Expectancy Ratio", "(({W. Avg % Profit}/ABS({L. Avg % Loss}))*{% of Winners}-{% of Losers})/100"],
  ];
  // long and short together: the result has no per-side stats, so no side's cost formula fits; only the side-free columns
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
/** The download keeps every column but opens showing only these and the optimized settings (the team's view); the rest are hidden, not removed.
 *  Matched on the AmiBroker and derived names only, never on an optimized setting's label. */
const EXCEL_SHOWN = new Set([...SCREEN_SHOWN, "W. Avg % Profit", "L. Avg % Loss", "Monthly Thickness", "Final W Avg", "Final L Avg", "Total Profit", "Holding Days"]);
const hiddenAt = (head: string[], settingsAt: number, nd: number, shown: Set<string>) =>
  head.flatMap((h, i) => ((i < settingsAt || i >= settingsAt + nd) && !shown.has(h) ? [i] : []));

/** The team's Dual ATR rule: Initial ATR × must be at most Favour ATR ×. When a multiple is optimized the engine leaves the other
 *  combinations unranked (never #1-#3); this drops them from the lists, the sheet and the tradesheet downloads too. */
export function withoutAtrOutOfOrder<R extends Pick<OptimizeResult, "dims" | "rows" | "table" | "sheet">>(r: R, ranSpec: Spec): R & { skipped: number } {
  const at = (p: string) => r.dims.findIndex((d) => d.path === p);
  const checks = ["", "short_side."].flatMap((pre) => {
    const [i, f] = [at(`${pre}exit.dual_atr.initial.mult`), at(`${pre}exit.dual_atr.favour.mult`)];
    if (i < 0 && f < 0) return [];
    const now = (k: string) => Number(getAt(ranSpec, `${pre}exit.dual_atr.${k}.mult`));
    return [(row: readonly (number | null)[]) => (i < 0 ? now("initial") : row[i]!) <= (f < 0 ? now("favour") : row[f]!)];
  });
  if (!checks.length) return { ...r, skipped: 0 };
  const ok = (row: readonly (number | null)[]) => checks.every((c) => c(row));
  const table = r.table?.filter(ok);
  return { ...r, rows: r.rows.filter(ok), table, sheet: r.sheet?.filter(ok), skipped: r.table && table ? r.table.length - table.length : 0 };
}

export interface SheetContext { ranSpec: Spec; label(path: string): string; group(path: string): string | undefined; instrumentLabel?: string;
  /** The rule a setting belongs to, worded as on its card ("RSI(14) crosses above 30"); its settings share one Rules line. */
  rule?(path: string): { key: string; text: string } | undefined;
  /** Every rule switched on, worded as on its card: all are listed, optimized or not. */
  rules?: Record<"entry" | "exit" | "filter", { key: string; text: string }[]> }

const instrumentName = (r: OptimizeResult, c: SheetContext) => c.instrumentLabel ?? r.instrument.label ?? r.instrument.name ?? r.instrument.ticker;
export const sheetFileName = (r: OptimizeResult, c: SheetContext) =>
  `${`${instrumentName(r, c)} ${dateFilterName(c.ranSpec.date_filter) ?? ""}`.replace(/[\\/:*?"<>|]+/g, " ").replace(/\s+/g, " ").trim()} ${{ long: "Long", short: "Short", both: "Long Short" }[directionOf(c.ranSpec)]} Daily Optims.xlsx`;

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
 *  `shown`: the column indices the team's screen layout keeps (the rest are hidden on the page). */
export function optimizationList(r: OptimizeResult, ranSpec: Spec, label: (path: string) => string): { head: string[]; rows: (number | null)[][]; settingsAt: number; shown: number[] } | null {
  const { sheet, sheet_cols: names } = r;
  if (!sheet || !names) return null;
  const nd = r.dims.length, derived = derivedFor(directionOf(ranSpec)), carMdd = names.indexOf("CAR/MDD");
  const rows = sheet.map((row, i) => {
    const vals = new Map<string, number | null>(names.map((n, j) => [n, row[nd + j] ?? null]));
    const out: (number | null)[] = [i + 1, ...names.map((n) => vals.get(n) ?? null), ...row.slice(0, nd)];
    for (const [n, f] of derived) { const v = evalTemplate(f, (k) => vals.get(k) ?? null); vals.set(n, v); out.push(v); }
    return out;
  });
  const key = (row: (number | null)[]) => (carMdd < 0 ? null : row[1 + carMdd]);
  rows.sort((a, b) => { const x = key(a), y = key(b); return x === y ? (a[0] as number) - (b[0] as number) : x === null ? 1 : y === null ? -1 : y - x; });
  const head = ["No.", ...names, ...r.dims.map((d) => label(d.path)), ...derived.map(([n]) => n)], settingsAt = 1 + names.length;
  const isSetting = (i: number) => i >= settingsAt && i < settingsAt + nd, all = head.map((_, i) => i);
  return { head, rows, settingsAt, shown: [...all.filter(isSetting), ...all.filter((i) => !isSetting(i) && SCREEN_SHOWN.has(head[i]))] };
}

/** Rules + Optimization sheets; null when the response carries no sheet (an engine without sheet=1). */
export function optimizationWorkbook(r: OptimizeResult, c: SheetContext): Sheet[] | null {
  const { sheet, sheet_cols: names } = r;
  if (!sheet || !names) return null;
  const nd = r.dims.length, ex = c.ranSpec.execution ?? {};
  const range = (path: string, values: number[]) => {
    const d = c.ranSpec.optimize?.find((o) => o.path === path);
    return `${c.label(path)} - ${d ? [d.from, d.to, d.step] : [values[0], values.at(-1), values.length > 1 ? values[1] - values[0] : 0]}`;
  };
  // Entry / Exit / Filter rows: one per optimized setting of that group (a group "Filters", "Long exit", "Entry order", …)
  const byGroup = (re: RegExp, seed: { key: string; text: string }[] = []) => {
    const lines = new Map<string, string[]>(seed.map((x) => [x.key, [x.text]]));
    for (const d of r.dims.filter((x) => re.test(c.group(x.path) ?? ""))) {
      const rule = c.rule?.(d.path), key = rule?.key ?? d.path;
      // on its rule's line the setting drops the rule's own name ("Entry 1: ", "Dual ATR · "): the line already says it
      const own = rule ? range(d.path, d.values).replace(/^(Short )?(Entry|Exit rule|Filter) \d+: |^(Short )?Dual ATR · /, "") : range(d.path, d.values);
      lines.set(key, [...(lines.get(key) ?? (rule ? [rule.text] : [])), own]);
    }
    return [...lines.values()].map((parts) => parts.join(" · "));
  };
  const df = c.ranSpec.date_filter, dates = dateFilterName(df);
  const filters = [...(dates ? [`Dates: ${dates} · ${df!.windows.length} periods`] : []), ...byGroup(/filter/i, c.rules?.filter)], exits = byGroup(/exit/i, c.rules?.exit), entries = byGroup(/^(?!.*(filter|exit)).*entry/i, c.rules?.entry);
  const setup = r.dims.filter((d) => !/filter|exit|entry/i.test(c.group(d.path) ?? "")).map((d) => range(d.path, d.values));
  const list = (label: string, items: string[]): CellValue[][] => (items.length ? items : [""]).map((v, i) => [null, i ? null : label, v]);
  const costs = r.costs, fee = "fees_bps" in costs ? null : costs.commission;
  const brokerage = fee ? [fee.per_order && `${fee.per_order} per order`, fee.per_unit && `${fee.per_unit} per share`, fee.pct && `${fee.pct}% of value`].filter(Boolean).join(" + ") || 0
    : "fees_bps" in costs && costs.fees_bps ? `${costs.fees_bps} bps per side` : 0;
  const sizing = ex.sizing ? ("pct_equity" in ex.sizing ? ex.sizing.pct_equity : Object.entries(ex.sizing).map(([k, v]) => `${k.replace(/_/g, " ")} ${v}`).join(" ")) : "";
  const name = typeof c.ranSpec.name === "string" ? c.ranSpec.name : r.origin?.method.name ?? "";
  const rules: Sheet = { name: "Rules", layout: "form", rows: [
    [],
    [null, "Timeframe:", "Daily"], [null, "Index:", instrumentName(r, c)],
    [null, "Period:", `${dayFirst(r.from)} to ${dayFirst(r.to)}${r.split ? ` (ranked on the window before ${dayFirst(r.split)})` : ""}`],
    [null, "Brokerage:", brokerage], [null, "Open Position:", 1], [null, "%Equity:", sizing],
    [null, "AFL Path:", "Strategy Vault · Indicators lab"], [null, "AFL Final check by:"], [null, "Done By:"], [null, "Verified By:"],
    [],
    [null, "Strategy:", name], [null, "Position:", POSITIONS[directionOf(c.ranSpec)]],
    ...list("Filter:", filters), ...list("Setup:", setup), ...list("Entry:", entries), ...list("Exit:", exits),
  ] };

  // Columns: No., the AmiBroker columns, the optimized settings, then the derived formulas.
  const at = new Map(names.map((n, i) => [n, col(1 + i)]));
  const DERIVED = derivedFor(directionOf(c.ranSpec));
  DERIVED.forEach(([n], i) => at.set(n, col(1 + names.length + nd + i)));
  const derived = DERIVED.every(([, f]) => [...f.matchAll(/\{([^}]+)\}/g)].every((m) => at.has(m[1]))) ? DERIVED : [];
  const carMdd = names.indexOf("CAR/MDD");
  const key = (row: (number | null)[]) => (carMdd < 0 ? null : row[nd + carMdd]);
  // AmiBroker's result list sorted on CAR/MDD, best first; blanks last; "No." keeps the combination number
  const order = sheet.map((_, i) => i).sort((a, b) => {
    const x = key(sheet[a]), y = key(sheet[b]);
    return x === y ? a - b : x === null ? 1 : y === null ? -1 : y - x;
  });
  const rows: CellValue[][] = order.map((i, k) => {
    const row = sheet[i], n = k + 2;
    return [i + 1, ...names.map((_, j) => row[nd + j] ?? null), ...row.slice(0, nd),
      ...derived.map(([, f]) => ({ f: f.replace(/\{([^}]+)\}/g, (_m, c: string) => `${at.get(c)}${n}`) }))];
  });
  const head = ["No.", ...names, ...r.dims.map((d) => c.label(d.path)), ...derived.map(([n]) => n)];
  const optimization: Sheet = { name: "Optimization", autoFilter: true, hidden: hiddenAt(head, 1 + names.length, nd, EXCEL_SHOWN), rows: [head, ...rows] };
  return [rules, optimization];
}
