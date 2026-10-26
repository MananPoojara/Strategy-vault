import { useEffect, useState } from "react";
import { NumField } from "./OperandEditor";
import { COPY, FUTURES_NOTE, MONEY_BASIS_HINT } from "./labels";
import { hasShorts, instrumentNotes, modeOf, moneyBlocked, sizingKind, supportsMoney, withMode, withSizingKind, type InstrumentContext, type SizingKind } from "./execution-model";
import { getAt, putAt, removeAt, setExecution } from "./spec-model";
import { Segmented } from "./ui";
import type { Catalogue, Spec } from "./types";
import "./risk.css";

const PERIODS: Record<string, string> = { W: "week", M: "month", Q: "quarter", Y: "year" };
const SIZE: Record<Exclude<SizingKind, "full">, string> = { pct_equity: "% of equity", risk_pct: "Risk %", units: "Shares", value: "Value" };
const sizeOptions = [{ value: "full", label: "All equity (default)" }, { value: "pct_equity", label: "% of equity" }, { value: "risk_pct", label: "Risk % per trade" }, { value: "units", label: "Shares" }, { value: "value", label: "Position value" }] as const;

/** Sets `execution.<path>` and removes objects the change leaves empty. */
function changeExecution(spec: Spec, path: string, value: unknown): Spec {
  let next = putAt(spec, `execution.${path}`, value);
  const parts = `execution.${path}`.split(".");
  for (let i = parts.length - 1; i >= 1; i--) {
    const at = parts.slice(0, i).join("."), node = getAt(next, at);
    if (node && typeof node === "object" && !Array.isArray(node) && Object.keys(node).length === 0) next = removeAt(next, at);
  }
  return next;
}
const limitInput = (spec: Spec, onChange: (spec: Spec) => void, label: string, key: string, min: number, max: number, step: number | null = null, placeholder = "none") =>
  <NumField label={label} value={getAt(spec.execution?.limits, key) as number | undefined} min={min} max={max} step={step} placeholder={placeholder} onChange={(n) => onChange(changeExecution(spec, `limits.${key}`, n))} />;
const limitBox = (spec: Spec, onChange: (spec: Spec) => void, label: string, key: "shrink" | "prev_bar_equity") => <label className="lab-check"><input type="checkbox" aria-label={label}
  checked={spec.execution?.limits?.[key] === true} onChange={(e) => onChange(changeExecution(spec, `limits.${key}`, e.target.checked ? true : undefined))} /> {label}</label>;

/** AmiBroker Analysis settings › Portfolio: the volume limit and previous-bar equity (money mode on one instrument). */
export function PortfolioLimits({ spec, onChange }: { spec: Spec; onChange(spec: Spec): void }) {
  if (modeOf(spec) !== "money" || spec.portfolio) return <p className="cap">In Money mode on one instrument: volume limit and previous-bar equity.</p>;
  return <fieldset className="lab-fieldset lab-risk-limits" data-lab-portfolio-limits>
    <legend>Portfolio limits (AmiBroker)</legend>
    <div className="lab-row">
      {limitInput(spec, onChange, "Limit trade size as % of entry bar volume", "max_volume_pct", 0, 100)}
      {limitBox(spec, onChange, "Use previous bar equity for position sizing", "prev_bar_equity")}
    </div>
    <p className="cap">The volume limit reads the last complete bar when the order fills: the fill bar for a close fill, otherwise the bar before it, so it never looks ahead.</p>
  </fieldset>;
}

const REFERENCE: Record<string, string> = { in: "NIFTY 50", us: "SPX" };
export const TICK_NOTE = "Tick size: as in AmiBroker, only exits by the built-in stops round to the tick; max loss, trailing and breakeven against you (sells down, covers up), profit targets and take-profits toward their level. Entries and other exits keep their price; stop levels are tested unrounded. Futures use the contract tick.";
export const PAD_NOTE = "Pad and align: bars follow the reference's trading days within this instrument's own dates. A missing day repeats the previous close (O=H=L=C, volume 0) and is an ordinary bar to rules and fills, as in AmiBroker; days the reference lacks are dropped.";
export const MARGIN_NOTE = "Account margin: buying power = equity × 100 / margin %. The part of a long position above the account's cash is a loan: Margin rate [%] is charged on it per year, by calendar day (0 = no charge). Shorts borrow no cash. There is no margin call; idle-cash interest is earned only on cash not in positions.";

