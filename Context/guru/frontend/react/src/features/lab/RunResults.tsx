import { useEffect, useRef, type ReactNode } from "react";
import { linePoints } from "./bars";
import { createLabChart, themeColors, useThemeKey } from "./charts";
import { int, num, pct } from "./format";
import { COPY, ENTRY_KIND_LABELS } from "./labels";
import { friendlyReason } from "./spec-model";
import type { RunResult, SavedMarket, Stats, Trade, Piece, ExecutionModel, MoneySummary } from "./types";
import { MetricTiles, ProvisionalStrip, ServerLabel, isProvisional } from "./ui";
import "./results.css";

const MAX_ROWS = 200;
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
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

export interface TradeTable { header: string[]; rows: (string | ReactNode)[][]; csv: string[][]; numeric: number[]; ret: number }
const ledgerAligned = ({ trades, ledger }: Pick<RunResult, "trades" | "ledger">) => !!ledger?.length && ledger.length === trades.length && ledger.every((r, i) => r.entry === trades[i].entry);
const pieceText = (pieces: Piece[]) => pieces.map(([date, price, weight]) => `${date} @ ${num(price)} × ${Math.round(weight * 100)}%`).join("; ");
export function tradeTable(result: Pick<RunResult, "trades" | "ledger">): TradeTable {
  const { trades, ledger } = result;
  const legs = trades.some((t) => !!t.legs?.length), fills = trades.some((t) => !!t.fills?.length), sized = trades.some((t) => t.weight !== undefined), money = ledgerAligned(result);
  const header = ["Entry date", "Side", "Order", "Entry", ...(legs ? ["Legs"] : []), "Exit date", "Exit", ...(fills ? ["Scale-outs"] : []), "Return", "Bars", "Exit reason", ...(sized ? ["Allocation"] : []), ...(money ? ["Qty", "Value", "Costs", "P&L"] : [])];
  const ret = header.indexOf("Return"), numeric = header.flatMap((h, i) => ["Entry", "Exit", "Return", "Bars", "Allocation", "Qty", "Value", "Costs", "P&L"].includes(h) ? [i] : []);
  const csv: string[][] = [header], rows: (string | ReactNode)[][] = [];
  for (let i = trades.length - 1; i >= 0; i--) {
    const t = trades[i], row = [t.entry, t.dir === -1 ? "Short" : "Long", ENTRY_KIND_LABELS[t.entry_kind] ?? t.entry_kind, num(t.entry_px), ...(legs ? [t.legs?.length ? pieceText(t.legs) : "1"] : []), t.open ? "Open" : t.exit, num(t.exit_px), ...(fills ? [t.fills?.length ? pieceText(t.fills) : "—"] : []), pct(t.ret, 2), int(t.bars), friendlyReason(t.reason), ...(sized ? [pct(t.weight ?? 1)] : [])];
    if (money) { const r = ledger![i]; row.push(int(r.qty), num(r.notional), num(r.costs), num(r.pnl)); }
    csv.push(row);
    const view: (string | ReactNode)[] = [...row];
    for (const [label, pieces, word] of [["Legs", t.legs, "legs"], ["Scale-outs", t.fills, "scale-outs"]] as const) {
      if (pieces?.length) view[header.indexOf(label)] = <details><summary>{pieces.length} {word}</summary>{pieceText(pieces)}</details>;
    }
    rows.push(view);
  }
  return { header, rows, csv, ret, numeric };
}
export function ExecutionStrip({ model, money, adaptations }: { model?: ExecutionModel; money?: Pick<MoneySummary, "capital"> & { currency?: string | null }; adaptations?: string[] }) {
  if (!model && !money && !adaptations?.length) return null;
  return <div className="lab-exec-strip">
    <div className="lab-row">{money ? <span className="chip" data-lab-money>Money · {money.currency ?? ""} {money.capital.toLocaleString("en-US")}</span> : model && <span className="chip">{COPY.unitStudy}</span>}{model?.executable === false && <span className="chip">{COPY.studyOnly}</span>}</div>
    {!!model?.notes.length && <ul className="rnote" data-lab-execution-notes>{model.notes.map((note, i) => <li key={i}>{note}</li>)}</ul>}
    {!!model?.events?.length && <p className="cap" data-lab-events>{COPY.riskEvents}: {model.events.join(" · ")}</p>}
    {adaptations?.map((note, i) => <p className="rnote" role="note" data-lab-adaptations key={i}>{note}</p>)}
  </div>;
}

