import { createContext, useContext, useState, type ReactNode } from "react";
import { directionOf, entryOrderType, modeOf, usesTradePrices, withDirection } from "./execution-model";
import { setExecution } from "./spec-model";
import { drawdown } from "./RunResults";
import { int, pct } from "./format";
import { Segmented } from "./ui";
import type { RunResult, RuleNode, Spec } from "./types";
import type { RunSettings } from "./RuleBuilder";
import "./simple.css";

/** Simple view (the default) hides the AmiBroker power-user controls; Advanced shows everything. Kept per browser. */
export type LabMode = "simple" | "advanced";
// Components rendered outside the page (tests, embeds) keep the full editor.
export const LabModeContext = createContext<LabMode>("advanced");
export const useSimple = () => useContext(LabModeContext) === "simple";
/** Set by the page: rule lists as Entry / Exit tabs and condition rows that keep their advanced parts behind "More". */
export const CompactContext = createContext(false);
export const useCompact = () => useContext(CompactContext);
/** Inside a condition row: false hides timing and source controls that are still at their defaults (Simple, or "More" closed). */
export const DetailContext = createContext(true);

const KEY = "lab.mode";
export function storedMode(): LabMode {
  try { return localStorage.getItem(KEY) === "advanced" ? "advanced" : "simple"; } catch { return "simple"; }
}
export function storeMode(mode: LabMode) {
  try { localStorage.setItem(KEY, mode); } catch { /* private window: the choice lasts this visit */ }
}

export const ModeSwitch = ({ mode, onChange }: { mode: LabMode; onChange(mode: LabMode): void }) =>
  <div className="lab-mode" data-lab-mode={mode} title="Simple hides the advanced AmiBroker settings; nothing in your strategy is lost">
    <Segmented label="View" value={mode} options={[{ value: "simple", label: "Simple" }, { value: "advanced", label: "Advanced" }]} onChange={onChange} />
  </div>;

/** About twenty indicators most analysts reach for first; the Library opens on these in the Simple view. */
export const POPULAR = ["sma", "ema", "wma", "rsi", "macd", "bb", "atr", "stoch", "dmi", "supertrend", "obv", "cci", "mfi", "willr", "roc", "mom", "donchian", "kc", "ichimoku", "sar", "highest", "lowest"];
/** The abbreviation analysts type to find an indicator in a list. */
export const SHORT: Record<string, string> = { sma: "SMA", ema: "EMA", wma: "WMA", rsi: "RSI", macd: "MACD", bb: "BB", atr: "ATR", dmi: "DMI", obv: "OBV", cci: "CCI", mfi: "MFI", roc: "ROC", kc: "KC" };

const series = (s: string, p?: Record<string, number>, extra: Record<string, unknown> = {}) => ({ s, ...(p ? { p } : {}), ...extra });
const cond = (lhs: object, op: string, rhs: object) => ({ lhs, op, rhs }) as RuleNode;
export const EXAMPLES: { id: string; label: string; hint: string; entry: RuleNode[]; exit: RuleNode[] }[] = [
  { id: "ma_cross", label: "Moving-average crossover", hint: "Buy when the 50-day average crosses above the 200-day; sell when it crosses back below",
    entry: [cond(series("sma", { length: 50 }), "crosses_above", series("sma", { length: 200 }))],
    exit: [cond(series("sma", { length: 50 }), "crosses_below", series("sma", { length: 200 }))] },
  { id: "rsi_oversold", label: "RSI oversold", hint: "Buy when the 14-day RSI drops below 30; sell when it rises above 70",
    entry: [cond(series("rsi", { length: 14 }), "<", { value: 30 })],
    exit: [cond(series("rsi", { length: 14 }), ">", { value: 70 })] },
  { id: "breakout", label: "52-week breakout", hint: "Buy when the close breaks above the previous 252-day high; sell when it closes below the 50-day average",
    entry: [cond(series("close"), "crosses_above", series("donchian", { length: 252 }, { out: "upper", offset: 1 }))],
    exit: [cond(series("close"), "crosses_below", series("sma", { length: 50 }))] },
];
export function withExample(spec: Spec, id: string): Spec {
  const ex = EXAMPLES.find((e) => e.id === id);
  return ex ? { ...spec, entry: { all: ex.entry }, exit: { any: ex.exit } } : spec;
}
export const ExamplePicker = ({ spec, onChange }: { spec: Spec; onChange(spec: Spec): void }) =>
  <p className="lab-examples" data-lab-examples><span className="cap">Start from an example:</span>
    {EXAMPLES.map((e) => <button key={e.id} type="button" className="dbtn sm" title={e.hint} data-lab-example={e.id} onClick={() => onChange(withExample(spec, e.id))}>{e.label}</button>)}
  </p>;

