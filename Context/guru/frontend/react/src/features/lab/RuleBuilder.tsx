import { COPY } from "./labels";
import { NumField, OperandEditor } from "./OperandEditor";
import { OrdersPanel } from "./OrdersPanel";
import { addCond, conditionCount, conds, isCond, isSeries, OPERATOR_LABELS, removeCond, setCond, setExecution, type Side } from "./spec-model";
import type { Catalogue, Cond, Spec } from "./types";
import { ExecutionPanel } from "./ExecutionPanel";
import { RiskPanel } from "./RiskPanel";
import { RuleList, type TreeCaps } from "./NodeEditor";
import { budget, limits, readsSessionDates, usesHigherTimeframes, type ListPath } from "./rule-tree";
import { instrumentNotes, modeOf, entryOrderType, type InstrumentContext } from "./execution-model";
import { getAt, setAt, specBytes, SPEC_BYTE_BUDGET } from "./spec-model";
import type { RuleNode } from "./types";
import "./rules.css";

export interface RunSettings { from: string; to: string; fees: string; slippage: string }

const NEW_COND: Record<Side, Cond> = {
  entry: { lhs: { s: "close" }, op: ">", rhs: { s: "ema", p: { length: 50 } } },
  exit: { lhs: { s: "bars_held" }, op: ">=", rhs: { value: 5 } },
};
const SIDES: { side: Side; title: string; tag: string }[] = [
  { side: "entry", title: COPY.entryTitle, tag: "Entry" },
  { side: "exit", title: COPY.exitTitle, tag: "Exit" },
];

export function RuleBuilder(props: {
  spec: Spec; catalogue: Catalogue; instrument?: InstrumentContext; onChange(spec: Spec): void;
  settings: RunSettings; onSettings(patch: Partial<RunSettings>): void;
  problems: string[]; running: boolean; onRun(): void;
}) {
  // Older engines keep their existing editor until they advertise the extended grammar.
  if (!props.catalogue.execution) return <LegacyRuleBuilder {...props} />;
  return <TreeRuleBuilder {...props} />;
}
function TreeRuleBuilder({ spec, catalogue, instrument = { kind: "index", market: "in" }, onChange, settings, onSettings, problems, running, onRun }: Parameters<typeof RuleBuilder>[0]) {
  const both = spec.direction === "both", b = budget(spec), lim = limits(catalogue), bytes = specBytes(spec);
  const caps = (top: number): TreeCaps => ({ leaves: b.leaves >= lim.leaves, nodes: b.nodes >= lim.nodes, top: top >= lim.top });
  const warn = b.leaves >= 0.9 * lim.leaves || b.nodes >= 0.9 * lim.nodes || bytes >= 0.9 * SPEC_BYTE_BUDGET || b.top.long >= lim.top || b.top.short >= lim.top;
  const top = `Top-level ${b.top.long}/${lim.top}${both ? ` · short ${b.top.short}/${lim.top}` : ""}`;
  const list = (path: ListPath, title: string, tag: string, count: number) => <RuleList listPath={path} title={title} tag={tag} catalogue={catalogue} caps={caps(count)} list={(getAt(spec, path) as RuleNode[]) ?? []} onChange={(nodes) => onChange(setAt(spec, path, nodes))} />;
  const money = modeOf(spec) === "money", cost = spec.execution?.cost_bps;
  const ordered = entryOrderType(spec.entry) !== "market" || (!!spec.short_side && entryOrderType(spec.short_side.entry) !== "market");
  const fill = ordered ? "order level" : spec.execution?.fill === "close" ? "same close" : "next open";
  return <section className="box lab-rules" tabIndex={-1}>
    <h2 className="box-h">Strategy rules <small>{COPY.rulesHint}</small><span className={`cap lab-caps${warn ? " lab-caps-warn" : ""}`} data-lab-caps>{COPY.caps(top, b.leaves, lim.leaves, b.nodes, lim.nodes, (bytes / 1024).toFixed(1))}</span></h2>
    <ExecutionPanel spec={spec} catalogue={catalogue} onChange={onChange} />
    <div className="lab-side-panel" data-lab-side="long">
      {both && <h3 className="lab-side-h">{COPY.longSide}</h3>}
      <div className="lab-sides">{list("entry.all", both ? COPY.longEntryTitle : COPY.entryTitle, both ? "Long entry" : "Entry", b.top.long)}{list("exit.any", both ? COPY.longExitTitle : COPY.exitTitle, both ? "Long exit" : "Exit", b.top.long)}</div>
      <OrdersPanel spec={spec} catalogue={catalogue} pre="" onChange={onChange} />
    </div>
    {both && spec.short_side && <div className="lab-side-panel" data-lab-side="short">
      <h3 className="lab-side-h">{COPY.shortSide}</h3>
      <div className="lab-sides">{list("short_side.entry.all", COPY.shortEntryTitle, "Short entry", b.top.short)}{list("short_side.exit.any", COPY.shortExitTitle, "Short exit", b.top.short)}</div>
      <OrdersPanel spec={spec} catalogue={catalogue} pre="short_side." onChange={onChange} />
    </div>}
    <RiskPanel spec={spec} catalogue={catalogue} instrument={instrument} onChange={onChange} />
    {instrumentNotes(instrument, spec).map((note, i) => <p className="cap" key={i}>{note}</p>)}
    {readsSessionDates(spec) && <p className="cap">{COPY.sessionDatesHint}</p>}
    {usesHigherTimeframes(spec) && <p className="cap">{COPY.wmRule}</p>}
    <div className="lab-row lab-settings">
      <label className="dfield">From<input type="date" aria-label="From" value={settings.from} onChange={(e) => onSettings({ from: e.target.value })} /></label>
      <label className="dfield">To<input type="date" aria-label="To" value={settings.to} onChange={(e) => onSettings({ to: e.target.value })} /></label>
      {money ? <span className="cap">{COPY.moneyCostsHere}</span> : <>
        {cost !== undefined ? <NumField label="Fees (bps / side)" value={cost} min={0} onChange={(n) => n !== undefined && onChange(setExecution(spec, { cost_bps: n }))} /> : <label className="dfield">Fees (bps / side)<input type="number" aria-label="Fees (bps / side)" min={0} step="any" value={settings.fees} onChange={(e) => onSettings({ fees: e.target.value })} /></label>}
        <label className="dfield">Slippage (bps / side)<input type="number" aria-label="Slippage (bps / side)" min={0} step="any" value={settings.slippage} onChange={(e) => onSettings({ slippage: e.target.value })} /></label>
      </>}
      <button type="button" className="dbtn primary" data-lab-action="run" disabled={running || problems.length > 0} onClick={onRun}>{running ? "Running…" : "Run backtest"}</button>
      <span className="cap">Signal at the bar's close · fill at the {fill}</span>
    </div>
    {problems.length > 0 && <ul className="rnote">{problems.map((p, i) => <li key={i}>{p}</li>)}</ul>}
  </section>;
}

