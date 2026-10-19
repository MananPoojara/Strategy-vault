import { useEffect, useRef, useState, type ReactNode } from "react";
import { linePoints } from "./bars";
import { createLabChart, themeColors, useThemeKey } from "./charts";
import { DASH, int, num, pct } from "./format";
import { COPY, ENTRY_KIND_LABELS } from "./labels";
import { friendlyReason } from "./spec-model";
import type { RunResult, SavedMarket, Stats, Piece, ExecutionModel, MoneySummary, Instrument } from "./types";
import { MetricTiles, ProvisionalStrip, Segmented, ServerLabel, isProvisional } from "./ui";
import "./results.css";

const PAGE = 100;
const sign = (v: number | null | undefined) => (typeof v === "number" ? (v > 0 ? "n pos" : v < 0 ? "n neg" : "n") : "n");
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** AmiBroker status line (Profit, CAR, MaxSysDD, CAR/MDD, # winners, # losers); engine stats are ratios. */
export const kpiItems = (s: Stats) => {
  const n = s.trades, won = typeof n === "number" && typeof s.win_rate === "number" ? Math.round(n * s.win_rate) : null;
  const share = (k: number | null) => (k === null ? DASH : `${int(k)} (${pct(n ? k / n : null)})`);
  return [
    { label: "Net Profit %", value: pct(s.total_return) }, { label: "CAR", value: pct(s.cagr) },
    { label: "MaxSysDD", value: pct(s.max_dd) }, { label: "CAR/MDD", value: num(s.mar) },
    { label: "# trades", value: int(n) }, { label: "# winners", value: share(won) }, { label: "# losers", value: share(won === null ? null : n! - won) },
    { label: "Avg. profit/loss %", value: pct(s.avg_trade, 2) }, { label: "Sharpe", value: num(s.sharpe) }, { label: "Buy & hold CAGR", value: pct(s.bh_cagr) },
  ];
};

/** Rows and CSV in trade order (oldest first, as AmiBroker); `cls` holds each cell's class. */
export interface TradeTable { header: string[]; rows: (string | ReactNode)[][]; csv: string[][]; cls: (string | undefined)[][] }
const ledgerAligned = ({ trades, ledger }: Pick<RunResult, "trades" | "ledger">) => !!ledger?.length && ledger.length === trades.length && ledger.every((r, i) => r.entry === trades[i].entry);
const pieceText = (pieces: Piece[]) => pieces.map(([date, price, weight]) => `${date} @ ${num(price)} × ${Math.round(weight * 100)}%`).join("; ");
const NUMERIC = new Set(["Price", "Ex. Price", "Cum. Profit", "Cum. %", "Profit", "% Profit", "Shares", "Position value", "Allocation", "# bars", "MAE", "MFE", "Profit/bar", "Costs"]);
export function tradeTable(result: Pick<RunResult, "trades" | "ledger"> & { money?: Pick<MoneySummary, "capital">; instrument?: Pick<Instrument, "ticker"> }): TradeTable {
  const { trades, ledger } = result;
  const legs = trades.some((t) => !!t.legs?.length), fills = trades.some((t) => !!t.fills?.length), sized = trades.some((t) => t.weight !== undefined), money = ledgerAligned(result);
  const excursions = trades.some((t) => typeof t.mae === "number");
  // AmiBroker result-list names and order; Order, Allocation, Costs and Exit reason are this lab's own columns.
  const header = ["Symbol", "Trade", "Order", "Date", "Price", "Ex. date", "Ex. Price", ...(money ? ["Cum. Profit", "Profit", "% Profit", "Shares", "Position value"] : ["Cum. %", "% Profit"]),
    ...(sized ? ["Allocation"] : []), "# bars", ...(excursions ? ["MAE", "MFE"] : []), "Profit/bar", ...(legs ? ["Scale in"] : []), ...(fills ? ["Scale out"] : []), ...(money ? ["Costs"] : []), "Exit reason"];
  // Cumulative in trade order, the open (marked) trade included: unit mode compounds the trade returns (the unit equity
  // line); money mode sums ledger P&L (cash equity, before idle-cash interest).
  let growth = 1, pnl = 0;
  const csv: string[][] = [header], rows: (string | ReactNode)[][] = [], cls: (string | undefined)[][] = [];
  trades.forEach((t, i) => {
    const r = money ? ledger![i] : undefined, cum = r ? (pnl += r.pnl) : (growth *= 1 + (t.ret ?? 0)) - 1;
    const cell: Record<string, string> = {
      Symbol: result.instrument?.ticker ?? "", Trade: t.dir === -1 ? "Short" : "Long", Order: ENTRY_KIND_LABELS[t.entry_kind] ?? t.entry_kind,
      Date: t.entry, Price: num(t.entry_px), "Ex. date": t.open ? "Open" : t.exit, "Ex. Price": num(t.exit_px),
      "Cum. Profit": num(cum), "Cum. %": pct(cum, 1), Profit: num(r?.pnl), "% Profit": pct(t.ret, 2), Shares: int(r?.qty), "Position value": num(r?.notional),
      Allocation: pct(t.weight ?? 1), "# bars": int(t.bars), MAE: pct(t.mae, 2), MFE: pct(t.mfe, 2),
      // AmiBroker # bars counts the entry and the exit bar
      "Profit/bar": r ? num(r.pnl / (t.bars + 1)) : pct(typeof t.ret === "number" ? t.ret / (t.bars + 1) : null, 3),
      "Scale in": t.legs?.length ? pieceText(t.legs) : "1", "Scale out": t.fills?.length ? pieceText(t.fills) : DASH, Costs: num(r?.costs), "Exit reason": friendlyReason(t.reason),
    };
    const row = header.map((h) => cell[h]);
    csv.push(row);
    const view: (string | ReactNode)[] = [...row];
    for (const [label, pieces, word] of [["Scale in", t.legs, "legs"], ["Scale out", t.fills, "scale-outs"]] as const) {
      if (pieces?.length) view[header.indexOf(label)] = <details><summary>{pieces.length} {word}</summary>{pieceText(pieces)}</details>;
    }
    rows.push(view);
    cls.push(header.map((h) => (h === "% Profit" ? sign(t.ret) : h === "Profit" || h === "Cum. Profit" ? sign(h === "Profit" ? r?.pnl : cum) : NUMERIC.has(h) ? "n" : undefined)));
  });
  return { header, rows, csv, cls };
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
      {result.money ? <><p className="cap">{result.money.basis}</p>{result.money.skipped > 0 && <p className="cap">{result.money.skipped} trade{result.money.skipped === 1 ? "" : "s"} skipped ({result.money.limits
        ? Object.entries(result.money.limits.skip_reasons).map(([k, n]) => `${k.replaceAll("_", " ")}: ${n}`).join(" · ") : "size rounded to 0"})</p>}
        {result.money.limits && <p className="cap" data-lab-limits-basis>Interest on idle cash: {result.money.limits.interest.toLocaleString("en-US", { maximumFractionDigits: 2 })}. {result.money.limits.basis}</p>}</> : <p className="cap">Fees {costs.fees_bps} bps + slippage {costs.slippage_bps} bps per side · taxes not modelled</p>}

      <div className="lab-result-charts">
        <figure className="lab-result-chart" data-lab-chart="equity">
          <figcaption className="lab-chart-h"><h3>Portfolio Equity</h3><output ref={equityValueRef} data-lab-equity-value>Latest {num(result.equity.at(-1))}×</output></figcaption>
          <div className="legend lab-legend lab-chart-sub">
            <span><i className="sw-line" style={{ background: "var(--chart)" }} />Strategy</span>
            <span><i className="sw-line" style={{ background: "var(--bh)" }} />Buy &amp; hold</span>
            <span className="cap lab-scale-note">log scale</span>
          </div>
          <div className="lwc" data-lab-equity aria-label="Portfolio Equity chart" ref={equityRef} />
        </figure>
        <figure className="lab-result-chart" data-lab-chart="drawdown">
          <figcaption className="lab-chart-h"><h3>Underwater Equity</h3><output ref={ddValueRef} data-lab-drawdown-value>Latest {pct(ddValues.at(-1))}</output></figcaption>
          <p className="cap lab-chart-sub">0% is the previous peak · losses appear below zero</p>
          <div className="lwc bkc" data-lab-drawdown aria-label="Underwater Equity chart" ref={ddRef} />
        </figure>
      </div>
    </section>
  );
}

