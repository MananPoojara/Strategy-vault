import { formulaQuery } from "./formula";
import { dayFirst } from "./DateField";
import { saveBytes, xlsx, XLSX_TYPE } from "./xlsx";
import { useEffect, useMemo, useState } from "react";
import { operandLabel } from "./spec-model";
import { shortName } from "./catalogue";
import type { ExploreResult, ScanResult } from "./client";
import { DASH, int, num } from "./format";
import type { Catalogue, Operand, SavedMarket, SeriesResponse, Spec } from "./types";
import { ErrorNotice, Loading, Segmented } from "./ui";
import type { Chip } from "./Workbench";
import "./explore.css";

/** AmiBroker Analysis window: Scan (signal list) and Explore (per-bar columns) over the strategy's rules. */
export const ANALYSIS_NOTE = "Raw rule signals on the selected timeframe's bars (one row per day, week or month), not filled trades: no position state, edge-only entry, delay, order or stop applies, and a bars-held exit never fires. Range is the Strategy From–To dates (blank = all quotes).";
const PAGE = 100;
type Cell = string | number | null;
type Fmt = (v: Cell) => string;
/** Entry / Exit: a long strategy's buy and sell, a short one's short and cover (one side per bar, so they never clash). */
const SIGNALS = [["entry", "Entry", ["buy", "short"]], ["exit", "Exit", ["sell", "cover"]]] as const;
const TRADE_WORDS: Record<string, string> = { Buy: "Entry", Sell: "Exit", Short: "Entry (short)", Cover: "Exit (short)" };

const cmp = (a: Cell, b: Cell) => a === null ? (b === null ? 0 : 1) : b === null ? -1
  : typeof a === "number" && typeof b === "number" ? a - b : String(a).localeCompare(String(b));
/** RFC 4180 text: a cell with a comma, quote or line break is quoted. */
export const csvOf = (header: string[], rows: Cell[][]) =>
  [header, ...rows].map((r) => r.map((v) => { const s = v === null ? "" : String(v); return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s; }).join(",")).join("\n");

/** The Analysis result list: click a header to sort (again to reverse), 100 rows a page, Copy CSV takes every row in the shown order. */
/** `file`: the Download Excel name. Numbers stay numbers in the workbook; dates are DD-MM-YYYY as on screen. */
function ResultTable({ name, header, rows, fmt, file }: { name: string; header: string[]; rows: Cell[][]; fmt: Fmt[]; file: string }) {
  const [sort, setSort] = useState<{ col: number; dir: 1 | -1 } | null>(null);
  const [page, setPage] = useState(0);
  useEffect(() => { setPage(0); setSort(null); }, [rows]);
  const order = useMemo(() => { const k = rows.map((_, i) => i); return sort ? k.sort((a, b) => cmp(rows[a][sort.col], rows[b][sort.col]) * sort.dir) : k; }, [rows, sort]);
  const pages = Math.max(1, Math.ceil(order.length / PAGE)), at = Math.min(page, pages - 1);
  // every row in the shown order, as the table sorts it
  const download = () => saveBytes(xlsx([{ name: "Explore", rows: [header, ...order.map((i) => rows[i].map((v, c) => (fmt[c] === date && typeof v === "string" ? dayFirst(v) : v)))] }]), file, XLSX_TYPE);
  if (!rows.length) return <p className="empty">No rows.</p>;
  return <>
    <div className="lab-row">
      <span className="cap">{int(rows.length)} rows</span>
      <button type="button" className="dbtn sm" data-lab-explore-download onClick={download}>Download Excel</button>
    </div>
    <div className="tlog lab-explore-table">
      <table className="tbl tl" data-lab-analysis={name}>
        <thead><tr>{header.map((h, c) => <th key={c} className={fmt[c] === text || fmt[c] === date ? undefined : "n"} aria-sort={sort?.col === c ? (sort.dir === 1 ? "ascending" : "descending") : "none"}>
          <button type="button" onClick={() => { setSort((s) => ({ col: c, dir: s?.col === c && s.dir === 1 ? -1 : 1 })); setPage(0); }}>{h}{sort?.col === c ? (sort.dir === 1 ? " ▲" : " ▼") : ""}</button>
        </th>)}</tr></thead>
        <tbody>{order.slice(at * PAGE, (at + 1) * PAGE).map((i) => <tr key={i}>{rows[i].map((v, c) => <td key={c} className={fmt[c] === text || fmt[c] === date ? undefined : "n"}>{fmt[c](v)}</td>)}</tr>)}</tbody>
      </table>
    </div>
    {pages > 1 && <div className="lab-row lab-pager" data-lab-pager>
      <button type="button" className="dbtn sm" disabled={at === 0} onClick={() => setPage(at - 1)}>‹ Previous</button>
      <span className="cap">{int(at * PAGE + 1)}–{int(Math.min((at + 1) * PAGE, order.length))} of {int(order.length)}</span>
      <button type="button" className="dbtn sm" disabled={at === pages - 1} onClick={() => setPage(at + 1)}>Next ›</button>
    </div>}
  </>;
}
const text: Fmt = (v) => v === null ? DASH : String(v);
const price: Fmt = (v) => num(v, 2);
const value: Fmt = (v) => num(v, 2);
/** Dates show DD-MM-YYYY; the cell keeps the ISO date, so sorting stays in time order. */
const date: Fmt = (v) => (typeof v === "string" ? dayFirst(v) : DASH);
const whole: Fmt = (v) => int(v);
const TF_WORD: Record<string, string> = { W: "week", M: "month", Q: "quarter", Y: "year" };

