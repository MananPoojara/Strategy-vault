import { dayFirst } from "../DateField";
import { WF_TARGETS, WfPreview, wfWindows } from "../WalkForward";
import type { Spec, WalkforwardOptions, WalkforwardTarget } from "../types";
import { useWfDraft, wfBad } from "./run-prefs";
import { Why } from "./common";

/** A date AmiBroker types in; here it follows from the test period and the lengths, so it is shown, not edited. */
const shown = (label: string, value: string | undefined) =>
  <label className="dfield">{label.replace(/^\S+ /, "")}<input type="text" aria-label={label} value={value ? dayFirst(value) : ""} placeholder="—" disabled /></label>;

/** AmiBroker Analysis settings › Walk-Forward: easy mode (EOD), the in-sample and out-of-sample windows, the target and the window list. */
export function WalkForwardTab({ spec, dates = [] }: { spec: Spec; /** Bars of the last backtest: the range the windows are previewed over. */ dates?: string[] }) {
  const [o, set] = useWfDraft(); // shared with the walk-forward under Optimize
  const bad = wfBad(o), preview = bad ? { windows: [] } : wfWindows(dates, o);
  const [first, last] = [preview.windows[0], preview.windows[preview.windows.length - 1]];
  const length = (k: "is" | "oos" | "step", label: string) => <label className="dfield">{label.replace(/^\S+ /, "")}
    <input type="number" min={1} step={1} aria-label={label} value={o[k]} onChange={(e) => set({ [k]: Number(e.target.value) } as Partial<WalkforwardOptions>)} /></label>;
  return <div className="lab-wf-tab" data-lab-wf-tab>
    {spec.portfolio && <p className="rnote" role="note">Walk-forward runs on one instrument; these options apply when the strategy trades one.</p>}
    <fieldset className="lab-fieldset" data-lab-wf-is>
      <legend>In-sample data</legend>
      <div className="lab-row">{shown("In-sample Start", first?.is[0])}{shown("In-sample End", first?.is[1])}{length("is", "In-sample Length")}
        <label className="lab-check"><input type="checkbox" aria-label="Anchored" checked={o.anchored} onChange={(e) => set({ anchored: e.target.checked })} /> Anchored</label></div>
      <div className="lab-row">{shown("In-sample Last", last?.is[1])}
        <label className="lab-check"><input type="checkbox" aria-label="In-sample Use today" checked disabled /> Use today</label><Why text="Windows run to the end of the test period (To in Your idea)" /></div>
      <div className="lab-row">{length("step", "In-sample Step")}
        <label className="dfield">Unit<select aria-label="Walk-forward unit" value={o.unit} onChange={(e) => set({ unit: e.target.value as WalkforwardOptions["unit"] })}>
          <option value="years">year(s)</option><option value="months">month(s)</option></select></label></div>
    </fieldset>
    <fieldset className="lab-fieldset" data-lab-wf-oos>
      <legend>Out-of-sample data</legend>
      <div className="lab-row">{shown("Out-of-sample Start", first?.oos[0])}{shown("Out-of-sample End", first?.oos[1])}{length("oos", "Out-of-sample Length")}
        <label className="lab-check"><input type="checkbox" aria-label="Out-of-sample Anchored" checked={false} disabled /> Anchored</label></div>
      <div className="lab-row">{shown("Out-of-sample Last", last?.oos[1])}
        <label className="lab-check"><input type="checkbox" aria-label="Out-of-sample Use today" checked disabled /> Use today</label></div>
      <div className="lab-row"><label className="dfield">Step<input type="number" aria-label="Out-of-sample Step" value={o.step} disabled /></label>
        <label className="dfield">Unit<select aria-label="Out-of-sample unit" value={o.unit} disabled><option value="years">year(s)</option><option value="months">month(s)</option></select></label></div>
      </fieldset>
    <div className="lab-row">
      <label className="dfield">Optimization target<select aria-label="Walk-forward target" value={o.target} onChange={(e) => set({ target: e.target.value as WalkforwardTarget })}>
        {WF_TARGETS.map(([v, l]) => <option key={v} value={v}>{l}</option>)}</select></label>
    </div>
    {bad ? <p className="rnote" data-lab-wf-bad>Lengths and step are whole numbers of 1 or more, and the step is at least the out-of-sample length; the last valid options are kept until this is fixed.</p>
      : <WfPreview dates={dates} preview={preview} />}
  </div>;
}
