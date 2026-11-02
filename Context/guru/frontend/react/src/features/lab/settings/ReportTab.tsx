import { ReportSettings } from "../ReportSettings";
import type { InstrumentContext } from "../execution-model";
import type { Spec } from "../types";

/** AmiBroker report options the lab's report always applies: shown as facts, not as greyed controls that look broken. */
const FIXED = ["Distribution charts spacing: 5 %", "MAE/MFE distribution charts", "Trade list in the report", "Formula in the report", "Buy-and-hold statistics of the tested instrument"];

/** AmiBroker Analysis settings › Report, in its order: the rates are execution.report; the rest shows what the lab's report always does (options it cannot do are left out). */
export function ReportTab({ spec, onChange }: { spec: Spec; instrument?: InstrumentContext; onChange(spec: Spec): void }) {
  const single = !spec.portfolio;
  // the lab report (Report.tsx) always has these pages, for single-instrument runs
  const always = single ? "Always in the lab report" : "The report applies to single-instrument runs only";
  return <>
    <p className="cap lab-stab-intro">What the report shows; the risk-free rates feed the Sharpe ratio of trades and the Ulcer performance index (UPI).</p>
    <fieldset className="lab-fieldset" data-lab-report-tab>
      <legend>Reporting</legend>
      <ReportSettings spec={spec} onChange={onChange} />
      <div data-lab-report-fixed>
        <p className="lab-fixed-h">{always}</p>
        <ul className="lab-fixed-list" aria-label={always}>
          {FIXED.map((f) => <li key={f} data-on={single || undefined}><span aria-hidden="true">{single ? "✓" : "–"}</span> {f}</li>)}
        </ul>
      </div>
    </fieldset>
  </>;
}
