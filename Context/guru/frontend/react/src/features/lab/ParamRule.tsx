import { createContext, useContext, useState, type ReactNode } from "react";
import { IndicatorSearch, NumField, TimeframeField } from "./OperandEditor";
import { OPERAND_KIND_LABELS, paramLabel } from "./labels";
import { OPERATOR_LABELS, isSeries, isValue } from "./spec-model";
import { CALENDAR_IDS, convertOperand, nodeKind, operandKind, withOp } from "./rule-tree";
import { isChoice, variantChoices } from "./ParamsDialog";
import { Segmented } from "./ui";
import { useForeignScope } from "./ForeignFields";
import { PARAM_NAMES, PICK_DEFAULTS } from "./catalogue";
import { BAND_NAMES, isBand } from "./BandRule";
import { foreignEnabled, foreignOptions, withForeignField } from "./foreign-model";
import type { Catalogue, Operand, RuleNode, SeriesOperand } from "./types";

type Obj = Record<string, unknown>;

/** On inside the Entry / Filters / Exit cards: plain conditions show as a parameter list there. */
export const ParamListContext = createContext(false);
export const useParamList = () => useContext(ParamListContext);

/** A condition the parameter list shows in full: a price or indicator (read on this or another symbol, any bars ago),
 *  compared with a threshold, price or indicator. Pair symbols and period timing open the full editor. */
export function plainRule(node: RuleNode, cat: Catalogue): boolean {
  const many = multiCompare(node);
  if (many) return many.kids.every((k) => plainRule(k as RuleNode, cat));
  if (nodeKind(node) !== "cmp") return false;
  const x = node as Obj;
  const plain = (o: Operand, threshold: boolean, lhs = false) => {
    const k = operandKind(o, cat), y = o as Obj;
    if (k === "number") return threshold;
    if (k !== "price" && k !== "indicator") return false;
    const ind = k === "indicator" ? cat.indicators.find((i) => i.id === (o as SeriesOperand).s) : undefined;
    return (lhs || y.symbol === undefined) && y.vs === undefined && y.fixup === undefined && (y.expand ?? "last") === "last"
      && !ind?.needs.second_symbol;
  };
  return plain(x.lhs as Operand, false, true) && plain(x.rhs as Operand, true);
}

const cmpRow = (k: unknown, first: Obj) => nodeKind(k as RuleNode) === "cmp" && Object.keys(k as Obj).length === 3 && JSON.stringify((k as Obj).lhs) === JSON.stringify(first.lhs) && (k as Obj).op === first.op;
const anyOf = (g: unknown): Obj[] | undefined => (g && typeof g === "object" && Object.keys(g).length === 1 && Array.isArray((g as Obj).any) ? (g as Obj).any as Obj[] : undefined);
/** One line compared with several, each line Required (it has to hold) or Any one (one of those lines has to hold), sharing the
 *  left side and the condition: {all: rows} (all Must), {any: rows} (all Either), or {all: [Must rows…, {any: Either rows}]}. */
export function multiCompare(node: RuleNode): { kids: Obj[]; must: boolean[] } | undefined {
  const x = node as Obj, key = Array.isArray(x.all) ? "all" : Array.isArray(x.any) ? "any" : undefined;
  if (!key || Object.keys(x).some((k) => k !== key && k !== "off")) return undefined;
  const list = x[key] as Obj[], either = key === "all" ? anyOf(list[list.length - 1]) : undefined;
  const kids = either ? [...list.slice(0, -1), ...either] : list;
  const must = either ? kids.map((_, i) => i < list.length - 1) : kids.map(() => key === "all");
  return kids.length >= 2 && (!either || list.length >= 2) && kids.every((k) => cmpRow(k, kids[0])) ? { kids, must } : undefined;
}

/** An oscillator in its own pane (RSI, Stochastic, CCI, ADX…) is read against a number, so its rule shows only the Threshold;
 *  price-chart lines (moving averages, bands) and indicators with a signal line keep Compare with. */
