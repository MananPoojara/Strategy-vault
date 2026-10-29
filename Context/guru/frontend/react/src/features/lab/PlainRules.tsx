import { useState } from "react";
import { NumField } from "./OperandEditor";
import { multiCompare } from "./ParamRule";
import { isSupertrendExit, nCandleOf } from "./ExitChoice";
import { describeNode } from "./rule-tree";
import { bandWords } from "./BandRule";
import { ordinal, ruleWords } from "./spec-model";
import { alwaysIn, defaultExitParts, MAX_DEFAULT_COUNT, stopsOn, tradeBlockers, usesDefaultExit, validDefaultExit, type DefaultExit } from "./defaults";
import { SHORT } from "./simple";
import type { Catalogue, Indicator, RuleNode, Spec } from "./types";
import "./plain-rules.css";

type Obj = Record<string, unknown>;
const isOn = (n: RuleNode) => (n as Obj).off !== true;
const isFilter = (n: RuleNode) => (n as Obj).filter === true;
/** A rule as words; a group or a trend falls back to the rule tree's description. */
export function words(n: RuleNode, cat: Catalogue): { text: string; when: boolean } {
  const bare = { ...(n as Obj) }; delete bare.off; delete bare.filter; delete bare.filter_exit;
  if (isSupertrendExit(bare as RuleNode)) { const p = ((bare.lhs as Obj).p ?? {}) as Record<string, number>; return { text: `Supertrend (${p.atr_length ?? 10}, ${p.factor ?? 3}) turns ${bare.op === "crosses_below" ? "up" : "down"}`, when: true }; }
  // one line against several: "the 9-day EMA crosses above the 14-day EMA and the 21-day EMA"
  const many = multiCompare(bare as RuleNode), parts = many?.kids.map((k) => ruleWords(k as RuleNode, cat)?.text);
  // the shared start ("the 9-day EMA crosses above ") is the sentence for a comparison with 0, without the 0
  const probe = many && ruleWords({ ...many.kids[0], rhs: { value: 0 } } as RuleNode, cat)?.text, head = probe && probe.endsWith(" 0") ? probe.slice(0, -1) : undefined;
  if (many && head && parts?.every((t) => t?.startsWith(head)))
  {
    const tails = (parts as string[]).map((t) => t.slice(head.length)), musts = tails.filter((_, i) => many.must[i]), eithers = tails.filter((_, i) => !many.must[i]);
    const anyOne = eithers.length > 1 ? `${musts.length ? "either " : ""}${join(eithers, "or")}` : eithers.join("");
    return { text: head + (!musts.length ? anyOne : join(eithers.length ? [...musts, anyOne] : musts, "and")), when: true };
  }
  // N candle on a weekly / monthly timeframe: "after 3 weekly candles", "at the 3rd weekly up close after entry"
  const nc = nCandleOf(bare as RuleNode);
  if (nc && nc.tf !== "D") {
    const per = nc.tf === "W" ? "weekly" : "monthly";
    return { text: nc.kind === "any" ? `after ${nc.n} ${per} candle${nc.n === 1 ? "" : "s"}` : `at the ${ordinal(nc.n)} ${per} ${nc.kind} close after entry`, when: false };
  }
  const band = bandWords(bare as RuleNode, cat);
  if (band) return { text: band, when: true };
  return ruleWords(bare as RuleNode, cat) ?? { text: describeNode(bare as RuleNode, cat), when: true };
}
const join = (xs: string[], word: string) => xs.length < 2 ? xs.join("") : `${xs.slice(0, -1).join(", ")} ${word} ${xs[xs.length - 1]}`;

