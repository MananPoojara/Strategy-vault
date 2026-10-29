import { createContext, useContext } from "react";
import { entryOrderType, usesTradePrices } from "./execution-model";
import { setExecution } from "./spec-model";
import { drawdown } from "./RunResults";
import { int, pct } from "./format";
import { Segmented } from "./ui";
import { dayFirst } from "./DateField";
import type { RunResult, Spec } from "./types";
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
    <span className="lab-mode-help">{mode === "simple" ? "Showing the essentials. Advanced shows every setting." : "Showing every setting. Simple shows only the essentials."}</span>
    <Segmented label="View" value={mode} options={[{ value: "simple", label: "Simple" }, { value: "advanced", label: "Advanced" }]} onChange={onChange} />
  </div>;

/** About twenty indicators most analysts reach for first; the Library opens on these in the Simple view. */
export const POPULAR = ["sma", "ema", "wma", "rsi", "macd", "bb", "atr", "stoch", "dmi", "supertrend", "obv", "cci", "mfi", "willr", "roc", "mom", "donchian", "kc", "ichimoku", "sar", "highest", "lowest"];
/** One plain sentence per Popular indicator for the Simple view; the Advanced view shows the formula. */
export const PLAIN: Record<string, string> = {
  sma: "Average closing price over the last N candles; smooths out noise to show the trend.",
  ema: "Average price that reacts faster than SMA because recent candles count more.",
  wma: "Average price with the newest candles weighted most.",
  rsi: "0–100 gauge of recent gains vs losses; above 70 is often called overbought, below 30 oversold.",
  macd: "Gap between a fast and a slow average; crossing its signal line hints at a change in momentum.",
  bb: "Bands a set number of standard deviations (2 by default) around a 20-candle average; price near a band is unusually high or low.",
  atr: "Average size of a candle's range; how much the price usually moves. Useful for stops.",
  stoch: "Where today's close sits within the recent high–low range, from 0 to 100.",
  dmi: "Shows whether buyers (+DI) or sellers (−DI) lead, and ADX shows how strong the trend is.",
  supertrend: "A line under the price in an uptrend and above it in a downtrend; a flip signals a trend change.",
  obv: "Running total of volume, added on up days and subtracted on down days.",
  cci: "How far the price is from its average, scaled; above +100 or below −100 is unusual.",
  mfi: "Like RSI but uses volume too; 0–100 gauge of money flowing in or out.",
  willr: "Where the close sits in the recent range, from −100 (at the low) to 0 (at the high).",
  roc: "Percentage change in price compared with N candles ago.",
  mom: "Price change compared with N candles ago, in points.",
  donchian: "The highest high and lowest low of the last N candles; a break above or below is a breakout.",
  kc: "A channel around an average, its width set by ATR.",
  ichimoku: "A Japanese trend system of several lines and a cloud showing trend, support and resistance.",
  sar: "Dots that trail the price and flip sides when the trend reverses; often used as a trailing stop.",
  highest: "The highest high of the last N candles.",
  lowest: "The lowest low of the last N candles.",
};
const CATEGORY_PLAIN: Record<string, string> = {
  "Moving averages": "A smoothed average of the price that shows the trend.",
  Momentum: "Measures how fast and how strongly the price is moving.",
  Trend: "Shows the direction of the trend and how strong it is.",
  Volatility: "Measures how much the price swings.",
  "Bands & channels": "Lines above and below the price that mark unusually high or low levels.",
  Volume: "Uses traded volume to confirm moves.",
  "Candlestick patterns": "Detects a candle pattern: 1 on candles where it appears, 0 otherwise.",
  Patterns: "Detects a chart pattern.",
  Statistics: "A statistical measure of the price over a window.",
  "Price functions": "A basic calculation on the price, such as the highest or lowest value.",
};
/** Plain description for the Simple view, or undefined (show the formula). */
export const plainOf = (ind: { id: string; category: string }) => PLAIN[ind.id] ?? CATEGORY_PLAIN[ind.category];

/** The abbreviation analysts type to find an indicator in a list. */
export const SHORT: Record<string, string> = { sma: "SMA", ema: "EMA", wma: "WMA", rsi: "RSI", macd: "MACD", bb: "BB", atr: "ATR", dmi: "DMI", obv: "OBV", cci: "CCI", mfi: "MFI", roc: "ROC", kc: "KC" };

/** Next day's open or the signal day's close for both buys and sells; nothing when orders or per-side prices decide. */
export function FillChoice({ spec, onChange }: { spec: Spec; onChange(spec: Spec): void }) {
  const ex = spec.execution, entryFill = ex?.fill ?? "next_open", exitFill = ex?.exit_fill ?? entryFill;
  if (entryOrderType(spec.entry) !== "market" || usesTradePrices(ex) || entryFill !== exitFill) return null;
  return <div className="lab-row"><span className="dfield">Buy and sell at</span>
    <Segmented label="Buy and sell at" value={entryFill} options={[{ value: "next_open", label: "Next day's open" }, { value: "close", label: "Signal day's close" }]}
      onChange={(f) => onChange(setExecution(spec, { fill: f === "close" ? f : undefined, exit_fill: undefined, ...(f === "close" ? { delay_bars: undefined } : {}) }))} /></div>;
}

/** The analyst's first question: did the idea beat simply holding? Same run numbers as the Advanced report. */
export function SimpleSummary({ result }: { result: RunResult }) {
  const s = result.stats, bh = result.bh, years = yearsBetween(result.dates[0], result.dates.at(-1));
  const bhTotal = bh.length > 1 && bh[0] > 0 ? bh.at(-1)! / bh[0] - 1 : null;
  const bhYear = typeof s.bh_cagr === "number" ? s.bh_cagr : bhTotal !== null && years > 0 ? (1 + bhTotal) ** (1 / years) - 1 : null;
  const bhDd = bh.length ? Math.min(...drawdown(bh)) : null;
  const rows: [string, unknown, unknown][] = [["Total return", s.total_return, bhTotal], ["Per year", s.cagr, bhYear], ["Worst fall", s.max_dd, bhDd]];
  return <section className="box lab-simple-result" data-lab-simple-result>
    <h2 className="box-h">Result <small>{dayFirst(result.from)} → {dayFirst(result.to)}</small></h2>
    <table className="lab-simple-table"><thead><tr><th /><th>Your idea</th><th>Buy &amp; hold</th></tr></thead>
      <tbody>{rows.map(([label, a, b]) => <tr key={label}><th>{label}</th><td className="n">{pct(a)}</td><td className="n">{pct(b)}</td></tr>)}</tbody></table>
    <p className="cap">Win rate {pct(s.win_rate, 0)} of {int(s.trades)} trades · in the market {pct(s.exposure, 0)} of the time</p>
  </section>;
}
const yearsBetween = (a?: string, b?: string) => a && b ? (Date.parse(b) - Date.parse(a)) / (365.25 * 864e5) : 0;
