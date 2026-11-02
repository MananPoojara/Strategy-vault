/** Monte Carlo options (AmiBroker Analysis settings tab), shared by Settings and the Monte Carlo panel. Kept per browser. */
import { useSyncExternalStore } from "react";

export type McSizing = "none" | "shares" | "value" | "fraction";
/** Numbers as typed (strings) so a half-typed box is not rewritten; the panel checks them before a run. */
export interface McPrefs {
  enabled: boolean; runs: string; sizing: McSizing; shares: string; value: string; percent: string;
  /** Enable MC equity curves (Min/Max/Avg) and how they are drawn. */
  curves: boolean; scale: "linear" | "log" | "percent";
  logFinal: boolean; negativeDd: boolean;
}
export interface RunPrefs { mc: McPrefs }

// the team's AmiBroker (Chetan's sheet): Monte Carlo on, 1000 runs, trade list
export const RUN_PREFS_DEFAULTS: RunPrefs = {
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
    return { mc: typed(RUN_PREFS_DEFAULTS.mc, s?.mc) };
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
export const setMc = (patch: Partial<McPrefs>) => save({ ...prefs, mc: { ...prefs.mc, ...patch } });
export const resetRunPrefs = () => save(RUN_PREFS_DEFAULTS);
export const useRunPrefs = () => useSyncExternalStore(subscribe, () => prefs);
