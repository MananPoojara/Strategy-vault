import { NumField } from "./OperandEditor";
import { orderOf, PRICE_ORDERS, setDirection, setExecution, setOrder, type PriceOrder } from "./spec-model";
import type { Dist, Fill, Spec } from "./types";
import { Segmented } from "./ui";
import "./rules.css";

const DEFAULT_PCT: Record<PriceOrder["key"], number> = { limit: 1, stop_loss: 5, target: 10, trailing: 5 };
type Mode = "off" | "pct" | "atr";
const MODES: { value: Mode; label: string }[] = [{ value: "off", label: "Off" }, { value: "pct", label: "%" }, { value: "atr", label: "ATR" }];
const FILLS: { value: Fill; label: string }[] = [{ value: "next_open", label: "Next open" }, { value: "close", label: "Same close" }];

export function OrdersPanel({ spec, onChange }: { spec: Spec; onChange(spec: Spec): void }) {
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
  );
}
