import { Component, useEffect, useState, type ReactNode } from "react";
import { PROVISIONAL_NOTE } from "./labels";
import type { LabErrorFields, SavedMarket } from "./types";
import "./lab.css";

export const isProvisional = (m: SavedMarket) => m === "gl" || m === "crypto";

export class LabErrorBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null };
  static getDerivedStateFromError(error: Error) { return { error }; }
  render() {
    if (!this.state.error) return this.props.children;
    return (
      <div className="empty" role="alert">
        <b>This part of the page could not be shown</b>
        {this.state.error.message}{" "}
        <button type="button" className="dbtn sm" onClick={() => this.setState({ error: null })}>Try again</button>
      </div>
    );
  }
}

const HEADINGS: Record<string, string> = { needs_data: "Data needed", unsupported: "Not supported yet", invalid: "Rule problem", not_found: "Unknown request" };

export function ErrorNotice({ error, market, instrumentLabel, onRetry }: {
  error: unknown; market: SavedMarket; instrumentLabel?: string; onRetry?: () => void;
}) {
  const e = (error ?? {}) as Partial<LabErrorFields>;
  const message = error instanceof Error ? error.message : String(error);
  // A deadline plus a ticking interval stays correct when several seconds pass in one batch (and under fake timers).
  const deadline = () => (e.retryAfter ? Date.now() + e.retryAfter * 1000 : 0);
  const [until, setUntil] = useState(deadline);
  const [, setTick] = useState(0);
  useEffect(() => setUntil(deadline()), [error]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    if (!until) return;
    const id = setInterval(() => { setTick((t) => t + 1); if (Date.now() >= until) clearInterval(id); }, 250);
    return () => clearInterval(id);
  }, [until]);
  const wait = until ? Math.max(0, Math.ceil((until - Date.now()) / 1000)) : 0;
  const heading = (e.engineStatus && HEADINGS[e.engineStatus]) || (e.kind === "auth" ? "Not saved" : "Request refused");
  return (
    <div className="empty lab-error" role="alert" data-lab-error>
      <b>{heading}</b>
      <span>{message}</span>
      {isProvisional(market) && <ProvisionalStrip note={e.provisionalNote} instrumentLabel={instrumentLabel} />}
      {onRetry && e.retryAfter !== undefined && (
        <button type="button" className="dbtn sm" disabled={wait > 0} onClick={onRetry}>
          {wait > 0 ? `Try again in ${wait} s` : "Try again"}
        </button>
      )}
    </div>
  );
}

export const Loading = ({ text }: { text: string }) => <div className="loading" role="status">{text}</div>;

export interface SegmentOption<T extends string> { value: T; label: string; disabled?: boolean; title?: string }
export function Segmented<T extends string>({ label, value, options, onChange }: {
  label: string; value: T; options: SegmentOption<T>[]; onChange(value: T): void;
}) {
  return (
    <div className="join sm" role="group" aria-label={label}>
      {options.map((o) => (
        <button type="button" key={o.value} className={`join-item${o.value === value ? " active" : ""}`}
          aria-pressed={o.value === value} disabled={o.disabled} title={o.title} onClick={() => onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

export const ServerLabel = ({ text }: { text: string }) => <p className="lab-label" data-server-label>{text}</p>;

export const ProvisionalStrip = ({ note, instrumentLabel }: { note?: string; instrumentLabel?: string }) => (
  <div className="rnote lab-provisional" role="note" data-lab-provisional>
    {note || PROVISIONAL_NOTE}{instrumentLabel ? ` · ${instrumentLabel}` : ""}
  </div>
);

export const MetricTiles = ({ items }: { items: { label: string; value: string; note?: string }[] }) => (
  <div className="metrics">
    {items.map((item) => (
      <div className="metric" key={item.label}><span>{item.label}</span><b>{item.value}</b>{item.note && <i>{item.note}</i>}</div>
    ))}
  </div>
);
