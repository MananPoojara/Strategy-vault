import { NumField } from "./OperandEditor";
import { setExecution } from "./spec-model";
import type { Spec } from "./types";

type Rates = NonNullable<NonNullable<Spec["execution"]>["report"]>;

/** AmiBroker Settings › Report: the risk-free rates of the Sharpe ratio of trades and the Ulcer performance index (execution.report). */
export function ReportSettings({ spec, onChange }: { spec: Spec; onChange(spec: Spec): void }) {
  const rates = spec.execution?.report ?? {}, single = !spec.portfolio;
  const set = (key: keyof Rates, n: number | undefined) => {
    const next: Rates = { ...rates, [key]: n };
    if (n === undefined) delete next[key];
    onChange(setExecution(spec, { report: Object.keys(next).length ? next : undefined }));
  };
  return <div className="lab-row lab-settings" data-lab-report-settings>
    <NumField label="Risk-free rate for Sharpe ratio (%)" value={rates.rf_sharpe} min={0} max={100} placeholder="5" disabled={!single} onChange={(n) => set("rf_sharpe", n)} />
    <NumField label="Risk-free rate for UPI (%)" value={rates.rf_upi} min={0} max={100} placeholder="5.4" disabled={!single} onChange={(n) => set("rf_upi", n)} />
    <span className="cap">{single ? "Settings › Report: empty uses AmiBroker's defaults (5 % and 5.4 %)." : "The report and these rates apply to single-instrument runs only."}</span>
    {!single && spec.execution?.report && <button type="button" className="dbtn sm" onClick={() => onChange(setExecution(spec, { report: undefined }))}>Remove them</button>}
  </div>;
}
