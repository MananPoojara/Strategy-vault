import { Fragment, useEffect, useRef, useState, type ReactNode } from "react";
import { COPY } from "./labels";
import { int, num, pct } from "./format";
import { canonicalJson, comboCount, setAt, specTooLong, splitDate, valuesCount, withCombination, withOptimize } from "./spec-model";
import { optimizablePaths, type OptimizableSetting } from "./optimize-paths";
import { useFormulaScope } from "./formula";
import {foreignDescription} from "./foreign-model";
import { portfolioGridProblems } from "./portfolio-model";
import { ExecutionStrip } from "./RunResults";
import { ErrorNotice, isProvisional, Loading, ProvisionalStrip, ServerLabel, tone } from "./ui";
import { InfoLog, WalkForward } from "./WalkForward";
import { OptimizeSurface } from "./OptimizeSurface";
import { TradesheetButton, TradesheetDownloads } from "./TradesheetDownloads";
import type { Catalogue, IndividualOptimizeResult, OptimizeDim, LabOptimizeResult, LabRunResult, Role, SavedMarket, Spec, WalkforwardOptions, WalkforwardResult } from "./types";
import "./optimize.css";

const MAX_VALUES = 1000;
const RANK_LABELS: Record<string, string> = { cagr: "CAGR", sharpe: "Sharpe", monthly_thickness: "Monthly thickness", expectancy_ratio: "Expectancy ratio", mar: "CAR/MDD" };
const SPLIT_STATS: [string, string][] = [["total_return", "Total return"], ["cagr", "CAGR"], ["sharpe", "Sharpe"], ["trades", "Trades"]];
const fmt = (name: string, v: number | null) =>
  /(^|_)(total_return|cagr|monthly_thickness)$/.test(name) ? pct(v) : /(^|_)trades$/.test(name) ? int(v) : num(v);
const dimFmt = (v: number | null | undefined) => (v === undefined || v === null ? "" : Number.isInteger(v) ? int(v) : num(v, 2));
// AmiBroker optimization result-list names for the engine's table columns.
const TABLE_LABELS: Record<string, string> = { total_return: "Net % Profit", exposure: "Exposure %", cagr: "CAR", max_dd: "Max. Sys % Drawdown", mar: "CAR/MDD",
  trades: "# Trades", win_rate: "% of Winners", profit_factor: "Profit Factor", avg_trade: "Avg. % Profit/Loss", sharpe: "Sharpe Ratio" };
const TABLE_PCT = new Set(["total_return", "exposure", "cagr", "max_dd", "win_rate", "avg_trade"]);
const UNSIGNED = new Set(["exposure", "trades", "win_rate", "profit_factor"]);
const PAGE = 100;

/** Value of a metric for a row of `cols` (dims first) or for `base` (cols after the dims). */
export function colValue(result: LabOptimizeResult, values: number[], name: string): number | null {
  const i = result.cols.indexOf(name);
  if (i < 0) return null;
  const j = values.length === result.cols.length ? i : i - result.dims.length;
  return typeof values[j] === "number" ? values[j] : null;
}

