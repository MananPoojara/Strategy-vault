import type { ChartRange, SeriesResponse } from "./types";

export interface Ohlc { d: string[]; o: number[]; h: number[]; l: number[]; c: number[] }

export function ohlcFromSeries(parts: Record<"open" | "high" | "low" | "close", SeriesResponse>): Ohlc {
  const v = (k: keyof typeof parts) => parts[k].series[parts[k].outputs[0]] ?? [];
  const [o, h, l, c] = [v("open"), v("high"), v("low"), v("close")];
  const out: Ohlc = { d: [], o: [], h: [], l: [], c: [] };
  parts.close.dates.forEach((date, i) => {
    if ([o[i], h[i], l[i], c[i]].some((x) => typeof x !== "number")) return;
    out.d.push(date); out.o.push(o[i]!); out.h.push(h[i]!); out.l.push(l[i]!); out.c.push(c[i]!);
  });
  return out;
}

/** Same keys as VaultReport.aggregate (Monday-start weeks, calendar months) plus Q and Y for Phase 7. */
export function periodKey(date: string, tf: string): string {
  if (tf === "M") return date.slice(0, 7);
  if (tf === "Y") return date.slice(0, 4);
  if (tf === "Q") return `${date.slice(0, 4)}-Q${Math.floor((Number(date.slice(5, 7)) - 1) / 3) + 1}`;
  if (tf === "W") {
    const day = new Date(`${date}T00:00:00Z`);
    day.setUTCDate(day.getUTCDate() - ((day.getUTCDay() + 6) % 7));
    return day.toISOString().slice(0, 10);
  }
  return date;
}

export function aggregate(px: Ohlc, tf: string): Ohlc {
  if (tf === "D") return px;
  const out: Ohlc = { d: [], o: [], h: [], l: [], c: [] };
  let previous = "";
  px.d.forEach((date, i) => {
    const key = periodKey(date, tf);
    if (key !== previous) {
      out.d.push(date); out.o.push(px.o[i]); out.h.push(px.h[i]); out.l.push(px.l[i]); out.c.push(px.c[i]);
      previous = key;
    } else {
      const j = out.d.length - 1;
      out.d[j] = date; out.h[j] = Math.max(out.h[j], px.h[i]); out.l[j] = Math.min(out.l[j], px.l[i]); out.c[j] = px.c[i];
    }
  });
  return out;
}

export const candles = (px: Ohlc) => px.d.map((time, i) => ({ time, open: px.o[i], high: px.h[i], low: px.l[i], close: px.c[i] }));

/** Whitespace points keep gaps (sparse outputs, warm-up) and keep logical ranges aligned. */
export const linePoints = (dates: string[], values: (number | null)[], keep?: Set<string>) =>
  dates.flatMap((time, i) => (keep && !keep.has(time) ? [] : [typeof values[i] === "number" ? { time, value: values[i]! } : { time }]));

export const isBooleanOutput = (values: (number | null)[]) => values.some((v) => v === 1) && values.every((v) => v === null || v === 0 || v === 1);

/** `out` named bull/up/buy (bear/down/sell): a small unlabelled dot under (over) the bar, as TradingView's SMC scripts draw them. */
export const booleanMarkers = (dates: string[], values: (number | null)[], text: string, color: string, keep?: Set<string>, out = "") => {
  const side = /^(bull|up|buy|long)/i.test(out) ? "belowBar" : /^(bear|down|sell|short)/i.test(out) ? "aboveBar" : "";
  return dates.flatMap((time, i) => (values[i] === 1 && (!keep || keep.has(time))
    ? [side ? { time, position: side, shape: "circle", color, text: "", size: 0.6 } : { time, position: "aboveBar", shape: "circle", color, text }] : []));
};

/** `x_top` / `x_bottom` output pairs (FVG zones): drawn as one shaded box per zone instead of two step lines. */
export const zonePairs = (outputs: string[]) =>
  outputs.flatMap((o) => { const m = /^(.*)_top$/.exec(o); return m && outputs.includes(`${m[1]}_bottom`) ? [m[1]] : []; });

export function rangeStart(dates: string[], range: ChartRange): string | null {
  if (range === "All" || !dates.length) return null;
  const last = new Date(`${dates[dates.length - 1]}T00:00:00Z`);
  last.setUTCFullYear(last.getUTCFullYear() - (range === "1Y" ? 1 : 3));
  const from = last.toISOString().slice(0, 10);
  return dates.find((d) => d >= from) ?? null;
}

