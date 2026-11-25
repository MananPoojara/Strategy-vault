/**
 * The tradesheet download (user request 25 Nov 2026), laid out like the desk's Spot tradesheet (Young Turtle /signals,
 * "Spot Tradesheet Download Sample File.xlsx"): the rules in rows 1-8, an Overall block and a Since 01-01-2011 block of
 * statistics from row 10, MAE / Gain-Pain / MFE under them, the trade list from row 31 with booked cumulative, peak and
 * drawdown columns as formulas. The statistic columns follow the costs of the run: one Profit % column when no cost is
 * charged, else Gross %, Slippage %, Brokerage % and Net %. Every value comes from the run's own trades; nothing is estimated.
 */
export interface SheetTrade {
  dir?: number; entry: string; exit: string; open: boolean; avg_px?: number; exit_px?: number; reason?: string;
  ret?: number; ret_full_precision?: number;
  accounting?: {symbol?: string; days_held?: number; gross_ret?: number; net_ret?: number; fee_ret?: number; slippage_ret?: number;
    mae_ret_observed?: number | null; mfe_ret_observed?: number | null} | null;
}
export interface SheetInput {
  strategy: string; book?: string; symbol: string; currency: string;
  rules: {setup: string[]; entry: string[]; scaleIn: string[]; exit: string[]};
  period: {from: string; to: string};
  /** As the run used them. */
  costs: {brokerageBps: number; slippageBps: number};
  filters: {deadStock: boolean; penny: number};
  account: {equity: number; compound: boolean; weight: number};
  trades: SheetTrade[];
}
export interface SheetCell {v?: string | number; t?: "s" | "n"; f?: string; z?: string; s?: Record<string, unknown>}
export interface TradesheetSheet {rows: (SheetCell | null)[][]; merges: {s: {r: number; c: number}; e: {r: number; c: number}}[]; cols: {wch: number}[]; autofilter?: string; heights: Record<number, number>}

const fin = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
const sum = (a: number[]) => a.reduce((s, v) => s + v, 0);
const mean = (a: number[]) => a.length ? sum(a) / a.length : null;
const dayNo = (iso: string) => Date.parse(iso.slice(0, 10)) / 864e5;
/** Excel date serial of an ISO date. */
export const serial = (iso: string) => dayNo(iso) + 25569;
const colName = (c: number) => { let s = ""; for (let n = c + 1; n > 0; n = Math.floor((n - 1) / 26)) s = String.fromCharCode(65 + (n - 1) % 26) + s; return s; };

// The template's look: Calibri 11, light-blue headings (Office accent 1, 60% tint), grey statistic rows, red bold-italic title.
const BLUE = "B4C6E7", GREY = "D9D9D9", GREEN = "006100", RED = "C00000";
const font = (o: Record<string, unknown> = {}) => ({name: "Calibri", sz: 11, ...o});
const S = {
  title: {font: font({sz: 14, bold: true, italic: true, color: {rgb: "FF0000"}}), alignment: {horizontal: "center", vertical: "top"}},
  label: {font: font({sz: 11.5, bold: true}), alignment: {horizontal: "center", vertical: "top"}},
  rule: {font: font({bold: true}), alignment: {horizontal: "left", vertical: "top", wrapText: true}},
  head: {font: font({bold: true}), fill: {patternType: "solid", fgColor: {rgb: BLUE}}, alignment: {horizontal: "center", vertical: "center", wrapText: true}},
  block: {font: font({bold: true, sz: 12})},
  statName: {font: font({bold: true}), fill: {patternType: "solid", fgColor: {rgb: GREY}}, alignment: {horizontal: "right"}},
  stat: {font: font({bold: true}), fill: {patternType: "solid", fgColor: {rgb: GREY}}, alignment: {horizontal: "center"}},
  plainName: {font: font({bold: true}), alignment: {horizontal: "right"}},
  plain: {font: font(), alignment: {horizontal: "center"}},
  helperHead: {font: font({bold: true}), alignment: {horizontal: "center", wrapText: true}},
  body: {font: font()},
  note: {font: font({italic: true, sz: 10, color: {rgb: "595959"}}), alignment: {wrapText: true, vertical: "top"}},
};
const signed = (base: Record<string, unknown>, v: number) => ({...base, font: {...(base.font as object), color: {rgb: v > 0 ? GREEN : v < 0 ? RED : "000000"}}});
const PCT = "0.00%", NUM = "0.00", INT = "0", DATE = "dd-mm-yyyy";
const text = (v: string, s: Record<string, unknown> = S.body): SheetCell => ({v, t: "s", s});
const num = (v: number | null | undefined, z = PCT, s: Record<string, unknown> = S.plain, color = false): SheetCell =>
  fin(v) ? {v, t: "n", z, s: color ? signed(s, v) : s} : text("—", s);