export function drawdown(equity: number[]): number[] {
  // Match the engine’s declared pre-event unit baseline, including initial costs.
  let peak = 1;
  return equity.map((v) => { peak = Math.max(peak, v); return v / peak - 1; });
}

const sign = (v: number | null | undefined) => (typeof v === "number" ? (v > 0 ? "n pos" : v < 0 ? "n neg" : "n") : "n");

export function RunResults({ result, market }: { result: RunResult; market: SavedMarket }) {
  const { instrument, stats, costs } = result;
  const equityRef = useRef<HTMLDivElement>(null);
  const ddRef = useRef<HTMLDivElement>(null);
  const equityValueRef = useRef<HTMLOutputElement>(null);
  const ddValueRef = useRef<HTMLOutputElement>(null);
  const ddValues = drawdown(result.equity);
  const themeKey = useThemeKey();

  useEffect(() => {
    if (!equityRef.current || !ddRef.current) return;
    const el = equityRef.current, ddEl = ddRef.current;
    const eq = createLabChart(el, 300, { timeScale: { minBarSpacing: 0.01, borderColor: themeColors(el).steel }, rightPriceScale: { mode: 1, borderColor: themeColors(el).steel } });
    let offCross: (() => void) | undefined;
    if (eq) {
      const { chart, colors } = eq;
      chart.addAreaSeries({ lineColor: colors.chart, topColor: colors.chart + "33", bottomColor: "transparent" }).setData(linePoints(result.dates, result.equity));
      chart.addLineSeries({ color: colors.bh, lineWidth: 1 }).setData(linePoints(result.dates, result.bh));
      chart.timeScale().fitContent();
      const byDate = new Map(result.dates.map((date, i) => [date, i]));
      const onCross = ({ time }: { time?: string }) => {
        const value: unknown = time;
        const date = typeof value === "string" ? value : value && typeof value === "object" && "year" in value && "month" in value && "day" in value
          ? `${value.year}-${String(value.month).padStart(2, "0")}-${String(value.day).padStart(2, "0")}` : undefined;
        const i = date ? byDate.get(date) : undefined;
        if (equityValueRef.current) equityValueRef.current.textContent = i === undefined ? `Latest ${num(result.equity.at(-1))}×` : `${result.dates[i]} · ${num(result.equity[i])}×`;
      };
      chart.subscribeCrosshairMove?.(onCross);
      offCross = () => chart.unsubscribeCrosshairMove?.(onCross);
    }
    const values = drawdown(result.equity);
    const renderDrawdown = () => {
      ddEl.classList.remove("lab-chart-empty");
      if (!values.length || values.length !== result.dates.length || values.some((value) => !Number.isFinite(value))) {
        ddEl.classList.add("lab-chart-empty"); ddEl.textContent = "No drawdown data for this period."; return;
      }
      // The shared renderer needs at least two x coordinates. Never invent a second session for a single observation.
      if (values.length === 1) {
        ddEl.classList.add("lab-chart-empty"); ddEl.textContent = `Drawdown ${pct(values[0], 2)} · ${result.dates[0]}`; return;
      }
      const render = window.VaultRuntime?.renderAreaChart;
      if (!render) { ddEl.classList.add("lab-chart-empty"); ddEl.textContent = "Drawdown chart is unavailable."; return; }
      // One renderer and the same house chart as book results: gradient, minimum marker, crosshair and tooltip.
      render(ddEl, result.dates, values, { color: themeColors(ddEl).red, fmt: (value) => pct(value, 2), axFmt: (value) => pct(value, 0), markMin: true, h: 300 });
    };
    renderDrawdown();
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(renderDrawdown);
    observer?.observe(ddEl);
    return () => { observer?.disconnect(); offCross?.(); eq?.dispose(); ddEl.replaceChildren(); };
  }, [result, themeKey]);

  return (
    <section className="box lab-result" data-lab-result tabIndex={-1}>
      <h2 className="box-h">Backtest <small>{result.from} → {result.to} · {instrument.name ?? instrument.ticker}</small></h2>
      <ServerLabel text={result.label} />
      {isProvisional(market) && <ProvisionalStrip note={result.provisional_note} instrumentLabel={instrument.label} />}
      {result.start && <p className="rnote">{COPY.startMoved(result.start.date)}</p>}

      <ExecutionStrip model={result.execution_model} money={result.money} adaptations={result.adaptations} />
      <MetricTiles items={[...(result.money ? [{ label: "Final equity", value: num(result.money.final_equity) }, { label: "Costs paid", value: num(result.money.costs_total) }] : []), ...kpiItems(stats)]} />
      <p className="cap">Exposure {pct(stats.exposure)} · before costs: {pct(stats.gross_total_return)}</p>
      {result.money ? <><p className="cap">{result.money.basis}</p>{result.money.skipped > 0 && <p className="cap">{result.money.skipped} trade{result.money.skipped === 1 ? "" : "s"} skipped (size rounded to 0)</p>}</> : <p className="cap">Fees {costs.fees_bps} bps + slippage {costs.slippage_bps} bps per side · taxes not modelled</p>}

      <div className="lab-result-charts">
        <figure className="lab-result-chart" data-lab-chart="equity">
          <figcaption className="lab-chart-h"><h3>Equity growth</h3><output ref={equityValueRef} data-lab-equity-value>Latest {num(result.equity.at(-1))}×</output></figcaption>
          <div className="legend lab-legend lab-chart-sub">
            <span><i className="sw-line" style={{ background: "var(--chart)" }} />Strategy</span>
            <span><i className="sw-line" style={{ background: "var(--bh)" }} />Buy &amp; hold</span>
            <span className="cap">Logarithmic scale</span>
          </div>
          <div className="lwc" data-lab-equity aria-label="Equity growth chart" ref={equityRef} />
        </figure>
        <figure className="lab-result-chart" data-lab-chart="drawdown">
          <figcaption className="lab-chart-h"><h3>Drawdown from peak</h3><output ref={ddValueRef} data-lab-drawdown-value>Latest {pct(ddValues.at(-1))}</output></figcaption>
          <p className="cap lab-chart-sub">0% is the previous peak · losses appear below zero</p>
          <div className="lwc bkc" data-lab-drawdown aria-label="Drawdown from peak chart" ref={ddRef} />
        </figure>
      </div>
    </section>
  );
}

