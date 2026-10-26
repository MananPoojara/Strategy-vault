import { COPY } from "./labels";
import { defaultLevel, entryOrderType, levelAt, levelForm, LEVELS, sideOf, takeProfits, TP_BOUNDS, withEntryOrder, withLevel, withTakeProfits,
  type EntryOrderType, type LevelForm, type LevelSlot, type SidePre } from "./execution-model";
import { getAt, putAt } from "./spec-model";
import "./orders.css";
import { useEffect, useState } from "react";
import { NumField, OperandEditor } from "./OperandEditor";
import { orderOf, PRICE_ORDERS, setDirection, setExecution, setOrder, type PriceOrder } from "./spec-model";
import type { Catalogue, Dist, EntryLevel, Fill, Spec } from "./types";
import { Segmented } from "./ui";
import { ExitAtStopSelect, NBarStop, ReentryDelay } from "./AmiSettings";
import "./rules.css";

const DEFAULT_PCT: Record<PriceOrder["key"], number> = { limit: 1, stop_loss: 5, target: 10, trailing: 5 };
type Mode = "off" | "pct" | "atr";
const MODES: { value: Mode; label: string }[] = [{ value: "off", label: "Off" }, { value: "pct", label: "%" }, { value: "atr", label: "ATR" }];
const FILLS: { value: Fill; label: string }[] = [{ value: "next_open", label: "Next open" }, { value: "close", label: "Same close" }];

function LegacyOrdersPanel({ spec, onChange }: { spec: Spec; onChange(spec: Spec): void }) {
  const fill = spec.execution?.fill ?? "next_open";
  // Without exit_fill the engine uses fill for exits too.
  const exitFill = spec.execution?.exit_fill ?? fill;
  return (
    <div className="lab-orders">
      <div className="lab-row">
        <span className="dfield">Direction</span>
        <Segmented label="Direction" value={spec.direction ?? "long"} onChange={(d) => onChange(setDirection(spec, d))}
          options={[{ value: "long", label: "Long" }, { value: "short", label: "Short" }]} />
        {!spec.entry.limit && (
          <label className="dfield">Entry fill
            <select value={fill} onChange={(e) => onChange(setExecution(spec, { fill: e.target.value === "next_open" ? undefined : (e.target.value as Fill) }))}>
              {FILLS.map((f) => <option key={f.value} value={f.value}>{f.label}</option>)}
            </select>
          </label>
        )}
        <label className="dfield">Exit fill
          <select value={exitFill} onChange={(e) => onChange(setExecution(spec, { exit_fill: e.target.value === fill ? undefined : (e.target.value as Fill) }))}>
            {FILLS.map((f) => <option key={f.value} value={f.value}>{f.label}</option>)}
          </select>
        </label>
      </div>
      {!spec.entry.limit && !spec.execution?.exit_fill && <p className="cap">Changing the entry fill also moves exits until an exit fill is chosen.</p>}
      <div className="lab-price-orders">
      {PRICE_ORDERS.map((order) => {
        const d = orderOf(spec, order.key);
        const mode: Mode = !d ? "off" : "pct" in d ? "pct" : "atr";
        const put = (dist: Dist | undefined) => onChange(setOrder(spec, order.key, dist));
        return (
          <div className="lab-order" key={order.key} role="group" aria-label={order.label}>
            <span className="dfield">{order.label}</span>
            <Segmented label={`${order.label} type`} value={mode} options={MODES} onChange={(m) => m !== mode &&
              put(m === "off" ? undefined : m === "pct" ? { pct: DEFAULT_PCT[order.key] } : { atr: 2, atr_length: 14 })} />
            {d && "pct" in d && (
              <NumField label="%" value={d.pct} min={order.pct[0]} max={order.pct[1]} step={0.5}
                onChange={(n) => n !== undefined && put({ ...d, pct: n })} />
            )}
            {d && "atr" in d && (
              <>
                <NumField label="ATR ×" value={d.atr} min={0} max={50} step={0.25} onChange={(n) => n !== undefined && put({ ...d, atr: n })} />
                <NumField label="ATR length" value={d.atr_length ?? 14} min={1} max={500} step={1}
                  onChange={(n) => n !== undefined && put({ ...d, atr_length: n })} />
              </>
            )}
          </div>
        );
      })}
      </div>
    </div>
  );
}


