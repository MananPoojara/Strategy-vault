import { useEffect, useRef, useState } from "react";
import { COPY } from "./labels";
import { int, num, pct } from "./format";
import { comboCount, optimizablePaths, type OptimizableSetting, setAt, specTooLong, splitDate, valuesCount, withOptimize } from "./spec-model";
import { ErrorNotice, isProvisional, Loading, ProvisionalStrip, ServerLabel } from "./ui";
import type { Catalogue, OptimizeDim, OptimizeResult, Role, SavedMarket, Spec } from "./types";
import "./optimize.css";

const MAX_VALUES = 1000;
const RANK_LABELS: Record<string, string> = { cagr: "CAGR", sharpe: "Sharpe", monthly_thickness: "Monthly thickness", expectancy_ratio: "Expectancy ratio", mar: "CAR/MDD" };
const SPLIT_STATS: [string, string][] = [["total_return", "Total return"], ["cagr", "CAGR"], ["sharpe", "Sharpe"], ["trades", "Trades"]];
const fmt = (name: string, v: number | null) =>
  /(^|_)(total_return|cagr|monthly_thickness)$/.test(name) ? pct(v) : /(^|_)trades$/.test(name) ? int(v) : num(v);
const dimFmt = (v: number | undefined) => (v === undefined ? "" : Number.isInteger(v) ? int(v) : num(v, 2));

/** Value of a metric for a row of `cols` (dims first) or for `base` (cols after the dims). */
export function colValue(result: OptimizeResult, values: number[], name: string): number | null {
  const i = result.cols.indexOf(name);
  if (i < 0) return null;
  const j = values.length === result.cols.length ? i : i - result.dims.length;
  return typeof values[j] === "number" ? values[j] : null;
}

