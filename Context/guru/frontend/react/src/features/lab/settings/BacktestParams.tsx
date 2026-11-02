import type { ReactNode } from "react";
import { NumField } from "../OperandEditor";
import { ExecutionPanel } from "../ExecutionPanel";
import { changeExecution } from "../RiskPanel";
import { modeOf, moneyBlocked, sizingKind, supportsMoney, withSizingKind, type InstrumentContext, type SizingKind } from "../execution-model";
import { getAt } from "../spec-model";
import type { Catalogue, Spec } from "../types";
import { SettingGroup } from "./common";

export const NEEDS_ENGINE = "Needs the updated engine";
/** The AFL entry gates after Maximum position, in its order: [spec field, label, off, on]. Off = the field is absent. */
const GATES = [["dead_stock", "Dead stock backtest", "No", "Yes"], ["turnover_filter", "Turnover filter", "No", "Yes"], ["aux1", "Aux1", "Off", "On"], ["aux2", "Aux2", "Off", "On"]] as const;

/** The AFLs' "BACKTEST PARAMETER" panel, in its order and with its names, the same for an index and a stock:
 *  Initial equity, Position size per Equity|Share, % of equity|Fixed equity, the size, Equity allocation, Brokerage, Maximum position,
 *  then Dead stock, Penny stock price, Turnover and Aux1/Aux2, then the circuits. Money follows the instrument (IndicatorsPage).
 *  `unitCosts` are the % mode's brokerage and slippage boxes; `orders` the entry limit / stop orders. */
export function BacktestParams({ spec, catalogue, instrument, onChange, unitCosts, orders }: {
  spec: Spec; catalogue: Catalogue; instrument: InstrumentContext; onChange(spec: Spec): void; unitCosts: ReactNode; orders: ReactNode;
}) {
  if (!catalogue.execution) return null;
  const ex = spec.execution ?? {}, money = modeOf(spec) === "money", caps = catalogue.execution;
  const blocked = moneyBlocked(instrument, spec) ?? (!supportsMoney(catalogue, instrument.kind) ? "Money mode is not supported by this engine yet." : null);
  const kind = sizingKind(spec), change = (path: string, value: unknown) => onChange(changeExecution(spec, path, value));
  // AFL BKTEST (Equity|SHARE) and bktest1 (% EQUITY|FIXED EQUITY); risk % stays reachable only for a spec that already uses it
  const per = kind === "units" ? "units" : kind === "risk_pct" ? "risk_pct" : "equity", fixed = kind === "value";
  const cash = (k: SizingKind) => !money || !caps.sizing.includes(k);
  // "% of equity" at 100 is the engine's default (no sizing field); any other percent is sizing.pct_equity
  const size = kind === "full" || fixed ? 100 : getAt(ex.sizing, kind) as number | undefined;
  // the engine refuses these gates on a portfolio (rules.rs portfolio_rules); a gate already on stays enabled so it can be turned off
  const gate = (field: string) => !caps.entry_gates?.includes(field) ? NEEDS_ENGINE
    : spec.portfolio && !getAt(ex, field) ? "Single-instrument runs only; a portfolio refuses this gate." : null;
  const noVolume = instrument.hasVolume === false ? "Needs volume; this instrument has no volume in its data." : null;
  return <SettingGroup title="Backtest parameters">
    <div className="lab-row" data-lab-equity>
      <NumField label="Initial equity" value={money ? ex.capital : undefined} min={0} max={1e12} disabled={!money}
        placeholder={money ? undefined : blocked ? "in %" : "10,00,000"} onChange={(n) => n !== undefined && change("capital", n)} />
    </div>
    {!money && blocked && <p className="cap" data-lab-money-blocked>{blocked}</p>}
    <div className="lab-row" data-lab-size>
      <label className="dfield">Position size per<select aria-label="Position size per" value={per}
        onChange={(e) => onChange(withSizingKind(spec, e.target.value === "equity" ? "full" : e.target.value as SizingKind))}>
        <option value="equity">Equity</option>
        <option value="units" disabled={cash("units")}>Share</option>
        {per === "risk_pct" && <option value="risk_pct">Risk % per trade</option>}
      </select></label>
      {per === "equity" && <label className="dfield">Equity mode<select aria-label="Equity mode" value={fixed ? "value" : "full"}
        onChange={(e) => onChange(withSizingKind(spec, e.target.value as SizingKind))}>
        <option value="full">% of equity</option>
        <option value="value" disabled={cash("value")}>Fixed equity</option>
      </select></label>}
      <NumField label={per === "units" ? "No. of shares" : per === "risk_pct" ? "Risk %" : "% of equity"} value={size} disabled={fixed}
        min={per === "units" ? 1 : 0} max={per === "units" ? 1e9 : 100} step={per === "units" ? 1 : null}
        onChange={(n) => n !== undefined && (per === "equity" ? onChange(changeExecution(spec, "sizing", n === 100 ? undefined : { ...ex.sizing, pct_equity: n })) : change(`sizing.${kind}`, n))} />
      <NumField label="Equity allocation" value={fixed ? getAt(ex.sizing, "value") as number | undefined : 100_000} min={0} max={1e12} disabled={!fixed}
        onChange={(n) => n !== undefined && change("sizing.value", n)} />
    </div>
    {per === "risk_pct" && <p className="cap">Risk % per trade: the lab's own sizing, kept because this strategy uses it; not in the AFL panel.</p>}
    <div className="lab-row" data-lab-brokerage>
      {money ? <>
        <NumField label="Brokerage (%)" value={ex.costs?.commission?.pct} min={0} max={10} placeholder="0" onChange={(n) => change("costs.commission.pct", n)} />
        <NumField label="Slippage (bps)" value={ex.costs?.slippage?.bps} min={0} max={1000} placeholder="0" onChange={(n) => change("costs.slippage.bps", n)} />
      </> : unitCosts}
    </div>
    <div className="lab-row" data-lab-max-position>
      {spec.portfolio
        ? <NumField label="Maximum position" value={spec.portfolio.max_open ?? 1} min={1} max={catalogue.portfolio?.max_open ?? 50} step={1}
            onChange={(n) => n !== undefined && onChange({ ...spec, portfolio: { ...spec.portfolio!, max_open: n } })} />
        : <NumField label="Maximum position" value={1} min={1} max={1} step={1} disabled onChange={() => {}} />}
      {!spec.portfolio && <span className="cap">one instrument holds one position</span>}
    </div>
    <div className="lab-row" data-lab-gates>
      {GATES.slice(0, 1).map(gateSelect)}
      <span title={gate("penny_price") ?? undefined}><NumField label="Penny stock price" value={ex.penny_price ?? 0} min={0} max={1e6} disabled={!!gate("penny_price")}
        onChange={(n) => change("penny_price", n ? n : undefined)} /></span>
      {GATES.slice(1).map(gateSelect)}
    </div>
    <ExecutionPanel spec={spec} catalogue={catalogue} onChange={onChange} part="circuits" />
    {orders}
  </SettingGroup>;

  function gateSelect([field, label, off, on]: typeof GATES[number]) {
    const why = gate(field) ?? (field === "turnover_filter" ? noVolume : null);
    return <label key={field} className="dfield" title={why ?? undefined}>{label}<select aria-label={label} value={ex[field] === true ? "on" : "off"} disabled={!!why}
      onChange={(e) => change(field, e.target.value === "on" ? true : undefined)}>
      <option value="off">{off}</option><option value="on">{on}</option>
    </select></label>;
  }
}
