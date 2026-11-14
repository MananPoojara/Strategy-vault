// YT backtest Results, laid out as the Indicators lab's (branch Nandini: RunResults.tsx, Report.tsx, TsDownload.tsx and the
// Results box of IndicatorsPage.tsx): tabs "Stats & equity drawdown", "Trades" and "YOY-Monthly". The numbers come from the
// engine's YT run (whole-unit account) and its AmiBroker report.
import { useEffect, useRef, useState, type ReactNode } from "react";
import { createLabChart, themeColors, useThemeKey } from "./charts";
import { DASH, int, num, pct } from "./format";
import { dayFirst, ReportPanel, reportSheets } from "./Report";
import { buildTsWorkbook, defaultTsFormat, loadXlsxStyle, tsCosts, tsFileName, type TsFormat, type TsInput } from "./ts-workbook";
import type { LabReport, YtRules, YtRunResult } from "./types";
import { HelpTip, MetricTiles, ResultTabs, Segmented } from "./ui";
import { saveBytes, xlsx, XLSX_TYPE, type CellValue, type Sheet } from "./xlsx";
import "./results.css";
import "./yt.css";

const PAGE = 100;
const ret = (v: number | null | undefined) => (typeof v === "number" && Math.abs(v) > 1e-12 ? pct(v) : "");
const sign = (v: number | null | undefined) => (typeof v === "number" ? (v > 0 ? "n pos" : v < 0 ? "n neg" : "n") : "n");
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const linePoints = (dates: string[], values: (number | null)[]) => dates.map((time, i) => (typeof values[i] === "number" ? { time, value: values[i]! } : { time }));

/** What each tile means, as the engine works it out (yt_report.rs, the lab's report.rs). */
export const STAT_HELP = {
  net: "Gain or loss of the account over the whole test, in % of the starting capital.",
  trades: "Number of trades taken, including one still open on the last day.",
  winners: "Share of trades that closed with a profit.",
  carMdd: "Annual return % divided by max. system % drawdown. Higher is better.",
  dd: "Deepest fall of the account from a peak, in %.",
  car: "Average yearly growth of the account, compounded, from the first to the last bar.",
  equity: "Growth of 1 unit of money: the strategy against simply holding the instrument.",
  underwater: "How far the account is below its last peak on each day: 0% is the peak, losses below zero.",
};
export const statsTiles = (r: LabReport) => [
  { label: "Net Profit %", value: pct(r.system.net_profit, 2), help: STAT_HELP.net },
  { label: "Annual return %", value: pct(r.system.cagr, 2), help: STAT_HELP.car },
  { label: "Max. system % drawdown", value: pct(r.system.max_sys_dd, 2), help: STAT_HELP.dd },
  { label: "CAR/MDD", value: num(r.system.car_mdd), help: STAT_HELP.carMdd },
  { label: "Winners %", value: pct(r.all.winners?.pct, 2), help: STAT_HELP.winners },
  { label: "# Trades", value: int(r.all.trades), help: STAT_HELP.trades },
  { label: "Exposure %", value: pct(r.system.exposure, 2) },
];

