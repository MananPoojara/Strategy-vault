import type { ReactNode } from "react";
import { NumField } from "./OperandEditor";
import { describeNode } from "./rule-tree";
import { ordinal, ruleWords } from "./spec-model";
import { alwaysIn, defaultExitParts, firstRule, MAX_DEFAULT_COUNT, ownExit, STARTER_PICKS, stopsOn, tradeBlockers, usesDefaultExit, validDefaultExit, type DefaultExit } from "./defaults";
import { SHORT } from "./simple";
import type { Catalogue, Indicator, RuleNode, Spec } from "./types";
import "./plain-rules.css";

type Obj = Record<string, unknown>;
const isOn = (n: RuleNode) => (n as Obj).off !== true;
const isFilter = (n: RuleNode) => (n as Obj).filter === true;
/** A rule as words; a group or a trend falls back to the rule tree's description. */
export function words(n: RuleNode, cat: Catalogue): { text: string; when: boolean } {
  const bare = { ...(n as Obj) }; delete bare.off; delete bare.filter; delete bare.filter_exit;
  return ruleWords(bare as RuleNode, cat) ?? { text: describeNode(bare as RuleNode, cat), when: true };
}
const join = (xs: string[], word: string) => xs.length < 2 ? xs.join("") : `${xs.slice(0, -1).join(", ")} ${word} ${xs[xs.length - 1]}`;

