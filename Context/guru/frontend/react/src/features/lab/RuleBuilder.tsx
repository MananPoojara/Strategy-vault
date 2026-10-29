import { Fragment, useId, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import { COPY } from "./labels";
import { NumField, OperandEditor } from "./OperandEditor";
import { OrdersPanel } from "./OrdersPanel";
import { nBarStop, toTradePrices, TradePrices, usesTradePrices } from "./AmiSettings";
import { addCond, conditionCount, conds, isCond, isSeries, OPERATOR_LABELS, removeCond, setCond, setExecution, type Side } from "./spec-model";
import type { Catalogue, Cond, ExitAtStop, Spec, StopKey } from "./types";
import { ExecutionPanel } from "./ExecutionPanel";
import { DateField } from "./DateField";
import { GeneralData, InitialEquity, PortfolioLimits, RiskPanel } from "./RiskPanel";
import { ReportSettings } from "./ReportSettings";
import { RuleList, type TreeCaps } from "./NodeEditor";
import { budget, limits, newFilter, readsSessionDates, usesHigherTimeframes, type ListPath } from "./rule-tree";
import { modeOf, entryOrderType, type InstrumentContext } from "./execution-model";
import { getAt, setAt, specBytes, SPEC_BYTE_BUDGET } from "./spec-model";
import type { RuleNode } from "./types";
import { FillChoice, useCompact, useSimple } from "./simple";
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

function TreeRuleBuilder({ spec, catalogue, instrument = { kind: "index", market: "in" }, onChange, settings, onSettings, problems, running, onRun, onOptimize, portfolio, part, dates, chartMatch }: Parameters<typeof RuleBuilder>[0]) {
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
  const ordered = entryOrderType(spec.entry) !== "market" || (!!spec.short_side && entryOrderType(spec.short_side.entry) !== "market");
  const fill = ordered ? "order level" : usesTradePrices(spec.execution) ? "price and delay set in Trades" : spec.execution?.fill === "close" ? "same close" : "next open";
  const ex = spec.execution ?? {};
  const changed = changedSettings(spec);
  const hasFilters = (path: ListPath) => ((getAt(spec, path) as RuleNode[]) ?? []).some(isFilterRow);
  /** The same entry, filter and exit controls, grouped into readable sections for each side. */
  const cards = (short: boolean) => {
    const pre = short ? "short_side." : "", count = short ? b.top.short : b.top.long;
    const [entry, filters] = entryLists(`${pre}entry.all` as ListPath, short ? "Short entry" : both ? "Long entry" : "Entry", short ? "Short entry" : both ? "Long entry" : "Entry", count);
    return <div className="lab-side-panel" data-lab-side={short ? "short" : "long"}>
      {both && <h3 className="lab-side-h">{short ? COPY.shortSide : COPY.longSide}</h3>}
      <ParamListContext.Provider value={true}><div className="lab-rule-cards">
        <div className="lab-rule-card" data-lab-card="entry">
          <header className="lab-rule-section-h"><h3>{short ? "Short entry" : both ? "Long entry" : "Entry"}</h3></header>
          <div className="lab-rule-section-body">{entry}</div>
        </div>
        <div className="lab-rule-card" data-lab-card="filters">
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
        <div className="lab-rule-card" data-lab-card="exit">
          <header className="lab-rule-section-h"><h3>{short ? "Short exit" : both ? "Long exit" : "Exit"}</h3></header>
          <div className="lab-rule-section-body">{list(`${pre}exit.any` as ListPath, short ? "Short exit" : both ? "Long exit" : "Exit", short ? "Short exit" : both ? "Long exit" : "Exit", count)}</div>
        </div>
      </div></ParamListContext.Provider>
    </div>;
  };
  const testDates = <>
    <label className="dfield">From<DateField label="From" title="Empty: from the first bar" value={settings.from} onChange={(from) => onSettings({ from })} /></label>
    <label className="dfield">To<DateField label="To" title="Empty: to the last bar" value={settings.to} onChange={(to) => onSettings({ to })} /></label>
  </>;
  const costs = money ? null : <div className="lab-row lab-settings">
    <>
      {/* Fees are not offered; the box shows only when a fee is already set (a loaded strategy or an earlier entry), so no hidden cost runs */}
      {cost !== undefined ? <NumField label="Fees (bps / side)" value={cost} min={0} placeholder="0" onChange={(n) => n !== undefined && onChange(setExecution(spec, { cost_bps: n }))} /> : Number(settings.fees) ? <label className="dfield">Fees (bps / side)<input type="number" aria-label="Fees (bps / side)" min={0} step="any" placeholder="0" value={settings.fees} onChange={(e) => onSettings({ fees: e.target.value })} /></label> : null}
      <label className="dfield">Slippage (bps / side)<input type="number" aria-label="Slippage (bps / side)" min={0} step="any" placeholder="0" value={settings.slippage} onChange={(e) => onSettings({ slippage: e.target.value })} /></label>
    </>
  </div>;
  const run = <button type="button" className="dbtn primary" data-lab-action="run" disabled={running || problems.length > 0} title={problems[0] ?? `Signal at the bar's close · fill at the ${fill}`} onClick={onRun}>{!running && <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true"><path d="M4 2.5v11L13 8 4 2.5Z" /></svg>}{running ? "Running…" : "Run backtest"}</button>;
  // the run and Optimize buttons sit at the right of the Positions · From · To row (Your idea only), with how trades fill beside them
  // after the rules are set, Optimize tries ranges of their numbers (AmiBroker's Optimize beside Backtest)
  const optimize = onOptimize && <button type="button" className="dbtn" data-lab-open-optimize disabled={problems.length > 0} title={problems[0] ?? "Try ranges of this strategy's numbers"} onClick={onOptimize}>Optimize</button>;
  // how trades fill is the Run backtest tooltip, not a line of text beside it
  const runTop = <span className="lab-run-top">{optimize}{run}</span>;
  const stray = !single && [ex.prices, ex.delays, ex.reentry, ex.limits, ex.exit_at_stop, ex.activate_stops_immediately, ex.nbar_has_priority, ex.allow_same_bar_exit, ex.bars_held_from, ex.reenter_on_exit_bar, ex.defer_exit_on_circuit].some((x) => x !== undefined);
  return <section className="box lab-rules lab-design-studio" tabIndex={-1}>
    {showRules && <>
    <h2 className="box-h">Strategy rules <small>{COPY.rulesHint}</small>{((!simple && !compact) || warn) && <span className={`cap lab-caps${warn ? " lab-caps-warn" : ""}`} data-lab-caps>{COPY.caps(top, b.leaves, lim.leaves, b.nodes, lim.nodes, (bytes / 1024).toFixed(1))}</span>}</h2>
    <StrategyInWords spec={spec} catalogue={catalogue} period={settings} onChange={onChange} />
    {chartMatch}
    {<div className="lab-row lab-idea-basics" data-lab-idea-basics><ExecutionPanel spec={spec} catalogue={catalogue} onChange={onChange} part="positions" />{testDates}{runTop}</div>}
    {cards(false)}
    {both && spec.short_side && cards(true)}
    </>}
    {part === "settings" && <h2 className="box-h">Settings</h2>}
    {showSettings && <>
    {part !== "settings" && <h3 className="lab-settings-h">Backtest settings <small>how trades are filled, sized and charged</small></h3>}
    {/* the basics, in both views; Simple leaves out the AmiBroker settings below them */}
    <div className="lab-basics" data-lab-basics>
      <SettingGroup title="Trade"><FillChoice spec={spec} onChange={onChange} />{entryOrders()}{costs}</SettingGroup>
      <SettingGroup title="Starting money"><InitialEquity spec={spec} catalogue={catalogue} instrument={instrument} onChange={onChange} instrumentData={false} /></SettingGroup>
    </div>
    {!simple && <>
    <section className="lab-more-settings lab-ami" data-lab-more-settings aria-label="AmiBroker settings">
      <h3 className="lab-ami-h">AmiBroker settings <span className="cap">{changed.length ? `· ${changed.length} changed: ${changed.join(", ")}` : "· all at their defaults"}</span></h3>
    <SettingsTabs labels={["General", "Trades", "Report", "Portfolio"]}>
      <>
        <p className="cap lab-stab-intro">Position size and risk, same-bar options and instrument data.</p>
        <SettingGroup title="Position size & risk"><RiskPanel spec={spec} catalogue={catalogue} instrument={instrument} onChange={onChange} /></SettingGroup>
        <SettingGroup title="Trading options">
          <ExecutionPanel spec={spec} catalogue={catalogue} onChange={onChange} part="options" />
          {!spec.portfolio && <GeneralData spec={spec} instrument={instrument} onChange={onChange} />}
        </SettingGroup>
      </>
      <>
        <p className="cap lab-stab-intro">When orders fill and at what price: the bar, the price for each side, any delay, and when stops start checking.</p>
        {!ordered && <ScenarioPreset spec={spec} custom={single} onChange={onChange} />}
        {single && <TradePrices spec={spec} preset={!usesTradePrices(spec.execution)} onChange={onChange} />}
        <ExecutionPanel spec={spec} catalogue={catalogue} onChange={onChange} part="fills" />
        <label className="lab-check" title={single ? undefined : "A portfolio checks stops from the entry bar (a single-instrument setting)"}><input type="checkbox" disabled={!single}
          checked={ex.activate_stops_immediately ?? true} onChange={(e) => onChange(setExecution(spec, { activate_stops_immediately: e.target.checked ? undefined : false }))} />Activate stops immediately</label>
        <p className="cap">{(ex.activate_stops_immediately ?? true) ? "Checked by default here (AmiBroker's default is unchecked): a stop can trigger on the entry bar when the entry fills at the open or as a limit/stop order; an entry at the close or another later price is checked from the next bar." : "Stops are first checked on the bar after the entry bar."} Stops and targets themselves are set under the sell rules in Your idea.</p>
        {stray && <p className="rnote" role="note" data-lab-portfolio-stray>
          Trade prices, re-entry delays, stop exit modes, same-bar and stop activation settings and position limits apply to one instrument only.{" "}
          <button type="button" className="dbtn sm" onClick={() => onChange(setExecution(spec, { prices: undefined, delays: undefined, reentry: undefined, limits: undefined,
            exit_at_stop: undefined, activate_stops_immediately: undefined, nbar_has_priority: undefined, allow_same_bar_exit: undefined, bars_held_from: undefined, reenter_on_exit_bar: undefined, defer_exit_on_circuit: undefined }))}>Remove them</button>
        </p>}
      </>
      <>
        <p className="cap lab-stab-intro">Risk-free rates the performance report uses for the Sharpe ratio of trades and the Ulcer performance index (UPI).</p>
        <ReportSettings spec={spec} onChange={onChange} />
      </>
      <>
        <p className="cap lab-stab-intro">Trade several instruments together with shared cash, and the limits on each position.</p>
        {portfolio}
        <PortfolioLimits spec={spec} onChange={onChange} />
      </>
    </SettingsTabs></section></>}
    </>}
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
        <label className="dfield">From<DateField label="From" title="Empty: from the first bar" value={settings.from} onChange={(from) => onSettings({ from })} /></label>
        <label className="dfield">To<DateField label="To" title="Empty: to the last bar" value={settings.to} onChange={(to) => onSettings({ to })} /></label>
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
        <button type="button" className="dbtn primary" data-lab-action="run" disabled={running || problems.length > 0} title={`Signal at the bar's close · fill at the ${fill}`} onClick={onRun}>
          {running ? "Running…" : "Run backtest"}
        </button>
        {onOptimize && <button type="button" className="dbtn" data-lab-open-optimize disabled={problems.length > 0} onClick={onOptimize}>Optimize</button>}
      </div>
      {problems.length > 0 && <ul className="rnote">{problems.map((p, i) => <li key={i}>{p}</li>)}</ul>}
    </section>
  );
}

/** A labelled group inside a settings tab: the label in a left column so related controls read as one block. */
const SettingGroup = ({ title, children }: { title: string; children: ReactNode }) =>
  <section className="lab-sgroup" aria-label={title}><h4>{title}</h4><div>{children}</div></section>;

/** AmiBroker Analysis settings: tabs over the execution controls; every panel stays mounted (hidden) so edits are never lost. */
export function SettingsTabs({ labels, children, ariaLabel = "Analysis settings" }: { labels: string[]; children: ReactNode[]; ariaLabel?: string }) {
  const [tab, setTab] = useState(0);
  const id = useId(), list = useRef<HTMLDivElement>(null);
  // ARIA tab pattern: one Tab stop, arrows / Home / End move and select
  const key = (e: KeyboardEvent) => {
    const n = labels.length, to = { ArrowRight: (tab + 1) % n, ArrowLeft: (tab + n - 1) % n, Home: 0, End: n - 1 }[e.key];
    if (to === undefined) return;
    e.preventDefault(); setTab(to);
    list.current?.querySelectorAll<HTMLElement>("[role=tab]")[to]?.focus();
  };
  return <div className="lab-settings-tabs" data-lab-settings-tabs>
    <div className="lab-stabs" role="tablist" aria-label={ariaLabel} ref={list} onKeyDown={key}>
      {labels.map((label, i) => <button key={label} type="button" role="tab" id={`${id}-tab-${i}`} aria-controls={`${id}-panel-${i}`} aria-selected={tab === i}
        tabIndex={tab === i ? 0 : -1} onClick={() => setTab(i)}>{label}</button>)}
    </div>
    {children.map((child, i) => <div key={labels[i]} role="tabpanel" id={`${id}-panel-${i}`} aria-labelledby={`${id}-tab-${i}`} className="lab-stab-panel" hidden={tab !== i}>{child}</div>)}
  </div>;
}

/** AmiBroker "Scenario" presets for market orders; `stops` sets Exit at stop? of all four stops (absent: intraday, the default). */
const SCENARIOS: { id: string; label: string; fill?: "next_open" | "close"; stops?: ExitAtStop }[] = [
  { id: "next_open", label: "Trade next bar on open, stops exit intraday", fill: "next_open" },
  { id: "close", label: "Trade current bar on close, stops exit intraday (default)", fill: "close" },
  { id: "next_open_stops_open", label: "Trade next bar on open, stops exit on next bar open", fill: "next_open", stops: 2 },
  { id: "close_stops_close", label: "Trade current bar on close, stops exit on close", fill: "close", stops: 0 },
  { id: "prices", label: "Custom: price and delay for buy, sell, short and cover" },
];
const STOP_KEYS: StopKey[] = ["stop_loss", "target", "trailing", "bars"];
function ScenarioPreset({ spec, custom, onChange }: { spec: Spec; custom: boolean; onChange(spec: Spec): void }) {
  const ex = spec.execution ?? {};
  // Same-bar order (intrabar) is separate from the fill scenario and is kept
  const fill = ex.fill ?? "next_open", plain = (ex.exit_fill ?? fill) === fill && ex.delay_bars === undefined;
  const modes = new Set(STOP_KEYS.map((k) => ex.exit_at_stop?.[k] ?? 1)), stops = modes.size > 1 ? undefined : ({ 0: "_stops_close", 1: "", 2: "_stops_open" } as const)[[...modes][0]];
  const id = plain && stops !== undefined ? fill + stops : "custom";
  const current = usesTradePrices(ex) ? "prices" : SCENARIOS.some((x) => x.id === id) ? id : "custom";
  return <label className="dfield lab-scenario">Scenario
    <select aria-label="Scenario" value={current} onChange={(e) => {
      const pick = SCENARIOS.find((x) => x.id === e.target.value);
      if (pick?.id === "prices") onChange(toTradePrices(spec));
      else if (pick?.fill) onChange(setExecution(spec, { fill: pick.fill === "next_open" ? undefined : pick.fill, exit_fill: undefined, delay_bars: undefined, prices: undefined, delays: undefined,
        exit_at_stop: pick.stops === undefined ? undefined : Object.fromEntries(STOP_KEYS.map((k) => [k, pick.stops])) }));
    }}>
      {current === "custom" && <option value="custom">Custom (see the fields below)</option>}
      {/* stop exit modes are single-instrument settings, like per-side prices */}
      {SCENARIOS.filter((x) => custom || x.id !== "prices" || current === "prices").map((x) => <option key={x.id} value={x.id} disabled={!custom && (x.id === "prices" || x.stops !== undefined)}>{x.label}</option>)}
    </select>
  </label>;
}