/** Monthly returns and tradesheet: a full-width row under the rules/backtest pair. */
export function RunTables({ result }: { result: RunResult }) {
  const { instrument, trades } = result;
  const report = window.VaultReport;
  const monthly = report && result.equity.length ? report.monthly({ dates: result.dates, equity: result.equity.map((v) => v / result.equity[0]) }) : null;
  const table = tradeTable(result), rows = table.rows;

  const download = () => {
    if (!report) return;
    const blob = new Blob([report.csv(table.csv)], { type: "text/csv;charset=utf-8" });
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
              <table className="tbl tl" data-lab-trades>
                <thead><tr>{table.header.map((h, i) => <th key={h} className={table.numeric.includes(i) ? "n" : undefined}>{h}</th>)}</tr></thead>
                <tbody>
                  {rows.slice(0, MAX_ROWS).map((r, k) => (
                    <tr key={k}>
                      {r.map((cell, i) => (
                        <td key={i} className={i === table.ret ? sign(trades[trades.length - 1 - k].ret) : table.header[i] === "P&L" ? sign(result.ledger?.[trades.length - 1 - k]?.pnl) : table.numeric.includes(i) ? "n" : undefined}>{cell}</td>
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
      {!!result.ledger?.length && !ledgerAligned(result) && <div className="tblwrap"><table className="tbl" data-lab-ledger>
        <caption>Money ledger</caption><thead><tr>{["Entry date", "Qty", "Value", "Costs", "P&L"].map((h) => <th key={h}>{h}</th>)}</tr></thead>
        <tbody>{result.ledger.map((r, i) => <tr key={i}><td>{r.entry}</td><td className="n">{int(r.qty)}</td><td className="n">{num(r.notional)}</td><td className="n">{num(r.costs)}</td><td className={sign(r.pnl)}>{num(r.pnl)}</td></tr>)}</tbody>
      </table></div>}
    </section>
  );
}