function LevelEditor({ value, slot, short, onChange, catalogue, allowOff, label, caption = label }: {
  value?: EntryLevel; slot: LevelSlot | "take_profit"; short: boolean; onChange(value: EntryLevel | undefined): void;
  catalogue?: Catalogue; allowOff?: boolean; label: string;
  /** Visible text before the unit select; a fixed stop group's legend already names it. */
  caption?: string;
}) {
  const bounds = slot === "take_profit" ? TP_BOUNDS : LEVELS[slot];
  const form = levelForm(value), at = bounds.at && !!catalogue;
  const unknown = value !== undefined && (!form || (form === "at" && !at));
  return <span className="lab-level" role="group" aria-label={label}>
    <label className="dfield">{caption}<select aria-label={`${label} unit`} value={unknown ? "kept" : form ?? (allowOff ? "off" : "pct")}
      onChange={(e) => {
        if (e.target.value === "off") { onChange(undefined); return; }
        const extra = { ...value } as Record<string, unknown>;
        for (const key of ["pct", "atr", "atr_length", "points", "at", "ticks", "profit_pct"]) delete extra[key];
        onChange({ ...extra, ...defaultLevel(slot, e.target.value as LevelForm, short) });
      }}>
      {allowOff && <option value="off">Disabled</option>}
      {unknown && <option value="kept" disabled>Kept as is</option>}
      <option value="pct">%</option><option value="atr">ATR</option><option value="points">Points</option>{bounds.profitPct && <option value="profit_pct">% of profit</option>}{at && <option value="at">At</option>}
    </select></label>
    {unknown ? <span className="lab-kept">{COPY.keptAsIs} <code>{JSON.stringify(value)}</code></span> : value && <>
      {"pct" in value && <NumField label="%" value={value.pct} min={0} max={bounds.maxPct} step={0.1} onChange={(n) => n !== undefined && onChange({ ...value, pct: n })} />}
      {"atr" in value && <><NumField label="ATR ×" value={value.atr} min={0} max={50} step={0.25} onChange={(n) => n !== undefined && onChange({ ...value, atr: n })} />
        <NumField label="ATR length" value={value.atr_length ?? 14} min={1} max={500} step={1} onChange={(n) => n !== undefined && onChange({ ...value, atr_length: n })} /></>}
      {"points" in value && <NumField label="Points" value={value.points} min={0} max={1e9} step={0.1} onChange={(n) => n !== undefined && onChange({ ...value, points: n })} />}
      {"profit_pct" in value && <NumField label="% of profit" value={value.profit_pct} min={1} max={99} step={1} onChange={(n) => n !== undefined && onChange({ ...value, profit_pct: n })} />}
      {"at" in value && catalogue && <OperandEditor operand={value.at} side="entry" heldOk={false} catalogue={catalogue} label={`${label} level`}
        onChange={(o) => onChange({ ...value, at: o })} />}
    </>}
  </span>;
}

