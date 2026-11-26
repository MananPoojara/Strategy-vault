/** Volume Profile and Market Profile (TPO) drawn on the price pane, from the bars the chart shows (daily data: each bar's volume is
 *  spread evenly over its high–low range; a TPO letter is one bar). Display only: the engine's volume_profile / market_profile_tpo
 *  indicators stay the values rules read. Value Area as TradingView: from the POC, add the larger neighbouring row until 70%. */
import type { LabChartApi, LabSeriesApi } from "./globals";
import type { ChartColors } from "./charts";
import { tint } from "./charts";

export type ProfileMode = "off" | "vp_visible" | "vp_W" | "vp_M" | "tpo_W" | "tpo_M";
export const PROFILE_MODES: [ProfileMode, string][] = [["off", "Off"], ["vp_visible", "Volume profile · visible range"], ["vp_W", "Volume profile · weekly"],
  ["vp_M", "Volume profile · monthly"], ["tpo_W", "Market profile (TPO) · weekly"], ["tpo_M", "Market profile (TPO) · monthly"]];

export interface ProfileBars { d: string[]; o: number[]; h: number[]; l: number[]; c: number[]; v: (number | null)[] }
export interface Profile {
  from: number; to: number; lo: number; step: number;
  /** Volume profile: up and down volume per row (bottom row first). TPO: the bars' letters per row. */
  up: number[]; down: number[]; letters?: string[][];
  poc: number; vaLo: number; vaHi: number;
  /** TPO only: the first bar's range (TradingView's Initial Balance, here the period's first bar). */
  ib?: [number, number];
}

const LETTERS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/** TradingView's Value Area: start at the POC and add whichever neighbouring row holds more, one row at a time, until `share` of the total. */
export function valueArea(rows: number[], poc: number, share = 0.7): [number, number] {
  const total = rows.reduce((a, b) => a + b, 0);
  let lo = poc, hi = poc, sum = rows[poc] ?? 0;
  while (sum < share * total && (lo > 0 || hi < rows.length - 1)) {
    const below = lo > 0 ? rows[lo - 1] : -1, above = hi < rows.length - 1 ? rows[hi + 1] : -1;
    if (above >= below) sum += rows[++hi]; else sum += rows[--lo];
  }
  return [lo, hi];
}

/** The price range of bars `from`–`to`. */
export const span = (b: ProfileBars, from: number, to: number) => {
  let lo = Infinity, hi = -Infinity;
  for (let i = from; i <= to; i++) { if (Number.isFinite(b.l[i])) lo = Math.min(lo, b.l[i]); if (Number.isFinite(b.h[i])) hi = Math.max(hi, b.h[i]); }
  return Number.isFinite(lo) && Number.isFinite(hi) ? [lo, hi] as const : null;
};
/** Rows for one profile: `rows` across its own range, or a fixed `step` (one row size for every profile on the chart, as TradingView). */
const grid = (b: ProfileBars, from: number, to: number, rows: number, step?: number) => {
  const r = span(b, from, to);
  if (!r) return null;
  const [lo, hi] = r;
  if (step && step > 0) { const base = Math.floor(lo / step) * step; return { lo: base, step, rows: Math.max(1, Math.ceil((hi - base) / step - 1e-9)) }; }
  return { lo, step: hi > lo ? (hi - lo) / rows : Math.max(Math.abs(lo) * 1e-4, 1e-9), rows };
};

