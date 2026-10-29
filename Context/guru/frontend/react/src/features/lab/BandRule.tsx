import type { ReactNode } from "react";
import { IndicatorSearch, NumField, TimeframeField } from "./OperandEditor";
import { paramLabel } from "./labels";
import { isChoice } from "./ParamsDialog";
import { nodeKind } from "./rule-tree";
import { isCond, isSeries } from "./spec-model";
import { Segmented } from "./ui";
import type { Catalogue, Indicator, Operand, RuleNode, SeriesOperand } from "./types";

type Obj = Record<string, unknown>;

/** A band setup (rules.rs setup node, the "close below LBB, then a green candle back inside" AFLs): the close goes outside a
 *  band, the setup stays armed until a trigger bar, a cancel bar, or a trade opening or closing on that side. */
export interface Band {
  ind: SeriesOperand; from: string; dir: "below" | "above"; candle: "green" | "red" | "any";
  /** compare the trigger close with the previous bar's band (the AFL's "Above Previous LBB") */
  prev: boolean;
  /** the trigger close also stays inside the bands (below the upper band after a dip, above the lower after a rise) */
  inside: boolean; cancel: "upper" | "lower" | "none"; periodEnd: boolean;
  /** more conditions the setup bar needs (the AFL's ROC filters) */
  extra: RuleNode[];
  /** Bollinger only: the setup reads %B (bbpct, same settings) against this level instead of the close against a band line */
  pct?: number;
}

/** A price-chart indicator with an upper and a lower line: Bollinger, Keltner, Donchian, Envelope… */
export const isBand = (ind?: Indicator) => !!ind && ind.placement === "price" && ind.outputs.includes("upper") && ind.outputs.includes("lower");
const middleOf = (ind: Indicator) => ind.outputs.find((o) => o !== "upper" && o !== "lower");
/** The band in an operand: a band line, or Bollinger %B (its setup then reads %B, the trigger the Bollinger lines). */
export const bandOf = (o: Operand, cat: Catalogue) => (isSeries(o) && (isBand(cat.indicators.find((i) => i.id === o.s)) || (o.s === "bbpct" && cat.indicators.some((i) => i.id === "bb"))) ? o : undefined);
const bbOf = (o: SeriesOperand & Obj) => ({ s: "bb", ...(o.p ? { p: o.p } : {}), ...(o.tf ? { tf: o.tf } : {}) }) as SeriesOperand;

export function buildBand(b: Band): RuleNode {
  const tf = b.ind.tf && b.ind.tf !== "D" ? { tf: b.ind.tf } : {};
  const line = (out: string, prev = false) => ({ ...b.ind, out, ...(prev ? { offset: 1 } : {}) });
  const px = (s: string) => ({ s, ...tf }), up = b.dir === "below";
  const first = b.pct !== undefined ? { lhs: { s: "bbpct", ...(b.ind.p ? { p: b.ind.p } : {}), ...tf }, op: up ? "<" : ">", rhs: { value: b.pct } }
    : { lhs: px("close"), op: up ? "<" : ">", rhs: line(b.from) };
  const parts: Obj[] = [];
  if (b.candle !== "any") parts.push({ lhs: px("close"), op: b.candle === "green" ? ">" : "<", rhs: px("open") });
  parts.push({ lhs: px("close"), op: up ? ">" : "<", rhs: line(b.from, b.prev) });
  if (b.inside) parts.push(up ? { lhs: px("close"), op: "<", rhs: line("upper") } : { lhs: px("close"), op: ">", rhs: line("lower") });
  const periodEnd = b.periodEnd && (b.ind.tf === "W" || b.ind.tf === "M") ? { period_end: b.ind.tf } : {};
  return { setup: b.extra.length ? { all: [first, ...b.extra] } : first, trigger: parts.length === 1 ? parts[0] : { all: parts },
    ...(b.cancel === "none" ? {} : { cancel: { lhs: px("close"), op: b.cancel === "upper" ? ">" : "<", rhs: line(b.cancel) } }), ...periodEnd } as unknown as RuleNode;
}

