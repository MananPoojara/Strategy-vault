import type { ReactNode } from "react";
import { modeOf } from "../execution-model";
import { MAX_RUNS } from "../monte-carlo";
import type { Spec } from "../types";
import { setMc, useRunPrefs, type McPrefs } from "./run-prefs";
import { CASH_ONLY, NOT_IN_LAB, Why } from "./common";

/** AmiBroker Analysis settings › Monte Carlo: the options the Monte Carlo panel uses on the closed trades of a run. */
export function MonteCarloTab({ spec }: { spec: Spec }) {
  const mc = useRunPrefs().mc, off = !mc.enabled, money = modeOf(spec) === "money";
  const runs = Number(mc.runs), badRuns = !Number.isInteger(runs) || runs < 1 || runs > MAX_RUNS;
  const check = (k: "enabled" | "curves" | "logFinal" | "negativeDd", label: string) =>
    <div className="lab-row"><label className="lab-check"><input type="checkbox" aria-label={label} checked={mc[k]} disabled={k !== "enabled" && off} onChange={(e) => setMc({ [k]: e.target.checked } as Partial<McPrefs>)} /> {label}</label></div>;
  const radio = (name: string, label: string, checked: boolean, pick: () => void, why?: string, disabled = false, amount?: ReactNode) =>
    <div className="lab-row" key={label}><label className="lab-check"><input type="radio" name={name} aria-label={label} checked={checked} disabled={off || disabled || !!why} onChange={pick} /> {label}</label>{amount}<Why text={why} /></div>;
  // money-only sizes stay visible (disabled, with the reason) in % returns mode
  const sizing = (v: McPrefs["sizing"], label: string, amount?: ["shares" | "value" | "percent", string, string?]) => {
    const cash = v === "shares" || v === "value";
    return radio("lab-mc-sizing", label, mc.sizing === v, () => setMc({ sizing: v }), cash && !money ? CASH_ONLY : undefined, false,
      amount && <label className="dfield"><input type="number" aria-label={amount[1]} min={0} step="any" value={mc[amount[0]]}
        disabled={off || (cash && !money)} onChange={(e) => setMc({ [amount[0]]: e.target.value } as Partial<McPrefs>)} />{amount[2] && <span className="cap">{amount[2]}</span>}</label>);
  };
  return <div className="lab-mc-tab" data-lab-mc-tab>
    <p className="cap lab-stab-intro">Resamples the closed trades of a run, many times, to show the spread of outcomes.</p>
    {/* MonteCarloPanel is not mounted in the Results tabs, so the tab says so rather than implying a run */}
    <p className="cap" data-lab-mc-unshown>The Monte Carlo results are not shown in the lab yet; these options are kept for them.</p>
    {check("enabled", "Enable Monte Carlo simulation")}
    <div className="lab-row">
      <label className="dfield">Number of runs<input type="number" aria-label="Number of runs" min={1} max={MAX_RUNS} step={1} value={mc.runs} disabled={off} onChange={(e) => setMc({ runs: e.target.value })} /></label>
      {badRuns && !off && <p className="rnote">Runs must be a whole number from 1 to {MAX_RUNS}.</p>}
    </div>
    {radio("lab-mc-source", "Simulate using portfolio equity changes", false, () => {}, NOT_IN_LAB)}
    {radio("lab-mc-source", "Simulate using trade list", true, () => {})}
    <fieldset className="lab-fieldset" data-lab-mc-sizing>
      <legend>Position sizing</legend>
      {sizing("none", "Don't change")}
      {sizing("shares", "Fixed size", ["shares", "Shares per trade", "shares/contracts"])}
      {sizing("value", "Constant Value", ["value", "Value per trade"])}
      {sizing("fraction", "Percent of Equity", ["percent", "Percent of equity"])}
    </fieldset>
    {check("curves", "Enable MC Equity curves (Min/Max/Avg)")}
    <fieldset className="lab-fieldset" data-lab-mc-curves>
      <legend>Equity curves</legend>
      {radio("lab-mc-scale", "Show Absolute values in linear scale", mc.scale === "linear", () => setMc({ scale: "linear" }), undefined, !mc.curves)}
      {radio("lab-mc-scale", "Show Absolute values in logarithmic scale", mc.scale === "log", () => setMc({ scale: "log" }), undefined, !mc.curves)}
      {radio("lab-mc-scale", "Show Percent change", mc.scale === "percent", () => setMc({ scale: "percent" }), undefined, !mc.curves)}
      {!mc.curves && !off && <Why text="Tick Enable MC Equity curves to draw them" />}
      <div className="lab-row"><label className="dfield">Straw broom chart plots<input type="number" aria-label="Straw broom chart plots" value={0} disabled readOnly /></label><Why text={NOT_IN_LAB} /></div>
    </fieldset>
    {check("logFinal", "Use logarithmic scale for Final equity")}
    <div className="lab-row"><label className="lab-check"><input type="checkbox" aria-label="Use logarithmic scale for $ Drawdown" checked={false} disabled readOnly /> Use logarithmic scale for $ Drawdown</label><Why text={NOT_IN_LAB} /></div>
    {check("negativeDd", "Use negative numbers for Drawdown (reverse Drawdown CDF)")}
    {off && <p className="cap">Monte Carlo is off: the panel will not run until this is ticked.</p>}
  </div>;
}