/** One volume profile over bars `from`–`to` (inclusive): null when the bars carry no volume. */
export function volumeProfile(b: ProfileBars, from: number, to: number, rows = 24, share = 0.7, step?: number): Profile | null {
  const g = grid(b, from, to, rows, step);
  if (!g) return null;
  rows = g.rows;
  const up = new Array<number>(rows).fill(0), down = new Array<number>(rows).fill(0);
  let any = false;
  for (let i = from; i <= to; i++) {
    const v = b.v[i];
    if (typeof v !== "number" || !(v > 0) || !Number.isFinite(b.h[i]) || !Number.isFinite(b.l[i])) continue;
    any = true;
    const side = b.c[i] >= b.o[i] ? up : down, span = b.h[i] - b.l[i];
    const r0 = Math.max(0, Math.min(rows - 1, Math.floor((b.l[i] - g.lo) / g.step))), r1 = Math.max(0, Math.min(rows - 1, Math.floor((b.h[i] - g.lo) / g.step - 1e-9)));
    if (span <= 0 || r0 === r1) { side[r0] += v; continue; }
    // the bar's volume, spread over its range in proportion to the overlap with each row
    for (let r = r0; r <= r1; r++) {
      const top = Math.min(b.h[i], g.lo + (r + 1) * g.step), bot = Math.max(b.l[i], g.lo + r * g.step);
      side[r] += v * Math.max(0, top - bot) / span;
    }
  }
  if (!any) return null;
  const tot = up.map((u, r) => u + down[r]), poc = tot.indexOf(Math.max(...tot)), [vaLo, vaHi] = valueArea(tot, poc, share);
  return { from, to, lo: g.lo, step: g.step, up, down, poc, vaLo, vaHi };
}

/** One TPO profile: each bar is a letter (A, B, …) in every row its high–low range touches. */
export function tpoProfile(b: ProfileBars, from: number, to: number, rows = 24, share = 0.7, step?: number): Profile | null {
  const g = grid(b, from, to, rows, step);
  if (!g) return null;
  rows = g.rows;
  const letters: string[][] = Array.from({ length: rows }, () => []);
  for (let i = from, k = 0; i <= to; i++, k++) {
    if (!Number.isFinite(b.h[i]) || !Number.isFinite(b.l[i])) continue;
    const r0 = Math.max(0, Math.floor((b.l[i] - g.lo) / g.step)), r1 = Math.min(rows - 1, Math.floor((b.h[i] - g.lo) / g.step - 1e-9));
    for (let r = r0; r <= Math.max(r0, r1); r++) letters[Math.min(r, rows - 1)].push(LETTERS[k % LETTERS.length]);
  }
  const counts = letters.map((l) => l.length), max = Math.max(...counts);
  // TradingView: among equal rows, the POC is the one nearest the middle of the profile
  const mid = (rows - 1) / 2, poc = counts.reduce((best, n, r) => (n === max && Math.abs(r - mid) < Math.abs(best - mid) ? r : best), counts.indexOf(max));
  const [vaLo, vaHi] = valueArea(counts, poc, share);
  return { from, to, lo: g.lo, step: g.step, up: counts, down: counts.map(() => 0), letters, poc, vaLo, vaHi, ib: [b.l[from], b.h[from]] };
}

/** Bar index ranges of each calendar week (Monday start) or month. */
export function periods(dates: string[], unit: "W" | "M"): [number, number][] {
  const key = (d: string) => {
    if (unit === "M") return d.slice(0, 7);
    const t = new Date(`${d}T00:00:00Z`), day = (t.getUTCDay() + 6) % 7;
    return new Date(t.getTime() - day * 864e5).toISOString().slice(0, 10);
  };
  const out: [number, number][] = [];
  dates.forEach((d, i) => { const k = key(d); if (i && key(dates[i - 1]) === k) out[out.length - 1][1] = i; else out.push([i, i]); });
  return out;
}

interface Target { useBitmapCoordinateSpace(f: (s: { context: CanvasRenderingContext2D; bitmapSize: { width: number; height: number }; horizontalPixelRatio: number; verticalPixelRatio: number }) => void): void }

