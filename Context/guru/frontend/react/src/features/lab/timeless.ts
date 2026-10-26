import { createLabChart, type ChartColors } from "./charts";
import type { ChartRange, SeriesResponse, Trade } from "./types";
import { rangeStart } from "./bars";

/** Chart types drawn without a date axis; each value except candles is the catalogue id whose daily-aligned outputs the view is rebuilt from. */
export type ChartType = "candles" | "heikin_ashi" | "renko" | "pnf" | "kagi" | "line_break";
export const CHART_TYPES: [ChartType, string][] = [["candles", "Candles"], ["heikin_ashi", "Heikin Ashi"], ["renko", "Renko"], ["pnf", "Point & Figure"], ["kagi", "Kagi"], ["line_break", "Line Break"]];
const NOUN: Record<ChartType, [string, string]> = { candles: ["bar", "bars"], heikin_ashi: ["candle", "candles"], renko: ["brick", "bricks"], pnf: ["column", "columns"], kagi: ["line", "lines"], line_break: ["line", "lines"] };
export const nounOf = (t: ChartType, many = false) => NOUN[t][many ? 1 : 0];
/** The view keeps the latest this many bricks, columns or lines (the whole history at a sensible box size; a tiny box would otherwise build millions). */
export const MAX_SLOTS = 10000;

/** One brick, column, line or candle. `at` = the date it appears (formed for Renko/Line Break/Heikin Ashi, started for P&F/Kagi), used to place trades;
 *  `from`–`to` = the dates that built it. P&F: `boxes` figures of `box` points. Kagi: thickness at the start and end, `split` = the level where it changes. */
export interface Slot {
  at: string; from: string; to: string; open: number; high: number; low: number; close: number; up: boolean;
  boxes?: number; box?: number; split?: number; thick0?: boolean; thick1?: boolean;
}
const ok = (v: number | null | undefined): v is number => typeof v === "number" && Number.isFinite(v);
const box = (at: string, from: string, to: string, open: number, close: number): Slot => ({ at, from, to, open, close, high: Math.max(open, close), low: Math.min(open, close), up: close > open });

