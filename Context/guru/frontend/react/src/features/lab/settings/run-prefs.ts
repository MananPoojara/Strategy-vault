/** Walk-forward and Monte Carlo options (AmiBroker Analysis settings tabs), shared by Settings and the Optimize / Monte Carlo panels. Kept per browser. */
import { useEffect, useState, useSyncExternalStore } from "react";
import type { WalkforwardOptions } from "../types";

export type McSizing = "none" | "shares" | "value" | "fraction";
/** Numbers as typed (strings) so a half-typed box is not rewritten; the panel checks them before a run. */
export interface McPrefs {
  enabled: boolean; runs: string; sizing: McSizing; shares: string; value: string; percent: string;
  /** Enable MC equity curves (Min/Max/Avg) and how they are drawn. */
  curves: boolean; scale: "linear" | "log" | "percent";
  logFinal: boolean; negativeDd: boolean;
}
export interface RunPrefs { wf: WalkforwardOptions; mc: McPrefs }

// the team's AmiBroker (Chetan's sheet): 4-year in-sample, 1-year out-of-sample and step, CAR/MDD; Monte Carlo on, 1000 runs, trade list
export const RUN_PREFS_DEFAULTS: RunPrefs = {
  wf: { is: 4, oos: 1, step: 1, unit: "years", anchored: false, target: "mar" },
  mc: { enabled: true, runs: "1000", sizing: "none", shares: "100", value: "1000", percent: "5", curves: true, scale: "linear", logFinal: false, negativeDd: true },
};

const KEY = "lab.runPrefs";
/** Saved values replace a default only when they have its type (an old or edited value falls back). */
const typed = <T extends object>(base: T, saved: unknown): T => Object.fromEntries(Object.entries(base).map(([k, v]) => {
  const s = (saved as Record<string, unknown> | null)?.[k];
  return [k, typeof s === typeof v ? s : v];
})) as T;
function load(): RunPrefs {
  try {
    const s = JSON.parse(localStorage.getItem(KEY) ?? "null") as Partial<RunPrefs> | null;
    return { wf: typed(RUN_PREFS_DEFAULTS.wf, s?.wf), mc: typed(RUN_PREFS_DEFAULTS.mc, s?.mc) };
  } catch { return RUN_PREFS_DEFAULTS; }
}

let prefs = load();
const listeners = new Set<() => void>();
const subscribe = (f: () => void) => { listeners.add(f); return () => { listeners.delete(f); }; };
function save(next: RunPrefs) {
  prefs = next;
  try { localStorage.setItem(KEY, JSON.stringify(next)); } catch { /* private window: kept for this visit */ }
  listeners.forEach((f) => f());
}
export const setWf = (patch: Partial<WalkforwardOptions>) => save({ ...prefs, wf: { ...prefs.wf, ...patch } });
export const setMc = (patch: Partial<McPrefs>) => save({ ...prefs, mc: { ...prefs.mc, ...patch } });
export const resetRunPrefs = () => save(RUN_PREFS_DEFAULTS);
export const useRunPrefs = () => useSyncExternalStore(subscribe, () => prefs);

/** Lengths are whole numbers of 1 or more and the step covers the out-of-sample window (backend walkforward.rs). */
export const wfBad = (o: WalkforwardOptions) => [o.is, o.oos, o.step].some((n) => !Number.isInteger(n) || n < 1) || o.step < o.oos;

/** The walk-forward options as edited in one view: a bad combination stays local until fixed, so the other view and the store only see runnable options. */
export function useWfDraft(): [WalkforwardOptions, (patch: Partial<WalkforwardOptions>) => void] {
  const stored = useRunPrefs().wf;
  const [o, setO] = useState(stored);
  useEffect(() => setO(stored), [stored]);
  return [o, (patch) => { const next = { ...o, ...patch }; setO(next); if (!wfBad(next)) setWf(next); }];
}