/** The Band a setup node was built from, or undefined when it was written some other way (it then shows as its parts). */
export function parseBand(n: RuleNode, cat: Catalogue): Band | undefined {
  const { off: _off, ...x } = n as Obj;
  if (nodeKind(n) !== "setup") return undefined;
  const s = x.setup as Obj, [first, ...extra] = (Array.isArray(s.all) ? s.all : [s]) as Obj[];
  if (!first || !isCond(first as RuleNode)) return undefined;
  const t = x.trigger as Obj, parts = (Array.isArray(t.all) ? t.all : [t]) as Obj[];
  const pctB = (first.lhs as Obj).s === "bbpct" && "value" in (first.rhs as Obj);
  if (!pctB && !bandOf(first.rhs as Operand, cat)) return undefined;
  // with %B the band line the close comes back past is the trigger's first band comparison
  const { out, offset: _o, ...ind } = pctB ? { ...bbOf(first.lhs as SeriesOperand & Obj), out: (parts.find((p) => (p.rhs as Obj)?.out)?.rhs as Obj)?.out } : first.rhs as SeriesOperand & Obj;
  const candle = parts.find((p) => (p.rhs as Obj)?.s === "open"), back = parts.find((p) => (p.rhs as Obj)?.out === out);
  const c = (x.cancel as Obj | undefined)?.rhs as Obj | undefined;
  const b: Band = { ind: ind as SeriesOperand, from: String(out), dir: first.op === "<" ? "below" : "above", candle: !candle ? "any" : candle.op === ">" ? "green" : "red",
    prev: !!(back?.rhs as Obj)?.offset, inside: parts.some((p) => p !== back && ["upper", "lower"].includes(String((p.rhs as Obj)?.out))),
    cancel: c?.out === "upper" ? "upper" : c?.out === "lower" ? "lower" : "none", periodEnd: x.period_end !== undefined, extra: extra as RuleNode[],
    ...(pctB ? { pct: (first.rhs as { value: number }).value } : {}) };
  return JSON.stringify(buildBand(b)) === JSON.stringify(x) ? b : undefined;
}

/** Defaults as in the AFL: entry after a close below the lower band, on a green candle back above the previous bar's lower band,
 *  still under the upper band, cancelled by a close above the upper band; the exit mirrors it from the upper band. */
export function defaultBand(o: SeriesOperand, exit: boolean): RuleNode {
  const { out: _out, offset: _off, ...ind } = o, pctB = o.s === "bbpct";
  return buildBand({ ind: pctB ? bbOf(o as SeriesOperand & Obj) : ind as SeriesOperand, from: exit ? "upper" : "lower", dir: exit ? "above" : "below", candle: exit ? "red" : "green",
    prev: true, inside: true, cancel: exit ? "lower" : "upper", periodEnd: true, extra: [], ...(pctB ? { pct: exit ? 1 : 0 } : {}) });
}

/** "Simple compare": the setup as one crossing (%B crossing its level, or the close crossing the band line). */
export function simpleOf(b: Band): RuleNode {
  const tf = b.ind.tf && b.ind.tf !== "D" ? { tf: b.ind.tf } : {}, op = b.dir === "below" ? "crosses_below" : "crosses_above";
  return (b.pct !== undefined ? { lhs: { s: "bbpct", ...(b.ind.p ? { p: b.ind.p } : {}), ...tf }, op, rhs: { value: b.pct } }
    : { lhs: { s: "close", ...tf }, op, rhs: { ...b.ind, out: b.from } }) as unknown as RuleNode;
}

/** A band's lines by their place, not the engine's output names (basis is the middle band). */
export const BAND_NAMES: Record<string, string> = { upper: "Upper band", basis: "Middle band", lower: "Lower band" };
const lineName = (o: string) => BAND_NAMES[o] ?? `Middle band (${o})`;