/** One line saying what the test assumes; Change opens the few settings an analyst adjusts. */
export function SimpleSettings({ spec, settings, onChange, children, open: initial = false }: { spec: Spec; settings: RunSettings; onChange(spec: Spec): void; children: ReactNode; open?: boolean }) {
  const [open, setOpen] = useState(initial);
  const money = modeOf(spec) === "money", dir = directionOf(spec), cost = spec.execution?.cost_bps;
  const period = settings.from || settings.to ? `${settings.from || "first bar"} → ${settings.to || "last bar"}` : "All data";
  const costs = money ? "costs from the cash settings" : `costs ${cost ?? (Number(settings.fees) || 0)} bps + slippage ${Number(settings.slippage) || 0} bps per side`;
  const side = dir === "short" ? "short only" : dir === "both" ? "long and short" : "long only";
  // Rules read the bar's close; this picks the price trades fill at. Orders, AmiBroker prices or split fills are set in Advanced.
  const ex = spec.execution, entryFill = ex?.fill ?? "next_open", exitFill = ex?.exit_fill ?? entryFill;
  const custom = entryOrderType(spec.entry) !== "market" || usesTradePrices(ex) || entryFill !== exitFill;
  const fillText = custom ? "trade prices set in Advanced" : entryFill === "close" ? "trade at the signal day's close" : "trade at the next day's open";
  return <div className="lab-simple-settings" data-lab-simple-settings>
    <p className="lab-simple-summary"><span>{period} · {fillText} · {costs} · {side} · {money ? "cash amounts" : "% returns"}</span>
      <button type="button" className="dbtn sm ghost" aria-expanded={open} onClick={() => setOpen(!open)}>{open ? "Done" : "Change"}</button></p>
    {open && <div className="lab-simple-panel">
      <div className="lab-row"><span className="dfield">Trade</span>
        <Segmented label="Positions" value={dir === "short" ? "short" : "long"} options={[{ value: "long", label: "Long only" }, { value: "short", label: "Short only" }]}
          onChange={(d) => onChange(withDirection(spec, d))} /></div>
      {!custom && <div className="lab-row"><span className="dfield">Buy and sell at</span>
        <Segmented label="Buy and sell at" value={entryFill} options={[{ value: "next_open", label: "Next day's open" }, { value: "close", label: "Signal day's close" }]}
          onChange={(f) => onChange(setExecution(spec, { fill: f === "close" ? f : undefined, exit_fill: undefined, ...(f === "close" ? { delay_bars: undefined } : {}) }))} /></div>}
      {children}
    </div>}
  </div>;
}

/** The analyst's first question: did the idea beat simply holding? Same run numbers as the Advanced report. */
export function SimpleSummary({ result }: { result: RunResult }) {
  const s = result.stats, bh = result.bh, years = yearsBetween(result.dates[0], result.dates.at(-1));
  const bhTotal = bh.length > 1 && bh[0] > 0 ? bh.at(-1)! / bh[0] - 1 : null;
  const bhYear = typeof s.bh_cagr === "number" ? s.bh_cagr : bhTotal !== null && years > 0 ? (1 + bhTotal) ** (1 / years) - 1 : null;
  const bhDd = bh.length ? Math.min(...drawdown(bh)) : null;
  const rows: [string, unknown, unknown][] = [["Total return", s.total_return, bhTotal], ["Per year", s.cagr, bhYear], ["Worst fall", s.max_dd, bhDd]];
  return <section className="box lab-simple-result" data-lab-simple-result>
    <h2 className="box-h">Result <small>{result.from} → {result.to}</small></h2>
    <table className="lab-simple-table"><thead><tr><th /><th>Your idea</th><th>Buy &amp; hold</th></tr></thead>
      <tbody>{rows.map(([label, a, b]) => <tr key={label}><th>{label}</th><td className="n">{pct(a)}</td><td className="n">{pct(b)}</td></tr>)}</tbody></table>
    <p className="cap">Win rate {pct(s.win_rate, 0)} of {int(s.trades)} trades · in the market {pct(s.exposure, 0)} of the time</p>
  </section>;
}
const yearsBetween = (a?: string, b?: string) => a && b ? (Date.parse(b) - Date.parse(a)) / (365.25 * 864e5) : 0;
