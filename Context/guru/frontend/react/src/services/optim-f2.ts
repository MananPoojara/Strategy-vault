/**
 * Optimisation F2: the team's AmiBroker optimisation workbook (as in "Nifty 50- Buy on Monday Upclose, N candle & ATR
 * Optim.xlsx"). A Rules sheet, then one sheet per combination of the varied dropdown settings, named from their values;
 * each row is one setting combination with AmiBroker's optimisation statistics, the varied number settings and the
 * team's formula columns (Monthly Thickness, Expectancy Ratio, the "Final" brokerage columns, totals, holding days).
 *
 * The statistics are calculated here from each run's whole-unit trades and closes, to AmiBroker's definitions as
 * checked against 20 AmiBroker optimisation rows of the Kora Reddy "BuyShort at Open or Close" Nifty workbook, which
 * the engine reproduces trade for trade: every column matched at the sheet's rounding except Sharpe Ratio, whose
 * AmiBroker definition was not reproduced, so it is left empty.
 */
import { serial, type F2Cell, type F2Sheet } from "./yt-format2";

/** One trade of a YT run (the engine's run JSON): legs are [bar index, price, weight]. */
export interface RunTrade { dir: number; legs: [number, number, number][]; exit_i: number; exit_px: number; bars: number; open: boolean; units?: number }
export interface RunInput { dates: string[]; close: number[]; equity: number[]; capital: number; costRate: number; trades: RunTrade[] }

export const AMI_COLS = ["No.", "Net Profit", "Net % Profit", "Exposure %", "CAR", "RAR", "Max. Trade Drawdown", "Max. Trade % Drawdown", "Max. Sys Drawdown",
  "Max. Sys % Drawdown", "Recovery Factor", "CAR/MDD", "RAR/MDD", "Profit Factor", "Payoff Ratio", "Standard Error", "RRR", "Ulcer Index", "Ulcer Perf. Index",
  "Sharpe Ratio", "K-Ratio", "# Trades", "Avg Profit/Loss", "Avg % Profit/Loss", "Avg Bars Held", "# of winners", "% of Winners", "W. Tot. Profit", "W. Avg. Profit",
  "W. Avg % Profit", "W. Avg. Bars Held", "# of losers", "% of Losers", "L. Tot. Loss", "L. Avg. Loss", "L. Avg % Loss", "L. Avg. Bars Held"] as const;
export type AmiStats = Record<Exclude<(typeof AMI_COLS)[number], "No.">, number | null>;

const mean = (a: number[]) => a.length ? a.reduce((x, y) => x + y, 0) / a.length : NaN;
const dayNo = (d: string) => Date.UTC(+d.slice(0, 4), +d.slice(5, 7) - 1, +d.slice(8, 10)) / 864e5;
const ratio = (a: number, b: number) => (b === 0 || !Number.isFinite(a / b) ? null : a / b);

/** Money equity at each close: the whole-unit account replayed from the trades (single-leg trades), else the engine's. */
function moneyEquity(r: RunInput): number[] {
  const N = r.dates.length;
  if (r.trades.some(t => t.legs.length !== 1 || t.units === undefined)) return r.equity.map(v => v * r.capital);
  const eq = new Array<number>(N).fill(r.capital);
  let cash = r.capital, next = 0;
  for (const t of r.trades) {
    const [ei, ep] = t.legs[0], u = t.units!;
    for (let k = next; k < ei; k++) eq[k] = cash;
    for (let k = ei; k <= t.exit_i; k++) {
      const px = k === t.exit_i ? t.exit_px : r.close[k];
      eq[k] = cash + u * (t.dir * (px - ep) - ep * r.costRate) - (k === t.exit_i ? u * px * r.costRate : 0);
    }
    cash = eq[t.exit_i]; next = t.exit_i + 1;
  }
  for (let k = next; k < N; k++) eq[k] = cash;
  return eq;
}