export interface TradeTable { header: string[]; rows: string[][]; csv: string[][]; cls: (string | undefined)[][] }
const CENTRED = new Set(["Trade type", "Entry date", "Exit date"]);
const NUMERIC = new Set(["Entry price", "Exit price", "% Change", "Bars held", "Days", "Net profit", "MAE", "MFE", "DD %", "Shares", "Position value", "Allocation", "Costs"]);
/** The lab's tradesheet table: rows in trade order, the running drawdown, and the whole-unit money columns. */
export function tradeTable(result: Pick<YtRunResult, "trades" | "ledger" | "money" | "instrument">): TradeTable {
  const { trades, ledger } = result;
  const money = !!ledger?.length && ledger.length === trades.length;
  const excursions = trades.some((t) => typeof t.mae === "number");
  const header = ["Symbol", "Trade type", "Entry date", "Entry price", "Exit date", "Exit price", "% Change", "Bars held", "Days", "Net profit",
    ...(excursions ? ["MAE", "MFE"] : []), "DD %", ...(money ? ["Shares", "Position value", "Costs"] : [])];
  let growth = 1, pnl = 0, peak = 1;
  const capital = result.money?.capital ?? 1;
  const csv: string[][] = [[...header, "Exit reason"]], rows: string[][] = [], cls: (string | undefined)[][] = [];
  trades.forEach((t, i) => {
    const r = money ? ledger![i] : undefined, before = growth;
    const cum = r ? (pnl += r.pnl) : (growth *= 1 + (t.ret ?? 0)) - 1;
    const equity = r ? 1 + cum / capital : growth, dd = equity / (peak = Math.max(peak, equity)) - 1;
    const net = r ? r.pnl : typeof t.ret === "number" ? before * t.ret : null;
    const days = t.exit && t.entry ? Math.round((Date.parse(t.exit) - Date.parse(t.entry)) / 864e5) : null;
    const cell: Record<string, string> = {
      Symbol: result.instrument?.ticker ?? "", "Trade type": t.dir === -1 ? "Short" : "Long",
      "Entry date": dayFirst(t.entry), "Entry price": num(t.entry_px), "Exit date": t.open ? "Open" : dayFirst(t.exit), "Exit price": num(t.exit_px),
      "% Change": pct(t.ret, 2), "Bars held": int(t.bars), Days: days === null ? DASH : int(days), "Net profit": r ? num(net) : pct(net, 2),
      MAE: pct(t.mae, 2), MFE: pct(t.mfe, 2), "DD %": pct(dd, 1), Shares: int(r?.qty), "Position value": num(r?.notional), Costs: num(r?.costs),
    };
    const row = header.map((h) => cell[h]);
    csv.push([...row, t.open ? "Still open at the end" : t.reason]);
    rows.push(row);
    const tone: Record<string, number | null | undefined> = { "% Change": t.ret, "Net profit": net, "DD %": dd < 0 ? dd : null };
    cls.push(header.map((h) => (h in tone ? sign(tone[h]) : NUMERIC.has(h) ? "n" : CENTRED.has(h) ? "c" : undefined)));
  });
  return { header, rows, csv, cls };
}

export function drawdown(equity: number[]): number[] {
  let peak = 1;
  return equity.map((v) => { peak = Math.max(peak, v); return v / peak - 1; });
}

