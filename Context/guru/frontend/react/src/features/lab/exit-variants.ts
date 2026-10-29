// "Download all exits": the same entry run once per exit variant the user types (N candle 3, 5, 7; ATR 5-1,1; Supertrend 5-1;
// indicator thresholds 55, 70), each variant a trade sheet in one TS workbook. HSL and Profit booking ride on every variant.
import { withExitAtStop } from "./AmiSettings";
import { isValue } from "./spec-model";
import { isIndicatorExit, isSupertrendExit, nCandleNode, nCandleOf } from "./ExitChoice";
import type { RuleNode, Spec } from "./types";

type Obj = Record<string, unknown>;
export type StopMode = "live" | "close" | "both";
export interface ExitPlan {
  ncandle: string; atr: string; supertrend: string; indicator: string;
  /** an exit type switched off keeps what was typed but makes no sheets (absent = on) */
  off?: Partial<Record<"ncandle" | "atr" | "supertrend" | "indicator", boolean>>;
  hsl: { on: boolean; pct: number; mode: StopMode }; target: { on: boolean; pct: number; mode: StopMode };
}
export interface ExitVariant { label: string; tab: string; spec: Spec }
export const MAX_SHEETS = 20; // the template's YOY Summary has 20 columns

const nums = (s: string) => s.split(/[\s,;]+/).map(Number).filter((n) => Number.isFinite(n) && n > 0);
/** "5-1,1; 5-2,2" → [[5, 1, 1], [5, 2, 2]]; "5-1" → [[5, 1]] (period-multiplier[,second multiplier]). */
const pairs = (s: string) => s.split(/[;\n]+|\s{2,}/).map((x) => x.trim()).filter(Boolean).flatMap((x) => {
  const m = /^(\d+(?:\.\d+)?)\s*[-–]\s*(\d+(?:\.\d+)?)(?:\s*,\s*(\d+(?:\.\d+)?))?$/.exec(x);
  return m ? [[Number(m[1]), Number(m[2]), ...(m[3] ? [Number(m[3])] : [])]] : [];
});
const fmt = (n: number) => String(+n.toFixed(4));

