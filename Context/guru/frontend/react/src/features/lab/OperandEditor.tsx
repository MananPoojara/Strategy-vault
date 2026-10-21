import { useEffect, useState, type ReactNode } from "react";
import { CALENDAR_OUTPUTS, COPY, EXPAND_FIRST_REFUSED, OPERAND_KIND_LABELS, TF_LABELS } from "./labels";
import { isSeries, isValue, type Side } from "./spec-model";
import { ARITH, CALENDAR_IDS, MAX_NTH, MAX_WHOLE, WINDOWS, convertOperand, describeNode, exprKey, firstAllowed, innerOperand, operandKind, type OperandKind } from "./rule-tree";
import type { Catalogue, FormulaOperand, Indicator, Operand, RuleNode, SavedIndicator, SeriesOperand } from "./types";
import { ForeignFields } from "./ForeignFields";
import { infoOf, useFormulaScope, wholeParam } from "./formula";
import "./rules.css";
import "./operand.css";

const parse = (t: string) => (t.trim() === "" || !Number.isFinite(Number(t)) ? undefined : Number(t));

/** Number input that keeps the typed text (e.g. "1.") until it parses; empty reports undefined. */
export function NumField({ label, value, onChange, min, max, step, hideLabel, placeholder, disabled }: {
  label: string; value: number | undefined; onChange(n: number | undefined): void; min?: number; max?: number; step?: number | null;
  /** Shown while empty: what an empty field means (e.g. "0", "off"). */
  placeholder?: string; disabled?: boolean;
  /** Inline in a sentence: the label stays for screen readers only. */
  hideLabel?: boolean;
}) {
  const [text, setText] = useState(value === undefined ? "" : String(value));
  useEffect(() => setText((t) => (parse(t) === value ? t : value === undefined ? "" : String(value))), [value]);
  return (
    <label className={hideLabel ? "dfield lab-inline-num" : "dfield"}>
      {hideLabel ? <span className="sr-only">{label}</span> : label}
      <input type="number" aria-label={label} value={text} min={min} max={max} step={step ?? "any"} placeholder={placeholder} disabled={disabled}
        onChange={(e) => { setText(e.target.value); onChange(parse(e.target.value)); }}
        onBlur={() => setText(value === undefined ? "" : String(value))} />
    </label>
  );
}


type Obj = Record<string, unknown>;
const FORMULAS: OperandKind[] = ["arith", "pct", "abs", "ref", "window", "change", "compress"];
const STATES: OperandKind[] = ["bars_since", "value_when", "since", "count"];
const dropParams = ({ p: _p, out: _out, ...rest }: SeriesOperand) => rest;

