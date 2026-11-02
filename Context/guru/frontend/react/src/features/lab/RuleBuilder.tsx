import { Fragment, useId, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import { COPY } from "./labels";
import { NumField, OperandEditor } from "./OperandEditor";
import { OrdersPanel } from "./OrdersPanel";
import { nBarStop, usesTradePrices } from "./AmiSettings";
import { addCond, conditionCount, conds, isCond, isSeries, OPERATOR_LABELS, removeCond, setCond, setExecution, type Side } from "./spec-model";
import type { Catalogue, Cond, Spec } from "./types";
import { ExecutionPanel } from "./ExecutionPanel";
import { DateField } from "./DateField";
import { RiskPanel } from "./RiskPanel";
import { BacktestParams } from "./settings/BacktestParams";
import { GeneralTab } from "./settings/GeneralTab";
import { ordered, TradesTab } from "./settings/TradesTab";
import { StopsTab } from "./settings/StopsTab";
import { ReportTab } from "./settings/ReportTab";
import { PortfolioTab } from "./settings/PortfolioTab";
import { WalkForwardTab } from "./settings/WalkForwardTab";
import { SettingGroup } from "./settings/common";
import { RuleList, type TreeCaps } from "./NodeEditor";
import { budget, limits, newFilter, readsSessionDates, usesHigherTimeframes, type ListPath } from "./rule-tree";
import { modeOf, entryOrderType, type InstrumentContext } from "./execution-model";
import { getAt, setAt, specBytes, SPEC_BYTE_BUDGET } from "./spec-model";
import type { RuleNode } from "./types";
import { useCompact, useSimple } from "./simple";
import { ParamListContext } from "./ParamRule";
import { StrategyInWords } from "./PlainRules";
import { ExitChoice, isIndicatorExit } from "./ExitChoice";
import { defaultDates } from "./DateFilterPanel";
import { changedSettings, stopsOn } from "./defaults";
import "./rules.css";

export interface RunSettings { from: string; to: string; fees: string; slippage: string; /** the instrument's latest data date (To's default) */ latest?: string }

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
  /** Shown at the right of the Settings heading (the Simple / Advanced switch). */
  settingsAside?: ReactNode;
  /** Opens the optimizer for these rules (the Optimize button beside Run backtest). */
  onOptimize?(): void;
  /** The portfolio controls (PortfolioPanel), shown on the Portfolio tab. */
  portfolio?: ReactNode;
  /** The custom date filter (DateFilterPanel), shown with the test period. */
  dates?: ReactNode;
  /** Step layout: "rules" (Your idea) or "settings" (Settings) only; both by default. */
  part?: "rules" | "settings";
  /** The exit a test uses when no sell rule is on (defaults.ts), editable here; absent: no fallback is shown. */
  /** Whether the chart shows the rules' indicators with the same settings (PlainRules ChartMatch). */
  chartMatch?: ReactNode;
}) {
  // Older engines keep their existing editor until they advertise the extended grammar.
  if (!props.catalogue.execution) return props.part === "settings" ? <p className="box cap">This engine shows the test period and costs with the rules, in step 2.</p> : <LegacyRuleBuilder {...props} />;
  return <TreeRuleBuilder {...props} />;
}
const isFilterRow = (n: RuleNode) => (n as Record<string, unknown>).filter === true;
/** The Indicator filter on (one condition to set, after the buy rows) or off (its rows removed). */
const withFilters = (spec: Spec, path: ListPath, on: boolean) => {
  const all = (getAt(spec, path) as RuleNode[]) ?? [];
  if (on === all.some(isFilterRow)) return spec;
  return setAt(spec, path, on ? [...all, newFilter()] : all.filter((n) => !isFilterRow(n)));
};
/** The Dates filter on (Base 1's bull periods to start) or off. */
const withDates = (spec: Spec, on: boolean): Spec => {
  if (on === !!spec.date_filter) return spec;
  if (on) return { ...spec, date_filter: defaultDates() };
  const { date_filter: _, ...rest } = spec;
  return rest as Spec;
};
/** No filter (the default), Dates and Indicator: the last two can be on together. Instant toggles, no motion: used often. */
function FilterChoice({ dates, indicator, onDates, onIndicator, onNone }: { dates?: boolean; indicator: boolean; onDates(on: boolean): void; onIndicator(on: boolean): void; onNone(): void }) {
  const chip = (label: string, on: boolean, click: () => void) => <button type="button" className="lab-filter-chip" aria-pressed={on} onClick={click}>{label}</button>;
  return <div className="lab-filter-choice" role="group" aria-label="Filters" data-lab-filter-choice>
    {chip("No filter", !dates && !indicator, onNone)}
    {dates !== undefined && chip("Dates", dates, () => onDates(!dates))}
    {chip("Indicator", indicator, () => onIndicator(!indicator))}
  </div>;
}

