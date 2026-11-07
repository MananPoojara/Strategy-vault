/** A short strategy's no-trade windows (the AFLs' hard-punched Master 5DP and QTR Midcap dates, hardpunch.ts): each Off,
 *  Fresh or Running. Shown for direction "short" only; they stack with the Dates and Indicator filters. */
import { REENTRY } from "./DateFilterPanel";
import type { RegimeSet } from "./master5dp";
import { setExecution } from "./spec-model";
import { Segmented } from "./ui";
import type { Blackout, Reentry, Spec } from "./types";

const OFF = "off";
const isQtr = (name: string) => name.startsWith("QTR ");
const isM5dp = (name: string) => name === "Master 5DP";
// How a Running window's exit-day short behaves (rules.rs fresh_reentry_exits, restart_virtual_on_signal); the AFL each copies.
const KEEP = "keep", FRESH = "fresh", RESTART = "restart";
const RERUN = [
  { value: KEEP, label: "Keep old stop", title: "The re-entered short keeps the original trade's stop and ends with it (StochD, RSI, UO AFLs)" },
  { value: FRESH, label: "Fresh stop", title: "The re-entered short gets fresh exits from its re-entry day (ST_Sell_Short ATR AFL)" },
  { value: RESTART, label: "Fresh stop + restart on signal", title: "Fresh exits, and every new short signal restarts the background trade (monthly ROC AFL)" },
];
const rerun = (spec: Spec) => spec.execution?.restart_virtual_on_signal ? RESTART : spec.execution?.fresh_reentry_exits ? FRESH : KEEP;

export function ShortWindowsPanel({ spec, onChange, sets }: { spec: Spec; onChange(spec: Spec): void; sets: readonly RegimeSet[] }) {
  if (spec.direction !== "short" || !sets.length) return null;
  const list = spec.blackouts ?? [], qtrs = sets.filter((s) => isQtr(s.name));
  const put = (group: (n: string) => boolean, b?: Blackout) => {
    const { blackouts: _, ...rest } = spec, next = [...list.filter((x) => !group(x.name)), ...(b ? [b] : [])];
    onChange(next.length ? { ...rest, blackouts: next } : rest);
  };
  const row = (label: string, group: (n: string) => boolean, first?: RegimeSet) => {
    const on = list.find((x) => group(x.name));
    return first && <div className="lab-param-row"><span className="lab-param-k">{label}</span><span className="lab-param-v">
      <Segmented<string> label={label} value={on?.reentry ?? OFF} options={[{ value: OFF, label: "Off", title: `No ${label} cut` }, ...REENTRY]}
        onChange={(r) => { const s = sets.find((x) => x.name === on?.name) ?? first; put(group, r === OFF ? undefined : { name: s.name, windows: s.windows, reentry: r as Reentry }); }} />
    </span></div>;
  };
  return <div className="lab-datefilter" data-lab-short-windows>
    {row("Master 5DP", isM5dp, sets.find((s) => isM5dp(s.name)))}
    {row("QTR", isQtr, qtrs.find((s) => s.name === "QTR Midcap") ?? qtrs[0])}
    {list.some((b) => b.reentry === "running") && <div className="lab-param-row"><span className="lab-param-k">Running re-entry</span><span className="lab-param-v">
      <Segmented<string> label="Running re-entry" value={rerun(spec)} options={RERUN}
        onChange={(v) => onChange(setExecution(spec, { fresh_reentry_exits: v === KEEP ? undefined : true, restart_virtual_on_signal: v === RESTART || undefined }))} />
    </span></div>}
    <p className="cap" data-lab-date-generated style={{ whiteSpace: "normal", overflowWrap: "anywhere", maxWidth: "100%" }}>
      No short inside a window: an open short is covered on its first day. Fresh waits for a new short signal; Running shorts again on the
      window's exit day if the trade would still be open. Hard-punched dates from the AFLs: Master 5DP to 05-10-2026, QTR (NIFTY MIDCAP 100) to 28-02-2025; later windows appear when their dates are added.</p>
  </div>;
}
