import {useEffect, useId, useRef, useState} from "react";
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
}
const valid = (p: StrategyParameter, raw: string) => raw.trim() !== "" && Number.isFinite(+raw)
  && (p.min === undefined || +raw >= p.min) && (p.max === undefined || +raw <= p.max);
/** One arrow click: the schema's step, else the precision of the book default (0.02 steps by 0.01), kept inside the limits. */
export function stepValue(p: StrategyParameter, raw: string, direction: 1 | -1) {
  const places = (n: number) => (String(n).split(".")[1] || "").length;
  const step = p.step || 10 ** -places(p.def);
  const base = raw.trim() !== "" && Number.isFinite(+raw) ? +raw : p.def;
  let next = +(base + direction * step).toFixed(Math.max(places(step), places(base)));
  if (p.min !== undefined) next = Math.max(p.min, next);
  if (p.max !== undefined) next = Math.min(p.max, next);
  return String(next);
}
const ARROW = {width: 9, height: 9, viewBox: "0 0 10 10", fill: "none", stroke: "currentColor", strokeWidth: 1.6, strokeLinecap: "round", strokeLinejoin: "round", "aria-hidden": true} as const;
/** Same encoding as the optimizer's Apply: every key when any differs from the book, empty for the book rule. */
export function encodeSettings(params: StrategyParameter[], values: Record<string, number>) {
  return params.some(p => values[p.key] !== p.def) ? params.map(p => `${p.key}:${values[p.key]}`).join(",") : "";
}

/** The strategy's parameters as a small strip on the page (user request 30 Oct 2026); Net Profit mode and assumptions stay in the settings popup. */
export function StrategyParameters({settings, onApply}: {settings?: StrategySettingsModel | null; onApply: (params: string) => void}) {
  const params = settings?.params || [];
  const id = useId();
  const current = () => Object.fromEntries(params.map(p => [p.key, String(p.value)]));
  const [values, setValues] = useState<Record<string, string>>(current);
  const applied = params.map(p => `${p.key}:${p.value}`).join(",");
  useEffect(() => setValues(current()), [applied]);
  if (!params.length) return null;
  const editable = !settings?.fixedNote;
  const invalid = params.some(p => !valid(p, values[p.key] ?? ""));
  const encoded = invalid ? null : encodeSettings(params, Object.fromEntries(params.map(p => [p.key, +values[p.key]])));
  const changed = encoded !== null && encoded !== encodeSettings(params, Object.fromEntries(params.map(p => [p.key, p.value])));
  const notDefault = params.some(p => values[p.key] !== String(p.def));
  return <section className="box strategy-params" data-strategy-params aria-label="Strategy parameters" title={settings?.fixedNote}>
    <strong className="strategy-params-title">Parameters</strong>
    <div className="strategy-params-list">{params.map(p => {
      const raw = values[p.key] ?? "", ok = valid(p, raw);
      return <div key={p.key} data-param={p.key} className={`strategy-param${ok && +raw !== p.def ? " changed" : ""}`}>
        <label htmlFor={`${id}-${p.key}`}>{p.label}</label>
        <span className="strategy-settings-stepper"><input id={`${id}-${p.key}`} type="number" value={raw} readOnly={!editable}
          min={p.min} max={p.max} step={p.step || "any"} aria-invalid={!ok || undefined} title={`Book default ${p.def}`}
          onChange={event => setValues(v => ({...v, [p.key]: event.target.value}))} />
          {editable && <span className="strategy-settings-arrows">
            <button type="button" tabIndex={-1} aria-label={`Increase ${p.label}`} onClick={() => setValues(v => ({...v, [p.key]: stepValue(p, v[p.key] ?? "", 1)}))}><svg {...ARROW}><path d="M2 6.5 5 3.5l3 3" /></svg></button>
            <button type="button" tabIndex={-1} aria-label={`Decrease ${p.label}`} onClick={() => setValues(v => ({...v, [p.key]: stepValue(p, v[p.key] ?? "", -1)}))}><svg {...ARROW}><path d="M2 3.5 5 6.5l3-3" /></svg></button>
          </span>}</span>
        {!ok && <span className="strategy-settings-marker invalid" role="alert">{p.min !== undefined && p.max !== undefined ? `${p.min} to ${p.max}` : "Enter a number"}</span>}
      </div>;
    })}</div>
    {editable && <div className="strategy-params-actions">
      <button type="button" disabled={!notDefault} onClick={() => setValues(Object.fromEntries(params.map(p => [p.key, String(p.def)])))}>Book defaults</button>
      <button type="button" className="strategy-settings-apply" disabled={!changed} onClick={() => onApply(encoded!)}>Apply</button>
    </div>}
  </section>;
}

export function StrategySettings({mode = "fixed", onApply, disabledReason, sourceAccount = false, unitExposure = false, pristine = false, settings: fullSettings, hideParameters = false}: {
  mode?: ReturnDisplayMode;
  /** The parameters are on the page in StrategyParameters; this popup keeps the Net Profit mode and the assumptions. */
  hideParameters?: boolean;
  /** `params` is passed only when the strategy settings changed (the encoded S.p). */
  onApply: (mode: ReturnDisplayMode, params?: string) => void;
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
        "button, input, [tabindex='0']",
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
      onClick={() => {setDraft(mode); setValues(current()); setOpen(true);}}>
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
          <p id={`${id}-description`} className="strategy-settings-intro">{settings && !hideParameters ? "The settings and assumptions this backtest used. Change a setting and Apply to re-run it on the same instrument and period." : settings ? "How this backtest calculates Net Profit, and the assumptions it used." : "Choose how closed-trade returns are displayed."}</p>
          {settings && !hideParameters && <section className="strategy-settings-group" data-settings-group="parameters" aria-labelledby={`${id}-params`}>
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
          <footer className="strategy-settings-actions">
            <button type="button" onClick={close}>Cancel</button>
            <button type="button" className="strategy-settings-apply" disabled={invalid || (Boolean(disabledReason) && !paramsChanged)}
              onClick={() => {close(); if (paramsChanged) onApply(draft, encoded!); else onApply(draft);}}>Apply</button>
          </footer>
        </div>
      </div>, document.body,
    )}
  </>;
}