export function OperandEditor({ operand, side, catalogue, onChange, label, heldOk, renderNode, nodesFull = false }: {
  operand: Operand; side: Side; catalogue: Catalogue; onChange(o: Operand): void; label: string;
  heldOk?: boolean;
  renderNode?: (node: RuleNode, onChange: (n: RuleNode) => void, label: string) => ReactNode;
  nodesFull?: boolean;
}) {
  const kind = operandKind(operand, catalogue);
  const { mine, infos } = useFormulaScope();
  if (kind === "unknown") return <span className="lab-kept" role="group" aria-label={label}>{COPY.keptAsIs} <code>{JSON.stringify(operand)}</code></span>;
  const series = isSeries(operand) ? operand : undefined, key = exprKey(operand), x = operand as Obj;
  const computed = catalogue.indicators.filter((i) => i.computed && !CALENDAR_IDS.includes(i.id));
  const groups = [...new Set(computed.map((i) => i.category))];
  const ind = series && catalogue.indicators.find((i) => i.id === series.s);
  const expression = FORMULAS.includes(kind) || STATES.includes(kind);
  const set = (patch: Obj) => onChange({ ...operand, ...patch });
  const child = (value: Operand, update: (o: Operand) => void, name: string) => <OperandEditor operand={value} side={side} catalogue={catalogue}
    onChange={update} label={`${label} › ${name}`} heldOk={false} renderNode={renderNode} nodesFull={nodesFull} />;
  const sub = (field: string) => child(x[field] as Operand, (o) => set({ [field]: o }), field === "of" ? "value" : field);
  const node = (field: string) => renderNode
    ? renderNode(x[field] as RuleNode, (n) => set({ [field]: n }), `${label} › condition`)
    : <span className="lab-kept">{describeNode(x[field] as RuleNode, catalogue)}</span>;
  const bars = (min = 1, field = "bars", max = MAX_WHOLE) => <NumField label={field === "nth" ? "occurrence" : "bars"}
    value={(x[field] ?? (field === "nth" ? 1 : min)) as number} min={min} max={max} step={1}
    onChange={(n) => n !== undefined && Number.isInteger(n) && n >= min && n <= max && set({ [field]: n })} />;
  const swap = (next: string) => {
    if (!key || key === next) return;
    const rest = { ...x };
    delete rest[key];
    onChange({ ...rest, [next]: x[key] });
  };
  const pair = key && Array.isArray(x[key]) ? x[key] as Operand[] : [];
  const pairChild = (i: number) => child(pair[i], (o) => set({ [key!]: pair.map((v, j) => j === i ? o : v) }), i === 0 ? "left" : "right");
  const word = (text: string) => <span className="lab-word">{text}</span>;
  const kindOptions = (k: OperandKind) => <option key={k} value={k} disabled={nodesFull && kind !== k}>{OPERAND_KIND_LABELS[k]}</option>;
  return <span className={`lab-operand${expression ? " lab-expr" : ""}`} data-lab-operand-kind={kind} role="group" aria-label={label}>
    <select aria-label="Operand type" value={kind} onChange={(e) => {
      const next = e.target.value as OperandKind;
      if (nodesFull && next !== kind && [...FORMULAS, ...STATES].includes(next)) return;
      if (next === "formula") { if (mine[0]) onChange(formulaOperand(mine[0], infos[mine[0].formula], series ? timingOf(x) : {})); return; }
      onChange(updatedTiming(convertOperand(operand, next, catalogue), {}, next === "compress"));
    }}>
      <option value="price">{OPERAND_KIND_LABELS.price}</option>
      <option value="indicator" disabled={computed.length === 0}>{OPERAND_KIND_LABELS.indicator}</option>
      <option value="calendar" disabled={!catalogue.indicators.some((i) => i.computed && CALENDAR_IDS.includes(i.id))}>{OPERAND_KIND_LABELS.calendar}</option>
      <option value="number">{OPERAND_KIND_LABELS.number}</option>
      {(mine.length > 0 || kind === "formula") && <optgroup label="My indicators"><option value="formula">{OPERAND_KIND_LABELS.formula}</option></optgroup>}
      {((heldOk ?? side === "exit") || kind === "bars_held") && <option value="bars_held">{OPERAND_KIND_LABELS.bars_held}</option>}
      <optgroup label="Formula">{FORMULAS.map(kindOptions)}</optgroup>
      <optgroup label="Since a condition">{STATES.map(kindOptions)}</optgroup>
    </select>
    {kind === "number" && isValue(operand) && <NumField label="Number" value={operand.value}
      onChange={(n) => n !== undefined && onChange({ ...operand, value: n })} />}
    {kind === "price" && <select aria-label="Price source" value={series!.s} onChange={(e) => onChange(updatedTiming({ ...dropParams(series!), s: e.target.value }, {}, false))}>
      {catalogue.price_sources.map((s) => <option key={s} value={s}>{s}</option>)}
    </select>}
    {kind === "indicator" && ind && <IndicatorFields operand={series!} ind={ind} computed={computed} groups={groups}
      set={set} replace={(id) => onChange({ ...dropParams(series!), s: id })} />}
    {kind === "calendar" && <>
      <select aria-label="Calendar value" value={`${series!.s}.${series!.out ?? catalogue.indicators.find((i) => i.id === series!.s)?.outputs[0] ?? ""}`}
        onChange={(e) => { const [s, out] = e.target.value.split("."); onChange({ ...dropParams(series!), s, out }); }}>
        {CALENDAR_OUTPUTS.filter((v) => catalogue.indicators.some((i) => i.id === v.id && i.computed && i.outputs.includes(v.out)))
          .map((v) => <option key={`${v.id}.${v.out}`} value={`${v.id}.${v.out}`}>{v.label}</option>)}
      </select>
      {series!.s === "sessions_left" && <span className="cap">{COPY.sessionDatesHint}</span>}
    </>}
    {kind === "arith" && <>{pairChild(0)}<select aria-label="Arithmetic" value={key} onChange={(e) => swap(e.target.value)}>
      {ARITH.map((k) => <option key={k} value={k}>{({ add: "plus", sub: "minus", mul: "times", div: "divided by", min: "min", max: "max" })[k]}</option>)}
    </select>{pairChild(1)}</>}
    {kind === "pct" && <>{word("% difference of")}{pairChild(0)}{word("vs")}{pairChild(1)}</>}
    {kind === "abs" && <>{word("|")}{sub("abs")}{word("|")}</>}
    {kind === "ref" && <>{sub("ref")}{word(",")}{bars(0)}{word("bars ago")}</>}
    {kind === "window" && <><select aria-label="Window" value={key} onChange={(e) => swap(e.target.value)}>
      {WINDOWS.map((k) => <option key={k} value={k}>{k === "avg" ? "average" : k}</option>)}
    </select>{word("of")}{sub(key!)}{word("over")}{bars()}</>}
    {kind === "change" && <>{word("change of")}{sub("change")}{word("over")}{bars()}</>}
    {kind === "bars_since" && <>{word("bars since")}{node("bars_since")}</>}
    {kind === "value_when" && <>{sub("of")}{word("when")}{node("value_when")}{word("occurrence")}{bars(1, "nth", MAX_NTH)}</>}
    {kind === "since" && <><select aria-label="Highest or lowest" value={key} onChange={(e) => swap(e.target.value)}>
      <option value="highest_since">highest</option><option value="lowest_since">lowest</option>
    </select>{word("of")}{sub("of")}{word("since")}{node(key!)}</>}
    {kind === "count" && <>{word("bars in the last")}{bars()}{word("where")}{node("count")}</>}
    {kind === "compress" && <>{sub("compress")}{word("as")}<TimeframeField operand={operand} catalogue={catalogue} onChange={onChange} compress />
      <select aria-label="Aggregate" value={(x.mode ?? "last") as string} onChange={(e) => onChange(updatedTiming(operand, { mode: e.target.value }, true))}>
        {["last", "first", "high", "low", "sum"].map((m) => <option key={m} value={m}>{m}</option>)}
      </select>
    </>}
    {kind === "formula" && <FormulaFields operand={operand as FormulaOperand} onChange={onChange} />}
    {(kind === "price" || kind === "indicator" || kind === "calendar" || kind === "compress" || kind === "formula") &&
      <Timing operand={operand} catalogue={catalogue} onChange={onChange} compress={kind === "compress"} />}
    {series && <ForeignFields operand={operand} onChange={onChange} label={label} />}
    {expression && <button type="button" className="dbtn sm ghost" aria-label="Remove formula" title="Remove formula, keep the inner value"
      onClick={() => onChange(innerOperand(operand) ?? { s: "close" })}>×</button>}
  </span>;
}

