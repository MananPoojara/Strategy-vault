import { useState, type ReactNode } from "react";
import { ChartPopup } from "./ChartPopup";
import { ProvisionalStrip, Segmented } from "./ui";
import type { InstrumentOption } from "./instruments";
import type { ChartRange, Market, Timeframe } from "./types";
import "./library.css";
import "./chart-timeframes.css";

const MARKETS: { value: Market; label: string }[] = [
  { value: "in", label: "India" }, { value: "us", label: "US" }, { value: "gl", label: "Global & crypto · provisional" },
];
const TF_LABEL: Record<string, string> = { D: "Daily", W: "Weekly", M: "Monthly", Q: "Quarterly", Y: "Yearly" };
const TF_SHORT: Record<string, string> = { D: "1D", W: "1W", M: "1M", Q: "3M", Y: "1Y" };
const QUICK_TF = ["D", "W", "M"];
const RANGES: ChartRange[] = ["1Y", "3Y", "All"];

export function Toolbar({ market, onMarket, options, ticker, onTicker, tf, timeframes, onTf, range, onRange, provisionalNote, chartWorkspace = false, children }: {
  market: Market; onMarket(m: Market): void; options: InstrumentOption[]; ticker: string; onTicker(t: string): void;
  tf: Timeframe; timeframes: Timeframe[]; onTf(tf: Timeframe): void; range: ChartRange; onRange(r: ChartRange): void; provisionalNote?: string; chartWorkspace?: boolean; children?: ReactNode;
}) {
  const [symbolsOpen, setSymbolsOpen] = useState(false);
  const [search, setSearch] = useState("");
  const selected = options.find((o) => o.ticker === ticker);
  const quick = QUICK_TF.filter((t) => timeframes.includes(t));
  const more = timeframes.filter((t) => !QUICK_TF.includes(t));
  if (chartWorkspace) return <div className="lab-toolbar lab-toolbar-chart">
    <div className="lab-chart-topbar" role="toolbar" aria-label="Chart controls">
      <button type="button" className="lab-symbol-button" aria-label="Symbol search" aria-haspopup="dialog" aria-expanded={symbolsOpen}
        onClick={() => { setSearch(""); setSymbolsOpen(true); }}>
        <svg width="16" height="16" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true"><circle cx="8.5" cy="8.5" r="5.5"/><path d="m13 13 4 4"/></svg>
        {selected?.label ?? (ticker || "Symbol")}
      </button>
      <div className="lab-chart-timeframes lab-intervals-flat" role="group" aria-label="Chart">
        {quick.map((t) => <button key={t} type="button" aria-label={TF_LABEL[t]} title={TF_LABEL[t]} aria-pressed={tf === t} onClick={() => onTf(t)}>{TF_SHORT[t]}</button>)}
        <details className="lab-interval-menu" onKeyDown={(event) => {
          if (event.key === "Escape") { event.currentTarget.removeAttribute("open"); event.currentTarget.querySelector('summary')?.focus(); }
        }}>
          <summary aria-label="More timeframes" title="Chart interval">{quick.includes(tf) ? "⌄" : TF_SHORT[tf] ?? tf}</summary>
          <div className="lab-interval-list" role="menu" aria-label="Chart intervals">
            <span>INTERVALS</span>
            {timeframes.map((t) => <button key={t} type="button" role="menuitemradio" aria-checked={tf === t} onClick={(event) => {
              onTf(t);
              const menu = event.currentTarget.closest('details'); menu?.removeAttribute('open'); menu?.querySelector('summary')?.focus();
            }}><span>{TF_LABEL[t] ?? t}</span><small>{TF_SHORT[t] ?? t}</small></button>)}
            {!timeframes.length && <p className="cap">No intervals available</p>}
          </div>
        </details>
      </div>
      {children}
    </div>
    {market === "gl" && <ProvisionalStrip note={provisionalNote} instrumentLabel={selected?.instrumentLabel} />}
    {symbolsOpen && <ChartPopup title="Symbol search" onClose={() => setSymbolsOpen(false)}>
      <div className="lab-browser-heading"><h2>Symbol search</h2><button type="button" className="dbtn sm ghost" aria-label="Close symbol search" onClick={() => setSymbolsOpen(false)}>×</button></div>
      <div className="lab-symbol-search">
        <label><span className="sr-only">Search instruments</span><input aria-label="Search instruments" type="search" placeholder="Search symbol or instrument name" value={search} onChange={(event) => setSearch(event.target.value)} /></label>
        <Segmented label="Market" value={market} options={MARKETS} onChange={(next) => { setSearch(""); onMarket(next); }} />
      </div>
      <div className="lab-symbol-results" aria-label="Instruments">
        {options.filter((option) => `${option.ticker} ${option.label}`.toLowerCase().includes(search.trim().toLowerCase())).map((option) =>
          <button key={option.ticker} type="button" disabled={option.disabled} title={option.reason} aria-pressed={option.ticker === ticker} onClick={() => { onTicker(option.ticker); setSymbolsOpen(false); }}>
            <span><b>{option.ticker}</b><small>{option.label !== option.ticker ? option.label : option.type ?? ""}</small></span>
            <span>{option.disabled ? option.reason ?? "Not available" : option.market?.toUpperCase() ?? MARKETS.find((item) => item.value === market)?.label}</span>
          </button>)}
        {!options.length && <p className="empty">No instruments available</p>}
        {!!options.length && !options.some((option) => `${option.ticker} ${option.label}`.toLowerCase().includes(search.trim().toLowerCase())) && <p className="empty">No instrument matches “{search}”.</p>}
      </div>
    </ChartPopup>}
  </div>;
  return (
    <div className="box lab-toolbar">
      <div className="lab-row">
        <div className="dfield" title="Which exchange's instruments to list">Market
          <Segmented label="Market" value={market} options={MARKETS} onChange={onMarket} />
        </div>
        <label className="dfield">Instrument
          <select value={options.length ? ticker : ""} disabled={!options.length} onChange={(e) => onTicker(e.target.value)}>
            {options.length ? options.map((o) => <option key={o.ticker} value={o.ticker} disabled={o.disabled} title={o.reason}>{o.label}</option>)
              : <option value="">No instruments available</option>}
          </select>
        </label>
        <div className="dfield" title="How much time one candle covers: 1D = daily, 1W = weekly, 1M = monthly">Timeframe
          <div className="lab-chart-timeframes join sm" role="group" aria-label="Chart">
            {quick.map((t) => <button key={t} type="button" className={`join-item${tf === t ? " active" : ""}`} aria-label={TF_LABEL[t]} title={TF_LABEL[t]} aria-pressed={tf === t} onClick={() => onTf(t)}>{TF_SHORT[t]}</button>)}
            {more.length > 0 && <select aria-label="More timeframes" value={more.includes(tf) ? tf : ""} onChange={(e) => { if (e.target.value) onTf(e.target.value); }}>
              <option value="" disabled>More intervals</option>
              {more.map((t) => <option key={t} value={t} title={TF_LABEL[t] ?? t}>{TF_SHORT[t] ?? t}</option>)}
            </select>}
            {!timeframes.length && <span className="cap">No intervals available</span>}
          </div>
        </div>
        <div className="dfield" title="How much history the chart shows; the backtest period is set in Settings">Chart range
          <Segmented label="Chart range" value={range} options={RANGES.map((r) => ({ value: r, label: r }))} onChange={onRange} />
        </div>
      </div>
      {market === "gl" && <ProvisionalStrip note={provisionalNote} instrumentLabel={selected?.instrumentLabel} />}
    </div>
  );
}
