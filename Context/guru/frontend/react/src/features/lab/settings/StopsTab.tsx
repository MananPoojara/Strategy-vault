import type { ReactNode } from "react";
import { COPY } from "../labels";
import { ExitAtStopSelect, NBarStop, nBarStop, ReentryDelay, withNBarStop } from "../AmiSettings";
import { defaultLevel, levelAt, levelForm, LEVELS, withLevel, type LevelForm, type SidePre } from "../execution-model";
import { NumField } from "../OperandEditor";
import { setExecution } from "../spec-model";
import { Segmented } from "../ui";
import type { EntryLevel, Spec } from "../types";
import { STOP_NOTES } from "../OrdersPanel";
import { ScenarioPreset } from "./TradesTab";
import { Note, ONE_INSTRUMENT, Why } from "./common";

type Kind = "off" | LevelForm;
/** AmiBroker's four stop boxes in its order; `amount` is its label for the box's number. ATR is the lab's extra kind (the Exit card's ATR stops). */
const STOPS = [
  { key: "stop_loss", legend: "Maximum loss stop", label: "Stop-loss", amount: "Loss" },
  { key: "target", legend: "Profit target", label: "Target", amount: "Profit" },
  { key: "trailing", legend: "Trailing stop", label: "Trailing stop", amount: "Risk" },
] as const;
const KINDS: { value: Kind; label: string }[] = [{ value: "off", label: "Disabled" }, { value: "pct", label: "Percent" }, { value: "points", label: "Point" },
  { value: "profit_pct", label: "Percent of profit" }, { value: "atr", label: "ATR" }];

/** One side's stop: AmiBroker's disabled / percent / point (/ percent of profit) choice and its amount, on the same spec fields as the Exit card. */
function StopAmount({ spec, pre, stop, label, amount, short, onChange }: { spec: Spec; pre: SidePre; stop: "stop_loss" | "target" | "trailing"; label: string; amount: string; short: boolean; onChange(spec: Spec): void }) {
  const slot = `exit.${stop}` as const, value = levelAt(spec, pre, slot), form = levelForm(value), bounds = LEVELS[slot];
  // a level this tab cannot edit (e.g. the Exit card's entry-candle HSL) is shown as is, never rewritten
  const kept = value !== undefined && (!form || form === "at");
  const put = (d: EntryLevel | undefined) => onChange(withLevel(spec, pre, slot, d));
  const v = value as Record<string, number> | undefined;
  return <div className="lab-row">
    <Segmented label={`${label} type`} value={kept ? ("kept" as Kind) : form ?? "off"} options={KINDS.filter((k) => k.value !== "profit_pct" || bounds.profitPct)}
      onChange={(k) => put(k === "off" ? undefined : defaultLevel(slot, k, short))} />
    {/* AmiBroker keeps the amount box on screen while the stop is disabled */}
    {!kept && !v && <NumField label={`${label} ${amount.toLowerCase()}`} value={undefined} placeholder="0" disabled onChange={() => undefined} />}
    {kept ? <span className="lab-kept">{COPY.keptAsIs} <code>{JSON.stringify(value)}</code></span> : v && <>
      {form === "pct" && <NumField label={`${label} ${amount.toLowerCase()} (%)`} value={v.pct} min={0} max={bounds.maxPct} step={0.1} onChange={(n) => n !== undefined && put({ pct: n })} />}
      {form === "points" && <NumField label={`${label} ${amount.toLowerCase()} (points)`} value={v.points} min={0} max={1e9} step={0.1} onChange={(n) => n !== undefined && put({ points: n })} />}
      {form === "profit_pct" && <NumField label={`${label} ${amount.toLowerCase()} (% of profit)`} value={v.profit_pct} min={1} max={99} step={1} onChange={(n) => n !== undefined && put({ profit_pct: n })} />}
      {form === "atr" && <><NumField label={`${label} ATR ×`} value={v.atr} min={0} max={50} step={0.25} onChange={(n) => n !== undefined && put({ atr: n, atr_length: v.atr_length ?? 14 })} />
        <NumField label={`${label} ATR length`} value={v.atr_length ?? 14} min={1} max={500} step={1} onChange={(n) => n !== undefined && put({ atr: v.atr, atr_length: n })} /></>}
    </>}
  </div>;
}