/** AmiBroker General › Tick size and Pad and align all data to reference symbol (one instrument, unit or money mode). */
export function GeneralData({ spec, instrument, onChange }: { spec: Spec; instrument: InstrumentContext; onChange(spec: Spec): void }) {
  const ex = spec.execution ?? {}, reference = REFERENCE[instrument.market];
  const futures = (ex.instrument as { kind?: string } | undefined)?.kind === "futures";
  return <>
    <div className="lab-row" data-lab-general-data>
      <NumField label="Tick size" value={futures ? undefined : ex.tick_size} min={0} max={1e6} placeholder={futures ? "contract" : "0"} disabled={futures}
        onChange={(n) => onChange(changeExecution(spec, "tick_size", n || undefined))} />
      <label className="lab-check" title={reference ? undefined : "No reference index on this market"}><input type="checkbox" aria-label="Pad and align all data to reference symbol" disabled={!reference && !ex.pad_align}
        checked={ex.pad_align === true} onChange={(e) => onChange(changeExecution(spec, "pad_align", e.target.checked ? true : undefined))} /> Pad and align all data to reference symbol</label>
      <span className="cap" data-lab-reference>Reference symbol: {reference ?? "none on this market"}</span>
    </div>
    {!!ex.tick_size && <p className="cap" data-lab-tick-note>{TICK_NOTE}</p>}
    {ex.pad_align === true && <p className="cap" data-lab-pad-note>{PAD_NOTE}</p>}
  </>;
}

/** AmiBroker General › Initial equity, first in the tab: editable in money mode, otherwise disabled with the reason. */
/** `instrumentData={false}` leaves out tick size and pad & align (shown under Trading, or not at all in the Simple view). */
export function InitialEquity({ spec, catalogue, instrument, onChange, instrumentData = true }: { spec: Spec; catalogue: Catalogue; instrument: InstrumentContext; onChange(spec: Spec): void; instrumentData?: boolean }) {
  if (!catalogue.execution) return null;
  const money = modeOf(spec) === "money";
  const blocked = moneyBlocked(instrument, spec) ?? (!supportsMoney(catalogue) ? "Money mode is not supported by this engine yet." : null);
  return <>
    <div className="lab-row" data-lab-equity>
      <NumField label="Initial equity" value={money ? spec.execution?.capital : undefined} min={0} max={1e12} disabled={!money} placeholder={money ? undefined : "off"}
        onChange={(n) => n !== undefined && onChange(changeExecution(spec, "capital", n))} />
      <Segmented label="Results in" value={modeOf(spec)} options={[{ value: "unit", label: "% returns", title: "Results as percentage returns on a unit position" }, { value: "money", label: "Cash amounts", disabled: !!blocked, title: blocked ?? "Results in cash: starting equity, position sizes, costs" }]}
        onChange={(m) => onChange(withMode(spec, m))} />
    </div>
    {blocked && <p className="cap" data-lab-money-blocked>{blocked}</p>}
    {!spec.portfolio && instrumentData && <GeneralData spec={spec} instrument={instrument} onChange={onChange} />}
  </>;
}

