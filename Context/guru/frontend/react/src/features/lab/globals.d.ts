/** Minimal typings for vendored globals loaded by index.html (no npm dependency). */
export interface LabSeriesApi {
  setData(points: unknown[]): void;
  setMarkers?(markers: unknown[]): void;
  applyOptions(options: Record<string, unknown>): void;
  createPriceLine?(options: Record<string, unknown>): unknown;
  removePriceLine?(line: unknown): void;
  coordinateToPrice?(y: number): number | null;
}
export interface LabChartApi {
  addCandlestickSeries(options: Record<string, unknown>): LabSeriesApi;
  addLineSeries(options: Record<string, unknown>): LabSeriesApi;
  addAreaSeries(options: Record<string, unknown>): LabSeriesApi;
  addHistogramSeries?(options: Record<string, unknown>): LabSeriesApi;
  priceScale?(id: string): { applyOptions(options: Record<string, unknown>): void; width?(): number };
  timeScale(): {
    fitContent(): void; getVisibleRange?(): { from: string; to: string } | null; setVisibleRange(range: { from: string; to: string }): void;
    getVisibleLogicalRange?(): { from: number; to: number } | null; setVisibleLogicalRange?(range: { from: number; to: number }): void;
    subscribeVisibleTimeRangeChange?(handler: (range: { from: string; to: string } | null) => void): void;
    unsubscribeVisibleTimeRangeChange?(handler: (range: { from: string; to: string } | null) => void): void;
    subscribeVisibleLogicalRangeChange?(handler: (range: { from: number; to: number } | null) => void): void;
    unsubscribeVisibleLogicalRangeChange?(handler: (range: { from: number; to: number } | null) => void): void;
  };
  subscribeCrosshairMove?(handler: (param: { time?: string; point?: { x: number; y: number } }) => void): void;
  unsubscribeCrosshairMove?(handler: (param: { time?: string; point?: { x: number; y: number } }) => void): void;
  setCrosshairPosition?(price: number, time: string, series: LabSeriesApi): void;
  clearCrosshairPosition?(): void;
  subscribeClick?(handler: (param: { time?: string; point?: { x: number; y: number } }) => void): void;
  unsubscribeClick?(handler: (param: { time?: string; point?: { x: number; y: number } }) => void): void;
  removeSeries?(series: LabSeriesApi): void;
  applyOptions(options: Record<string, unknown>): void;
  resize(width: number, height: number): void;
  remove(): void;
}
export interface LabChartLibrary { createChart(element: HTMLElement, options: Record<string, unknown>): LabChartApi; CrosshairMode?: { Normal: number } }
export interface VaultReportMonthly {
  months: Record<string, number | null>; annual: Record<string, number | null>; years: string[];
  averages: (number | null)[]; annualAverage: number | null;
}
export interface VaultReportApi {
  monthly(d: { dates: string[]; equity: number[] }): VaultReportMonthly;
  csv(rows: unknown[][]): string;
}
export interface VaultTruthApi {
  actionMarkers?(trades: { dir: number; legs: [number, number, number][]; exit_i: number; open: boolean }[], dates: string[], barOf: number[], colors: { buy: string; sell: string }):
    { time: string; position: string; shape: string; color: string; size: number }[];
}
declare global {
  interface Window { LightweightCharts?: LabChartLibrary; VaultReport?: VaultReportApi; VaultTruth?: VaultTruthApi }
}