/** Seven headline tiles, then Portfolio Equity (log scale, against buy & hold) and Underwater Equity. */
function StatsCharts({ result }: { result: YtRunResult }) {
  const equityRef = useRef<HTMLDivElement>(null), ddRef = useRef<HTMLDivElement>(null);
  const equityValueRef = useRef<HTMLOutputElement>(null), ddValueRef = useRef<HTMLOutputElement>(null);
  const ddValues = drawdown(result.equity);
  const ddDrawn = ddValues.length > 1 && ddValues.length === result.dates.length && ddValues.every(Number.isFinite);
  const worst = ddValues.reduce((k, v, i) => (v < ddValues[k] ? i : k), 0);
  const themeKey = useThemeKey();
  useEffect(() => {
    if (!equityRef.current || !ddRef.current) return;
    const el = equityRef.current, ddEl = ddRef.current;
    const day = (t: unknown) => typeof t === "string" ? dayFirst(t) : t && typeof t === "object" && "year" in t && "month" in t && "day" in t
      ? `${String((t as { day: number }).day).padStart(2, "0")}-${String((t as { month: number }).month).padStart(2, "0")}-${(t as { year: number }).year}` : "";
    const eq = createLabChart(el, 300, { timeScale: { minBarSpacing: 0.01, borderColor: themeColors(el).steel }, rightPriceScale: { mode: 1, borderColor: themeColors(el).steel }, localization: { timeFormatter: day } });
    let offCross: (() => void) | undefined;
    if (eq) {
      const { chart, colors } = eq;
      chart.addAreaSeries({ lineColor: colors.chart, topColor: colors.chart + "33", bottomColor: "transparent" }).setData(linePoints(result.dates, result.equity));
      chart.addLineSeries({ color: colors.bh, lineWidth: 1 }).setData(linePoints(result.dates, result.bh));
      chart.timeScale().fitContent();
      const byDate = new Map(result.dates.map((date, i) => [date, i]));
      const onCross = ({ time }: { time?: string }) => {
        const i = typeof time === "string" ? byDate.get(time) : undefined;
        if (equityValueRef.current) equityValueRef.current.textContent = i === undefined ? `Latest ${num(result.equity.at(-1))}×` : `${dayFirst(result.dates[i])} · ${num(result.equity[i])}×`;
      };
      chart.subscribeCrosshairMove?.(onCross);
      offCross = () => chart.unsubscribeCrosshairMove?.(onCross);
    }
    const values = drawdown(result.equity);
    let dd: ReturnType<typeof createLabChart> = null, offDd: (() => void) | undefined;
    ddEl.classList.remove("lab-chart-empty");
    if (!values.length || values.length !== result.dates.length || values.some((value) => !Number.isFinite(value))) {
      ddEl.classList.add("lab-chart-empty"); ddEl.textContent = "No drawdown data for this period.";
    } else if (values.length === 1) {
      ddEl.classList.add("lab-chart-empty"); ddEl.textContent = `Drawdown ${pct(values[0], 2)} · ${dayFirst(result.dates[0])}`;
    } else {
      dd = createLabChart(ddEl, 300, { timeScale: { minBarSpacing: 0.01, borderColor: themeColors(ddEl).steel }, rightPriceScale: { borderColor: themeColors(ddEl).steel, scaleMargins: { top: 0.05, bottom: 0.1 } },
        localization: { timeFormatter: day, priceFormatter: (v: number) => pct(v, 1) } });
      if (dd) {
        const { chart, colors } = dd;
        const area = chart.addAreaSeries({ lineColor: colors.red, topColor: colors.red + "22", bottomColor: colors.red + "66", invertFilledArea: true, lineWidth: 1, priceLineVisible: false });
        area.setData(linePoints(result.dates, values));
        const low = values.reduce((k, v, i) => (v < values[k] ? i : k), 0);
        area.setMarkers?.([{ time: result.dates[low], position: "belowBar", shape: "circle", color: colors.red }]);
        chart.timeScale().fitContent();
        const at = new Map(result.dates.map((date, i) => [date, i]));
        const onDd = ({ time }: { time?: string }) => {
          const i = typeof time === "string" ? at.get(time) : undefined;
          if (ddValueRef.current) ddValueRef.current.textContent = i === undefined ? `Latest ${pct(values.at(-1))}` : `${dayFirst(result.dates[i])} · ${pct(values[i], 2)}`;
        };
        chart.subscribeCrosshairMove?.(onDd);
        offDd = () => chart.unsubscribeCrosshairMove?.(onDd);
      }
    }
    return () => { offCross?.(); offDd?.(); eq?.dispose(); dd?.dispose(); ddEl.replaceChildren(); };
  }, [result, themeKey]);

  return (
    <section className="box lab-result" data-lab-result>
      <h2 className="box-h">Backtest <small>{dayFirst(result.from)} → {dayFirst(result.to)} · {result.instrument.name ?? result.instrument.ticker}</small></h2>
      {result.report && <MetricTiles items={statsTiles(result.report)} />}
      <div className="lab-result-charts">
        <figure className="lab-result-chart" data-lab-chart="equity">
          <figcaption className="lab-chart-h"><span className="lab-chart-title"><h3>Portfolio Equity</h3><HelpTip name="Portfolio Equity" text={STAT_HELP.equity} /></span><output ref={equityValueRef}>Latest {num(result.equity.at(-1))}×</output></figcaption>
          <div className="legend lab-legend lab-chart-sub">
            <span><i className="sw-line" style={{ background: "var(--chart)" }} />Strategy</span>
            <span><i className="sw-line" style={{ background: "var(--bh)" }} />Buy &amp; hold</span>
            <span className="cap lab-scale-note">log scale</span>
          </div>
          <div className="lwc" data-lab-equity aria-label="Portfolio Equity chart" ref={equityRef} />
        </figure>
        <figure className="lab-result-chart" data-lab-chart="drawdown">
          <figcaption className="lab-chart-h"><span className="lab-chart-title"><h3>Underwater Equity</h3><HelpTip name="Underwater Equity" text={STAT_HELP.underwater} /></span><output ref={ddValueRef}>Latest {pct(ddValues.at(-1))}</output></figcaption>
          <p className="cap lab-chart-sub" data-lab-drawdown-worst>{ddDrawn && `Worst ${pct(ddValues[worst], 2)} · ${dayFirst(result.dates[worst])}`}</p>
          <div className="lwc bkc" data-lab-drawdown aria-label="Underwater Equity chart" ref={ddRef} />
        </figure>
      </div>
    </section>
  );
}