function ScaleInEditor({ spec, pre, catalogue, onChange }: { spec: Spec; pre: SidePre; catalogue: Catalogue; onChange(spec: Spec): void }) {
  const side = sideOf(spec, pre)!;
  const si = side.entry.scale_in;
  const max = catalogue.execution?.max_legs ?? 5;
  const parse = (text: string): number[] | undefined => {
    const parts = text.trim().split(/[,\s]+/).filter(Boolean);
    const values = parts.map(Number);
    return values.length >= 2 && values.length <= max && values.every((n) => Number.isFinite(n) && n > 0 && n <= 100) &&
      values.reduce((n, v) => n + v, 0) <= 100 + 1e-7 ? values.map((n) => n / 100) : undefined;
  };
  const stored = Array.isArray(si?.legs) ? si.legs.map((n) => n * 100).join(", ") : "";
  const [text, setText] = useState(stored);
  useEffect(() => setText((old) => JSON.stringify(parse(old)) === JSON.stringify(si?.legs) ? old : stored), [stored]);
  const tps = takeProfits(spec, pre).length > 0;
  return <div className="lab-scale-in">
    <label className="lab-check"><input type="checkbox" aria-label="Scale in" checked={!!si} disabled={!si && tps}
      onChange={(e) => onChange(putAt(spec, `${pre}entry.scale_in`, e.target.checked ? { legs: [0.5, 0.5] } : undefined))} />Scale in</label>
    {(si || tps) && <span className="cap">{COPY.scaleInWithTp}</span>}
    {si && !Array.isArray(si.legs) && <span className="lab-kept">{COPY.keptAsIs} <code>{JSON.stringify(si)}</code></span>}
    {si && Array.isArray(si.legs) && <>
      <label className="dfield">Legs (% of full size)<input className="lab-scale-legs" aria-label="Scale-in legs (% of full size)" value={text} aria-invalid={!parse(text)}
        onChange={(e) => { const raw = e.target.value; setText(raw); const legs = parse(raw); if (legs) onChange(putAt(spec, `${pre}entry.scale_in`, { ...si, legs })); }}
        onBlur={() => { if (!parse(text)) setText(stored); }} /></label>
      <NumField label="Bars between legs" value={si.min_bars ?? 1} min={1} max={1000} step={1}
        onChange={(n) => n !== undefined && onChange(putAt(spec, `${pre}entry.scale_in.min_bars`, n === 1 ? undefined : n))} />
      <label className="dfield">Leg price<select aria-label="Leg price" value={si.price ?? "any"}
        onChange={(e) => onChange(putAt(spec, `${pre}entry.scale_in.price`, e.target.value === "any" ? undefined : e.target.value))}>
        <option value="any">Any</option><option value="better">Better</option><option value="worse">Worse</option>
      </select></label>
    </>}
  </div>;
}

/** Exits beyond AmiBroker's four fixed stops, added on demand. */
const EXITS = [
  { key: "breakeven", label: "Breakeven", initial: { after: { pct: 2 } } },
  { key: "take_profits", label: "Take-profits", initial: [{ pct: 5, size: 0.5 }] }, { key: "min_bars", label: "Minimum bars", initial: 1 },
  { key: "dual_atr", label: "Dual ATR exit", initial: { initial: { period: 5, mult: 2, price: "hl2" }, favour: { period: 5, mult: 4, price: "hl2" } } },
  { key: "dual_line", label: "Two-line exit (initial → favour)", initial: {
    initial: { s: "supertrend", p: { atr_length: 5, factor: 1, mode: 1 }, out: "supertrend" }, favour: { s: "supertrend", p: { atr_length: 5, factor: 4, mode: 1 }, out: "supertrend" } } },
] as const;
/** rules.rs DUAL_ATR_PRICES, in the AmiBroker AFL "ATR Avg Val" list's order. */
const DUAL_PRICES: [string, string][] = [["open", "Open"], ["high", "High"], ["low", "Low"], ["close", "Close"], ["oc2", "(Open+Close)/2"],
  ["hl2", "(High+Low)/2"], ["hlc3", "(H+L+C)/3"], ["ohlc4", "(O+H+L+C)/4"]];