export function RiskPanel({ spec, catalogue, instrument, onChange }: { spec: Spec; catalogue: Catalogue; instrument: InstrumentContext; onChange(spec: Spec): void }) {
  const ex = spec.execution ?? {};
  const active = [ex.sizing, ex.capital, ex.cooldown, ex.max_trades, ex.risk, ex.shadow].some((x) => x !== undefined);
  const [open, setOpen] = useState(active);
  useEffect(() => { if (active) setOpen(true); }, [active]);
  if (!catalogue.execution) return null;
  const cap = catalogue.execution;
  const kind = sizingKind(spec), money = modeOf(spec) === "money";
  const rawTaxes = ex.costs?.taxes;
  const editableTaxes = rawTaxes === undefined || (Array.isArray(rawTaxes) && rawTaxes.every((t) => t && typeof t === "object" && typeof t.name === "string" && typeof t.pct === "number" && typeof t.side === "string"));
  const taxes = editableTaxes && Array.isArray(rawTaxes) ? rawTaxes : [];
  const notes = instrumentNotes(instrument, spec);
  const summary: string[] = [];
  if (money) summary.push(`money · ${ex.capital?.toLocaleString("en-US")}`);
  if (kind !== "full") {
    const value = getAt(ex.sizing, kind);
    summary.push(kind === "risk_pct" ? `risk ${value}% per trade` : kind === "pct_equity" ? `${value}% of equity` : kind === "units" ? `${value} shares` : `value ${value}`);
  }
  if (ex.cooldown) summary.push(`cooldown ${ex.cooldown.bars} bars`);
  if (ex.max_trades) summary.push(`max ${ex.max_trades.count} per ${PERIODS[ex.max_trades.per] ?? ex.max_trades.per}`);
  if (ex.risk?.max_consecutive_losses !== undefined) summary.push(`pause after ${ex.risk.max_consecutive_losses} losses`);
  if (ex.risk?.max_drawdown_pct !== undefined) summary.push(`stop at ${ex.risk.max_drawdown_pct}% drawdown`);
  if (ex.shadow) summary.push(`trade after a ${ex.shadow.loss_pct}% paper loss`);
  const change = (path: string, value: unknown) => onChange(changeExecution(spec, path, value));
  const patchRisk = (patch: Record<string, number | undefined>) => {
    const risk = { ...ex.risk, ...patch } as Record<string, unknown>;
    for (const k of Object.keys(risk)) if (risk[k] === undefined) delete risk[k];
    change("risk", Object.keys(risk).length ? risk : undefined);
  };
  const limitField = (label: string, key: string, min: number, max: number, step: number | null = null, placeholder?: string) => limitInput(spec, onChange, label, key, min, max, step, placeholder);
  const costField = (label: string, path: string, max: number) => <NumField label={label} value={getAt(ex.costs, path) as number | undefined} min={0} max={max} placeholder="0"
    onChange={(n) => change(`costs.${path}`, n)} />;
  return <details className="lab-risk" data-lab-risk open={open} onToggle={(e) => setOpen(e.currentTarget.open)}>
    <summary>Position size, re-entry and risk <small>{summary.join(" · ") || "off"}</small></summary>
    <div className="lab-risk-body">
      <div className="lab-row">
        <label className="dfield">Size<select aria-label="Size" value={kind} onChange={(e) => onChange(withSizingKind(spec, e.target.value as SizingKind))}>
          {sizeOptions.map((o) => <option key={o.value} value={o.value} disabled={(o.value === "units" || o.value === "value") && (!money || !cap.sizing.includes(o.value))}>{spec.portfolio&&o.value==="full"?"Equal allocation (1 / Max positions)":o.label}</option>)}
        </select></label>
        {kind !== "full" && <NumField label={SIZE[kind]} value={getAt(ex.sizing, kind) as number | undefined} min={kind === "units" ? 1 : 0}
          max={kind === "units" ? 1e9 : kind === "value" ? 1e12 : 100} step={kind === "units" ? 1 : null}
          onChange={(n) => n !== undefined && change(`sizing.${kind}`, n)} />}
      </div>
      {money && <>
        <div className="lab-row">
          {costField("Commission per order", "commission.per_order", 1e9)}
          {costField("Commission per share", "commission.per_unit", 1e6)}
          {costField("Commission %", "commission.pct", 10)}
          {costField("Slippage (bps)", "slippage.bps", 1000)}
          {hasShorts(spec) && instrument.market === "us" && costField("Borrow % per year", "borrow_pct_year", 100)}
        </div>
        <div className="lab-risk-taxes">
          {!editableTaxes && <span className="lab-kept">{COPY.keptAsIs} <code>{JSON.stringify(rawTaxes)}</code></span>}
          {taxes.map((tax, i) => <div className="lab-row" key={i}>
            <label className="dfield">Tax {i + 1}<input type="text" aria-label={`Tax ${i + 1} name`} value={tax.name} maxLength={40}
              onChange={(e) => change("costs.taxes", taxes.map((t, j) => i === j ? { ...t, name: e.target.value } : t))} /></label>
            <NumField label={`Tax ${i + 1} %`} value={tax.pct} min={0} max={10} onChange={(n) => n !== undefined && change("costs.taxes", taxes.map((t, j) => i === j ? { ...t, pct: n } : t))} />
            <label className="dfield">Side<select aria-label={`Tax ${i + 1} side`} value={tax.side}
              onChange={(e) => change("costs.taxes", taxes.map((t, j) => i === j ? { ...t, side: e.target.value } : t))}>
              <option value="buy">Buy</option><option value="sell">Sell</option><option value="both">Both</option>
            </select></label>
            <button type="button" className="dbtn sm ghost" aria-label={`Remove tax ${i + 1}`} onClick={() => change("costs.taxes", taxes.length === 1 ? undefined : taxes.filter((_, j) => i !== j))}>×</button>
          </div>)}
          {editableTaxes && taxes.length < 6 && <button type="button" className="dbtn sm" aria-label="Add tax" onClick={() => change("costs.taxes", [...taxes, { name: "Tax", pct: 0.1, side: "both" }])}>+ Tax</button>}
        </div>
        <p className="cap">{MONEY_BASIS_HINT}</p>
        {!spec.portfolio && <fieldset className="lab-fieldset lab-risk-limits" data-lab-limits>
          <legend>Position limits (AmiBroker)</legend>
          <div className="lab-row">
            {limitField("Min. shares", "min_shares", 0, 1e9)}
            {limitField("Round lot size", "round_lot", 1, 1e9, 1)}
            {limitField("Min. pos. value", "min_value", 0, 1e12)}
            {limitField("Max. pos. value", "max_value", 0, 1e12)}
            {limitField("Fixed annual interest [%]", "interest_pct", 0, 100, null, "0")}
            {limitField("Margin rate [%]", "margin_rate_pct", 0, 100, null, "0")}
            {limitField("Account margin [%]", "margin_pct", 1, 100, null, "100")}
          </div>
          <div className="lab-row">
            {limitBox(spec, onChange, "Allow position size shrinking", "shrink")}
          </div>
          {ex.limits?.margin_pct !== undefined && ex.limits.margin_pct < 100 && <p className="cap" data-lab-margin-note>{MARGIN_NOTE}</p>}
          <p className="cap">Volume limit and previous-bar equity are on the Portfolio tab.</p>
        </fieldset>}
      </>}
      <div className="lab-row">
        <NumField label="Cooldown (bars)" value={ex.cooldown?.bars} placeholder="off" min={1} max={cap.max_cooldown_bars ?? 1000} step={1}
          onChange={(n) => change("cooldown", n === undefined ? undefined : { ...ex.cooldown, bars: n })} />
        <label className="dfield">Cooldown after<select aria-label="Cooldown after" value={ex.cooldown?.after ?? "any"} disabled={!ex.cooldown}
          onChange={(e) => change("cooldown.after", e.target.value === "any" ? undefined : e.target.value)}>
          <option value="any">Any trade</option><option value="loss">A loss</option><option value="win">A win</option>
        </select></label>
        <NumField label="Max entries" value={ex.max_trades?.count} placeholder="off" min={1} max={cap.max_trades_count ?? 1000} step={1}
          onChange={(n) => change("max_trades", n === undefined ? undefined : { ...ex.max_trades, count: n, per: ex.max_trades?.per ?? "M" })} />
        <label className="dfield">Per<select aria-label="Max entries per" value={ex.max_trades?.per ?? "M"} disabled={!ex.max_trades}
          onChange={(e) => change("max_trades.per", e.target.value)}>
          {Object.entries(PERIODS).map(([key, label]) => <option key={key} value={key}>{label}</option>)}
        </select></label>
      </div>
      <div className="lab-row">
        <NumField label="Losses in a row" value={ex.risk?.max_consecutive_losses} placeholder="off" min={1} max={cap.max_consecutive_losses ?? 1000} step={1}
          onChange={(n) => patchRisk({ max_consecutive_losses: n, pause_bars: n === undefined ? undefined : ex.risk?.pause_bars ?? 20 })} />
        <NumField label="Pause (bars)" value={ex.risk?.pause_bars} min={1} max={cap.max_pause_bars ?? 100000} step={1} onChange={(n) => patchRisk({ pause_bars: n })} />
        <NumField label="Stop new entries at drawdown (%)" value={ex.risk?.max_drawdown_pct} placeholder="off" min={0} max={cap.max_drawdown_pct_below ?? 100} onChange={(n) => patchRisk({ max_drawdown_pct: n })} />
      </div>
      <ShadowFields spec={spec} onChange={onChange} />
      {notes.length > 0 && <p className="cap">{notes.join(" ")}</p>}
      <p className="cap">{FUTURES_NOTE}</p>
    </div>
  </details>;
}

