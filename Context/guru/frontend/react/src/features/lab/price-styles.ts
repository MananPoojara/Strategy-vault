/** TradingView's time-based chart types for the price pane (the date-free ones, Renko … Range, are in timeless.ts). Each draws the
 *  same daily bars, so indicators, drawings, trades and profiles keep working on every style. */
import type { LabChartApi, LabSeriesApi } from "./globals";
import { tint, type ChartColors } from "./charts";

export type PriceStyle = "candles" | "bars" | "hollow" | "volume_candles" | "line" | "line_markers" | "step" | "area" | "hlc_area" | "baseline" | "columns" | "high_low";
export const PRICE_STYLES: [PriceStyle, string][] = [["bars", "Bars"], ["candles", "Candles"], ["hollow", "Hollow candles"], ["volume_candles", "Volume candles"],
  ["line", "Line"], ["line_markers", "Line with markers"], ["step", "Step line"], ["area", "Area"], ["hlc_area", "HLC area"], ["baseline", "Baseline"],
  ["columns", "Columns"], ["high_low", "High-low"]];
export const isPriceStyle = (v: string): v is PriceStyle => PRICE_STYLES.some(([s]) => s === v);
/** Styles drawn from open/high/low/close candles (TPO can hide them by clearing their colours). */
export const CANDLE_LIKE: ReadonlySet<PriceStyle> = new Set(["candles", "hollow", "bars", "high_low", "volume_candles"]);

export interface PriceBars { d: string[]; o: number[]; h: number[]; l: number[]; c: number[]; v: (number | null)[] }
// TradingView's line-style default blue
const TV_BLUE = "#2962ff";

type Bar = { x: number; originalData: { open: number; high: number; low: number; close: number; vol?: number } };
type View = { bars: readonly Bar[]; barSpacing: number; visibleRange: { from: number; to: number } | null };
type Target = { useMediaCoordinateSpace(f: (scope: { context: CanvasRenderingContext2D }) => void): void };

/** A custom series drawing each bar with `paint` (volume candles, HLC area): Lightweight Charts' plugin API. */
function custom(chart: LabChartApi, colors: ChartColors, paint: (g: CanvasRenderingContext2D, v: View, y: (p: number) => number) => void) {
  let view: View | null = null;
  return chart.addCustomSeries?.({
    renderer: () => ({ draw: (t: Target, toY: (p: number) => number | null) => t.useMediaCoordinateSpace(({ context }) => view?.visibleRange && paint(context, view, (p) => toY(p) ?? NaN)) }),
    update: (d: View) => { view = d; },
    priceValueBuilder: (d: Bar["originalData"]) => [d.low, d.high, d.close],
    isWhitespace: (d: Partial<Bar["originalData"]>) => typeof d.close !== "number",
    defaultOptions: () => ({ color: colors.ink }),
  }, { priceLineVisible: false, lastValueVisible: true });
}

