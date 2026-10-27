/** Custom indicators ("My indicators"): the one place that reads the engine's formula shapes, plus labels, examples and the page scope.
 *  Engine contract: GET /lab/formula?formula= → {ok, params:[{key,default,min,max,step}], plots:[name], placement:"pane"|"price"}
 *  or 400 {error:"line L, col C: message"}. Adjust parseFormulaInfo if the engine reports a different shape. */
import { createContext, useContext } from "react";
import type { FormulaInfo, FormulaOperand, FormulaParam, Indicator, SavedIndicator } from "./types";

export const MAX_FORMULA = 4000, MAX_NAME = 60;

const num = (v: unknown, what: string) => {
  if (typeof v !== "number" || !Number.isFinite(v)) throw new Error(`The engine returned an unreadable formula ${what}.`);
  return v;
};
/** Validates the engine answer (lab/formula, or `formula` on a series) into FormulaInfo. */
export function parseFormulaInfo(body: unknown): FormulaInfo {
  const b = (body ?? {}) as Record<string, unknown>;
  if (!Array.isArray(b.params) || !Array.isArray(b.plots)) throw new Error("The engine returned an unreadable formula check.");
  const params: FormulaParam[] = b.params.map((x) => {
    const p = (x ?? {}) as Record<string, unknown>;
    if (typeof p.key !== "string" || !p.key) throw new Error("The engine returned a formula parameter without a name.");
    return { key: p.key, default: num(p.default, "default"), min: num(p.min, "minimum"), max: num(p.max, "maximum"), step: num(p.step, "step") };
  });
  const plots = b.plots.filter((x): x is string => typeof x === "string");
  return { params, plots, placement: b.placement === "price" ? "price" : "pane" };
}

export type FormulaCheck = FormulaInfo | { error: string };
export const isFormulaOperand = (o: unknown): o is FormulaOperand =>
  typeof o === "object" && o !== null && typeof (o as FormulaOperand).formula === "string";
/** Whole-number Param: the engine accepts only whole values when the step is whole. */
export const wholeParam = (p: FormulaParam) => Number.isInteger(p.step) && Number.isInteger(p.default);

/** "My Momentum(Fast=9).Osc"; with `info`, params left at their default are omitted. */
export function formulaLabel(o: FormulaOperand, info?: FormulaInfo): string {
  const shown = Object.entries(o.p ?? {}).filter(([k, v]) => !info || info.params.find((p) => p.key === k)?.default !== v);
  return `${o.name || "My indicator"}${shown.length ? `(${shown.map(([k, v]) => `${k}=${v}`).join(", ")})` : ""}${o.out ? `.${o.out}` : ""}`;
}

export const myId = (id: number) => `my:${id}`;
/** A saved formula as a catalogue-like indicator, so chips, panes and the Parameters window treat it like a built-in. */
export function formulaIndicator(row: Pick<SavedIndicator, "id" | "name" | "formula">, info: FormulaInfo): Indicator {
  return {
    id: myId(row.id), name: row.name, category: "My indicators", placement: info.placement, computed: true, formula: row.formula,
    needs: { volume: false, intraday: false, breadth: false, second_symbol: false, fundamentals: false, derivatives: false, onchain: false, order_flow: false, order_book: false },
    params: info.params.map((p) => ({ key: p.key, label: p.key, default: p.default, min: p.min, max: p.max, step: p.step || null, integer: wholeParam(p) })),
    outputs: info.plots, first: info.plots.map(() => null), sparse: false, definition: row.formula, note: "", tv: "", ab: "",
  };
}
/** Extra series query fields for a chip drawn from a formula. */
export const formulaQuery = (ind: Indicator) => (ind.formula ? { formula: ind.formula, name: ind.name } : {});

/** "line 2, col 5: …" → 1-based position. */
export function errorPosition(message: string): { line: number; col: number } | undefined {
  const m = /line\s+(\d+),\s*col(?:umn)?\s+(\d+)/i.exec(message);
  return m ? { line: Number(m[1]), col: Number(m[2]) } : undefined;
}

export const FORMULA_EXAMPLES: { title: string; name: string; formula: string }[] = [
  { title: "MACD-style oscillator", name: "My MACD", formula: `// difference of two EMAs and its signal line
Fast = Param("Fast", 12, 2, 50, 1);
Slow = Param("Slow", 26, 5, 200, 1);
Sig = Param("Signal", 9, 2, 50, 1);
Osc = EMA(C, Fast) - EMA(C, Slow);
Plot(Osc, "Osc");
Plot(EMA(Osc, Sig), "Trigger");` },
  { title: "RSI of an EMA", name: "Smooth RSI", formula: `// Wilder's RSI computed on a smoothed close
Len = Param("EMA length", 10, 2, 100, 1);
N = Param("RSI length", 14, 2, 100, 1);
X = EMA(C, Len);
Up = Wilders(Max(X - X[1], 0), N);
Dn = Wilders(Max(X[1] - X, 0), N);
Plot(100 - 100 / (1 + Up / Dn), "RSI");` },
  { title: "Bollinger %B", name: "My %B", formula: `// where the close sits inside the bands: 0 lower, 1 upper
N = Param("Periods", 20, 5, 200, 1);
W = Param("Width", 2, 0.5, 4, 0.5);
Mid = MA(C, N);
Dev = StDev(C, N);
Plot((C - (Mid - W * Dev)) / (2 * W * Dev), "PctB");` },
];

/** Page-wide: the user's saved indicators and the check result per formula text (filled by the page for every formula in the spec). */
export interface FormulaScope { mine: SavedIndicator[]; infos: Record<string, FormulaCheck> }
export const FormulaContext = createContext<FormulaScope>({ mine: [], infos: {} });
export const useFormulaScope = () => useContext(FormulaContext);
export const infoOf = (c: FormulaCheck | undefined): FormulaInfo | undefined => (c && "params" in c ? c : undefined);
