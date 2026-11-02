import { useState } from "react";
import { int, num, pct } from "./format";
import { fixedReturns, MAX_RUNS, MAX_WORK, runMonteCarlo, spanYears, tradeReturns, type McResult } from "./monte-carlo";
import { ChartFrame } from "./Report";
import type { LedgerRow, Trade } from "./types";
import { setMc, useRunPrefs } from "./settings/run-prefs";
import "./report.css";
import "./monte-carlo.css";

const POINTS = 200; // shortcut: charts sample at most 200 points per line; fine at panel width.
const pick = <T,>(a: T[]) => (a.length <= POINTS ? a.map((v, i) => [i, v] as const) : Array.from({ length: POINTS }, (_, j) => Math.round((j * (a.length - 1)) / (POINTS - 1))).map((i) => [i, a[i]] as const));

/** Lines over x positions `xs` (trade numbers); every series has one value per x. */
function Lines({ series, xs, xMax, caption, data, fmt, log: logScale }: { series: [string, number[]][]; xs: number[]; xMax: number; caption: string; data: string; fmt: (v: number) => string; log?: boolean }) {
  const raw = series.flatMap(([, v]) => v);
  // Settings › Monte Carlo picks the log scale; a ruined run (equity 0) has no log, so that chart stays linear
  const log = !!logScale && Math.min(...raw) > 0, f = (y: number) => (log ? Math.log(y) : y);
  const lo = Math.min(...raw), hi = Math.max(...raw), [y0, y1] = [f(lo), f(hi)];
  const sx = (x: number) => 2 + (x / (xMax || 1)) * 96, sy = (y: number) => 56 - ((f(y) - y0) / (y1 - y0 || 1)) * 52;
  return (
    <ChartFrame caption={caption} sub={`min · median · max${log ? " · log scale" : ""}`} label={`${caption} chart`} data={{ "data-lab-mc-chart": data }}
      y={[fmt(hi), fmt(lo)]} x={["0", "Trade #", int(xMax)]}>
      {series.map(([cls, v]) => <polyline key={cls} className={cls} points={v.map((y, j) => `${sx(xs[j]).toFixed(2)},${sy(y).toFixed(2)}`).join(" ")} />)}
    </ChartFrame>
  );
}

/** AmiBroker percentile chart: x = percentile of runs (0–100), y = the value at that percentile (sorted ascending). */
function Percentiles({ sorted, caption, data, fmt, log: logScale }: { sorted: number[]; caption: string; data: string; fmt: (v: number) => string; log?: boolean }) {
  const log = !!logScale && sorted[0] > 0, f = (y: number) => (log ? Math.log(y) : y);
  const [y0, y1] = [f(sorted[0]), f(sorted[sorted.length - 1])], n = sorted.length;
  const sx = (i: number) => 2 + (i / (n - 1 || 1)) * 96, sy = (y: number) => 56 - ((f(y) - y0) / (y1 - y0 || 1)) * 52;
  return (
    <ChartFrame caption={caption} sub={log ? "log scale" : undefined} label={`${caption} chart`} data={{ "data-lab-mc-chart": data }} y={[fmt(sorted[n - 1]), fmt(sorted[0])]} x={["0", "Percentile", "100"]}>
      <polyline className="mid" points={pick(sorted).map(([i, y]) => `${sx(i).toFixed(2)},${sy(y).toFixed(2)}`).join(" ")} />
    </ChartFrame>
  );
}

