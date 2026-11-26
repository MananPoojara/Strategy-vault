/** AmiBroker's Backtest "Detailed log" for a portfolio run (portfolio.rs Ledger.log): per session with activity, the entry
 *  signals in rank order with their score, the exit signals, each Enter / Exit Long fill, the signals skipped for want of a
 *  free position or cash, and the open positions after the bar. */
import { useMemo, useState } from "react";
import { dayFirst } from "./DateField";
import type { PortfolioLogDay } from "./types";

const SHOWN = 300; // sessions drawn at once; the download has every line

export const logText = (days: PortfolioLogDay[]) => days.map((d) => `${d.date ? dayFirst(d.date) : ""}\n${d.lines.map((l) => `   ${l}`).join("\n")}`).join("\n");

export function PortfolioLog({ days, name }: { days: PortfolioLogDay[]; name: string }) {
  const [find, setFind] = useState("");
  const hits = useMemo(() => {
    const q = find.trim().toLowerCase();
    return q ? days.filter((d) => (d.date && (d.date.includes(q) || dayFirst(d.date).includes(q))) || d.lines.some((l) => l.toLowerCase().includes(q))) : days;
  }, [days, find]);
  const download = () => {
    const url = URL.createObjectURL(new Blob([logText(days)], { type: "text/plain;charset=utf-8" }));
    const a = document.createElement("a"); a.href = url; a.download = name; a.click(); URL.revokeObjectURL(url);
  };
  return <details className="lab-portfolio-log" data-lab-portfolio-log>
    <summary>Detailed log · {days.length} sessions with signals or trades</summary>
    <p className="cap">As AmiBroker's Settings › Report › Detailed log: entry signals in rank order (Buy(score); 1 without a rank score), exit signals, each fill, skipped signals and the open positions after the session.</p>
    <div className="lab-row">
      <label className="dfield"><span>Find a stock or date</span><input type="search" value={find} placeholder="e.g. INFY or 30-04-2008" aria-label="Find in the detailed log" onChange={(e) => setFind(e.target.value)} /></label>
      <button type="button" className="dbtn sm" onClick={download}>Download log (.txt)</button>
    </div>
    {hits.length ? <pre tabIndex={0} aria-label="Detailed log">{logText(hits.slice(0, SHOWN))}</pre> : <p className="cap">No session matches “{find}”.</p>}
    {hits.length > SHOWN && <p className="cap">First {SHOWN} of {hits.length} sessions shown; narrow the search or download the full log.</p>}
  </details>;
}