export interface ExplorePanelProps {
  spec: Spec; catalogue: Catalogue; chips: Chip[]; portfolio: boolean; market: SavedMarket;
  /** The instrument explored: the Symbol column and the download's name. */
  ticker?: string;
  /** Selected timeframe (AmiBroker Periodicity): Explore rows and daily-rule indicator columns are on these bars. */
  tf?: string;
  onScan(apply: "current" | "all"): Promise<ScanResult>;
  onExplore(): Promise<ExploreResult>;
  loadSeries(q: { id: string; tf: string; p?: Record<string, number>; vs?: string; formula?: string; name?: string }): Promise<SeriesResponse>;
}
type Table = { header: string[]; rows: Cell[][]; fmt: Fmt[] };
type Loaded<T> = { result: T; table: Table; failed: string[] };

/** Scan rows as AmiBroker lists them: Symbol, Date/Time, Trade, Close. */
export const scanTable = (r: ScanResult): Table => ({ header: ["Symbol", "Date/Time", "Trade", "Close"], fmt: [text, date, text, price],
  rows: r.rows.map((row) => row.map((c, k) => (k === 2 && typeof c === "string" ? TRADE_WORDS[c] ?? c : c))) });

/** The indicators the strategy's rules read (entry, exit, filters, both sides), once each: RSI(14), Supertrend(5, 1) … */
export function ruleIndicators(spec: Spec, cat: Catalogue): Operand[] {
  const seen = new Map<string, Operand>();
  const walk = (v: unknown) => {
    if (Array.isArray(v)) { v.forEach(walk); return; }
    if (!v || typeof v !== "object") return;
    const o = v as Record<string, unknown>;
    if (typeof o.s === "string" && cat.indicators.some((i) => i.id === o.s && i.computed && !cat.price_sources.includes(i.id))) {
      const { offset: _o, ...one } = o; // the value itself, not n bars ago
      seen.set(JSON.stringify([one.s, one.p ?? {}, one.out ?? "", one.tf ?? "D", one.symbol ?? ""]), one as Operand);
    }
    Object.values(o).forEach(walk);
  };
  walk([spec.entry, spec.exit, (spec as { short_side?: unknown }).short_side]);
  return [...seen.values()];
}

/** Explore columns: Date, O/H/L/C/V, one per indicator the rules read, then Entry and Exit as 0/1. */
export function exploreTable(r: ExploreResult, columns: { name: string; at: Map<string, number | null> }[], ticker = ""): Table {
  // Volume always follows OHLC; an index (no volume data) shows blanks
  // Symbol first (which instrument this is), then Date and OHLC, Volume after OHLC
  const header = ["Symbol", "Date", "Open", "High", "Low", "Close", "Volume", ...columns.map((c) => c.name), ...SIGNALS.map(([, h]) => h)];
  const fmt = [text, date, price, price, price, price, whole, ...columns.map(() => value), ...SIGNALS.map(() => whole)];
  const rows = r.dates.map((d, i): Cell[] => [ticker, d, r.open[i], r.high[i], r.low[i], r.close[i], r.volume?.[i] ?? null, ...columns.map((c) => c.at.get(d) ?? null), ...SIGNALS.map(([, , ks]) => (ks.some((k) => r.signals[k]?.[i] === 1) ? 1 : 0))]);
  return { header, rows, fmt };
}