/** Rebuilds the drawn elements from the engine's per-bar outputs (the engine stays the authority); keeps the latest `cap`. */
export function rebuild(type: ChartType, res: SeriesResponse, params: Record<string, number> = {}, cap = MAX_SLOTS): { slots: Slot[]; total: number } {
  const s = (out: string) => res.series[out] ?? [], d = res.dates;
  const slots: Slot[] = [];
  let start = 0; // first bar after the last formed element
  if (type === "renko") {
    const [k, hi, lo, dir] = ["bricks", "brick_high", "brick_low", "direction"].map(s);
    const counts = d.map((_, i) => (ok(k[i]) && ok(hi[i]) && ok(lo[i]) ? Math.abs(k[i]!) : 0));
    // AmiBroker grid anchor: a seed down brick on the anchor bar (bricks 0, direction −1)
    const anchor = d.findIndex((_, i) => ok(k[i]) && ok(hi[i]) && ok(lo[i]));
    if (anchor >= 0 && k[anchor] === 0 && dir[anchor] === -1) counts[anchor] = 1;
    start = Math.max(0, anchor); // the first brick's dates start where the engine's values do
    const total = counts.reduce((a, b) => a + b, 0);
    let skip = Math.max(0, total - cap);
    counts.forEach((n, i) => {
      if (!n) return;
      const size = hi[i]! - lo[i]!;
      for (let j = n - 1; j >= 0; j--) {
        if (skip > 0) { skip--; continue; }
        const from = j === n - 1 ? d[start] : d[i];
        slots.push(k[i]! > 0 ? box(d[i], from, d[i], hi[i]! - (j + 1) * size, hi[i]! - j * size) : box(d[i], from, d[i], lo[i]! + (j + 1) * size, lo[i]! + j * size));
      }
      start = i + 1;
    });
    return { slots, total };
  }
  if (type === "line_break") {
    const [nl, lo, lc] = ["new_line", "line_open", "line_close"].map(s);
    start = Math.max(0, d.findIndex((_, i) => ok(lc[i])));
    d.forEach((date, i) => { if (ok(nl[i]) && nl[i] !== 0 && ok(lo[i]) && ok(lc[i])) { slots.push({ ...box(date, d[start], date, lo[i]!, lc[i]!), up: nl[i]! > 0 }); start = i + 1; } });
  } else if (type === "heikin_ashi") {
    const [o, h, l, c] = ["open", "high", "low", "close"].map(s);
    d.forEach((date, i) => { if ([o[i], h[i], l[i], c[i]].every(ok)) slots.push({ at: date, from: date, to: date, open: o[i]!, high: h[i]!, low: l[i]!, close: c[i]!, up: c[i]! >= o[i]! }); });
  } else if (type === "pnf") {
    const [dir, hi, lo, nb, nc, added, rev] = ["col_dir", "col_high", "col_low", "boxes", "new_col", "boxes_added", "rev_level"].map(s);
    const reversal = params.reversal ?? res.params?.reversal ?? 3;
    let cur: Slot | undefined;
    d.forEach((date, i) => {
      if (!ok(dir[i]) || dir[i] === 0 || !ok(hi[i]) || !ok(lo[i])) return;
      if (!cur || nc[i] === 1) slots.push(cur = { at: date, from: date, to: date, open: 0, high: 0, low: 0, close: 0, up: true });
      const up = dir[i]! > 0;
      // rev_level = top − reversal·box (X) or bottom + reversal·box (O): the box in force on this bar
      Object.assign(cur, { high: hi[i]!, low: lo[i]!, open: up ? lo[i]! : hi[i]!, close: up ? hi[i]! : lo[i]!, up, boxes: nb[i] ?? 1,
        box: ok(rev[i]) ? Math.abs((up ? hi[i]! : lo[i]!) - rev[i]!) / reversal : cur.box });
      if (ok(added[i]) && added[i] !== 0) cur.to = date;
    });
  } else if (type === "kagi") {
    const [tip, dir, turn, shoulder, waist, yang, yangFlip, yinFlip] = ["tip", "direction", "reversal", "shoulder", "waist", "yang", "yang_flip", "yin_flip"].map(s);
    let cur: Slot | undefined, last = NaN;
    d.forEach((date, i) => {
      const t = tip[i];
      if (!ok(t)) return;
      if (!dir[i]) { last = t; return; } // the start price, before the first line
      if (!cur || turn[i] === 1) {
        const open = Number.isFinite(last) ? last : t;
        slots.push(cur = { ...box(date, date, date, open, t), thick0: cur ? cur.thick1 : yang[i]! > 0 });
      }
      if (yangFlip[i] === 1 && ok(shoulder[i])) cur.split = shoulder[i]!;
      if (yinFlip[i] === 1 && ok(waist[i])) cur.split = waist[i]!;
      if (t !== cur.close) cur.to = date;
      Object.assign(cur, { close: t, high: Math.max(cur.open, t), low: Math.min(cur.open, t), up: dir[i]! > 0, thick1: yang[i]! > 0 });
      last = t;
    });
  }
  // a column or line holds its quiet days too: it lasts until the day before the next one starts, so hover dates and trade markers agree
  if (type === "pnf" || type === "kagi") {
    const pos = new Map(d.map((date, i) => [date, i]));
    slots.forEach((x, j) => { x.to = j + 1 < slots.length ? d[pos.get(slots[j + 1].from)! - 1] : d[d.length - 1]; });
  }
  return { slots: slots.slice(-cap), total: slots.length };
}

/** Index of the element whose dates hold `date`: the last one starting on or before it (-1 before the first); a date after the
 *  last formed brick or line belongs to that last element, still being extended. */
export function slotAt(slots: Slot[], date: string) {
  let a = 0, b = slots.length - 1, hit = -1;
  while (a <= b) { const m = (a + b) >> 1; if (slots[m].from <= date) { hit = m; a = m + 1; } else b = m - 1; }
  return hit;
}

