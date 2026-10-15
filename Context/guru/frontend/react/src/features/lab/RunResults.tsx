import { useEffect, useRef } from "react";
import { linePoints } from "./bars";
import { createLabChart, themeColors, useThemeKey } from "./charts";
import { int, num, pct } from "./format";
import { COPY } from "./labels";
import { friendlyReason } from "./spec-model";
import type { RunResult, SavedMarket, Stats, Trade } from "./types";
import { MetricTiles, ProvisionalStrip, ServerLabel, isProvisional } from "./ui";
import "./results.css";

const MAX_ROWS = 200;
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const HEADER = ["Entry date", "Side", "Fill", "Entry", "Exit date", "Exit", "Return", "Bars", "Exit reason"];
const FILL: Record<string, string> = { next_open: "Next open", close: "Close", limit: "Limit" };

/** Engine stats are ratios (same units as the book presenters' `label(…, 'percent')`). */
export const kpiItems = (s: Stats) => [
  { label: "Net return", value: pct(s.total_return) }, { label: "CAGR", value: pct(s.cagr) },
  { label: "Max drawdown", value: pct(s.max_dd) }, { label: "Sharpe", value: num(s.sharpe) },
  { label: "Trades", value: int(s.trades) }, { label: "Win rate", value: pct(s.win_rate) },
  { label: "Avg trade", value: pct(s.avg_trade, 2) }, { label: "Buy & hold CAGR", value: pct(s.bh_cagr) },
];

/** Newest first; an open trade's exit price is the last mark. */
export const tradeRows = (trades: Trade[]) => [...trades].reverse().map((t) => [
  t.entry, t.dir === -1 ? "Short" : "Long", FILL[t.entry_kind] ?? t.entry_kind, num(t.entry_px),
  t.open ? "Open" : t.exit, num(t.exit_px), pct(t.ret, 2), int(t.bars), friendlyReason(t.reason),
]);

export function drawdown(equity: number[]): number[] {
  let peak = -Infinity;
  return equity.map((v) => { peak = Math.max(peak, v); return v / peak - 1; });
}

const sign = (v: number | null | undefined) => (typeof v === "number" ? (v > 0 ? "n pos" : v < 0 ? "n neg" : "n") : "n");

export function RunResults({ result, market }: { result: RunResult; market: SavedMarket }) {
  const { instrument, stats, costs } = result;
  const equityRef = useRef<HTMLDivElement>(null);
  const ddRef = useRef<HTMLDivElement>(null);
  const themeKey = useThemeKey();

  useEffect(() => {
    if (!equityRef.current || !ddRef.current) return;
    const el = equityRef.current;
    const eq = createLabChart(el, 320, { rightPriceScale: { mode: 1, borderColor: themeColors(el).steel } });
    const dd = createLabChart(ddRef.current, 160);
    if (eq) {
      const { chart, colors } = eq;
      chart.addAreaSeries({ lineColor: colors.chart, topColor: colors.chart + "33", bottomColor: "transparent" }).setData(linePoints(result.dates, result.equity));
      chart.addLineSeries({ color: colors.bh, lineWidth: 1 }).setData(linePoints(result.dates, result.bh));
      chart.timeScale().fitContent();
    }
    if (dd) {
      const { chart, colors } = dd;
      chart.addAreaSeries({ lineColor: colors.red, topColor: "transparent", bottomColor: colors.red + "33" }).setData(linePoints(result.dates, drawdown(result.equity)));
      chart.timeScale().fitContent();
    }
    return () => { eq?.dispose(); dd?.dispose(); };
  }, [result, themeKey]);

  return (
    <section className="box lab-result" data-lab-result>
      <h2 className="box-h">Backtest <small>{result.from} → {result.to} · {instrument.name ?? instrument.ticker}</small></h2>
      <ServerLabel text={result.label} />
      {isProvisional(market) && <ProvisionalStrip note={result.provisional_note} instrumentLabel={instrument.label} />}
      {result.start && <p className="rnote">{COPY.startMoved(result.start.date)}</p>}

      <MetricTiles items={kpiItems(stats)} />
      <p className="cap">Exposure {pct(stats.exposure)} · before costs: {pct(stats.gross_total_return)}</p>
      <p className="cap">Fees {costs.fees_bps} bps + slippage {costs.slippage_bps} bps per side · taxes not modelled</p>

      <div className="legend lab-legend">
        <span><i className="sw-line" style={{ background: "var(--chart)" }} />Strategy</span>
        <span><i className="sw-line" style={{ background: "var(--bh)" }} />Buy &amp; hold</span>
      </div>
      <div className="lwc" data-lab-equity ref={equityRef} />
      <div className="lwc" data-lab-drawdown ref={ddRef} />
    </section>
  );
}

/** Monthly returns and tradesheet: a full-width row under the rules/backtest pair. */
export function RunTables({ result }: { result: RunResult }) {
  const { instrument, trades } = result;
  const report = window.VaultReport;
  const monthly = report && result.equity.length ? report.monthly({ dates: result.dates, equity: result.equity.map((v) => v / result.equity[0]) }) : null;
  const rows = tradeRows(trades);

  const download = () => {
    if (!report) return;
    const blob = new Blob([report.csv([HEADER, ...rows])], { type: "text/csv;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `lab-${instrument.ticker}-${result.from}-${result.to}.csv`;
    document.body.append(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  };

  return (
    <section className="box lab-result-tables" data-lab-result-tables>
      {monthly && (
        <div className="tblwrap lab-monthly">
          <table className="tbl">
            <thead><tr><th>Year</th>{MONTHS.map((m) => <th className="n" key={m}>{m}</th>)}<th className="n">Total</th></tr></thead>
            <tbody>
              {monthly.years.map((y) => (
                <tr key={y}>
                  <td>{y}</td>
                  {MONTHS.map((_, i) => {
                    const v = monthly.months[`${y}-${String(i + 1).padStart(2, "0")}`];
                    return <td className={sign(v)} key={i}>{pct(v)}</td>;
                  })}
                  <td className={sign(monthly.annual[y])}>{pct(monthly.annual[y])}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <section className="lab-trades">
        <div className="lab-row">
          <h3 className="box-h">Tradesheet <small>{trades.length} trades · newest first</small></h3>
          {report && trades.length > 0 && <button type="button" className="dbtn sm" onClick={download}>Download CSV</button>}
        </div>
        {trades.length ? (
          <>
            <div className="tlog">
              <table className="tbl tl">
                <thead><tr>{HEADER.map((h, i) => <th key={h} className={[3, 5, 6, 7].includes(i) ? "n" : undefined}>{h}</th>)}</tr></thead>
                <tbody>
                  {rows.slice(0, MAX_ROWS).map((r, k) => (
                    <tr key={k}>
                      {r.map((cell, i) => (
                        <td key={i} className={i === 6 ? sign(trades[trades.length - 1 - k].ret) : [3, 5, 7].includes(i) ? "n" : undefined}>{cell}</td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {rows.length > MAX_ROWS && <p className="cap">{rows.length - MAX_ROWS} older trades are in the CSV.</p>}
          </>
        ) : <p className="empty">{COPY.noTrades}</p>}
      </section>
    </section>
  );
}