export function ExplorePanel({ spec, catalogue, chips, portfolio, market, onScan, onExplore, loadSeries, ticker, tf = "D" }: ExplorePanelProps) {
  // Explore only: the Scan list is not offered on the Results tab (onScan stays in the props for other callers)
  const [view] = useState<"scan" | "explore">("explore");
  const [apply, setApply] = useState<"current" | "all">("current");
  const [range, setRange] = useState<"all" | "recent">("all");
  const [recent, setRecent] = useState("20");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [scanned, setScanned] = useState<Loaded<ScanResult> | null>(null);
  const [explored, setExplored] = useState<Loaded<ExploreResult> | null>(null);
  const specKey = JSON.stringify(spec);
  useEffect(() => { setScanned(null); setExplored(null); setError(null); }, [specKey, tf]); // a list belongs to the rules and bars that made it
  useEffect(() => { if (!portfolio) setApply("current"); }, [portfolio]);
  const n = Number(recent), badRecent = range === "recent" && !(Number.isInteger(n) && n >= 1);

  const go = async () => {
    setBusy(true); setError(null);
    try {
      if (view === "scan") { const result = await onScan(apply); setScanned({ result, table: scanTable(result), failed: [] }); return; }
      const raw = await onExplore();
      // Explore drops volume when any bar lacks it (NIFTY has none before 2000): fill it from the Volume series the chart uses
      const vol = raw.volume ? null : await loadSeries({ id: "volume", tf }).catch(() => null);
      const at = vol ? new Map(vol.dates.map((d, i) => [d, vol.series[vol.outputs[0]]?.[i] ?? null])) : null;
      const result = at ? { ...raw, volume: raw.dates.map((d) => at.get(d) ?? null) } : raw;
      // the indicators the rules read, with the rule's own settings, line and timeframe (RSI for an RSI rule)
      // every line of each indicator the rules read, loaded once per setting (Ichimoku: tenkan, kijun, span_a, span_b, as AmiBroker's TL, SL and cloud)
      const seen = new Set<string>();
      const shown = ruleIndicators(spec, catalogue).flatMap((o) => {
        const { out: _out, ...s } = o as { s: string; p?: Record<string, number>; out?: string; tf?: string }, ind = catalogue.indicators.find((i) => i.id === s.s), key = JSON.stringify([s.s, s.p, s.tf]);
        if (!ind || seen.has(key)) return [];
        seen.add(key);
        return [{ s, ind, label: operandLabel(s as Operand, catalogue).replace(ind.name, shortName(ind)) }];
      });
      const parts = await Promise.allSettled(shown.map(({ s, ind }) => loadSeries({ id: s.s, tf: !s.tf || s.tf === "D" ? tf : s.tf, p: s.p, ...formulaQuery(ind) })));
      const failed: string[] = [], columns: { name: string; at: Map<string, number | null> }[] = [];
      parts.forEach((p, k) => {
        const { label } = shown[k];
        if (p.status === "rejected") { failed.push(`${label}: ${p.reason instanceof Error ? p.reason.message : String(p.reason)}`); return; }
        const outs = p.value.outputs;
        for (const out of outs) columns.push({ name: outs.length > 1 ? `${label}.${out}` : label, at: new Map(p.value.dates.map((d, i) => [d, p.value.series[out]?.[i] ?? null])) });
      });
      setExplored({ result, table: exploreTable(result, columns, ticker), failed });
    } catch (e) { setError(e); } finally { setBusy(false); }
  };
  const shownExplore = useMemo(() => explored && (range === "recent" && !badRecent ? { ...explored.table, rows: explored.table.rows.slice(-n) } : explored.table), [explored, range, n, badRecent]);
  const result = view === "scan" ? scanned?.result : explored?.result;

  return (
    <section className="box lab-explore" data-lab-explore>
      <div className="lab-explore-body">
        <div className="lab-row">
          {view === "scan" ? <label className="dfield">Apply to
            <select aria-label="Apply to" value={apply} onChange={(e) => setApply(e.target.value as "current" | "all")}>
              <option value="current">Current</option>
              <option value="all" disabled={!portfolio}>All symbols</option>
            </select>
          </label> : <>
            <Segmented<"all" | "recent"> label="Range" value={range} onChange={setRange} options={[{ value: "all", label: "All quotes" }, { value: "recent", label: "Recent bar(s)" }]} />
            {range === "recent" && <label className="dfield">Bars<input type="number" aria-label="Recent bars" min={1} step={1} value={recent} onChange={(e) => setRecent(e.target.value)} /></label>}
          </>}
          <button type="button" className="dbtn primary" data-lab-action={view} disabled={busy || badRecent} onClick={() => void go()}>{view === "scan" ? "Scan" : "Explore"}</button>
        </div>
        {busy ? <Loading text={view === "scan" ? "Scanning…" : "Exploring…"} />
          : error ? <ErrorNotice error={error} market={market} />
          : result && <>
            {view === "scan" && scanned && <>
              <p className="cap">{int(scanned.result.symbols)} symbol{scanned.result.symbols === 1 ? "" : "s"} scanned{scanned.result.from ? ` · ${dayFirst(scanned.result.from)} to ${dayFirst(scanned.result.to!)}` : ""}</p>
              {scanned.result.membership && <p className="cap">{scanned.result.membership.note}</p>}
              {!!scanned.result.refused.length && <p className="cap" data-lab-scan-refused>Not scanned: {scanned.result.refused.map((r) => `${r.ticker} (${r.reason})`).join("; ")}</p>}
              <ResultTable name="scan" {...scanned.table} file="Scan.xlsx" />
            </>}
            {view === "explore" && explored && shownExplore && <>
              {explored.result.tf && explored.result.tf !== "D" && <p className="cap" data-lab-explore-tf>One row per {TF_WORD[explored.result.tf] ?? `${explored.result.tf} bar`}, dated its last trading day (AmiBroker Periodicity {explored.result.tf}).</p>}
              {!!explored.failed.length && <p className="cap" data-lab-explore-failed>Columns not shown: {explored.failed.join("; ")}</p>}
              <ResultTable name="explore" {...shownExplore} file={`${ticker ? `${ticker} ` : ""}Explore${explored.result.from ? ` ${dayFirst(explored.result.from)} to ${dayFirst(explored.result.to ?? explored.result.from)}` : ""}.xlsx`} />
            </>}
          </>}
      </div>
    </section>
  );
}
