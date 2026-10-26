import { useEffect, useRef, useState } from "react";
import { PALETTE, themeColors, type ChartColors } from "./charts";
import type { Indicator, IndicatorParam } from "./types";
import type { InstrumentOption } from "./instruments";
import { TF_LABELS } from "./labels";
import { AB_DEFAULTS } from "./catalogue";
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

/** A variant's choices named as the catalogue label lists them ("Variant (0 Wilder/TradingView, 1 Cutler/TradeStation RSI JP)");
 *  a value the label does not name shows its number. */
/** A whole-number param whose label names its values ("Smoothing (0 Sum/SMA, 1 EMA)") is picked from a list, like Variant. */
export const isChoice = (p: IndicatorParam) => p.key === "variant" || (p.integer && p.max - p.min <= 10 && /\(0 .+, 1 .+\)\s*$/.test(p.label));
export function variantOptions(p: IndicatorParam): [number, string][] {
  const listed = (p.label.match(/\((.*)\)\s*$/)?.[1] ?? "").split(/,\s*(?=\d+ )/).map((s) => s.match(/^(\d+) (.+)$/));
  const names = new Map(listed.flatMap((m) => (m ? [[Number(m[1]), m[2]] as const] : [])));
  return Array.from({ length: p.max - p.min + 1 }, (_, k) => [p.min + k, names.get(p.min + k) ?? String(p.min + k)]);
}

export interface ParamsDialogProps {
  name: string; indicator: Indicator; params: Record<string, number>; looks?: Record<string, Look>; index: number;
  vs?: string; comparisonOptions?: readonly InstrumentOption[]; showComparison: boolean; tf?: string; timeframes?: readonly string[]; onTf?(tf?: string): void; levels?: number[]; onLevels?(levels: number[]): void;
  onParams(params: Record<string, number>): void; onLook(looks: Record<string, Look> | undefined): void; onVs?(vs?: string): void; onClose(): void;
  /** false: no Color / Style rows (a chart type draws its own elements, not plots). */
  showLooks?: boolean;
}

/**
 * AmiBroker-style Parameters window ("Properties of …"): one section per indicator, name | value rows, changes apply live,
 * Reset all restores defaults, OK closes. Axes & Grid is shown disabled, as the lab charts keep the house axes.
 */
export function ParamsDialog({ name, indicator, params, looks, index, vs, comparisonOptions = [], showComparison, tf, timeframes = [], onTf, levels, onLevels, onParams, onLook, onVs, onClose, showLooks = true }: ParamsDialogProps) {
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
              <span>{isChoice(p) ? p.label.replace(/\s*\(.*\)\s*$/, "") : p.label}</span>
              {isChoice(p) ? (
                <select aria-label={`${name} ${p.label}`} value={params[p.key] ?? p.default} onChange={(e) => set(p.key, Number(e.target.value))}>
                  {variantOptions(p).map(([v, text]) => <option key={v} value={v}>{text}</option>)}
                </select>
              ) : <ParamValue param={p} name={name} value={params[p.key] ?? p.default} onValid={(v) => set(p.key, v)} />}
            </div>
          ))}
          {levels && onLevels && (
            <div className="lab-prow">
              <span>Levels</span>
              <LevelsField name={name} levels={levels} onLevels={onLevels} />
            </div>
          )}
          {onTf && timeframes.length > 0 && (
            <div className="lab-prow">
              <span>Timeframe</span>
              <select aria-label={`${name} · Timeframe`} value={tf ?? ""} onChange={(e) => onTf(e.target.value || undefined)}>
                <option value="">Same as chart</option>
                {timeframes.filter((t) => !/^\d+D$/.test(t)).map((t) => <option key={t} value={t}>{TF_LABELS[t] ?? t}</option>)}
              </select>
            </div>
          )}
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
          {showLooks && outputs.map((out, k) => {
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
        {AB_DEFAULTS[indicator.id] && <button type="button" className="dbtn sm" data-lab-ab-defaults title="Set the parameters to AmiBroker's own defaults for this indicator"
          onClick={() => onParams({ ...params, ...AB_DEFAULTS[indicator.id] })}>AmiBroker defaults</button>}
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

/** Threshold lines as "70, 30": applied on Enter or when the field loses focus; empty removes them. */
function LevelsField({ name, levels, onLevels }: { name: string; levels: number[]; onLevels(levels: number[]): void }) {
  const [draft, setDraft] = useState(levels.join(", "));
  const parts = draft.split(/[,\s]+/).filter(Boolean), bad = parts.some((p) => !Number.isFinite(Number(p)));
  const apply = () => { if (!bad) onLevels(parts.map(Number)); };
  return <input aria-label={`${name} · Levels`} value={draft} placeholder="e.g. 70, 30" aria-invalid={bad} title={bad ? "Numbers separated by commas" : undefined}
    onChange={(e) => setDraft(e.target.value)} onBlur={apply} onKeyDown={(e) => { if (e.key === "Enter") apply(); }} />;
}