type Shadow = NonNullable<NonNullable<Spec["execution"]>["shadow"]>;
const UNTIL: [NonNullable<Shadow["until"]>, string][] = [["trade_profit", "One real trade makes"], ["total_profit", "Real trades together make"], ["trades", "After this many real trades"]];
/** execution.shadow: paper-trade the rules, trade for real only after a losing streak, back to paper once the target is met. */
function ShadowFields({ spec, onChange }: { spec: Spec; onChange(spec: Spec): void }) {
  const sh = spec.execution?.shadow;
  const until = sh?.until ?? "trade_profit";
  const put = (patch: Partial<Shadow> | undefined) => onChange(setExecution(spec, patch === undefined ? { shadow: undefined } : { shadow: { ...(sh as Shadow), ...patch } }));
  return <fieldset className="lab-shadow" data-lab-shadow>
    <legend>Trade after a losing streak</legend>
    <label className="lab-check" title={spec.portfolio ? "A single-instrument setting" : undefined}><input type="checkbox" checked={!!sh} disabled={!!spec.portfolio || (!sh && !!spec.short_side)}
      onChange={(e) => onChange(setExecution(spec, e.target.checked ? { shadow: { loss_pct: 5, min_bars: 5, until: "trade_profit", profit_pct: 3 }, fill: "close" } : { shadow: undefined }))} />
      Paper-trade the rules first; trade for real only after they lose</label>
    {sh && <div className="lab-row">
      <NumField label="Paper loss (%)" value={sh.loss_pct} min={0} max={1000} onChange={(n) => n !== undefined && n > 0 && put({ loss_pct: n })} />
      <NumField label="Days in paper trades (at least)" value={sh.min_bars ?? 0} min={0} max={100000} step={1} onChange={(n) => n !== undefined && Number.isInteger(n) && n >= 0 && put({ min_bars: n })} />
      <label className="dfield">Back to paper when<select aria-label="Back to paper when" value={until}
        onChange={(e) => { const u = e.target.value as NonNullable<Shadow["until"]>; put(u === "trades" ? { until: u, trades: sh.trades ?? 1, profit_pct: undefined } : { until: u, profit_pct: sh.profit_pct ?? 3, trades: undefined }); }}>
        {UNTIL.map(([v, t]) => <option key={v} value={v}>{t}</option>)}
      </select></label>
      {until === "trades"
        ? <NumField label="Real trades" value={sh.trades ?? 1} min={1} max={1000} step={1} onChange={(n) => n !== undefined && Number.isInteger(n) && n >= 1 && put({ trades: n })} />
        : <NumField label="Profit (%)" value={sh.profit_pct ?? 3} min={0} max={1000} onChange={(n) => n !== undefined && n > 0 && put({ profit_pct: n })} />}
    </div>}
    {sh && <p className="cap">{SHADOW_NOTE}</p>}
  </fieldset>;
}
const SHADOW_NOTE = "The entry and exit rules run all the time as paper trades. Their % returns are added up (a total above 0 starts again from 0). When the total is below minus the paper loss and the paper trades have spent at least the given days in a position, the next signals become real trades. Rules on the Filters tab gate only the real trades: a signal they block is bought later, on the first close they allow while that signal's trade is still running; tick \"exit when false\" on a filter to sell when it fails. Entries and exits fill at the close. As the AmiBroker \"Long-Loss DD with N Days\" AFL.";
