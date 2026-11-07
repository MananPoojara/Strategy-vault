/**
 * Parameter optimization of the ATH stock-universe sub-variants (`guru/yt/ath-optimize`): pure helpers for the Optimize page.
 * The engine runs the portfolio backtest once per combination of the ticked settings and answers rows `{default, p, stats}`;
 * the page ranks them. The limits mirror the engine's (60 combinations, 20 values per setting).
 */
import type {ParamDef} from './param-dialog';

export const ATH_OPT_MAX_COMBOS = 60;
export const ATH_OPT_MAX_VALUES = 20;
/**
 * Settings that are not strategy parameters, never offered for variation: account size, brokerage, how the money is split
 * (fixed amount or % of equity, the fixed amount, POS. SIZE %) and the dead-stock data choice. They run at the backtest page's values.
 * MAXIMUM POSITION stays: it decides which trades are taken.
 */
const NOT_VARIED = new Set(['equity', 'comm', 'brokerage', 'sizing', 'alloc', 'pct', 'deadstock', 'deadticker']);
/** Settings the AFL declares but its trades never read (the engine's label says so): varying them only repeats the same run. */
const UNUSED = /not used|never used|unused array|plot only|plotted only|overwritten by|Explore columns only|no effect|no backtest signal|not applied/i;
export interface AthOptRow {default?: boolean; p: Record<string, number>; stats: Record<string, number | null>}
export type AthOptSort = 'cagr' | 'sharpe' | 'car_mdd';

export const athOptParams = (schema: ParamDef[]): ParamDef[] => schema.filter(d => !NOT_VARIED.has(d.key) && !UNUSED.test(d.label || '') && d.max > d.min && d.step > 0);

/**
 * The values from..to in steps of `step`; [] when the input is not a usable range. A pick-list or on/off setting stays inside its own
 * range; any other number runs as typed (the AFL's range is only a hint), never below zero when the AFL's minimum is zero or more.
 */
export function rangeValues(from: number, to: number, step: number, d: Pick<ParamDef, 'min' | 'max'> & {step?: number}): number[] {
  if (![from, to, step].every(Number.isFinite) || step <= 0 || to < from) return [];
  const pick = !!d.step && d.step > 0 && Math.round((d.max - d.min) / d.step) + 1 <= 3;
  const lo = Math.max(from, pick || d.min >= 0 ? d.min : -Infinity), hi = pick ? Math.min(to, d.max) : to;
  if (hi < lo) return [];
  const n = Math.floor((hi - lo) / step + 1e-9) + 1;
  if (n > ATH_OPT_MAX_VALUES * 50) return Array.from({length: ATH_OPT_MAX_VALUES + 1}, (_, k) => lo + k * step); // over the cap anyway
  return Array.from({length: n}, (_, k) => Math.round((lo + k * step) * 1e9) / 1e9);
}

/** The starting range of a setting: From = To = its default, in the AFL's step (like the Indicators page). */
export const defaultRange = (d: ParamDef): {from: number; to: number; step: number} => ({from: d.def, to: d.def, step: d.step});

/** Settings the optimization grid shows before it scrolls, and the row heights the stylesheet gives it (styles.css `.ogrid`). */
export const OPT_ROWS_SHOWN = 10;
const HEAD_PX = 40, GROUP_PX = 36, ROW_PX = 46;

/** Height of the grid's scroll box: the header, the first 10 settings and the group headings above them; 0 = no limit (10 settings or fewer). */
export function optGridHeight(sections: {params: unknown[]}[], shown = OPT_ROWS_SHOWN): number {
  if (sections.reduce((a, s) => a + s.params.length, 0) <= shown) return 0;
  let px = HEAD_PX, left = shown;
  for (const s of sections) {
    if (left <= 0) break;
    px += GROUP_PX + Math.min(left, s.params.length) * ROW_PX;
    left -= s.params.length;
  }
  return px + 2; // the box's borders
}

export const comboCount = (grids: Record<string, number[]>): number => Object.values(grids).reduce((a, v) => a * v.length, 1);

/** Why the grid cannot run, or '' when it can. */
export function gridProblem(grids: Record<string, number[]>): string {
  const keys = Object.keys(grids);
  if (!keys.length) return 'Tick at least one setting to vary.';
  const empty = keys.find(k => !grids[k].length);
  if (empty) return `The range of ${empty} is empty or outside its allowed values.`;
  const big = keys.find(k => grids[k].length > ATH_OPT_MAX_VALUES);
  if (big) return `${big} has more than ${ATH_OPT_MAX_VALUES} values; use a bigger step.`;
  const n = comboCount(grids);
  return n > ATH_OPT_MAX_COMBOS ? `${n} combinations; the limit is ${ATH_OPT_MAX_COMBOS}. Tick fewer settings or use a bigger step.` : '';
}

export function athOptimizeQuery(id: string, o: {from?: string; to?: string; slippage?: string | number; params?: Record<string, number>; grids: Record<string, number[]>}): URLSearchParams {
  const q = new URLSearchParams({id, cost: '0', slippage: String(o.slippage ?? 0)});
  if (o.from) q.set('from', o.from);
  if (o.to) q.set('to', o.to);
  for (const [k, v] of Object.entries(o.params || {})) if (Number.isFinite(v) && !(k in o.grids)) q.set('p.' + k, String(v));
  for (const [k, v] of Object.entries(o.grids)) q.set('grid.' + k, v.join(','));
  return q;
}

const num = (x: unknown): number => (typeof x === 'number' && Number.isFinite(x) ? x : NaN);
export const carMdd = (s: Record<string, number | null>): number => { const c = num(s.cagr), dd = num(s.max_dd); return dd < 0 ? c / -dd : NaN; };
const key = (r: AthOptRow, by: AthOptSort): number => (by === 'car_mdd' ? carMdd(r.stats) : num(r.stats[by]));

/** Rows best first by the chosen measure; rows without a finite value go last (stable). */
export function sortRows(rows: AthOptRow[], by: AthOptSort): AthOptRow[] {
  return rows.map((r, i) => ({r, i, k: key(r, by)})).sort((a, b) => (Number.isNaN(a.k) ? 1 : 0) - (Number.isNaN(b.k) ? 1 : 0) || (Number.isNaN(a.k) ? 0 : b.k - a.k) || a.i - b.i).map(x => x.r);
}

/** A usable answer: every row has parameters and a stats object. */
export const validAthResult = (d: any): d is {rows: AthOptRow[]; grid: Record<string, number[]>; combos: number; seconds: number} =>
  !!d && d.status === 'ready' && Array.isArray(d.rows) && d.rows.length > 0 && d.rows.every((r: any) => r && typeof r.p === 'object' && r.p && typeof r.stats === 'object' && r.stats);
