import { Fragment, useRef, useState } from "react";
import { dayFirst } from "./DateField";
import { int, num, pct } from "./format";
import { ErrorNotice, Loading, MetricTiles, ServerLabel, tone } from "./ui";
import type { LabInfo, SavedMarket, Spec, WalkforwardMetrics, WalkforwardOptions, WalkforwardResult, WalkforwardTarget } from "./types";
import "./report.css";

export const WF_LABEL = "out-of-sample results only · simulated";
const MAX_WINDOWS = 60, MIN_IS_BARS = 21; // backend walkforward.rs MAX_WINDOWS and the 21-bar in-sample floor

const addMonths = (d: string, m: number) => {
  const k = Number(d.slice(0, 4)) * 12 + Number(d.slice(5, 7)) - 1 + m;
  return `${String(Math.floor(k / 12)).padStart(4, "0")}-${String((k % 12) + 1).padStart(2, "0")}${d.slice(7)}`;
};
export interface WfWindow { is: [string, string]; oos: [string, string] }
/** Client copy of walkforward.rs windows() over the backtest's bars, for the preview before running; the server stays authoritative. */
export function wfWindows(dates: string[], o: Pick<WalkforwardOptions, "is" | "oos" | "step" | "unit" | "anchored">): { windows: WfWindow[]; error?: string } {
  const u = o.unit === "years" ? 12 : 1, to = dates.length - 1, out: WfWindow[] = [];
  if (to < 0) return { windows: out };
  const at = (months: number) => { const t = addMonths(dates[0], months), i = dates.findIndex((d) => d >= t); return i < 0 ? dates.length : i; };
  for (let k = 0; ; k++) {
    const isFrom = o.anchored ? 0 : at(k * o.step * u), oosFrom = at((o.is + k * o.step) * u);
    if (oosFrom > to) break;
    if (out.length === MAX_WINDOWS) return { windows: out, error: `More than ${MAX_WINDOWS} windows; lengthen the step or shorten the range.` };
    const oosEnd = Math.min(at((o.is + k * o.step + o.oos) * u), to + 1);
    if (oosFrom < isFrom + MIN_IS_BARS) return { windows: out, error: `The in-sample window starting ${dates[isFrom]} has ${oosFrom - isFrom} bars; at least ${MIN_IS_BARS} are needed.` };
    out.push({ is: [dates[isFrom], dates[oosFrom - 1]], oos: [dates[oosFrom], dates[oosEnd - 1]] });
  }
  return out.length ? { windows: out } : { windows: out, error: "The range is shorter than one in-sample window plus one out-of-sample bar." };
}
// CAR/MDD first: AmiBroker's walk-forward and optimizer default target
const TARGETS: [WalkforwardTarget, string][] = [["mar", "CAR/MDD"], ["score", "Optimizer score"], ["cagr", "CAGR"], ["sharpe", "Sharpe"], ["monthly_thickness", "Monthly thickness"], ["expectancy_ratio", "Expectancy ratio"]];
const POINTS = 200; // shortcut: the curve samples at most 200 points; fine at panel width.

function Equity({ dates, equity }: { dates: string[]; equity: number[] }) {
  if (equity.length < 2) return null;
  const idx = equity.length <= POINTS ? equity.map((_, i) => i) : Array.from({ length: POINTS }, (_, j) => Math.round((j * (equity.length - 1)) / (POINTS - 1)));
  const [y0, y1] = [Math.min(...equity), Math.max(...equity)];
  const sx = (i: number) => 2 + (i / (equity.length - 1)) * 96, sy = (y: number) => 56 - ((y - y0) / (y1 - y0 || 1)) * 52;
  return (
    <figure className="lab-report-chart lab-wf-chart" data-lab-wf-equity>
      <figcaption>Stitched out-of-sample equity <small>segments compounded</small></figcaption>
      <svg viewBox="0 0 100 60" preserveAspectRatio="none" role="img" aria-label="Stitched out-of-sample equity chart">
        <polyline points={idx.map((i) => `${sx(i).toFixed(2)},${sy(equity[i]).toFixed(2)}`).join(" ")} />
      </svg>
      <div className="lab-report-axis"><span>{dayFirst(dates[0])}</span><span>{dayFirst(dates[dates.length - 1])}</span></div>
    </figure>
  );
}

const cells = (m: WalkforwardMetrics | null) => <>
  <td className={tone(m?.total_return)}>{pct(m?.total_return)}</td><td className={tone(m?.cagr)}>{pct(m?.cagr)}</td>
  <td className="n">{pct(m?.max_dd)}</td><td className="n">{int(m?.trades)}</td><td className="n">{pct(m?.win_rate)}</td>
