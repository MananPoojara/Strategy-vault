import { PRICE_FIELDS } from "../AmiSettings";
import { ExecutionPanel } from "../ExecutionPanel";
import { NumField } from "../OperandEditor";
import { directionOf, entryOrderType, MAX_TRADE_DELAY, SIDE_LABEL, toTradePrices, tradeKeys, tradePriceProblems, usesTradePrices, withTradeKeys } from "../execution-model";
import { setExecution } from "../spec-model";
import type { Catalogue, ExitAtStop, Spec, StopKey, TradePrice, TradeSide } from "../types";
import { ONE_INSTRUMENT, Why } from "./common";

/** Either side enters by a limit or stop order, which fills at its own level. */
export const ordered = (spec: Spec) => entryOrderType(spec.entry) !== "market" || (!!spec.short_side && entryOrderType(spec.short_side.entry) !== "market");

/** AmiBroker "Scenario" presets for market orders; `stops` sets Exit at stop? of all four stops (absent: intraday, the default). */
const SCENARIOS: { id: string; label: string; fill?: "next_open" | "close"; stops?: ExitAtStop }[] = [
  { id: "next_open", label: "Trade next bar on open, stops exit intraday", fill: "next_open" },
  { id: "close", label: "Trade current bar on close, stops exit intraday (default)", fill: "close" },
  { id: "next_open_stops_open", label: "Trade next bar on open, stops exit on next bar open", fill: "next_open", stops: 2 },
  { id: "close_stops_close", label: "Trade current bar on close, stops exit on close", fill: "close", stops: 0 },
  { id: "prices", label: "Custom: price and delay for buy, sell, short and cover" },
];
const STOP_KEYS: StopKey[] = ["stop_loss", "target", "trailing", "bars"];
/** The Scenario box at the top of AmiBroker's Trades and Stops tabs (one setting, shown on both). */
export function ScenarioPreset({ spec, onChange }: { spec: Spec; onChange(spec: Spec): void }) {
  const ex = spec.execution ?? {}, custom = !spec.portfolio, locked = ordered(spec);
  // Same-bar order (intrabar) is separate from the fill scenario and is kept
  const fill = ex.fill ?? "next_open", plain = (ex.exit_fill ?? fill) === fill && ex.delay_bars === undefined;
  const modes = new Set(STOP_KEYS.map((k) => ex.exit_at_stop?.[k] ?? 1)), stops = modes.size > 1 ? undefined : ({ 0: "_stops_close", 1: "", 2: "_stops_open" } as const)[[...modes][0]];
  const id = plain && stops !== undefined ? fill + stops : "custom";
  const current = usesTradePrices(ex) ? "prices" : SCENARIOS.some((x) => x.id === id) ? id : "custom";
  return <div className="lab-row" data-lab-scenario><label className="dfield lab-scenario">Scenario
    <select aria-label="Scenario" value={current} disabled={locked} onChange={(e) => {
      const pick = SCENARIOS.find((x) => x.id === e.target.value);
      if (pick?.id === "prices") onChange(toTradePrices(spec));
      else if (pick?.fill) onChange(setExecution(spec, { fill: pick.fill === "next_open" ? undefined : pick.fill, exit_fill: undefined, delay_bars: undefined, prices: undefined, delays: undefined,
        exit_at_stop: pick.stops === undefined ? undefined : Object.fromEntries(STOP_KEYS.map((k) => [k, pick.stops])) }));
    }}>
      {current === "custom" && <option value="custom">Custom (see the fields below)</option>}
      {/* stop exit modes are single-instrument settings, like per-side prices */}
      {SCENARIOS.map((x) => <option key={x.id} value={x.id} disabled={!custom && (x.id === "prices" || x.stops !== undefined)}>{x.label}</option>)}
    </select>
  </label><Why text={locked ? "The entry is a limit or stop order, which fills at its own level." : !custom && "Portfolio runs offer only the open and close scenarios with intraday stops."} /></div>;
}