export function OptimizePanel(props: {
  spec: Spec; catalogue: Catalogue; market: SavedMarket; role: Role | null; dates: string[];
  onOptimize(spec: Spec, split: string | null): Promise<OptimizeResult>;
  onApply(spec: Spec): void; onTried(n: number): void; tried: number; instrumentLabel?: string;
}) {
  const { spec, catalogue, market, role, dates, onOptimize, onApply, onTried, tried, instrumentLabel } = props;
  const settings = optimizablePaths(spec, catalogue);
  const [sel, setSel] = useState<Record<string, OptimizeDim>>({});
  const [heldBack, setHeldBack] = useState(0);
  const [running, setRunning] = useState<number | null>(null); // start time
  const [, setTick] = useState(0);
  const [result, setResult] = useState<OptimizeResult | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [retryUntil, setRetryUntil] = useState(0); // 429 deadline; the main button waits for it too
  const last = useRef<[Spec, string | null] | null>(null);
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
  useEffect(() => () => { // a market change resets optimization; a run still in flight is ignored
    runId.current++;
    setResult(null); setError(null); setRunning(null); setRetryUntil(0);
  }, [market]);

  if (role === null || role === "viewer")
    return <section className="box" data-lab-optimize><h2 className="box-h">Optimization</h2><p className="rnote">{COPY.researcherOnly}</p></section>;

  const dims = settings.filter((s) => sel[s.path]).map((s) => sel[s.path]);
  const combos = comboCount(dims);
  const tooBig = dims.some((d) => valuesCount(d) > MAX_VALUES) || combos > catalogue.max_combos;
  const split = heldBack ? splitDate(dates, heldBack) : null;
  const tooLong = dims.length > 0 && specTooLong(withOptimize(spec, dims));
  const disabled = tooLong || running !== null || retryUntil > 0 || combos === 0 || tooBig || (heldBack > 0 && split === null);

  const toggle = (s: OptimizableSetting, on: boolean) => setSel((cur) => {
    const next = { ...cur };
    if (on) next[s.path] = { path: s.path, from: s.current, to: Math.min(s.max, s.current + 4 * s.step), step: s.step };
    else delete next[s.path];
    return next;
  });
  const edit = (path: string, key: "from" | "to" | "step", v: string) => setSel((cur) => ({ ...cur, [path]: { ...cur[path], [key]: Number(v) } }));

  const run = async (args: [Spec, string | null]) => {
    const id = ++runId.current;
    last.current = args;
    setResult(null);
    setError(null);
    setRetryUntil(0);
    setRunning(Date.now());
    try {
      const r = await onOptimize(...args);
      if (id !== runId.current) return;
      setResult(r);
      onTried(r.combos);
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
            {settings.map((s) => {
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
                  <td>{d ? valuesCount(d).toLocaleString("en-US") : ""}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      <p className="cap">{dims.length ? `${combos.toLocaleString("en-US")} combinations` : COPY.tickOne}</p>
      {tooBig && <p className="rnote">At most {MAX_VALUES.toLocaleString("en-US")} values per setting and {catalogue.max_combos.toLocaleString("en-US")} combinations. Narrow the ranges.</p>}
      <div className="lab-row">
        <label className="dfield">Held back
          <select aria-label="Held back" value={heldBack} onChange={(e) => setHeldBack(Number(e.target.value))}>
            <option value={0}>None</option>
            {[20, 30, 40].map((p) => <option key={p} value={p}>last {p}%</option>)}
          </select>
        </label>
        <button type="button" className="dbtn primary" data-lab-action="optimize" disabled={disabled}
          onClick={() => run([withOptimize(spec, dims), split])}>Run optimization</button>
      </div>
      {tooLong && <p className="rnote">{COPY.tooLong}</p>}
      {heldBack > 0 && <p className="cap">{split ? COPY.splitLine(heldBack, split) : dates.length ? "Not enough history for this split." : "Run a backtest first to set the hold-back date."}</p>}
      <p className="cap">Variants tried this session: {tried.toLocaleString("en-US")}</p>
      {running !== null && <><Loading text={`Optimizing… ${Math.floor((Date.now() - running) / 1000)} s`} /><p className="cap">{COPY.optimizeLimit}</p></>}
      {error !== null && <ErrorNotice error={error} market={market} instrumentLabel={instrumentLabel}
        onRetry={() => last.current && run(last.current)} />}
      {result && <OptimizeResultView result={result} spec={spec} ranSpec={last.current?.[0] ?? spec} catalogue={catalogue} settings={settings} market={market} instrumentLabel={instrumentLabel} onApply={onApply} />}
      {!result && !error && running === null && <p className="empty">{COPY.emptyOptimize}</p>}
    </section>
  );
}

function OptimizeResultView({ result, spec, ranSpec, catalogue, settings, market, instrumentLabel, onApply }: {
  result: OptimizeResult; spec: Spec; ranSpec: Spec; catalogue: Catalogue; settings: OptimizableSetting[]; market: SavedMarket; instrumentLabel?: string; onApply(spec: Spec): void;
}) {
  const { rank, dims } = result;
  const stats = rank.stats ?? [];
  const phases = result.split ? (["is", "oos"] as const) : [];
  const apply = (values: number[]) =>
    onApply(dims.reduce((s, d, i) => setAt(s, d.path, values[i]), withOptimize(spec, [])));
  const setting = (path: string) => settings.find((s) => s.path === path);
  const ranSettings = optimizablePaths(ranSpec, catalogue); // "Now" = the spec that was optimized
  // With a split the backend ranks on the in-sample window and returns those values as is_*.
  const rankCol = (s: string) => (result.split && result.cols.includes(`is_${s}`) ? `is_${s}` : s);
  const rows = [
    { name: "Now", values: result.base, pct: null as number[] | null },
    ...rank.top3.map((c, i) => ({ name: `#${i + 1}`, values: result.picked[String(c.row)] ?? [], pct: c.pct })),
  ];
  return (
    <div data-lab-optimize-result>
      <ServerLabel text={result.label} />
      {isProvisional(market) && <ProvisionalStrip note={result.provisional_note} instrumentLabel={result.instrument.label ?? instrumentLabel} />}
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
                {dims.map((d) => <th key={d.path}>{setting(d.path)?.label ?? d.path}</th>)}
                {stats.map((s) => <th key={s}>{RANK_LABELS[s] ?? s}</th>)}
                {phases.map((ph) => SPLIT_STATS.map(([k, l]) => <th key={`${ph}_${k}`}>{l}</th>))}
                <th></th>
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <tr key={r.name}>
                  <td>{r.name}</td>
                  {dims.map((d, i) => <td key={d.path}>{dimFmt(r.pct ? r.values[i] : ranSettings.find((s) => s.path === d.path)?.current)}</td>)}
                  {stats.map((s, k) => {
                    const v = fmt(s, colValue(result, r.values, rankCol(s)));
                    return <td key={s}>{r.pct && typeof r.pct[k] === "number" ? `${v} · p${Math.round(r.pct[k])}` : v}</td>;
                  })}
                  {phases.map((ph) => SPLIT_STATS.map(([k]) => <td key={`${ph}_${k}`}>{fmt(k, colValue(result, r.values, `${ph}_${k}`))}</td>))}
                  <td>{r.pct && r.values.length === result.cols.length && dims.every((d) => setting(d.path)) &&
                    <button type="button" className="dbtn sm" aria-label={`Apply ${r.name}`} onClick={() => apply(r.values)}>Apply</button>}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {result.split && <p className="rnote">{COPY.splitNote}</p>}
    </div>
  );
}