/** The figures of one basis (a list of closed-trade returns, oldest exit first) as the site computes them. */
export function basisStats(rets: number[], days: number[], years: number) {
  const n = rets.length, wins = rets.filter(r => r > 0), losses = rets.filter(r => r <= 0);
  const avgWin = mean(wins), avgLoss = mean(losses), wr = n ? wins.length / n : null, al = avgLoss == null ? null : -avgLoss;
  let cum = 0, peak = 0, dd = 0, eq = 1;
  for (const r of rets) { cum += r; peak = Math.max(peak, cum); dd = Math.min(dd, cum - peak); eq *= 1 + r; }
  const cagr = n && years > 0 && eq > 0 ? eq ** (1 / years) - 1 : null, hold = mean(days);
  return {
    total: n ? sum(rets) : null, avg: mean(rets), trades: n, winRatio: wr, winAvg: avgWin, lossAvg: avgLoss,
    best: n ? Math.max(...rets) : null, worst: n ? Math.min(...rets) : null,
    expectancy: wr != null && al ? ((avgWin ?? 0) * wr - (1 - wr) * al) / al : null,
    thickness: hold && n ? (mean(rets)! / hold) * 30 : null,
    riskReward: avgWin != null && al ? avgWin / al : null,
    cagr, dd: n ? dd : null, carMdd: cagr != null && dd < 0 ? cagr / -dd : null,
  };
}

