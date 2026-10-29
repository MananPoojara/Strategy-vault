import { useEffect, useRef, useState, type ReactNode } from "react";
import { dayFirst } from "./DateField";
import { linePoints } from "./bars";
import { createLabChart, themeColors, useThemeKey } from "./charts";
import { DASH, int, num, pct } from "./format";
import { COPY } from "./labels";
import { friendlyReason } from "./spec-model";
import { reasonLabeler, type RunContext } from "./trade-explain";
import { TsDownload } from "./TsDownload";
import type { Catalogue, RunResult, SavedMarket, Spec, Stats, Piece, ExecutionModel, MoneySummary, Instrument } from "./types";
import { neverMeets } from "./scale-check";
import { MetricTiles, ProvisionalStrip, Segmented, ServerLabel, isProvisional } from "./ui";
import "./results.css";

const PAGE = 100;
/** Profit Table cell: blank for a month or year with no return (flat equity, or not reached yet). */
const ret = (v: number | null | undefined) => (typeof v === "number" && Math.abs(v) > 1e-12 ? pct(v) : "");
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
/** Dates and the side read centred in their column; numbers right-aligned; the symbol left. */
const CENTRED = new Set(["Trade type", "Entry date", "Exit date"]);
const NUMERIC = new Set(["Entry price", "Exit price", "% Change", "Bars held", "Days", "Net profit", "MAE", "MFE", "DD %", "Shares", "Position value", "Allocation", "Costs"]);
export function tradeTable(result: Pick<RunResult, "trades" | "ledger"> & { money?: Pick<MoneySummary, "capital">; instrument?: Pick<Instrument, "ticker"> },
  reasonOf: (reason: string) => string = friendlyReason): TradeTable {
  const { trades, ledger } = result;
  const legs = trades.some((t) => !!t.legs?.length), fills = trades.some((t) => !!t.fills?.length), sized = trades.some((t) => t.weight !== undefined), money = ledgerAligned(result);
  const excursions = trades.some((t) => typeof t.mae === "number");
  // The analyst's order: the trade, its prices, the move, how long it ran, the profit, then the running result and its drawdown.
  // Allocation, scale-in/out and the money columns appear only when the run has them; the Excel download adds the exit reason.
  const header = ["Symbol", "Trade type", "Entry date", "Entry price", "Exit date", "Exit price", "% Change", "Bars held", "Days", "Net profit",
    ...(excursions ? ["MAE", "MFE"] : []), "DD %",
    ...(sized ? ["Allocation"] : []), ...(money ? ["Shares", "Position value", "Costs"] : []), ...(legs ? ["Scale in"] : []), ...(fills ? ["Scale out"] : [])];
  // Cumulative in trade order, the open (marked) trade included: unit mode compounds the trade returns (the unit equity
  // line); money mode sums ledger P&L (cash equity, before idle-cash interest). DD % is that equity against its running peak.
  let growth = 1, pnl = 0, peak = 1;
  const capital = result.money?.capital ?? 1;
  const csv: string[][] = [[...header, "Exit reason"]], rows: (string | ReactNode)[][] = [], cls: (string | undefined)[][] = [];
  trades.forEach((t, i) => {
    const r = money ? ledger![i] : undefined, before = growth;
    const cum = r ? (pnl += r.pnl) : (growth *= 1 + (t.ret ?? 0)) - 1;
    const equity = r ? 1 + cum / capital : growth, dd = equity / (peak = Math.max(peak, equity)) - 1;
    // unit mode: the trade's profit as a share of the starting equity (the % change on the equity it was opened with)
    const net = r ? r.pnl : typeof t.ret === "number" ? before * t.ret : null;
    const days = t.exit && t.entry ? Math.round((Date.parse(t.exit) - Date.parse(t.entry)) / 864e5) : null;
    const cell: Record<string, string> = {
      Symbol: result.instrument?.ticker ?? "", "Trade type": t.dir === -1 ? "Short" : "Long",
      "Entry date": dayFirst(t.entry), "Entry price": num(t.entry_px), "Exit date": t.open ? "Open" : dayFirst(t.exit), "Exit price": num(t.exit_px),
      "% Change": pct(t.ret, 2), "Bars held": int(t.bars), Days: days === null ? DASH : int(days), "Net profit": r ? num(net) : pct(net, 2),
      MAE: pct(t.mae, 2), MFE: pct(t.mfe, 2), "DD %": pct(dd, 1),
      Allocation: pct(t.weight ?? 1), Shares: int(r?.qty), "Position value": num(r?.notional), Costs: num(r?.costs),
      "Scale in": t.legs?.length ? pieceText(t.legs) : "1", "Scale out": t.fills?.length ? pieceText(t.fills) : DASH,
    };
    const row = header.map((h) => cell[h]);
    csv.push([...row, reasonOf(t.reason)]);
    const view: (string | ReactNode)[] = [...row];
    for (const [label, pieces, word] of [["Scale in", t.legs, "legs"], ["Scale out", t.fills, "scale-outs"]] as const) {
      if (pieces?.length) view[header.indexOf(label)] = <details><summary>{pieces.length} {word}</summary>{pieceText(pieces)}</details>;
    }
    rows.push(view);
    const tone: Record<string, number | null | undefined> = { "% Change": t.ret, "Net profit": net, "DD %": dd < 0 ? dd : null };
    cls.push(header.map((h) => (h in tone ? sign(tone[h]) : NUMERIC.has(h) ? "n" : CENTRED.has(h) ? "c" : undefined)));
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

/** `chartsOnly`: the Stats tab's top, the headline return and risk tiles and the two charts; the trade counts are in the
 *  statistics cards below it (which also count a trade still open). */
export function RunResults({ result, market, chartsOnly = false }: { result: RunResult; market: SavedMarket; chartsOnly?: boolean }) {
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
    // both charts: hover dates DD-MM-YYYY
    const day = (t: unknown) => typeof t === "string" ? dayFirst(t) : t && typeof t === "object" && "year" in t && "month" in t && "day" in t
      ? `${String((t as { day: number }).day).padStart(2, "0")}-${String((t as { month: number }).month).padStart(2, "0")}-${(t as { year: number }).year}` : "";
    const eq = createLabChart(el, 300, { timeScale: { minBarSpacing: 0.01, borderColor: themeColors(el).steel }, rightPriceScale: { mode: 1, borderColor: themeColors(el).steel }, localization: { timeFormatter: day } });
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
        if (equityValueRef.current) equityValueRef.current.textContent = i === undefined ? `Latest ${num(result.equity.at(-1))}×` : `${dayFirst(result.dates[i])} · ${num(result.equity[i])}×`;
      };
      chart.subscribeCrosshairMove?.(onCross);
      offCross = () => chart.unsubscribeCrosshairMove?.(onCross);
    }
    // Underwater: equity ÷ its running peak − 1, on the same chart engine as Portfolio Equity (DD-MM-YYYY hover inside
    // the chart, the deepest point marked), shaded from 0 % down to the line
    const values = drawdown(result.equity);
    let dd: ReturnType<typeof createLabChart> = null, offDd: (() => void) | undefined;
    ddEl.classList.remove("lab-chart-empty");
    if (!values.length || values.length !== result.dates.length || values.some((value) => !Number.isFinite(value))) {
      ddEl.classList.add("lab-chart-empty"); ddEl.textContent = "No drawdown data for this period.";
    } else if (values.length === 1) {
      ddEl.classList.add("lab-chart-empty"); ddEl.textContent = `Drawdown ${pct(values[0], 2)} · ${dayFirst(result.dates[0])}`;
    } else {
      dd = createLabChart(ddEl, 300, { timeScale: { minBarSpacing: 0.01, borderColor: themeColors(ddEl).steel }, rightPriceScale: { borderColor: themeColors(ddEl).steel, scaleMargins: { top: 0, bottom: 0.1 } }, // 0 % (the peak) is the top edge
        localization: { timeFormatter: day, priceFormatter: (v: number) => pct(v, 1) } });
      if (dd) {
        const { chart, colors } = dd;
        const area = chart.addAreaSeries({ lineColor: colors.red, topColor: colors.red + "22", bottomColor: colors.red + "66", invertFilledArea: true, lineWidth: 1, priceLineVisible: false });
        area.setData(linePoints(result.dates, values));
        const worst = values.reduce((k, v, i) => (v < values[k] ? i : k), 0);
        area.setMarkers?.([{ time: result.dates[worst], position: "belowBar", shape: "circle", color: colors.red, text: `Worst ${pct(values[worst], 2)} · ${dayFirst(result.dates[worst])}` }]);
        chart.timeScale().fitContent();
        const at = new Map(result.dates.map((date, i) => [date, i]));
        const onDd = ({ time }: { time?: string }) => {
          const i = typeof time === "string" ? at.get(time) : undefined;
          if (ddValueRef.current) ddValueRef.current.textContent = i === undefined ? `Latest ${pct(values.at(-1))}` : `${dayFirst(result.dates[i])} · ${pct(values[i], 2)}`;
        };
        chart.subscribeCrosshairMove?.(onDd);
        offDd = () => chart.unsubscribeCrosshairMove?.(onDd);
      }
    }
    return () => { offCross?.(); offDd?.(); eq?.dispose(); dd?.dispose(); ddEl.replaceChildren(); };
  }, [result, themeKey]);

  return (
    <section className="box lab-result" data-lab-result tabIndex={-1}>
      <h2 className="box-h">Backtest <small>{dayFirst(result.from)} → {dayFirst(result.to)} · {instrument.name ?? instrument.ticker}</small></h2>
      {isProvisional(market) && <ProvisionalStrip note={result.provisional_note} instrumentLabel={instrument.label} />}
      {result.start && <p className="rnote">{COPY.startMoved(result.start.date)}</p>}

      {/* the Stats tab drops the run's labels (the page header keeps "simulated · not a book method"); elsewhere they stay */}
      {!chartsOnly && <><ServerLabel text={result.label} /><ExecutionStrip model={result.execution_model} money={result.money} adaptations={result.adaptations} /></>}
      {chartsOnly && <MetricTiles items={kpiItems(stats).filter((k) => !/#|Avg\./.test(k.label))} />}
      {!chartsOnly && <>
      <MetricTiles items={[...(result.money ? [{ label: "Final equity", value: num(result.money.final_equity) }, { label: "Costs paid", value: num(result.money.costs_total) }] : []), ...kpiItems(stats)]} />
      <p className="cap">Exposure {pct(stats.exposure)} · before costs: {pct(stats.gross_total_return)}</p>
      {result.money ? <><p className="cap">{result.money.basis}</p>{result.money.skipped > 0 && <p className="cap">{result.money.skipped} trade{result.money.skipped === 1 ? "" : "s"} skipped ({result.money.limits
        ? Object.entries(result.money.limits.skip_reasons).map(([k, n]) => `${k.replaceAll("_", " ")}: ${n}`).join(" · ") : "size rounded to 0"})</p>}
        {result.money.limits && <p className="cap" data-lab-limits-basis>Interest on idle cash: {result.money.limits.interest.toLocaleString("en-US", { maximumFractionDigits: 2 })}.{result.money.limits.margin_interest !== undefined && <> Margin loan interest paid: <span data-lab-margin-interest>{result.money.limits.margin_interest.toLocaleString("en-US", { maximumFractionDigits: 2 })}</span>.</>} {result.money.limits.basis}</p>}</> : <p className="cap">Fees {costs.fees_bps} bps + slippage {costs.slippage_bps} bps per side · taxes not modelled</p>}
      </>}

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

/** YOY-Monthly pivot totals, Excel-style sums: each year across its months, each month across the years, and all of them.
 *  Blank months add nothing; a row or column with no month at all stays blank. */
export function monthPivot(years: string[], months: Record<string, number | null | undefined>) {
  const add = (vs: (number | null | undefined)[]) => { const n = vs.filter((v): v is number => typeof v === "number" && Number.isFinite(v)); return n.length ? n.reduce((a, b) => a + b, 0) : undefined; };
  const cell = (y: string, i: number) => months[`${y}-${String(i + 1).padStart(2, "0")}`];
  return { rows: Object.fromEntries(years.map((y) => [y, add(MONTHS.map((_, i) => cell(y, i)))])) as Record<string, number | undefined>,
    cols: MONTHS.map((_, i) => add(years.map((y) => cell(y, i)))), all: add(years.flatMap((y) => MONTHS.map((_, i) => cell(y, i)))) };
}

/** AmiBroker Profit Table and the tradesheet (result list): a full-width row under the rules/backtest pair. */
/** `part` shows only the Profit Table ("monthly") or only the Tradesheet and ledger ("trades"); both when absent. */
export function RunTables({ result, part, explain }: { result: RunResult; part?: "monthly" | "trades";
  /** The test behind the result: exit reasons name the rule that closed each trade; the Excel download adds a Rules sheet. */
  explain?: RunContext }) {
  const { instrument, trades } = result;
  const [order, setOrder] = useState<"oldest" | "newest">("oldest");
  const [page, setPage] = useState(0);
  const [tsSlot, setTsSlot] = useState<HTMLSpanElement | null>(null); // Download TS renders here, beside the order toggle
  useEffect(() => setPage(0), [result]); // a new backtest starts on its first page
  const report = window.VaultReport;
  const monthly = report && result.equity.length ? report.monthly({ dates: result.dates, equity: result.equity.map((v) => v / result.equity[0]) }) : null;
  const pivot = monthly ? monthPivot(monthly.years, monthly.months) : { rows: {}, cols: [], all: undefined };
  // the Avg row's Total: the average of the yearly Totals (sums, like the column above it)
  const totals = monthly ? monthly.years.map((y) => pivot.rows[y]).filter((v): v is number => typeof v === "number") : [];
  const avgTotal = totals.length ? totals.reduce((a, b) => a + b, 0) / totals.length : null;
  const table = tradeTable(result, explain ? reasonLabeler(explain) : undefined), pages = Math.max(1, Math.ceil(table.rows.length / PAGE)), at = Math.min(page, pages - 1);
  const shown = (order === "oldest" ? table.rows.map((_, k) => k) : table.rows.map((_, k) => table.rows.length - 1 - k)).slice(at * PAGE, (at + 1) * PAGE);


  return (
    <section className="box lab-result-tables" data-lab-result-tables>
      {part !== "trades" && monthly && (
        <div className="tblwrap lab-monthly">
          <table className="tbl">
            <caption>YOY-Monthly</caption>
            <thead><tr><th>Year</th>{MONTHS.map((m) => <th className="n" key={m}>{m}</th>)}<th className="n" title="Sum of the year's monthly returns">Total</th></tr></thead>
            <tbody>
              {monthly.years.map((y) => (
                <tr key={y}>
                  <td>{y}</td>
                  {MONTHS.map((_, i) => {
                    const v = monthly.months[`${y}-${String(i + 1).padStart(2, "0")}`];
                    return <td className={sign(v)} key={i}>{ret(v)}</td>;
                  })}
                  <td className={sign(pivot.rows[y])}>{ret(pivot.rows[y])}</td>
                </tr>
              ))}
            </tbody>
            <tfoot>
              <tr data-lab-pivot-total><td>Total</td>{pivot.cols.map((v, i) => <td className={sign(v)} key={i}>{ret(v)}</td>)}
                <td className={sign(pivot.all)}>{ret(pivot.all)}</td></tr>
              {!!monthly.averages?.length && <tr><td>Avg</td>{MONTHS.map((_, i) => <td className={sign(monthly.averages[i])} key={i}>{pct(monthly.averages[i])}</td>)}
                <td className={sign(avgTotal)}>{pct(avgTotal)}</td></tr>}
            </tfoot>
          </table>
        </div>
      )}

      {part !== "monthly" && <>
      <section className="lab-trades">
        <div className="lab-row">
          <h3 className="box-h">Tradesheet <small>{trades.length} trades</small></h3>
          {trades.length > 1 && <Segmented label="Trade order" value={order} onChange={(v) => { setOrder(v); setPage(0); }}
            options={[{ value: "oldest", label: "Oldest first" }, { value: "newest", label: "Newest first" }]} />}
          {trades.length > 0 && explain && <span className="lab-ts-slot" ref={setTsSlot} />}
        </div>
        {trades.length > 0 && explain && <TsDownload result={result} explain={explain} buttonSlot={tsSlot} />}
        {trades.length ? (
          <>
            <div className="tlog">
              <table className="tbl tl" data-lab-trades>
                <thead><tr>{table.header.map((h) => <th key={h} className={NUMERIC.has(h) ? "n" : CENTRED.has(h) ? "c" : undefined}>{h}</th>)}</tr></thead>
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

/** A run with 0 trades: name the buy rules whose two sides never meet on this instrument (different scales, a level out of range). */
export function NeverMeetsNote({ spec, catalogue, market, ticker }: { spec: Spec; catalogue: Catalogue; market: SavedMarket; ticker: string }) {
  const [notes, setNotes] = useState<string[]>([]);
  useEffect(() => {
    let live = true;
    setNotes([]);
    neverMeets(spec, catalogue, market, ticker).then((n) => live && setNotes(n), () => undefined);
    return () => { live = false; };
  }, [spec, catalogue, market, ticker]);
  return notes.length ? <div className="rnote" role="note" data-lab-never-meets>{notes.map((n) => <p key={n}>{n}</p>)}</div> : null;
}
