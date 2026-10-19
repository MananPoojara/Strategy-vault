import {useEffect, useId, useRef, useState} from "react";
import {createPortal} from "react-dom";
import {sourceAccountCopy,type SourceAccountView} from "../services/source-account";
import type {ReturnDisplayMode} from "../services/return-display";
import {UNIT_CLOSED_BASIS} from '../services/unit-exposure-presenter';
import "./StrategySettings.css";

export function StrategySettings({mode = "fixed", onApply, disabledReason, sourceAccount = false, unitExposure = false}: {
  mode?: ReturnDisplayMode;
  onApply: (mode: ReturnDisplayMode) => void;
  /** Portfolio reports have their own chronological accounting; they must not
   * multiply the returns of overlapping positions in closing order. */
  disabledReason?: string;
  sourceAccount?: boolean | SourceAccountView;
  unitExposure?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState<ReturnDisplayMode>(mode);
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
      onClick={() => {setDraft(mode); setOpen(true);}}>
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
          <p id={`${id}-description`} className="strategy-settings-intro">Choose how closed-trade returns are displayed.</p>
          <fieldset className="strategy-settings-group" data-settings-group="return-display" disabled={Boolean(disabledReason)}>
            <legend>{unitExposure ? 'Closed unit-return display' : sourceAccount ? "Closed weighted-return display" : "Net Profit calculation"}</legend>
            <label className={`strategy-settings-option ${draft === "fixed" ? "selected" : ""}`}>
              <input type="radio" name={`${id}-return-mode`} value="fixed" checked={draft === "fixed"} onChange={() => setDraft("fixed")} />
              <span><strong>{unitExposure || sourceAccount ? "Closed return sum" : "Fixed quantity"}</strong><small>{unitExposure
                ? 'Add audited closed net fractions; no funded share quantity is assumed.' : sourceAccount
                ? "Add accepted closed net fractions of each actual entry equity."
                : "Additive trade-return view. No share quantity supplied."}</small></span>
            </label>
            <label className={`strategy-settings-option ${draft === "compound" ? "selected" : ""}`}>
              <input type="radio" name={`${id}-return-mode`} value="compound" checked={draft === "compound"} onChange={() => setDraft("compound")} />
              <span><strong>{unitExposure ? 'Compounded closed model' : sourceAccount ? "Hypothetical closed compounding" : "Compounding"}</strong><small>{unitExposure
                ? 'Use engine equity at the last closed exit; terminal-open marks stay excluded.' : sourceAccount
                ? "Multiply accepted closed weighted-return factors; this does not resize positions."
                : "Multiply each closed trade’s net return factor."}</small></span>
            </label>
          </fieldset>
          <p className="strategy-settings-note">{disabledReason || (unitExposure ? UNIT_CLOSED_BASIS : sourceAccount
            ? sourceAccountCopy(typeof sourceAccount === "object" ? sourceAccount : {model:"elder_protected_impulse_fractional_cash_v1"}).settingsNote
            : "Percentage report using closed trades only. Open-position marks are excluded.")}</p>
          <footer className="strategy-settings-actions">
            <button type="button" onClick={close}>Cancel</button>
            <button type="button" className="strategy-settings-apply" disabled={Boolean(disabledReason)} onClick={() => {close(); onApply(draft);}}>Apply</button>
          </footer>
        </div>
      </div>, document.body,
    )}
  </>;
}
