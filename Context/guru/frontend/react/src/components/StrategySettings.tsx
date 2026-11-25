import {useEffect, useId, useRef, useState, type ReactNode} from "react";
import {createPortal} from "react-dom";
import {sourceAccountCopy,type SourceAccountView} from "../services/source-account";
import type {ReturnDisplayMode} from "../services/return-display";
import {PRISTINE_CLOSED_BASIS} from '../services/pristine-presenter';
import {UNIT_CLOSED_BASIS} from '../services/unit-exposure-presenter';
import "./StrategySettings.css";

export interface StrategyParameter {key: string; label: string; def: number; min?: number; max?: number; step?: number; value: number}
/** Built by the runtime from the parameter schema, the selection and the run result. */
export interface StrategySettingsModel {
  params: StrategyParameter[];
  assumptions: [string, string][];
  /** Set when the reviewed configuration is fixed (source account / unit exposure): values are shown read-only. */
  fixedNote?: string;
  /** Money, costs and trade filters of this run (absent for a reviewed fixed account). */
  backtest?: BacktestSettings | null;
}
export interface BacktestSettings {
  currency: string; equity: number; sizeMode: ReturnDisplayMode; sizePct: number; alloc: number;
  /** % per side */ brokerage: number; /** bps per side */ slippage: number; /** bps per actual futures roll */ rollover: number; deadStock: boolean; penny: number;
  maxPositions: number; basket: boolean;
}
type Draft = Record<"equity" | "sizeMode" | "sizePct" | "alloc" | "brokerage" | "slippage" | "rollover" | "deadStock" | "penny", string>;
const draftOf = (b: BacktestSettings): Draft => ({equity: String(b.equity), sizeMode: b.sizeMode, sizePct: String(b.sizePct), alloc: String(b.alloc),
  brokerage: String(b.brokerage), slippage: String(b.slippage), rollover: "0", deadStock: b.deadStock ? "1" : "0", penny: String(b.penny)});
const BOOK_DRAFT = (b: BacktestSettings): Draft => ({...draftOf(b), sizeMode: "fixed", sizePct: "100", alloc: String(b.equity), brokerage: "0", slippage: "0", rollover: "0", deadStock: "0", penny: "0"});
const num = (raw: string, lo: number, hi: number, open = false) => raw.trim() !== "" && Number.isFinite(+raw) && (open ? +raw > lo : +raw >= lo) && +raw <= hi;
/** The first problem in a draft, by field; empty when every value can be run. */
export function draftErrors(d: Draft): Partial<Record<keyof Draft, string>> {
  const e: Partial<Record<keyof Draft, string>> = {};
  if (!num(d.equity, 0, 1e12, true)) e.equity = "Enter an amount above 0.";
  if (d.sizeMode === "compound" && !num(d.sizePct, 0, 100, true)) e.sizePct = "Enter a percentage above 0 and at most 100.";
  if (d.sizeMode === "fixed" && !num(d.alloc, 0, 1e12, true)) e.alloc = "Enter an amount above 0.";
  else if (d.sizeMode === "fixed" && !e.equity && +d.alloc > +d.equity) e.alloc = "At most the initial equity (no borrowing).";
  if (!num(d.brokerage, 0, 5)) e.brokerage = "Enter 0 to 5%.";
  if (!num(d.slippage, 0, 500)) e.slippage = "Enter 0 to 500 bps.";
  if (!num(d.rollover, 0, 500)) e.rollover = "Enter 0 to 500 bps.";
  if (!num(d.penny, 0, 1e6)) e.penny = "Enter a price of 0 or more.";
  return e;
}
/** URL-state keys of a draft. Brokerage is stored as the engine's fee in bps per side. */
export function draftPatch(d: Draft): Record<string, string> {
  return {report_mode: d.sizeMode, equity: String(+d.equity), size_pct: String(+d.sizePct), alloc: String(+d.alloc),
    cost: String(+(+d.brokerage * 100).toFixed(6)), slippage: String(+d.slippage), rollover: "0", dead_stock: d.deadStock, penny: String(+d.penny)};
}
const RERUN: (keyof Draft)[] = ["brokerage", "slippage", "rollover", "deadStock", "penny"];
const valid = (p: StrategyParameter, raw: string) => raw.trim() !== "" && Number.isFinite(+raw)
  && (p.min === undefined || +raw >= p.min) && (p.max === undefined || +raw <= p.max);
