import { useEffect, useRef, useState } from "react";
import { NumField } from "./OperandEditor";
import { COPY } from "./labels";
import { directionOf, entryOrderType, withDirection } from "./execution-model";
import { setExecution } from "./spec-model";
import { Segmented } from "./ui";
import type { Catalogue, Fill, ShortSide, Spec } from "./types";
import { usesTradePrices } from "./AmiSettings";
import "./orders.css";

/** `part` splits the panel for the AmiBroker-style layout: "direction" (positions) above the rules, "fills" (Trades tab). */
export function ExecutionPanel({ spec, catalogue, onChange, part = "all" }: { spec: Spec; catalogue: Catalogue; onChange(spec: Spec): void; part?: "all" | "direction" | "fills" }) {
  const kept = useRef<ShortSide | undefined>(undefined);
  const ex = spec.execution ?? {};
  const [timing, setTiming] = useState(ex.delay_bars !== undefined || ex.entry_on !== undefined || ex.intrabar !== undefined);
  useEffect(() => { if (ex.delay_bars !== undefined || ex.entry_on !== undefined || ex.intrabar !== undefined) setTiming(true); }, [ex.delay_bars, ex.entry_on, ex.intrabar]);
  const direction = directionOf(spec), opposite = ex.opposite ?? "reverse", fill = ex.fill ?? "next_open", exitFill = ex.exit_fill ?? fill;
  const ordered = entryOrderType(spec.entry) !== "market" || (direction === "both" && !!spec.short_side && entryOrderType(spec.short_side.entry) !== "market");
  const delayed = fill === "next_open" && exitFill === "next_open", sameBar = ex.allow_same_bar_exit ?? true;
  const fills: { value: Fill; label: string }[] = [{ value: "next_open", label: "Next open" }, { value: "close", label: "Same close" }];
  const showDirection = part !== "fills", showFills = part !== "direction", ami = usesTradePrices(ex); // AmiBroker per-side prices replace fill/exit_fill/delay
  return <div className="lab-exec" data-lab-exec={part}>
    <div className="lab-row">
      {showDirection && <><span className="dfield">Positions</span>
      <Segmented label="Positions" value={direction} options={[{ value: "long", label: "Long" }, { value: "short", label: "Short" }, { value: "both", label: "Long and short", disabled: !catalogue.execution?.direction.includes("both") }]}
        onChange={(d) => { if (spec.short_side) kept.current = spec.short_side; onChange(withDirection(spec, d, kept.current)); }} />
      {direction === "both" && <Segmented label="When the other side signals" value={opposite}
        options={[{ value: "reverse", label: "Reverse" }, { value: "exit", label: "Exit only" }, { value: "ignore", label: "Ignore" }]}
        onChange={(o) => onChange(setExecution(spec, { opposite: o === "reverse" ? undefined : o }))} />}
      <label className="lab-check" title={spec.portfolio ? "A single-instrument setting" : undefined}><input type="checkbox" disabled={!!spec.portfolio} checked={sameBar}
        onChange={(e) => onChange(setExecution(spec, { allow_same_bar_exit: e.target.checked ? undefined : false }))} />Allow same bar exit / entry signal</label></>}
      {showFills && <>{!ordered && !ami && <label className="dfield">Entry fill<select aria-label="Entry fill" value={fill} onChange={(e) => {
        const f = e.target.value as Fill;
        onChange(setExecution(spec, { fill: f === "next_open" ? undefined : f, ...(f === "close" ? { delay_bars: undefined } : {}) }));
      }}>{fills.map((f) => <option key={f.value} value={f.value}>{f.label}</option>)}</select></label>}
      {!ami && <label className="dfield">Exit fill<select aria-label="Exit fill" value={exitFill} onChange={(e) => {
        const f = e.target.value as Fill;
        onChange(setExecution(spec, { exit_fill: f === fill ? undefined : f, ...(f === "close" ? { delay_bars: undefined } : {}) }));
      }}>{fills.map((f) => <option key={f.value} value={f.value}>{f.label}</option>)}</select></label>}
      <label className="lab-check" title={spec.portfolio ? "A single-instrument setting" : "AmiBroker AFL counts BarsSince(Buy) from the signal bar; with a next-open fill that exits one bar earlier. Close fills are not affected."}>
        <input type="checkbox" disabled={!!spec.portfolio} checked={ex.bars_held_from === "signal"}
          onChange={(e) => onChange(setExecution(spec, { bars_held_from: e.target.checked ? "signal" : undefined }))} />Count bars held from the signal bar</label>
      <button type="button" className="dbtn sm ghost" data-lab-timing-options aria-expanded={timing} onClick={() => setTiming(!timing)}>{timing ? "▾" : "▸"} Timing options</button></>}
    </div>
    {showDirection && direction === "both" && <p className="cap">{COPY.oppositeHelp[opposite]} {COPY.bothFlat}</p>}
    {showDirection && <p className="cap" data-lab-same-bar>{sameBar ? "Checked (default): an exit and a new entry may fill on one bar, except a close fill or a limit/stop order after a stop or close exit inside that bar." : "Unchecked: no entry fills on a bar where a position closed, and no position exits on its entry bar (a reversal stays one fill)."}</p>}
    {showFills && !ordered && !ami && ex.exit_fill === undefined && <p className="cap">Changing the entry fill also moves exits until an exit fill is chosen.</p>}
    {showFills && timing && <>
      <div className="lab-row">
        <fieldset className="lab-delay" hidden={ami} disabled={!delayed} title={delayed ? undefined : COPY.delayOnlyNextOpen}>
          <NumField label="Fill delay (bars)" value={ex.delay_bars ?? 1} min={1} max={catalogue.execution?.max_delay_bars ?? 10} step={1}
            onChange={(n) => n !== undefined && onChange(setExecution(spec, { delay_bars: n === 1 ? undefined : n }))} />
        </fieldset>
        <span className="lab-row"><span className="dfield">Enter on</span><Segmented label="Enter on" value={ex.entry_on ?? "level"} options={[{ value: "level", label: "Every bar the rules are true" }, { value: "edge", label: "Only the bar they turn true" }]}
          onChange={(v) => onChange(setExecution(spec, { entry_on: v === "level" ? undefined : v }))} /></span>
        <span className="lab-row"><span className="dfield">Same-bar order</span><Segmented label="Same-bar order" value={ex.intrabar ?? "stop_first"} options={[{ value: "stop_first", label: "Stop first" }, { value: "ohlc_path", label: "OHLC path" }]}
          onChange={(v) => onChange(setExecution(spec, { intrabar: v === "stop_first" ? undefined : v }))} /></span>
      </div>
      {!delayed && !ami && <p className="cap">{COPY.delayOnlyNextOpen}</p>}
      <p className="cap">{COPY.intrabarHelp[ex.intrabar ?? "stop_first"]}</p>
    </>}
  </div>;
}
