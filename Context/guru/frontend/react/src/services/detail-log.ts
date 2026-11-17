/* Detail log of an AmiBroker-style portfolio run: one block per day on which something happened, in two columns
   (Date | Information), laid out like the Detailed Log of AmiBroker's backtester: the date, the day's exits and
   entries as one line each, the day's closing account, a blank row. Built only from the run's own trade list and
   daily account series; no trading rule is evaluated here. */

export interface LogTrade {
  symbol: string; dir: number; shares?: number | null; entry: string; entry_px: number; exit: string; exit_px: number;
  /** Intraday runs: "YYYY-MM-DD hh:mm" candle stamps. */
  entry_time?: string; exit_time?: string;
  open?: boolean; reason?: string; bars?: number; entry_value?: number; fees?: number; pnl?: number; ret?: number; shrunk?: boolean;
}
export interface LogRun {
  dates?: string[]; equity_money?: (number | null)[]; cash?: (number | null)[]; positions?: (number | null)[];
  /** Portfolio runs: every signal of the AFL trade lists, taken or not, as the engine sends it:
   *  [symbol, direction, entry date index, exit date index (-1 while open), 1 if the portfolio took the trade]. */
  signals?: [string, number, number, number, number][];
  /** `brokerage_pct` given (portfolio runs): commissions are split into entry and exit; without it only a trade's total costs are shown. */
  trades?: LogTrade[]; portfolio?: { brokerage_pct?: number };
}
/** [ISO date or "", text]; the first row is the header. */
export type LogRow = [string, string];

const num = (v: number | null | undefined, digits = 2) =>
  v == null || !Number.isFinite(v) ? "–" : String(Number(v.toFixed(digits)));
const side = (t: LogTrade) => (t.dir < 0 ? "Short" : "Long");
const bySymbol = (a: LogTrade, b: LogTrade) => String(a.symbol).localeCompare(String(b.symbol));

