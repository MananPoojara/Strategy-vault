import { useSyncExternalStore } from "react";
import type { LabChartApi, LabChartLibrary } from "./globals";

export interface ChartColors { ink: string; ink3: string; rule: string; steel: string; panel: string; chart: string; bh: string; green: string; red: string; blue: string }
export const PALETTE: (keyof ChartColors)[] = ["blue", "bh", "green", "red", "chart", "ink3"];

export function themeColors(el: HTMLElement): ChartColors {
  const css = getComputedStyle(el);
  const c = (name: string, fallback: string) => css.getPropertyValue(name).trim() || fallback;
  return {
    ink: c("--ink", "#1d1f23"), ink3: c("--ink3", "#8b8d93"), rule: c("--rule", "#e3e3e4"), steel: c("--steel", "#b9b9ba"),
    panel: c("--panel", "#f3f3f3"), chart: c("--chart", "#2f5a8f"), bh: c("--bh", "#8b738f"), green: c("--green", "#2e7d4f"),
    red: c("--red", "#c43c3c"), blue: c("--blue", "#2f5a8f"),
  };
}

/** Lab pages bypass VaultRuntime.cleanup, so every chart owns its resize observer and removal. */
export function createLabChart(el: HTMLElement, height: number, options: Record<string, unknown> = {}, library: LabChartLibrary | undefined = window.LightweightCharts) {
  if (!library) return null;
  const colors = themeColors(el);
  const chart: LabChartApi = library.createChart(el, {
    width: el.clientWidth || 600, height,
    layout: { background: { color: colors.panel }, textColor: colors.ink, fontFamily: "IBM Plex Mono, monospace" },
    grid: { vertLines: { color: colors.rule }, horzLines: { color: colors.rule } },
    rightPriceScale: { borderColor: colors.steel }, timeScale: { borderColor: colors.steel },
    ...options,
  });
  const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(() => chart.resize(el.clientWidth, height));
  observer?.observe(el);
  return { chart, colors, dispose() { observer?.disconnect(); chart.remove(); } };
}

function subscribe(onChange: () => void) {
  const observer = new MutationObserver(onChange);
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
  return () => observer.disconnect();
}
/** Chart colours are read at creation; effects that depend on this key remount on a theme change. */
export const useThemeKey = () => useSyncExternalStore(subscribe, () => document.documentElement.getAttribute("data-theme") ?? "");