/** The YOY-Monthly table as workbook rows: returns in % (2 decimals), blanks where the strategy had no return. */
export function yoyRows(m: { years: string[]; months: Record<string, number | null | undefined>; averages?: (number | null)[] }, pivot: ReturnType<typeof monthPivot>) {
  const p = (v: number | null | undefined) => (typeof v === "number" && Number.isFinite(v) && Math.abs(v) > 1e-12 ? Math.round(v * 1e4) / 100 : null);
  const avg = (vs: (number | undefined)[]) => { const n = vs.filter((v): v is number => typeof v === "number"); return n.length ? n.reduce((a, b) => a + b, 0) / n.length : null; };
  return [["Year", ...MONTHS, "Total"].map(String),
    ...m.years.map((y) => [y, ...MONTHS.map((_, i) => p(m.months[`${y}-${String(i + 1).padStart(2, "0")}`])), p(pivot.rows[y])]),
    ["Total", ...pivot.cols.map(p), p(pivot.all)],
    ...(m.averages?.length ? [["Avg", ...m.averages.map(p), p(avg(m.years.map((y) => pivot.rows[y])))]] : [])];
}
/** YOY-Monthly pivot totals, Excel-style sums: each year across its months, each month across the years, and all of them. */
export function monthPivot(years: string[], months: Record<string, number | null | undefined>) {
  const add = (vs: (number | null | undefined)[]) => { const n = vs.filter((v): v is number => typeof v === "number" && Number.isFinite(v)); return n.length ? n.reduce((a, b) => a + b, 0) : undefined; };
  const cell = (y: string, i: number) => months[`${y}-${String(i + 1).padStart(2, "0")}`];
  return { rows: Object.fromEntries(years.map((y) => [y, add(MONTHS.map((_, i) => cell(y, i)))])) as Record<string, number | undefined>,
    cols: MONTHS.map((_, i) => add(years.map((y) => cell(y, i)))), all: add(years.flatMap((y) => MONTHS.map((_, i) => cell(y, i)))) };
}

