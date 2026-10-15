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

export const booleanMarkers = (dates: string[], values: (number | null)[], text: string, color: string, keep?: Set<string>) =>
  dates.flatMap((time, i) => (values[i] === 1 && (!keep || keep.has(time)) ? [{ time, position: "aboveBar", shape: "circle", color, text }] : []));

export function rangeStart(dates: string[], range: ChartRange): string | null {
  if (range === "All" || !dates.length) return null;
  const last = new Date(`${dates[dates.length - 1]}T00:00:00Z`);
  last.setUTCFullYear(last.getUTCFullYear() - (range === "1Y" ? 1 : 3));
  const from = last.toISOString().slice(0, 10);
  return dates.find((d) => d >= from) ?? null;
}