/** A backtest fill at its real price on the element holding its date; fills without a price or before the first element are not drawn. */
export interface Mark { slot: number; price: number; buy: boolean; leg: boolean }
export function tradeMarks(trades: Trade[], slots: Slot[]): Mark[] {
  return trades.flatMap((t) => {
    const long = t.dir > 0;
    const legs: [string, number | null, ...unknown[]][] = t.legs?.length ? t.legs : [[t.entry, t.entry_px]];
    const fills = legs.map(([date, price], k) => ({ slot: slotAt(slots, date), price, buy: long, leg: k > 0 }));
    if (!t.open) fills.push({ slot: slotAt(slots, t.exit), price: t.exit_px, buy: !long, leg: false });
    return fills.filter((m): m is Mark => m.slot >= 0 && ok(m.price));
  });
}

const fmt = (v: number) => v.toFixed(2);
export const spanText = (s: Slot) => (s.from === s.to ? s.to : `${s.from} → ${s.to}`);
/** Legend line for one element: its dates and prices. */
export function slotText(type: ChartType, s: Slot) {
  if (type === "pnf") return `${s.up ? "X" : "O"} column · ${s.boxes ?? 1} boxes · High ${fmt(s.high)}  Low ${fmt(s.low)}`;
  if (type === "kagi") return `${s.up ? "Up" : "Down"} line · ${s.thick1 ? "thick (yang)" : "thin (yin)"} · ${fmt(s.open)} → ${fmt(s.close)}`;
  if (type === "heikin_ashi") return `O ${fmt(s.open)}  H ${fmt(s.high)}  L ${fmt(s.low)}  C ${fmt(s.close)}`;
  return `${s.up ? "Up" : "Down"} ${nounOf(type)} · ${fmt(s.open)} → ${fmt(s.close)}`;
}

type Item = { time: number; slot: Slot; marks: Mark[] };
type Bars = { bars: readonly { x: number; originalData: Item }[]; barSpacing: number; visibleRange: { from: number; to: number } | null };
type Target = { useMediaCoordinateSpace(f: (scope: { context: CanvasRenderingContext2D }) => void): void };

/** Draws every element type in one Lightweight Charts custom series (P&F glyphs, Kagi thickness and fills at real prices need their own drawing). */
export function drawItems(g: CanvasRenderingContext2D, type: ChartType, data: Bars, toY: (price: number) => number | null, colors: ChartColors) {
  const { bars, barSpacing, visibleRange } = data;
  if (!visibleRange) return;
  const w = Math.max(1, barSpacing * 0.8), y = (p: number) => toY(p) ?? NaN;
  for (let i = visibleRange.from; i < visibleRange.to; i++) {
    const { x, originalData: { slot: s } } = bars[i];
    const tone = s.up ? colors.green : colors.red;
    g.fillStyle = g.strokeStyle = tone;
    if (type === "pnf") {
      const n = Math.max(1, s.boxes ?? 1), step = n > 1 ? (s.high - s.low) / (n - 1) : s.box ?? 0;
      const r = Math.min(w / 2, Math.abs(y(s.low) - y(s.low + step)) / 2) * 0.85;
      g.lineWidth = 1.5;
      g.beginPath();
      // too dense for glyphs: one bar from the bottom box to the top box
      if (!(r >= 1.5)) { g.moveTo(x, y(s.low)); g.lineTo(x, y(s.high)); }
      else for (let j = 0; j < n; j++) {
        const cy = y(s.low + j * step);
        if (s.up) { g.moveTo(x - r, cy - r); g.lineTo(x + r, cy + r); g.moveTo(x + r, cy - r); g.lineTo(x - r, cy + r); }
        else { g.moveTo(x + r, cy); g.arc(x, cy, r, 0, 2 * Math.PI); }
      }
      g.stroke();
    } else if (type === "kagi") {
      const width = (thick?: boolean) => (thick ? 3 : 1.2), next = bars[i + 1]?.x;
      const seg = (a: number, b: number, thick?: boolean, x2 = x) => {
        g.lineWidth = width(thick); g.strokeStyle = thick ? colors.green : colors.red;
        g.beginPath(); g.moveTo(x, y(a)); g.lineTo(x2, y(b)); g.stroke();
      };
      if (s.split !== undefined) { seg(s.open, s.split, s.thick0); seg(s.split, s.close, s.thick1); } else seg(s.open, s.close, s.thick1);
      if (next !== undefined) seg(s.close, s.close, s.thick1, next);
    } else {
      if (type === "heikin_ashi") { g.lineWidth = 1; g.beginPath(); g.moveTo(x, y(s.high)); g.lineTo(x, y(s.low)); g.stroke(); }
      const top = y(Math.max(s.open, s.close));
      g.fillRect(x - w / 2, top, w, Math.max(1, y(Math.min(s.open, s.close)) - top));
    }
  }
  // fills last, on top: a triangle whose tip is the real fill price (buy below it pointing up, sell above it pointing down)
  for (let i = visibleRange.from; i < visibleRange.to; i++) {
    const { x, originalData: { marks } } = bars[i];
    for (const m of marks) {
      const py = y(m.price), h = m.leg ? 6 : 9, dir = m.buy ? 1 : -1;
      g.fillStyle = m.buy ? colors.green : colors.red; g.strokeStyle = colors.panel; g.lineWidth = 1;
      g.beginPath(); g.moveTo(x, py); g.lineTo(x - h * 0.6, py + dir * h); g.lineTo(x + h * 0.6, py + dir * h); g.closePath(); g.fill(); g.stroke();
    }
  }
}