function YoyMonthly({ result }: { result: YtRunResult }) {
  const report = window.VaultReport;
  const monthly = report && result.equity.length ? report.monthly({ dates: result.dates, equity: result.equity.map((v) => v / result.equity[0]) }) : null;
  if (!monthly) return <p className="empty">No monthly returns for this run.</p>;
  const pivot = monthPivot(monthly.years, monthly.months);
  const totals = monthly.years.map((y) => pivot.rows[y]).filter((v): v is number => typeof v === "number");
  const avgTotal = totals.length ? totals.reduce((a, b) => a + b, 0) / totals.length : null;
  return (
    <section className="box lab-result-tables" data-lab-result-tables>
      <div className="tblwrap lab-monthly">
        <table className="tbl">
          <caption>YOY-Monthly <button type="button" className="dbtn sm" data-lab-yoy-download onClick={() => saveBytes(xlsx([{ name: "YOY-Monthly", rows: yoyRows(monthly, pivot) }]),
            `${result.instrument.ticker} YOY-Monthly.xlsx`, XLSX_TYPE)}>Download Excel</button></caption>
          <thead><tr><th>Year</th>{MONTHS.map((m) => <th className="n" key={m}>{m}</th>)}<th className="n" title="Sum of the year's monthly returns">Total</th></tr></thead>
          <tbody>{monthly.years.map((y) => (
            <tr key={y}><td>{y}</td>
              {MONTHS.map((_, i) => { const v = monthly.months[`${y}-${String(i + 1).padStart(2, "0")}`]; return <td className={sign(v)} key={i}>{ret(v)}</td>; })}
              <td className={sign(pivot.rows[y])}>{ret(pivot.rows[y])}</td></tr>
          ))}</tbody>
          <tfoot>
            <tr data-lab-pivot-total><td>Total</td>{pivot.cols.map((v, i) => <td className={sign(v)} key={i}>{ret(v)}</td>)}<td className={sign(pivot.all)}>{ret(pivot.all)}</td></tr>
            {!!monthly.averages?.length && <tr><td>Avg</td>{MONTHS.map((_, i) => <td className={sign(monthly.averages[i])} key={i}>{pct(monthly.averages[i])}</td>)}
              <td className={sign(avgTotal)}>{pct(avgTotal)}</td></tr>}
          </tfoot>
        </table>
      </div>
    </section>
  );
}

/** "Download backtest results": one workbook with the run's Settings (rules and every setting), the AmiBroker report
 *  (Statistics), the trade list (Trades) and the YOY-Monthly returns. */