export function OptimizePanel(props: {
  spec: Spec; catalogue: Catalogue; market: SavedMarket; role: Role | null; dates: string[];
  onOptimize(spec: Spec, split: string | null): Promise<LabOptimizeResult>;
  onApply(spec: Spec): void; onTried(n: number): void; tried: number; instrumentLabel?: string;
  onWalkforward?(spec: Spec, o: WalkforwardOptions): Promise<WalkforwardResult>;
  /** AmiBroker Individual Optimize (portfolio universe): each symbol optimized alone. */
  onIndividual?(spec: Spec): Promise<IndividualOptimizeResult>;
  /** One backtest of a spec on this instrument: the tradesheet downloads re-run each combination with it. */
  onRun?(spec: Spec): Promise<LabRunResult>; ticker?: string;
}) {
  const { spec, catalogue, market, role, dates, onOptimize, onApply, onTried, tried, instrumentLabel, onWalkforward, onIndividual, onRun, ticker } = props;
  const [individual, setIndividual] = useState(false);
  const each = individual && !!spec.portfolio && !!onIndividual; // portfolio settings do not apply to a symbol run alone
  const { infos } = useFormulaScope();
  const settings = optimizablePaths(spec, catalogue, infos).filter((s) => !each || !s.path.startsWith("portfolio."));
  const [sel, setSel] = useState<Record<string, OptimizeDim>>({});
  const [heldBack, setHeldBack] = useState(0);
  const [running, setRunning] = useState<number | null>(null); // start time
  const [, setTick] = useState(0);
  const [result, setResult] = useState<LabOptimizeResult | IndividualOptimizeResult | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [retryUntil, setRetryUntil] = useState(0); // 429 deadline; the main button waits for it too
  const [walking, setWalking] = useState(false); // a walk-forward holds the shared optimizer slot
  const last = useRef<{ spec: Spec; go(): Promise<LabOptimizeResult | IndividualOptimizeResult> } | null>(null);
  const runId = useRef(0);

  const pathsKey = settings.map((s) => s.path).join("|");
  useEffect(() => setSel((cur) => Object.fromEntries(Object.entries(cur).filter(([p]) => settings.some((s) => s.path === p)))), [pathsKey]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    if (running === null) return;
    const id = setInterval(() => setTick((t) => t + 1), 1000);
    return () => clearInterval(id);
  }, [running]);
  useEffect(() => {
    if (!retryUntil) return;
    const id = setTimeout(() => setRetryUntil(0), Math.max(0, retryUntil - Date.now()));
    return () => clearTimeout(id);
  }, [retryUntil]);
  const contextKey=spec.portfolio?canonicalJson(spec):"single";
  useEffect(()=>{runId.current++;last.current=null;setResult(null);setError(null);setRunning(null);setRetryUntil(0);},[market,contextKey]);
  useEffect(()=>()=>{runId.current++;},[]);

  if (role === null || role === "viewer")
    return <section className="box" data-lab-optimize><h2 className="box-h">Optimization</h2><p className="rnote">{COPY.researcherOnly}</p></section>;

  const dims = settings.filter((s) => sel[s.path]).map((s) => sel[s.path]);
  const combos = comboCount(dims);
  const comboCap=spec.portfolio?catalogue.portfolio?.max_combos??200:catalogue.max_combos;
  const portfolioIssues=portfolioGridProblems(withOptimize(spec,dims),catalogue);
  const tooBig = dims.some((d) => valuesCount(d) > MAX_VALUES) || combos > comboCap || portfolioIssues.length>0;
  const split = !spec.portfolio && heldBack ? splitDate(dates, heldBack) : null;
  const tooLong = dims.length > 0 && specTooLong(withOptimize(spec, dims));
  const disabled = tooLong || running !== null || walking || retryUntil > 0 || combos === 0 || tooBig || (!spec.portfolio && heldBack > 0 && split === null);

  const toggle = (s: OptimizableSetting, on: boolean) => setSel((cur) => {
    const next = { ...cur };
    if (on) next[s.path] = { path: s.path, from: s.current, to: Math.min(s.max, s.current + 4 * s.step), step: s.step };
    else delete next[s.path];
    return next;
  });
  const edit = (path: string, key: "from" | "to" | "step", v: string) => setSel((cur) => ({ ...cur, [path]: { ...cur[path], [key]: Number(v) } }));

  const run = async (args: NonNullable<typeof last.current>) => {
    const id = ++runId.current;
    last.current = args;
    setResult(null);
    setError(null);
    setRetryUntil(0);
    setRunning(Date.now());
    try {
      const r = await args.go();
      if (id !== runId.current) return;
      setResult(r);
      onTried(r.kind === "lab_individual_optimization" ? r.info.combos : r.combos);
    } catch (e) {
      if (id !== runId.current) return;
      setError(e);
      const wait = (e as { retryAfter?: number } | null)?.retryAfter;
      if (wait) setRetryUntil(Date.now() + wait * 1000);
    } finally {
      if (id === runId.current) setRunning(null);
    }
  };

  return (
    <section className="box" data-lab-optimize>
      <h2 className="box-h">Optimization <small>same ranking as the Guru optimizer · tick what to vary</small></h2>
      <div className="tblwrap">
        <table className="tbl lab-dims">
          <thead><tr><th></th><th>Setting</th><th>From</th><th>To</th><th>Step</th><th>Values</th></tr></thead>
          <tbody>
            {[...new Set(settings.map((s) => s.group))].map((group) => <Fragment key={group}>
              <tr className="lab-dim-group"><th colSpan={6}>{group}</th></tr>
              {settings.filter((s) => s.group === group).map((s) => {
              const d = sel[s.path];
              return (
                <tr key={s.path} data-lab-dim={s.path}>
                  <td><input type="checkbox" aria-label={`Vary ${s.label}`} checked={!!d} onChange={(e) => toggle(s, e.target.checked)} /></td>
                  <td>{s.label}</td>
                  {(["from", "to", "step"] as const).map((k) => (
                    <td key={k}><span className="dfield">
                      <input type="number" aria-label={`${s.label} ${k}`} disabled={!d}
                        value={d ? d[k] : k === "step" ? s.step : s.current} onChange={(e) => edit(s.path, k, e.target.value)} />
                    </span></td>
                  ))}
                  <td className="lab-dim-values">{d ? valuesCount(d).toLocaleString("en-US") : <span className="cap">fixed</span>}</td>
                </tr>
              );
              })}
            </Fragment>)}
          </tbody>
        </table>
      </div>
      {tooBig && <p className="rnote">At most {MAX_VALUES.toLocaleString("en-US")} values per setting and {comboCap.toLocaleString("en-US")} combinations. Narrow the ranges.</p>}
      <div className="lab-row lab-optimize-run">
        <span className="cap">{dims.length ? `${combos.toLocaleString("en-US")} combinations` : COPY.tickOne}</span>
        {!spec.portfolio && <label className="dfield">Held back
          <select aria-label="Held back" value={heldBack} onChange={(e) => setHeldBack(Number(e.target.value))}>
            <option value={0}>None</option>
            {[20, 30, 40].map((p) => <option key={p} value={p}>last {p}%</option>)}
          </select>
        </label>}
        {spec.portfolio && onIndividual && <label className="lab-check"><input type="checkbox" checked={individual} onChange={(e) => setIndividual(e.target.checked)} />Individual optimize</label>}
        {spec.portfolio && <span className="cap">{each ? "Full period only · each symbol alone with the whole capital" : "Full period only · shared portfolio cash"}</span>}
        <button type="button" className="dbtn primary" data-lab-action="optimize" disabled={disabled}
          onClick={() => { const s = withOptimize(spec, dims); run({ spec: s, go: () => (each ? onIndividual!(s) : onOptimize(s, split)) }); }}>Run optimization</button>
        <span className="cap">Variants tried this session: {tried.toLocaleString("en-US")}</span>
      </div>
      {each && <p className="cap" data-lab-individual-note>Individual optimize: every universe symbol runs the single-instrument optimizer on its own; portfolio settings (positions, ranking, shared cash) are not used. Symbols × combinations is capped at 12,000 backtests.</p>}
      {portfolioIssues.map(p=><p className="rnote" key={p}>{p}</p>)}
      {tooLong && <p className="rnote">{COPY.tooLong}</p>}
      {!spec.portfolio && heldBack > 0 && <p className="cap">{split ? COPY.splitLine(heldBack, split) : dates.length ? "Not enough history for this split." : "Run a backtest first to set the hold-back date."}</p>}
      {running !== null && <><Loading text={`Optimizing… ${Math.floor((Date.now() - running) / 1000)} s`} /><p className="cap">{COPY.optimizeLimit}</p></>}
      {error !== null && <ErrorNotice error={error} market={market} instrumentLabel={instrumentLabel}
        onRetry={() => last.current && run(last.current)} />}
      {result && (result.kind === "lab_individual_optimization" ? <IndividualResults result={result} label={(p) => settings.find((s) => s.path === p)?.label ?? p} />
        : <LabOptimizeResultView result={result} spec={spec} ranSpec={last.current?.spec ?? spec} catalogue={catalogue} settings={settings} market={market} instrumentLabel={instrumentLabel} onApply={onApply} onRun={onRun} ticker={ticker} />)}
      {!result && !error && running === null && <p className="empty">{spec.portfolio?"Portfolio optimization ranks the full period using shared cash.":COPY.emptyOptimize}</p>}
      {onWalkforward && !spec.portfolio && <WalkForward dates={dates} spec={withOptimize(spec, dims)} blocked={tooLong || combos === 0 || tooBig || running !== null || retryUntil > 0}
        onTried={onTried} onRunning={setWalking}
        market={market} instrumentLabel={instrumentLabel} label={(p) => settings.find((s) => s.path === p)?.label ?? p} onRun={onWalkforward} />}
    </section>
  );
}

