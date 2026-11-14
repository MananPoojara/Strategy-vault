// AmiBroker-style backtest report (the Indicators lab's Report.tsx, branch Nandini), for YT runs: the same cards, charts and
// download; the Settings fold lists the YT strategy's settings instead of the lab's rule spec.
import { useState, type ReactNode } from "react";
import { DASH, int, num, pct } from "./format";
import type { LabReport, MoneySummary, ReportGroup, ReportSide, Trade, YtRunResult } from "./types";
import { Segmented } from "./ui";
import { saveBytes, xlsx, XLSX_TYPE } from "./xlsx";
import "./report.css";

export const dayFirst = (iso: string) => (/^\d{4}-\d{2}-\d{2}$/.test(iso) ? `${iso.slice(8, 10)}-${iso.slice(5, 7)}-${iso.slice(0, 4)}` : iso);


/** What the report needs from the run beyond the report itself (optional: the report still renders without it). */
export type ReportRun = Pick<YtRunResult, "from" | "to"> & { dates?: string[]; instrument?: Pick<YtRunResult["instrument"], "ticker">; money?: Pick<MoneySummary, "capital" | "final_equity" | "costs_total"> };

type Col = "all" | "long" | "short" | "bh";
const COLS: [Col, string][] = [["all", "All trades"], ["long", "Long trades"], ["short", "Short trades"], ["bh", "Buy&Hold"]];
type Line = [label: string, cell: (c: Col) => string];
type Block = { key: string; head?: Line; rows: Line[] };

/** Rows coloured by sign as in AmiBroker's report: profit, loss, return and drawdown figures (green gain, red loss). */
const SIGNED = /Profit|Loss|Return|drawdown|Largest|Recovery|CAR\/MaxDD|RAR\/MaxDD/;
const tone = (v: string) => (/^-\d|^−\d/.test(v) ? " neg" : /^[1-9]|^0\.\d*[1-9]/.test(v) ? " pos" : "");

/** Trade-based rows have no buy-and-hold value. */
const NO_BH = new Set(["max_trade_dd", "sharpe_trades"]);

/**
 * One AmiBroker statistics table (All | Long | Short | Buy&Hold): the capital block, trades, winners, losers, then the
 * system rows. The engine computes system metrics on the whole equity line only, so Long/Short show "—" there.
 */
const BLOCK_TITLES: Record<string, string> = { capital: "Returns", trades: "Trades", winners: "Winners", losers: "Losers", system: "Risk & ratios" };