</>;
/** AmiBroker Analysis "Info" tab: elapsed time, combinations evaluated, threads and warnings; collapsed until asked for. */
export function InfoLog({ info }: { info?: LabInfo }) {
  const [open, setOpen] = useState(false);
  if (!info) return null;
  const n = info.warnings.length;
  return (
    <div className="lab-info" data-lab-info>
      <button type="button" className="dbtn sm" aria-expanded={open} onClick={() => setOpen(!open)}>{open ? "▾" : "▸"} Info{n ? ` · ${n} warning${n === 1 ? "" : "s"}` : ""}</button>
      {open && <ul className="lab-info-log">
        <li>Elapsed: {(info.ms / 1000).toFixed(2)} s</li><li>Combinations evaluated: {int(info.combos)}</li><li>Threads: {int(info.threads)}</li>
        {n ? info.warnings.map((w, i) => <li key={i} className="lab-info-warn">{w}</li>) : <li>No warnings.</li>}
      </ul>}
    </div>
  );
}

// The server's declared notes name a response field; say it in words.
const noteText = (n: string) => n.replace(/counted in open_at_edge/g, "counted as open at end");

/** AmiBroker "easy mode" walk-forward under the optimizer: uses the ticked settings as the grid of every in-sample window. */
export function WalkForward({ spec, dates = [], blocked, market, instrumentLabel, label, onRun, onTried, onRunning }: {
  spec: Spec; blocked: boolean; market: SavedMarket; instrumentLabel?: string; label(path: string): string;
  /** Bars of the last backtest: the range the windows are previewed over. */
  dates?: string[];
  onRun(spec: Spec, o: WalkforwardOptions): Promise<WalkforwardResult>;
  /** Settings optimized (combinations × windows), for the "Variants tried this session" count. */
  onTried?(n: number): void;
  /** The optimizer and walk-forward share one engine slot: the panel holds its own button while this runs. */
  onRunning?(running: boolean): void;
}) {
  const [o, setO] = useState<WalkforwardOptions>({ is: 2, oos: 1, step: 1, unit: "years", anchored: false, target: "mar" });
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<WalkforwardResult | null>(null);
  const [error, setError] = useState<unknown>(null);
  const runId = useRef(0);
  const set = <K extends keyof WalkforwardOptions>(k: K, v: WalkforwardOptions[K]) => setO((cur) => ({ ...cur, [k]: v }));
  const bad = [o.is, o.oos, o.step].some((n) => !Number.isInteger(n) || n < 1) || o.step < o.oos;
  const run = async () => {
    const id = ++runId.current;
    setResult(null); setError(null); setRunning(true); onRunning?.(true);
    try { const r = await onRun(spec, o); onTried?.(r.combos * r.windows.length); if (id === runId.current) setResult(r); }
    catch (e) { if (id === runId.current) setError(e); }
    finally { if (id === runId.current) { setRunning(false); onRunning?.(false); } }
  };
  const preview = bad ? { windows: [] } : wfWindows(dates, o);
  const [first, lastW] = [preview.windows[0], preview.windows[preview.windows.length - 1]];
  const lengthField = (k: "is" | "oos" | "step", text: string, span?: string) => (
    <label className="dfield">{text}
      <input type="number" min={1} step={1} aria-label={`Walk-forward ${text}`} value={o[k]} onChange={(e) => set(k, Number(e.target.value))} />
      {span && <span className="cap lab-wf-span">{span}</span>}
    </label>
  );
  const paths = (spec.optimize ?? []).map((d) => d.path);
  const value = (v: number | undefined) => (v === undefined ? "—" : num(v, Number.isInteger(v) ? 0 : 2));
  const params = (p: Record<string, number> | null) => paths.map((k) => <td className="n" key={k}>{p ? value(p[k]) : "—"}</td>);
  const sum = result?.summary;
  return (
    <div className="lab-wf" data-lab-walkforward>
      <h3 className="box-h">Walk-forward <small>{WF_LABEL}</small></h3>
      <div className="lab-row lab-wf-form">
        {lengthField("is", "In-sample", first && `${dayFirst(first.is[0])} – ${dayFirst(first.is[1])}`)}
        {lengthField("oos", "Out-of-sample", first && `${dayFirst(first.oos[0])} – ${dayFirst(first.oos[1])}`)}
        {lengthField("step", "Step", lastW && `last ends ${dayFirst(lastW.oos[1])}`)}
        <label className="dfield">Unit
          <select aria-label="Walk-forward unit" value={o.unit} onChange={(e) => set("unit", e.target.value as WalkforwardOptions["unit"])}>
            <option value="years">years</option><option value="months">months</option>
          </select>
        </label>
        <label className="dfield"><input type="checkbox" aria-label="Anchored" checked={o.anchored} onChange={(e) => set("anchored", e.target.checked)} /> Anchored</label>
        <label className="dfield">Optimization target
          <select aria-label="Walk-forward target" value={o.target} onChange={(e) => set("target", e.target.value as WalkforwardTarget)}>
            {TARGETS.map(([v, l]) => <option key={v} value={v}>{l}</option>)}
          </select>
        </label>
        <button type="button" className="dbtn primary" data-lab-action="walkforward" disabled={blocked || bad || running} onClick={() => void run()}>Run walk-forward</button>
      </div>
      <p className="cap">Each window optimizes the ticked settings on its in-sample period only, then runs the best setting on the next out-of-sample period. At most 60 windows; the step must be at least the out-of-sample length.</p>
      {!result && !running && (dates.length === 0 ? <p className="cap">Run a backtest first to preview the windows.</p> : <>
        {preview.error && <p className="rnote">{preview.error}</p>}
        {preview.windows.length > 0 && <div className="tblwrap lab-wf-preview" data-lab-wf-preview>
          <table className="tbl ocmp">
            <thead><tr><th>Mode</th><th>Start</th><th>End</th></tr></thead>
            <tbody>{preview.windows.map((w) => <Fragment key={w.is[0] + w.oos[0]}>
              <tr data-lab-wf-mode="is"><td>IS</td><td>{dayFirst(w.is[0])}</td><td>{dayFirst(w.is[1])}</td></tr>
              <tr data-lab-wf-mode="oos"><td>OOS</td><td>{dayFirst(w.oos[0])}</td><td>{dayFirst(w.oos[1])}</td></tr>
            </Fragment>)}</tbody>
          </table>
        </div>}
        {preview.windows.length > 0 && <p className="cap">{preview.windows.length} windows over the last backtest's range.</p>}
      </>)}
      {running && <Loading text="Running walk-forward…" />}
      {error !== null && <ErrorNotice error={error} market={market} instrumentLabel={instrumentLabel} onRetry={() => void run()} />}
      {result && (
        <div data-lab-wf-result>
          <ServerLabel text={result.label} />
          {sum && <MetricTiles items={[
            { label: "Return", value: pct(sum.total_return) }, { label: "CAGR", value: pct(sum.cagr) }, { label: "Max DD", value: pct(sum.max_dd) },
            { label: "Trades", value: int(sum.trades) }, { label: "Win %", value: pct(sum.win_rate) }, { label: "Windows", value: int(sum.windows) },
          ]} />}
          <p className="cap" data-lab-wf-summary>
            Out-of-sample {dayFirst(result.summary.from)} – {dayFirst(result.summary.to)}
            {result.summary.skipped ? ` · ${result.summary.skipped} held flat: no eligible setting in-sample` : ""}
            {result.summary.open_at_edge ? ` · ${result.summary.open_at_edge} window${result.summary.open_at_edge === 1 ? "" : "s"} ended with a position open (marked, not counted as a trade)` : ""}
            {result.summary.gaps ? " · gaps between windows are not traded and not in the CAGR" : ""}
          </p>
          {!!result.notes?.length && <ul className="rnote" data-lab-wf-notes>{result.notes.map((n) => <li key={n}>{noteText(n)}</li>)}</ul>}
          <div className="tblwrap">
            <table className="tbl ocmp lab-wf-table">
              <thead><tr><th>Mode</th><th>Start</th><th>End</th>{paths.map((k) => <th className="n" key={k}>{label(k)}</th>)}
                <th className="n">Return</th><th className="n">CAGR</th><th className="n">Max DD</th><th className="n">Trades</th><th className="n">Win %</th></tr></thead>
              <tbody>
                {result.windows.map((w) => (
                  <Fragment key={w.is.from + w.oos.from}>
                    <tr data-lab-wf-mode="is"><td>IS</td><td>{dayFirst(w.is.from)}</td><td>{dayFirst(w.is.to)}</td>{params(w.params)}{cells(w.is_metrics)}</tr>
                    <tr data-lab-wf-mode="oos"><td><b>OOS</b>{w.open_at_end && <small title="A position was open at the window's end (marked at its last close)"> · open at end</small>}</td>
                      <td>{dayFirst(w.oos.from)}</td><td>{dayFirst(w.oos.to)}</td>{w.skipped ? <td colSpan={paths.length || 1}>held flat</td> : params(w.params)}{cells(w.oos_metrics)}</tr>
                  </Fragment>
                ))}
              </tbody>
            </table>
          </div>
          <Equity dates={result.oos_equity.dates} equity={result.oos_equity.equity} />
          <InfoLog info={result.info} />
        </div>
      )}
    </div>
  );
}