/** Lightweight Charts series primitive: draws the chosen profiles behind the candles of the price pane and follows zoom and scroll. */
export class ProfilePrimitive {
  private chart?: LabChartApi; private series?: LabSeriesApi; private update?: () => void;
  private readonly onRange = () => this.update?.();
  constructor(private bars: ProfileBars, private mode: Exclude<ProfileMode, "off">, private colors: ChartColors, private rows = 24) {}
  attached(p: { chart: LabChartApi; series: LabSeriesApi; requestUpdate: () => void }) {
    this.chart = p.chart; this.series = p.series; this.update = p.requestUpdate;
    p.chart.timeScale().subscribeVisibleLogicalRangeChange?.(this.onRange);
  }
  detached() { this.chart?.timeScale().unsubscribeVisibleLogicalRangeChange?.(this.onRange); this.chart = undefined; }
  updateAllViews() { /* drawn from the current scales on every paint */ }
  /** The profiles for the current view: one for the visible bars, or one per week / month that is on screen. */
  profiles(): Profile[] {
    const n = this.bars.d.length, range = this.chart?.timeScale().getVisibleLogicalRange?.();
    if (!n) return [];
    const from = Math.max(0, Math.floor(range?.from ?? 0)), to = Math.min(n - 1, Math.ceil(range?.to ?? n - 1));
    if (to < from) return [];
    if (this.mode === "vp_visible") { const p = volumeProfile(this.bars, from, to, this.rows); return p ? [p] : []; }
    const tpo = this.mode.startsWith("tpo"), unit = this.mode.endsWith("W") ? "W" : "M", prices = span(this.bars, from, to);
    // one row size for every profile on screen (TradingView's "Row size"): the visible price range in 60 rows
    const step = prices ? (prices[1] - prices[0]) / 60 || undefined : undefined;
    return periods(this.bars.d, unit).filter(([a, b]) => b >= from && a <= to)
      .map(([a, b]) => (tpo ? tpoProfile(this.bars, a, b, this.rows, 0.7, step) : volumeProfile(this.bars, a, b, this.rows, 0.7, step))).filter((p): p is Profile => !!p);
  }
  paneViews() {
    const self = this;
    return [{ zOrder: () => "bottom", renderer: () => ({ draw: (target: Target) => self.draw(target) }) }];
  }
  private draw(target: Target) {
    const ts = this.chart?.timeScale(), y = (p: number) => this.series?.priceToCoordinate?.(p) ?? null;
    if (!ts?.logicalToCoordinate) return;
    const list = this.profiles(), c = this.colors;
    target.useBitmapCoordinateSpace(({ context: ctx, bitmapSize, horizontalPixelRatio: hr, verticalPixelRatio: vr }) => {
      // TPO: letters only when every profile on screen has room for them (TradingView zoomed in), else blocks everywhere
      const readable = list.every((p) => {
        if (!p.letters) return true;
        const a = ts.logicalToCoordinate!(p.from), b = ts.logicalToCoordinate!(p.to + 1), y0 = y(p.lo), y1 = y(p.lo + p.step);
        const most = Math.max(...p.letters.map((l) => l.length)) || 1;
        return a !== null && b !== null && y0 !== null && y1 !== null && Math.abs(y0 - y1) >= 9 && ((b - a) * 0.96 - 6) / most >= 9;
      });
      for (const p of list) {
        const rows = p.up.length, tot = p.up.map((u, r) => u + p.down[r]), max = Math.max(...tot) || 1;
        const right = this.mode === "vp_visible";
        let x0: number, width: number;
        // TradingView: the visible-range profile hugs the right edge (30% of the pane); a periodic or TPO profile starts at its period
        if (right) { width = bitmapSize.width * 0.3; x0 = bitmapSize.width - width; }
        else {
          const a = ts.logicalToCoordinate!(p.from), b = ts.logicalToCoordinate!(p.to + 1);
          if (a === null || b === null) continue;
          x0 = a * hr; width = Math.max(4 * hr, (b - a) * hr * 0.96);
        }
        const rowY = (r: number) => { const yt = y(p.lo + (r + 1) * p.step), yb = y(p.lo + r * p.step); return yt === null || yb === null ? null : [Math.round(yt * vr), Math.max(1, Math.round((yb - yt) * vr) - Math.round(vr))] as const; };
        if (p.letters) { this.drawTpo(ctx, p, x0, width, rowY, hr, vr, readable); continue; }
        for (let r = 0; r < rows; r++) {
          const at = rowY(r);
          if (!at) continue;
          const [top, h] = at, inVa = r >= p.vaLo && r <= p.vaHi;
          const wUp = width * p.up[r] / max, wDn = width * p.down[r] / max;
          // TradingView colours: up volume teal, down volume pink; bright inside the 70% Value Area, dark outside it.
          // Up volume sits on the anchor side, down volume beyond it.
          ctx.fillStyle = inVa ? UP_VA : UP; ctx.fillRect(right ? x0 + width - wUp : x0, top, wUp, h);
          ctx.fillStyle = inVa ? DOWN_VA : DOWN; ctx.fillRect(right ? x0 + width - wUp - wDn : x0 + wUp, top, wDn, h);
        }
        // the Point of Control: a red line across the profile
        const yp = y(p.lo + (p.poc + 0.5) * p.step);
        if (yp !== null) { ctx.fillStyle = c.red; ctx.fillRect(x0, Math.round(yp * vr), width, Math.max(1, Math.round(1.5 * vr))); }
      }
    });
  }
  /** TradingView's TPO: a block (or, with room, the letter) per bar in each row, VA blocks brighter, the POC row red, the period's
   *  range as a thin line at its left and the VAH / POC / VAL marks beside it. */
  private drawTpo(ctx: CanvasRenderingContext2D, p: Profile, x0: number, width: number, rowY: (r: number) => readonly [number, number] | null, hr: number, vr: number, readable: boolean) {
    const c = this.colors, letters = p.letters!, rows = letters.length;
    const most = Math.max(...letters.map((l) => l.length)) || 1, gap = Math.max(1, Math.round(hr));
    const first = rowY(0), last = rowY(rows - 1);
    if (!first || !last) return;
    const rowH = first[1], cell = Math.max(1, Math.min(rowH + Math.round(vr), Math.floor((width - 6 * hr) / most)));
    // the period's range: a thin line at the profile's left
    ctx.fillStyle = tint(c.ink2, 0.8); ctx.fillRect(x0, last[0], Math.max(1, Math.round(hr)), first[0] + first[1] - last[0]);
    const x1 = x0 + 4 * hr;
    for (let r = 0; r < rows; r++) {
      const at = rowY(r);
      if (!at) continue;
      const [top, h] = at, inVa = r >= p.vaLo && r <= p.vaHi;
      const color = r === p.poc ? c.red : inVa ? tint(c.ink, 0.85) : tint(c.ink3, 0.55);
      ctx.fillStyle = color;
      letters[r].forEach((ch, k) => {
        const x = x1 + k * cell;
        if (readable) { ctx.font = `${Math.round(Math.min(rowH, cell) * 0.8)}px ui-monospace, Menlo, monospace`; ctx.textBaseline = "middle"; ctx.fillText(ch, x, top + h / 2); }
        else ctx.fillRect(x, top, Math.max(1, cell - gap), h);
      });
    }
    // VAH / POC / VAL marks at the left edge, as on TradingView, when the profile is wide enough to carry them
    if (width < 60 * hr) return;
    ctx.font = `${Math.round(10 * vr)}px system-ui, sans-serif`; ctx.textBaseline = "middle"; ctx.textAlign = "right";
    const mark = (r: number, text: string, color: string) => { const at = rowY(r); if (!at) return; ctx.fillStyle = color; ctx.fillText(`${text} ▸`, x0 - 2 * hr, at[0] + at[1] / 2); };
    mark(p.vaHi, "VAH", c.ink2); mark(p.poc, "POC", c.red); mark(p.vaLo, "VAL", c.ink2);
    ctx.textAlign = "left";
  }
}

// TradingView's Volume Profile colours (Up / Down, and their Value Area versions)
const UP = "rgba(38, 198, 218, 0.30)", UP_VA = "rgba(38, 198, 218, 0.75)";
const DOWN = "rgba(242, 54, 105, 0.30)", DOWN_VA = "rgba(242, 54, 105, 0.75)";

/** Whether a profile mode can draw for these bars: volume profiles need volume. */
export const profileNeedsVolume = (mode: ProfileMode) => mode.startsWith("vp");
