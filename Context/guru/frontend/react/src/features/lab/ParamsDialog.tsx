import { useEffect, useRef, useState } from "react";
import { PALETTE, themeColors, type ChartColors } from "./charts";
import type { Indicator, IndicatorParam } from "./types";
import type { InstrumentOption } from "./instruments";
import "./params.css";

/** AmiBroker ParamStyle subset that a line plot can show. */
export type LineStyle = "line" | "thick" | "dashed" | "dotted" | "hidden";
export const LINE_STYLES: [LineStyle, string][] = [["line", "Line"], ["thick", "Thick"], ["dashed", "Dashed"], ["dotted", "Dotted"], ["hidden", "Hidden"]];
/** Named by role: the theme sets the hue (its "blue" is teal, brown or plum). bh / ink3 remain only for looks saved before. */
export const COLOR_NAMES: Record<string, string> = { blue: "Theme main", red: "Red", green: "Green", chart: "Theme accent", ink2: "Dark grey", bh: "Muted", ink3: "Light grey" };
/** A theme colour name, or a "#rrggbb" picked with Custom… (fixed: it does not follow the light/dark theme). */
export type PlotColor = keyof ChartColors | `#${string}`;
/** AmiBroker ParamColor / ParamStyle for one plot (indicator output); absent fields use the defaults. */
export interface Look { color?: PlotColor; style?: LineStyle }
const isHex = (c: string): c is `#${string}` => /^#[0-9a-f]{6}$/i.test(c);
/** CSS for a plot colour (swatches, chip dots). */
export const plotCss = (c: PlotColor) => (isHex(c) ? c : `var(--${c})`);
/** Chart colour for a plot colour on the current theme. */
export const plotPaint = (c: PlotColor, colors: ChartColors) => (isHex(c) ? c : colors[c as keyof ChartColors]);
/** Default colour of output k of the chip at `index`: successive palette entries, so MACD's lines differ. */
export const defaultColor = (index: number, k = 0): keyof ChartColors => PALETTE[(index + k) % PALETTE.length];
/** Slider commits wait this long after the last move, so a drag sends one series request, not one per step. */
export const SLIDE_MS = 200;

export interface ParamsDialogProps {
  name: string; indicator: Indicator; params: Record<string, number>; looks?: Record<string, Look>; index: number;
  vs?: string; comparisonOptions?: readonly InstrumentOption[]; showComparison: boolean;
  onParams(params: Record<string, number>): void; onLook(looks: Record<string, Look> | undefined): void; onVs?(vs?: string): void; onClose(): void;
}

/**
 * AmiBroker-style Parameters window ("Properties of …"): one section per indicator, name | value rows, changes apply live,
 * Reset all restores defaults, OK closes. Axes & Grid is shown disabled, as the lab charts keep the house axes.
 */
