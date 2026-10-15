import { useEffect, useState } from "react";
import { COPY } from "./labels";
import { isSeries, isValue, SPECIAL_OPERANDS, type Side } from "./spec-model";
import type { Catalogue, Indicator, Operand, SeriesOperand } from "./types";
import "./rules.css";

const parse = (t: string) => (t.trim() === "" || !Number.isFinite(Number(t)) ? undefined : Number(t));

/** Number input that keeps the typed text (e.g. "1.") until it parses; empty reports undefined. */
export function NumField({ label, value, onChange, min, max, step }: {
  label: string; value: number | undefined; onChange(n: number | undefined): void; min?: number; max?: number; step?: number | null;
}) {
  const [text, setText] = useState(value === undefined ? "" : String(value));
  useEffect(() => setText((t) => (parse(t) === value ? t : value === undefined ? "" : String(value))), [value]);
  return (
    <label className="dfield">
      {label}
      <input type="number" aria-label={label} value={text} min={min} max={max} step={step ?? "any"}
        onChange={(e) => { setText(e.target.value); onChange(parse(e.target.value)); }}
        onBlur={() => setText(value === undefined ? "" : String(value))} />
    </label>
  );
}

/** tf/offset survive a kind switch; params, output and unknown keys do not. */
const keepTiming = (o: Operand) => (isSeries(o) ? { ...(o.tf ? { tf: o.tf } : {}), ...(o.offset ? { offset: o.offset } : {}) } : {});
const dropParams = ({ p: _p, out: _out, ...rest }: SeriesOperand) => rest;

export function OperandEditor({ operand, side, catalogue, onChange, label }: {
  operand: Operand; side: Side; catalogue: Catalogue; onChange(o: Operand): void; label: string;
}) {
  const series = isSeries(operand) ? operand : undefined;
  const special = series && SPECIAL_OPERANDS.find((x) => x.s === series.s);
  const ind = series && catalogue.indicators.find((x) => x.id === series.s);
  const kind = isValue(operand) ? "number" : !series ? undefined : special ? special.s
    : catalogue.price_sources.includes(series.s) ? "price" : ind ? "indicator" : undefined;
  if (!kind)
    return <span className="lab-kept" role="group" aria-label={label}>{COPY.keptAsIs} <code>{JSON.stringify(operand)}</code></span>;

  const computed = catalogue.indicators.filter((x) => x.computed);
  const groups = [...new Set(computed.map((x) => x.category))];
  // The current special stays listed even on the wrong side so the editor shows what the spec holds; the pre-checks flag it.
  const specials = SPECIAL_OPERANDS.filter((x) => x.sides.includes(side) || x.s === special?.s);
  const switchKind = (k: string) => {
    if (k === kind) return;
    if (k === "number") onChange({ value: 0 });
    else if (k === "price") onChange({ ...keepTiming(operand), s: catalogue.price_sources.includes("close") ? "close" : catalogue.price_sources[0] });
    else if (k === "indicator") onChange({ ...keepTiming(operand), s: computed[0].id });
    else onChange({ s: k });
  };
  const set = (patch: Partial<SeriesOperand>) => onChange({ ...series!, ...patch });
  const without = (key: "tf" | "offset") => { const copy = { ...series! }; delete copy[key]; return copy; };

  return (
    <span className="lab-operand" role="group" aria-label={label}>
      <select aria-label="Operand type" value={kind} onChange={(e) => switchKind(e.target.value)}>
        <option value="price">Price</option>
        <option value="indicator" disabled={computed.length === 0}>Indicator</option>
        <option value="number">Number</option>
        {specials.map((x) => <option key={x.s} value={x.s}>{x.label}</option>)}
      </select>
      {kind === "number" && isValue(operand) && (
        <NumField label="Number" value={operand.value} onChange={(n) => n !== undefined && onChange({ value: n })} />
      )}
      {kind === "price" && (
        <select aria-label="Price source" value={series!.s} onChange={(e) => onChange({ ...dropParams(series!), s: e.target.value })}>
          {catalogue.price_sources.map((s) => <option key={s} value={s}>{s}</option>)}
        </select>
      )}
      {kind === "indicator" && ind && <IndicatorFields operand={series!} ind={ind} computed={computed} groups={groups} set={set}
        replace={(id) => onChange({ ...dropParams(series!), s: id })} />}
      {(kind === "price" || kind === "indicator") && (
        <>
          <select aria-label="Timeframe" value={series!.tf ?? "D"} onChange={(e) => onChange(e.target.value === "D" ? without("tf") : { ...series!, tf: e.target.value })}>
            {[...new Set(["D", ...catalogue.timeframes, series!.tf ?? "D"])].map((tf) => <option key={tf} value={tf}>{tf}</option>)}
          </select>
          <NumField label="bars ago" value={series!.offset ?? 0} min={0} max={1000} step={1}
            onChange={(n) => n !== undefined && Number.isInteger(n) && onChange(n === 0 ? without("offset") : { ...series!, offset: n })} />
        </>
      )}
    </span>
  );
}

function IndicatorFields({ operand, ind, computed, groups, set, replace }: {
  operand: SeriesOperand; ind: Indicator; computed: Indicator[]; groups: string[];
  set(patch: Partial<SeriesOperand>): void; replace(id: string): void;
}) {
  return (
    <>
      <select aria-label="Indicator" value={ind.id} onChange={(e) => replace(e.target.value)}>
        {!ind.computed && <option value={ind.id} disabled>{ind.name} (listed only)</option>}
        {groups.map((g) => (
          <optgroup key={g} label={g}>
            {computed.filter((x) => x.category === g).map((x) => <option key={x.id} value={x.id}>{x.name}</option>)}
          </optgroup>
        ))}
      </select>
      {ind.params.map((p) => (
        <NumField key={p.key} label={p.label} value={operand.p?.[p.key] ?? p.default} min={p.min} max={p.max} step={p.step}
          onChange={(n) => n !== undefined && set({ p: { ...operand.p, [p.key]: n } })} />
      ))}
      {ind.outputs.length > 1 && (
        <select aria-label="Output" value={operand.out ?? ind.outputs[0]} onChange={(e) => set({ out: e.target.value })}>
          {ind.outputs.map((o) => <option key={o} value={o}>{o}</option>)}
        </select>
      )}
    </>
  );
}