function TreeRuleBuilder({ spec, catalogue, instrument = { kind: "index", market: "in" }, onChange, settings, onSettings, problems, running, onRun, onOptimize, portfolio, part, dates, chartMatch, settingsAside }: Parameters<typeof RuleBuilder>[0]) {
  const showRules = part !== "settings", showSettings = part !== "rules";
  const both = spec.direction === "both", b = budget(spec), lim = limits(catalogue), bytes = specBytes(spec);
  const single = !spec.portfolio; // per-side prices, re-entry delays and limits are single-instrument settings
  const caps = (top: number): TreeCaps => ({ leaves: b.leaves >= lim.leaves, nodes: b.nodes >= lim.nodes, top: top >= lim.top });
  const warn = b.leaves >= 0.9 * lim.leaves || b.nodes >= 0.9 * lim.nodes || bytes >= 0.9 * SPEC_BYTE_BUDGET || b.top.long >= lim.top || b.top.short >= lim.top;
  const top = `Top-level ${b.top.long}/${lim.top}${both ? ` · short ${b.top.short}/${lim.top}` : ""}`;
  /** The Exit card: N candle, ATR, Supertrend, HSL, Profit booking and Indicator (its own sell rows, the others kept apart);
   *  the other AmiBroker exits (breakeven, take-profits, two ATR stops…) stay folded under More exit settings in Advanced. */
  const list = (path: ListPath, title: string, tag: string, count: number) => {
    const short = path.startsWith("short_side.") || spec.direction === "short", pre = path.startsWith("short_side.") ? "short_side." : "";
    const rows = (getAt(spec, path) as RuleNode[]) ?? [], mine = rows.filter(isIndicatorExit), kept = rows.filter((r) => !isIndicatorExit(r));
    return <div className="lab-list" data-lab-list={path}>
      <h3>{title}</h3>
      <ExitChoice spec={spec} pre={pre} onChange={onChange} indicatorList={<RuleList listPath={path} title="Indicator" tag={tag} catalogue={catalogue} caps={caps(count)} list={mine}
        onChange={(next) => onChange(setAt(spec, path, [...kept, ...next]))} short={short} />} />
      <OrdersPanel spec={spec} catalogue={catalogue} pre={pre} part="exits" brief={simple} extrasOnly onChange={onChange} />
    </div>;
  };
  /** Exits that close a trade by price (stops, targets, N-bar, more exits) sit with the sell rules; the entry order type and
   *  scale-in (limit and stop orders are for live-style fills) in Settings. Folded unless one is set, the heading names what is on. */
  const ordersBox = (pre: "" | "short_side.", part: "entry" | "exits") => {
    const side = pre ? spec.short_side : spec;
    if (!side) return null;
    const on = part === "entry" ? [entryOrderType(side.entry) !== "market" ? `${entryOrderType(side.entry).replace("_", "-")} order` : "", side.entry.scale_in ? "scale in" : ""].filter(Boolean)
      : stopsOn(side.exit as unknown as Record<string, unknown>, nBarStop(spec, pre));
    // Order type is not offered in Settings; it shows only when a loaded strategy already sets one, so it can be seen and cleared
    if (part === "entry" && !on.length) return null;
    return <details className="lab-orders-box" data-lab-orders-box={part} open={on.length > 0}>
      <summary>{part === "entry" ? "Order type" : "More exit settings"} <span className="cap">· {on.length ? on.join(", ") : part === "entry" ? "market order (buy at the fill price)" : "none (optional)"}</span></summary>
      <OrdersPanel spec={spec} catalogue={catalogue} pre={pre} reentry={single && !pre} part={part} brief={simple} onChange={onChange} />
    </details>;
  };
  const entryOrders = () => <>{ordersBox("", "entry")}{spec.short_side && ordersBox("short_side.", "entry")}</>;
  // the AFLs' filter sections: entry.all rows marked `filter`, kept after the signal rows so those keep their entry.all[i] paths
  const entryLists = (path: ListPath, title: string, tag: string, count: number) => {
    const all = (getAt(spec, path) as RuleNode[]) ?? [], isFilter = (n: RuleNode) => (n as Record<string, unknown>).filter === true;
    const signals = all.filter((n) => !isFilter(n)), filters = all.filter(isFilter), short = path.startsWith("short_side.") || spec.direction === "short";
    const one = (rows: RuleNode[], keep: (rows: RuleNode[]) => RuleNode[], title: string, tag: string, f = false, notice?: ReactNode) => <RuleList key={tag} listPath={path} title={title} tag={tag}
      catalogue={catalogue} caps={caps(count)} list={rows} onChange={(nodes) => onChange(setAt(spec, path, keep(nodes)))} short={short} filters={f} notice={notice} />;
    // the filter rows show once the Indicator filter is on (FilterChoice); the card heading names the section
    const filterList = filters.length > 0 && one(filters, (n) => [...signals, ...n], "Indicator",
      path.startsWith("short_side.") ? "Short filter" : "Filter", true);
    return [<Fragment key={`${tag} signals`}>{one(signals, (n) => [...n, ...filters], title, tag)}</Fragment>, filterList] as const;
  };
  const simple = useSimple(), compact = useCompact();
  const money = modeOf(spec) === "money", cost = spec.execution?.cost_bps;
  const fill = ordered(spec) ? "order level" : usesTradePrices(spec.execution) ? "price and delay set in Trades" : spec.execution?.fill === "close" ? "same close" : "next open";
  const changed = changedSettings(spec);
  const hasFilters = (path: ListPath) => ((getAt(spec, path) as RuleNode[]) ?? []).some(isFilterRow);
  /** The same entry, filter and exit controls, grouped into readable sections for each side. */
  const cards = (short: boolean) => {
    const pre = short ? "short_side." : "", count = short ? b.top.short : b.top.long;
    const [entry, filters] = entryLists(`${pre}entry.all` as ListPath, short ? "Short entry" : both ? "Long entry" : "Entry", short ? "Short entry" : both ? "Long entry" : "Entry", count);
    // the section navigator's counts: buy rows, filter rows (and the date filter), exit rules plus built-in exits (stops, targets, ...)
    const rows = (getAt(spec, `${pre}entry.all` as ListPath) as RuleNode[]) ?? [], exitSpec = (getAt(spec, `${pre}exit`) ?? {}) as Record<string, unknown>;
    const nFilters = rows.filter(isFilterRow).length + (!short && spec.date_filter ? 1 : 0);
    const nExit = ((exitSpec.any as RuleNode[] | undefined) ?? []).length + Object.entries(exitSpec).filter(([k, v]) => k !== "any" && v !== undefined).length;
    const id = (k: string) => `lab-card-${short ? "short-" : ""}${k}`;
    const sections: [string, string, number][] = [["entry", short ? "Short entry" : both ? "Long entry" : "Entry", rows.length - rows.filter(isFilterRow).length],
      ["exit", short ? "Short exit" : both ? "Long exit" : "Exit", nExit], ["filters", short ? "Short filters" : "Filters", nFilters]];
    const go = (k: string) => { const el = document.getElementById(id(k)); el?.scrollIntoView?.({ block: "start" }); el?.focus({ preventScroll: true }); };
    return <div className="lab-side-panel" data-lab-side={short ? "short" : "long"}>
      {both && <h3 className="lab-side-h">{short ? COPY.shortSide : COPY.longSide}</h3>}
      <nav className="lab-rule-nav" aria-label={`${short ? "Short" : both ? "Long" : "Rule"} sections`} data-lab-rule-nav>
        {sections.map(([k, label, n]) => <button key={k} type="button" onClick={() => go(k)} aria-label={`${label}: ${n} ${n === 1 ? "rule" : "rules"}`}>{label}<b aria-hidden="true">{n}</b></button>)}
      </nav>
      <ParamListContext.Provider value={true}><div className="lab-rule-cards">
        <div className="lab-rule-card" data-lab-card="entry" id={id("entry")} tabIndex={-1}>
          <header className="lab-rule-section-h"><h3>{short ? "Short entry" : both ? "Long entry" : "Entry"}</h3></header>
          <div className="lab-rule-section-body">{entry}</div>
        </div>
        <div className="lab-rule-card" data-lab-card="exit" id={id("exit")} tabIndex={-1}>
          <header className="lab-rule-section-h"><h3>{short ? "Short exit" : both ? "Long exit" : "Exit"}</h3></header>
          <div className="lab-rule-section-body">{list(`${pre}exit.any` as ListPath, short ? "Short exit" : both ? "Long exit" : "Exit", short ? "Short exit" : both ? "Long exit" : "Exit", count)}</div>
        </div>
        <div className="lab-rule-card" data-lab-card="filters" id={id("filters")} tabIndex={-1}>
          <header className="lab-rule-section-h"><h3>{short ? "Short filters" : "Filters"}</h3></header>
          <div className="lab-rule-section-body">
          <h3>{short ? "Short filters" : "Filters"}</h3>
          <FilterChoice dates={short || !dates ? undefined : !!spec.date_filter} indicator={hasFilters(`${pre}entry.all` as ListPath)}
            onDates={(on) => onChange(withDates(spec, on))} onIndicator={(on) => onChange(withFilters(spec, `${pre}entry.all` as ListPath, on))}
            onNone={() => onChange(withFilters(withDates(spec, false), `${pre}entry.all` as ListPath, false))} />
          {!short && spec.date_filter && dates}
          {filters}
          </div>
        </div>
      </div></ParamListContext.Provider>
    </div>;
  };
  const testDates = <>
    <label className="dfield">From<DateField label="From" title="Empty: from the first bar" value={settings.from} onChange={(from) => onSettings({ from })} /></label>
    <label className="dfield">To<DateField label="To" title="Empty: to the last bar" value={settings.to} onChange={(to) => onSettings({ to })} /></label>
  </>;
  // % mode costs (AFL "BROKERAGE", 0 by default); money mode charges General › Commissions and the slippage in Backtest parameters
  const costs = money ? null : <>
    {cost !== undefined ? <NumField label="Brokerage (bps / side)" value={cost} min={0} placeholder="0" onChange={(n) => n !== undefined && onChange(setExecution(spec, { cost_bps: n }))} /> : <label className="dfield">Brokerage (bps / side)<input type="number" aria-label="Brokerage (bps / side)" min={0} step="any" placeholder="0" value={settings.fees} onChange={(e) => onSettings({ fees: e.target.value })} /></label>}
    <label className="dfield">Slippage (bps / side)<input type="number" aria-label="Slippage (bps / side)" min={0} step="any" placeholder="0" value={settings.slippage} onChange={(e) => onSettings({ slippage: e.target.value })} /></label>
  </>;
  const run = <button type="button" className="dbtn primary" data-lab-action="run" disabled={running || problems.length > 0} title={problems[0] ?? `Signal at the bar's close · fill at the ${fill}`} onClick={onRun}>{!running && <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true"><path d="M4 2.5v11L13 8 4 2.5Z" /></svg>}{running ? "Running…" : "Run backtest"}</button>;
  // the run and Optimize buttons sit at the right of the Positions · From · To row (Your idea only), with how trades fill beside them
  // after the rules are set, Optimize tries ranges of their numbers (AmiBroker's Optimize beside Backtest)
  const optimize = onOptimize && <button type="button" className="dbtn" data-lab-open-optimize disabled={problems.length > 0} title={problems[0] ?? "Try ranges of this strategy's numbers"} onClick={onOptimize}>Optimize</button>;
  // how trades fill is the Run backtest tooltip, not a line of text beside it
  const runTop = <span className="lab-run-top">{optimize}{run}</span>;
  return <section className={`box lab-rules lab-design-studio${showRules ? " lab-idea" : ""}`} tabIndex={-1}>
    {showRules && <>
    <h2 className="box-h">Strategy rules{!simple && <small>{COPY.rulesHint}</small>}{((!simple && !compact) || warn) && <span className={`cap lab-caps${warn ? " lab-caps-warn" : ""}`} data-lab-caps>{COPY.caps(top, b.leaves, lim.leaves, b.nodes, lim.nodes, (bytes / 1024).toFixed(1))}</span>}</h2>
    <StrategyInWords spec={spec} catalogue={catalogue} period={settings} onChange={onChange} />
    {chartMatch}
    {<div className="lab-row lab-idea-basics" data-lab-idea-basics><ExecutionPanel spec={spec} catalogue={catalogue} onChange={onChange} part="positions" />{testDates}{runTop}</div>}
    {cards(false)}
    {both && spec.short_side && cards(true)}
    </>}
    {part === "settings" && <h2 className="box-h">Settings{settingsAside && <span className="lab-settings-aside">{settingsAside}</span>}</h2>}
    {showSettings && <div className="lab-settings-body" data-lab-settings-body>
    {part !== "settings" && <h3 className="lab-settings-h">Backtest settings <small>how trades are filled, sized and charged</small></h3>}
    {/* the basics, in both views; Simple leaves out the AmiBroker settings below them */}
    <div className="lab-basics" data-lab-basics>
      {/* the AFLs' Backtest parameter panel; Buy and sell at is the Scenario on the Trades tab, as in AmiBroker */}
      <BacktestParams spec={spec} catalogue={catalogue} instrument={instrument} onChange={onChange} unitCosts={costs} orders={entryOrders()} />
    </div>
    {!simple && <>
    <section className="lab-more-settings lab-ami" data-lab-more-settings aria-label="AmiBroker settings">
      <h3 className="lab-ami-h">AmiBroker settings <span className="cap">{changed.length ? `· ${changed.length} changed: ${changed.join(", ")}` : "· all at their defaults"}</span></h3>
    {/* AmiBroker's Analysis settings window, tab by tab in its order; the lab's own position size and risk follow General's fields */}
    <SettingsTabs rail labels={["General", "Trades", "Stops", "Report", "Portfolio", "Walk-Forward"]}>
      <>
        <p className="cap lab-stab-intro">In AmiBroker's order: the data, the backtest options, commissions and rates, then the lab's position size and risk.</p>
        <GeneralTab spec={spec} catalogue={catalogue} instrument={instrument} onChange={onChange} />
        <SettingGroup title="Position size & risk"><RiskPanel spec={spec} catalogue={catalogue} instrument={instrument} onChange={onChange} /></SettingGroup>
      </>
      <TradesTab spec={spec} catalogue={catalogue} onChange={onChange} />
      <StopsTab spec={spec} onChange={onChange} />
      <ReportTab spec={spec} instrument={instrument} onChange={onChange} />
      <PortfolioTab spec={spec} catalogue={catalogue} instrument={instrument} onChange={onChange} portfolio={portfolio} />
      <WalkForwardTab spec={spec} />
    </SettingsTabs></section></>}
    </div>}
    {readsSessionDates(spec) && <p className="cap">{COPY.sessionDatesHint}</p>}
    {usesHigherTimeframes(spec) && <p className="cap">{COPY.wmRule}</p>}
    {problems.length > 0 && <ul className="rnote">{problems.map((p, i) => <li key={i}>{p}</li>)}</ul>}
  </section>;
}

function LegacyRuleBuilder({ spec, catalogue, onChange, settings, onSettings, problems, running, onRun, onOptimize }: {
  spec: Spec; catalogue: Catalogue; onChange(spec: Spec): void;
  settings: RunSettings; onSettings(patch: Partial<RunSettings>): void;
  problems: string[]; running: boolean; onRun(): void; onOptimize?(): void;
}) {
  const full = conditionCount(spec) >= catalogue.max_conditions;
  const all = [...spec.entry.all, ...spec.exit.any];
  const weeklyOrMonthly = all.filter(isCond).some((c) => [c.lhs, c.rhs].some((o) => isSeries(o) && o.tf !== undefined && o.tf !== "D"));
  const costBps = spec.execution?.cost_bps;
  const fill = spec.entry.limit ? "limit price" : spec.execution?.fill === "close" ? "same close" : "next open";

  return (
    <section className="box lab-rules" tabIndex={-1}>
      <h2 className="box-h">Strategy rules</h2>
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
        <label className="dfield">From<DateField label="From" title="Empty: from the first bar" value={settings.from} onChange={(from) => onSettings({ from })} /></label>
        <label className="dfield">To<DateField label="To" title="Empty: to the last bar" value={settings.to} onChange={(to) => onSettings({ to })} /></label>
        {costBps !== undefined ? (
          // The spec's own cost wins and the cost query is not sent, so this box edits the spec.
          <NumField label="Brokerage (bps / side)" value={costBps} min={0} onChange={(n) => n !== undefined && onChange(setExecution(spec, { cost_bps: n }))} />
        ) : (
          <label className="dfield">Brokerage (bps / side)
            <input type="number" aria-label="Brokerage (bps / side)" min={0} step="any" value={settings.fees} onChange={(e) => onSettings({ fees: e.target.value })} />
          </label>
        )}
        <label className="dfield">Slippage (bps / side)
          <input type="number" aria-label="Slippage (bps / side)" min={0} step="any" value={settings.slippage} onChange={(e) => onSettings({ slippage: e.target.value })} />
        </label>
        <button type="button" className="dbtn primary" data-lab-action="run" disabled={running || problems.length > 0} title={`Signal at the bar's close · fill at the ${fill}`} onClick={onRun}>
          {running ? "Running…" : "Run backtest"}
        </button>
        {onOptimize && <button type="button" className="dbtn" data-lab-open-optimize disabled={problems.length > 0} onClick={onOptimize}>Optimize</button>}
      </div>
      {problems.length > 0 && <ul className="rnote">{problems.map((p, i) => <li key={i}>{p}</li>)}</ul>}
    </section>
  );
}

/** AmiBroker Analysis settings: tabs over the execution controls; every panel stays mounted (hidden) so edits are never lost.
 *  `rail`: a vertical category list beside the panel when the settings card is wide (orders.css), a scrolling strip when it is not. */
export function SettingsTabs({ labels, children, ariaLabel = "Analysis settings", rail = false }: { labels: string[]; children: ReactNode[]; ariaLabel?: string; rail?: boolean }) {
  const [tab, setTab] = useState(0);
  const id = useId(), list = useRef<HTMLDivElement>(null);
  // ARIA tab pattern: one Tab stop, arrows / Home / End move and select; a rail also takes Up / Down
  const key = (e: KeyboardEvent) => {
    const n = labels.length, next = (tab + 1) % n, prev = (tab + n - 1) % n;
    const to: number | undefined = ({ ArrowRight: next, ArrowLeft: prev, Home: 0, End: n - 1, ...(rail ? { ArrowDown: next, ArrowUp: prev } : {}) } as Record<string, number>)[e.key];
    if (to === undefined) return;
    e.preventDefault(); setTab(to);
    list.current?.querySelectorAll<HTMLElement>("[role=tab]")[to]?.focus();
  };
  return <div className={rail ? "lab-settings-tabs lab-settings-rail" : "lab-settings-tabs"} data-lab-settings-tabs>
    <div className="lab-stabs" role="tablist" aria-label={ariaLabel} aria-orientation={rail ? "vertical" : undefined} ref={list} onKeyDown={key}>
      {labels.map((label, i) => <button key={label} type="button" role="tab" id={`${id}-tab-${i}`} aria-controls={`${id}-panel-${i}`} aria-selected={tab === i}
        tabIndex={tab === i ? 0 : -1} onClick={() => setTab(i)}>{label}</button>)}
    </div>
    {children.map((child, i) => <div key={labels[i]} role="tabpanel" id={`${id}-panel-${i}`} aria-labelledby={`${id}-tab-${i}`} className="lab-stab-panel" hidden={tab !== i}>{child}</div>)}
  </div>;
}