/** Per-bar trade colour (undefined outside trades); open trades run to the last bar, unmapped dates are skipped. */
export function tradeSpans(trades: { entry: string; exit: string; open: boolean; ret: number | null }[], dateIndex: Map<string, number>, barOf: number[], bars: number,
  color: (t: { open: boolean; ret: number | null }) => string) {
  const shade: (string | undefined)[] = new Array(bars);
  for (const t of trades) {
    const a = barOf[dateIndex.get(t.entry) ?? -1], b = t.open ? bars - 1 : barOf[dateIndex.get(t.exit) ?? -1];
    if (a === undefined || b === undefined) continue;
    for (let i = a; i <= b; i++) shade[i] = color(t);
  }
  return shade;
}

const BREAK_TAGS: Record<string, string> = { bos: "BOS", choch: "CHoCH", mss: "MSS", eqh: "EQH", eql: "EQL" };
export interface BreakLine { from: number; to: number; price: number; text: string; up: boolean; dotted: boolean; above: boolean; solid?: boolean; atEnd?: boolean }
/** `bull_bos` / `bear_choch` / `bear_eqh` … events with a `<side>_level` or `<tag>_level` output: TradingView's line from the
 *  swing bar (the latest bar whose high, or low, is that level) to the event bar, labelled BOS / CHoCH / EQH / EQL. */
export function breakLines(outputs: string[], series: Record<string, (number | null)[]>, dates: string[], px: Ohlc, lookback = 300): BreakLine[] {
  const at = new Map(px.d.map((d, i) => [d, i])), out: BreakLine[] = [];
  for (const o of outputs) {
    const m = /^(bull|bear)_([a-z]+)$/.exec(o), tag = m?.[2];
    const level = m && tag && BREAK_TAGS[tag] ? [`${m[1]}_level`, `${tag}_level`].find((k) => outputs.includes(k)) : undefined;
    if (!m || !tag || !level) continue;
    const sig = series[o] ?? [], lvl = series[level] ?? [];
    dates.forEach((d, k) => {
      const t = at.get(d), p = lvl[k];
      if (sig[k] !== 1 || t === undefined || typeof p !== "number") return;
      // a bull break (and EQH) is a swing high's level, a bear break (and EQL) a swing low's
      const up = m[1] === "bull", dotted = tag.startsWith("eq"), high = up !== dotted;
      let from = -1, best = Infinity;
      for (let i = t - 1; i >= Math.max(0, t - lookback) && best > 0; i--) {
        const gap = Math.abs((high ? px.h[i] : px.l[i]) - p);
        if (gap < best) { best = gap; from = i; }
      }
      if (from >= 0) out.push({ from, to: t, price: p, text: BREAK_TAGS[tag], up, dotted, above: up !== dotted });
    });
  }
  return out;
}
/** The outputs breakLines draws (events and their levels), so they are not drawn again as dots. */
export const breakOutputs = (lines: BreakLine[], outputs: string[]) => lines.length
  ? new Set(outputs.filter((o) => { const m = /^(bull|bear)_([a-z]+)$/.exec(o); return (m && BREAK_TAGS[m[2]]) || /_level$/.test(o); })) : new Set<string>();

/** `trailing_high` / `trailing_low` / `bias` (Strong / Weak High-Low, LuxAlgo): only the latest top and bottom, a solid line
 *  from the bar that made it to the last bar, labelled at its right end; the top is Strong in a down bias, the bottom in an up bias. */
export function strongWeakLines(outputs: string[], series: Record<string, (number | null)[]>, dates: string[], px: Ohlc): BreakLine[] {
  if (!["trailing_high", "trailing_low", "bias"].every((o) => outputs.includes(o)) || !dates.length) return [];
  const last = px.d.indexOf(dates[dates.length - 1]), k = dates.length - 1, bias = series.bias?.[k];
  const line = (price: number | null | undefined, high: boolean): BreakLine[] => {
    if (typeof price !== "number" || last < 0) return [];
    let from = last;
    while (from > 0 && (high ? px.h[from] : px.l[from]) !== price) from--;
    const strong = high ? bias === -1 : bias === 1;
    return [{ from, to: last, price, text: `${strong ? "Strong" : "Weak"} ${high ? "High" : "Low"}`, up: !high, dotted: false, above: high, solid: true, atEnd: true }];
  };
  return [...line(series.trailing_high?.[k], true), ...line(series.trailing_low?.[k], false)];
}
