/** The page uses the AFLs' hard-punched dates (hardpunch.ts, regimeSets below). The computed Master 5DP here is kept to
 *  compare with them (05-11-2026: it matches 107 of the 108 hard-punched windows and adds about 20 the AFL leaves out).
 *  Master 5DP generated from index bars, not from typed date lists. The system set of the short-strategy AFLs
 *  (Mahek's hard-punch lists, "Master 5DP (3 systems - QDP+Nov., 2 Tradeloss, 0.11 ADR)") on the date arithmetic of the
 *  PairAlgo dynamic desk (Dharmil, server/src/jobs/blackout5dp.js, backend_date_query.js):
 *  - every month has a 5DP range: the 6th-from-last full trading day (5th when the calendar month has 30 days or fewer)
 *    to the 2nd full trading day of the next month;
 *  - QDP+Nov: months 3, 6, 9, 11, 12 emit their range;
 *  - 2 Trade Loss: on NIFTY 50, NIFTYBANK and NIFTYMIDCAP100 each, a notional long from range start close to range end close;
 *    after two or more losses in a row the next range is emitted (only a gain or flat range resets the count);
 *  - 0.11 ADR: the NIFTY 50 members' advance/decline ratio is 0.11 or lower on a full session in [start, end)
 *    (#Pre Final Folder/5 Dp Stats/AFL/ADR Long 2-11 and 5DP ROC.afl); an ADR-only window starts on that day;
 *  - QTR anchor: when a Midcap QTR date (the month-end of a 3MR NIFTY + 2MR Midcap, 4MR, or quarter-month 2MR Midcap
 *    month) falls in the range, the entry moves to it, except a QDP window whose QTR date is also a Base-1 start or end
 *    (kept at the range start); an ADR-only window whose ADR day comes before that QTR date is dropped.
 *  Measured against Mahek's 125 Nifty windows (2008-03..2026-09) on the lab's own bars: 124 exact, 1 entry on another day
 *  (2019-02), 0 missing, 8 ADR-only windows her list leaves out (no rule found that separates them).
 *  The same bars also give the QTR / quarterly 2MR sets per index (qtrDates); see regimeSets.
 *  Shortcut: a month's range appears once the month has traded out (no forward NSE calendar), so a backtest ending inside
 *  the current 5DP range does not see that range yet; upgrade with the exchange calendar for live use. */
import { series } from "./client";
import { MASTER_5DP_NIFTY, QTR_MIDCAP } from "./hardpunch";
/** A generated no-trade window set: [from, to) pairs, the short is covered on `from` and may re-enter on `to`. */
export interface RegimeSet { name: string; windows: [string, string][] }

export interface Bar { d: string; o: number; h: number; l: number; c: number }
export interface M5dpInput { nifty: Bar[]; bank: Bar[]; midcap: Bar[]; ad: ReadonlyMap<string, number> }
export interface M5dpWindow { from: string; to: string; systems: string[] }
export interface Range { mo: string; start: string; end: string | null; sc?: number; ec?: number }

/** NSE weekday Muhurat sessions: real sessions, not full trading days, so they do not count in the 5DP ordinals
 *  (exchange calendar, as in PairAlgo tradingCalendar.js MUHURAT_WEEKDAY_SESSIONS; weekend sessions are dropped by weekday). */
const MUHURAT_WEEKDAY = new Set(["2011-10-26", "2014-10-23", "2015-11-11", "2018-11-07", "2021-11-04", "2022-10-24", "2024-11-01", "2025-10-21"]);
const QDP = new Set([3, 6, 9, 11, 12]), QTR_MONTHS = new Set([1, 2, 3, 5, 6, 8, 9, 11, 12]), QUARTER = new Set([3, 6, 9, 12]);
export const ADR_MAX = 0.11;

const weekday = (d: string) => new Date(`${d}T00:00:00Z`).getUTCDay();
export const isFullDay = (d: string) => weekday(d) % 6 !== 0 && !MUHURAT_WEEKDAY.has(d);
export const shiftMonth = (ym: string, n: number) => { const t = Number(ym.slice(0, 4)) * 12 + Number(ym.slice(5, 7)) - 1 + n; return `${String(Math.floor(t / 12)).padStart(4, "0")}-${String((t % 12) + 1).padStart(2, "0")}`; };
const daysIn = (ym: string) => new Date(Date.UTC(Number(ym.slice(0, 4)), Number(ym.slice(5, 7)), 0)).getUTCDate();
const month = (ym: string) => Number(ym.slice(5, 7));

interface Candle { mo: string; o: number; h: number; l: number; c: number; last: string }
function monthly(bars: readonly Bar[]): Candle[] {
  const out: Candle[] = [];
  for (const b of bars) {
    const m = out[out.length - 1];
    if (m?.mo === b.d.slice(0, 7)) { m.h = Math.max(m.h, b.h); m.l = Math.min(m.l, b.l); m.c = b.c; m.last = b.d; }
    else out.push({ mo: b.d.slice(0, 7), o: b.o, h: b.h, l: b.l, c: b.c, last: b.d });
  }
  return out;
}
const red = (by: ReadonlyMap<string, Candle>, mo: string, n: number) => Array.from({ length: n }, (_, k) => by.get(shiftMonth(mo, -k))).every((m) => m !== undefined && m.c < m.o);