function LabOptimizeResultView({ result, spec, ranSpec, catalogue, settings, market, instrumentLabel, onApply, onRun, ticker = "" }: {
  result: LabOptimizeResult; spec: Spec; ranSpec: Spec; catalogue: Catalogue; settings: OptimizableSetting[]; market: SavedMarket; instrumentLabel?: string; onApply(spec: Spec): void;
  onRun?(spec: Spec): Promise<LabRunResult>; ticker?: string;
}) {
  const { rank, dims } = result;
  const stats = rank.stats ?? [];
  const phases = result.split ? (["is", "oos"] as const) : [];
  const apply = (values: number[]) =>
    onApply(withCombination(spec, dims, values));
  const setting = (path: string) => settings.find((s) => s.path === path);
  const ranSettings = optimizablePaths(ranSpec, catalogue, useFormulaScope().infos); // "Now" = the spec that was optimized
  const label = (p: string) => setting(p)?.label ?? ranSettings.find((s) => s.path === p)?.label ?? p;
  // tradesheets: single-instrument runs of the optimized spec (a portfolio has no one tradesheet per combination)
  const sheets = onRun && !ranSpec.portfolio ? { dims, ranSpec, label, onRun, ticker } : null;
  // With a split the backend ranks on the in-sample window and returns those values as is_*.
  const rankCol = (s: string) => (result.split && result.cols.includes(`is_${s}`) ? `is_${s}` : s);
  const rows = [
    { name: "Now", values: result.base, pct: null as number[] | null },
    ...rank.top3.map((c, i) => ({ name: `#${i + 1}`, values: result.picked[String(c.row)] ?? [], pct: c.pct })),
  ];
  return (
    <div data-lab-optimize-result>
      <ServerLabel text={result.label} />
      <ExecutionStrip model={result.execution_model} money={result.money} adaptations={result.adaptations} />
      {result.money && <p className="cap">{result.money.basis}</p>}
      {isProvisional(market) && <ProvisionalStrip note={result.provisional_note} instrumentLabel={("instrument" in result ? result.instrument.label : undefined) ?? instrumentLabel} />}
      {"membership" in result && <p className="rnote">{result.reference.ticker} reference · {result.membership.note}</p>}
      {result.foreign?.map(f=><p className="cap" key={f.ticker}>{foreignDescription(f)}</p>)}
      <p className="cap">Ranked on: {result.ranked_on === "in_sample" ? "in-sample only" : "full period"}</p>
      <p className="cap">
        {result.combos.toLocaleString("en-US")} settings tested · {rank.eligible.toLocaleString("en-US")} eligible (at least {rank.min_trades} trades) · {rank.distinct.toLocaleString("en-US")} distinct
      </p>
      {rank.eligible === 0 ? <p className="rnote">{COPY.noEligible(rank.min_trades)}</p> : (
        <div className="tblwrap">
          <table className="tbl ocmp">
            <thead>
              {result.split && (
                <tr>
                  <th colSpan={1 + dims.length + stats.length}></th>
                  <th colSpan={SPLIT_STATS.length}>Ranking period</th>
                  <th colSpan={SPLIT_STATS.length}>Held back · display only</th>
                  <th></th>
                </tr>
              )}
              <tr>
                <th></th>
                {dims.map((d) => <th className="n" key={d.path}>{setting(d.path)?.label ?? d.path}</th>)}
                {stats.map((s) => <th className="n" key={s}>{RANK_LABELS[s] ?? s}</th>)}
                {phases.map((ph) => SPLIT_STATS.map(([k, l]) => <th className="n" key={`${ph}_${k}`}>{l}</th>))}
                <th></th>
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <tr key={r.name}>
                  <td>{r.name}</td>
                  {dims.map((d, i) => <td className="n" key={d.path}>{dimFmt(r.pct ? r.values[i] : ranSettings.find((s) => s.path === d.path)?.current)}</td>)}
                  {stats.map((s, k) => {
                    const x = colValue(result, r.values, rankCol(s)), v = fmt(s, x);
                    return <td className={tone(x)} key={s}>{r.pct && typeof r.pct[k] === "number" ? `${v} · p${Math.round(r.pct[k])}` : v}</td>;
                  })}
                  {phases.map((ph) => SPLIT_STATS.map(([k]) => { const x = colValue(result, r.values, `${ph}_${k}`); return <td className={k === "trades" ? "n" : tone(x)} key={`${ph}_${k}`}>{fmt(k, x)}</td>; }))}
                  <td>{r.pct && r.values.length === result.cols.length && dims.every((d) => setting(d.path)) &&
                    <button type="button" className="dbtn sm" aria-label={`Apply ${r.name}`} onClick={() => apply(r.values)}>Apply</button>}
                    {sheets && r.pct && r.values.length === result.cols.length && <TradesheetButton {...sheets} values={r.values.slice(0, dims.length)} />}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {result.split && <p className="rnote">{COPY.splitNote}</p>}
      {dims.length === 2 && result.table && result.table_cols && <OptimizeSurface dims={dims} table={result.table} cols={result.table_cols}
        metricLabel={(k) => TABLE_LABELS[k] ?? k} show={tableShow} label={(p) => setting(p)?.label ?? p} />}
      {dims.length === 2 && result.table && result.split && <p className="cap">Chart: in-sample (ranking period) values.</p>}
      {sheets && result.table && <TradesheetDownloads {...sheets} table={result.table} />}
      <AllResults result={result} label={(p) => setting(p)?.label ?? p}
        excel={sheets ? (row) => <TradesheetButton {...sheets} values={row.slice(0, dims.length) as number[]} /> : undefined} />
      <InfoLog info={result.info} />
    </div>
  );
}

const tableShow = (k: string, v: number | null) => (TABLE_PCT.has(k) ? pct(v, 2) : k === "trades" ? int(v) : num(v));
const BEST_COLS = ["total_return", "cagr", "max_dd", "sharpe", "trades"];

/** Individual optimize: each symbol's best setting (the optimizer's #1), its full table on request, notes and the Info log. */
function IndividualResults({ result, label }: { result: IndividualOptimizeResult; label(path: string): string }) {
  const nd = result.dims.length, shown = BEST_COLS.filter((k) => result.cols.includes(k));
  const best = (r: number[], k: string) => r[result.cols.indexOf(k)] ?? null;
  return (
    <div data-lab-optimize-result data-lab-individual>
      <ServerLabel text={result.label} />
      <p className="cap">{result.universe.symbols.toLocaleString("en-US")} symbols · {result.combos.toLocaleString("en-US")} combinations each · best = #1 of the optimizer's ranking</p>
      <div className="tblwrap">
        <table className="tbl ocmp">
          <thead><tr><th>Symbol</th>{result.dims.map((d) => <th className="n" key={d.path}>{label(d.path)}</th>)}{shown.map((k) => <th className="n" key={k}>{TABLE_LABELS[k] ?? RANK_LABELS[k] ?? k}</th>)}<th className="n">Eligible</th></tr></thead>
          <tbody>{result.results.map((r) => (
            <tr key={r.ticker} data-lab-individual-row={r.ticker}>
              <td>{r.ticker}</td>
              {r.skipped ? <td colSpan={nd + shown.length + 1} className="cap">Skipped: {r.skipped}</td> : r.best ? <>
                {result.dims.map((d, i) => <td className="n" key={d.path}>{dimFmt(r.best![i])}</td>)}
                {shown.map((k) => { const v = best(r.best!, k); return <td key={k} className={k === "trades" ? "n" : tone(v)}>{tableShow(k, v)}</td>; })}
                <td className="n">{int(r.eligible)}</td>
              </> : <td colSpan={nd + shown.length + 1} className="cap">No combination with at least {int(r.min_trades)} trades</td>}
            </tr>
          ))}</tbody>
        </table>
      </div>
      {result.results.filter((r) => r.table && result.table_cols).map((r) => <div key={r.ticker} className="lab-individual-table">
        <AllResults result={{ dims: result.dims, table: r.table, table_cols: result.table_cols, split: null }} label={label} title={`${r.ticker} · all results`} total={r.table_rows} />
      </div>)}
      <ul className="cap lab-individual-notes">{result.notes.map((n) => <li key={n}>{n}</li>)}</ul>
      <InfoLog info={result.info} />
    </div>
  );
}

/** AmiBroker's optimization result list: every combination, click a header to sort. Collapsed until asked for. */
function AllResults({ result, label, title = "All results", total, excel }: {
  result: Pick<LabOptimizeResult, "dims" | "table" | "table_cols" | "split"> & { money?: { capital: number } }; label(path: string): string; title?: string;
  /** Rows the server had before capping the list (Individual optimize). */
  total?: number;
  /** A last column per row, e.g. its tradesheet download. */
  excel?(row: (number | null)[]): ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const [all, setAll] = useState(false);
  const [sort, setSort] = useState<{ col: number; dir: 1 | -1 }>({ col: 0, dir: 1 });
  const { table, table_cols: names } = result;
  if (!table || !names) return null;
  const nd = result.dims.length, capital = result.money?.capital, ti = names.indexOf("total_return");
  type Col = { label: string; get(r: (number | null)[], i: number): number | null; show(v: number | null): string; signed: boolean };
  const metric = (k: string, j: number): Col => ({ label: TABLE_LABELS[k] ?? k, get: (r) => r[nd + j], signed: !UNSIGNED.has(k),
    show: (v) => tableShow(k, v) });
  const cols: Col[] = [
    { label: "No.", get: (_, i) => i + 1, show: int, signed: false },
    // AmiBroker's Net Profit is money: ending minus initial capital = capital × Net %.
    ...(capital && ti >= 0 ? [{ label: "Net Profit", get: (r: (number | null)[]) => (typeof r[nd + ti] === "number" ? r[nd + ti]! * capital : null), show: (v: number | null) => num(v), signed: true }] : []),
    ...names.map(metric),
    ...result.dims.map((d, i): Col => ({ label: label(d.path), get: (r) => r[i], show: dimFmt, signed: false })),
  ];
  const c = cols[sort.col] ?? cols[0];
  const order = table.map((_, i) => i).sort((a, b) => {
    const x = c.get(table[a], a), y = c.get(table[b], b);
    return x === y ? a - b : x === null ? 1 : y === null ? -1 : (x - y) * sort.dir; // gaps last either way
  });
  const shown = all ? order : order.slice(0, PAGE);
  const by = (i: number) => setSort((s) => (s.col === i ? { col: i, dir: s.dir === 1 ? -1 : 1 } : { col: i, dir: i === 0 ? 1 : -1 }));
  return (
    <div className="lab-opt-all" data-lab-optimize-all>
      <button type="button" className="dbtn sm" aria-expanded={open} onClick={() => setOpen(!open)}>
        {open ? "▾" : "▸"} {title} · {table.length.toLocaleString("en-US")}{total && total > table.length ? ` of ${total.toLocaleString("en-US")}` : ""} combinations
      </button>
      {open && <>
        {result.split && <p className="cap">In-sample (ranking period) values.</p>}
        <div className="tblwrap">
          <table className="tbl ocmp lab-opt-all-table">
            <thead><tr>{cols.map((col, i) => (
              <th key={col.label + i} className={`n${i === sort.col ? " on" : ""}`} aria-sort={i === sort.col ? (sort.dir === 1 ? "ascending" : "descending") : "none"}>
                <button type="button" className="lab-sort" onClick={() => by(i)}>{col.label}{i === sort.col ? (sort.dir === 1 ? " ▲" : " ▼") : ""}</button>
              </th>
            ))}{excel && <th>Tradesheet</th>}</tr></thead>
            <tbody>{shown.map((i) => (
              <tr key={i}>{cols.map((col, k) => { const v = col.get(table[i], i); return <td key={k} className={col.signed ? tone(v) : "n"}>{col.show(v)}</td>; })}{excel && <td>{excel(table[i])}</td>}</tr>
            ))}</tbody>
          </table>
        </div>
        {!all && table.length > PAGE && <button type="button" className="dbtn sm" onClick={() => setAll(true)}>Show all {table.length.toLocaleString("en-US")}</button>}
      </>}
    </div>
  );
}