type DualBand = { period: number; mult: number; price?: string };
type DualAtr = { initial: DualBand; favour: DualBand; tf?: "D" | "W" | "M"; period_end_only?: boolean };
/** exit.dual_atr: the initial and favour ATR bands, the bands' timeframe and the period-end test (W/M only). */
function DualAtrFields({ value, onChange }: { value: DualAtr; onChange(v: DualAtr): void }) {
  const band = (key: "initial" | "favour", label: string) => {
    const b = value[key];
    const put = (patch: Partial<DualBand>) => onChange({ ...value, [key]: { ...b, ...patch } });
    return <span className="lab-row" role="group" aria-label={`${label} ATR`}>
      <span className="dfield">{label}</span>
      <NumField label={`${label} ATR period`} value={b.period} min={1} max={500} step={1} onChange={(n) => n !== undefined && Number.isInteger(n) && n >= 1 && n <= 500 && put({ period: n })} />
      <NumField label={`${label} ATR ×`} value={b.mult} min={0} max={100} step={0.5} onChange={(n) => n !== undefined && n > 0 && n <= 100 && put({ mult: n })} />
      <label className="dfield">Price<select aria-label={`${label} price`} value={b.price ?? "hl2"} onChange={(e) => put({ price: e.target.value })}>
        {DUAL_PRICES.map(([v, t]) => <option key={v} value={v}>{t}</option>)}
      </select></label>
    </span>;
  };
  const tf = value.tf ?? "D";
  return <>
    {band("initial", "Initial")}
    {band("favour", "Favour")}
    <span className="lab-row">
      <Segmented label="Dual ATR timeframe" value={tf} options={[{ value: "D", label: "Daily" }, { value: "W", label: "Weekly" }, { value: "M", label: "Monthly" }]}
        onChange={(t) => onChange({ ...value, tf: t === "D" ? undefined : t, period_end_only: t === "D" ? undefined : value.period_end_only })} />
      {tf !== "D" && <label className="lab-check"><input type="checkbox" checked={!!value.period_end_only}
        onChange={(e) => onChange({ ...value, period_end_only: e.target.checked || undefined })} />Exit only on the {tf === "W" ? "week's" : "month's"} last bar</label>}
    </span>
    <span className="cap">{COPY.dualAtrHint}</span>
  </>;
}
/** AmiBroker Stops tab: the four fixed groups in its order and naming. */
const STOPS = [
  { key: "stop_loss", legend: "Maximum loss stop", label: "Stop-loss" }, { key: "target", legend: "Profit target", label: "Target" },
  { key: "trailing", legend: "Trailing stop", label: "Trailing stop" },
] as const;
/** One-line disclosures of how the AmiBroker stop options run in this engine. */
const STOP_NOTES = {
  modes: "Exit at trade price only (0): the stop is tested only on each bar's Sell/Cover price (its open or close) and exits there, on that bar. Exit next bar at trade price (2): High-Low triggers it and it exits on the next bar at the Sell/Cover price (the next bar's open when trading on the open). Stops ignore trade delays. Breakeven and take-profits always exit intraday.",
  nbar: "The N-bar stop has no stop price and exits at the regular trade price. Has priority: on the bar the N-bar stop exits, the other stops are not checked.",
  portfolio: "Portfolio runs exit stops intraday at the stop level; exit modes, N-bar priority and re-entry delays are single-instrument settings.",
};