export function reportBlocks(report: LabReport, run?: ReportRun): Block[] {
  const bh = report.buy_hold, money = run?.money, start = money?.capital ?? 1, cash = (v: number | null | undefined) => num(v, money ? 2 : 3);
  const sys = (key: string, fmt: (v: number | null | undefined) => string): Line[1] => (c) =>
    c === "all" ? fmt(report.system[key]) : c === "bh" && bh && !NO_BH.has(key) ? fmt(bh[key]) : DASH;
  // Buy&Hold per-side cells come from report.buyhold (one buy-and-hold trade); older responses have none.
  const side = (fmt: (s: ReportSide) => string): Line[1] => (c) => { const s = c === "bh" ? report.buyhold : report[c]; return s ? fmt(s) : DASH; };
  const group = (g: "winners" | "losers", fmt: (x: ReportGroup) => string) => side((s) => (s[g] ? fmt(s[g]) : DASH));
  const net = (c: Col) => (c === "all" ? (money ? money.final_equity - start : typeof report.system.net_profit === "number" ? start * report.system.net_profit : null)
    : c === "bh" && typeof bh?.net_profit === "number" ? start * bh.net_profit : null);
  const p2 = (v: unknown) => pct(v, 2);
  const groupRows = (g: "winners" | "losers", word: string, largest: string): Block => ({
    key: g, head: [g === "winners" ? "Winners" : "Losers", group(g, (x) => `${int(x.count)} (${pct(x.pct, 2)})`)],
    rows: [[`Total ${word} (× initial equity)`, group(g, (x) => num(x.profit, 3))], ["Sum of trade %", group(g, (x) => pct(x.total, 1))],
      [`Avg. ${word} %`, group(g, (x) => p2(x.avg))], ["Avg. Bars Held", group(g, (x) => num(x.avg_bars, 1))], ["Max. Consecutive", group(g, (x) => int(x.max_consecutive))],
      [`Largest ${largest} %`, group(g, (x) => p2(x.largest))], [`# bars in largest ${largest}`, group(g, (x) => int(x.bars_in_largest))]],
  });
  return [
    { key: "capital", rows: [
      ["Initial capital", () => cash(start)],
      ["Ending capital", (c) => { const n = net(c); return n === null ? DASH : cash(start + n); }],
      ["Net Profit", (c) => cash(net(c))],
      ["Net Profit %", sys("net_profit", p2)], ["Exposure %", sys("exposure", p2)], ["Net Risk Adjusted Return %", sys("net_rar", p2)],
      ["Annual Return %", sys("cagr", p2)], ["Risk Adjusted Return %", sys("rar", p2)],
      ["Transaction costs", (c) => (c === "all" && money ? cash(money.costs_total) : DASH)],
    ] },
    { key: "trades", head: ["All trades", side((s) => int(s.trades))], rows: [["Avg. Profit/Loss %", side((s) => p2(s.avg))], ["Avg. Bars Held", side((s) => num(s.avg_bars, 1))]] },
    groupRows("winners", "Profit", "win"),
    groupRows("losers", "Loss", "loss"),
    { key: "system", rows: [
      ["Max. trade % drawdown", sys("max_trade_dd", p2)], ["Max. system drawdown (× initial equity)", sys("max_sys_dd_abs", (v) => num(v, 3))],
      ["Max. system % drawdown", sys("max_sys_dd", p2)], ["Recovery Factor", sys("recovery_factor", num)], ["CAR/MaxDD", sys("car_mdd", num)],
      ["RAR/MaxDD", sys("rar_mdd", num)], ["Profit Factor", side((s) => num(s.profit_factor))], ["Payoff Ratio", side((s) => num(s.payoff))],
      ["Standard Error (× initial equity)", sys("std_error", (v) => num(v, 3))], ["Risk-Reward Ratio", sys("rrr", num)], ["Ulcer Index", sys("ulcer_index", num)],
      ["Ulcer Performance Index", sys("upi", num)], ["Sharpe Ratio of trades", sys("sharpe_trades", num)], ["K-Ratio", sys("k_ratio", (v) => num(v, 3))],
    ] },
  ];
}

/** Trade % returns bucketed by `step` (AmiBroker "distribution charts spacing", default 5 %). */
export function distribution(rets: number[], step = 0.05): { from: number; count: number }[] {
  if (!rets.length) return [];
  const lo = Math.floor(Math.min(...rets) / step), hi = Math.floor(Math.max(...rets) / step);
  const out = Array.from({ length: hi - lo + 1 }, (_, i) => ({ from: (lo + i) * step, count: 0 }));
  for (const r of rets) out[Math.floor(r / step) - lo].count++;
  return out;
}

/** Shared chart frame: caption, y-range labels on the left, x labels (start · title · end) under the plot. */
export function ChartFrame({ caption, sub, label, y, x, data, children }: { caption: string; sub?: string; label: string; y: [string, string]; x: [string, string, string]; data: Record<string, string>; children: ReactNode }) {
  return (
    <figure className="lab-report-chart" {...data}>
      <figcaption>{caption}{sub && <small>{sub}</small>}</figcaption>
      <div className="lab-plot">
        <div className="lab-plot-y"><span>{y[0]}</span><span>{y[1]}</span></div>
        <svg viewBox="0 0 100 60" preserveAspectRatio="none" role="img" aria-label={label}>{children}</svg>
        <span />
        <div className="lab-report-axis"><span>{x[0]}</span><span>{x[1]}</span><span>{x[2]}</span></div>
      </div>
    </figure>
  );
}

