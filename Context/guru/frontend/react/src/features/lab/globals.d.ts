/** Minimal typings for vendored globals loaded by index.html (no npm dependency). */
export interface LabSeriesApi {
  setData(points: unknown[]): void;
  setMarkers?(markers: unknown[]): void;
  applyOptions(options: Record<string, unknown>): void;
}
export interface LabChartApi {
  addCandlestickSeries(options: Record<string, unknown>): LabSeriesApi;
  addLineSeries(options: Record<string, unknown>): LabSeriesApi;
  addAreaSeries(options: Record<string, unknown>): LabSeriesApi;
  timeScale(): {
    fitContent(): void; getVisibleRange?(): { from: string; to: string } | null; setVisibleRange(range: { from: string; to: string }): void;
    subscribeVisibleTimeRangeChange?(handler: (range: { from: string; to: string } | null) => void): void;
    unsubscribeVisibleTimeRangeChange?(handler: (range: { from: string; to: string } | null) => void): void;
  };
  subscribeCrosshairMove?(handler: (param: { time?: string }) => void): void;
  unsubscribeCrosshairMove?(handler: (param: { time?: string }) => void): void;
  setCrosshairPosition?(price: number, time: string, series: LabSeriesApi): void;
  clearCrosshairPosition?(): void;
  applyOptions(options: Record<string, unknown>): void;
  resize(width: number, height: number): void;
  remove(): void;
}
export interface LabChartLibrary { createChart(element: HTMLElement, options: Record<string, unknown>): LabChartApi }
export interface VaultReportMonthly {
  months: Record<string, number | null>; annual: Record<string, number | null>; years: string[];
  averages: (number | null)[]; annualAverage: number | null;
}
export interface VaultReportApi {
  monthly(d: { dates: string[]; equity: number[] }): VaultReportMonthly;
  csv(rows: unknown[][]): string;
}
declare global {
  interface Window { LightweightCharts?: LabChartLibrary; VaultReport?: VaultReportApi }
}