export function OrdersPanel({ spec, onChange, pre = "", catalogue, reentry = false }: {
  spec: Spec; onChange(spec: Spec): void; pre?: SidePre; catalogue?: Catalogue;
  /** Show each stop's re-entry delay, exit mode and the N-bar priority (single instrument; one value shared by both sides). */
  reentry?: boolean;
}) {
  // The original builder has no catalogue prop until Phase 7 assembly wires the shared execution panel.
  if (!catalogue && !pre) return <LegacyOrdersPanel spec={spec} onChange={onChange} />;
  const side = sideOf(spec, pre);
  if (!side) return <p className="cap">Choose Both directions to edit the short side.</p>;
  const sideName = pre || spec.direction === "short" ? "Short" : "Long", both = spec.direction === "both";
  const reentryLabel = both ? "Re-entry delay, both sides (bars)" : "Re-entry delay (bars)";
  const short = sideName === "Short", order = entryOrderType(side.entry);
  const tps = takeProfits(spec, pre);
  const editableTps = Array.isArray(side.exit.take_profits) && tps.every((tp) => tp && typeof tp === "object" && typeof tp.size === "number");
  const level = (slot: LevelSlot, label: string, allowOff = false) => <LevelEditor value={levelAt(spec, pre, slot)} slot={slot} short={short}
    onChange={(d) => onChange(withLevel(spec, pre, slot, d))} catalogue={catalogue} label={label} allowOff={allowOff} />;
  return <div className="lab-orders" data-lab-orders={pre ? "short" : "long"}>
    <div className="lab-row">
      <span className="dfield">Entry order</span>
      <Segmented label={`${sideName} entry order`} value={order} options={[
        { value: "market", label: "Market" }, { value: "limit", label: "Limit" }, { value: "stop", label: "Stop" }, { value: "stop_limit", label: "Stop-limit" },
      ]} onChange={(t: EntryOrderType) => onChange(withEntryOrder(spec, pre, t))} />
      {side.entry.limit && level("entry.limit", "Limit")}
      {side.entry.stop && level("entry.stop", "Stop")}
      {order !== "market" && <NumField label="Order stays (bars)" value={side.entry.valid_bars ?? 1} min={1} max={catalogue?.execution?.max_valid_bars ?? 20} step={1}
        onChange={(n) => n !== undefined && onChange(putAt(spec, `${pre}entry.valid_bars`, n === 1 ? undefined : n))} />}
    </div>
    {order === "stop_limit" && levelForm(side.entry.limit) !== "at" && <p className="cap">{COPY.stopLimitHint}</p>}
    {catalogue?.execution && <ScaleInEditor spec={spec} pre={pre} catalogue={catalogue} onChange={onChange} />}
    <div className="lab-stops">
      {STOPS.map(({ key, legend, label }) => <fieldset className="lab-fieldset" data-lab-exit={key} key={key}>
        <legend>{legend}</legend>
        <div className="lab-row">
          <LevelEditor value={levelAt(spec, pre, `exit.${key}`)} slot={`exit.${key}`} short={short} catalogue={catalogue} label={label} caption="" allowOff
            onChange={(d) => onChange(withLevel(spec, pre, `exit.${key}`, d))} />
          {key === "trailing" && side.exit.trailing !== undefined && level("exit.trailing.activate", "Trailing activation", true)}
          {reentry && <ExitAtStopSelect spec={spec} stop={key} label={label} onChange={onChange} />}
          {reentry && <ReentryDelay spec={spec} stop={key} label={reentryLabel} onChange={onChange} />}
        </div>
      </fieldset>)}
      <fieldset className="lab-fieldset" data-lab-exit="n_bar">
        <legend>N-bar stop</legend>
        <div className="lab-row">
          <NBarStop spec={spec} pre={pre} label="Bars held" onChange={onChange} />
          {reentry && <ReentryDelay spec={spec} stop="bars" label={reentryLabel} onChange={onChange} />}
          {reentry && <label className="lab-check"><input type="checkbox" checked={!!spec.execution?.nbar_has_priority}
            onChange={(e) => onChange(setExecution(spec, { nbar_has_priority: e.target.checked || undefined }))} />Has priority</label>}
        </div>
        {reentry && <p className="cap">{STOP_NOTES.nbar}</p>}
      </fieldset>
    </div>
    {reentry && <p className="cap" data-lab-stop-modes>{both ? "Exit at stop? applies to both sides. " : ""}{STOP_NOTES.modes}</p>}
    {!reentry && !pre && <p className="cap">{STOP_NOTES.portfolio}</p>}
    <div className="lab-exits">
      {EXITS.filter(({ key }) => side.exit[key] !== undefined).map(({ key, label }) => {
        const controls = <>
          {key === "breakeven" && <>{level("exit.breakeven.after", "Breakeven after")}{level("exit.breakeven.plus", "Breakeven plus", true)}</>}
          {key === "min_bars" && <><NumField label="Rule exits wait (bars)" value={side.exit.min_bars} min={0} max={1000} step={1}
            onChange={(n) => n !== undefined && onChange(putAt(spec, `${pre}exit.min_bars`, n))} /><span className="cap">{COPY.minBarsHint}</span></>}
          {key === "dual_atr" && <DualAtrFields value={side.exit.dual_atr as DualAtr} onChange={(v) => onChange(putAt(spec, `${pre}exit.dual_atr`, v))} />}
          {key === "dual_line" && side.exit.dual_line && (catalogue ? <>
            {(["initial", "favour"] as const).map((k) => <span className="lab-row" role="group" aria-label={`${k === "initial" ? "Initial" : "Favour"} line`} key={k}>
              <span className="dfield">{k === "initial" ? "Initial line" : "Favour line"}</span>
              <OperandEditor operand={side.exit.dual_line![k]} side="exit" heldOk={false} catalogue={catalogue} label={`${k === "initial" ? "Initial" : "Favour"} line`}
                onChange={(o) => onChange(putAt(spec, `${pre}exit.dual_line.${k}`, o))} />
            </span>)}
            <span className="cap">{COPY.dualLineHint}</span>
          </> : <span className="lab-kept">{COPY.keptAsIs} <code>{JSON.stringify(side.exit.dual_line)}</code></span>)}
          {key === "take_profits" && (editableTps ? <>
            {tps.map((tp, i) => <div className="lab-tp-row" key={i}>
              <NumField label={`Take-profit ${i + 1} size (%)`} value={tp.size * 100} min={0} max={100} step={1}
                onChange={(n) => n !== undefined && onChange(withTakeProfits(spec, pre, tps.map((p, j) => j === i ? { ...p, size: n / 100 } : p)))} />
              <span>at</span><LevelEditor value={tp} slot="take_profit" short={short} catalogue={catalogue} label={`Take-profit ${i + 1}`}
                onChange={(d) => d && !("at" in d) && onChange(withTakeProfits(spec, pre, tps.map((p, j) => j === i ? { ...d, size: p.size } : p)))} />
              <button type="button" className="dbtn sm ghost" aria-label={`Remove take-profit ${i + 1}`}
                onClick={() => onChange(withTakeProfits(spec, pre, tps.filter((_, j) => j !== i)))}>×</button>
            </div>)}
            {tps.length < (catalogue?.execution?.max_take_profits ?? 3) && <button type="button" className="dbtn sm" aria-label="Add take-profit"
              onClick={() => onChange(withTakeProfits(spec, pre, [...tps, { pct: 5, size: 0.25 }]))}>+ Take-profit</button>}
          </> : <span className="lab-kept">{COPY.keptAsIs} <code>{JSON.stringify(side.exit.take_profits)}</code></span>)}
        </>;
        return <div className="lab-order" data-lab-exit={key} role="group" aria-label={label} key={key}>
          <details className="lab-extra-exit" open><summary>{label}</summary><div className="lab-row">{controls}</div></details>
          <button type="button" className="dbtn sm ghost" aria-label={`Remove ${label}`} onClick={() => onChange(putAt(spec, `${pre}exit.${key}`, undefined))}>×</button>
        </div>;
      })}
      <select aria-label={`Add exit (${sideName.toLowerCase()})`} value="" onChange={(e) => {
        const item = EXITS.find((x) => x.key === e.target.value);
        if (item) onChange(putAt(spec, `${pre}exit.${item.key}`, structuredClone(item.initial)));
      }}>
        <option value="">+ More exits…</option>
        {EXITS.filter(({ key }) => side.exit[key] === undefined).map(({ key, label }) => <option key={key} value={key}
          disabled={(key === "take_profits" && !!side.entry.scale_in) || ((key === "dual_atr" || key === "dual_line") && !!spec.portfolio)}>{label}</option>)}
      </select>
    </div>
  </div>;
}