/** The timeless price chart: one custom series on synthetic times 1, 2, 3, … mapped back to dates for the axis and the legend. */
export function drawTimeless(el: HTMLElement, legend: HTMLElement | null, opts: {
  type: ChartType; name: string; ticker: string; res: SeriesResponse; params: Record<string, number>; trades?: Trade[]; range: ChartRange; onBehind(behind: boolean): void;
}) {
  const { type, res } = opts;
  const { slots, total } = rebuild(type, res, opts.params);
  const price = createLabChart(el, 420);
  if (!price) return null;
  const { chart, colors } = price;
  const marks = tradeMarks(opts.trades ?? [], slots);
  const items: Item[] = slots.map((slot, k) => ({ time: k + 1, slot, marks: marks.filter((m) => m.slot === k) }));
  const slotOf = (time: unknown) => slots[Number(time) - 1];
  chart.applyOptions({
    localization: { timeFormatter: (t: number) => (slotOf(t) ? spanText(slotOf(t)) : "") },
    timeScale: { tickMarkFormatter: (t: number) => slotOf(t)?.to ?? "", rightOffset: 2 },
  });
  let view: Bars | null = null;
  const series = chart.addCustomSeries?.({
    renderer: () => ({ draw: (target: Target, toY: (p: number) => number | null) => target.useMediaCoordinateSpace(({ context }) => view && drawItems(context, type, view, toY, colors)) }),
    update: (data: Bars) => { view = data; },
    priceValueBuilder: (d: Item) => [Math.min(d.slot.low, ...d.marks.map((m) => m.price)), Math.max(d.slot.high, ...d.marks.map((m) => m.price)), d.slot.close],
    isWhitespace: (d: Partial<Item>) => !d.slot,
    defaultOptions: () => ({ color: colors.ink }),
  }, { priceLineVisible: false, lastValueVisible: true });
  if (!series) { price.dispose(); return null; }
  series.setData(items);
  const ts = chart.timeScale(), last = slots.length - 1;
  const from = rangeStart(res.dates, opts.range), first = from ? slots.findIndex((s) => s.to >= from) : 0;
  if (last >= 0 && ts.setVisibleLogicalRange) ts.setVisibleLogicalRange({ from: first < 0 ? last : first, to: last + 2 }); else ts.fitContent();
  ts.subscribeVisibleLogicalRangeChange?.((r) => opts.onBehind(!!r && r.to < last - 0.5));
  const show = (s?: Slot) => {
    if (!legend) return;
    const row = document.createElement("div");
    row.textContent = s ? `${opts.ticker} · ${opts.name} · ${spanText(s)}  ${slotText(type, s)}` : `${opts.ticker} · ${opts.name} · no ${nounOf(type, true)} yet`;
    legend.replaceChildren(row);
  };
  show(slots[last]);
  chart.subscribeCrosshairMove?.((param) => show(slotOf(param.time) ?? slots[last]));
  return {
    ...price, slots, total,
    toLatest() {
      const r = ts.getVisibleLogicalRange?.();
      if (r && ts.setVisibleLogicalRange) ts.setVisibleLogicalRange({ from: last + 2 - (r.to - r.from), to: last + 2 });
    },
  };
}
