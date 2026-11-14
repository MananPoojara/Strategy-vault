// Shared bits of the Indicators lab's results UI (branch Nandini, features/lab/ui.tsx), for the YT results.
import { useState, type ReactNode } from "react";
import "./lab.css";

/** Numeric table cell class: right-aligned, green/red by sign. */
export const tone = (v: number | null | undefined) => `n${typeof v === "number" && v > 0 ? " pos" : typeof v === "number" && v < 0 ? " neg" : ""}`;

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

/** A "?" that shows how a number is worked out on hover or keyboard focus. */
export const HelpTip = ({ name, text }: { name: string; text: string }) => (
  <button type="button" className="help-q" aria-label={`How ${name} is calculated: ${text}`}>?<em className="help-tip" role="tooltip">{text}</em></button>
);
/** `help`: how the number is worked out, behind the "?" in the tile's corner. */
export const MetricTiles = ({ items }: { items: { label: string; value: string; note?: string; help?: string }[] }) => (
  <div className="metrics">
    {items.map((item) => (
      <div className="metric" key={item.label}><span className={item.help ? "metric-h" : undefined}>{item.label}{item.help && <HelpTip name={item.label} text={item.help} />}</span>
        <b>{item.value}</b>{item.note && <i>{item.note}</i>}</div>
    ))}
  </div>
);

/** The Results box's sub-tabs (the lab's SettingsTabs): one view at a time instead of a long scroll. */
export function ResultTabs({ label, tabs }: { label: string; tabs: [string, ReactNode][] }) {
  const [at, setAt] = useState(0);
  return (
    <div className="yt-rtabs">
      <div className="yt-rtabs-list" role="tablist" aria-label={label}>
        {tabs.map(([name], i) => <button type="button" role="tab" key={name} aria-selected={i === at} tabIndex={i === at ? 0 : -1}
          className={i === at ? "active" : undefined} onClick={() => setAt(i)}>{name}</button>)}
      </div>
      {tabs.map(([name, el], i) => <div role="tabpanel" key={name} hidden={i !== at} aria-label={name}>{i === at && el}</div>)}
    </div>
  );
}