export function bandWords(n: RuleNode, cat: Catalogue): string | undefined {
  const b = parseBand(n, cat), ind = b && cat.indicators.find((i) => i.id === b.ind.s);
  if (!b || !ind) return undefined;
  const p = ind.params.filter((q) => !isChoice(q)).map((q) => b.ind.p?.[q.key] ?? q.default).join(", ");
  const tf = b.ind.tf === "W" ? "weekly " : b.ind.tf === "M" ? "monthly " : "", band = `the ${tf}${ind.name} (${p}) ${lineName(b.from).toLowerCase()}`;
  const back = `${b.candle === "any" ? "a" : `a ${b.candle}`} candle closes back ${b.dir === "below" ? "above" : "below"} ${b.pct !== undefined ? band : "it"}${b.prev ? " (previous bar's value)" : ""}`;
  return `after ${b.pct !== undefined ? `${tf}%B (${p}) ${b.dir} ${b.pct}` : `a close ${b.dir} ${band}`}, ${back}${b.inside ? `, still ${b.dir === "below" ? "below the upper" : "above the lower"} band` : ""}`
    + `${b.cancel === "none" ? "" : `; a close ${b.cancel === "upper" ? "above the upper" : "below the lower"} band cancels it`}${b.periodEnd && tf ? ` (at ${tf === "weekly " ? "week" : "month"} end)` : ""}`;
}

