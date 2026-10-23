import { isSeries, getAt } from "./spec-model";
import { walkSpec, pathLabel } from "./rule-tree";
import { LEVEL_SLOTS, LEVELS, TP_BOUNDS, levelAt, levelForm, takeProfits, type LevelBounds } from "./execution-model";
import { shortName } from "./catalogue";
import { infoOf, isFormulaOperand, wholeParam, type FormulaCheck } from "./formula";
import type { Spec, Catalogue, EntryLevel } from "./types";
export interface OptimizableSetting { path: string; label: string; group: string; current: number; min: number; max: number; step: number; integer: boolean }
/** Uses the same operand walker as validation; the backend remains authoritative for joint constraints. */
/** `formulas`: check results per formula text (the page's FormulaContext); a formula operand's Params are listed once its check is known. */
export function optimizablePaths(spec: Spec, cat: Catalogue, formulas: Record<string, FormulaCheck> = {}): OptimizableSetting[] {
  const out: OptimizableSetting[] = [], both = spec.direction === "both";
  const groupOf = (root: string) => {
    const short = root.startsWith("short_side."), prefix = short ? "Short " : both ? "Long " : "", base = root.replace(/^short_side\./, "");
    const text = base === "portfolio.score" ? "rank score" : base === "entry.all" ? "entry" : base === "exit.any" ? "exit" : "entry order";
    return prefix ? prefix + text : text[0].toUpperCase() + text.slice(1);
  };
  const full = new Map<string, string>(); // path -> long label, used only where the short one would be ambiguous
  // "Entry 1 › right · Exponential Moving Average Length" -> "Entry 1: EMA Length" (sides dropped, AmiBroker-style name: value)
  const named = (path: string, at: string, tail?: string) => {
    const [head, ...rest] = pathLabel(at, both).split(" › ").filter((p) => p !== "left" && p !== "right");
    const name = tail ?? rest.pop() ?? "value";
    full.set(path, tail ? `${pathLabel(at, both)} · ${tail}` : pathLabel(at, both));
    return `${[head, ...rest].join(" › ")}: ${name}`;
  };
  walkSpec(spec, (item) => {
    // a rule's on/off switch (rules.rs `….off`): top-level rules and the conditions of an all/any group
    if (item.t === "node" && (item.top || /\.(all|any)\[\d+\]$/.test(item.path))) {
      out.push({ path: `${item.path}.off`, label: `${pathLabel(item.path, both)}: switched off (0 on, 1 off)`, group: groupOf(item.root),
        current: (item.node as { off?: unknown }).off === true ? 1 : 0, min: 0, max: 1, step: 1, integer: true });
      return;
    }
    if (item.t === "knob") {
      const span = Math.max(Math.abs(item.value) * 0.5, 1);
      out.push({ path: item.path, label: named(item.path, item.path), group: groupOf(item.root), current: item.value,
        min: item.whole ? item.min : item.value - span, max: item.whole ? item.max : item.value + span, step: 1, integer: item.whole || Number.isInteger(item.value) });
    } else if (item.t === "operand" && isSeries(item.operand)) {
      const o = item.operand, ind = cat.indicators.find((d) => d.id === o.s);
      for (const p of ind?.params ?? []) out.push({ path: `${item.path}.p.${p.key}`, label: named(`${item.path}.p.${p.key}`, item.path, `${shortName(ind!)} ${p.label}`),
        group: groupOf(item.root), current: o.p?.[p.key] ?? p.default, min: p.min, max: p.max, step: p.step ?? (p.integer ? 1 : (p.max - p.min) / 100), integer: p.integer });
    } else if (item.t === "operand" && isFormulaOperand(item.operand)) {
      const o = item.operand;
      for (const p of infoOf(formulas[o.formula])?.params ?? []) out.push({ path: `${item.path}.p.${p.key}`, label: named(`${item.path}.p.${p.key}`, item.path, `${o.name} › ${p.key}`),
        group: groupOf(item.root), current: o.p?.[p.key] ?? p.default, min: p.min, max: p.max, step: p.step > 0 ? p.step : (p.max - p.min) / 100, integer: wholeParam(p) });
    }
  });
  const distance = (path: string, label: string, group: string, level: EntryLevel | undefined, bounds: LevelBounds) => {
    const form = levelForm(level);
    if (!level || !form || form === "at") return;
    const current = (level as unknown as Record<string, number>)[form];
    const [min, max, step, unit] = form === "pct" ? [bounds.zeroOk ? 0 : 0.5, bounds.maxPct, 0.5, "%"] : form === "atr" ? [0.25, 50, 0.25, "ATR ×"] : form === "profit_pct" ? [5, 95, 5, "% of profit"] : [bounds.zeroOk ? 0 : 1, 1e9, 1, "points"];
    out.push({ path: `${path}.${form}`, label: `${label} · ${unit}`, group, current, min: Number(min), max: Number(max), step: Number(step), integer: false });
  };
  for (const pre of both ? ["", "short_side."] as const : [""] as const) {
    const prefix = pre ? "Short " : both ? "Long " : "", group = (text: string) => prefix ? prefix + text : text[0].toUpperCase() + text.slice(1);
    for (const slot of LEVEL_SLOTS.filter((s) => s !== "exit.breakeven.plus")) distance(pre + slot, prefix + LEVELS[slot].label, group(slot.startsWith("entry") ? "entry order" : "exits"), levelAt(spec, pre, slot), LEVELS[slot]);
    takeProfits(spec, pre).forEach((tp, i) => distance(`${pre}exit.take_profits[${i}]`, `${prefix}Take-profit ${i + 1}`, group("exits"), tp, TP_BOUNDS));
    const dual = getAt(spec, pre + "exit.dual_atr") as { initial?: { period?: number; mult?: number }; favour?: { period?: number; mult?: number } } | undefined;
    if (dual) for (const [key, name] of [["initial", "Initial"], ["favour", "Favour"]] as const) {
      const b = dual[key] ?? {};
      out.push({ path: `${pre}exit.dual_atr.${key}.period`, label: `${prefix}Dual ATR · ${name} ATR period`, group: group("exits"), current: Number(b.period ?? 5), min: 1, max: 500, step: 1, integer: true },
        { path: `${pre}exit.dual_atr.${key}.mult`, label: `${prefix}Dual ATR · ${name} ATR ×`, group: group("exits"), current: Number(b.mult ?? 2), min: 0.25, max: 20, step: 0.25, integer: false });
    }
    if (getAt(spec, pre + "entry.scale_in")) out.push({ path: pre + "entry.scale_in.min_bars", label: prefix + "Scale-in · bars between legs", group: group("entry order"), current: Number(getAt(spec, pre + "entry.scale_in.min_bars") ?? 1), min: 1, max: 1000, step: 1, integer: true });
  }
  const z = spec.execution?.sizing;
  if (z && ("pct_equity" in z || "risk_pct" in z)) {
    const risk = "risk_pct" in z;
    out.push({ path: `execution.sizing.${risk ? "risk_pct" : "pct_equity"}`, label: risk ? "Risk per trade · %" : "Position size · % of equity", group: "Position size", current: risk ? Number(z.risk_pct) : Number(z.pct_equity), min: risk ? 0.25 : 5, max: 100, step: risk ? 0.25 : 5, integer: false });
  }
  if(spec.portfolio){const p=spec.portfolio,cap=cat.portfolio;out.push({path:"portfolio.max_open",label:"Max positions",group:"Portfolio",current:p.max_open??1,min:1,max:cap?.max_open??50,step:1,integer:true});if(p.mode==="rotation")out.push({path:"portfolio.worst_rank_held",label:"Worst rank held",group:"Portfolio",current:p.worst_rank_held??p.max_open??1,min:1,max:cap?.max_universe??120,step:1,integer:true});}
  const seen = new Map<string, number>();
  for (const o of out) seen.set(o.label, (seen.get(o.label) ?? 0) + 1);
  for (const o of out) if (seen.get(o.label)! > 1 && full.has(o.path)) o.label = full.get(o.path)!;
  return out;
}