/** Every month's 5DP range from one index's sessions (full trading days only); the end is null until the next month has two. */
export function fiveDpRanges(bars: readonly Bar[]): Range[] {
  const days = new Map<string, string[]>(), close = new Map(bars.map((b) => [b.d, b.c]));
  for (const b of bars) if (isFullDay(b.d)) { const k = b.d.slice(0, 7); days.set(k, [...(days.get(k) ?? []), b.d]); }
  const out: Range[] = [];
  for (const [mo, ds] of days) {
    const n = daysIn(mo) > 30 ? 6 : 5;
    if (ds.length < n) continue;
    const next = days.get(shiftMonth(mo, 1)), start = ds[ds.length - n], end = next && next.length >= 2 ? next[1] : null;
    out.push({ mo, start, end, sc: close.get(start), ec: end ? close.get(end) : undefined });
  }
  return out;
}

/** The months whose range n consecutive losing notional trades emit (blackout5dp.js tradeLossDates). */
export function tradeLossMonths(ranges: readonly Range[], n = 2): Set<string> {
  const hits = new Set<string>();
  let run = 0;
  for (const r of ranges) {
    if (run >= n && r.end) hits.add(r.mo);
    if (r.sc === undefined || r.ec === undefined) continue; // an unsettled range is no result: it neither adds nor resets
    run = r.ec < r.sc ? run + 1 : 0;
  }
  return hits;
}

/** QTR (quarterly 2MR) dates of one index, the month-end of a settled month in 1, 2, 3, 5, 6, 8, 9, 11, 12 that is a quarter month
 *  with 2 red months on the index, 3MR NIFTY 50 + 2MR Midcap, or 4 red months on the index, NIFTY 50 or Midcap. For Midcap this is
 *  backend_date_query.js midcapQtr (defaults); on all three it reproduces every date of the AFLs' hard-punched QTR lists (2008..2025-02)
 *  with 0 missing (extras: Midcap 2, BankNifty 5). */
function qtrDates(own: Candle[], nifty: Map<string, Candle>, midBy: Map<string, Candle>): string[] {
  const by = new Map(own.map((m) => [m.mo, m]));
  return own.slice(0, -1).filter((m) => QTR_MONTHS.has(month(m.mo)) && ((QUARTER.has(month(m.mo)) && red(by, m.mo, 2)) || (red(nifty, m.mo, 3) && red(midBy, m.mo, 2))
    || red(by, m.mo, 4) || red(nifty, m.mo, 4) || red(midBy, m.mo, 4))).map((m) => m.last);
}

/** Base-1 start and end dates (backend_date_query.js buildBase1): a phase opens at the month-end of a 3MR NIFTY 50 + 2MR Midcap
 *  month and closes at the earlier monthly ATR(5) x1 trailing-stop break of NIFTY 50 or Midcap; phases do not overlap. */
function base1Dates(series: Record<"nifty" | "midcap", Candle[]>, by: Record<"nifty" | "midcap", Map<string, Candle>>): Set<string> {
  const atr = (ms: Candle[]) => {
    const tr = ms.map((m, i) => (i ? Math.max(m.h - m.l, Math.abs(m.h - ms[i - 1].c), Math.abs(m.l - ms[i - 1].c)) : m.h - m.l));
    return ms.map((m, i) => ({ ...m, atr: i >= 4 ? (tr[i] + tr[i - 1] + tr[i - 2] + tr[i - 3] + tr[i - 4]) / 5 : null }));
  };
  const withAtr = { nifty: atr(series.nifty), midcap: atr(series.midcap) };
  const exitFrom = (k: "nifty" | "midcap", mo: string) => {
    let stop: number | null = null;
    for (const m of withAtr[k].slice(Math.max(0, withAtr[k].findIndex((x) => x.mo === mo)))) {
      if (m.mo < mo || m.atr === null) continue;
      if (stop !== null && m.c < stop) return m;
      const cand = (m.h + m.l) / 2 - m.atr;
      stop = stop === null ? cand : Math.max(stop, cand);
    }
    return null;
  };
  const out = new Set<string>();
  let openTill = "";
  for (const m of series.midcap.slice(0, -1)) {
    if (!(red(by.nifty, m.mo, 3) && red(by.midcap, m.mo, 2)) || (openTill && m.mo <= openTill)) continue;
    out.add(m.last);
    const exits = (["nifty", "midcap"] as const).map((k) => exitFrom(k, m.mo)).filter((x) => x !== null).sort((a, b) => (a.mo < b.mo ? -1 : 1));
    if (!exits.length) break;
    out.add(exits[0].last); openTill = exits[0].mo;
  }
  return out;
}

