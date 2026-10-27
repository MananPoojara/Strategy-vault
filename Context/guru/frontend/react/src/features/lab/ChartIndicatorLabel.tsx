import type { RefCallback } from "react";
import "./ChartIndicatorLabel.css";

export interface ChartIndicatorLabelProps {
  name: string; settings?: string; settingsTitle?: string; color: string; hidden?: boolean;
  onSettings(): void; onToggle(): void; onRemove(): void;
  valuesRef?: RefCallback<HTMLSpanElement>;
}

/** Actions stay mounted while the chart updates only the values span. */
export function ChartIndicatorLabel({ name, settings, settingsTitle, color, hidden = false, onSettings, onToggle, onRemove, valuesRef }: ChartIndicatorLabelProps) {
  return <div className={`lab-indicator-label${hidden ? " lab-indicator-label-hidden" : ""}`}>
    <button type="button" className="lab-indicator-name" aria-label={`Edit ${name} indicator`} title={name} onClick={onSettings}>{name}</button>
    {settings && <span className="lab-indicator-settings" title={settingsTitle ?? settings}>{settings}</span>}
    <span className="lab-indicator-values" aria-label={`${name} values`} ref={valuesRef} style={{ color: hidden ? "var(--ink2)" : color }} />
    <span className="lab-indicator-actions">
      <button type="button" aria-label={`${hidden ? "Show" : "Hide"} ${name}`} aria-pressed={hidden} title={hidden ? "Show indicator" : "Hide indicator"} onClick={onToggle}>
        <svg width="16" height="16" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.4" aria-hidden="true">
          <path d="M2 10s3-5 8-5 8 5 8 5-3 5-8 5-8-5-8-5Z" /><circle cx="10" cy="10" r="2.3" />{hidden && <path d="m3 3 14 14" />}
        </svg>
      </button>
      <button type="button" aria-label={`Indicator settings for ${name}`} title="Indicator settings" onClick={onSettings}>
        <svg width="16" height="16" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.4" aria-hidden="true"><path d="M3 5h14M3 10h14M3 15h14M7 2v6M13 7v6M8 12v6" /></svg>
      </button>
      <button type="button" aria-label={`Remove indicator ${name}`} title="Remove indicator" onClick={onRemove}>
        <svg width="16" height="16" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.4" aria-hidden="true"><path d="m5 5 10 10M15 5 5 15" /></svg>
      </button>
    </span>
  </div>;
}