/** Same encoding as the optimizer's Apply: every key when any differs from the book, empty for the book rule. */
export function encodeSettings(params: StrategyParameter[], values: Record<string, number>) {
  return params.some(p => values[p.key] !== p.def) ? params.map(p => `${p.key}:${values[p.key]}`).join(",") : "";
}

export function StrategySettings({mode = "fixed", onApply, disabledReason, sourceAccount = false, unitExposure = false, pristine = false, settings: fullSettings, hideParameters = false}: {
  mode?: ReturnDisplayMode;
  /** Hide the parameter list (none of the current callers do). */
  hideParameters?: boolean;
  /** `params` is passed only when the strategy settings changed (the encoded S.p); `patch` carries the backtest settings. */
  onApply: (mode: ReturnDisplayMode, params?: string, patch?: Record<string, string>) => void;
  settings?: StrategySettingsModel | null;
  /** Portfolio reports have their own chronological accounting; they must not
   * multiply the returns of overlapping positions in closing order. */
  disabledReason?: string;
  sourceAccount?: boolean | SourceAccountView;
  unitExposure?: boolean;
  /** Pristine reads its own audited closed fields, not the index adapter. */
  pristine?: boolean;
}) {
  const settings = fullSettings && hideParameters ? {...fullSettings, params: []} : fullSettings;
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState<ReturnDisplayMode>(mode);
  const params = settings?.params || [];
  const current = () => Object.fromEntries(params.map(p => [p.key, String(p.value)]));
  const [values, setValues] = useState<Record<string, string>>(current);
  const editable = !settings?.fixedNote;
  const invalid = params.some(p => !valid(p, values[p.key] ?? ""));
  const encoded = invalid ? null : encodeSettings(params, Object.fromEntries(params.map(p => [p.key, +values[p.key]])));
  const paramsChanged = encoded !== null && encoded !== encodeSettings(params, Object.fromEntries(params.map(p => [p.key, p.value])));
  const bt = settings?.backtest || null;
  const [draftBt, setDraftBt] = useState<Draft | null>(bt && draftOf(bt));
  const errors = draftBt ? draftErrors(draftBt) : {};
  const btChanged = Boolean(bt && draftBt && (Object.keys(draftBt) as (keyof Draft)[]).some(k => draftBt[k] !== draftOf(bt)[k]));
  const rerun = Boolean(bt && draftBt && RERUN.some(k => +draftBt[k] !== +draftOf(bt)[k]));
  const trigger = useRef<HTMLButtonElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const id = useId();
  const close = () => setOpen(false);

  useEffect(() => {
    if (!open) return;
    const initial = panel.current?.querySelector<HTMLElement>("input:checked:not(:disabled)")
      || panel.current?.querySelector<HTMLElement>("button:not([disabled])");
    initial?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault(); event.stopPropagation(); setOpen(false);
        return;
      }
      if (event.key !== "Tab" || !panel.current) return;
      const controls = [...panel.current.querySelectorAll<HTMLElement>(
        "button, input, select, [tabindex='0']",
      )].filter(control => !control.matches(":disabled"));
      const first = controls[0], last = controls.at(-1);
      if (!first || !last) return;
      if (!panel.current.contains(document.activeElement)
        || (event.shiftKey && document.activeElement === first)
        || (!event.shiftKey && document.activeElement === last)) {
        event.preventDefault(); (event.shiftKey ? last : first).focus();
      }
    };
    document.addEventListener("keydown", onKeyDown, true);
    return () => {
      document.removeEventListener("keydown", onKeyDown, true);
      if (trigger.current?.isConnected) trigger.current.focus();
    };
  }, [open]);

  return <>
    <button ref={trigger} type="button" className="strategy-settings-trigger"
      aria-label="Strategy settings" title="Strategy settings" aria-haspopup="dialog"
      aria-expanded={open} aria-controls={open ? `${id}-dialog` : undefined}
      onClick={() => {setDraft(mode); setValues(current()); setDraftBt(bt && draftOf(bt)); setOpen(true);}}>
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor"
        strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M21 4h-7M10 4H3M21 12h-9M8 12H3M21 20h-5M12 20H3M14 2v4M8 10v4M16 18v4" />
      </svg>
    </button>
    {open && createPortal(
      <div className="strategy-settings-backdrop" data-strategy-settings-backdrop
        onClick={event => {if (event.target === event.currentTarget) {event.stopPropagation(); close();}}}>
        <div ref={panel} id={`${id}-dialog`} className="strategy-settings-dialog"
          role="dialog" aria-modal="true" aria-labelledby={`${id}-title`}
          aria-describedby={`${id}-description`} tabIndex={-1}>
          <header className="strategy-settings-heading">
            <h2 id={`${id}-title`}>Strategy settings</h2>
            <button type="button" className="strategy-settings-close" aria-label="Close strategy settings" onClick={close}>
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                strokeWidth="1.75" strokeLinecap="round" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18" /></svg>
            </button>
          </header>
          <p id={`${id}-description`} className="strategy-settings-intro">{bt ? "Set the account, costs and filters for this backtest. Costs and filters re-run it on the same instrument and period." : settings && !hideParameters ? "The settings and assumptions this backtest used. Change a setting and Apply to re-run it on the same instrument and period." : settings ? "How this backtest calculates Net Profit, and the assumptions it used." : "Choose how closed-trade returns are displayed."}</p>
          {settings && !hideParameters && !bt && <section className="strategy-settings-group" data-settings-group="parameters" aria-labelledby={`${id}-params`}>
            <h3 id={`${id}-params`} className="strategy-settings-legend">Strategy settings used in this backtest</h3>
            {params.length ? <>
              <table className="strategy-settings-params">
                <thead><tr><th scope="col">Setting</th><th scope="col">Book default</th><th scope="col">Used</th></tr></thead>
                <tbody>{params.map(p => {
                  const raw = values[p.key] ?? "", ok = valid(p, raw), changed = ok && +raw !== p.def;
                  return <tr key={p.key} data-param={p.key} className={changed ? "changed" : undefined}>
                    <th scope="row"><label htmlFor={`${id}-p-${p.key}`}>{p.label}</label>
                      {changed && <span className="strategy-settings-marker" data-not-default>Not the book default</span>}
                      {!ok && <span className="strategy-settings-marker invalid" role="alert">Enter a number{p.min !== undefined && p.max !== undefined ? ` from ${p.min} to ${p.max}` : ""}</span>}</th>
                    <td className="n">{p.def}</td>
                    <td className="n"><input id={`${id}-p-${p.key}`} type="number" value={raw} readOnly={!editable}
                      min={p.min} max={p.max} step={p.step || "any"} aria-invalid={!ok || undefined}
                      onChange={event => setValues(v => ({...v, [p.key]: event.target.value}))} /></td>
                  </tr>;
                })}</tbody>
              </table>
              {settings.fixedNote ? <p className="strategy-settings-note">{settings.fixedNote}</p>
                : <button type="button" className="strategy-settings-reset" onClick={() => setValues(Object.fromEntries(params.map(p => [p.key, String(p.def)])))}>Reset to book defaults</button>}
            </> : <p className="strategy-settings-note">This rule has no adjustable settings; it runs exactly as written in the book.</p>}
          </section>}
          {bt && draftBt && !hideParameters && params.length > 0 && <fieldset className="bt-section bt-params" data-settings-group="parameters">
            <legend>Parameters <em>re-runs the backtest</em></legend>
            {params.map(p => {
              const raw = values[p.key] ?? "", ok = valid(p, raw), changed = ok && +raw !== p.def;
              return <div key={p.key} data-param={p.key} className={`bt-row${ok ? "" : " bad"}${changed ? " changed" : ""}`}>
                <label htmlFor={`${id}-p-${p.key}`}>{p.label}</label>
                <span className="bt-input"><input id={`${id}-p-${p.key}`} type="number" value={raw} min={p.min} max={p.max} step={p.step || "any"}
                  aria-invalid={!ok || undefined} onChange={event => setValues(v => ({...v, [p.key]: event.target.value}))} /></span>
                <small>{!ok ? <span role="alert">Enter a number{p.min !== undefined && p.max !== undefined ? ` from ${p.min} to ${p.max}` : ""}</span>
                  : changed ? <><b>Changed</b> · book default {p.def}</> : `Book default ${p.def}`}</small>
              </div>;
            })}
          </fieldset>}
          {bt && draftBt ? <BacktestForm id={id} bt={bt} draft={draftBt} errors={errors} set={(k, v) => setDraftBt(d => d && {...d, [k]: v})} /> : <>
          <fieldset className="strategy-settings-group" data-settings-group="return-display" disabled={Boolean(disabledReason)}>
            <legend>{pristine ? 'Closed Pristine-return display' : unitExposure ? 'Closed unit-return display' : sourceAccount ? "Closed weighted-return display" : "Net Profit calculation"}</legend>
            <label className={`strategy-settings-option ${draft === "fixed" ? "selected" : ""}`}>
              <input type="radio" name={`${id}-return-mode`} value="fixed" checked={draft === "fixed"} onChange={() => setDraft("fixed")} />
              <span><strong>{pristine || unitExposure || sourceAccount ? "Closed return sum" : "Fixed quantity"}</strong><small>{pristine
                ? 'Use the Pristine engine’s audited closed-return sum; no funded quantity is assumed.' : unitExposure
                ? 'Add audited closed net fractions; no funded share quantity is assumed.' : sourceAccount
                ? "Add accepted closed net fractions of each actual entry equity."
                : "Additive trade-return view. No share quantity supplied."}</small></span>
            </label>
            <label className={`strategy-settings-option ${draft === "compound" ? "selected" : ""}`}>
              <input type="radio" name={`${id}-return-mode`} value="compound" checked={draft === "compound"} onChange={() => setDraft("compound")} />
              <span><strong>{pristine || unitExposure ? 'Compounded closed model' : sourceAccount ? "Hypothetical closed compounding" : "Compounding"}</strong><small>{pristine
                ? 'Use the Pristine engine’s audited closed-compound return; terminal-open marks stay excluded.' : unitExposure
                ? 'Use engine equity at the last closed exit; terminal-open marks stay excluded.' : sourceAccount
                ? "Multiply accepted closed weighted-return factors; this does not resize positions."
                : "Multiply each closed trade’s net return factor."}</small></span>
            </label>
          </fieldset>
          <p className="strategy-settings-note">{disabledReason || (pristine ? PRISTINE_CLOSED_BASIS : unitExposure ? UNIT_CLOSED_BASIS : sourceAccount
            ? sourceAccountCopy(typeof sourceAccount === "object" ? sourceAccount : {model:"elder_protected_impulse_fractional_cash_v1"}).settingsNote
            : "Percentage report using closed trades only. Open-position marks are excluded.")}</p>
          {settings && <section className="strategy-settings-group" data-settings-group="assumptions" aria-labelledby={`${id}-assumptions`}>
            <h3 id={`${id}-assumptions`} className="strategy-settings-legend">Backtest assumptions</h3>
            <dl className="strategy-settings-assumptions">{settings.assumptions.map(([term, text]) => <div key={term}><dt>{term}</dt><dd>{text}</dd></div>)}</dl>
          </section>}
          </>}
          <footer className="strategy-settings-actions">
            {bt && draftBt && <>
              <button type="button" className="strategy-settings-link" onClick={() => {setDraftBt(BOOK_DRAFT(bt)); setValues(Object.fromEntries(params.map(p => [p.key, String(p.def)])));}}>Reset to defaults</button>
              <span className="strategy-settings-status" role="status">{Object.keys(errors).length ? "Fix the marked setting." : rerun || paramsChanged ? "Apply re-runs the backtest." : btChanged ? "Apply recalculates Net P&L." : ""}</span>
            </>}
            <button type="button" onClick={close}>Cancel</button>
            <button type="button" className="strategy-settings-apply" disabled={invalid || Object.keys(errors).length > 0 || (bt ? !btChanged && !paramsChanged : Boolean(disabledReason) && !paramsChanged)}
              onClick={() => {close(); const patch = bt && draftBt && btChanged ? draftPatch(draftBt) : undefined;
                if (patch) onApply(patch.report_mode as ReturnDisplayMode, paramsChanged ? encoded! : undefined, patch);
                else if (paramsChanged) onApply(draft, encoded!); else onApply(draft);}}>{bt && (rerun || paramsChanged) ? "Run backtest" : "Apply"}</button>
          </footer>
        </div>
      </div>, document.body,
    )}
  </>;
}

