import { NumField } from "./OperandEditor";
import { MAX_TRADE_DELAY, SIDE_LABEL, toTradePrices, tradeKeys, tradePriceProblems, withTradeKeys } from "./execution-model";
import { getAt, setAt, setExecution } from "./spec-model";
import type { SidePre } from "./execution-model";
import type { Execution, ExitAtStop, RuleNode, Spec, StopKey, TradePrice, TradeSide } from "./types";

// The model lives in execution-model.ts (the Run gate uses it); re-exported for the settings UI and its tests.
export { toTradePrices, tradePriceProblems, usesTradePrices } from "./execution-model";
/** AmiBroker Analysis settings › Trades: price field and delay for each of buy, sell, short and cover (engine `execution.prices` / `delays`). */
export const PRICE_FIELDS: [TradePrice, string][] = [["open", "Open"], ["high", "High"], ["low", "Low"], ["close", "Close"], ["average", "Average (O+H+L+C)/4"]];

/** `preset`: the grid stays visible but greyed, showing what the scenario's fills mean as prices and delays. */
export function TradePrices({ spec, onChange, preset = false }: { spec: Spec; onChange(spec: Spec): void; preset?: boolean }) {
  const ex = spec.execution ?? {}, keys = tradeKeys(spec), shown = preset ? toTradePrices(spec).execution ?? {} : ex;
  // only the sides the engine takes: this direction's, without an entry placed as a limit or stop order
  const groups = ([["Long trades", ["buy", "sell"]], ["Short trades", ["short", "cover"]]] as [string, TradeSide[]][])
    .map(([title, sides]) => [title, sides.filter((k) => keys.includes(k))] as [string, TradeSide[]]).filter(([, sides]) => sides.length);
  const set = (k: TradeSide, patch: { price?: TradePrice; delay?: number }) => onChange(withTradeKeys(setExecution(spec, {
    prices: { ...ex.prices, ...(patch.price ? { [k]: patch.price } : {}) },
    delays: { ...ex.delays, ...(patch.delay !== undefined ? { [k]: patch.delay } : {}) },
  })));
  return <div className="lab-trade-prices" data-lab-trade-prices={preset ? "preset" : "custom"}>
    {groups.map(([title, sides]) => <fieldset key={title} disabled={preset}>
      <legend>{title}</legend>
      {sides.map((k) => <div className="lab-row" key={k}>
        <label className="dfield">{SIDE_LABEL[k]} price<select aria-label={`${SIDE_LABEL[k]} price`} value={shown.prices?.[k] ?? "open"} onChange={(e) => set(k, { price: e.target.value as TradePrice })}>
          {PRICE_FIELDS.map(([v, label]) => <option key={v} value={v}>{label}</option>)}
        </select></label>
        <NumField label={`${SIDE_LABEL[k]} delay`} value={shown.delays?.[k] ?? 1} min={0} max={MAX_TRADE_DELAY} step={1} onChange={(n) => n !== undefined && set(k, { delay: n })} />
      </div>)}
    </fieldset>)}
    <p className="cap">{preset ? "Set by the scenario; choose Custom to edit." : "Delay = bars after the signal bar; delay 0 fills on the signal bar and needs Close."} Stops, targets and limit orders keep their own fills.</p>
    {!preset && tradePriceProblems(spec).map((p) => <p className="rnote" role="note" key={p}>{p}</p>)}
  </div>;
}

type ReentryKey = keyof NonNullable<Execution["reentry"]>;
/** AmiBroker Stops › Re-entry delay: bars to wait after a stop of that kind before a new entry on the same side (one value for both sides). */
export function ReentryDelay({ spec, stop, label = "Re-entry delay (bars)", onChange }: { spec: Spec; stop: ReentryKey; label?: string; onChange(spec: Spec): void }) {
  const re = spec.execution?.reentry ?? {};
  const set = (n: number | undefined) => {
    const clean = Object.fromEntries(Object.entries({ ...re, [stop]: n || undefined }).filter(([, v]) => v !== undefined));
    onChange(setExecution(spec, { reentry: Object.keys(clean).length ? clean : undefined }));
  };
  return <NumField label={label} value={re[stop] ?? 0} min={0} max={1000} step={1} onChange={set} />;
}

/** AmiBroker Stops › N-bar stop: a top-level `bars_held >= n` exit rule (the form the engine's N-bar re-entry delay recognises). */
const isNBar = (n: RuleNode) => {
  const c = n as { lhs?: { s?: string }; op?: string; rhs?: { value?: unknown }; off?: unknown };
  return c.lhs?.s === "bars_held" && c.op === ">=" && typeof c.rhs?.value === "number" && c.off !== true;
};
export function nBarStop(spec: Spec, pre: SidePre): number | undefined {
  const node = ((getAt(spec, `${pre}exit.any`) as RuleNode[] | undefined) ?? []).find(isNBar) as { rhs: { value: number } } | undefined;
  return node?.rhs.value;
}
export function withNBarStop(spec: Spec, pre: SidePre, n: number | undefined): Spec {
  const list = (getAt(spec, `${pre}exit.any`) as RuleNode[] | undefined) ?? [], at = list.findIndex(isNBar);
  const node = { lhs: { s: "bars_held" }, op: ">=", rhs: { value: n } } as RuleNode;
  const next = !n ? list.filter((_, i) => i !== at) : at < 0 ? [...list, node] : list.map((x, i) => (i === at ? node : x));
  return setAt(spec, `${pre}exit.any`, next);
}
export function NBarStop({ spec, pre, label, onChange }: { spec: Spec; pre: SidePre; label: string; onChange(spec: Spec): void }) {
  return <NumField label={label} value={nBarStop(spec, pre)} min={1} max={1000} step={1} placeholder="off" onChange={(n) => onChange(withNBarStop(spec, pre, n))} />;
}

/** AmiBroker Stops › "Exit at stop?" (ApplyStop ExitAtStop), one value per stop shared by both sides; 1 is the engine default and is not stored. */
export const EXIT_AT_STOP: [ExitAtStop, string][] = [[0, "Exit at trade price only (0)"], [1, "Exit intraday at stop (1)"], [2, "Exit next bar at trade price (2)"]];
export function withExitAtStop(spec: Spec, stop: StopKey, mode: ExitAtStop): Spec {
  const clean = Object.fromEntries(Object.entries({ ...spec.execution?.exit_at_stop, [stop]: mode === 1 ? undefined : mode }).filter(([, v]) => v !== undefined));
  return setExecution(spec, { exit_at_stop: Object.keys(clean).length ? clean : undefined });
}
export function ExitAtStopSelect({ spec, stop, label, onChange }: { spec: Spec; stop: StopKey; label: string; onChange(spec: Spec): void }) {
  return <label className="dfield"><select aria-label={`${label} exit mode`} value={spec.execution?.exit_at_stop?.[stop] ?? 1}
    onChange={(e) => onChange(withExitAtStop(spec, stop, Number(e.target.value) as ExitAtStop))}>
    {EXIT_AT_STOP.map(([v, text]) => <option key={v} value={v}>{text}</option>)}
  </select></label>;
}