/** AmiBroker Monte Carlo: resamples this run's closed trades in the browser; runs on click only. */
export function MonteCarloPanel({ trades, from, to, ledger, capital }: { trades: Trade[]; from: string; to: string; ledger?: LedgerRow[]; capital?: number }) {
  const mc = useRunPrefs().mc, { runs, sizing, percent, shares, value } = mc; // shared with Settings › Monte Carlo
  const [seed, setSeed] = useState("1");
  const [out, setOut] = useState<{ trades: Trade[]; res: McResult }>();
  const money = !!capital && !!ledger?.length, fixed = sizing === "shares" || sizing === "value";
  const nFixed = Number(sizing === "shares" ? shares : value);
  const rets = fixed ? (money ? fixedReturns(trades, sizing, nFixed, capital!) : []) : tradeReturns(trades, ledger, capital);
  // Money mode shows equity in money from the capital (AmiBroker); unit mode in multiples of the starting 1.0.
  const scale = money ? capital! : 1, eq = (v: number) => (money ? num(v * scale, 0) : num(v, 3)), ddUnit = money ? "$" : "×";
  const open = trades.filter((t) => t.open).length;
  const nRuns = Number(runs), nPct = Number(percent), nSeed = Number(seed);
  const problem = !mc.enabled ? "Monte Carlo is off: tick Enable Monte Carlo simulation in Settings › Monte Carlo."
    : fixed && !money ? "Fixed size and Fixed value need a money-mode run (Initial equity set)."
    : fixed && !(nFixed > 0 && Number.isFinite(nFixed)) ? `${sizing === "shares" ? "Shares" : "Value"} per trade must be above 0.`
    : !rets.length ? "No closed trades to resample."
    : !Number.isInteger(nRuns) || nRuns < 1 || nRuns > MAX_RUNS ? `Runs must be a whole number from 1 to ${MAX_RUNS}.`
    : sizing === "fraction" && !(nPct > 0 && nPct <= 100) ? "Percent of equity must be above 0 and at most 100."
    : !Number.isInteger(nSeed) ? "Seed must be a whole number." : null;
  const res = out?.trades === trades ? out.res : undefined; // a new backtest clears the previous Monte Carlo
  const run = () => setOut({ trades, res: runMonteCarlo({ rets, runs: nRuns, seed: nSeed, years: spanYears(from, to), sizing: sizing === "fraction" ? { kind: "fraction", pct: nPct } : fixed ? { kind: "fixed" } : { kind: "none" } }) });
  // positive drawdowns (AmiBroker's reverse CDF): PERCENTILES are symmetric, so row i takes row n−1−i negated and the worst moves to 99%
  const flip = (v: number) => (mc.negativeDd ? v : -v || 0), asc = (a: number[]) => (mc.negativeDd ? a : a.map(flip).reverse());
  const ddRows = res ? (mc.negativeDd ? res.rows : [...res.rows].reverse()) : [];
  const change = mc.scale === "percent", curve = (a: number[]) => (change ? a.map((v) => v - 1) : a);
  return (
    <section className="box lab-mc" data-lab-monte-carlo>
      <h3 className="box-h">Monte Carlo <small>resampled simulated trades · simulated · not a book method · not a forecast</small></h3>
      <div className="lab-row lab-mc-controls">
        <label className="dfield">Runs<input type="number" aria-label="Runs" min={1} max={MAX_RUNS} step={1} value={runs} onChange={(e) => setMc({ runs: e.target.value })} /></label>
        <label className="dfield">Sizing<select aria-label="Sizing" value={sizing} onChange={(e) => setMc({ sizing: e.target.value as typeof sizing })}>
          <option value="none">{money ? "Don't change (backtest position sizes)" : "Don't change"}</option>
          <option value="shares" disabled={!money} title={money ? undefined : "Needs a money-mode run (Initial equity set)"}>Fixed size</option>
          <option value="value" disabled={!money} title={money ? undefined : "Needs a money-mode run (Initial equity set)"}>Fixed value</option>
          <option value="fraction">Percent of equity</option></select></label>
        {sizing === "shares" && <label className="dfield">Shares<input type="number" aria-label="Shares per trade" min={0} step="any" value={shares} onChange={(e) => setMc({ shares: e.target.value })} /></label>}
        {sizing === "value" && <label className="dfield">Value<input type="number" aria-label="Value per trade" min={0} step="any" value={value} onChange={(e) => setMc({ value: e.target.value })} /></label>}
        {sizing === "fraction" && <label className="dfield">Percent<input type="number" aria-label="Percent of equity" min={0} max={100} step="any" value={percent} onChange={(e) => setMc({ percent: e.target.value })} /></label>}
        <label className="dfield">Seed<input type="number" aria-label="Seed" step={1} value={seed} onChange={(e) => setSeed(e.target.value)} /></label>
        <button type="button" className="dbtn sm primary" disabled={!!problem} onClick={run}>Run Monte Carlo</button>
      </div>
      <p className="rnote" data-lab-mc-note>{problem ?? `Resamples ${int(rets.length)} closed trades with replacement${open ? `; ${int(open)} open trade${open === 1 ? " is" : "s are"} excluded` : ""}.${fixed ? ` ${sizing === "shares" ? "Fixed size: shares × entry price × trade %" : "Fixed value: value × trade %"} is added to equity per trade (not compounded); the trade % includes costs; no cash or margin check.` : money ? " Money mode: each trade's P&L over the cash equity before it." : ""}`}</p>
      {res && <>
        <div className="tblwrap">
          <table className="tbl" data-lab-mc-table>
            <thead><tr><th>Percentile</th><th className="n">Final Equity</th><th className="n">Annual Return</th><th className="n">Max. Drawdown {ddUnit}</th><th className="n">Max. Drawdown %</th><th className="n">Lowest Eq.</th></tr></thead>
            <tbody>{res.rows.map((r, i) => <tr key={r.p}><td>{r.p}%</td><td className="n">{eq(r.final)}</td><td className="n">{pct(r.cagr, 2)}</td><td className="n">{eq(flip(ddRows[i].maxDdAbs))}</td><td className="n">{pct(flip(ddRows[i].maxDd), 2)}</td><td className="n">{eq(r.lowest)}</td></tr>)}</tbody>
          </table>
        </div>
        <div className="lab-mc-charts">
          {mc.curves && <Lines caption={change ? "MC Min/Max Equity % change" : "MC Min/Max Equity"} data="envelope" xs={res.envelope.at} xMax={res.trades} fmt={change ? (v) => pct(v, 0) : eq}
            log={mc.scale === "log"} series={[["lo", curve(res.envelope.min)], ["hi", curve(res.envelope.max)], ["mid", curve(res.envelope.median)]]} />}
          <Percentiles caption="MC Final Equity" data="final" sorted={res.finals} fmt={eq} log={mc.logFinal} />
          <Percentiles caption="MC Drawdown %" data="drawdown" sorted={asc(res.drawdowns)} fmt={(v) => pct(v, 0)} />
          <Percentiles caption={`MC Drawdown ${ddUnit}`} data="drawdown-abs" sorted={asc(res.drawdownsAbs)} fmt={eq} />
          <Percentiles caption="MC Lowest Equity" data="lowest" sorted={res.lowests} fmt={eq} />
        </div>
        <p className="rnote">{int(res.runs)} runs × {int(res.trades)} trades, equity from {money ? num(capital, 0) : "1.0"}; each column is its own percentile, so a row is not one run. {mc.negativeDd ? "Drawdowns are negative: the 1% row is the worst." : "Drawdowns are positive: the 99% row is the worst."}</p>
        {res.capped && <p className="rnote" role="note" data-lab-mc-capped>Runs were capped at {int(res.runs)} so that runs × trades stays within {int(MAX_WORK)}.</p>}
      </>}
    </section>
  );
}