const GROUPS: [string, TradeSide[]][] = [["Long trades", ["buy", "sell"]], ["Short trades", ["short", "cover"]]];
/** AmiBroker Analysis settings › Trades: Scenario, then price and delay for buy, sell, short and cover; sides that do not apply stay visible, greyed, with the reason. */
export function TradesTab({ spec, catalogue, onChange }: { spec: Spec; catalogue: Catalogue; onChange(spec: Spec): void }) {
  const ex = spec.execution ?? {}, single = !spec.portfolio, custom = usesTradePrices(ex), keys = tradeKeys(spec), dir = directionOf(spec);
  // a scenario shows its fills as the equivalent prices and delays
  const shown = custom ? ex : toTradePrices(spec).execution ?? {};
  // a long-only scenario has no short keys: short and cover show the scenario's entry and exit, as AmiBroker shows one scenario for both sides
  const twin = (k: TradeSide): TradeSide => (custom ? k : k === "short" ? "buy" : k === "cover" ? "sell" : k);
  const set = (k: TradeSide, patch: { price?: TradePrice; delay?: number }) => onChange(withTradeKeys(setExecution(spec, {
    prices: { ...ex.prices, ...(patch.price ? { [k]: patch.price } : {}) },
    delays: { ...ex.delays, ...(patch.delay !== undefined ? { [k]: patch.delay } : {}) },
  })));
  const groupWhy = (title: string) => !single ? ONE_INSTRUMENT
    : title === "Short trades" && dir === "long" ? "This strategy takes long trades only."
    : title === "Long trades" && dir === "short" ? "This strategy takes short trades only."
    : !custom ? "Set by the scenario; choose Custom to edit." : undefined;
  const stray = !single && [ex.prices, ex.delays, ex.reentry, ex.limits, ex.exit_at_stop, ex.activate_stops_immediately, ex.nbar_has_priority, ex.allow_same_bar_exit, ex.bars_held_from, ex.reenter_on_exit_bar].some((x) => x !== undefined);
  return <div className="lab-trades-tab" data-lab-trades-tab>
    <p className="cap lab-stab-intro">When orders fill and at what price: a scenario, or Custom for each side's price and delay.</p>
    <ScenarioPreset spec={spec} onChange={onChange} />
    <div className="lab-trade-prices" data-lab-trade-prices={custom ? "custom" : "preset"}>
      {GROUPS.map(([title, sides]) => {
        const why = groupWhy(title);
        return <fieldset key={title} disabled={!!why} data-lab-trade-group={title}>
          <legend>{title}</legend>
          {sides.map((k) => {
            // a limit or stop entry fills at its order level, not at a trade price
            const rowWhy = !why && !keys.includes(k) ? "Enters by a limit or stop order, which fills at its own level." : undefined;
            return <div className="lab-row" key={k}>
              <label className="dfield">{SIDE_LABEL[k]} price<select aria-label={`${SIDE_LABEL[k]} price`} disabled={!!rowWhy} value={shown.prices?.[k] ?? shown.prices?.[twin(k)] ?? "open"} onChange={(e) => set(k, { price: e.target.value as TradePrice })}>
                {PRICE_FIELDS.map(([v, label]) => <option key={v} value={v}>{label}</option>)}
              </select></label>
              <NumField label={`${SIDE_LABEL[k]} delay`} disabled={!!rowWhy} value={shown.delays?.[k] ?? shown.delays?.[twin(k)] ?? 1} min={0} max={MAX_TRADE_DELAY} step={1} onChange={(n) => n !== undefined && set(k, { delay: n })} />
              <Why text={rowWhy} />
            </div>;
          })}
          <Why text={why} />
        </fieldset>;
      })}
      <p className="cap">Delay = bars after the signal bar; delay 0 fills on the signal bar and needs Close. Stops, targets and limit orders keep their own fills.</p>
      {custom && tradePriceProblems(spec).map((p) => <p className="rnote" role="note" key={p}>{p}</p>)}
    </div>
    {/* the lab's own fill options (no AmiBroker equivalent), after AmiBroker's fields */}
    <ExecutionPanel spec={spec} catalogue={catalogue} onChange={onChange} part="fills" />
    {stray && <p className="rnote" role="note" data-lab-portfolio-stray>
      Trade prices, re-entry delays, stop exit modes, same-bar and stop activation settings and position limits apply to one instrument only.{" "}
      <button type="button" className="dbtn sm" onClick={() => onChange(setExecution(spec, { prices: undefined, delays: undefined, reentry: undefined, limits: undefined,
        exit_at_stop: undefined, activate_stops_immediately: undefined, nbar_has_priority: undefined, allow_same_bar_exit: undefined, bars_held_from: undefined, reenter_on_exit_bar: undefined }))}>Remove them</button>
    </p>}
  </div>;
}
