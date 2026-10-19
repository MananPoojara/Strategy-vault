import { renderToStaticMarkup } from "react-dom/server";
import { StrategyIndicatorControls } from "../components/StrategyIndicatorControls";
import type { IndicatorPoint, StrategyIndicator, StrategyIndicators } from "./strategy-indicators";

export function indicatorControlsHTML(result: StrategyIndicators, collapsible = false): string {
  const controls = <StrategyIndicatorControls result={result} />;
  return renderToStaticMarkup(collapsible && result.status === "ready" ? (
    <details className="indicator-disclosure" data-indicator-disclosure>
      <summary>
        <strong>Indicators</strong>
        <span>{result.series.map(series => series.label).join(" · ")}</span>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden="true"><path d="m9 5 7 7-7 7" /></svg>
      </summary>
      {controls}
    </details>
  ) : controls);
}

type LogicalRange = { from: number; to: number } | null;
interface TimeScale {
  getVisibleLogicalRange(): LogicalRange;
  setVisibleLogicalRange(range: Exclude<LogicalRange, null>): void;
  subscribeVisibleLogicalRangeChange(listener: (range: LogicalRange) => void): void;
  unsubscribeVisibleLogicalRangeChange(listener: (range: LogicalRange) => void): void;
}
interface Crosshair { time?: string | number | { year: number; month: number; day: number } }
interface LineSeries {
  setData(points: ({ time: string; value: number } | { time: string })[]): void;
  applyOptions(options: Record<string, unknown>): void;
  createPriceLine(options: Record<string, unknown>): unknown;
}
export interface IndicatorChart {
  addLineSeries(options: Record<string, unknown>): LineSeries;
  removeSeries(series: LineSeries): void;
  timeScale(): TimeScale;
  subscribeCrosshairMove(listener: (event: Crosshair) => void): void;
  unsubscribeCrosshairMove(listener: (event: Crosshair) => void): void;
  setCrosshairPosition?(value: number, time: string, series: unknown): void;
  clearCrosshairPosition?(): void;
  resize(width: number, height: number): void;
  remove(): void;
}
export interface IndicatorChartLibrary {
  createChart(element: HTMLElement, options: Record<string, unknown>): IndicatorChart;
}
interface IndicatorColors { trend: string; ma: string; rsi: string; di: string; entry: string; exit: string; reference: string }
export interface IndicatorMount {
  root: HTMLElement | null;
  chart: IndicatorChart;
  priceSeries?: unknown;
  result: StrategyIndicators;
  library: IndicatorChartLibrary;
  /** Existing chart theme options; width/height are set independently per pane. */
  chartOptions?: Record<string, unknown>;
  colors?: Partial<IndicatorColors>;
  /** Price closes at displayed candle dates, for pane-to-price crosshair sync. */
  pricePoints?: readonly IndicatorPoint[];
}
export interface IndicatorController {
  dispose(): void;
  setVisible(key: string, visible: boolean): void;
}

function dateString(time: Crosshair["time"]): string | null {
  if (typeof time === "string") return time;
  if (time && typeof time === "object")
    return `${time.year}-${String(time.month).padStart(2, "0")}-${String(time.day).padStart(2, "0")}`;
  return null;
}

/** Lightweight Charts v4 companion panes. Every line has the same dates as the
 * main candles, including whitespace during warmup, so logical ranges stay aligned.
 * Call dispose before replacing the main chart, period, source or timeframe. */
