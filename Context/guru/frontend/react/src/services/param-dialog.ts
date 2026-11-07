/**
 * The "Run backtest" parameter window of YT strategies, laid out like AmiBroker's Parameters dialog:
 * sections, one row per parameter (label, slider or list, value, default). Built from the engine's
 * parameter schema (`/guru/<guru>/params`); the window returns only the values that differ from the defaults.
 */
export interface ParamDef {key: string; label: string; def: number; min: number; max: number; step: number}
export interface ParamOption {value: number; label: string}
export interface ParamSection {title: string; params: ParamDef[]}

const SECTIONS: [string, (k: string) => boolean][] = [
  ['Data', k => ['src', 'tf'].includes(k)],
  ['Entry', k => ['days', 'entry_time', 'entry_candle', 'entry_n'].includes(k)],
  ['Exit', k => ['exit_time', 'exit_candle', 'exit_n', 'pb_pct', 'hardstop', 'buffer_pct', 'pt_pct'].includes(k)],
  ['Trend filter', k => k.startsWith('flt_') || k.startsWith('st_') || k === 'sma_period'],
  ['Portfolio', k => ['max_positions', 'rank_dir'].includes(k)],
];

/** Parameters grouped into sections in a fixed order; everything else goes to "Rule settings". */
export function paramSections(schema: ParamDef[]): ParamSection[] {
  const out: ParamSection[] = SECTIONS.map(([title]) => ({title, params: []}));
  const rest: ParamDef[] = [];
  for (const p of schema) {
    const i = SECTIONS.findIndex(([, test]) => test(p.key));
    (i >= 0 ? out[i].params : rest).push(p);
  }
  out.splice(2, 0, {title: 'Rule settings', params: rest});
  return out.filter(s => s.params.length);
}

/** Named choices for a coded parameter, read from its label: "Trend filter (1 Supertrend, 2 SMA, 3 off)". */
export function paramOptions(p: ParamDef): ParamOption[] | null {
  const m = /\(([^()]*)\)\s*$/.exec(p.label);
  if (!m || !(p.step >= 1) || !Number.isInteger(p.step)) return null;
  // items are "<n> <name>" separated by ";" or by "," before the next number ("1 IBS <= level, final AFL; 2 …")
  const parts = m[1].split(/;\s*|,\s*(?=\d)/).map(s => /^(\d+)\s*(?:=\s*)?(.+)$/.exec(s.trim()));
  if (parts.some(x => !x)) return null;
  // a shared label may list more choices than this variant offers ("… 12 LAST ATHC where offered" with range 1-11): keep those in range
  const opts = parts.map(x => ({value: +x![1], label: x![2].trim().replace(/\s+where offered$/i, '')})).filter(o => o.value >= p.min && o.value <= p.max);
  const span = Math.round((p.max - p.min) / p.step) + 1;
  return opts.length === span && opts.every((o, i) => o.value === p.min + i * p.step) ? opts : null;
}

/** The label without its coded-choices part. */
export function paramTitle(p: ParamDef): string {
  return paramOptions(p) || isTime(p) ? p.label.replace(/\s*\([^()]*\)\s*$/, '') : p.label;
}

/** Rows that do nothing under the current trend-filter choice (greyed out, still editable). */
export function inactiveParams(values: Record<string, number>): Set<string> {
  const f = values.flt_type, off = new Set<string>();
  // position sizing: the amount per position matters only for fixed quantity, the % of equity only for compounding
  if (values.sizing === 1) off.add('alloc');
  if (values.sizing === 2) off.add('pct');
  if (f === undefined) return off;
  if (f !== 1) ['st_point', 'st_period', 'st_factor'].forEach(k => off.add(k));
  if (f !== 2) off.add('sma_period');
  if (f === 3) off.add('flt_tf');
  return off;
}

const isTime = (p: ParamDef): boolean => /_time$/.test(p.key) && p.min >= 0 && p.max <= 2359;
const toMin = (hhmm: number) => Math.floor(hhmm / 100) * 60 + (hhmm % 100);
export const hhmm = (x: number) => `${String(Math.floor(x / 100)).padStart(2, '0')}:${String(x % 100).padStart(2, '0')}`;

/** Clock choices of a HHMM parameter on its minute grid (HHMM numbers cannot be stepped by +5: 1455 -> 1500). */
export function timeOptions(p: ParamDef): ParamOption[] | null {
  if (!isTime(p)) return null;
  const step = Math.max(1, Math.round(p.step)), out: ParamOption[] = [];
  for (let m = toMin(p.min); m <= toMin(p.max) && out.length < 1500; m += step) {
    const v = Math.floor(m / 60) * 100 + (m % 60);
    out.push({value: v, label: hhmm(v)});
  }
  return out;
}

/** True when the row is a pick-list (coded choice, clock time or on/off toggle): those stay on their own grid. */
export function isCoded(p: ParamDef): boolean {
  return !!(paramOptions(p) || isTime(p) || PLAIN_OPTIONS[p.key] || (p.key !== 'noofloss' && p.step > 0 && Math.round((p.max - p.min) / p.step) + 1 <= 3));
}

/**
 * A value made usable. Pick-lists are forced into their range and grid; every other number runs as typed (the AFL's
 * range and step are only a hint): whole numbers when the AFL steps in whole numbers, never below zero when the AFL's minimum is zero or more.
 */