export function backtestWorkbook(result: YtRunResult, rules: YtRules): Sheet[] {
  const list = (label: string, items: string[]): CellValue[][] => (items.length ? items : [""]).map((v, i) => [null, i ? null : label, v]);
  const settings: Sheet = { name: "Settings", layout: "form", rows: [[],
    [null, "Strategy:", rules.strategy], [null, "Index:", result.instrument.name || result.instrument.ticker],
    [null, "Period:", `${dayFirst(result.from)} to ${dayFirst(result.to)}`], [null, "Position:", rules.position],
    ...(result.money ? [[null, "Initial equity:", result.money.capital] as CellValue[]] : []),
    [], [null, "Filter:", rules.filter], [null, "Setup:", rules.setup], ...list("Entry:", rules.entry), ...list("Exit:", rules.exit),
    [], ...rules.settings.map(([k, v], i): CellValue[] => [null, i ? null : "Settings:", `${k}: ${v}`])] };
  const trades = tradeTable(result).csv;
  const stats: Sheet[] = result.report ? reportSheets(result.report, result, trades) : [{ name: "Trades", rows: trades }];
  const monthly = window.VaultReport && result.equity.length ? window.VaultReport.monthly({ dates: result.dates, equity: result.equity.map((v) => v / result.equity[0]) }) : null;
  return [settings, ...stats, ...(monthly ? [{ name: "YOY-Monthly", rows: yoyRows(monthly, monthPivot(monthly.years, monthly.months)) }] : [])];
}
export function saveBacktest(result: YtRunResult, rules: YtRules) {
  const name = `${result.instrument.ticker} ${rules.tab} backtest ${dayFirst(result.from)} to ${dayFirst(result.to)}.xlsx`.replace(/[\\/:*?"<>|]+/g, " ");
  saveBytes(xlsx(backtestWorkbook(result, rules)), name, XLSX_TYPE);
}

export type DailyBar = { date: string; open: number; high: number; low: number; close: number };
const tradesOf = (r: Pick<YtRunResult, "trades">, to: string) => r.trades.filter((t) => typeof t.entry_px === "number" && typeof t.exit_px === "number")
  .map((t) => ({ dir: (t.dir === -1 ? -1 : 1) as 1 | -1, entry: t.entry, exit: t.exit || to, entry_px: t.entry_px, exit_px: t.exit_px, open: t.open }));
/** The Rules sheet text of the tradesheet, from the YT strategy (the lab's tsRules). */
export const tsRules = (rules: YtRules, r: YtRunResult) => {
  const open = r.trades.find((t) => t.open);
  return { timeframe: "Daily", index: r.instrument.ticker, period: `${dayFirst(r.from)} to ${dayFirst(r.to)}`, strategy: rules.tab, position: rules.position,
    filter: rules.filter, setup: rules.setup, entry: rules.entry, exit: [...rules.exit, ...(open ? [`Last trade still open: valued at the close on ${dayFirst(open.exit || r.to)}`] : [])] };
};
export async function saveTsInput(input: TsInput) {
  const XLSX = await loadXlsxStyle();
  const bytes = XLSX.write(buildTsWorkbook(XLSX, input), { bookType: "xlsx", type: "array", cellStyles: true, compression: true });
  saveBytes(new Uint8Array(bytes), tsFileName(input), XLSX_TYPE);
}
/** Download TS for one run (format from the start date, rollover from the page's setting): the optimizer's per-row Excel. */
export async function saveTs(result: YtRunResult, rules: YtRules, loadBars: () => Promise<DailyBar[]>) {
  await saveTsInput({ format: defaultTsFormat(result.from), symbol: result.instrument.ticker, ohlc: await loadBars(), rules: tsRules(rules, result),
    costs: tsCosts(result.rollover), sheetName: rules.tab, trades: tradesOf(result, result.to) });
}

const pct100 = (x: number) => Math.round(x * 1e4) / 100;
/** The research team's tradesheet: From 2000 / From 2008 format, the template's impact cost and the page's rollover, live formulas. */
function TsDownload({ result, rules, loadBars }: { result: YtRunResult; rules: YtRules; loadBars(): Promise<DailyBar[]> }) {
  const [format, setFormat] = useState<TsFormat | null>(null);
  const [costs, setCosts] = useState(() => { const c = tsCosts(result.rollover); return { impact: pct100(c.impact), rollLong: pct100(c.rollLong), rollShort: pct100(c.rollShort) }; });
  const [busy, setBusy] = useState(false), [error, setError] = useState("");
  const fmt = format ?? defaultTsFormat(result.from);
  const put = (k: keyof typeof costs) => (e: React.ChangeEvent<HTMLInputElement>) => { const n = Number(e.target.value); if (e.target.value !== "" && Number.isFinite(n)) setCosts({ ...costs, [k]: n }); };
  const download = async () => {
    setBusy(true); setError("");
    try {
      await saveTsInput({ format: fmt, symbol: result.instrument.ticker, ohlc: await loadBars(), rules: tsRules(rules, result), sheetName: rules.tab, trades: tradesOf(result, result.to),
        costs: { impact: costs.impact / 100, rollLong: costs.rollLong / 100, rollShort: costs.rollShort / 100 } });
    } catch (e) { setError(e instanceof Error ? e.message : String(e)); } finally { setBusy(false); }
  };
  return <div className="lab-ts" data-lab-ts>
    <span className="lab-ts-k">Tradesheet format</span>
    <Segmented label="Tradesheet format" value={fmt} onChange={(v) => setFormat(v)} options={[{ value: "2000", label: "From 2000" }, { value: "2008", label: "From 2008" }]} />
    {format === null && <span className="cap">auto, from the start date</span>}
    <label className="dfield">Impact cost %<input type="number" step={0.01} defaultValue={costs.impact} onChange={put("impact")} /></label>
    <label className="dfield">Rollover Long % / 30 days<input type="number" step={0.01} defaultValue={costs.rollLong} onChange={put("rollLong")} /></label>
    <label className="dfield">Rollover Short % / 30 days<input type="number" step={0.01} defaultValue={costs.rollShort} onChange={put("rollShort")} /></label>
    <button type="button" className="dbtn sm primary" data-lab-download-ts disabled={busy} onClick={() => void download()}>{busy ? "Preparing…" : "Download TS"}</button>
    {error && <span className="neg" role="alert">{error}</span>}
  </div>;
}

function Trades({ result, rules, loadBars }: { result: YtRunResult; rules: YtRules; loadBars(): Promise<DailyBar[]> }) {
  const { trades } = result;
  const [order, setOrder] = useState<"oldest" | "newest">("oldest");
  const [page, setPage] = useState(0);
  useEffect(() => setPage(0), [result]);
  const table = tradeTable(result), pages = Math.max(1, Math.ceil(table.rows.length / PAGE)), at = Math.min(page, pages - 1);
  const shown = (order === "oldest" ? table.rows.map((_, k) => k) : table.rows.map((_, k) => table.rows.length - 1 - k)).slice(at * PAGE, (at + 1) * PAGE);
  return (
    <section className="box lab-result-tables" data-lab-result-tables>
      <section className="lab-trades">
        <div className="lab-row">
          <h3 className="box-h">Tradesheet <small>{trades.length} trades</small></h3>
          {trades.length > 1 && <Segmented label="Trade order" value={order} onChange={(v) => { setOrder(v); setPage(0); }}
            options={[{ value: "oldest", label: "Oldest first" }, { value: "newest", label: "Newest first" }]} />}
        </div>
        {trades.length > 0 && <TsDownload key={result.rollover ?? ""} result={result} rules={rules} loadBars={loadBars} />}
        {trades.length ? <>
          <div className="tlog">
            <table className="tbl tl" data-lab-trades>
              <thead><tr>{table.header.map((h) => <th key={h} className={NUMERIC.has(h) ? "n" : CENTRED.has(h) ? "c" : undefined}>{h}</th>)}</tr></thead>
              <tbody>{shown.map((k) => <tr key={k}>{table.rows[k].map((cell, i) => <td key={i} className={table.cls[k][i]}>{cell}</td>)}</tr>)}</tbody>
            </table>
          </div>
          {pages > 1 && <div className="lab-row lab-pager" data-lab-pager>
            <button type="button" className="dbtn sm" disabled={at === 0} onClick={() => setPage(at - 1)}>‹ Previous</button>
            <span className="cap">{int(at * PAGE + 1)}–{int(Math.min((at + 1) * PAGE, table.rows.length))} of {int(table.rows.length)}</span>
            <button type="button" className="dbtn sm" disabled={at === pages - 1} onClick={() => setPage(at + 1)}>Next ›</button>
          </div>}
        </> : <p className="empty">No trades in this period.</p>}
      </section>
    </section>
  );
}

/** The YT Results box: the lab's three result tabs. `onBack` shows "‹ Back to optimization" (a run applied from the optimizer). */
export function YtResults({ result, rules, loadBars, onBack }: { result: YtRunResult; rules: YtRules; loadBars(): Promise<DailyBar[]>; onBack?: () => void }) {
  const tabs: [string, ReactNode][] = [
    ["Stats & equity drawdown", <><StatsCharts result={result} />
      {result.report && <ReportPanel report={result.report} trades={result.trades} settings={rules.settings} run={result} tradeRows={tradeTable(result).csv} />}</>],
    ["Trades", <Trades result={result} rules={rules} loadBars={loadBars} />],
    ["YOY-Monthly", <YoyMonthly result={result} />],
  ];
  return (
    <div className="lab-page yt-results" data-yt-results>
      {onBack && <div className="lab-step-nav lab-opt-back"><button type="button" className="dbtn" data-lab-back-optimize onClick={onBack}>‹ Back to optimization</button></div>}
      <section className="box lab-results-tabs" data-lab-results-tabs>
        <h2 className="box-h">Results</h2>
        <ResultTabs label="Backtest results" tabs={tabs} />
      </section>
    </div>
  );
}