/** Money, costs and trade filters, one row per setting (user request 25 Nov 2026). */
function BacktestForm({id, bt, draft, errors, set}: {id: string; bt: BacktestSettings; draft: Draft; errors: Partial<Record<keyof Draft, string>>; set: (k: keyof Draft, v: string) => void}) {
  const sizing = bt.basket ? "Basket runs give each position an equal slot of the portfolio; set the number of positions on the page." : "";
  const filters = bt.basket ? "Not available for basket runs." : "";
  const row = (k: keyof Draft, label: string, help: string, control: ReactNode) => <div className={`bt-row${errors[k] ? " bad" : ""}`} data-bt={k}>
    <label htmlFor={`${id}-bt-${k}`}>{label}</label>{control}
    <small>{errors[k] ? <span role="alert">{errors[k]}</span> : help}</small></div>;
  const input = (k: keyof Draft, props: {min?: number; max?: number; step?: number | "any"; unit?: string; disabled?: boolean}) => <span className="bt-input">
    {props.unit === "cur" && <i aria-hidden="true">{bt.currency}</i>}
    <input id={`${id}-bt-${k}`} type="number" inputMode="decimal" value={draft[k]} min={props.min} max={props.max} step={props.step ?? "any"} disabled={props.disabled}
      aria-invalid={errors[k] ? true : undefined} onChange={e => set(k, e.target.value)} />
    {props.unit && props.unit !== "cur" && <i aria-hidden="true">{props.unit}</i>}</span>;
  return <div className="bt-form">
    <fieldset className="bt-section" disabled={Boolean(sizing)}>
      <legend>Account and position size <em>changes Net P&amp;L only</em></legend>
      {row("equity", "Initial equity", "The account you start with. Net P&L is shown in money from this.", input("equity", {min: 1, unit: "cur"}))}
      {row("sizeMode", "Position size", draft.sizeMode === "compound" ? "Profits are reinvested: each trade uses a share of the equity at that time." : "Profits are not reinvested: each trade uses the same amount.",
        <select id={`${id}-bt-sizeMode`} value={draft.sizeMode} onChange={e => set("sizeMode", e.target.value)}>
          <option value="compound">% of equity (compounding)</option><option value="fixed">Fixed amount per trade</option></select>)}
      {draft.sizeMode === "compound"
        ? row("sizePct", "% of equity per trade", "100% puts the whole account in each trade; the rest stays in cash.", input("sizePct", {min: 0.01, max: 100, unit: "%"}))
        : row("alloc", "Amount per trade", "Up to the initial equity. Equal to it means the full account in each trade.", input("alloc", {min: 1, unit: "cur"}))}
      <div className="bt-row" data-bt="max"><label htmlFor={`${id}-bt-max`}>Maximum positions</label>
        <span className="bt-input"><input id={`${id}-bt-max`} type="number" value={bt.maxPositions} disabled readOnly /></span>
        <small>{bt.basket ? "Set on the page for basket runs." : "One instrument holds one position at a time."}</small></div>
      {sizing && <p className="bt-locked">{sizing}</p>}
    </fieldset>
    <fieldset className="bt-section">
      <legend>Costs <em>re-runs the backtest</em></legend>
      {row("brokerage", "Brokerage", "Charged on the value traded, at entry and again at exit.", input("brokerage", {min: 0, max: 5, step: 0.01, unit: "% per side"}))}
      {row("slippage", "Slippage", "Fills this much worse than the rule's price, on both sides. 100 bps = 1%.", input("slippage", {min: 0, max: 500, step: 0.1, unit: "bps per side"}))}
      {row("rollover", "Rollover", "Not available: no futures roll dates are loaded, so the engine does not charge rolls (it refuses any value above 0).", input("rollover", {unit: "bps per roll", disabled: true}))}
    </fieldset>
    <fieldset className="bt-section" disabled={Boolean(filters)}>
      <legend>Trade filters <em>re-runs the backtest</em></legend>
      {row("deadStock", "Dead stock backtest", draft.deadStock === "1" ? "No entry on the last bar; a trade still open there is sold at that day's close and counted." : "A trade still open at the end stays open and is left out of the results.",
        <select id={`${id}-bt-deadStock`} value={draft.deadStock} onChange={e => set("deadStock", e.target.value)}><option value="0">No</option><option value="1">Yes</option></select>)}
      {row("penny", "Penny stock price", "Skips a trade when the close of the bar before its entry (the entry bar for a close fill) is at or below this price. 0 = off. A skipped trade is not replaced by a later signal.", input("penny", {min: 0, unit: "cur"}))}
      {filters && <p className="bt-locked">{filters}</p>}
    </fieldset>
  </div>;
}
