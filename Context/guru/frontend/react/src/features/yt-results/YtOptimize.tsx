// YT optimization results, laid out as the Indicators lab's (branch Nandini: OptimizePanel.tsx's result view, OptimizeSurface.tsx
// and TradesheetDownloads.tsx, without its "Download all tradesheets"): the settings tested line, Now / #1 / #2 / #3 with Stats and Tradesheet, the 3D surface for two
// settings, the tradesheet and optimization-sheet downloads, and the optimization result list sorted on CAR/MDD.
import { useState, type ReactNode } from "react";
import { int, num, pct } from "./format";
import { optimizationList, optimizationWorkbook, sheetFileName, withoutAtrOutOfOrder, type SheetContext, type YtOptimizeResult } from "./optimize-sheet";
import { OptimizeSurface } from "./OptimizeSurface";
import type { YtRules, YtRunResult } from "./types";
import { tone } from "./ui";
import { saveBytes, xlsx, XLSX_TYPE } from "./xlsx";
import { saveTs, type DailyBar } from "./YtResults";
import "./optimize.css";

const PAGE = 100;
const dimFmt = (v: number | null | undefined) => (v === undefined || v === null ? "" : Number.isInteger(v) ? int(v) : num(v, 2));
const TABLE_LABELS: Record<string, string> = { total_return: "Net % Profit", exposure: "Exposure %", cagr: "CAR", max_dd: "Max. Sys % Drawdown", mar: "CAR/MDD",
  trades: "# Trades", win_rate: "% of Winners", profit_factor: "Profit Factor", avg_trade: "Avg. % Profit/Loss", sharpe: "Sharpe Ratio" };
const TABLE_PCT = new Set(["total_return", "exposure", "cagr", "max_dd", "win_rate", "avg_trade"]);
const tableShow = (k: string, v: number | null) => (TABLE_PCT.has(k) ? pct(v, 2) : k === "trades" ? int(v) : num(v));

export interface YtOptimizeContext extends SheetContext {
  ticker: string;
  /** The current value of every setting ("Now"). */
  now(key: string): number;
  /** A value as the settings panel shows it (a dropdown's name). */
  show(key: string, v: number): string;
  /** Runs one combination as a plain backtest (the Tradesheet buttons). */
  runCombo(values: Record<string, number>): Promise<YtRunResult>;
  /** The tradesheet's Rules text for a combination. */
  tsRules(values: Record<string, number>): YtRules;
  loadBars(): Promise<DailyBar[]>;
  /** Stats: put the combination into the settings and show its backtest. */
  onApply(values: Record<string, number>): void;
}

const comboOf = (keys: string[], values: (number | null)[]) => Object.fromEntries(keys.map((k, i) => [k, values[i] as number]));
function TradesheetButton({ ctx, keys, values }: { ctx: YtOptimizeContext; keys: string[]; values: (number | null)[] }) {
  const [busy, setBusy] = useState(false), [error, setError] = useState<string | null>(null);
  const combo = comboOf(keys, values), name = keys.map((k) => `${ctx.label(k)} ${ctx.show(k, combo[k])}`).join(" · ");
  const go = async () => {
    setBusy(true); setError(null);
    try { await saveTs(await ctx.runCombo(combo), ctx.tsRules(combo), ctx.loadBars); }
    catch (e) { setError(e instanceof Error ? e.message : String(e)); } finally { setBusy(false); }
  };
  return <><button type="button" className="dbtn sm ghost" aria-label={`Tradesheet: ${name}`} title={`Download the tradesheet of ${name} as an Excel file`} disabled={busy} onClick={() => void go()}>{busy ? "…" : "Tradesheet"}</button>
    {error && <span className="rnote" role="alert">{error}</span>}</>;
}

type SheetListData = NonNullable<ReturnType<typeof optimizationList>>;
const COUNT_COLS = new Set(["No.", "# Trades", "# of winners", "# of losers"]);
const sheetCell = ({ head, settingsAt: s0 }: SheetListData, nd: number, show: (i: number, v: number) => string) => {
  const plain = (k: number) => COUNT_COLS.has(head[k]) || (k >= s0 && k < s0 + nd);
  return { cls: (k: number, v: number | null) => (plain(k) ? "n" : tone(v)),
    show: (k: number, v: number | null) => (v === null ? "" : COUNT_COLS.has(head[k]) ? int(v) : k >= s0 && k < s0 + nd ? show(k - s0, v) : num(v)) };
};

/** Now / #1 / #2 / #3 with the sheet's screen columns. */
function TopSheetRows({ list, nd, rows, action, showSetting }: { list: SheetListData; nd: number; rows: { name: string; settings: (number | null)[]; best: boolean }[];
  action(name: string): ReactNode; showSetting(i: number, v: number): string }) {
  const { head, shown, settingsAt: s0 } = list, { cls, show } = sheetCell(list, nd, showSetting);
  return (
    <div className="tblwrap">
      <table className="tbl ocmp" data-lab-optimize-top>
        <thead><tr><th></th>{shown.map((k) => <th className="n" key={k}>{head[k]}</th>)}<th></th></tr></thead>
        <tbody>{rows.map((r) => {
          const hit = list.rows.find((lr) => r.settings.every((v, i) => lr[s0 + i] === v));
          return <tr key={r.name} className={r.best ? undefined : "yt-now"}><td>{r.name}</td>
            {shown.map((k) => { const v = k >= s0 && k < s0 + nd ? r.settings[k - s0] ?? null : hit?.[k] ?? null; return <td key={k} className={cls(k, v)}>{show(k, v)}</td>; })}
            <td>{action(r.name)}</td></tr>;
        })}</tbody>
      </table>
    </div>
  );
}

