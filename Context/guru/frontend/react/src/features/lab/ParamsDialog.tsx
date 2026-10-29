import { useEffect, useId, useRef, useState } from "react";
import { PALETTE, themeColors, type ChartColors } from "./charts";
import type { Indicator, IndicatorParam } from "./types";
import type { InstrumentOption } from "./instruments";
import { TF_LABELS, paramLabel } from "./labels";
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

/** A variant's choices named as the catalogue label lists them ("Variant (0 Wilder/TradingView, 1 Cutler/TradeStation RSI JP)");
 *  a value the label does not name shows its number. */
/** A whole-number param whose label names its values ("Smoothing (0 Sum/SMA, 1 EMA)") is picked from a list, like Variant. */
import { isChoice } from "./spec-model";
export { isChoice };
export function variantOptions(p: IndicatorParam): [number, string][] {
  const listed = (p.label.match(/\((.*)\)\s*$/)?.[1] ?? "").split(/,\s*(?=\d+ )/).map((s) => s.match(/^(\d+) (.+)$/));
  const names = new Map(listed.flatMap((m) => (m ? [[Number(m[1]), m[2]] as const] : [])));
  return Array.from({ length: p.max - p.min + 1 }, (_, k) => [p.min + k, names.get(p.min + k) ?? String(p.min + k)]);
}

const PLATFORMS = ["TradingView", "TradeStation", "MetaTrader", "NinjaTrader", "thinkorswim", "MetaStock", "TA-Lib", "TradeTiger", "ChartIQ", "Hull Suite"];
/** Choices that give the same results as the default once the warm-up is over (RSI's AmiBroker from-bar-1 loop, momentum.rs):
 *  listed only when a strategy already uses one. */
const SAME_AS_DEFAULT = ["AmiBroker from bar 1"];
/** The Calculation list in plain names: a choice AmiBroker uses is "AmiBroker", another is its platform ("TradeStation"); two
 *  choices that would share a name, and choices with no platform, keep the catalogue's words. */
export function variantChoices(p: IndicatorParam, current: number): [number, string][] {
  const all = variantOptions(p);
  if (p.key !== "variant") return all;
  const kept = all.filter(([v, text]) => v === current || !SAME_AS_DEFAULT.includes(text));
  const short = kept.map(([v, text]) => [v, text.includes("AmiBroker") ? "AmiBroker" : PLATFORMS.find((x) => text.includes(x)) ?? text] as [number, string]);
  return short.map(([v, name], i) => short.filter(([, n]) => n === name).length > 1 ? kept[i] : [v, name]);
}

export interface ParamsDialogProps {
  name: string; indicator: Indicator; params: Record<string, number>; looks?: Record<string, Look>; index: number;
  vs?: string; comparisonOptions?: readonly InstrumentOption[]; showComparison: boolean; tf?: string; timeframes?: readonly string[]; onTf?(tf?: string): void; levels?: number[]; onLevels?(levels: number[]): void;
  onParams(params: Record<string, number>): void; onLook(looks: Record<string, Look> | undefined): void; onVs?(vs?: string): void; onClose(): void;
  /** false: no Color / Style rows (a chart type draws its own elements, not plots). */
  showLooks?: boolean;
}

