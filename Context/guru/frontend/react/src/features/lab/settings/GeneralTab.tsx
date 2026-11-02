import { NumField } from "../OperandEditor";
import { ExecutionPanel } from "../ExecutionPanel";
import { changeExecution, GeneralData, MARGIN_NOTE, TickSize } from "../RiskPanel";
import { directionOf, type InstrumentContext } from "../execution-model";
import { getAt, setExecution } from "../spec-model";
import type { Catalogue, Spec } from "../types";
import { cashWhy, ONE_INSTRUMENT, SettingGroup, Why } from "./common";
import "./settings.css";

const BOTH_ONLY = "Only with Long and short positions.";

/** AmiBroker Analysis settings › General, in its order: Data, Backtest, Commissions & rates. Options the lab cannot do are left out; ones that do not apply
 *  right now stay visible, disabled with the reason. Initial equity is under Backtest parameters above (one place only). */
export function GeneralTab({ spec, catalogue, instrument, onChange }: { spec: Spec; catalogue: Catalogue; instrument: InstrumentContext; onChange(spec: Spec): void }) {
  if (!catalogue.execution) return null;
  const ex = spec.execution ?? {}, both = directionOf(spec) === "both";
  const cash = cashWhy(spec, catalogue, instrument);
  const limitWhy = cash ?? (spec.portfolio ? ONE_INSTRUMENT : null); // rules.rs refuses execution.limits in a portfolio
  const change = (path: string, value: unknown) => onChange(changeExecution(spec, path, value));
  const num = (label: string, path: string, min: number, max: number, why: string | null, placeholder = "0", step: number | null = null, zeroIsNone = false) =>
    <NumField label={label} value={getAt(ex, path) as number | undefined} min={min} max={max} step={step} placeholder={placeholder} disabled={!!why}
      onChange={(n) => change(path, zeroIsNone && n === 0 ? undefined : n)} />;
  const box = (label: string, checked: boolean, disabled: boolean, set?: (on: boolean) => void) => <label className="lab-check">
    <input type="checkbox" aria-label={label} checked={checked} disabled={disabled} onChange={(e) => set?.(e.target.checked)} /> {label}</label>;
  return <>
    <SettingGroup title="Data">
      {spec.portfolio ? <div className="lab-row">{box("Pad and align all data to reference symbol", false, true)}<Why text={ONE_INSTRUMENT} /></div>
        : <GeneralData spec={spec} instrument={instrument} onChange={onChange} tick={false} />}
    </SettingGroup>
    {/* the reason for greyed fields is said once per group, not on every row */}
    <SettingGroup title="Backtest" why={[limitWhy, spec.portfolio && ONE_INSTRUMENT]}>
      {/* Positions (Long / Short) is set once, on the Your idea row */}
      <div className="lab-row" data-lab-general-limits>
        {num("Min. shares", "limits.min_shares", 0, 1e9, limitWhy)}
        {num("Min. pos. value", "limits.min_value", 0, 1e12, limitWhy)}
        {num("Max. pos. value", "limits.max_value", 0, 1e12, limitWhy)}
        {num("Round lot size", "limits.round_lot", 0, 1e9, limitWhy, "0", 1, true)}
        <TickSize spec={spec} onChange={onChange} why={spec.portfolio ? ONE_INSTRUMENT : null} quiet />
        {box("Allow position size shrinking", ex.limits?.shrink === true, !!limitWhy, (on) => change("limits.shrink", on ? true : undefined))}
      </div>
      <p className="cap">0 = no maximum value · fractional shares (round lot) · no tick rounding.</p>
      <ExecutionPanel spec={spec} catalogue={catalogue} onChange={onChange} part="options" />
      <div className="lab-row" data-lab-general-flags>
        {/* AmiBroker's ReverseSignalForcesExit: off = an opposite entry signal is ignored while a position is open */}
        {box("Reverse entry signal forces exit", (ex.opposite ?? "reverse") !== "ignore", !both, (on) => onChange(setExecution(spec, { opposite: on ? undefined : "ignore" })))}
        <Why text={!both && BOTH_ONLY} />
      </div>
    </SettingGroup>
    <SettingGroup title="Commissions & rates" why={[cash, limitWhy]}>
      {/* three fields, not AmiBroker's radio: the lab charges all three together */}
      <div className="lab-row" data-lab-commissions>
        {num("percent", "costs.commission.pct", 0, 10, cash)}
        {num("$ per trade", "costs.commission.per_order", 0, 1e9, cash)}
        {num("$ per share/contract", "costs.commission.per_unit", 0, 1e6, cash)}
      </div>
      <div className="lab-row" data-lab-rates>
        {num("Fixed annual interest [%]", "limits.interest_pct", 0, 100, limitWhy)}
        {num("Margin rate [%]", "limits.margin_rate_pct", 0, 100, limitWhy)}
        {num("Account margin", "limits.margin_pct", 1, 100, limitWhy, "100")}
      </div>
      <p className="cap">Account margin 100 = no margin account.</p>
      {ex.limits?.margin_pct !== undefined && ex.limits.margin_pct < 100 && <p className="cap" data-lab-margin-note>{MARGIN_NOTE}</p>}
    </SettingGroup>
  </>;
}
