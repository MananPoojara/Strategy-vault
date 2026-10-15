import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { isSeries } from "./spec-model";
import { foreignEnabled, foreignOperandProblems, foreignOptions, foreignProblems, isPair, withForeignField, type ForeignField, type ForeignScope } from "./foreign-model";
import type { Operand } from "./types";
import "./foreign.css";

const ForeignContext = createContext<ForeignScope | null>(null);
/** Page supplies API-listed same-market choices; recursively nested operand editors share the scope. */
export function ForeignProvider({ value, children }: { value: ForeignScope; children: ReactNode }) {
  return <ForeignContext.Provider value={value}>{children}</ForeignContext.Provider>;
}
export const useForeignScope = () => useContext(ForeignContext);

/** Mount only beside series leaves. Formula children receive their own fields through the shared context. */
export function ForeignFields({ operand, onChange, label = "Value" }: { operand: Operand; onChange(o: Operand): void; label?: string }) {
  const scope = useForeignScope();
  const x = operand as Record<string, unknown>;
  const present = !!operand && typeof operand === "object" && ["symbol", "vs", "fixup"].some((key) => Object.hasOwn(operand, key));
  const pair = isSeries(operand) && isPair(operand.s);
  const [open, setOpen] = useState(present || pair);
  useEffect(() => { if (present || pair) setOpen(true); }, [present, pair, x?.symbol, x?.vs, x?.fixup]);
  if (!scope || !isSeries(operand)) return null;
  if(scope.allowSymbol===false&&!present&&!pair)return null;
  const problems = foreignOperandProblems(operand, scope);
  const malformed = ["symbol", "vs", "fixup"].some((key) => Object.hasOwn(x, key) && typeof x[key] !== "string");
  if (operand.s === "bars_held" || !foreignEnabled(scope) || malformed) return present || pair
    ? <span className="lab-foreign-kept lab-kept">Foreign fields kept as is: <code>{JSON.stringify(operand)}</code>{problems.map((p, i) => <span key={i}>{p}</span>)}</span> : null;
  const options = foreignOptions(scope);
  const symbol = typeof x.symbol === "string" ? x.symbol : "", vs = typeof x.vs === "string" ? x.vs : "";
  const fixup = typeof x.fixup === "string" ? x.fixup : "none";
  const selected = (v: string) => options.find((o) => o.ticker === v)?.label ?? v;
  const change = (field: ForeignField, value: string) => onChange(withForeignField(operand, field, value || undefined));
  const choices = (value: string) => <>
    {value && !options.some((o) => o.ticker === value) && <option value={value}>Kept: {value}</option>}
    {options.map((o) => <option key={o.ticker} value={o.ticker} disabled={o.disabled} title={o.reason}>{o.label}</option>)}
  </>;
  const cap = scope.spec ? foreignProblems(scope.spec, scope).filter((p) => p.startsWith("The spec reads")) : [];
  const sourceSummary=`Source · ${symbol ? selected(symbol) : scope.allowSymbol===false||scope.portfolio?"current member":"current instrument"}${vs ? ` · compared with ${selected(vs)}` : pair ? " · choose comparison" : ""}`;
  return <details className="lab-foreign" data-lab-foreign open={open} onToggle={(e) => setOpen(e.currentTarget.open)}>
    <summary title={sourceSummary} aria-label={`${label} · ${sourceSummary}`}>{!open&&!present&&!pair?"Source":sourceSummary}</summary>
    <div className="lab-row">
      {(scope.allowSymbol!==false||!!symbol)&&<label className="dfield">Read from<select aria-label={`${label} · Read from`} value={symbol} onChange={(e) => change("symbol", e.target.value)}>
        <option value="">{scope.portfolio?"Current member":"Current instrument"}{!scope.portfolio&&scope.ticker ? ` (${scope.ticker})` : ""}</option>{scope.allowSymbol===false&&symbol?<option value={symbol}>Kept: {symbol}</option>:choices(symbol)}
      </select></label>}
      {(symbol || Object.hasOwn(x, "fixup")) && <label className="dfield">Missing dates<select aria-label={`${label} · Missing dates`} value={fixup}
        onChange={(e) => change("fixup", e.target.value === "none" ? "" : e.target.value)}>
        {!["none", "previous"].includes(fixup) && <option value={fixup}>Kept: {fixup}</option>}
        <option value="none">Keep gaps</option><option value="previous">Previous available value</option>
      </select></label>}
      {(pair || Object.hasOwn(x, "vs")) && <label className="dfield">Compared with<select aria-label={`${label} · Compared with`} value={vs} required={pair}
        onChange={(e) => change("vs", e.target.value)}>
        <option value="">{pair ? "Choose instrument…" : "No comparison"}</option>{choices(vs)}
      </select></label>}
    </div>
    {symbol && <span className="cap">Values use that instrument's own bars. The engine reports alignment; previous fills missing dates only.</span>}
    {[...problems, ...cap].map((p, i) => <span className="lab-foreign-problem cap" role="note" key={i}>{p}</span>)}
  </details>;
}