/** Least-squares line through y against its bar number. */
function regression(y: number[]) {
  const n = y.length, mx = (n - 1) / 2, my = mean(y);
  let sxy = 0, sxx = 0;
  for (let i = 0; i < n; i++) { sxy += (i - mx) * (y[i] - my); sxx += (i - mx) ** 2; }
  const slope = sxy / sxx, icpt = my - slope * mx;
  let ss = 0; for (let i = 0; i < n; i++) ss += (y[i] - icpt - slope * i) ** 2;
  return { slope, sxx, ss };
}

/** AmiBroker optimisation statistics of one run (money in the account's currency, percentages as numbers). */
export function amiStats(r: RunInput): AmiStats {
  const cap = r.capital, N = r.dates.length, eq = moneyEquity(r);
  // every trade counts, a position still open on the last bar at that bar's close (as AmiBroker reports it)
  const closed = r.trades.map(t => {
    const [ei, ep] = t.legs[0], u = t.units ?? 0;
    const pnl = u * (t.dir * (t.exit_px - ep) - (ep + t.exit_px) * r.costRate);
    // largest fall from the trade's own peak, on closes (AmiBroker's Max. Trade Drawdown), and as % of the entry value
    let peak = 0, dd = 0;
    for (let k = ei; k <= t.exit_i; k++) { const v = u * t.dir * ((k === t.exit_i ? t.exit_px : r.close[k]) - ep); dd = Math.min(dd, v - peak); peak = Math.max(peak, v); }
    return { ei, xi: t.exit_i, u, pnl, pct: pnl / (u * ep) * 100, bars: t.bars + 1, dd, ddPct: dd / (u * ep) * 100 };
  });
  const net = eq[N - 1] - cap, yrs = (dayNo(r.dates[N - 1]) - dayNo(r.dates[0])) / 365;
  const car = yrs > 0 ? ((cap + net) / cap) ** (1 / yrs) * 100 - 100 : 0;
  // exposure: position value / equity on each bar the position is held at the close (or its single bar)
  let held = 0;
  for (const t of closed) for (let k = t.ei; k <= Math.max(t.ei, t.xi - 1); k++) held += t.u * r.close[k] / eq[k];
  const exposure = held / N * 100;
  let peak = cap, sysDd = 0, sysDdPct = 0, ulcer = 0;
  for (const v of eq) { peak = Math.max(peak, v); sysDd = Math.min(sysDd, v - peak); sysDdPct = Math.min(sysDdPct, (v - peak) / peak * 100); ulcer += ((v - peak) / peak * 100) ** 2; }
  const ui = Math.sqrt(ulcer / N), lr = regression(eq), stdErr = Math.sqrt(lr.ss / (N - 2)), slopeErr = Math.sqrt(lr.ss / (N - 2) / lr.sxx);
  const W = closed.filter(t => t.pnl > 0), L = closed.filter(t => t.pnl <= 0), tot = (a: typeof closed) => a.reduce((s, t) => s + t.pnl, 0);
  const wTot = tot(W), lTot = tot(L), rar = ratio(car * 100, exposure);
  const n = closed.length;
  return {
    "Net Profit": net, "Net % Profit": net / cap * 100, "Exposure %": exposure, CAR: car, RAR: rar,
    "Max. Trade Drawdown": Math.min(0, ...closed.map(t => t.dd)), "Max. Trade % Drawdown": Math.min(0, ...closed.map(t => t.ddPct)), "Max. Sys Drawdown": sysDd, "Max. Sys % Drawdown": sysDdPct,
    "Recovery Factor": ratio(net, -sysDd), "CAR/MDD": ratio(car, -sysDdPct), "RAR/MDD": rar === null ? null : ratio(rar, -sysDdPct),
    "Profit Factor": ratio(wTot, -lTot), "Payoff Ratio": W.length && L.length ? ratio(wTot / W.length, -lTot / L.length) : null,
    "Standard Error": stdErr, RRR: ratio(lr.slope * (N / yrs), stdErr), "Ulcer Index": ui, "Ulcer Perf. Index": ratio(car - 5.4, ui),
    "Sharpe Ratio": null, "K-Ratio": ratio(lr.slope, slopeErr * N),
    "# Trades": n, "Avg Profit/Loss": n ? tot(closed) / n : null, "Avg % Profit/Loss": n ? mean(closed.map(t => t.pct)) : null, "Avg Bars Held": n ? mean(closed.map(t => t.bars)) : null,
    "# of winners": W.length, "% of Winners": n ? W.length / n * 100 : null, "W. Tot. Profit": wTot, "W. Avg. Profit": W.length ? wTot / W.length : null,
    "W. Avg % Profit": W.length ? mean(W.map(t => t.pct)) : null, "W. Avg. Bars Held": W.length ? mean(W.map(t => t.bars)) : null,
    "# of losers": L.length, "% of Losers": n ? L.length / n * 100 : null, "L. Tot. Loss": lTot, "L. Avg. Loss": L.length ? lTot / L.length : null,
    "L. Avg % Loss": L.length ? mean(L.map(t => t.pct)) : null, "L. Avg. Bars Held": L.length ? mean(L.map(t => t.bars)) : null,
  };
}