function Histogram({ values, caption, axis, data }: { values: number[]; caption: string; axis: string; data: Record<string, string> }) {
  const bins = distribution(values);
  if (!bins.length) return null;
  const max = Math.max(1, ...bins.map((b) => b.count)), w = 100 / bins.length;
  return (
    <ChartFrame caption={caption} sub="trades per 5 % bucket" label={`${caption} chart`} data={data} y={[int(max), "0"]}
      x={[pct(bins[0].from, 0), axis, pct(bins[bins.length - 1].from + 0.05, 0)]}>
      {bins.map((b, i) => <rect key={i} x={i * w + w * 0.1} width={w * 0.8} y={57 - (b.count / max) * 54} height={(b.count / max) * 54}
        className={b.from >= 0 ? "pos" : "neg"}><title>{`${pct(b.from, 0)} to ${pct(b.from + 0.05, 0)}: ${b.count} trades`}</title></rect>)}
      <line x1="0" x2="100" y1="57" y2="57" className="axis" />
    </ChartFrame>
  );
}

/** MAE (or MFE) against the trade's own return, winners green and losers red (AmiBroker MAE/MFE distribution). */
function Excursions({ trades, field, title }: { trades: Trade[]; field: "mae" | "mfe"; title: string }) {
  const pts = trades.flatMap((t) => (typeof t[field] === "number" && typeof t.ret === "number" ? [{ x: t[field] as number, y: t.ret }] : []));
  if (!pts.length) return null;
  const xs = pts.map((p) => p.x), ys = pts.map((p) => p.y);
  const [x0, x1] = [Math.min(0, ...xs), Math.max(0, ...xs)], [y0, y1] = [Math.min(0, ...ys), Math.max(0, ...ys)];
  // inset so no point sits on the frame
  const sx = (x: number) => 6 + ((x - x0) / (x1 - x0 || 1)) * 88, sy = (y: number) => 54 - ((y - y0) / (y1 - y0 || 1)) * 48;
  return (
    <ChartFrame caption={title} sub="y: trade %" label={`${title} chart`} data={{ "data-lab-excursion": field }} y={[pct(y1, 0), pct(y0, 0)]} x={[pct(x0, 0), `${field.toUpperCase()} %`, pct(x1, 0)]}>
      <line x1={sx(0)} x2={sx(0)} y1="2" y2="58" className="axis" /><line x1="2" x2="98" y1={sy(0)} y2={sy(0)} className="axis" />
      {/* a zero-length round-capped line stays a round dot under the stretched viewBox */}
      {pts.map((p, i) => <line key={i} x1={sx(p.x)} x2={sx(p.x)} y1={sy(p.y)} y2={sy(p.y)} className={`dot ${p.y > 0 ? "pos" : "neg"}`} />)}
    </ChartFrame>
  );
}

/** AmiBroker backtest report: one statistics table, distributions and the declared conventions. */
/** The report as a workbook: a Statistics sheet with every card (same columns as on screen) and the trade list. */
export function reportSheets(report: LabReport, run: ReportRun | undefined, tradeRows?: string[][]) {
  const cols = report.buy_hold || report.buyhold ? COLS : COLS.filter(([c]) => c !== "bh");
  const head = cols.map(([k, label]) => (k === "bh" && run?.instrument ? `${label} (${run.instrument.ticker})` : label));
  const stats = reportBlocks(report, run).flatMap(({ key, head: h, rows }) => [[BLOCK_TITLES[key] ?? key], ["", ...head],
    ...(h ? [[h[0], ...cols.map(([c]) => h[1](c))]] : []), ...rows.map(([label, cell]) => [label, ...cols.map(([c]) => cell(c))]), []]);
  return [{ name: "Statistics", layout: "blocks" as const, rows: stats }, ...(tradeRows?.length ? [{ name: "Trades", rows: tradeRows }] : [])];
}