/** Everything the test will do, as sentences, including rows switched off and the settings that can stop a trade. */
export function StrategyInWords({ spec, catalogue, defaultExit, period, onChange }: {
  spec: Spec; catalogue: Catalogue; defaultExit: DefaultExit; period?: { from?: string; to?: string; latest?: string }; onChange(spec: Spec): void;
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
        <p data-lab-words-buy><b>{open}</b> {buy.length ? <>when {join(buy, "and")}.</> : <span className="lab-words-missing">— no {short ? "short-sale" : "buy"} rule yet. Pick one below.</span>}</p>
        {only.length > 0 && <p><b>Only if</b> {join(only, "and")} <span className="cap">(filter)</span>.</p>}
        <p data-lab-words-sell><b>{close}</b> {[...sells, ...priced].length ? <>{join([...sells, ...priced], "or")}.</>
          : fallback ? <>{defaultWords(defaultExit, short)}. <span className="lab-tag" title="No sell rule is switched on, so the test uses the default exit">default exit</span></>
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
const defaultWords = (d: DefaultExit, short: boolean) => defaultExitParts(d, short).join(", or ");

/** The sell list's fallback when no sell rule is on: the default exit, each part on or off with its number (kept per browser). */
export function DefaultExitCard({ value, short, onChange, own = [], onOwn }: {
  value: DefaultExit; short: boolean; onChange(d: DefaultExit): void;
  /** The buy rules' indicators that have their own exit (TradingView style), offered instead of the default exit. */
  own?: { label: string; node: RuleNode }[]; onOwn?(node: RuleNode): void;
}) {
  const set = (patch: Partial<DefaultExit>) => { const next = { ...value, ...patch }; if (validDefaultExit(next)) onChange(next); };
  const onCount = [!value.upOff, !value.downOff, !!value.days, !!value.stopPct, !!value.targetPct, !!value.trailPct].filter(Boolean).length;
  const [win, lose] = short ? ["down", "up"] : ["up", "down"];
  // the last part switched on cannot be switched off: a test always has an exit
  const tick = (on: boolean, name: string, toggle: (on: boolean) => void) => <input type="checkbox" aria-label={`Default exit: use ${name}`} checked={on}
    disabled={on && onCount === 1} title={on && onCount === 1 ? "At least one part stays on" : undefined} onChange={(e) => toggle(e.target.checked)} />;
  const num = (v: number, label: string, min: number, max: number, step: number, put: (n: number) => void, off: boolean) =>
    <NumField label={label} hideLabel value={v} min={min} max={max} step={step} disabled={off} onChange={(n) => n !== undefined && n >= min && n <= max && put(n)} />;
  return <div className="lab-default-exit" data-lab-default-exit role="note">
    <p><b>No sell rule yet</b>, so the test uses your <b>default exit</b>, for every indicator. Tick the parts you want:</p>
    <ul className="lab-default-exit-parts">
      <li>{tick(!value.upOff, `the ${win} close`, (on) => set({ upOff: on ? undefined : true }))}{short ? "Cover" : "Sell"} at the{num(value.up, `Default exit: which ${win} close`, 1, MAX_DEFAULT_COUNT, 1, (n) => set({ up: n }), !!value.upOff)}
        <span>{ordinal(value.up).replace(/^\d+/, "")} {win} close after entry</span></li>
      <li>{tick(!value.downOff, `the ${lose} close`, (on) => set({ downOff: on ? undefined : true }))}or at the{num(value.down, `Default exit: which ${lose} close`, 1, MAX_DEFAULT_COUNT, 1, (n) => set({ down: n }), !!value.downOff)}
        <span>{ordinal(value.down).replace(/^\d+/, "")} {lose} close (a losing streak)</span></li>
      <li>{tick(!!value.days, "days in the trade", (on) => set({ days: on ? 10 : undefined }))}or after{num(value.days ?? 10, "Default exit: days in the trade", 1, 1000, 1, (n) => set({ days: n }), !value.days)}<span>days in the trade</span></li>
      <li>{tick(!!value.stopPct, "a stop-loss", (on) => set({ stopPct: on ? 5 : undefined }))}or at a{num(value.stopPct ?? 5, "Default exit: stop-loss %", 0.1, 99, 0.5, (n) => set({ stopPct: n }), !value.stopPct)}<span>% stop-loss</span></li>
      <li>{tick(!!value.targetPct, "a profit target", (on) => set({ targetPct: on ? 10 : undefined }))}or at a{num(value.targetPct ?? 10, "Default exit: profit target %", 0.1, 1000, 0.5, (n) => set({ targetPct: n }), !value.targetPct)}<span>% profit target</span></li>
      <li>{tick(!!value.trailPct, "a trailing stop", (on) => set({ trailPct: on ? 5 : undefined }))}or with a{num(value.trailPct ?? 5, "Default exit: trailing stop %", 0.1, 99, 0.5, (n) => set({ trailPct: n }), !value.trailPct)}<span>% trailing stop</span></li>
    </ul>
    {own.length > 0 && onOwn && <p className="lab-starter-row"><span className="cap">Or use:</span>
      {own.map((o) => <button key={o.label} type="button" className="dbtn sm" data-lab-own-exit title={o.label} onClick={() => onOwn(o.node)}>{o.label}</button>)}</p>}
    <p className="cap">Remembered in this browser. Add a sell rule, or Stops &amp; targets below, to use your own exit instead.</p>
  </div>;
}

/** An indicator drawn on the chart, offered as a first rule with the same settings. */
export interface ChartPick { indicator: Indicator; params: Record<string, number> }
/** An empty buy list: one-click first rules from the chart's indicators and the usual starting points. */
export function BuyStarter({ catalogue, chart, short, onPick, children }: {
  catalogue: Catalogue; chart: ChartPick[]; short: boolean;
  /** The indicator's first rule and, when it has one, its own exit (TradingView style). */
  onPick(entry: RuleNode, exit?: RuleNode): void; children?: ReactNode;
}) {
  const popular = STARTER_PICKS.flatMap(({ id, p }) => { const ind = catalogue.indicators.find((i) => i.id === id && i.computed); return ind ? [{ indicator: ind, params: p }] : []; });
  const button = (key: string, { indicator, params }: ChartPick) => {
    const p = shown(indicator, params), entry = firstRule(indicator, p, short), exit = ownExit(indicator, p, short);
    const hint = `${short ? "Sell short" : "Buy"} when ${words(entry, catalogue).text}${exit ? `; ${short ? "cover" : "sell"} ${words(exit, catalogue).when ? "when " : ""}${words(exit, catalogue).text}` : ""}`;
    return <button key={key} type="button" className="dbtn sm" data-lab-pick={key} title={hint} onClick={() => onPick(entry, exit)}>{pickName(indicator, p)}</button>;
  };
  return <div className="lab-starter" data-lab-starter>
    <p><b>No {short ? "short-sale" : "buy"} rule yet.</b> Pick an indicator: it adds its usual {short ? "short-sale" : "buy"} rule and, where it has one, its own exit (as on TradingView). You can change every number afterwards.</p>
    {chart.length > 0 && <p className="lab-starter-row"><span className="cap">From your chart:</span>{chart.map((c, i) => button(`chart-${i}-${c.indicator.id}`, c))}</p>}
    {popular.length > 0 && <p className="lab-starter-row"><span className="cap">Popular:</span>{popular.map((c) => button(c.indicator.id, c))}</p>}
    {children}
  </div>;
}
/** "RSI 14", "EMA 50", "MACD 12": the short name and the first parameter. */
export const pickName = (ind: Indicator, p: Record<string, number>) => {
  const first = ind.params[0], n = first ? p[first.key] ?? first.default : undefined;
  return `${SHORT[ind.id] ?? ind.name}${n !== undefined ? ` ${n}` : ""}`;
};
/** A chart chip's parameters for the rule: the ones changed from the defaults, and always the first (so "RSI(14)" reads whole). */
const shown = (ind: Indicator, params: Record<string, number>) =>
  Object.fromEntries(ind.params.filter((p, i) => params[p.key] !== undefined && (i === 0 || params[p.key] !== p.default)).map((p) => [p.key, params[p.key]]));

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
