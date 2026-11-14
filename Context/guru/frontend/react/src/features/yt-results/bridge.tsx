// The YT strategy page is markup from the runtime; its Results and optimization views are these React components, mounted in
// the page's "yt-results" and "yt-optimize" slots. The runtime hands them their data through this small store.
import { useState, useSyncExternalStore } from "react";
import type { YtOptimizeContext } from "./YtOptimize";
import { saveOptimization, YtOptimize } from "./YtOptimize";
import type { YtOptimizeResult } from "./optimize-sheet";
import { saveBacktest, saveTs, YtResults, type DailyBar } from "./YtResults";
import type { YtRules, YtRunResult } from "./types";

export interface YtResultsView { result: YtRunResult; rules: YtRules; loadBars(): Promise<DailyBar[]>; onBack?: () => void }
export type YtOptimizeView = { result: YtOptimizeResult; ctx: YtOptimizeContext } | null;

let results: YtResultsView | null = null, optimize: YtOptimizeView = null;
const listeners = new Set<() => void>();
const notify = () => listeners.forEach((l) => l());
const subscribe = (l: () => void) => { listeners.add(l); return () => { listeners.delete(l); }; };

export const ytBridge = {
  setResults(v: YtResultsView | null) { results = v; notify(); },
  setOptimize(v: YtOptimizeView) { optimize = v; notify(); },
};

export function YtResultsSlot() {
  const v = useSyncExternalStore(subscribe, () => results);
  return v ? <YtResults {...v} /> : null;
}
export function YtOptimizeSlot() {
  const v = useSyncExternalStore(subscribe, () => optimize);
  return v ? <YtOptimize result={v.result} ctx={v.ctx} /> : null;
}
/** "Download backtest results" (Settings, Statistics, Trades and YOY-Monthly in one workbook) is switched off for now;
 *  true brings the button back beside Download TS. */
const SHOW_BACKTEST_RESULTS = false;

/** At the bottom right of the strategy's header box: Download TS, the same tradesheet as the Results' Trades tab
 *  (the team template at its default format and costs) for the run shown on the page. */
export function YtDownloadSlot() {
  const v = useSyncExternalStore(subscribe, () => results);
  const [busy, setBusy] = useState(false), [error, setError] = useState("");
  if (!v) return null;
  const ts = async () => {
    setBusy(true); setError("");
    try { await saveTs(v.result, v.rules, v.loadBars); } catch (e) { setError(e instanceof Error ? e.message : String(e)); } finally { setBusy(false); }
  };
  return <span className="yt-downloads">
    {error && <span className="rnote neg" role="alert">{error}</span>}
    {SHOW_BACKTEST_RESULTS && <button type="button" className="dbtn sm" data-yt-download-results
      title="One Excel file: Settings, Statistics (AmiBroker report), Trades and YOY-Monthly of the backtest shown on this page"
      onClick={() => saveBacktest(v.result, v.rules)}>Download backtest results</button>}
    <button type="button" className="dbtn sm primary" data-yt-download-ts disabled={busy}
      title="The team tradesheet of the backtest shown on this page: the same file as Download TS in the Results' Trades tab"
      onClick={() => void ts()}>{busy ? "Preparing…" : "Download TS"}</button>
  </span>;
}
/** Beside Run optimization: downloads the optimization results (Rules and every combination, sorted on CAR/MDD);
 *  greyed until an optimization has run on this page. */
export function YtOptimizeDownloadSlot() {
  const v = useSyncExternalStore(subscribe, () => optimize);
  const ready = !!v?.result.sheet && !!v.result.sheet_cols;
  return <button type="button" className="dbtn sm" data-yt-optimize-download disabled={!ready}
    title={ready ? "Excel with the Rules sheet and every combination tested, sorted on CAR/MDD, with the team's Final columns" : "Run the optimization first"}
    onClick={() => { if (v) saveOptimization(v.result, v.ctx); }}>Download optimization results</button>;
}