/** `tradeRows`: the trade list (header first) for the download's Trades sheet. */
export function ReportPanel({ report, trades, settings, run, tradeRows }: { report: LabReport; trades: Trade[]; settings?: [string, string][]; run?: ReportRun; tradeRows?: string[][] }) {
  const [mode, setMode] = useState<"scatter" | "histogram">("scatter");
  const excursion = (field: "mae" | "mfe", title: string) => mode === "scatter" ? <Excursions trades={trades} field={field} title={title} />
    : <Histogram values={trades.flatMap((t) => (typeof t[field] === "number" ? [t[field] as number] : []))} caption={`${title} distribution`} axis={`${field.toUpperCase()} %`} data={{ "data-lab-excursion": field }} />;
  // the Buy&Hold column only when the engine sent buy-and-hold figures (older responses may have none)
  const cols = report.buy_hold || report.buyhold ? COLS : COLS.filter(([c]) => c !== "bh");
  return (
    <section className="box lab-report" data-lab-report>
      <h3 className="box-h">Report <small>AmiBroker-style statistics · simulated · not a book method</small>
        <button type="button" className="dbtn sm primary lab-report-download" data-lab-download-report onClick={() => saveBytes(xlsx(reportSheets(report, run, tradeRows)),
          `${run?.instrument?.ticker ?? "backtest"} report ${run ? `${dayFirst(run.from)} to ${dayFirst(run.to)}` : ""}.xlsx`.replace(/\s+\.xlsx$/, ".xlsx"), XLSX_TYPE)}>Download report</button></h3>
      {/* one titled card per AmiBroker block, flowing in columns across the width (no tall table beside an empty half page) */}
      <div className="lab-report-cards" data-lab-report-stats>
        {reportBlocks(report, run).map(({ key, head, rows }) => (
          <table className="tbl lab-report-card" key={key} {...(key === "system" ? { "data-lab-report-system": "" } : {})}>
            <caption>{BLOCK_TITLES[key] ?? key}</caption>
            <thead><tr><th><span className="sr-only">Statistics</span></th>{cols.map(([k, label]) => <th className="n" key={k}>{k === "bh" && run?.instrument ? `${label} (${run.instrument.ticker})` : label}</th>)}</tr></thead>
            <tbody className={head ? "lab-report-group" : undefined}>
              {head && <tr className="lab-report-block"><td>{head[0]}</td>{cols.map(([c]) => <td className="n" key={c}>{head[1](c)}</td>)}</tr>}
              {rows.map(([label, cell]) => <tr key={label}><td>{label}</td>{cols.map(([c]) => { const v = cell(c); return <td className={`n${SIGNED.test(label) ? tone(v) : ""}`} key={c}>{v}</td>; })}</tr>)}
            </tbody>
          </table>
        ))}
      </div>
      {trades.some((t) => typeof t.mae === "number") && <Segmented<"scatter" | "histogram"> label="MAE/MFE chart" value={mode} onChange={setMode}
        options={[{ value: "scatter", label: "MAE/MFE vs trade %" }, { value: "histogram", label: "MAE/MFE distribution" }]} />}
      <div className="lab-report-charts">
        <Histogram values={trades.flatMap((t) => (typeof t.ret === "number" ? [t.ret] : []))} caption="Profit distribution" axis="% Profit" data={{ "data-lab-profit-distribution": "" }} />
        {excursion("mae", "Max. Adverse Excursion")}
        {excursion("mfe", "Max. Favourable Excursion")}
      </div>
      {!!settings?.length && <details className="lab-report-settings" data-lab-report-settings>
        <summary>Settings</summary>
        <table className="tbl"><tbody>{[...(run ? [["Range", `${dayFirst(run.from)} → ${dayFirst(run.to)}`] as [string, string]] : []), ["Periodicity", "Daily"] as [string, string], ...settings].map(([k, v]) => <tr key={k}><td className="lab-report-key">{k}</td><td>{v}</td></tr>)}</tbody></table>
      </details>}
      <details className="lab-report-how" data-lab-report-how><summary>How these numbers are calculated</summary>
        <p className="cap lab-report-constants" data-lab-report-constants>
        Risk-free rate {report.rates?.sharpe ?? 5} % (Sharpe of trades) · {report.rates?.upi ?? 5.4} % (Ulcer performance index) · distribution buckets 5 % · Buy&amp;Hold: {run?.instrument?.ticker ?? "the same instrument"}{report.buyhold ? " (trade columns: one buy-and-hold trade over the run)" : ""}
      </p>
        <ul className="rnote lab-report-notes">{report.notes.map((n) => <li key={n}>{n}</li>)}</ul></details>
    </section>
  );
}