// ---------------------------------------------------------------- workbook

export interface F2Row { no: number; stats: AmiStats; params: number[] }
export interface F2ResultSheet { name: string; paramNames: string[]; rows: F2Row[] }
export interface F2Rules { index: string; from: string; to: string; capital: number; brokerage: string; strategy: string; aflPath: string; settings: [string, string][]; note: string }

const col = (i: number) => { let s = ""; for (let n = i + 1; n > 0; n = Math.floor((n - 1) / 26)) s = String.fromCharCode(65 + ((n - 1) % 26)) + s; return s; };
const r2 = (x: number | null) => (x === null || !Number.isFinite(x) ? null : Math.round(x * 100) / 100);

/** Sheet names: Excel's 31 characters, no []:*?/\, unique. */
export function sheetNames(names: string[]): string[] {
  const used = new Set<string>();
  return names.map(raw => {
    const base = (raw.replace(/[[\]:*?/\\]/g, "-").trim() || "Settings").slice(0, 31);
    let name = base;
    for (let k = 2; used.has(name.toLowerCase()); k++) name = `${base.slice(0, 31 - String(k).length - 1)}~${k}`;
    used.add(name.toLowerCase());
    return name;
  });
}

/** The team's formula columns after the parameters (letters of the fixed AmiBroker columns: V trades, X avg %, Y bars...). */
function formulaCells(r: number, first: number): [string, string][] {
  const L = (k: number) => col(first + k) + r;
  return [
    ["Monthly Thickness", `X${r}/(Y${r}-1)*21`],
    ["Expectancy Ratio", `((AD${r}/ABS(AJ${r}))*AA${r}-AG${r})/100`],
    ["Final Avg % Profit/Loss", `X${r}-(((7/5*(Y${r}-1))*0.033)+0.3)`],
    ["Final W Avg", `AD${r}-(((7/5*(AE${r}-1))*0.033)+0.3)`],
    ["Final L Avg", `AJ${r}-(((7/5*(AK${r}-1))*0.033)+0.3)`],
    ["Final M. Thk", `(${L(2)}/(Y${r}-1))*21`],
    ["Final Expectancy Ratio", `((${L(3)}*(AA${r}/100))-(ABS(${L(4)})*(AG${r}/100)))/(ABS(${L(4)}))`],
    ["Total Profit", `V${r}*X${r}`],
    ["Total Net Profit", `V${r}*${L(2)}`],
    ["Holding Days", `V${r}*Y${r}`],
  ];
}
export const FORMULA_COLS = formulaCells(2, 0).map(([h]) => h);