export function tradesheetSheet(input: SheetInput): TradesheetSheet {
  const rows: (SheetCell | null)[][] = [], merges: TradesheetSheet["merges"] = [], heights: Record<number, number> = {};
  const put = (r: number, c: number, cell: SheetCell | null) => { (rows[r] ||= [])[c] = cell; };
  const charged = {slip: input.costs.slippageBps > 0, fee: input.costs.brokerageBps > 0};
  const anyCost = charged.slip || charged.fee;

  // Trades in time order. Per trade: gross, slippage, brokerage and net as fractions of the position (the engine's ledger).
  const all = [...input.trades].sort((a, b) => a.entry.localeCompare(b.entry) || a.exit.localeCompare(b.exit)).map(t => {
    const a = t.accounting || {}, net = fin(t.ret_full_precision) ? t.ret_full_precision : fin(a.net_ret) ? a.net_ret : fin(t.ret) ? t.ret : NaN;
    const fee = fin(a.fee_ret) ? a.fee_ret : 0, slip = fin(a.slippage_ret) ? a.slippage_ret : 0;
    const gross = fin(a.gross_ret) ? a.gross_ret : net + fee + slip;
    const days = fin(a.days_held) ? a.days_held : Math.max(1, dayNo(t.exit) - dayNo(t.entry));
    return {t, net, fee, slip, gross, days, mae: a.mae_ret_observed ?? null, mfe: a.mfe_ret_observed ?? null, symbol: a.symbol || input.symbol};
  });
  const closed = all.filter(x => !x.t.open && Number.isFinite(x.net));

  // Rows 1-8: the rules, one heading per row, its points on their own lines.
  const money = (v: number) => input.currency + Math.round(v).toLocaleString(input.currency === "₹" ? "en-IN" : "en-US");
  const ruleRows: [string, string[]][] = [
    ["Rules:", [input.strategy + (input.book ? ` (${input.book})` : "")]],
    ["Setup:", input.rules.setup],
    ["Entry:", [...input.rules.entry, ...input.rules.scaleIn.map(s => "Scale-in: " + s)]],
    ["Exit:", input.rules.exit],
    ["Filter:", [`${input.symbol}, ${input.period.from} to ${input.period.to}`,
      input.filters.penny > 0 ? `Penny stock price: no trade when the signal-day close is at or below ${input.currency}${input.filters.penny}` : "",
      input.filters.deadStock ? "Dead stock backtest: no entry on the last bar; an open trade is sold at the last close" : ""].filter(Boolean)],
    ["Costs:", [`Slippage ${input.costs.slippageBps} bps per side · Brokerage ${+(input.costs.brokerageBps / 100).toFixed(4)}% per side`]],
    ["Account:", [`Initial equity ${money(input.account.equity)} · ` + (input.account.compound
      ? `${+(input.account.weight * 100).toFixed(4)}% of equity per trade (compounding)`
      : `${money(input.account.weight * input.account.equity)} per trade (fixed amount)`)]],
  ];
  ruleRows.forEach(([label, lines], r) => {
    put(r, 0, text(label, r ? S.label : S.title));
    put(r, 1, text(lines.join("\n") || "—", S.rule));
    merges.push({s: {r, c: 1}, e: {r, c: 12}});
    if (lines.length > 1 || (lines[0]?.length ?? 0) > 160) heights[r] = 15 * lines.reduce((n, l) => n + Math.max(1, Math.ceil(l.length / 160)), 0);
  });

  // Rows 10-29: Overall (from B) and Since 01-01-2011 (from the next free column), the columns following the costs.
  const statCols: [string, "gross" | "slip" | "fee" | "net"][] = anyCost
    ? [["Gross %", "gross"], ...(charged.slip ? [["Slippage %", "slip"]] as [string, "slip"][] : []), ...(charged.fee ? [["Brokerage %", "fee"]] as [string, "fee"][] : []), ["Net %", "net"]]
    : [["Profit %", "net"]];
  const width = statCols.length + 2; // name column, the stat columns, Days
  const NAMES = ["Total Profit %", "Avg %", "Total Trades", "Win Ratio", "Win Avg", "Loss Avg", "Max Winner", "Max Looser", "Expectancy Ratio", "Monthly Thickness", "Risk Reward", "CAGR", "DD", "CAR/MDD"];
  const block = (c0: number, title: string, list: typeof closed) => {
    put(9, c0, text(title, S.block));
    statCols.forEach(([h], k) => put(10, c0 + 1 + k, text(h, S.head)));
    put(10, c0 + width - 1, text("Days", S.head));
    const years = list.length ? (dayNo(list.at(-1)!.t.exit) - dayNo(list[0].t.entry)) / 365.25 : 0, days = list.map(x => x.days);
    NAMES.forEach((name, k) => put(11 + k, c0, text(name, k === NAMES.length - 1 ? {...S.statName, fill: undefined} : S.statName)));
    statCols.forEach(([, key], k) => {
      const c = c0 + 1 + k, rets = list.map(x => x[key]);
      if (key === "slip" || key === "fee") { // a cost: its total and average; the other rows do not apply
        put(11, c, num(rets.length ? sum(rets) : null, PCT, S.stat)); put(12, c, num(mean(rets), PCT, S.stat));
        for (let r = 13; r < 25; r++) put(r, c, text("", S.stat));
        return;
      }
      const s = basisStats(rets, days, years);
      const vals: [number | null, string, boolean][] = [[s.total, PCT, true], [s.avg, PCT, true], [s.trades, INT, false], [s.winRatio, PCT, false], [s.winAvg, PCT, true], [s.lossAvg, PCT, true],
        [s.best, PCT, true], [s.worst, PCT, true], [s.expectancy, NUM, true], [s.thickness, PCT, true], [s.riskReward, NUM, false], [s.cagr, PCT, true], [s.dd, PCT, true], [s.carMdd, NUM, true]];
      vals.forEach(([v, z, color], r) => put(11 + r, c, num(v, z, S.stat, color)));
    });
    const dc = c0 + width - 1, hold = mean(days);
    put(11, dc, num(list.length ? sum(days) : null, INT, S.stat)); put(12, dc, num(hold, "0.0", S.stat)); put(13, dc, num(hold == null ? null : hold / 30.5, "0.0 \"months\"", S.stat));
    for (let r = 14; r < 25; r++) put(r, dc, text("", S.stat));
    // MAE / Gain-Pain / MFE: sum, worst and average of the observed excursions (net returns for Gain / Pain).
    const mae = list.map(x => x.mae).filter(fin), mfe = list.map(x => x.mfe).filter(fin), nets = list.map(x => x.net);
    const pain = -sum(nets.filter(v => v < 0)), gain = sum(nets.filter(v => v > 0));
    [["MAE", 1], ["Gain / Pain", 2], ["MFE", 3]].forEach(([h, k]) => put(25, c0 + (k as number), text(h as string, S.helperHead)));
    [["Sum", sum(mae), sum(mfe)], ["Max", mae.length ? Math.min(...mae) : null, mfe.length ? Math.max(...mfe) : null], ["Avg.", mean(mae), mean(mfe)]].forEach(([name, a, b], k) => {
      put(26 + k, c0, text(name as string, S.plainName)); put(26 + k, c0 + 1, num(mae.length ? a as number : null, PCT)); put(26 + k, c0 + 3, num(mfe.length ? b as number : null, PCT));
    });
    put(26, c0 + 2, num(pain > 0 ? gain / pain : null, NUM));
  };
  block(0, `Overall ${input.period.from} to ${input.period.to}`, closed);
  block(width + 1, "Since 01-01-2011", closed.filter(x => x.t.entry >= "2011-01-01"));

  // Row 31 on: the trade list (oldest first) and the booked cumulative / peak / drawdown columns as formulas.
  const head: [string, number][] = [["Symbol", 14], ["Signal", 8], ["Entry Date", 11], ["Entry Price", 11], ["Exit Date", 11], ["Exit Price", 11],
    [anyCost ? "Gross P&L %" : "P&L %", 11], ["Days Held", 9], ...(charged.slip ? [["Slippage %", 10]] as [string, number][] : []),
    ...(charged.fee ? [["Brokerage %", 11]] as [string, number][] : []), ...(anyCost ? [["Net P&L %", 11]] as [string, number][] : []),
    [`Net P&L (${input.currency})`, 14], [`Equity (${input.currency})`, 15], ["MAE %", 9], ["MFE %", 9], ["Exit Reason", 28]];
  const at = (h: string) => head.findIndex(([x]) => x === h);
  const H0 = 30, gross = colName(at(anyCost ? "Gross P&L %" : "P&L %")), net = colName(anyCost ? at("Net P&L %") : at("P&L %"));
  const helper = head.length + 1, helpers = ["Gross Cum %", "Gross Peak %", "Gross Booked DD %", "", "Net Cum %", "Net Peak %", "Net Booked DD %", "", "Exit Year", "Entry Year"];
  head.forEach(([h], c) => put(H0, c, text(h, S.head)));
  helpers.forEach((h, k) => h && put(H0, helper + k, text(h, S.head)));
  const first = H0 + 2; // the first trade's Excel row
  let equity = input.account.equity, cumG = 0, peakG = 0, cumN = 0, peakN = 0, prev: number | null = null;
  all.forEach((x, i) => {
    const r = H0 + 1 + i, n = r + 1, t = x.t, short = (t.dir ?? 1) < 0;
    const row: SheetCell[] = [];
    const c = (h: string, cell: SheetCell) => { row[at(h)] = cell; };
    c("Symbol", text(x.symbol)); c("Signal", text(short ? "SHORT" : "LONG"));
    c("Entry Date", {v: serial(t.entry), t: "n", z: DATE, s: S.body}); c("Entry Price", num(t.avg_px, NUM, S.body));
    c("Exit Date", t.open ? text("OPEN") : {v: serial(t.exit), t: "n", z: DATE, s: S.body}); c("Exit Price", num(t.exit_px, NUM, S.body));
    c(anyCost ? "Gross P&L %" : "P&L %", num(anyCost ? x.gross : x.net, PCT, S.body, true));
    c("Days Held", t.open ? num(x.days, INT, S.body) : {v: x.days, t: "n", z: INT, s: S.body, f: `MAX(1,E${n}-C${n})`});
    if (charged.slip) c("Slippage %", num(x.slip, PCT, S.body));
    if (charged.fee) c("Brokerage %", num(x.fee, PCT, S.body));
    if (anyCost) c("Net P&L %", {...num(x.net, PCT, S.body, true), f: `${gross}${n}${charged.slip ? `-${colName(at("Slippage %"))}${n}` : ""}${charged.fee ? `-${colName(at("Brokerage %"))}${n}` : ""}`});
    // Money from the account settings: a fixed amount, or the stated share of the equity before the trade.
    if (!t.open && Number.isFinite(x.net)) {
      const pnl = input.account.weight * (input.account.compound ? equity : input.account.equity) * x.net;
      equity += pnl;
      c(`Net P&L (${input.currency})`, num(pnl, "#,##0", S.body, true)); c(`Equity (${input.currency})`, num(equity, "#,##0", S.body));
    } else { c(`Net P&L (${input.currency})`, text("open", S.body)); c(`Equity (${input.currency})`, text("", S.body)); }
    c("MAE %", num(x.mae, PCT, S.body)); c("MFE %", num(x.mfe, PCT, S.body)); c("Exit Reason", text(t.open ? "open (marked at the last close)" : t.reason || ""));
    if (!t.open && Number.isFinite(x.net)) {
      const p = prev ?? 0;
      cumG += x.gross; peakG = Math.max(peakG, cumG); cumN += x.net; peakN = Math.max(peakN, cumN);
      [[0, cumG, peakG, gross], [4, cumN, peakN, net]].forEach(([o, cum, peak, src]) => {
        const k = helper + (o as number), A = colName(k), B = colName(k + 1);
        row[k] = {v: cum as number, t: "n", z: PCT, s: S.body, f: p ? `${A}${p}+${src}${n}` : `${src}${n}`};
        row[k + 1] = {v: peak as number, t: "n", z: PCT, s: S.body, f: p ? `MAX(${B}${p},${A}${n})` : `MAX(0,${A}${n})`};
        row[k + 2] = {...num((cum as number) - (peak as number), PCT, S.body, true), f: `${A}${n}-${B}${n}`};
      });
      prev = n;
    }
    row[helper + 8] = t.open ? text("") : {v: Number(t.exit.slice(0, 4)), t: "n", z: INT, s: S.body, f: `YEAR(E${n})`};
    row[helper + 9] = {v: Number(t.entry.slice(0, 4)), t: "n", z: INT, s: S.body, f: `YEAR(C${n})`};
    rows[r] = row;
  });
  const last = H0 + all.length;
  put(last + 2, 0, text("Statistics use closed trades only; an open trade is listed but not counted. Returns are fractions of each position; Net P&L and Equity apply the account settings. CAGR compounds the closed returns over the first entry to the last exit; DD is the booked drawdown of the added returns.", S.note));
  merges.push({s: {r: last + 2, c: 0}, e: {r: last + 2, c: 12}}); heights[last + 2] = 30;
  const cols = Array.from({length: helper + helpers.length}, (_, c) => ({wch: c < head.length ? head[c][1] : c === head.length ? 3 : 12}));
  cols[0] = {wch: 20};
  return {rows, merges, cols, heights, autofilter: `A${H0 + 1}:${colName(head.length - 1)}${Math.max(H0 + 1, last + 1)}`};
}