export function detailLogRows(run: LogRun): LogRow[] {
  const rows: LogRow[] = [["Date", "Information"]];
  const trades = run.trades || [], split = typeof run.portfolio?.brokerage_pct === "number", rate = (run.portfolio?.brokerage_pct || 0) / 100;
  const entries = new Map<string, LogTrade[]>(), exits = new Map<string, LogTrade[]>();
  const push = (m: Map<string, LogTrade[]>, day: string, t: LogTrade) => { if (day) m.set(day, [...(m.get(day) || []), t]); };
  for (const t of trades) { push(entries, t.entry, t); if (!t.open) push(exits, t.exit, t); }
  const index = new Map((run.dates || []).map((day, i) => [day, i]));
  // Signals of a day, as AmiBroker's Detailed Log lists them: every Buy / Short signal with its score, and every
  // Sell / Cover signal of a symbol whose signal state is "in", whether or not the portfolio took the trade. A
  // single-instrument run has no rejected signals: its trades are its signals. No PositionScore here: score 1.
  const sigIn = new Map<string, string[]>(), sigOut = new Map<string, string[]>();
  const note = (m: Map<string, string[]>, day: string | undefined, text: string) => { if (day) m.set(day, [...(m.get(day) || []), text]); };
  if (run.signals) {
    for (const [symbol, dir, e, x] of run.signals) {
      note(sigIn, run.dates?.[e], `${symbol}=${dir < 0 ? "Short" : "Buy"}(1)`);
      if (x >= 0) note(sigOut, run.dates?.[x], `${symbol}=${dir < 0 ? "Cover" : "Sell"}`);
    }
  } else {
    for (const t of trades) {
      note(sigIn, t.entry, `${t.symbol}=${t.dir < 0 ? "Short" : "Buy"}(1)`);
      if (!t.open) note(sigOut, t.exit, `${t.symbol}=${t.dir < 0 ? "Cover" : "Sell"}`);
    }
  }
  const days = [...new Set([...entries.keys(), ...exits.keys(), ...sigIn.keys(), ...sigOut.keys()])].sort();
  const list = (m: Map<string, string[]>, day: string) => [...(m.get(day) || [])].sort((p, q) => p.localeCompare(q)).map(x => `${x}, `).join("");
  // positions held at a day's close, for the account line: taken trades entered on or before it and not yet exited
  const held = (day: string) => trades.filter(t => t.entry <= day && (t.open || t.exit > day)).sort(bySymbol);
  // commission on entry = entry value x brokerage %; the trade's total fees minus that is the exit commission
  const entryFee = (t: LogTrade) => (t.entry_value == null ? null : t.entry_value * rate);
  const has = (v: unknown) => typeof v === "number" && Number.isFinite(v);
  const time = (stamp?: string) => (stamp && stamp.includes(" ") ? `, Time: ${stamp.split(" ")[1]}` : "");
  const sized = (t: LogTrade) => (has(t.shares) ? `, Shares: ${num(t.shares, 0)}` : "");
  const profit = (t: LogTrade) => (has(t.pnl) ? `${num(t.pnl)} (${num((t.ret ?? NaN) * 100)} %)` : `${num((t.ret ?? NaN) * 100)} %`);
  for (const day of days) {
    const out = [...(exits.get(day) || [])].sort(bySymbol), inn = [...(entries.get(day) || [])].sort(bySymbol);
    rows.push([day, ""]);
    rows.push(["", `Entry signals(score):${list(sigIn, day)}`]);
    rows.push(["", `Exit signals:${list(sigOut, day)}`]);
    for (const t of out) {
      const fee = entryFee(t), exitFee = fee == null || t.fees == null ? null : t.fees - fee;
      const costs = split ? `, Commission: ${num(exitFee)}, (Total comm.: ${num(t.fees)})` : has(t.fees) ? `, Costs: ${num(t.fees)}` : "";
      rows.push(["", `Exit ${side(t)}, ${t.symbol}, Price: ${num(t.exit_px)}${time(t.exit_time)}${sized(t)}${costs}, `
        + `Profit: ${profit(t)}, Bars: ${num(t.bars, 0)}, Reason: ${t.reason || "–"}`]);
    }
    for (const t of inn)
      rows.push(["", `Enter ${side(t)}, ${t.symbol}, Price: ${num(t.entry_px)}${time(t.entry_time)}${sized(t)}${has(t.entry_value) ? `, Position value: ${num(t.entry_value)}` : ""}`
        + `${split ? `, Commission: ${num(entryFee(t))}` : ""}${t.shrunk ? ", Shrunk to available cash" : ""}`]);
    const i = index.get(day);
    if (i !== undefined) {
      const open = held(day), eq = run.equity_money?.[i], cash = run.cash?.[i];
      const holdings = open.filter(t => has(t.shares)).map(t => `${t.symbol} (${t.dir < 0 ? "-" : "+"}${num(t.shares, 0)})`).join(", ");
      const parts = [has(eq) && has(cash) ? `Market Value: ${num((eq as number) - (cash as number))}` : "", has(eq) ? `Equity: ${num(eq)}` : "", has(cash) ? `Cash: ${num(cash)}` : ""].filter(Boolean);
      const count = has(run.positions?.[i]) ? (run.positions?.[i] as number) : open.length;
      rows.push(["", `${count} Open Positions: ${holdings}${holdings && parts.length ? ", " : ""}${parts.join(", ")}`]);
    }
    rows.push(["", ""]);
  }
  const open = trades.filter(t => t.open).sort(bySymbol);
  if (open.length) {
    rows.push([run.dates?.at(-1) || open[0].exit || "", split ? "Open at the end of the backtest (marked at the last close, no exit commission):" : "Open at the end of the backtest (marked at the last close):"]);
    for (const t of open)
      rows.push(["", `Open ${side(t)}, ${t.symbol}, Entered: ${t.entry}, Entry price: ${num(t.entry_px)}, Marked at: ${num(t.exit_px)}${sized(t)}, `
        + `Unrealised profit: ${profit(t)}`]);
    rows.push(["", ""]);
  }
  return rows;
}
