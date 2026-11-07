/** Pure helpers for the 5-minute view of an intraday strategy (BTST): which bars to load and where the trade markers go. */
export interface IntradayBar {t: string; o: number; h: number; l: number; c: number}
export interface TradeLike {entry: string; exit: string; open?: boolean; reason?: string; legs: Array<[number, number, ...number[]]>; exit_px: number; dir?: number}
export interface TradeMarker {time: string; price: number; side: 'buy' | 'sell'; text: string}
export interface TradeOnBars {entry: TradeMarker | null; exit: TradeMarker | null; profitLine: number | null; stopLine: number | null}

export const ENTRY_TIME_DEFAULT = 1515;
export const EXIT_TIME_DEFAULT = 920;

/** True for a strategy that reads 5-minute bars (needs_series starts with BTST_5M.). */
export function isIntradayStrategy(needs: string[] | undefined): boolean {
  return !!needs?.length && needs.every(name => name.startsWith('BTST_5M.'));
}

/** The entry time in effect: the page's entry_time when set, else the default. */
export function entryHHMM(params: Record<string, number> | undefined): number {
  const v = params?.entry_time;
  return typeof v === 'number' && Number.isFinite(v) && v > 0 ? Math.round(v) : ENTRY_TIME_DEFAULT;
}

/** The exit time from the trade's reason ("... at 0935" or "Time exit 0920"), else the page's exit_time, else the default. */
export function exitHHMM(reason: string | undefined, params: Record<string, number> | undefined): number {
  const m = /(?:\bat|Time exit)\s+(\d{4})\b/.exec(reason ?? '');
  if (m) return Number(m[1]);
  const v = params?.exit_time;
  return typeof v === 'number' && Number.isFinite(v) && v > 0 ? Math.round(v) : EXIT_TIME_DEFAULT;
}

/** Epoch seconds of a "YYYY-MM-DD" date at HHMM, the exchange clock read as UTC (as the engine's compact bars are stamped). */
export function stampSeconds(date: string, hhmm = 0): number {
  return Date.parse(`${date}T00:00:00Z`) / 1000 + Math.floor(hhmm / 100) * 3600 + (hhmm % 100) * 60;
}

/** Chart time (seconds) of a "YYYY-MM-DD HH:MM" bar stamp. */
export function barSeconds(t: string): number {
  return Date.parse(`${t.replace(' ', 'T')}:00Z`) / 1000;
}

/** First index whose time is >= x in an ascending array (times.length when none). */
export function lowerBound(times: ArrayLike<number>, x: number): number {
  let lo = 0;
  let hi = times.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (times[mid] < x) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}

export interface TradeLayer {
  index: number;
  open: boolean;
  entryTime: number | null;
  entryPx: number;
  exitTime: number | null;
  exitPx: number;
  long: boolean;
  profitLine: number | null;
  stopLine: number | null;
}

/** First bar of `date` at or after `hhmm` (a missing bar snaps to the next one on that day); null when the day has none. */
function barTime(times: ArrayLike<number>, date: string, hhmm: number): number | null {
  const i = lowerBound(times, stampSeconds(date, hhmm));
  return i < times.length && times[i] < stampSeconds(date, 0) + 86400 ? times[i] : null;
}

/** Every trade placed on the loaded bars: entry and exit bar times, prices, and the profit-booking / hard-stop level when it exited that way. */
export function tradeLayers(trades: TradeLike[], times: ArrayLike<number>, params?: Record<string, number>): TradeLayer[] {
  const inHHMM = entryHHMM(params);
  return trades.map((t, index) => {
    const entryPx = t.legs[0]?.[1] ?? NaN;
    const reason = t.reason ?? '';
    const open = !!t.open || !t.exit;
    const early = /^(Profit booking|Hard stop)/.test(reason);
    const exitTime = open ? null : barTime(times, t.exit, exitHHMM(reason, params)) ?? (early ? barTime(times, t.exit, 0) : null);
    const pb = /^Profit booking ([\d.]+)%/.exec(reason);
    const hs = /^Hard stop ([\d.]+)%/.exec(reason);
    return {
      index, open, entryPx, exitPx: t.exit_px, exitTime, long: (t.dir ?? 1) >= 0,
      entryTime: Number.isFinite(entryPx) ? barTime(times, t.entry, inHHMM) : null,
      profitLine: pb && Number.isFinite(entryPx) ? entryPx * (1 + Number(pb[1]) / 100) : null,
      stopLine: hs && Number.isFinite(entryPx) ? entryPx * (1 - Number(hs[1]) / 100) : null,
    };
  });
}

export interface LayerMarker {time: number; position: 'belowBar' | 'aboveBar'; shape: 'arrowUp' | 'arrowDown'; color: string}

/** Arrows with the Daily chart's convention: buy/cover green up, sell/short red down, whatever the profit; sorted by time. */
export function layerMarkers(layers: TradeLayer[], colors: {buy: string; sell: string}): LayerMarker[] {
  const out: LayerMarker[] = [];
  const mark = (time: number, buy: boolean): void => {
    out.push({time, position: buy ? 'belowBar' : 'aboveBar', shape: buy ? 'arrowUp' : 'arrowDown', color: buy ? colors.buy : colors.sell});
  };
  for (const l of layers) {
    if (l.entryTime != null) mark(l.entryTime, l.long);
    if (l.exitTime != null) mark(l.exitTime, !l.long);
  }
  return out.sort((a, b) => a.time - b.time);
}

/** Shaded spans: entry bar to exit bar (an open trade runs to the last bar). */
export function tradeSpans(layers: TradeLayer[], lastTime: number): Array<{a: number; b: number; index: number}> {
  const spans: Array<{a: number; b: number; index: number}> = [];
  for (const l of layers) {
    const b = l.open ? lastTime : l.exitTime;
    if (l.entryTime != null && b != null && b >= l.entryTime) spans.push({a: l.entryTime, b, index: l.index});
  }
  return spans;
}

/** The two sessions of a trade as a chart time range (entry day start to the end of the exit day). */
export function tradeWindow(t: Pick<TradeLike, 'entry' | 'exit' | 'open'>): {from: number; to: number} {
  const end = t.open || !t.exit ? t.entry : t.exit;
  return {from: stampSeconds(t.entry, 0), to: stampSeconds(end, 0) + 86399};
}

/** Index of the previous / next trade in the picker, clamped to the list. */
export function stepTrade(index: number, delta: number, count: number): number {
  return count <= 0 ? 0 : Math.min(count - 1, Math.max(0, index + delta));
}