/** Everything the test will do, as sentences, including rows switched off and the settings that can stop a trade. */
export function StrategyInWords({ spec, catalogue, period, onChange }: {
  spec: Spec; catalogue: Catalogue; period?: { from?: string; to?: string; latest?: string }; onChange(spec: Spec): void;
}) {
  const both = spec.direction === "both" && !!spec.short_side;
  const sides = [{ s: spec, short: spec.direction === "short", other: false, title: both ? "Long" : "" },
    ...(both ? [{ s: spec.short_side!, short: true, other: true, title: "Short" }] : [])];
  const off = [...spec.entry.all, ...spec.exit.any, ...(spec.short_side ? [...spec.short_side.entry.all, ...spec.short_side.exit.any] : [])].filter((n) => !isOn(n));
  const blockers = tradeBlockers(spec, period);
  return <section className="lab-words" data-lab-words aria-label="Your strategy in plain words">
    <h3>Your strategy in plain words</h3>
    {sides.map(({ s, short, other, title }) => {
      const buy = s.entry.all.filter((n) => isOn(n) && !isFilter(n)).map((n) => words(n, catalogue).text);
      const only = s.entry.all.filter((n) => isOn(n) && isFilter(n)).map((n) => words(n, catalogue).text);
      const sells = s.exit.any.filter(isOn).map((n) => words(n, catalogue)).map((w) => w.when ? `when ${w.text}` : w.text);
      // the N-bar stop is a sell row ("after N days"), already among the sells
      const priced = stopsOn(s.exit as unknown as Obj, undefined).map((w) => `at the ${w}`);
      const fallback = usesDefaultExit(spec, other);
      const [open, close] = short ? ["Sell short", "Cover"] : ["Buy", "Sell"];
      return <div key={title || "main"} className="lab-words-side" data-lab-words-side={title || "main"}>
        {title && <h4>{title}</h4>}
        <p data-lab-words-buy><b>{open}</b> {buy.length ? <>when {join(buy, "and")}.</> : <span className="lab-words-missing">— no {short ? "short-sale" : "buy"} rule yet. Press + Add condition below.</span>}</p>
        {only.length > 0 && <p><b>Only if</b> {join(only, "and")} <span className="cap">(filter)</span>.</p>}
        <p data-lab-words-sell><b>{close}</b> {[...sells, ...priced].length ? <>{join([...sells, ...priced], "or")}.</>
          : fallback ? <span className="lab-words-missing">— no exit yet. Pick one in the Exit card.</span>
          : alwaysIn(spec) ? <span className="cap">when the opposite signal comes (always in the market: each signal flips the position).</span>
          : <span className="cap">when the date window or a filter closes the trade.</span>}</p>
      </div>;
    })}
    {off.length > 0 && <p className="cap" data-lab-words-off>Switched off, not used: {off.map((n) => words(n, catalogue).text).join("; ")}.</p>}
    {blockers.length > 0 && <div className="lab-words-blockers" data-lab-words-blockers>
      <p className="cap">Also switched on. Each of these can stop a trade from opening:</p>
      <ul>{blockers.map((b) => <li key={b.id}>{b.text}{b.off && <button type="button" className="dbtn sm ghost" onClick={() => onChange(b.off!(spec))}>Turn off</button>}</li>)}</ul>
    </div>}
  </section>;
}
/** An indicator drawn on the chart, offered as a first rule with the same settings. */
export interface ChartPick { indicator: Indicator; params: Record<string, number> }
/** "RSI 14", "EMA 50", "MACD 12": the short name and the first parameter. */
export const pickName = (ind: Indicator, p: Record<string, number>) => {
  const first = ind.params[0], n = first ? p[first.key] ?? first.default : undefined;
  return `${SHORT[ind.id] ?? ind.name}${n !== undefined ? ` ${n}` : ""}`;
};
/** An indicator the rules use, with every parameter (defaults filled in) so it compares with a chart chip. */
interface RuleIndicator { ind: Indicator; p: Record<string, number> }
export function ruleIndicators(spec: Spec, cat: Catalogue): RuleIndicator[] {
  const out = new Map<string, RuleIndicator>();
  const visit = (x: unknown) => {
    if (Array.isArray(x)) { x.forEach(visit); return; }
    if (!x || typeof x !== "object") return;
    const o = x as Obj, ind = typeof o.s === "string" && !o.vs && !o.symbol && (!o.tf || o.tf === "D") ? cat.indicators.find((i) => i.id === o.s && i.computed) : undefined;
    if (ind) {
      const p = Object.fromEntries(ind.params.map((q) => [q.key, ((o.p ?? {}) as Record<string, number>)[q.key] ?? q.default]));
      out.set(`${ind.id}:${JSON.stringify(p)}`, { ind, p });
    }
    Object.values(o).forEach(visit);
  };
  visit([spec.entry.all, spec.exit.any, spec.short_side?.entry.all ?? [], spec.short_side?.exit.any ?? []]);
  return [...out.values()];
}
/** Whether the chart and the rules use an indicator with the same settings, with one click to make them match. */
export function ChartMatch({ spec, catalogue, chips, onSpec, onChartSet, onChartAdd }: {
  spec: Spec; catalogue: Catalogue; chips: { key: string; id: string; params: Record<string, number> }[];
  onSpec(spec: Spec): void; onChartSet(key: string, params: Record<string, number>): void; onChartAdd(ind: Indicator, params: Record<string, number>): void;
}) {
  const used = ruleIndicators(spec, catalogue);
  if (!used.length) return null;
  const same = (a: Record<string, number>, b: Record<string, number>) => Object.keys(a).every((k) => a[k] === b[k]);
  const name = (r: RuleIndicator) => pickName(r.ind, r.p);
  // the rule's operands of this indicator with these settings take the chart's settings instead
  const useChart = (r: RuleIndicator, params: Record<string, number>) => {
    const walk = (x: unknown): unknown => Array.isArray(x) ? x.map(walk) : x && typeof x === "object" ? (() => {
      const o = Object.fromEntries(Object.entries(x as Obj).map(([k, v]) => [k, walk(v)]));
      if (o.s !== r.ind.id || o.vs || o.symbol) return o;
      const full = Object.fromEntries(r.ind.params.map((q) => [q.key, ((o.p ?? {}) as Record<string, number>)[q.key] ?? q.default]));
      return same(full, r.p) ? { ...o, p: { ...params } } : o;
    })() : x;
    return walk(spec) as Spec;
  };
  return <div className="lab-chart-match" data-lab-chart-match>
    <span className="cap">Chart and rules:</span>
    {used.map((r) => {
      const onChart = chips.filter((c) => c.id === r.ind.id), match = onChart.find((c) => same(r.p, { ...r.p, ...c.params }) && same({ ...r.p, ...c.params }, r.p));
      const key = `${r.ind.id}:${JSON.stringify(r.p)}`;
      if (match) return <span key={key} className="lab-match-ok" data-lab-match={r.ind.id}>{name(r)} ✓ same as the chart</span>;
      if (onChart.length) {
        const c = onChart[0], chartP = { ...r.p, ...c.params };
        return <span key={key} className="lab-match-diff" data-lab-match={r.ind.id}>Chart {pickName(r.ind, chartP)}, rule {name(r)}:
          <button type="button" className="dbtn sm" onClick={() => onSpec(useChart(r, chartP))}>Use the chart's in the rule</button>
          <button type="button" className="dbtn sm" onClick={() => onChartSet(c.key, { ...c.params, ...r.p })}>Set the chart to {name(r)}</button></span>;
      }
      return <span key={key} className="lab-match-diff" data-lab-match={r.ind.id}>{name(r)} is not on the chart
        <button type="button" className="dbtn sm" onClick={() => onChartAdd(r.ind, r.p)}>Show on chart</button></span>;
    })}
  </div>;
}
