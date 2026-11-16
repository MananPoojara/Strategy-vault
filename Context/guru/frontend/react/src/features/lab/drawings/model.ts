import type { ReactNode } from "react";

/** A drawing point: [bar date, price]. */
export type Pt = [string, number];
export type Props = Record<string, number | string | boolean>;
/** Saved drawings. The first two shapes are what earlier versions stored (kept as they are on disk); new tools store pts. */
export type Drawing =
  | { kind: "h"; price: number }
  | { kind: "t" | "fib" | "pr" | "dr" | "dpr"; a: Pt; b: Pt; ext?: "right" | "both" }
  | { kind: string; pts: Pt[]; text?: string; props?: Props };
/** A drawing as renderers see it: always pts (a horizontal line's one point has no date). */
export type Shape = { kind: string; pts: Pt[]; text?: string; props?: Props; ext?: "right" | "both" };

export const shapeOf = (d: Drawing): Shape =>
  "price" in d ? { kind: d.kind, pts: [["", d.price as number]] }
  : "a" in d ? { kind: d.kind, pts: [d.a as Pt, d.b as Pt], ext: d.ext as Shape["ext"] }
  : d as Shape;
/** The drawing with new points, kept in its own stored form. */
export const withPts = (d: Drawing, pts: Pt[]): Drawing =>
  "price" in d ? { ...d, price: pts[0][1] } : "a" in d ? { ...d, a: pts[0], b: pts[1] } : { ...d, pts } as Drawing;

export type Bars = { d: string[]; o: number[]; h: number[]; l: number[]; c: number[]; v: (number | null)[] };
type XY = readonly [number, number];

export interface RenderCtx {
  /** Pixel x of a bar date (null off the data), of a bar index (also past the last bar), pixel y of a price. */
  x(date: string): number | null; xAt(bar: number): number | null; y(price: number): number | null;
  /** The bar index of a date, −1 when it is not a bar date. */
  index(date: string): number;
  bars: Bars;
  /** Plot width (up to the price axis), the price axis width, the layer height. */
  plot: number; axis: number; height: number;
  color: string;
  /** Selected, being moved or being drawn. */
  live: boolean;
  /** The drawing's index, null for the one being drawn (no hit targets). */
  i: number | null;
  /** Invisible hit targets (select / move): a line, a filled box, a path (fill: hit inside it too). */
  hit(x1: number, y1: number, x2: number, y2: number): ReactNode;
  hitArea(x: number, y: number, w: number, h: number): ReactNode;
  hitPath(d: string, fill?: boolean): ReactNode;
  /** The point handles (data-part p0…pN): by default at the drawing's pts; pass pixel positions to place them elsewhere. */
  handles(xy?: (XY | null)[]): ReactNode;
  /** A TradingView read-out box: white lines on fill, centred on x and kept in the plot; its top at y (below) or its bottom at y. */
  label(lines: string[], x: number, y: number, fill: string, below?: boolean): ReactNode;
  fmt(p: number): string; priceChange(a: number, b: number): string; dateSpan(bars: number, from: string, to: string): string;
}

export interface ToolDef {
  id: string;
  /** TradingView's name, and the fly-out section it is listed under ("LINES", "CHANNELS"…). */
  name: string; section: string;
  /** The contents of a 24×24 stroked SVG. */
  icon: ReactNode;
  /** Clicks to place it; "free": a press-drag path; "multi": click points, double-click or Enter finishes. */
  points: number | "free" | "multi";
  /** The stored drawing; default { kind: id, pts }. Also called with the points so far while it is drawn. Its kind must be a tool id. */
  make?(pts: Pt[], ctx: RenderCtx): Drawing;
  /** The drawing's SVG, inside a <g data-lab-drawing={kind}>; null draws nothing. */
  render(d: Shape, ctx: RenderCtx): ReactNode;
  /** Opens the text editor after placing; a double-click edits the text. */
  text?: boolean;
  /** TradingView's shortcut, e.g. "Alt+T", "Alt+Shift+R". */
  shortcut?: string;
  /** The hint while placing; a default follows from points. */
  hint?: string;
}

export const fmt = (p: number) => p.toLocaleString("en-US", { maximumFractionDigits: 2, minimumFractionDigits: 2 }); // TradingView: 25,281.85
/** Price range / Date range read-outs as TradingView labels them: "120.50 (0.48%)", "25 bars, 35d". */
export const priceChange = (a: number, b: number) => `${b < a ? "−" : ""}${fmt(Math.abs(b - a))} (${b < a ? "−" : ""}${a ? Math.abs((b - a) / a * 100).toFixed(2) : "0.00"}%)`;
export const dateSpan = (bars: number, from: string, to: string) => `${bars} bar${Math.abs(bars) === 1 ? "" : "s"}, ${Math.round((Date.parse(to) - Date.parse(from)) / 864e5)}d`;

/** The drawing's points in pixels, null while one is off the data. */
export const px = (d: Shape, c: RenderCtx): [number, number][] | null => {
  const q: [number, number][] = [];
  for (const [t, p] of d.pts) { const x = c.x(t), y = c.y(p); if (x === null || y === null) return null; q.push([x, y]); }
  return q;
};
export const path = (q: readonly (readonly [number, number])[], close = false) => q.map(([x, y], k) => `${k ? "L" : "M"}${x} ${y}`).join("") + (close ? "Z" : "");
