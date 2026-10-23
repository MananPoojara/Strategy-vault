import { useId, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import { COPY } from "./labels";
import { NumField, OperandEditor } from "./OperandEditor";
import { OrdersPanel } from "./OrdersPanel";
import { toTradePrices, TradePrices, usesTradePrices } from "./AmiSettings";
import { addCond, conditionCount, conds, isCond, isSeries, OPERATOR_LABELS, removeCond, setCond, setExecution, type Side } from "./spec-model";
import type { Catalogue, Cond, ExitAtStop, Spec, StopKey } from "./types";
import { ExecutionPanel } from "./ExecutionPanel";
import { DateField } from "./DateField";
import { GeneralData, InitialEquity, PortfolioLimits, RiskPanel } from "./RiskPanel";
import { ReportSettings } from "./ReportSettings";
import { RuleList, type TreeCaps } from "./NodeEditor";
import { budget, limits, readsSessionDates, usesHigherTimeframes, type ListPath } from "./rule-tree";
import { modeOf, entryOrderType, type InstrumentContext } from "./execution-model";
import { getAt, setAt, specBytes, SPEC_BYTE_BUDGET } from "./spec-model";
import type { RuleNode } from "./types";
import { ExamplePicker, SimpleSettings, useCompact, useSimple } from "./simple";
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
  /** The portfolio controls (PortfolioPanel), shown on the Portfolio tab. */
  portfolio?: ReactNode;
  /** The custom date filter (DateFilterPanel), shown with the test period. */
  dates?: ReactNode;
  /** Step layout: "rules" (Your idea) or "settings" (Settings) only; both by default. */
  part?: "rules" | "settings";
}) {
  // Older engines keep their existing editor until they advertise the extended grammar.
  if (!props.catalogue.execution) return props.part === "settings" ? <p className="box cap">This engine shows the test period and costs with the rules, in step 2.</p> : <LegacyRuleBuilder {...props} />;
  return <TreeRuleBuilder {...props} />;
}
function TreeRuleBuilder({ spec, catalogue, instrument = { kind: "index", market: "in" }, onChange, settings, onSettings, problems, running, onRun, portfolio, part, dates }: Parameters<typeof RuleBuilder>[0]) {
  const showRules = part !== "settings", showSettings = part !== "rules";
  const both = spec.direction === "both", b = budget(spec), lim = limits(catalogue), bytes = specBytes(spec);
  const caps = (top: number): TreeCaps => ({ leaves: b.leaves >= lim.leaves, nodes: b.nodes >= lim.nodes, top: top >= lim.top });
  const warn = b.leaves >= 0.9 * lim.leaves || b.nodes >= 0.9 * lim.nodes || bytes >= 0.9 * SPEC_BYTE_BUDGET || b.top.long >= lim.top || b.top.short >= lim.top;
  const top = `Top-level ${b.top.long}/${lim.top}${both ? ` · short ${b.top.short}/${lim.top}` : ""}`;
  const list = (path: ListPath, title: string, tag: string, count: number) => <RuleList listPath={path} title={title} tag={tag} catalogue={catalogue} caps={caps(count)} list={(getAt(spec, path) as RuleNode[]) ?? []} onChange={(nodes) => onChange(setAt(spec, path, nodes))}
    short={path.startsWith("short_side.") || spec.direction === "short"} />;
  // the AFLs' filter sections: entry.all rows marked `filter`, kept after the signal rows so those keep their entry.all[i] paths
  const entryLists = (path: ListPath, title: string, tag: string, count: number) => {
    const all = (getAt(spec, path) as RuleNode[]) ?? [], isFilter = (n: RuleNode) => (n as Record<string, unknown>).filter === true;
    const signals = all.filter((n) => !isFilter(n)), filters = all.filter(isFilter), short = path.startsWith("short_side.") || spec.direction === "short";
    const one = (rows: RuleNode[], keep: (rows: RuleNode[]) => RuleNode[], title: string, tag: string, f = false) => <RuleList key={tag} listPath={path} title={title} tag={tag}
      catalogue={catalogue} caps={caps(count)} list={rows} onChange={(nodes) => onChange(setAt(spec, path, keep(nodes)))} short={short} filters={f} />;
    return [one(signals, (n) => [...n, ...filters], title, tag),
      one(filters, (n) => [...signals, ...n], path.startsWith("short_side.") ? "Short filters · the short sale also needs all of these" : COPY.filtersTitle,
        path.startsWith("short_side.") ? "Short filter" : "Filter", true)] as const;
  };
  const [longEntry, longFilters] = entryLists("entry.all", both ? "Buy (long) when all of these are true" : COPY.simpleEntryTitle, both ? "Long entry" : "Entry", b.top.long);
  const money = modeOf(spec) === "money", cost = spec.execution?.cost_bps;
  const ordered = entryOrderType(spec.entry) !== "market" || (!!spec.short_side && entryOrderType(spec.short_side.entry) !== "market");
  const fill = ordered ? "order level" : usesTradePrices(spec.execution) ? "price and delay set in Trades" : spec.execution?.fill === "close" ? "same close" : "next open";
  const ex = spec.execution ?? {}, single = !spec.portfolio; // per-side prices, re-entry delays and limits are single-instrument settings
  const simple = useSimple(), compact = useCompact();
  const tabbed = compact && !simple; // Advanced in the page: AmiBroker-style Entry / Exit tabs
  const entryTitle = simple ? COPY.simpleEntryTitle : COPY.entryTitle, exitTitle = simple ? COPY.simpleExitTitle : COPY.exitTitle;
  const period = <div className="lab-row lab-settings">
    <label className="dfield">From<DateField label="From" title="Empty: from the first bar" value={settings.from} onChange={(from) => onSettings({ from })} /></label>
    <label className="dfield">To<DateField label="To" title="Empty: to the last bar" value={settings.to} onChange={(to) => onSettings({ to })} /></label>
    {money ? <span className="cap">{COPY.moneyCostsHere}</span> : <>
      {cost !== undefined ? <NumField label="Fees (bps / side)" value={cost} min={0} placeholder="0" onChange={(n) => n !== undefined && onChange(setExecution(spec, { cost_bps: n }))} /> : <label className="dfield">Fees (bps / side)<input type="number" aria-label="Fees (bps / side)" min={0} step="any" placeholder="0" value={settings.fees} onChange={(e) => onSettings({ fees: e.target.value })} /></label>}
      <label className="dfield">Slippage (bps / side)<input type="number" aria-label="Slippage (bps / side)" min={0} step="any" placeholder="0" value={settings.slippage} onChange={(e) => onSettings({ slippage: e.target.value })} /></label>
    </>}
  </div>;
  const stray = !single && [ex.prices, ex.delays, ex.reentry, ex.limits, ex.exit_at_stop, ex.activate_stops_immediately, ex.nbar_has_priority, ex.allow_same_bar_exit, ex.bars_held_from, ex.reenter_on_exit_bar].some((x) => x !== undefined);
  return <section className="box lab-rules" tabIndex={-1}>
    {showRules && <>
    <h2 className="box-h">{simple ? "Your idea" : "Strategy rules"} <small>{COPY.rulesHint}</small>{((!simple && !compact) || warn) && <span className={`cap lab-caps${warn ? " lab-caps-warn" : ""}`} data-lab-caps>{COPY.caps(top, b.leaves, lim.leaves, b.nodes, lim.nodes, (bytes / 1024).toFixed(1))}</span>}</h2>
    {simple && <ExamplePicker spec={spec} onChange={onChange} />}
    {tabbed ? <SettingsTabs ariaLabel="Rules" labels={["Entry · Buy", "Filters", "Exit · Sell", ...(both && spec.short_side ? ["Short entry", "Short filters", "Short exit"] : [])]}>
      {[longEntry, <>{longFilters}{dates && <SettingGroup title="Date filter (Base 1 / Base 2 windows)">{dates}</SettingGroup>}</>,
        list("exit.any", both ? "Sell the long when any of these is true" : COPY.simpleExitTitle, both ? "Long exit" : "Exit", b.top.long),
        ...(both && spec.short_side ? [...entryLists("short_side.entry.all", "Sell short when all of these are true", "Short entry", b.top.short),
          list("short_side.exit.any", "Cover the short when any of these is true", "Short exit", b.top.short)] : [])]}
    </SettingsTabs> : <>
    <div className="lab-side-panel" data-lab-side="long">
      {both && <h3 className="lab-side-h">{COPY.longSide}</h3>}
      <div className="lab-sides">{entryLists("entry.all", both ? COPY.longEntryTitle : entryTitle, both ? "Long entry" : "Entry", b.top.long)}{list("exit.any", both ? COPY.longExitTitle : exitTitle, both ? "Long exit" : "Exit", b.top.long)}</div>
    </div>
    {both && spec.short_side && <div className="lab-side-panel" data-lab-side="short">
      <h3 className="lab-side-h">{COPY.shortSide}</h3>
      <div className="lab-sides">{entryLists("short_side.entry.all", COPY.shortEntryTitle, "Short entry", b.top.short)}{list("short_side.exit.any", COPY.shortExitTitle, "Short exit", b.top.short)}</div>
    </div>}
    </>}
    </>}
    {part === "settings" && <h2 className="box-h">Settings <small>test period, costs and money</small></h2>}
    {showSettings && <>
    {simple ? <SimpleSettings spec={spec} settings={settings} onChange={onChange} open={part === "settings"}>
      {period}
      {dates}
      <InitialEquity spec={spec} catalogue={catalogue} instrument={instrument} onChange={onChange} instrumentData={false} />
    </SimpleSettings> : <>{part !== "settings" && <h3 className="lab-settings-h">Backtest settings <small>how trades are filled, sized and charged</small></h3>}
    <SettingsTabs labels={["General", "Trades", "Orders & stops", "Report", "Portfolio"]}>
      <>
        <p className="cap lab-stab-intro">The basics: starting money, the test period and costs, long or short, and how big each trade is.</p>
        <SettingGroup title="Capital"><InitialEquity spec={spec} catalogue={catalogue} instrument={instrument} onChange={onChange} instrumentData={false} /></SettingGroup>
        <SettingGroup title="Test period & costs">{period}</SettingGroup>
        {dates && !tabbed && <SettingGroup title="Date filter">{dates}</SettingGroup>}
        <SettingGroup title="Trading">
          <ExecutionPanel spec={spec} catalogue={catalogue} onChange={onChange} part="direction" />
          {!spec.portfolio && <GeneralData spec={spec} instrument={instrument} onChange={onChange} />}
        </SettingGroup>
        <SettingGroup title="Position size & risk"><RiskPanel spec={spec} catalogue={catalogue} instrument={instrument} onChange={onChange} /></SettingGroup>
      </>
      <>
        <p className="cap lab-stab-intro">When orders fill and at what price: the bar, the price for each side and any delay.</p>
        {!ordered && <ScenarioPreset spec={spec} custom={single} onChange={onChange} />}
        {single && <TradePrices spec={spec} preset={!usesTradePrices(spec.execution)} onChange={onChange} />}
        <ExecutionPanel spec={spec} catalogue={catalogue} onChange={onChange} part="fills" />
        {stray && <p className="rnote" role="note" data-lab-portfolio-stray>
          Trade prices, re-entry delays, stop exit modes, same-bar and stop activation settings and position limits apply to one instrument only.{" "}
          <button type="button" className="dbtn sm" onClick={() => onChange(setExecution(spec, { prices: undefined, delays: undefined, reentry: undefined, limits: undefined,
            exit_at_stop: undefined, activate_stops_immediately: undefined, nbar_has_priority: undefined, allow_same_bar_exit: undefined, bars_held_from: undefined, reenter_on_exit_bar: undefined }))}>Remove them</button>
        </p>}
      </>
      <>
        <p className="cap lab-stab-intro">Entry orders (limit or stop) and the exits that close a trade automatically: stop loss, profit target, trailing, breakeven, N-bar.</p>
        <label className="lab-check" title={single ? undefined : "A portfolio checks stops from the entry bar (a single-instrument setting)"}><input type="checkbox" disabled={!single}
          checked={ex.activate_stops_immediately ?? true} onChange={(e) => onChange(setExecution(spec, { activate_stops_immediately: e.target.checked ? undefined : false }))} />Activate stops immediately</label>
        <p className="cap">{(ex.activate_stops_immediately ?? true) ? "Checked by default here (AmiBroker's default is unchecked): a stop can trigger on the entry bar when the entry fills at the open or as a limit/stop order; an entry at the close or another later price is checked from the next bar." : "Stops are first checked on the bar after the entry bar."}</p>
        {both && <h3 className="lab-side-h">{COPY.longSide}</h3>}
        <OrdersPanel spec={spec} catalogue={catalogue} pre="" reentry={single} onChange={onChange} />
        {both && spec.short_side && <><h3 className="lab-side-h">{COPY.shortSide}</h3><OrdersPanel spec={spec} catalogue={catalogue} pre="short_side." onChange={onChange} /></>}
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
    </SettingsTabs></>}
    </>}
    {readsSessionDates(spec) && <p className="cap">{COPY.sessionDatesHint}</p>}
    {usesHigherTimeframes(spec) && <p className="cap">{COPY.wmRule}</p>}
    <div className="lab-row lab-run">
      <button type="button" className="dbtn primary" data-lab-action="run" disabled={running || problems.length > 0} onClick={onRun}>{running ? "Running…" : simple ? "▶ Test this idea" : "Run backtest"}</button>
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
        <button type="button" className="dbtn primary" data-lab-action="run" disabled={running || problems.length > 0} onClick={onRun}>
          {running ? "Running…" : "Run backtest"}
        </button>
        <span className="cap">Signal at the bar's close · fill at the {fill}</span>
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