/** AmiBroker Analysis settings › Stops: Scenario, Activate stops immediately, then the four stop boxes. Long and short each keep their own amounts;
 *  exit mode, re-entry delay and Has priority are one value for both sides (single instrument only, greyed with the reason in a portfolio). */
export function StopsTab({ spec, onChange }: { spec: Spec; onChange(spec: Spec): void }) {
  const ex = spec.execution ?? {}, single = !spec.portfolio, both = spec.direction === "both";
  const sides: [SidePre, string, boolean][] = both ? [["", "Long", false], ["short_side.", "Short", true]] : [["", "", spec.direction === "short"]];
  const name = (side: string, label: string) => side ? `${side} ${label.toLowerCase()}` : label;
  const shared = (body: ReactNode) => <><fieldset className="lab-row lab-delay" disabled={!single}>{body}</fieldset><Why text={!single && ONE_INSTRUMENT} /></>;
  const immediate = ex.activate_stops_immediately ?? true;
  return <div className="lab-stops-tab" data-lab-stops-tab>
    <p className="cap lab-stab-intro">Applied to every trade, in AmiBroker's order; the same stops as the Exit card in Your idea.</p>
    <div className="lab-row lab-stops-top">
      <ScenarioPreset spec={spec} onChange={onChange} />
      <label className="lab-check"><input type="checkbox" disabled={!single} checked={immediate}
        onChange={(e) => onChange(setExecution(spec, { activate_stops_immediately: e.target.checked ? undefined : false }))} />Activate stops immediately (trigger stops AFTER regular signals)</label>
      <Why text={!single && ONE_INSTRUMENT} />
    </div>
    {immediate ? <Note gist="Checked by default here (AmiBroker's default is unchecked).">A stop can trigger on the entry bar when the entry fills at the open or as a limit/stop order; an entry at the close or another later price is checked from the next bar.</Note>
      : <Note gist="Stops are first checked on the bar after the entry bar." />}
    <div className="lab-stops">
      {STOPS.map(({ key, legend, label, amount }) => <fieldset className="lab-fieldset" data-lab-stop={key} key={key}>
        <legend>{legend}</legend>
        {sides.map(([pre, side, short]) => <StopAmount key={pre} spec={spec} pre={pre} stop={key} label={name(side, label)} amount={amount} short={short} onChange={onChange} />)}
        {shared(<><ExitAtStopSelect spec={spec} stop={key} label={label} onChange={onChange} /><ReentryDelay spec={spec} stop={key} label={`${label} re-entry delay`} onChange={onChange} /></>)}
      </fieldset>)}
      <fieldset className="lab-fieldset" data-lab-stop="n_bar">
        <legend>N-bar stop</legend>
        {sides.map(([pre, side]) => <div className="lab-row" key={pre}>
          <Segmented label={`${name(side, "N-bar stop")} type`} value={nBarStop(spec, pre) !== undefined ? "bars" : "off"} options={[{ value: "off", label: "Disabled" }, { value: "bars", label: "Bars" }]}
            onChange={(k) => onChange(withNBarStop(spec, pre, k === "bars" ? 5 : undefined))} />
          {nBarStop(spec, pre) !== undefined ? <NBarStop spec={spec} pre={pre} label={name(side, "Bars")} onChange={onChange} />
            : <NumField label={name(side, "Bars")} value={undefined} placeholder="0" disabled onChange={() => undefined} />}
        </div>)}
        {shared(<><label className="lab-check"><input type="checkbox" checked={!!ex.nbar_has_priority}
          onChange={(e) => onChange(setExecution(spec, { nbar_has_priority: e.target.checked || undefined }))} />Has priority</label>
          <ReentryDelay spec={spec} stop="bars" label="N-bar re-entry delay" onChange={onChange} /></>)}
        <p className="cap">{STOP_NOTES.nbar}</p>
      </fieldset>
    </div>
    <Note data-lab-stop-modes gist={`How the "Exit at stop?" modes fill${both ? " (one choice for both sides)" : ""}`}>{both ? "Exit at stop? applies to both sides. " : ""}{STOP_NOTES.modes}</Note>
  </div>;
}
