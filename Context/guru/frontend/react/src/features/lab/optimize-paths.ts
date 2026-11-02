import { isChoice, isSeries, getAt } from "./spec-model";
import { walkSpec, pathLabel } from "./rule-tree";
import { LEVEL_SLOTS, LEVELS, TP_BOUNDS, levelAt, levelForm, takeProfits, type LevelBounds } from "./execution-model";
import { PARAM_NAMES, shortName } from "./catalogue";
import { paramLabel } from "./labels";
import { infoOf, isFormulaOperand, wholeParam, type FormulaCheck } from "./formula";
import type { Spec, Catalogue, EntryLevel } from "./types";
export interface OptimizableSetting { path: string; label: string; group: string; current: number; min: number; max: number; step: number; integer: boolean }
/** Uses the same operand walker as validation; the backend remains authoritative for joint constraints. */
/** `formulas`: check results per formula text (the page's FormulaContext); a formula operand's Params are listed once its check is known. */
export function optimizablePaths(spec: Spec, cat: Catalogue, formulas: Record<string, FormulaCheck> = {}): OptimizableSetting[] {
  const out: OptimizableSetting[] = [], both = spec.direction === "both";
  // a filter row lives in entry.all with `filter: true`; it is grouped as a filter, as the rule cards show it
  const isFilter = (path: string) => { const m = /^((?:short_side\.)?entry\.all)\[(\d+)\]/.exec(path); return !!m && ((getAt(spec, m[1]) as { filter?: boolean }[] | undefined) ?? [])[Number(m[2])]?.filter === true; };
  const groupOf = (root: string, path = "") => {
    const short = root.startsWith("short_side."), prefix = short ? "Short " : both ? "Long " : "", base = root.replace(/^short_side\./, "");
    const text = base === "portfolio.score" ? "rank score" : base === "entry.all" ? (isFilter(path) ? "filters" : "entry") : base === "exit.any" ? "exit" : "entry order";
    return prefix ? prefix + text : text[0].toUpperCase() + text.slice(1);
  };
  const full = new Map<string, string>(); // path -> long label, used only where the short one would be ambiguous
  // "Entry 1 › right · Exponential Moving Average Length" -> "Entry 1: EMA Length" (sides dropped, AmiBroker-style name: value)
  // the Filters card's rows are entry.all items marked `filter`: "Filter 1", as the card names them, not "Entry 2"
  const asFilter = (label: string, at: string) => {
    const m = /^((?:short_side\.)?entry\.all)\[(\d+)\]/.exec(at);
    const all = m ? (getAt(spec, m[1]) as { filter?: boolean }[] | undefined) ?? [] : [];
    if (!m || !all[Number(m[2])]?.filter) return label;
    const n = all.slice(0, Number(m[2]) + 1).filter((x) => x.filter).length;
    return label.replace(/(^|Long |Short )?Entry \d+/, (_x, side = "") => `${side}Filter ${n}`);
  };
  const named = (path: string, at: string, tail?: string) => asFilter(namedRaw(path, at, tail), at);
  const namedRaw = (path: string, at: string, tail?: string) => {
    // "number" is the path's generic name for a compared value: the tail names it ("RSI threshold", "Days held")
    const [head, ...rest] = pathLabel(at, both).split(" › ").filter((p) => p !== "left" && p !== "right" && !(tail && p === "number"));
    const name = tail ?? rest.pop() ?? "value";
    full.set(path, tail ? `${pathLabel(at, both)} · ${tail}` : pathLabel(at, both));
    return `${[head, ...rest].join(" › ")}: ${name}`;
  };
  // the number a rule compares with, named as its card names it: "RSI threshold", "Candles", "Days held"
  const knobName = (path: string): string | undefined => {
    if (!path.endsWith(".rhs.value")) return undefined;
    const lhs = (getAt(spec, path.slice(0, -".rhs.value".length)) as { lhs?: { s?: string; since_entry?: string } } | undefined)?.lhs;
    if (lhs?.since_entry) return "Candles";
    if (lhs?.s === "bars_held") return "Days held";
    const ind = lhs?.s ? cat.indicators.find((d) => d.id === lhs.s) : undefined;
    return ind ? `${shortName(ind)} threshold` : "Threshold";
  };
  walkSpec(spec, (item) => {
    // rules switch on and off in the cards, not as a number to sweep: no "switched off (0 on, 1 off)" rows
    if (item.t === "node") return;
    if (item.t === "knob") {
      const span = Math.max(Math.abs(item.value) * 0.5, 1);
      out.push({ path: item.path, label: named(item.path, item.path, knobName(item.path)), group: groupOf(item.root, item.path), current: item.value,
        min: item.whole ? item.min : item.value - span, max: item.whole ? item.max : item.value + span, step: 1, integer: item.whole || Number.isInteger(item.value) });
    } else if (item.t === "operand" && isSeries(item.operand)) {
      const o = item.operand, ind = cat.indicators.find((d) => d.id === o.s);
      // the calculation choice (AmiBroker / Wilder …) is a setting the card keeps, not a number to sweep; numbers use the card's names
      for (const p of (ind?.params ?? []).filter((q) => !isChoice(q))) out.push({ path: `${item.path}.p.${p.key}`, label: named(`${item.path}.p.${p.key}`, item.path, `${shortName(ind!)} ${PARAM_NAMES[ind!.id]?.[p.key] ?? paramLabel(p.label)}`),
        group: groupOf(item.root, item.path), current: o.p?.[p.key] ?? p.default, min: p.min, max: p.max, step: p.step ?? (p.integer ? 1 : (p.max - p.min) / 100), integer: p.integer });
    } else if (item.t === "operand" && isFormulaOperand(item.operand)) {
      const o = item.operand;
      for (const p of infoOf(formulas[o.formula])?.params ?? []) out.push({ path: `${item.path}.p.${p.key}`, label: named(`${item.path}.p.${p.key}`, item.path, `${o.name} › ${p.key}`),
        group: groupOf(item.root, item.path), current: o.p?.[p.key] ?? p.default, min: p.min, max: p.max, step: p.step > 0 ? p.step : (p.max - p.min) / 100, integer: wholeParam(p) });
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