/** Excel result of a formula column, so the file shows numbers before Excel recalculates. */
function formulaValue(h: string, s: AmiStats): number | null {
  const X = s["Avg % Profit/Loss"], Y = s["Avg Bars Held"], AD = s["W. Avg % Profit"], AJ = s["L. Avg % Loss"], AA = s["% of Winners"], AG = s["% of Losers"], AE = s["W. Avg. Bars Held"], AK = s["L. Avg. Bars Held"], V = s["# Trades"];
  const cost = (b: number | null) => (b === null ? null : (7 / 5 * (b - 1)) * 0.033 + 0.3);
  const fin = (x: number | null, b: number | null) => (x === null || b === null ? null : x - cost(b)!);
  const div = (a: number | null, b: number | null) => (a === null || b === null || b === 0 ? null : a / b);
  const fx = fin(X, Y), fw = fin(AD, AE), fl = fin(AJ, AK);
  switch (h) {
    case "Monthly Thickness": return div(X, Y === null ? null : Y - 1) === null ? null : div(X, Y! - 1)! * 21;
    case "Expectancy Ratio": return AD === null || AJ === null || AA === null || AG === null || AJ === 0 ? null : ((AD / Math.abs(AJ)) * AA - AG) / 100;
    case "Final Avg % Profit/Loss": return fx;
    case "Final W Avg": return fw;
    case "Final L Avg": return fl;
    case "Final M. Thk": return div(fx, Y === null ? null : Y - 1) === null ? null : div(fx, Y! - 1)! * 21;
    case "Final Expectancy Ratio": return fw === null || fl === null || AA === null || AG === null || fl === 0 ? null : ((fw * AA / 100) - Math.abs(fl) * AG / 100) / Math.abs(fl);
    case "Total Profit": return V === null || X === null ? null : V * X;
    case "Total Net Profit": return V === null || fx === null ? null : V * fx;
    case "Holding Days": return V === null || Y === null ? null : V * Y;
  }
  return null;
}

const INT_COLS = new Set(["No.", "# Trades", "# of winners", "# of losers"]);
export function resultSheet(sh: F2ResultSheet): F2Sheet {
  const cells: Record<string, F2Cell> = {}, first = AMI_COLS.length + sh.paramNames.length;
  const heads = [...AMI_COLS, ...sh.paramNames, ...FORMULA_COLS];
  heads.forEach((h, c) => { cells[col(c) + 1] = { v: h, bold: true }; });
  sh.rows.forEach((row, i) => {
    const r = i + 2;
    AMI_COLS.forEach((h, c) => {
      const v = h === "No." ? row.no : row.stats[h];
      cells[col(c) + r] = v === null ? { v: null } : { v: h === "K-Ratio" ? Math.round(v * 1e4) / 1e4 : INT_COLS.has(h) ? v : r2(v) };
    });
    row.params.forEach((v, j) => { cells[col(AMI_COLS.length + j) + r] = { v }; });
    formulaCells(r, first).forEach(([h, f], j) => { const v = formulaValue(h, row.stats); cells[col(first + j) + r] = v === null ? { f, e: "#DIV/0!" } : { f, v }; });
  });
  return { name: sh.name, cells, widths: heads.map(h => Math.max(9, Math.min(24, h.length + 2))) };
}

export function rulesSheet(r: F2Rules): F2Sheet {
  const cells: Record<string, F2Cell> = {};
  const put = (row: number, a: string, b?: string | number | null, c?: string | number | null, fmt?: F2Cell["fmt"]) => {
    cells[`A${row}`] = { v: a, bold: true }; if (b !== undefined) cells[`B${row}`] = { v: b ?? null, fmt: typeof b === "number" ? fmt : undefined };
    if (c !== undefined) cells[`C${row}`] = { v: c ?? null, fmt: typeof c === "number" ? fmt : undefined };
  };
  put(1, "INDEX", r.index);
  put(3, "TIME PERIOD", serial(r.from), serial(r.to), "date");
  put(5, "STARTING CAPITAL", r.capital, undefined, "int");
  put(6, "BROKERAGE", r.brokerage);
  put(7, "%EQUITY", "100% (whole units)");
  put(8, "MAX OPEN POSITION", 1);
  put(10, "AFL PATH", r.aflPath);
  put(12, "STRATEGY", r.strategy);
  let row = 14;
  cells[`A${row}`] = { v: "SETTINGS", bold: true };
  for (const [name, value] of r.settings) { cells[`B${row}`] = { v: name }; cells[`C${row}`] = { v: value }; row++; }
  row++;
  put(row, "DONE BY", "Strategy Vault (Optimisation F2)"); put(row + 1, "VERIFY BY", "");
  put(row + 3, "NOTE", r.note);
  return { name: "Rules", cells, widths: [22, 40, 30] };
}
