import { useSyncExternalStore } from "react";
import type { LabChartApi, LabChartLibrary, LabSeriesApi } from "./globals";

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

/** A translucent version of a theme colour (#rrggbb or rgb()); any other format gives no tint rather than a solid fill. */
export function tint(color: string, alpha: number) {
  if (/^#[0-9a-f]{6}$/i.test(color)) return color + Math.round(alpha * 255).toString(16).padStart(2, "0");
  const rgb = /^rgb\((.+)\)$/.exec(color);
  return rgb ? `rgba(${rgb[1]}, ${alpha})` : "transparent";
}

/** Lab pages bypass VaultRuntime.cleanup, so every chart owns its resize observer and removal. */
export function createLabChart(el: HTMLElement, height: number, options: Record<string, unknown> = {}, library: LabChartLibrary | undefined = window.LightweightCharts) {
  if (!library) return null;
  const colors = themeColors(el);
  const chart: LabChartApi = library.createChart(el, {
    width: el.clientWidth || 600, height,
    layout: { attributionLogo: false, background: { color: colors.panel }, textColor: colors.ink3,
      fontFamily: getComputedStyle(document.body).fontFamily, fontSize: 11 },
    grid: { vertLines: { visible: false }, horzLines: { color: colors.rule } },
    rightPriceScale: { borderColor: colors.steel }, timeScale: { borderColor: colors.steel, minBarSpacing: 0.05 },
    crosshair: { mode: library.CrosshairMode?.Normal ?? 0 }, handleScale: { axisPressedMouseMove: false },
    ...options,
  });
  const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(() => {
    // A resize changes the bars shown; keep the time range the user (or the preset) chose.
    const range = chart.timeScale().getVisibleRange?.();
    chart.resize(el.clientWidth, height);
    if (range) chart.timeScale().setVisibleRange(range);
  });
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

export interface SyncedPane { chart: LabChartApi; el: HTMLElement; series: LabSeriesApi }
/** Keeps every pane on one visible time range and one crosshair bar; only the pane under the pointer drives the others, so updates cannot loop. */
export function syncPanes(panes: SyncedPane[]) {
  let active: SyncedPane | null = null;
  const undo: (() => void)[] = [];
  for (const pane of panes) {
    const claim = () => { active = pane; };
    pane.el.addEventListener("pointerenter", claim); pane.el.addEventListener("pointerdown", claim); pane.el.addEventListener("wheel", claim);
    undo.push(() => { pane.el.removeEventListener("pointerenter", claim); pane.el.removeEventListener("pointerdown", claim); pane.el.removeEventListener("wheel", claim); });
    const onRange = (range: { from: string; to: string } | null) => {
      if (active !== pane || !range) return;
      for (const other of panes) if (other !== pane) other.chart.timeScale().setVisibleRange(range);
    };
    const onCross = (param: { time?: string }) => {
      if (active !== pane) return;
      for (const other of panes) {
        if (other === pane) continue;
        if (param.time) other.chart.setCrosshairPosition?.(0, param.time, other.series); else other.chart.clearCrosshairPosition?.();
      }
    };
    pane.chart.timeScale().subscribeVisibleTimeRangeChange?.(onRange);
    pane.chart.subscribeCrosshairMove?.(onCross);
    undo.push(() => { pane.chart.timeScale().unsubscribeVisibleTimeRangeChange?.(onRange); pane.chart.unsubscribeCrosshairMove?.(onCross); });
  }
  return () => undo.forEach((u) => u());
}
