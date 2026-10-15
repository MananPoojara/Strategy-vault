import { useEffect, useRef, useState } from "react";
import { NumField } from "./OperandEditor";
import { COPY } from "./labels";
import { directionOf, entryOrderType, withDirection } from "./execution-model";
import { setExecution } from "./spec-model";
import { Segmented } from "./ui";
import type { Catalogue, Fill, ShortSide, Spec } from "./types";
import "./orders.css";

export function ExecutionPanel({ spec, catalogue, onChange }: { spec: Spec; catalogue: Catalogue; onChange(spec: Spec): void }) {
  const kept = useRef<ShortSide | undefined>(undefined);
  const ex = spec.execution ?? {};
  const [timing, setTiming] = useState(ex.delay_bars !== undefined || ex.entry_on !== undefined || ex.intrabar !== undefined);
  useEffect(() => { if (ex.delay_bars !== undefined || ex.entry_on !== undefined || ex.intrabar !== undefined) setTiming(true); }, [ex.delay_bars, ex.entry_on, ex.intrabar]);
  const direction = directionOf(spec), opposite = ex.opposite ?? "reverse", fill = ex.fill ?? "next_open", exitFill = ex.exit_fill ?? fill;
  const ordered = entryOrderType(spec.entry) !== "market" || (direction === "both" && !!spec.short_side && entryOrderType(spec.short_side.entry) !== "market");
  const delayed = fill === "next_open" && exitFill === "next_open";
  const fills: { value: Fill; label: string }[] = [{ value: "next_open", label: "Next open" }, { value: "close", label: "Same close" }];
  return <div className="lab-exec" data-lab-exec>
    <div className="lab-row">
      <span className="dfield">Direction</span>
      <Segmented label="Direction" value={direction} options={[{ value: "long", label: "Long" }, { value: "short", label: "Short" }, { value: "both", label: "Both", disabled: !catalogue.execution?.direction.includes("both") }]}
        onChange={(d) => { if (spec.short_side) kept.current = spec.short_side; onChange(withDirection(spec, d, kept.current)); }} />
      {direction === "both" && <Segmented label="When the other side signals" value={opposite}
        options={[{ value: "reverse", label: "Reverse" }, { value: "exit", label: "Exit only" }, { value: "ignore", label: "Ignore" }]}
        onChange={(o) => onChange(setExecution(spec, { opposite: o === "reverse" ? undefined : o }))} />}
      {!ordered && <label className="dfield">Entry fill<select aria-label="Entry fill" value={fill} onChange={(e) => {
        const f = e.target.value as Fill;
        onChange(setExecution(spec, { fill: f === "next_open" ? undefined : f, ...(f === "close" ? { delay_bars: undefined } : {}) }));
      }}>{fills.map((f) => <option key={f.value} value={f.value}>{f.label}</option>)}</select></label>}
      <label className="dfield">Exit fill<select aria-label="Exit fill" value={exitFill} onChange={(e) => {
        const f = e.target.value as Fill;
        onChange(setExecution(spec, { exit_fill: f === fill ? undefined : f, ...(f === "close" ? { delay_bars: undefined } : {}) }));
      }}>{fills.map((f) => <option key={f.value} value={f.value}>{f.label}</option>)}</select></label>
      <button type="button" className="dbtn sm ghost" data-lab-timing-options aria-expanded={timing} onClick={() => setTiming(!timing)}>Timing options</button>
    </div>
    {direction === "both" && <p className="cap">{COPY.oppositeHelp[opposite]} {COPY.bothFlat}</p>}
    {!ordered && ex.exit_fill === undefined && <p className="cap">Changing the entry fill also moves exits until an exit fill is chosen.</p>}
    {timing && <>
      <div className="lab-row">
        <fieldset className="lab-delay" disabled={!delayed} title={delayed ? undefined : COPY.delayOnlyNextOpen}>
          <NumField label="Fill delay (bars)" value={ex.delay_bars ?? 1} min={1} max={catalogue.execution?.max_delay_bars ?? 10} step={1}
            onChange={(n) => n !== undefined && onChange(setExecution(spec, { delay_bars: n === 1 ? undefined : n }))} />
        </fieldset>
        <Segmented label="Enter on" value={ex.entry_on ?? "level"} options={[{ value: "level", label: "Every bar the rules are true" }, { value: "edge", label: "Only the bar they turn true" }]}
          onChange={(v) => onChange(setExecution(spec, { entry_on: v === "level" ? undefined : v }))} />
        <Segmented label="Same-bar order" value={ex.intrabar ?? "stop_first"} options={[{ value: "stop_first", label: "Stop first" }, { value: "ohlc_path", label: "OHLC path" }]}
          onChange={(v) => onChange(setExecution(spec, { intrabar: v === "stop_first" ? undefined : v }))} />
      </div>
      {!delayed && <p className="cap">{COPY.delayOnlyNextOpen}</p>}
      <p className="cap">{COPY.intrabarHelp[ex.intrabar ?? "stop_first"]}</p>
    </>}
  </div>;
}
