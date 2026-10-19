import { ProvisionalStrip, Segmented } from "./ui";
import type { InstrumentOption } from "./instruments";
import type { ChartRange, Market, Timeframe } from "./types";
import "./library.css";

const MARKETS: { value: Market; label: string }[] = [
  { value: "in", label: "India" }, { value: "us", label: "US" }, { value: "gl", label: "Global & crypto · provisional" },
];
const TF_LABEL: Record<string, string> = { D: "Daily", W: "Weekly", M: "Monthly" };
const RANGES: ChartRange[] = ["1Y", "3Y", "All"];

export function Toolbar({ market, onMarket, options, ticker, onTicker, tf, timeframes, onTf, range, onRange, provisionalNote }: {
  market: Market; onMarket(m: Market): void; options: InstrumentOption[]; ticker: string; onTicker(t: string): void;
  tf: Timeframe; timeframes: Timeframe[]; onTf(tf: Timeframe): void; range: ChartRange; onRange(r: ChartRange): void; provisionalNote?: string;
}) {
  const selected = options.find((o) => o.ticker === ticker);
  return (
    <div className="box lab-toolbar">
      <div className="lab-row">
        <Segmented label="Market" value={market} options={MARKETS} onChange={onMarket} />
        <label className="dfield">Instrument
          <select value={options.length ? ticker : ""} disabled={!options.length} onChange={(e) => onTicker(e.target.value)}>
            {options.length ? options.map((o) => <option key={o.ticker} value={o.ticker} disabled={o.disabled} title={o.reason}>{o.label}</option>)
              : <option value="">No instruments available</option>}
          </select>
        </label>
        <Segmented label="Chart" value={tf} options={timeframes.map((t) => ({ value: t, label: TF_LABEL[t] ?? t }))} onChange={onTf} />
        <div className="dfield">Chart range
          <Segmented label="Chart range" value={range} options={RANGES.map((r) => ({ value: r, label: r }))} onChange={onRange} />
        </div>
      </div>
      {market === "gl" && <ProvisionalStrip note={provisionalNote} instrumentLabel={selected?.instrumentLabel} />}
    </div>
  );
}