/** Nonmodal chart settings: changes apply live, while Inputs / Style keep their drafts when switching tabs. */
export function ParamsDialog({ name, indicator, params, looks, index, vs, comparisonOptions = [], showComparison, tf, timeframes = [], onTf, levels, onLevels, onParams, onLook, onVs, onClose, showLooks = true }: ParamsDialogProps) {
  const box = useRef<HTMLDivElement>(null);
  const initial = useRef({ params: { ...params }, looks: looks && Object.fromEntries(Object.entries(looks).map(([out, value]) => [out, { ...value }])), vs, tf, levels: levels?.slice() });
  const cancel = () => {
    const saved = initial.current;
    onParams(saved.params); onLook(saved.looks); onVs?.(saved.vs); onTf?.(saved.tf);
    if (saved.levels) onLevels?.(saved.levels);
    onClose();
  };
  const confirm = () => {
    const invalid = box.current?.querySelector<HTMLInputElement>('input[aria-invalid="true"]');
    if (invalid) { invalid.focus(); invalid.reportValidity(); return; }
    onClose();
  };
  const [tab, setTab] = useState<"inputs" | "style">("inputs");
  const activeTab = showLooks ? tab : "inputs";
  const titleId = useId();
  useEffect(() => { box.current?.querySelector<HTMLElement>("input, select")?.focus(); }, []);
  const set = (k: string, v: number) => onParams({ ...params, [k]: v });
  const reset = () => { onParams(Object.fromEntries(indicator.params.map((p) => [p.key, p.default]))); onLook(undefined); };
  const outputs = indicator.outputs.length ? indicator.outputs : [indicator.id];
  const look = (out: string, patch: Look) => onLook({ ...looks, [out]: { ...looks?.[out], ...patch } });
  return (
    <div className="lab-params" role="region" aria-labelledby={titleId} ref={box} data-lab-params={indicator.id}
      onKeyDown={(e) => { if (e.key === "Escape") { e.stopPropagation(); cancel(); } }}>
      <div className="lab-params-h">
        <b id={titleId}>{name}</b>
        <button type="button" aria-label="Close parameters" onClick={cancel}>×</button>
      </div>
      <div className="lab-params-tabs" role="tablist" aria-label={`${name} settings`} onKeyDown={(e) => {
        const buttons = [...e.currentTarget.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
        const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
        const next = e.key === "ArrowRight" ? (at + 1) % buttons.length : e.key === "ArrowLeft" ? (at + buttons.length - 1) % buttons.length : e.key === "Home" ? 0 : e.key === "End" ? buttons.length - 1 : undefined;
        if (next !== undefined) { e.preventDefault(); buttons[next].click(); buttons[next].focus(); }
      }}>
        <button type="button" role="tab" id={`${titleId}-inputs-tab`} aria-controls={`${titleId}-inputs`} aria-selected={activeTab === "inputs"} tabIndex={activeTab === "inputs" ? 0 : -1} onClick={() => setTab("inputs")}>Inputs</button>
        {showLooks && <button type="button" role="tab" id={`${titleId}-style-tab`} aria-controls={`${titleId}-style`} aria-selected={activeTab === "style"} tabIndex={activeTab === "style" ? 0 : -1} onClick={() => setTab("style")}>Style</button>}
      </div>
      <div className="lab-params-grid">
        <div role="tabpanel" id={`${titleId}-inputs`} aria-labelledby={`${titleId}-inputs-tab`} hidden={activeTab !== "inputs"}>

          {indicator.params.filter((p) => indicator.id !== "mfi" || p.key !== "smoothing" || (params.variant ?? indicator.params.find((v) => v.key === "variant")?.default ?? 0) === 1).map((p) => (
            <div className="lab-prow" key={p.key}>
              <span>{p.key === "variant" ? "Calculation" : indicator.id === "mfi" && p.key === "smoothing" ? "Smoothing" : isChoice(p) ? p.label.replace(/\s*\(.*\)\s*$/, "") : paramLabel(p.label)}</span>
              {isChoice(p) ? (
                <select aria-label={`${name} ${p.label}`} value={params[p.key] ?? p.default} onChange={(e) => set(p.key, Number(e.target.value))}>
                  {variantChoices(p, params[p.key] ?? p.default).map(([v, text]) => <option key={v} value={v}>{text}</option>)}
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
          <div className="lab-prow lab-prow-info"><span>Plotted</span><span>{indicator.placement === "price" ? "On the price chart" : "In its own pane"}</span></div>
        </div>
        {showLooks && <div role="tabpanel" id={`${titleId}-style`} aria-labelledby={`${titleId}-style-tab`} hidden={activeTab !== "style"}>
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
        </div>}
      </div>
      <div className="lab-params-f">
        <button type="button" className="dbtn sm" onClick={reset}>Defaults</button>
        {AB_DEFAULTS[indicator.id] && <button type="button" className="dbtn sm" data-lab-ab-defaults title="Set the parameters to AmiBroker's own defaults for this indicator"
          onClick={() => onParams({ ...params, ...AB_DEFAULTS[indicator.id] })}>AmiBroker defaults</button>}
        <button type="button" className="dbtn sm" onClick={cancel}>Cancel</button>
        <button type="button" className="dbtn sm primary" onClick={confirm}>OK</button>
      </div>
    </div>
  );
}

/** Typed drafts stay local: invalid inputs never update the plotted series. */
function ParamValue({ param, name, value, onValid }: { param: IndicatorParam; name: string; value: number; onValid(v: number): void }) {
  const [text, setText] = useState(String(value));
  useEffect(() => setText(String(value)), [value]);
  const number = text.trim() === "" ? NaN : Number(text);
  const valid = Number.isFinite(number) && number >= param.min && number <= param.max && (!param.integer || Number.isInteger(number));
  return <input type="number" aria-label={`${name} ${param.label}`} min={param.min} max={param.max} step={param.step ?? "any"} value={text} aria-invalid={!valid}
    onChange={(event) => {
      const raw = event.target.value, number = raw.trim() === "" ? NaN : Number(raw), next = param.integer ? Math.round(number) : number;
      setText(raw);
      if (Number.isFinite(next) && next >= param.min && next <= param.max) onValid(next);
    }} />;
}

/** Threshold lines as "70, 30": applied on Enter or when the field loses focus; empty removes them. */
function LevelsField({ name, levels, onLevels }: { name: string; levels: number[]; onLevels(levels: number[]): void }) {
  const [draft, setDraft] = useState(levels.join(", "));
  const parts = draft.split(/[,\s]+/).filter(Boolean), bad = parts.some((p) => !Number.isFinite(Number(p)));
  const apply = () => { if (!bad) onLevels(parts.map(Number)); };
  return <input aria-label={`${name} · Levels`} value={draft} placeholder="e.g. 70, 30" aria-invalid={bad} title={bad ? "Numbers separated by commas" : undefined}
    onChange={(e) => setDraft(e.target.value)} onBlur={apply} onKeyDown={(e) => { if (e.key === "Enter") apply(); }} />;
}