function updatedTiming(operand: Operand, patch: Obj, compress: boolean): Operand {
  const next = { ...operand, ...patch } as Obj;
  if (next.tf === "D") { delete next.tf; delete next.expand; }
  if (next.offset === 0) delete next.offset;
  if (next.expand === "last" || (next.expand === "first" && !firstAllowed(next, compress))) delete next.expand;
  return next;
}
function TimeframeField({ operand, catalogue, onChange, compress = false }: {
  operand: Operand; catalogue: Catalogue; onChange(o: Operand): void; compress?: boolean;
}) {
  const x = operand as Obj, tf = (x.tf ?? (compress ? "W" : "D")) as string, custom = /^\d+D$/.test(tf);
  const choices = [...new Set([...(compress ? [] : ["D"]), ...catalogue.timeframes.filter((t) => !compress || t !== "D"), ...(!custom && tf !== "D" ? [tf] : [])])];
  return <>
    <select aria-label={compress ? "Aggregate timeframe" : "Timeframe"} value={custom ? "nD" : tf}
      onChange={(e) => onChange(updatedTiming(operand, { tf: e.target.value === "nD" ? "5D" : e.target.value }, compress))}>
      {choices.map((t) => <option key={t} value={t}>{TF_LABELS[t] ?? t}</option>)}
      <option value="nD">Every N sessions…</option>
    </select>
    {custom && <NumField label="sessions" value={Number(tf.slice(0, -1))} min={2} max={catalogue.custom_days?.[1] ?? 63} step={1}
      onChange={(n) => n !== undefined && Number.isInteger(n) && n >= 2 && n <= (catalogue.custom_days?.[1] ?? 63) &&
        onChange(updatedTiming(operand, { tf: `${n}D` }, compress))} />}
  </>;
}
function Timing({ operand, catalogue, onChange, compress }: {
  operand: Operand; catalogue: Catalogue; onChange(o: Operand): void; compress: boolean;
}) {
  const [open, setOpen] = useState(false);
  const x = operand as Obj, tf = (x.tf ?? "D") as string, offset = Number(x.offset ?? 0), expand = x.expand ?? "last";
  const allowed = firstAllowed(operand, compress);
  const summary = [tf !== "D" ? TF_LABELS[tf] ?? `Every ${tf.slice(0, -1)} sessions` : "", offset ? `${offset} ${tf === "D" ? "bars" : "periods"} ago` : "",
    expand === "point" ? "period's last day" : expand === "first" ? "from the period's first day" : ""].filter(Boolean).join(" · ");
  return <>
    <button type="button" className="dbtn sm ghost" data-lab-timing aria-expanded={open} aria-label="Timeframe and offset" title="Timeframe and offset" onClick={() => setOpen(!open)}>{summary || "Daily · now"} <span aria-hidden="true">{open ? "▴" : "▾"}</span></button>
    {open && <span className="lab-timing">
      {!compress && <TimeframeField operand={operand} catalogue={catalogue} onChange={onChange} />}
      {tf !== "D" && <>
        <select aria-label="Use the period value" value={expand as string} onChange={(e) => {
          if (e.target.value !== "first" || allowed) onChange(updatedTiming(operand, { expand: e.target.value }, compress));
        }}>
          <option value="last">after the period closes</option><option value="point">on the period's last day only</option>
          <option value="first" disabled={!allowed}>from the period's first day</option>
        </select>
        {!allowed && <span className="cap">{EXPAND_FIRST_REFUSED}</span>}
      </>}
      <NumField label={tf === "D" ? "bars ago" : "periods ago"} value={offset} min={0} max={MAX_WHOLE} step={1}
        onChange={(n) => n !== undefined && Number.isInteger(n) && n >= 0 && n <= MAX_WHOLE && onChange(updatedTiming(operand, { offset: n }, compress))} />
    </span>}
  </>;
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

const timingOf = (o: Obj) => Object.fromEntries(Object.entries(o).filter(([k]) => ["tf", "offset", "expand"].includes(k)));
/** The saved formula's text is copied into the spec: later edits to the saved indicator do not change this strategy. */
function formulaOperand(row: SavedIndicator, check: Parameters<typeof infoOf>[0], timing: Obj): FormulaOperand {
  const info = infoOf(check);
  return { ...timing, formula: row.formula, name: row.name, ...(info ? { p: Object.fromEntries(info.params.map((p) => [p.key, p.default])), ...(info.plots[0] ? { out: info.plots[0] } : {}) } : {}) };
}
function FormulaFields({ operand, onChange }: { operand: FormulaOperand; onChange(o: Operand): void }) {
  const { mine, infos } = useFormulaScope();
  const check = infos[operand.formula], info = infoOf(check);
  const saved = mine.find((r) => r.formula === operand.formula && r.name === operand.name) ?? mine.find((r) => r.formula === operand.formula);
  return <>
    <select aria-label="My indicator" value={saved ? String(saved.id) : ""} onChange={(e) => {
      const row = mine.find((r) => String(r.id) === e.target.value);
      if (row) onChange(formulaOperand(row, infos[row.formula], timingOf(operand)));
    }}>
      {!saved && <option value="">{operand.name || "My indicator"} (copy kept in this strategy)</option>}
      {mine.map((r) => <option key={r.id} value={String(r.id)}>{r.name}</option>)}
    </select>
    {!check && <span className="cap">Checking the formula…</span>}
    {check && "error" in check && <span className="cap lab-formula-err" role="note">Formula error: {check.error}</span>}
    {info?.params.map((p) => <NumField key={p.key} label={p.key} value={operand.p?.[p.key] ?? p.default} min={p.min} max={p.max} step={p.step || null}
      onChange={(n) => n !== undefined && n >= p.min && n <= p.max && (!wholeParam(p) || Number.isInteger(n)) && onChange({ ...operand, p: { ...operand.p, [p.key]: n } })} />)}
    {info && info.plots.length > 1 && <select aria-label="Output" value={operand.out ?? info.plots[0]} onChange={(e) => onChange({ ...operand, out: e.target.value })}>
      {info.plots.map((o) => <option key={o} value={o}>{o}</option>)}
    </select>}
  </>;
}