/** Adds the price series in `style` and fills it from the bars. */
export function addPriceSeries(chart: LabChartApi, style: PriceStyle, px: PriceBars, colors: ChartColors, viewFrom = 0): LabSeriesApi {
  const { green, red } = colors, n = px.d.length;
  const ohlc = (i: number) => ({ time: px.d[i], open: px.o[i], high: px.h[i], low: px.l[i], close: px.c[i] });
  const close = (i: number) => ({ time: px.d[i], value: px.c[i] });
  const all = <T>(f: (i: number) => T) => Array.from({ length: n }, (_, i) => f(i));
  const candle = { upColor: green, borderUpColor: green, wickUpColor: green, downColor: red, borderDownColor: red, wickDownColor: red, priceLineVisible: false };
  const line = { color: TV_BLUE, lineWidth: 2, priceLineVisible: false };
  let s: LabSeriesApi | undefined;
  switch (style) {
    case "bars":
      s = chart.addBarSeries?.({ upColor: green, downColor: red, openVisible: true, thinBars: false, priceLineVisible: false });
      s?.setData(all(ohlc)); break;
    case "hollow": {
      // TradingView: the colour follows the close against the previous close; the body is hollow when the close is above the open
      s = chart.addCandlestickSeries({ ...candle });
      s.setData(all((i) => {
        const tone = i && px.c[i] < px.c[i - 1] ? red : green;
        return { ...ohlc(i), color: px.c[i] >= px.o[i] ? "rgba(0,0,0,0)" : tone, borderColor: tone, wickColor: tone };
      }));
      break;
    }
    case "volume_candles": {
      // TradingView: each candle's width follows its volume against the largest volume in view
      s = custom(chart, colors, (g, v, y) => {
        const { from, to } = v.visibleRange!;
        let most = 0;
        for (let i = from; i < to; i++) most = Math.max(most, v.bars[i].originalData.vol ?? 0);
        for (let i = from; i < to; i++) {
          const { x, originalData: d } = v.bars[i], tone = d.close >= d.open ? green : red;
          const w = Math.max(1, v.barSpacing * 0.95 * (most > 0 && d.vol ? Math.max(0.15, d.vol / most) : 0.5));
          g.fillStyle = g.strokeStyle = tone; g.lineWidth = 1;
          g.beginPath(); g.moveTo(x, y(d.high)); g.lineTo(x, y(d.low)); g.stroke();
          const top = y(Math.max(d.open, d.close));
          g.fillRect(x - w / 2, top, w, Math.max(1, y(Math.min(d.open, d.close)) - top));
        }
      });
      s?.setData(all((i) => ({ ...ohlc(i), vol: px.v[i] ?? 0 })));
      break;
    }
    case "line": s = chart.addLineSeries({ ...line }); s.setData(all(close)); break;
    case "line_markers": s = chart.addLineSeries({ ...line, pointMarkersVisible: true, pointMarkersRadius: 3 }); s.setData(all(close)); break;
    case "step": s = chart.addLineSeries({ ...line, lineType: 1 }); s.setData(all(close)); break;
    case "area": s = chart.addAreaSeries({ lineColor: TV_BLUE, lineWidth: 2, topColor: tint(TV_BLUE, 0.28), bottomColor: tint(TV_BLUE, 0.04), priceLineVisible: false }); s.setData(all(close)); break;
    case "hlc_area": {
      // TradingView: high line green, low line red, close line grey; filled green between close and high, red between close and low
      s = custom(chart, colors, (g, v, y) => {
        const { from, to } = v.visibleRange!, pts = v.bars.slice(from, to);
        const band = (edge: "high" | "low", fill: string) => {
          g.beginPath(); pts.forEach((b, k) => (k ? g.lineTo : g.moveTo).call(g, b.x, y(b.originalData[edge])));
          for (let k = pts.length - 1; k >= 0; k--) g.lineTo(pts[k].x, y(pts[k].originalData.close));
          g.closePath(); g.fillStyle = fill; g.fill();
        };
        const stroke = (key: "high" | "low" | "close", color: string) => { g.beginPath(); pts.forEach((b, k) => (k ? g.lineTo : g.moveTo).call(g, b.x, y(b.originalData[key]))); g.strokeStyle = color; g.lineWidth = key === "close" ? 1.5 : 1; g.stroke(); };
        band("high", tint(green, 0.2)); band("low", tint(red, 0.2));
        stroke("high", green); stroke("low", red); stroke("close", colors.ink2);
      });
      s?.setData(all(ohlc));
      break;
    }
    case "baseline": {
      // TradingView's base level sits at 50% of the price range in view (the range preset's bars): green above it, red below
      const shown = (a: number[]) => a.slice(viewFrom).filter(Number.isFinite), lo = Math.min(...shown(px.l)), hi = Math.max(...shown(px.h));
      s = chart.addBaselineSeries?.({ baseValue: { type: "price", price: (lo + hi) / 2 }, topLineColor: green, bottomLineColor: red,
        topFillColor1: tint(green, 0.28), topFillColor2: tint(green, 0.05), bottomFillColor1: tint(red, 0.05), bottomFillColor2: tint(red, 0.28), lineWidth: 2, priceLineVisible: false });
      s?.setData(all(close)); break;
    }
    case "columns": {
      // TradingView: a column per bar from the bottom of the pane up to its close, green when the close rose, red when it fell;
      // the scale follows the closes (a histogram from 0 would flatten them)
      s = custom(chart, colors, (g, v, y) => {
        const { from, to } = v.visibleRange!, bottom = g.canvas.height / (window.devicePixelRatio || 1), w = Math.max(1, v.barSpacing * 0.8);
        for (let i = from; i < to; i++) {
          const { x, originalData: d } = v.bars[i], prev = i ? v.bars[i - 1].originalData.close : d.close, top = y(d.close);
          g.fillStyle = tint(d.close < prev ? red : green, 0.7); g.fillRect(x - w / 2, top, w, Math.max(1, bottom - top));
        }
      });
      s?.setData(all((i) => ({ time: px.d[i], open: px.c[i], high: px.c[i], low: px.c[i], close: px.c[i] })));
      break;
    }
    case "high_low":
      // TradingView: a box from each bar's low to its high, no open or close
      s = chart.addCandlestickSeries({ ...candle, upColor: tint(TV_BLUE, 0.6), downColor: tint(TV_BLUE, 0.6), borderUpColor: TV_BLUE, borderDownColor: TV_BLUE, wickVisible: false });
      s.setData(all((i) => ({ time: px.d[i], open: px.l[i], close: px.h[i], high: px.h[i], low: px.l[i] })));
      break;
    default:
      s = chart.addCandlestickSeries(candle); s.setData(all(ohlc));
  }
  // an engine without the plugin API or a series type falls back to candles
  if (!s) { s = chart.addCandlestickSeries(candle); s.setData(all(ohlc)); }
  return s;
}