/** The band setup as one parameter list, like ParamRule; a setup written another way shows its three parts. */
export function BandRule({ node, catalogue: cat, label, exit, onChange, onOff, remove, renderNode, onSimple }: {
  node: RuleNode; catalogue: Catalogue; label: string; exit: boolean; onChange(n: RuleNode): void; onOff?: ReactNode; remove?: ReactNode;
  renderNode(n: RuleNode, put: (n: RuleNode) => void, name: string): ReactNode; onSimple(): void;
}) {
  const x = node as Obj, off = x.off === true, b = parseBand(node, cat), ind = b && cat.indicators.find((i) => i.id === b.ind.s);
  const put = (patch: Partial<Band>) => b && onChange({ ...buildBand({ ...b, ...patch }), ...(off ? { off: true } : {}) } as RuleNode);
  const row = (name: string, el: ReactNode, key = name) => <div className="lab-param-row" key={key}><span className="lab-param-k">{name}</span><span className="lab-param-v">{el}</span></div>;
  const head = <div className="lab-param-h">{onOff}<b>{label}</b><span className="lab-tag">Band setup</span>
    <span className="lab-param-tools"><button type="button" className="dbtn sm ghost" data-lab-band-simple onClick={onSimple} title="Back to a plain comparison with the band">Simple compare</button>{remove}</span></div>;
  if (!b || !ind) return <div className={`lab-param lab-node lab-band${off ? " lab-off" : ""}`} data-lab-node data-lab-kind="setup">
    {head}
    {(["setup", "trigger", "cancel"] as const).filter((k) => x[k] !== undefined).map((k) =>
      <div className="lab-param-line" key={k}><p className="cap lab-param-either">{k === "setup" ? "Setup" : k === "trigger" ? "Then, on a later bar" : "Cancel when"}</p>
        {renderNode(x[k] as RuleNode, (n) => onChange({ ...x, [k]: n } as RuleNode), `${label} › ${k}`)}</div>)}
  </div>;
  const computed = cat.indicators.filter((i) => i.computed && isBand(i)), groups = [...new Set(computed.map((i) => i.category))];
  const named = (n: string) => `${label} ${n}`, tf = b.ind.tf ?? "D", lines = [...new Set(["lower", middleOf(ind), "upper"].filter(Boolean) as string[])];
  const word = exit ? "Exit" : "Enter";
  return <div className={`lab-param lab-node lab-band${off ? " lab-off" : ""}`} data-lab-node data-lab-kind="setup" data-lab-band>
    {head}
    {row("Indicator", <IndicatorSearch ind={ind} computed={computed} groups={groups} onPick={(id) => put({ ind: { s: id, ...(b.ind.tf ? { tf: b.ind.tf } : {}) } as SeriesOperand, pct: id === "bb" ? b.pct : undefined })} />)}
    {ind.params.filter((p) => !isChoice(p)).map((p) => row(paramLabel(p.label),
      <NumField hideLabel label={named(paramLabel(p.label))} value={b.ind.p?.[p.key] ?? p.default} min={p.min} max={p.max} step={p.step}
        onChange={(n) => n !== undefined && n >= p.min && n <= p.max && put({ ind: { ...b.ind, p: { ...b.ind.p, [p.key]: n } } })} />, p.key))}
    {row("Timeframe", <TimeframeField operand={b.ind} catalogue={cat} onChange={(o) => put({ ind: o as SeriesOperand })} />)}
    {row("Setup", <>
      {b.ind.s === "bb" ? <select aria-label={named("setup reads")} value={b.pct !== undefined ? "pct" : "close"} onChange={(e) => put({ pct: e.target.value === "pct" ? (b.dir === "below" ? 0 : 1) : undefined })}>
        <option value="close">Close</option><option value="pct">%B</option></select> : <span className="lab-word">Close</span>}
      <select aria-label={named("setup side")} value={b.dir} onChange={(e) => put({ dir: e.target.value as Band["dir"] })}>
        <option value="below">below</option><option value="above">above</option></select>
      {b.pct !== undefined
        ? <NumField hideLabel label={named("%B level")} value={b.pct} min={-5} max={5} step={0.1} onChange={(n) => n !== undefined && put({ pct: n })} />
        : <select aria-label={named("band line")} value={b.from} onChange={(e) => put({ from: e.target.value })}>
          {lines.map((o) => <option key={o} value={o}>{lineName(o)}</option>)}</select>}
    </>)}
    {row("Then", <>
      <select aria-label={named("candle")} value={b.candle} onChange={(e) => put({ candle: e.target.value as Band["candle"] })}>
        <option value="green">Green candle</option><option value="red">Red candle</option><option value="any">Any candle</option></select>
      <span className="lab-word">closes back {b.dir === "below" ? "above" : "below"} the</span>
      {b.pct !== undefined ? <select aria-label={named("band line")} value={b.from} onChange={(e) => put({ from: e.target.value })}>
        {lines.map((o) => <option key={o} value={o}>{lineName(o).toLowerCase()}</option>)}</select> : <span className="lab-word">band</span>}
    </>)}
    {row("Band value", <Segmented label={named("band value")} value={b.prev ? "prev" : "now"} onChange={(v) => put({ prev: v === "prev" })}
      options={[{ value: "prev", label: "Previous bar" }, { value: "now", label: "Same bar" }]} />)}
    {row("Inside bands", <label className="lab-check"><input type="checkbox" aria-label={named("inside bands")} checked={b.inside} onChange={(e) => put({ inside: e.target.checked })} />
      Close also {b.dir === "below" ? "below the upper" : "above the lower"} band</label>)}
    {row("Cancel when", <select aria-label={named("cancel")} value={b.cancel} onChange={(e) => put({ cancel: e.target.value as Band["cancel"] })}>
      <option value="upper">Close above the upper band</option><option value="lower">Close below the lower band</option><option value="none">Never</option></select>)}
    {(tf === "W" || tf === "M") && row(`${word} at`, <Segmented label={named("signal at")} value={b.periodEnd ? "end" : "bar"} onChange={(v) => put({ periodEnd: v === "end" })}
      options={[{ value: "end", label: tf === "W" ? "Week end" : "Month end" }, { value: "bar", label: "Any day" }]} />)}
    {b.extra.map((n, i) => <div className="lab-param-line" key={i}>
      {i === 0 && <p className="cap lab-param-either">The setup bar also needs</p>}
      {renderNode(n, (m) => put({ extra: b.extra.map((k, j) => (j === i ? m : k)) }), `${label} › setup ${i + 2}`)}
      <button type="button" className="dbtn sm ghost lab-param-remove" aria-label={`${label} remove setup condition ${i + 1}`} onClick={() => put({ extra: b.extra.filter((_, j) => j !== i) })}>Remove</button>
    </div>)}
    <button type="button" className="dbtn sm ghost lab-param-addline" data-lab-band-extra title="For example ROC of this or another symbol, as in the AFL's ROC filter"
      onClick={() => put({ extra: [...b.extra, { lhs: { s: "roc", p: { length: 1 }, ...(b.ind.tf ? { tf: b.ind.tf } : {}) }, op: "<=", rhs: { value: -2 } } as RuleNode] })}>+ Setup condition</button>
  </div>;
}