export function mountStrategyIndicators(options: IndicatorMount): IndicatorController {
  const { root, chart, result, library } = options;
  if (!root || result.status !== "ready") return { dispose() {}, setVisible() {} };
  const css = getComputedStyle(root);
  const color = (name: string, fallback: string) => css.getPropertyValue(name).trim() || fallback;
  const colors: IndicatorColors = {
    trend: color("--blue", "#2f5a8f"), ma: color("--bh", "#8b738f"),
    rsi: color("--blue", "#2f5a8f"), di: color("--green", "#2e7d4f"),
    entry: color("--green", "#2e7d4f"), exit: color("--red", "#c43c3c"),
    reference: color("--ink3", "#8b8d93"), ...options.colors,
  };
  const series = new Map<string, { definition: StrategyIndicator; line: LineSeries; chart: IndicatorChart; pane: HTMLElement | null; element: HTMLElement | null; values: Map<string, number | null> }>();
  const paneCharts: IndicatorChart[] = [];
  const disclosure = root.closest<HTMLDetailsElement>("details[data-indicator-disclosure]");
  const preferences = new Map<string, boolean>();
  let disposed = false, synchronizingRange = false, synchronizingCrosshair = false;
  const lineColor = (definition: StrategyIndicator) => definition.key === "upper" ? colors.reference
    : definition.key === "lower" ? colors.di
    : ["ma", "slow", "macd-signal", "slow-rsi", "percent-b", "rsi-sum"].includes(definition.key)
      ? colors.ma : definition.kind === "di" ? colors.di : definition.kind === "sma" ? colors.trend : colors.rsi;
  for (const definition of result.series) {
    const element = definition.placement === "pane" ? root.querySelector<HTMLElement>(`[data-indicator-pane="${definition.key}"]`) : null;
    const pane = definition.placement === "pane" ? root.querySelector<HTMLElement>(`[data-indicator-pane-wrap="${definition.key}"]`) : null;
    if (definition.placement === "pane" && !element) continue;
    const ownChart = element ? library.createChart(element, {
      ...options.chartOptions, width: element.clientWidth || root.clientWidth || 600,
      height: element.clientHeight || 145,
      rightPriceScale: { borderColor: color("--steel", "#b9b9ba"), minimumWidth: 60 },
    }) : chart;
    if (element) paneCharts.push(ownChart);
    const line = ownChart.addLineSeries({ color: lineColor(definition), lineWidth: 2,
      priceLineVisible: false, lastValueVisible: false,
      crosshairMarkerVisible: true, priceFormat: { type: "price", precision: 2, minMove: 0.01 } });
    line.setData(definition.points.map(({ time, value }) => value === null ? { time } : { time, value }));
    for (const threshold of definition.thresholds) line.createPriceLine({
      price: threshold.value, color: colors[threshold.kind], lineWidth: 1,
      lineStyle: 2, axisLabelVisible: true, title: threshold.label,
    });
    series.set(definition.key, { definition, line, chart: ownChart, pane, element,
      values: new Map(definition.points.map(({ time, value }) => [time, value])) });
    const toggle = root.querySelector<HTMLInputElement>(`[data-indicator-toggle="${definition.key}"]`);
    preferences.set(definition.key, toggle ? toggle.checked : true);
    if (toggle && !toggle.checked) { line.applyOptions({ visible: false }); if (pane) pane.hidden = true; }
    if (!definition.points.some((point) => point.value !== null)) { line.applyOptions({ visible: false }); if (pane) pane.hidden = true; }
  }
  const allCharts = [chart, ...paneCharts];
  const rangeListeners = allCharts.map((ownChart) => {
    const listener = (range: LogicalRange) => {
      if (disposed || synchronizingRange || !range) return;
      synchronizingRange = true;
      try { for (const other of allCharts) if (other !== ownChart) other.timeScale().setVisibleLogicalRange(range); }
      finally { synchronizingRange = false; }
    };
    ownChart.timeScale().subscribeVisibleLogicalRangeChange(listener);
    return { chart: ownChart, listener };
  });
  const priceValues = new Map(options.pricePoints?.map(({ time, value }) => [time, value]) || []);
  const crosshairListeners = allCharts.map((ownChart) => {
    const listener = (event: Crosshair) => {
      if (disposed || synchronizingCrosshair) return;
      synchronizingCrosshair = true;
      try {
        const date = dateString(event.time);
        for (const [key, state] of series) {
          const value = date ? state.values.get(date) : undefined;
          const output = root.querySelector<HTMLOutputElement>(`[data-indicator-value="${key}"]`);
          if (output) output.textContent = value == null ? "" : value.toFixed(2);
          if (state.chart !== ownChart && state.chart !== chart) {
            if (date && value != null) state.chart.setCrosshairPosition?.(value, date, state.line);
            else state.chart.clearCrosshairPosition?.();
          }
        }
        if (ownChart !== chart && options.priceSeries) {
          const value = date ? priceValues.get(date) : undefined;
          if (date && value != null) chart.setCrosshairPosition?.(value, date, options.priceSeries);
          else chart.clearCrosshairPosition?.();
        }
      } finally { synchronizingCrosshair = false; }
    };
    ownChart.subscribeCrosshairMove(listener);
    return { chart: ownChart, listener };
  });
  const currentRange = chart.timeScale().getVisibleLogicalRange();
  if (currentRange) for (const ownChart of paneCharts) ownChart.timeScale().setVisibleLogicalRange(currentRange);
  const resize = () => {
    if (disposed) return;
    for (const state of series.values()) if (state.element && !state.pane?.hidden)
      state.chart.resize(state.element.clientWidth || root.clientWidth || 600, state.element.clientHeight || 145);
  };
  const observer = typeof ResizeObserver === "function" ? new ResizeObserver(resize) : null;
  observer?.observe(root);
  const applyVisibility = (key: string) => {
    if (disposed) return;
    const state = series.get(key); if (!state) return;
    const visible = preferences.get(key) === true && (!disclosure || disclosure.open)
      && state.definition.points.some(point => point.value !== null);
    state.line.applyOptions({ visible });
    if (state.pane) state.pane.hidden = !visible;
  };
  const setVisible = (key: string, visible: boolean) => {
    if (disposed || !series.has(key)) return;
    preferences.set(key, visible);
    applyVisibility(key);
    const toggle = root.querySelector<HTMLInputElement>(`[data-indicator-toggle="${key}"]`);
    if (toggle) toggle.checked = visible;
    resize();
  };
  const change = (event: Event) => {
    const target = event.target;
    if (target instanceof HTMLInputElement && target.dataset.indicatorToggle) {
      event.stopPropagation(); setVisible(target.dataset.indicatorToggle, target.checked);
    }
  };
  root.addEventListener("change", change);
  const disclosureChanged = () => {
    for (const key of series.keys()) applyVisibility(key);
    resize();
  };
  if (disclosure) {
    disclosure.addEventListener("toggle", disclosureChanged);
    disclosureChanged();
  }
  return {
    setVisible,
    dispose() {
      if (disposed) return; disposed = true;
      root.removeEventListener("change", change); observer?.disconnect();
      disclosure?.removeEventListener("toggle", disclosureChanged);
      for (const state of rangeListeners) state.chart.timeScale().unsubscribeVisibleLogicalRangeChange(state.listener);
      for (const state of crosshairListeners) state.chart.unsubscribeCrosshairMove(state.listener);
      for (const state of series.values()) if (state.chart === chart) chart.removeSeries(state.line);
      for (const ownChart of paneCharts) ownChart.remove();
      series.clear();
    },
  };
}
