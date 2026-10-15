import { useEffect, useState } from "react";
import { NumField } from "./OperandEditor";
import { COPY, FUTURES_NOTE, MONEY_BASIS_HINT } from "./labels";
import { hasShorts, instrumentNotes, modeOf, moneyBlocked, sizingKind, supportsMoney, withMode, withSizingKind, type InstrumentContext, type SizingKind } from "./execution-model";
import { getAt, putAt, removeAt } from "./spec-model";
import { Segmented } from "./ui";
import type { Catalogue, Spec } from "./types";
import "./risk.css";

const PERIODS: Record<string, string> = { W: "week", M: "month", Q: "quarter", Y: "year" };
const SIZE: Record<Exclude<SizingKind, "full">, string> = { pct_equity: "% of equity", risk_pct: "Risk %", units: "Shares", value: "Value" };
const sizeOptions = [{ value: "full", label: "All equity (default)" }, { value: "pct_equity", label: "% of equity" }, { value: "risk_pct", label: "Risk % per trade" }, { value: "units", label: "Shares" }, { value: "value", label: "Position value" }] as const;

export function RiskPanel({ spec, catalogue, instrument, onChange }: { spec: Spec; catalogue: Catalogue; instrument: InstrumentContext; onChange(spec: Spec): void }) {
  const ex = spec.execution ?? {};
  const active = [ex.sizing, ex.capital, ex.cooldown, ex.max_trades, ex.risk].some((x) => x !== undefined);
  const [open, setOpen] = useState(active);
  useEffect(() => { if (active) setOpen(true); }, [active]);
  if (!catalogue.execution) return null;
  const cap = catalogue.execution;
  const mode = modeOf(spec), kind = sizingKind(spec), money = mode === "money";
  const blocked = moneyBlocked(instrument, spec) ?? (!supportsMoney(catalogue) ? "Money mode is not supported by this engine yet." : null);
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
  const change = (path: string, value: unknown) => {
    let next = putAt(spec, `execution.${path}`, value);
    const parts = `execution.${path}`.split(".");
    for (let i = parts.length - 1; i >= 1; i--) {
      const at = parts.slice(0, i).join("."), node = getAt(next, at);
      if (node && typeof node === "object" && !Array.isArray(node) && Object.keys(node).length === 0) next = removeAt(next, at);
    }
    onChange(next);
  };
  const patchRisk = (patch: Record<string, number | undefined>) => {
    const risk = { ...ex.risk, ...patch } as Record<string, unknown>;
    for (const k of Object.keys(risk)) if (risk[k] === undefined) delete risk[k];
    change("risk", Object.keys(risk).length ? risk : undefined);
  };
  const costField = (label: string, path: string, max: number) => <NumField label={label} value={getAt(ex.costs, path) as number | undefined} min={0} max={max}
    onChange={(n) => change(`costs.${path}`, n)} />;
  return <details className="lab-risk" data-lab-risk open={open} onToggle={(e) => setOpen(e.currentTarget.open)}>
    <summary>Position size, re-entry and risk <small>{summary.join(" · ") || "off"}</small></summary>
    <div className="lab-risk-body">
      <div className="lab-row">
        <Segmented label="Results in" value={mode} options={[{ value: "unit", label: "Unit study" }, { value: "money", label: "Money", disabled: !!blocked, title: blocked ?? undefined }]}
          onChange={(m) => onChange(withMode(spec, m))} />
        {money && <NumField label="Capital" value={ex.capital} min={0} max={1e12} onChange={(n) => n !== undefined && change("capital", n)} />}
        <label className="dfield">Size<select aria-label="Size" value={kind} onChange={(e) => onChange(withSizingKind(spec, e.target.value as SizingKind))}>
          {sizeOptions.map((o) => <option key={o.value} value={o.value} disabled={(o.value === "units" || o.value === "value") && (!money || !cap.sizing.includes(o.value))}>{spec.portfolio&&o.value==="full"?"Equal allocation (1 / Max positions)":o.label}</option>)}
        </select></label>
        {kind !== "full" && <NumField label={SIZE[kind]} value={getAt(ex.sizing, kind) as number | undefined} min={kind === "units" ? 1 : 0}
          max={kind === "units" ? 1e9 : kind === "value" ? 1e12 : 100} step={kind === "units" ? 1 : null}
          onChange={(n) => n !== undefined && change(`sizing.${kind}`, n)} />}
      </div>
      {blocked && <p className="cap" data-lab-money-blocked>{blocked}</p>}
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
      </>}
      <div className="lab-row">
        <NumField label="Cooldown (bars)" value={ex.cooldown?.bars} min={1} max={cap.max_cooldown_bars ?? 1000} step={1}
          onChange={(n) => change("cooldown", n === undefined ? undefined : { ...ex.cooldown, bars: n })} />
        <label className="dfield">Cooldown after<select aria-label="Cooldown after" value={ex.cooldown?.after ?? "any"} disabled={!ex.cooldown}
          onChange={(e) => change("cooldown.after", e.target.value === "any" ? undefined : e.target.value)}>
          <option value="any">Any trade</option><option value="loss">A loss</option><option value="win">A win</option>
        </select></label>
        <NumField label="Max entries" value={ex.max_trades?.count} min={1} max={cap.max_trades_count ?? 1000} step={1}
          onChange={(n) => change("max_trades", n === undefined ? undefined : { ...ex.max_trades, count: n, per: ex.max_trades?.per ?? "M" })} />
        <label className="dfield">Per<select aria-label="Max entries per" value={ex.max_trades?.per ?? "M"} disabled={!ex.max_trades}
          onChange={(e) => change("max_trades.per", e.target.value)}>
          {Object.entries(PERIODS).map(([key, label]) => <option key={key} value={key}>{label}</option>)}
        </select></label>
      </div>
      <div className="lab-row">
        <NumField label="Losses in a row" value={ex.risk?.max_consecutive_losses} min={1} max={cap.max_consecutive_losses ?? 1000} step={1}
          onChange={(n) => patchRisk({ max_consecutive_losses: n, pause_bars: n === undefined ? undefined : ex.risk?.pause_bars ?? 20 })} />
        <NumField label="Pause (bars)" value={ex.risk?.pause_bars} min={1} max={cap.max_pause_bars ?? 100000} step={1} onChange={(n) => patchRisk({ pause_bars: n })} />
        <NumField label="Stop new entries at drawdown (%)" value={ex.risk?.max_drawdown_pct} min={0} max={cap.max_drawdown_pct_below ?? 100} onChange={(n) => patchRisk({ max_drawdown_pct: n })} />
      </div>
      {notes.length > 0 && <p className="cap">{notes.join(" ")}</p>}
      <p className="cap">{FUTURES_NOTE}</p>
    </div>
  </details>;
}