const thresholdOnly = (lhs: Operand, rhs: Operand, cat: Catalogue) => {
  const ind = isSeries(lhs) ? cat.indicators.find((i) => i.id === lhs.s) : undefined;
  return isValue(rhs) && !!ind && ind.placement !== "price" && !ind.outputs.some((o) => o === "signal" || o === "trigger");
};

/** One condition as AmiBroker's Parameters window shows it: one setting per line (Indicator, Period, Timeframe, Condition,
 *  Threshold). More adds Bars ago, Symbol and the calculation variant; it starts open when the rule already uses one. */
export function ParamRule({ node, catalogue: cat, label, onChange, onLhs, onOff, remove, onBand }: {
  node: RuleNode; catalogue: Catalogue; label: string; onChange(n: RuleNode): void;
  /** A band indicator is in the rule: offer the band setup (close outside the band, then back inside on a later bar). */
  onBand?(): void;
  /** The left side changed (picking an indicator also sets its usual comparison). */
  onLhs(o: Operand): void; onOff?: ReactNode; remove?: ReactNode;
}) {
  const x = node as Obj, off = x.off === true, scope = useForeignScope(), many = multiCompare(node);
  const first = many ? many.kids[0] : x, lhs = first.lhs as Operand, op = first.op as string;
  const rhss = many ? many.kids.map((k) => k.rhs as Operand) : [x.rhs as Operand], must = many?.must ?? [true];
  /** One comparison, or the same left side and condition against each line: the Must lines all, and one of the Either lines. */
  const build = (l: Operand, o: string, rs: Operand[], m: boolean[]) => {
    const row = (r: Operand) => ({ lhs: l, op: o, rhs: r }), musts = rs.filter((_, i) => m[i]).map(row), eithers = rs.filter((_, i) => !m[i]).map(row);
    const body = rs.length === 1 ? row(rs[0]) : !eithers.length ? { all: musts } : !musts.length ? { any: eithers } : { all: [...musts, { any: eithers }] };
    return { ...body, ...(off ? { off: true } : {}) } as RuleNode;
  };
  const putRhs = (i: number) => (o: Operand) => onChange(many ? build(lhs, op, rhss.map((r, j) => (j === i ? o : r)), must) : { ...x, rhs: o } as RuleNode);
  const addLine = () => onChange(build(lhs, op, [...rhss, rhss[rhss.length - 1]], [...must, true]));
  const removeLine = (i: number) => onChange(build(lhs, op, rhss.filter((_, j) => j !== i), must.filter((_, j) => j !== i)));
  const used = !!many || [lhs, ...rhss].some((o) => {
    const s = isSeries(o as Operand) ? o as SeriesOperand : undefined, ind = s && cat.indicators.find((i) => i.id === s.s);
    return !!s && (!!s.offset || !!s.symbol || (ind?.params ?? []).some((p) => isChoice(p) && (s.p?.[p.key] ?? p.default) !== p.default));
  });
  const [more, setMore] = useState(used);
  const extra = more || used;
  const symbols = scope && foreignEnabled(scope) && scope.allowSymbol !== false && !scope.portfolio ? foreignOptions(scope) : [];
  const computed = cat.indicators.filter((i) => i.computed && !CALENDAR_IDS.includes(i.id));
  const groups = [...new Set(computed.map((i) => i.category))];
  const row = (name: string, el: ReactNode, key = name) => <div className="lab-param-row" key={key}><span className="lab-param-k">{name}</span><span className="lab-param-v">{el}</span></div>;
  const side = (o: Operand, put: (o: Operand) => void, who: string, threshold: boolean, fixed = false, isLhs = false, title = who): ReactNode[] => {
    const kind = operandKind(o, cat), s = isSeries(o) ? o : undefined, ind = s && cat.indicators.find((i) => i.id === s.s);
    const named = (n: string) => `${label} ${who.toLowerCase()} ${n}`;
    return [
      !fixed && row(title, <select aria-label={named("type")} value={kind} onChange={(e) => put(convertOperand(o, e.target.value as "price", cat))}>
        {threshold && <option value="number">{OPERAND_KIND_LABELS.number}</option>}
        <option value="indicator">{OPERAND_KIND_LABELS.indicator}</option>
        <option value="price">{OPERAND_KIND_LABELS.price}</option>
      </select>, `${who}-kind`),
      kind === "number" && isValue(o) && row("Threshold",
        <NumField hideLabel label={named("threshold")} value={o.value} onChange={(n) => n !== undefined && put({ ...o, value: n })} />, `${who}-value`),
      kind === "price" && s && row("Price", <select aria-label={named("price")} value={s.s} onChange={(e) => put({ ...s, s: e.target.value })}>
        {cat.price_sources.map((p) => <option key={p} value={p}>{p}</option>)}
      </select>, `${who}-price`),
      ...(kind === "indicator" && s && ind ? [
        row("Indicator", <IndicatorSearch ind={ind} computed={computed} groups={groups} onPick={(id) => {
          const { p: _p, out: _out, ...rest } = s;
          // a new indicator starts on AmiBroker's calculation (its choice settings from AB_DEFAULTS) and the team's AFL numbers where set
          const picked = computed.find((i) => i.id === id);
          const ab = Object.fromEntries((picked?.params ?? []).filter(isChoice).flatMap((q) => {
            const opts = variantChoices(q, q.default), own = opts.find(([v]) => v === q.default);
            const amibroker = own && /AmiBroker/.test(own[1]) ? undefined : opts.find(([, text]) => /AmiBroker/.test(text));
            return amibroker ? [[q.key, amibroker[0]]] : [];
          }));
          const p = { ...ab, ...PICK_DEFAULTS[id] };
          put({ ...rest, s: id, ...(Object.keys(p).length ? { p } : {}) });
        }} />, `${who}-ind`),
        ...ind.params.filter((p) => !isChoice(p) || extra).map((p) => isChoice(p) ? row(PARAM_NAMES[ind.id]?.[p.key] ?? "Calculation",
          <select aria-label={named("calculation")} value={s.p?.[p.key] ?? p.default} onChange={(e) => put({ ...s, p: { ...s.p, [p.key]: Number(e.target.value) } })}>
            {variantChoices(p, s.p?.[p.key] ?? p.default).map(([v, text]) => <option key={v} value={v}>{text}</option>)}
          </select>, `${who}-${p.key}`) : row(PARAM_NAMES[ind.id]?.[p.key] ?? paramLabel(p.label),
          <NumField hideLabel label={named(paramLabel(p.label))} value={s.p?.[p.key] ?? p.default} min={p.min} max={p.max} step={p.step}
            onChange={(n) => n !== undefined && n >= p.min && n <= p.max && put({ ...s, p: { ...s.p, [p.key]: n } })} />, `${who}-${p.key}`)),
        ind.outputs.length > 1 && row(isBand(ind) ? "Band" : "Line", <select aria-label={named("line")} value={s.out ?? ind.outputs[0]} onChange={(e) => put({ ...s, out: e.target.value })}>
          {ind.outputs.map((v) => <option key={v} value={v}>{(isBand(ind) && BAND_NAMES[v]) || v}</option>)}
        </select>, `${who}-out`),
      ] : []),
      s && row("Timeframe", <TimeframeField operand={o} catalogue={cat} onChange={put} />, `${who}-tf`),
      s && extra && row((s.tf ?? "D") === "D" ? "Bars ago" : "Periods ago", <NumField hideLabel label={named("bars ago")} value={s.offset ?? 0} min={0} step={1}
        onChange={(n) => { if (n === undefined || !Number.isInteger(n) || n < 0) return; const { offset: _o, ...rest } = s; put(n ? { ...rest, offset: n } : rest); }} />, `${who}-ago`),
      s && isLhs && extra && (symbols.length > 0 || !!s.symbol) && row("Foreign ticker", <select aria-label={named("symbol")} value={s.symbol ?? ""}
        onChange={(e) => put(withForeignField(s, "symbol", e.target.value || undefined))}>
        <option value="">This instrument{scope?.ticker ? ` (${scope.ticker})` : ""}</option>
        {s.symbol && !symbols.some((o) => o.ticker === s.symbol) && <option value={s.symbol}>{s.symbol}</option>}
        {symbols.map((o) => <option key={o.ticker} value={o.ticker} disabled={o.disabled} title={o.reason}>{o.label}</option>)}
      </select>, `${who}-symbol`),
    ];
  };
  return <div className={`lab-param lab-node${off ? " lab-off" : ""}`} data-lab-node data-lab-kind="cmp" data-lab-param data-lab-off={off || undefined}>
    <div className="lab-param-h">
      {onOff}<b>{label}</b>
      <span className="lab-param-tools">
        {!used && <button type="button" className="dbtn sm ghost" data-lab-param-more aria-expanded={more} title="Bars ago, a foreign ticker, calculation variant, more lines to compare with"
          onClick={() => setMore(!more)}>{more ? "Less ▴" : "More ▾"}</button>}
        {remove}
      </span>
    </div>
    {onBand && !many && row("Rule", <Segmented label={`${label} rule type`} value="compare" onChange={(v) => v === "band" && onBand()}
      options={[{ value: "compare", label: "Compare" }, { value: "band", label: "Band setup" }]} />, "band")}
    {side(lhs, many ? (o) => onChange(build(o, op, rhss, must)) : onLhs, "Check", false, false, true)}
    {row("Condition", <select aria-label={`${label} condition`} value={op} onChange={(e) => onChange(many && nodeKind(withOp({ lhs, op, rhs: rhss[0] } as RuleNode, e.target.value)) === "cmp" ? build(lhs, e.target.value, rhss, must) : withOp(many ? build(lhs, op, [rhss[0]], must) : node, e.target.value))}>
      {[...new Set([...cat.operators, op])].map((o) => <option key={o} value={o}>{OPERATOR_LABELS[o] ?? o}</option>)}
    </select>)}
    {thresholdOnly(lhs, rhss[0], cat) && !many ? side(rhss[0], putRhs(0), "Compare with", true, true) : <div className="lab-param-comparisons">
      {rhss.map((r, i) => many
        ? <div className="lab-param-line" key={i}>
            {!must[i] && (i === 0 || must[i - 1]) && <p className="cap lab-param-either" data-lab-either-head>{must.some(Boolean) ? "Plus any one of these:" : "Any one of these:"}</p>}
            {side(r, putRhs(i), `Compare with ${i + 1}`, true, false, false, `Line ${i + 1}`)}
            {row("This line is", <Segmented label={`${label} line ${i + 1} is`} value={must[i] ? "must" : "either"}
              options={[{ value: "must", label: "Required" }, { value: "either", label: "Any one" }]} onChange={(v) => onChange(build(lhs, op, rhss, must.map((m, j) => (j === i ? v === "must" : m))))} />, `must-${i}`)}
            <button type="button" className="dbtn sm ghost lab-param-remove" aria-label={`${label} remove line ${i + 1}`} onClick={() => removeLine(i)}>Remove line {i + 1}</button></div>
        : side(r, putRhs(i), "Compare with", true))}
      {many && <p className="cap lab-param-rule" data-lab-must-rule>{must.every(Boolean) ? "Every line is required." : !must.some(Boolean) ? "Any one line is enough." : `${must.filter(Boolean).length > 1 ? "The required lines" : "The required line"}, plus any one of the others.`}</p>}
      {extra && rhss.length < 5 && <button type="button" className="dbtn sm ghost lab-param-addline" data-lab-add-line onClick={addLine}>+ Compare with another line</button>}
    </div>}
  </div>;
}