/** Every variant the plan names, with HSL / Profit booking added (Both = one sheet with each mode). */
export function exitVariants(base: Spec, plan: ExitPlan): ExitVariant[] {
  if (base.short_side) throw new Error("Download all exits runs one side: set Positions to Long or Short.");
  const short = base.direction === "short", cross = short ? "crosses_below" : "crosses_above";
  const rows = (base.exit.any ?? []) as RuleNode[];
  const clean = (exit: Obj): Spec => ({ ...base, exit: { any: [], ...exit } as Spec["exit"] });
  // each variant inherits the Exit card's settings: ATR timeframe / Avg Val / period end, Supertrend timeframe / calculation,
  // and the N candle count type (any, up close, down close)
  const ex = base.exit as Obj, da = ex.dual_atr as { tf?: string; period_end_only?: boolean; initial?: { price?: string }; favour?: { price?: string } } | undefined;
  const stRow = rows.find(isSupertrendExit) as { lhs?: { p?: Record<string, number>; tf?: string } } | undefined;
  const dl = (ex.dual_line as { initial?: { s?: string; p?: Record<string, number>; tf?: string } } | undefined)?.initial;
  const stBase = stRow?.lhs ?? (dl?.s === "supertrend" ? dl : undefined), stTf = stBase?.tf;
  const nc = rows.map((r) => nCandleOf(r)).find(Boolean);
  const TF: Record<string, string> = { W: " weekly", M: " monthly" }, TAB: Record<string, string> = { W: " W", M: " M" };
  const out: ExitVariant[] = [], use = (k: "ncandle" | "atr" | "supertrend" | "indicator") => (plan.off?.[k] ? "" : plan[k]);
  for (const n of nums(use("ncandle"))) {
    const kind = nc?.kind ?? "any", tf = nc?.tf ?? "D";
    out.push({ label: `N candle ${fmt(n)}${kind === "any" ? "" : ` ${kind} close`}${TF[tf] ?? ""}`, tab: `Ncandle-${fmt(n)}${kind === "any" ? "" : ` ${kind}`}${TAB[tf] ?? ""}`,
      spec: clean({ any: [nCandleNode(kind, Math.round(n), tf)] }) });
  }
  for (const [p, a, b = a] of pairs(use("atr"))) out.push({ label: `ATR (${fmt(p)},${fmt(a)}) (${fmt(p)},${fmt(b)})${TF[da?.tf ?? ""] ?? ""}`, tab: `ATR ${fmt(p)}-${fmt(a)},${fmt(b)}${TAB[da?.tf ?? ""] ?? ""}`,
    spec: clean({ dual_atr: { initial: { period: p, mult: a, price: da?.initial?.price ?? "hl2" }, favour: { period: p, mult: b, price: da?.favour?.price ?? "hl2" },
      ...(da?.tf ? { tf: da.tf } : {}), ...(da?.period_end_only !== undefined ? { period_end_only: da.period_end_only } : {}) } }) });
  for (const [p, f, f2] of pairs(use("supertrend"))) {
    const st = (factor: number) => ({ s: "supertrend", p: { atr_length: p, factor, mode: stBase?.p?.mode ?? 1, price: stBase?.p?.price ?? 0 }, ...(stTf && stTf !== "D" ? { tf: stTf } : {}) });
    out.push(f2 === undefined
      ? { label: `Supertrend (${fmt(p)},${fmt(f)})${TF[stTf ?? ""] ?? ""}`, tab: `ST ${fmt(p)}-${fmt(f)}${TAB[stTf ?? ""] ?? ""}`, spec: clean({ any: [{ lhs: { ...st(f), out: "direction" }, op: cross, rhs: { value: 0 } }] }) }
      : { label: `Supertrend (${fmt(p)},${fmt(f)}) (${fmt(p)},${fmt(f2)})${TF[stTf ?? ""] ?? ""}`, tab: `ST ${fmt(p)}-${fmt(f)},${fmt(f2)}${TAB[stTf ?? ""] ?? ""}`,
        spec: clean({ dual_line: { initial: { ...st(f), out: "supertrend" }, favour: { ...st(f2), out: "supertrend" } } }) });
  }
  // the indicator exit rows of the strategy (not N candle / Supertrend) with a number: one variant per threshold typed
  const ind = rows.filter((r) => isIndicatorExit(r) && !isSupertrendExit(r) && isValue((r as Obj).rhs as never));
  const thresholds = use("indicator").split(/[\s,;]+/).filter(Boolean).map(Number).filter(Number.isFinite);
  for (const t of ind.length ? thresholds : []) {
    out.push({ label: `Indicator exit at ${fmt(t)}`, tab: `IND ${fmt(t)}`, spec: clean({ any: ind.map((r) => ({ ...(r as Obj), rhs: { value: t } }) as RuleNode) }) });
  }
  // HSL / Profit booking on every variant; Both doubles the sheets (live, then close)
  const stops: { key: "stop_loss" | "target"; tag: string; on: boolean; pct: number; mode: StopMode }[] = [
    { key: "stop_loss", tag: "HSL", ...plan.hsl }, { key: "target", tag: "PB", ...plan.target }];
  return stops.reduce((list, s) => !s.on ? list : list.flatMap((v) => (s.mode === "both" ? ["live", "close"] as const : [s.mode]).map((m) => ({
    label: `${v.label} · ${s.tag} ${fmt(s.pct)}% ${m}`, tab: `${v.tab} ${s.tag}${fmt(s.pct)}${s.mode === "both" ? (m === "live" ? "L" : "C") : ""}`,
    spec: withExitAtStop({ ...v.spec, exit: { ...v.spec.exit, [s.key]: { pct: s.pct } } as Spec["exit"] }, s.key, m === "live" ? 1 : 0),
  }))), out);
}