/** AmiBroker Profit Table and the tradesheet (result list): a full-width row under the rules/backtest pair. */
/** `part` shows only the Profit Table ("monthly") or only the Tradesheet and ledger ("trades"); both when absent. */
export function RunTables({ result, part }: { result: RunResult; part?: "monthly" | "trades" }) {
  const { instrument, trades } = result;
  const [order, setOrder] = useState<"oldest" | "newest">("oldest");
  const [page, setPage] = useState(0);
  useEffect(() => setPage(0), [result]); // a new backtest starts on its first page
  const report = window.VaultReport;
  const monthly = report && result.equity.length ? report.monthly({ dates: result.dates, equity: result.equity.map((v) => v / result.equity[0]) }) : null;
  const table = tradeTable(result), pages = Math.max(1, Math.ceil(table.rows.length / PAGE)), at = Math.min(page, pages - 1);
  const shown = (order === "oldest" ? table.rows.map((_, k) => k) : table.rows.map((_, k) => table.rows.length - 1 - k)).slice(at * PAGE, (at + 1) * PAGE);

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
      {part !== "trades" && monthly && (
        <div className="tblwrap lab-monthly">
          <table className="tbl">
            <caption>Profit Table</caption>
            <thead><tr><th>Year</th>{MONTHS.map((m) => <th className="n" key={m}>{m}</th>)}<th className="n">Yr%</th></tr></thead>
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
            {!!monthly.averages?.length && <tfoot><tr><td>Avg</td>{MONTHS.map((_, i) => <td className={sign(monthly.averages[i])} key={i}>{pct(monthly.averages[i])}</td>)}
              <td className={sign(monthly.annualAverage)}>{pct(monthly.annualAverage)}</td></tr></tfoot>}
          </table>
        </div>
      )}

      {part !== "monthly" && <>
      <section className="lab-trades">
        <div className="lab-row">
          <h3 className="box-h">Tradesheet <small>{trades.length} trades</small></h3>
          {trades.length > 1 && <Segmented label="Trade order" value={order} onChange={(v) => { setOrder(v); setPage(0); }}
            options={[{ value: "oldest", label: "Oldest first" }, { value: "newest", label: "Newest first" }]} />}
          {report && trades.length > 0 && <button type="button" className="dbtn sm" onClick={download}>Download CSV</button>}
        </div>
        {trades.length ? (
          <>
            <div className="tlog">
              <table className="tbl tl" data-lab-trades>
                <thead><tr>{table.header.map((h) => <th key={h} className={NUMERIC.has(h) ? "n" : undefined}>{h}</th>)}</tr></thead>
                <tbody>
                  {shown.map((k) => <tr key={k}>{table.rows[k].map((cell, i) => <td key={i} className={table.cls[k][i]}>{cell}</td>)}</tr>)}
                </tbody>
              </table>
            </div>
            {pages > 1 && <div className="lab-row lab-pager" data-lab-pager>
              <button type="button" className="dbtn sm" disabled={at === 0} onClick={() => setPage(at - 1)}>‹ Previous</button>
              <span className="cap">{int(at * PAGE + 1)}–{int(Math.min((at + 1) * PAGE, table.rows.length))} of {int(table.rows.length)}</span>
              <button type="button" className="dbtn sm" disabled={at === pages - 1} onClick={() => setPage(at + 1)}>Next ›</button>
            </div>}
          </>
        ) : <p className="empty">{COPY.noTrades}</p>}
      </section>
      {!!result.ledger?.length && !ledgerAligned(result) && <div className="tblwrap"><table className="tbl" data-lab-ledger>
        <caption>Money ledger</caption><thead><tr>{["Entry date", "Qty", "Value", "Costs", "P&L"].map((h) => <th key={h}>{h}</th>)}</tr></thead>
        <tbody>{result.ledger.map((r, i) => <tr key={i}><td>{r.entry}</td><td className="n">{int(r.qty)}</td><td className="n">{num(r.notional)}</td><td className="n">{num(r.costs)}</td><td className={sign(r.pnl)}>{num(r.pnl)}</td></tr>)}</tbody>
      </table></div>}
      </>}
      {part === "monthly" && !monthly && <p className="empty">No monthly returns for this run.</p>}
    </section>
  );
}
