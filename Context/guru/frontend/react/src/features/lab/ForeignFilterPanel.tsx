import type { ReactNode } from "react";
import { NumField } from "./OperandEditor";
import { useForeignScope } from "./ForeignFields";
import { foreignOptions } from "./foreign-model";
import { setExecution } from "./spec-model";
import { Segmented } from "./ui";
import type { Spec } from "./types";

/** The team's AFL "ST For Foreign Filter" (EMA Cross_Foreign Filter_SuperTreand_SMA200_Price Drop.afl): its parameters as
 *  execution.foreign_filter. The engine computes it as the AFL does (series.rs foreign_filter). */
export type ForeignFilter = NonNullable<NonNullable<Spec["execution"]>["foreign_filter"]>;
/** The AFL's defaults: Monthly, period 5, factors 2 and 1, (High+Low)/2, Bullish, exit NO. */
export const FOREIGN_FILTER_DEFAULT = { tf: "M", period: 5, factor1: 2, factor2: 1, price: 5, type: "bullish", exit: false } as const;
/** The AFL's Foreign ST Average Value list, in its order (engine price 0–7). */
export const AVG_VALUES = ["Open", "High", "Low", "Close", "(Open+Close)/2", "(High+Low)/2", "(HLC)/3", "(OHLC)/4"];

/** The filter's symbol when it is switched on: NIFTY 50 when offered (the AFL's first choices are NIFTY / NIFTY 50), else the
 *  first index. */
export function defaultForeignSymbol(tickers: readonly string[]): string {
  return tickers.find((t) => t === "NIFTY 50") ?? tickers.find((t) => /nifty/i.test(t)) ?? tickers[0] ?? "NIFTY 50";
}

export function withForeignFilter(spec: Spec, on: boolean, symbol: string): Spec {
  return setExecution(spec, { foreign_filter: on ? { symbol } : undefined });
}

/** Filters › Foreign filter: one row per AFL parameter, laid out like the rule rows. */
export function ForeignFilterPanel({ spec, onChange }: { spec: Spec; onChange(spec: Spec): void }) {
  const scope = useForeignScope();
  const f = spec.execution?.foreign_filter;
  if (!f) return null;
  const v = { ...FOREIGN_FILTER_DEFAULT, ...f };
  // indices first (their rows carry no stock type), then the stocks
  const options = scope ? foreignOptions(scope).slice().sort((a, b) => Number(!!a.type) - Number(!!b.type)) : [];
  const put = (patch: Partial<ForeignFilter>) => {
    const next: Record<string, unknown> = { ...f, ...patch };
    // a value back at the AFL default is left out, so a saved strategy shows only what was changed
    for (const [k, d] of Object.entries(FOREIGN_FILTER_DEFAULT)) if (next[k] === d) delete next[k];
    onChange(setExecution(spec, { foreign_filter: next as ForeignFilter }));
  };
  const row = (name: string, el: ReactNode) => <div className="lab-param-row" key={name}><span className="lab-param-k">{name}</span><span className="lab-param-v">{el}</span></div>;
  return <div className="lab-param lab-node lab-foreign-filter" data-lab-foreign-filter>
    <div className="lab-param-h"><b>Foreign filter</b><span className="cap">a buy needs the {v.type === "bearish" ? "bearish" : "bullish"} SuperTrend on {v.symbol} ({v.tf === "D" ? "daily" : v.tf === "W" ? "weekly" : "monthly"})</span></div>
    {row("Ticker", <select aria-label="Foreign filter ticker" value={v.symbol} onChange={(e) => put({ symbol: e.target.value })}>
      {!options.some((o) => o.ticker === v.symbol) && <option value={v.symbol}>{v.symbol}</option>}
      {options.map((o) => <option key={o.ticker} value={o.ticker} disabled={o.disabled} title={o.reason}>{o.label}</option>)}
    </select>)}
    {row("Timeframe", <select aria-label="Foreign filter timeframe" value={v.tf} onChange={(e) => put({ tf: e.target.value as ForeignFilter["tf"] })}>
      <option value="M">Monthly</option><option value="W">Weekly</option><option value="D">Daily</option>
    </select>)}
    {row("ST period", <NumField hideLabel label="Foreign filter ST period" value={v.period} min={2} max={100} step={1} onChange={(n) => n !== undefined && put({ period: n })} />)}
    {row("ST factor 1", <NumField hideLabel label="Foreign filter ST factor 1" value={v.factor1} min={0.1} max={50} step={0.5} onChange={(n) => n !== undefined && put({ factor1: n })} />)}
    {row("ST factor 2", <NumField hideLabel label="Foreign filter ST factor 2" value={v.factor2} min={0.1} max={50} step={0.5} onChange={(n) => n !== undefined && put({ factor2: n })} />)}
    {row("Average value", <select aria-label="Foreign filter average value" value={v.price} onChange={(e) => put({ price: Number(e.target.value) })}>
      {AVG_VALUES.map((label, i) => <option key={label} value={i}>{label}</option>)}
    </select>)}
    {row("Type", <Segmented label="Foreign filter type" value={v.type} onChange={(t) => put({ type: t as ForeignFilter["type"] })}
      options={[{ value: "bullish", label: "Bullish" }, { value: "bearish", label: "Bearish" }]} />)}
    {row("Exit", <select aria-label="Foreign filter exit" value={v.exit ? "yes" : "no"} onChange={(e) => put({ exit: e.target.value === "yes" })}>
      <option value="no">No</option><option value="yes">Yes · sell when it fails</option>
    </select>)}
  </div>;
}