function LegacyRuleBuilder({ spec, catalogue, onChange, settings, onSettings, problems, running, onRun }: {
  spec: Spec; catalogue: Catalogue; onChange(spec: Spec): void;
  settings: RunSettings; onSettings(patch: Partial<RunSettings>): void;
  problems: string[]; running: boolean; onRun(): void;
}) {
  const full = conditionCount(spec) >= catalogue.max_conditions;
  const all = [...spec.entry.all, ...spec.exit.any];
  const weeklyOrMonthly = all.filter(isCond).some((c) => [c.lhs, c.rhs].some((o) => isSeries(o) && o.tf !== undefined && o.tf !== "D"));
  const costBps = spec.execution?.cost_bps;
  const fill = spec.entry.limit ? "limit price" : spec.execution?.fill === "close" ? "same close" : "next open";

  return (
    <section className="box lab-rules" tabIndex={-1}>
      <h2 className="box-h">Strategy rules <small>{COPY.rulesHint}</small></h2>
      <OrdersPanel spec={spec} onChange={onChange} />
      <div className="lab-sides">
      {SIDES.map(({ side, title, tag }) => (
        <div className="lab-side" key={side}>
          <h3>{title}</h3>
          {conds(spec, side).map((c, i) => {
            if (!isCond(c)) return <div className="lab-cond" key={i}>
              <span className="lab-kept">{COPY.keptAsIs} <code>{JSON.stringify(c)}</code></span>
              <button type="button" className="dbtn sm ghost" aria-label={`Remove ${tag} ${i + 1}`} onClick={() => onChange(removeCond(spec, side, i))}>×</button>
            </div>;
            const put = (patch: Partial<Cond>) => onChange(setCond(spec, side, i, { ...c, ...patch }));
            const ops = catalogue.operators.includes(c.op) ? catalogue.operators : [...catalogue.operators, c.op];
            return (
              <div className="lab-cond" key={i}>
                <OperandEditor operand={c.lhs} side={side} catalogue={catalogue} label={`${tag} ${i + 1} left`} onChange={(lhs) => put({ lhs })} />
                <select aria-label="Operator" value={c.op} onChange={(e) => put({ op: e.target.value })}>
                  {ops.map((op) => <option key={op} value={op}>{OPERATOR_LABELS[op] ?? op}</option>)}
                </select>
                <OperandEditor operand={c.rhs} side={side} catalogue={catalogue} label={`${tag} ${i + 1} right`} onChange={(rhs) => put({ rhs })} />
                <button type="button" className="dbtn sm ghost" aria-label={`Remove ${tag} ${i + 1}`} onClick={() => onChange(removeCond(spec, side, i))}>×</button>
              </div>
            );
          })}
          <button type="button" className="dbtn sm" disabled={full} onClick={() => onChange(addCond(spec, side, NEW_COND[side]))}>
            {COPY.addCondition}
          </button>
        </div>
      ))}
      </div>
      {weeklyOrMonthly && <p className="cap">{COPY.wmRule}</p>}
      <div className="lab-row lab-settings">
        <label className="dfield">From<input type="date" aria-label="From" value={settings.from} onChange={(e) => onSettings({ from: e.target.value })} /></label>
        <label className="dfield">To<input type="date" aria-label="To" value={settings.to} onChange={(e) => onSettings({ to: e.target.value })} /></label>
        {costBps !== undefined ? (
          // The spec's own cost wins and the cost query is not sent, so this box edits the spec.
          <NumField label="Fees (bps / side)" value={costBps} min={0} onChange={(n) => n !== undefined && onChange(setExecution(spec, { cost_bps: n }))} />
        ) : (
          <label className="dfield">Fees (bps / side)
            <input type="number" aria-label="Fees (bps / side)" min={0} step="any" value={settings.fees} onChange={(e) => onSettings({ fees: e.target.value })} />
          </label>
        )}
        <label className="dfield">Slippage (bps / side)
          <input type="number" aria-label="Slippage (bps / side)" min={0} step="any" value={settings.slippage} onChange={(e) => onSettings({ slippage: e.target.value })} />
        </label>
        <button type="button" className="dbtn primary" data-lab-action="run" disabled={running || problems.length > 0} onClick={onRun}>
          {running ? "Running…" : "Run backtest"}
        </button>
        <span className="cap">Signal at the bar's close · fill at the {fill}</span>
      </div>
      {problems.length > 0 && <ul className="rnote">{problems.map((p, i) => <li key={i}>{p}</li>)}</ul>}
    </section>
  );
}