function SheetList({ list, nd, excel, showSetting }: { list: SheetListData; nd: number; excel?(values: (number | null)[]): ReactNode; showSetting(i: number, v: number): string }) {
  const [sort, setSort] = useState<{ col: number; dir: 1 | -1 } | null>(null);
  const [all, setAll] = useState(false);
  const { head, rows, settingsAt: s0, shown: cols } = list, { cls, show } = sheetCell(list, nd, showSetting);
  const order = sort === null ? rows : [...rows].sort((a, b) => {
    const x = a[sort.col], y = b[sort.col];
    return x === y ? 0 : x === null ? 1 : y === null ? -1 : (x - y) * sort.dir;
  });
  const shown = all ? order : order.slice(0, PAGE);
  const by = (i: number) => setSort((s) => (s?.col === i ? { col: i, dir: s.dir === 1 ? -1 : 1 } : { col: i, dir: i === 0 ? 1 : -1 }));
  return (
    <div className="lab-opt-all" data-lab-optimize-sheet-list>
      <h4 className="lab-opt-sheet-h">Optimization result list <small>{rows.length.toLocaleString("en-US")} combinations</small></h4>
      <div className="tblwrap">
        <table className="tbl ocmp lab-opt-all-table">
          <thead><tr>{cols.map((i) => (
            <th key={i} className={`n${sort?.col === i ? " on" : ""}`} aria-sort={sort?.col === i ? (sort.dir === 1 ? "ascending" : "descending") : "none"}>
              <button type="button" className="lab-sort" onClick={() => by(i)}>{head[i]}{sort?.col === i ? (sort.dir === 1 ? " ▲" : " ▼") : ""}</button>
            </th>
          ))}{excel && <th>Tradesheet</th>}</tr></thead>
          <tbody>{shown.map((r) => (
            <tr key={r[0]}>{cols.map((k) => <td key={k} className={cls(k, r[k])}>{show(k, r[k])}</td>)}
              {excel && <td>{excel(r.slice(s0, s0 + nd))}</td>}</tr>
          ))}</tbody>
        </table>
      </div>
      {!all && rows.length > PAGE && <button type="button" className="dbtn sm" onClick={() => setAll(true)}>Show all {rows.length.toLocaleString("en-US")}</button>}
    </div>
  );
}

/** The optimization results workbook (Rules + Optimization sheets, the team's layout), out-of-order ATR combinations left out;
 *  false when the response carries no sheet. Downloaded by the button beside Run optimization. */
export function saveOptimization(raw: YtOptimizeResult, ctx: YtOptimizeContext): boolean {
  const book = optimizationWorkbook(withoutAtrOutOfOrder(raw, ctx.now), ctx);
  if (book) saveBytes(xlsx(book), sheetFileName(ctx), XLSX_TYPE);
  return !!book;
}

export function YtOptimize({ result: raw, ctx }: { result: YtOptimizeResult; ctx: YtOptimizeContext }) {
  const result = withoutAtrOutOfOrder(raw, ctx.now), { rank } = result, keys = result.dims.map((d) => d.key), nd = keys.length;
  const list = optimizationList(result, ctx.dir, ctx.label);
  const showSetting = (i: number, v: number) => ctx.show(keys[i], v);
  const rows = [
    { name: "Now", settings: keys.map((k) => ctx.now(k)), best: false },
    ...rank.top3.map((c, i) => ({ name: `#${i + 1}`, settings: (result.picked[String(c.row)] ?? []).slice(0, nd), best: true })),
  ];
  return (
    <div className="lab-page yt-optimize" data-lab-optimize-result>
      <p className="cap">
        {result.combos.toLocaleString("en-US")} settings tested · {rank.eligible.toLocaleString("en-US")} eligible (at least {rank.min_trades} trades) · {rank.distinct.toLocaleString("en-US")} distinct
        {result.skipped > 0 && ` · ${result.skipped.toLocaleString("en-US")} skipped: Initial ATR × above Favour ATR ×`}
      </p>
      {rank.eligible === 0 ? <p className="rnote" data-lab-no-eligible>{result.skipped > 0 && !result.table?.length
        ? "Every combination has Initial ATR × above Favour ATR ×. Keep the Initial ATR × range at or below the Favour ATR × range."
        : `No setting had enough trades to rank (at least ${rank.min_trades}). Nothing is ranked.`}</p>
        : list && <TopSheetRows list={list} nd={nd} rows={rows} showSetting={showSetting} action={(name) => { const r = rows.find((x) => x.name === name)!; return r.best && <>
          <button type="button" className="dbtn sm" aria-label={`Stats ${r.name}`} title="Put this setting into the strategy settings and show its backtest stats" onClick={() => ctx.onApply(comboOf(keys, r.settings))}>Stats</button>
          <TradesheetButton ctx={ctx} keys={keys} values={r.settings} /></>; }} />}
      {nd === 2 && result.table && result.table_cols && <OptimizeSurface dims={result.dims.map((d) => ({ path: d.key, values: d.values }))} table={result.table} cols={result.table_cols}
        metricLabel={(k) => TABLE_LABELS[k] ?? k} show={tableShow} label={ctx.label} />}
      {list && <SheetList list={list} nd={nd} showSetting={showSetting} excel={(vals) => <TradesheetButton ctx={ctx} keys={keys} values={vals} />} />}
    </div>
  );
}