/** The Master 5DP windows, from the first month all three indices trade in full. */
export function master5dp({ nifty, bank, midcap, ad }: M5dpInput): M5dpWindow[] {
  if (!nifty.length || !bank.length || !midcap.length) return [];
  const first = shiftMonth([nifty, bank, midcap].map((b) => b[0].d.slice(0, 7)).sort().pop()!, 1);
  const tl = new Map<string, string[]>();
  for (const [name, bars] of [["NIFTY 50", nifty], ["NIFTYBANK", bank], ["NIFTYMIDCAP100", midcap]] as const)
    for (const mo of tradeLossMonths(fiveDpRanges(bars))) tl.set(mo, [...(tl.get(mo) ?? []), name]);
  const nm = monthly(nifty), mm = monthly(midcap), nBy = new Map(nm.map((m) => [m.mo, m])), mBy = new Map(mm.map((m) => [m.mo, m]));
  const qd = qtrDates(mm, nBy, mBy), b1 = base1Dates({ nifty: nm, midcap: mm }, { nifty: nBy, midcap: mBy });
  const days = nifty.map((b) => b.d).filter(isFullDay);
  const out: M5dpWindow[] = [];
  for (const r of fiveDpRanges(nifty)) {
    if (!r.end || r.mo < first) continue;
    const end = r.end, q = QDP.has(month(r.mo)), t = tl.get(r.mo);
    const a = days.find((d) => d >= r.start && d < end && (ad.get(d) ?? Infinity) <= ADR_MAX);
    if (!q && !t && !a) continue;
    const qq = qd.find((d) => d >= r.start && d < end);
    let from = q || t ? r.start : a!;
    if (qq) {
      if (!q && !t) { if (a! < qq) continue; }
      else if (!(q && b1.has(qq))) from = qq;
    }
    out.push({ from, to: end, systems: [...(q ? ["QDP+Nov"] : []), ...(t ? [`2 Trade Loss (${t.join(", ")})`] : []), ...(a ? [`0.11 ADR ${a}`] : [])] });
  }
  return out;
}

/** QTR windows from QTR dates: each date to the last trading day of the next month (the AFLs' 1-month hold), overlapping
 *  holds joined. Shortcut: while the next month is still trading its last day is not known, so the hold runs to its last
 *  calendar day; upgrade with the exchange calendar. */
function qtrWindows(dates: readonly string[], bars: readonly Bar[], systems: string): M5dpWindow[] {
  const last = new Map<string, string>();
  for (const b of bars) last.set(b.d.slice(0, 7), b.d);
  const open = bars.length ? bars[bars.length - 1].d.slice(0, 7) : "";
  const out: M5dpWindow[] = [];
  for (const d of [...dates].sort()) {
    const nm = shiftMonth(d.slice(0, 7), 1), to = nm < open ? last.get(nm) ?? `${nm}-${daysIn(nm)}` : `${nm}-${daysIn(nm)}`;
    const prev = out[out.length - 1];
    if (prev && d <= prev.to) { if (to > prev.to) prev.to = to; prev.systems = [...new Set([...prev.systems, systems])]; }
    else out.push({ from: d, to, systems: [systems] });
  }
  return out;
}

/** The short-strategy AFLs' no-trade sets from their hard-punched dates: Master 5DP (Nifty) as typed, and QTR Midcap held from
 *  each date to the last session of the next month. `midcap` gives that last session (the NSE calendar of the bars). */
export function regimeSets({ midcap }: { midcap: readonly Bar[] }): RegimeSet[] {
  if (!midcap.length) return [];
  return [
    { name: "Master 5DP", windows: MASTER_5DP_NIFTY.map(([a, b]): [string, string] => [a, b]) },
    { name: "QTR Midcap", windows: qtrWindows(QTR_MIDCAP, midcap, "QTR NIFTYMIDCAP100").map((x): [string, string] => [x.from, x.to]) },
  ];
}

const ohlc = async (ticker: string): Promise<Bar[]> => {
  const [o, h, l, c] = await Promise.all(["open", "high", "low", "close"].map((id) => series("in", { id, ticker })));
  const col = (r: typeof o) => r.series[r.outputs[0]];
  const [O, H, L, C] = [o, h, l, c].map(col);
  return o.dates.flatMap((d, i) => (O[i] != null && H[i] != null && L[i] != null && C[i] != null ? [{ d, o: O[i]!, h: H[i]!, l: L[i]!, c: C[i]! }] : []));
};
let loading: Promise<RegimeSet[]> | null = null;
/** Master 5DP and the QTR sets (a short strategy's no-trade windows), built from the India engine's NIFTY 50 / NIFTYBANK / NIFTYMIDCAP100
 *  bars and NIFTY 50 member breadth; loaded once per page. */
export function loadRegimeSets() {
  loading ??= (async () => {
    return regimeSets({ midcap: await ohlc("NIFTYMIDCAP100") });
  })().catch((e: unknown) => { loading = null; throw e; });
  return loading;
}