export function clampParam(p: ParamDef, x: number): number {
  if (!Number.isFinite(x)) return p.def;
  if (!isCoded(p)) {
    const whole = Number.isInteger(p.step) && Number.isInteger(p.min) && p.step >= 1;
    const v = p.min >= 0 ? Math.max(whole && p.min >= 1 ? 1 : 0, x) : x; // the floor is the AFL's own minimum: 0 stays 0 (Hard Stop, Penny), a period never drops below 1
    return whole ? Math.round(v) : +v.toFixed(6);
  }
  const opts = timeOptions(p);
  if (opts) return opts.reduce((a, b) => Math.abs(b.value - x) < Math.abs(a.value - x) ? b : a).value;
  const v = Math.min(p.max, Math.max(p.min, x));
  if (!(p.step > 0)) return v;
  const k = Math.round((v - p.min) / p.step);
  return +(p.min + k * p.step).toFixed(6);
}

/** Only the values that differ from the defaults (what the page keeps in its URL). */
export function changedParams(schema: ParamDef[], values: Record<string, number>): Record<string, number> {
  const out: Record<string, number> = {};
  for (const p of schema) {
    if (!(p.key in values)) continue;
    const v = clampParam(p, values[p.key]);
    if (v !== p.def) out[p.key] = v;
  }
  return out;
}

/** Short plain-English labels of the BTST engine keys (the "Backtest settings" window). */
export const PLAIN_LABELS: Record<string, string> = {
  src: 'Data source', days: 'Lookback days', entry_time: 'Entry time', exit_time: 'Exit time (next day)', pb_pct: 'Profit booking %',
  hardstop: 'Hard stop %', flt_type: 'Trend filter', flt_tf: 'Filter timeframe', st_point: 'Supertrend price', st_period: 'Supertrend period',
  st_factor: 'Supertrend factor', sma_period: 'SMA period', tf: 'Candle size', entry_candle: 'Entry candle (0 = last)', exit_candle: 'Exit candle',
  pt_pct: 'Profit target %', buffer_pct: 'Selling buffer %', ibs_dir: 'IBS test', ibs_thr: 'IBS threshold', roc_days: 'ROC days',
  roc_thr: 'ROC threshold %', hdays: 'Highest close days', candle: 'Candle colour', green_days: 'Green candle days', vol_days: 'Volume days',
  vol_mult: 'Volume multiple', athc_pct: 'ATHC exit %', max_positions: 'Maximum positions', pct: 'Position size (% of equity, 0 = equal)',
  pad: 'Pad & align to NIFTY', rank_dir: 'Ranking', lb_version: 'Lookback ranking version',
  sizing: 'Position sizing', alloc: 'Amount per position (₹)', equity: 'Initial capital (₹)',
  entry_n: 'Entry candle from close (1 = last candle)', exit_n: 'Exit candle from open, next day (1 = first candle)',
  penny: 'Penny stock price (₹, 0 = off)',
};

/** Plain option texts of the BTST coded keys, by value. */
export const PLAIN_OPTIONS: Record<string, Record<number, string>> = {
  src: {1: 'IEOD (as stored)', 2: 'Futures feed 2019-2026', 3: 'IEOD – team\'s 2021 copy'},
  flt_type: {1: 'Supertrend', 2: 'SMA', 3: 'Off'},
  flt_tf: {1: 'Daily', 2: 'Weekly', 3: 'Monthly'},
  st_point: {1: 'Mid point', 2: 'Close', 3: 'Pivot'},
  rank_dir: {1: 'High to low', 2: 'Low to high'},
  lb_version: {2: 'Ver 2 (lower lookback first)', 11: 'Ver 11 (higher lookback first)'},
  pad: {0: 'Off', 1: 'On'},
  ibs_dir: {1: 'IBS ≤ threshold', 2: 'IBS ≥ threshold'},
  candle: {1: 'Green', 2: 'Any'},
  tf: {1: '5 minutes', 2: '15 minutes'},
  sizing: {1: 'Compounding (% of equity)', 2: 'Fixed quantity (fixed amount)'},
};

/** The plain choices of a coded BTST parameter on its own grid (min..max by step), or null when it is not coded. */
export function plainOptions(p: ParamDef): ParamOption[] | null {
  const names = PLAIN_OPTIONS[p.key];
  if (!names || !(p.step > 0)) return null;
  const out: ParamOption[] = [];
  for (let v = p.min; v <= p.max + 1e-9 && out.length < 50; v = +(v + p.step).toFixed(6)) out.push({value: v, label: names[v] ?? String(v)});
  return out;
}

/**
 * Order of the "Strategy settings" rows: the engine keys the AFL sets, in AFL order (`aflKeys`, '@cost' and src are
 * period/cost rows), then every other schema key (ranking, portfolio rows …) in schema order. Never src.
 */
export function strategyKeys(schema: ParamDef[], aflKeys: (string | undefined)[] = []): string[] {
  const have = new Set(schema.map(p => p.key)), out: string[] = [];
  for (const k of aflKeys) if (k && k !== 'src' && k !== '@cost' && have.has(k) && !out.includes(k)) out.push(k);
  for (const p of schema) if (p.key !== 'src' && !out.includes(p.key)) out.push(p.key);
  return out;
}