export function ParamsDialog({ name, indicator, params, looks, index, vs, comparisonOptions = [], showComparison, onParams, onLook, onVs, onClose }: ParamsDialogProps) {
  const box = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(true);
  const titleId = `lab-params-${indicator.id}`;
  useEffect(() => { box.current?.querySelector<HTMLElement>("input, select")?.focus(); }, []);
  const set = (k: string, v: number) => onParams({ ...params, [k]: v });
  const reset = () => { onParams(Object.fromEntries(indicator.params.map((p) => [p.key, p.default]))); onLook(undefined); };
  const outputs = indicator.outputs.length ? indicator.outputs : [indicator.id];
  const look = (out: string, patch: Look) => onLook({ ...looks, [out]: { ...looks?.[out], ...patch } });
  return (
    <div className="lab-params" role="dialog" aria-labelledby={titleId} ref={box} data-lab-params={indicator.id}
      onKeyDown={(e) => { if (e.key === "Escape") { e.stopPropagation(); onClose(); } }}>
      <div className="lab-params-h">
        <b id={titleId}>Properties of {name}</b>
        <button type="button" aria-label="Close parameters" onClick={onClose}>×</button>
      </div>
      <div className="lab-params-tabs"><span>Parameters</span><span aria-disabled="true" title="Not offered in the lab: charts keep the house axes and grid">Axes &amp; Grid</span></div>
      <div className="lab-params-grid">
        <button type="button" className="lab-params-sec" aria-expanded={open} onClick={() => setOpen((o) => !o)}>{name}</button>
        {open && <>
          {indicator.params.map((p) => (
            <div className="lab-prow" key={p.key}>
              <span>{p.label}</span>
              {p.key === "variant" ? (
                <select aria-label={`${name} ${p.label}`} value={params[p.key] ?? p.default} onChange={(e) => set(p.key, Number(e.target.value))}>
                  <option value={0}>TradingView</option>
                  <option value={1}>AmiBroker</option>
                </select>
              ) : <ParamValue param={p} name={name} value={params[p.key] ?? p.default} onValid={(v) => set(p.key, v)} />}
            </div>
          ))}
          {showComparison && (
            <div className="lab-prow">
              <span>Compared with</span>
              <select aria-label={`${name} · Compared with`} value={vs ?? ""} onChange={(e) => onVs?.(e.target.value || undefined)}>
                <option value="">Choose instrument…</option>
                {vs && !comparisonOptions.some((o) => o.ticker === vs) && <option value={vs}>Kept: {vs}</option>}
                {comparisonOptions.map((o) => <option key={o.ticker} value={o.ticker} disabled={o.disabled}>{o.label}</option>)}
              </select>
            </div>
          )}
          {outputs.map((out, k) => {
            // one Color / Style pair per plot, as AmiBroker's ParamColor / ParamStyle per Plot()
            const color = looks?.[out]?.color ?? defaultColor(index, k), plot = outputs.length > 1 ? ` ${out}` : "";
            return <div key={out} className="lab-plot-look">
              <div className="lab-prow">
                <span>Color{plot}</span>
                <span className="lab-pval">
                  <i className="lab-swatch" style={{ background: plotCss(color) }} aria-hidden="true" />
                  <select aria-label={`${name}${plot} Color`} value={isHex(color) ? "custom" : color} onChange={(e) => {
                    // Custom… starts from the colour shown now, so the plot does not jump
                    const v = e.target.value, hex = v === "custom" ? plotPaint(color, themeColors(box.current ?? document.documentElement)) : "";
                    look(out, { color: v === "custom" ? (isHex(hex) ? hex : "#2f5a8f") : (v as keyof ChartColors) });
                  }}>
                    {!isHex(color) && !PALETTE.includes(color) && <option value={color}>{COLOR_NAMES[color] ?? color}</option>}
                    {PALETTE.map((c) => <option key={c} value={c}>{COLOR_NAMES[c] ?? c}</option>)}
                    <option value="custom">Custom…</option>
                  </select>
                  {isHex(color) && <input type="color" className="lab-color" aria-label={`${name}${plot} custom colour`} value={color} title="A custom colour stays the same in light and dark themes"
                    onChange={(e) => look(out, { color: e.target.value as `#${string}` })} />}
                </span>
              </div>
              <div className="lab-prow">
                <span>Style{plot}</span>
                <select aria-label={`${name}${plot} Style`} value={looks?.[out]?.style ?? "line"} onChange={(e) => look(out, { style: e.target.value as LineStyle })}>
                  {LINE_STYLES.map(([v, label]) => <option key={v} value={v}>{label}</option>)}
                </select>
              </div>
            </div>;
          })}
          <div className="lab-prow lab-prow-info"><span>Plotted</span><span>{indicator.placement === "price" ? "On the price chart" : "In its own pane"}</span></div>
        </>}
      </div>
      <div className="lab-params-f">
        <button type="button" className="dbtn sm" onClick={reset}>Reset all</button>
        <button type="button" className="dbtn sm primary" onClick={onClose}>OK</button>
      </div>
    </div>
  );
}

/** Edit field plus slider (AmiBroker numeric Param). Typed drafts stay local so an out-of-range value does not snap back. */
function ParamValue({ param, name, value, onValid }: { param: IndicatorParam; name: string; value: number; onValid(v: number): void }) {
  const [text, setText] = useState(String(value));
  const [slide, setSlide] = useState<number | null>(null); // the slider's position while a drag waits to commit
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => setText(String(value)), [value]);
  useEffect(() => () => clearTimeout(timer.current), []);
  // A 1–2000 slider cannot pick 9 or 14; the typed field keeps the full range.
  const top = param.max > 500 ? Math.min(param.max, Math.max(200, param.default * 10)) : param.max;
  const accept = (raw: number) => {
    const v = param.integer ? Math.round(raw) : raw;
    if (Number.isFinite(v) && v >= param.min && v <= param.max) onValid(v);
  };
  return (
    <span className="lab-pval">
      <input type="number" aria-label={`${name} ${param.label}`} min={param.min} max={param.max} step={param.step ?? "any"} value={text}
        onChange={(e) => { setText(e.target.value); accept(e.target.value.trim() === "" ? NaN : Number(e.target.value)); }} />
      <input type="range" aria-label={`${name} ${param.label} slider`} min={param.min} max={top}
        step={param.step ?? (param.integer ? 1 : (top - param.min) / 100)} value={slide ?? value} onChange={(e) => {
          const v = Number(e.target.value);
          setSlide(v); setText(String(v)); clearTimeout(timer.current);
          timer.current = setTimeout(() => { setSlide(null); accept(v); }, SLIDE_MS);
        }} />
    </span>
  );
}
