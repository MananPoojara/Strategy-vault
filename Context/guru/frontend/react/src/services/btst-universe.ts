/**
 * BTST on the F&O stock universe (Phase 2): the engine's `guru/yt/universe` info and `guru/yt/universe-run` portfolio
 * run. Pure helpers for the page: which sub-variants run there, the parameter schema of a universe run, the query, the
 * "Traded:" line and the trade sheet table.
 */
import type {ParamDef} from './param-dialog';
import type {AflParam} from './afl-params';
import {IEOD_DATABASE} from './btst-instrument';

export interface UniverseInfo {
  universe: string; symbols: number; first: string; last: string; ids: string[]; params: ParamDef[];
  schemas?: Record<string, ParamDef[]>;
  /** wording of a family's universe (ATH): choice button, "Traded:" line, card text; the BTST wording is the default */
  choice_label?: string; traded?: string; badge?: string; stamp?: string; run_label?: string; on_label?: string; card_note?: string; unit?: string; calendar_note?: string;
}
export interface UniverseTrade {
  symbol: string; entry: string; entry_time?: string; entry_ts?: string; exit: string; exit_time?: string | null; exit_ts?: string | null;
  entry_px: number; exit_px: number; shares: number; weight: number; score: number; ret: number; reason?: string; open?: boolean; short?: boolean;
}
export type RunScope = 'single' | 'basket' | 'universe';

/** Sub-variants that exist only on the stock universe (no single-instrument run). */
export const UNIVERSE_ONLY_IDS = ['ytbtst_rank_price', 'ytbtst_rank_ibs', 'ytbtst_rank_lookback', 'ytbtst_rank_roc', 'ytbtst_rank_ibs_volume', 'ytbtst_nifty_upclose', 'ytbtst_nifty_incremental'];
/** BTST: the listed ids. ATH: every sub-variant (they run on a stock universe only; whether the engine runs one is `universeRunnable`). */
export const isUniverseOnly = (id: string | undefined): boolean => !!id && (UNIVERSE_ONLY_IDS.includes(id) || id.startsWith('ytath_'));

/** True when the engine lists `id` as runnable on the universe (the universe-only ids also when the info is not loaded yet). */
export function universeRunnable(info: UniverseInfo | null | undefined, id: string | undefined): boolean {
  if (!id) return false;
  return info ? info.ids.includes(id) : isUniverseOnly(id) && !id.startsWith('ytath_'); // ATH runs only when the engine reported its universe
}

/**
 * The run scope kept in the URL: `basket` and `universe` only on a strategy page; a universe-only sub-variant is always
 * on the universe; anything else is a single instrument.
 */
export function normaliseRunScope(raw: string | null | undefined, isStrategy: boolean, id?: string): RunScope {
  if (!isStrategy) return 'single';
  if ((id?.startsWith('ytbtst_') || id?.startsWith('ytath_')) && (raw === 'universe' || isUniverseOnly(id))) return 'universe';
  return raw === 'basket' ? 'basket' : 'single';
}

export const universeChoiceLabel = (info: Pick<UniverseInfo, 'universe' | 'symbols'> & Partial<UniverseInfo> | null | undefined): string =>
  info?.choice_label || `F&O stocks · ${info?.universe || 'FO STOCKS C&P'}${info?.symbols ? ` (${info.symbols})` : ''}`;

/** The "Traded:" line of a universe run (src 3 = the database as the team's 2021 copy). */
export function universeTradedLine(src: number, info?: (Pick<UniverseInfo, 'universe' | 'symbols'> & Partial<UniverseInfo>) | null, database = IEOD_DATABASE): string {
  if (info?.traded) return info.traded;
  const name = info?.universe || 'FO STOCKS C&P', n = info?.symbols ? ` (${info.symbols} symbols)` : '';
  return `Traded: F&O stock futures, near-month, ${name} watchlist${n} from the IEOD_FO database (${database})${src === 3 ? ', as the team\'s 2021 copy' : ''}`;
}

/** A family's own wording of the universe (`key` of the engine's info), else the BTST default. */
export const universeText = (info: Partial<UniverseInfo> | null | undefined, key: 'badge' | 'stamp' | 'run_label' | 'on_label' | 'card_note' | 'unit' | 'calendar_note', fallback: string): string => (info && info[key]) || fallback;

// AmiBroker-style labels (with coded choices the parameter window turns into lists).
const LABELS: Record<string, string> = {
  src: 'Data source (1 IEOD as stored; 3 IEOD as the team\'s 2021 copy)',
  max_positions: 'MAXIMUM POSITION',
  pct: 'POS. SIZE NO OF SHARE / %OF EQUITY',
  pad: 'Pad and align to NIFTY (0 off; 1 on)',
  rank_dir: 'Ranking (1 High to low; 2 Low to high)',
  lb_version: 'Lookback ranking (2 ver 2, lower lookback first; 11 ver 11, higher lookback first)',
};

const FALLBACK_OWN: Record<string, ParamDef[]> = {
  ytbtst_rank_price: [{key: 'rank_dir', label: '', def: 1, min: 1, max: 2, step: 1}],
  ytbtst_rank_ibs: [{key: 'rank_dir', label: '', def: 1, min: 1, max: 2, step: 1}],
  ytbtst_rank_lookback: [{key: 'lb_version', label: '', def: 2, min: 2, max: 11, step: 9}],
  ytbtst_rank_roc: [{key: 'roc_days', label: 'ROC days (3 = a 2-day ROC)', def: 3, min: 2, max: 5, step: 1}],
  ytbtst_rank_ibs_volume: [{key: 'vol_days', label: 'Volume check days (average of N completed days)', def: 5, min: 1, max: 100, step: 1}, {key: 'vol_mult', label: 'Volume multiple', def: 1, min: 1, max: 100, step: 0.1}],
};

/**
 * Parameters of a universe run of `id`: the engine's own schema (`schemas[id]`), else the core rules' schema plus the
 * id's own keys and the universe's max_positions / pct / pad. src is limited to 1 | 3; labels made AmiBroker-like.
 */
export function universeSchema(info: UniverseInfo | null | undefined, id: string, core: ParamDef[] = []): ParamDef[] {
  let out: ParamDef[];
  const own = info?.schemas?.[id];
  if (own?.length) out = own.map(p => ({...p}));
  else {
    out = core.map(p => ({...p}));
    for (const p of [...(FALLBACK_OWN[id] || []), ...(info?.params || [])]) if (!out.some(q => q.key === p.key)) out.push({...p});
  }
  return out.map(p => {
    const q = LABELS[p.key] ? {...p, label: LABELS[p.key]} : p;
    return q.key === 'src' ? {...q, def: 1, min: 1, max: 3, step: 2} : q;
  });
}

/**
 * The AFL rows of a sub-variant as shown for a universe run: the core AFLs mark the portfolio rows as "an index run
 * holds one position"; on the universe MAXIMUM POSITION and POS. SIZE … %OF EQUITY are the run's max_positions / pct.
 */
export function universeAflRows(rows: AflParam[] | undefined): AflParam[] | undefined {
  if (!rows) return rows;
  const has = (k: string) => rows.some(r => r.key === k);
  return rows.map(r => {
    if (r.key || !r.why) return r;
    if (/^MAXIMUM POSITION$/i.test(r.label.trim()) && !has('max_positions')) return {...r, key: 'max_positions', why: undefined};
    if (/^POS\. SIZE NO OF SHARE \/ %OF EQUITY$/i.test(r.label.trim()) && !has('pct')) return {...r, key: 'pct', why: undefined};
    if (/index run holds one position|position sizing/i.test(r.why)) return {...r, why: 'the Vault sizes each position as % of equity (MAXIMUM POSITION and POS. SIZE rows)'};
    return r;
  });
}

/** Query string of `guru/yt/universe-run` (cost is fixed at 0; the brokerage goes in as slippage bps a side). */
export function universeQuery(id: string, o: {from?: string; to?: string; slippage?: string | number; params?: Record<string, number>}): URLSearchParams {
  const q = new URLSearchParams({id, cost: '0', slippage: String(o.slippage ?? 0)});
  if (o.from) q.set('from', o.from);
  if (o.to) q.set('to', o.to);
  for (const [k, v] of Object.entries(o.params || {})) if (Number.isFinite(v)) q.set('p.' + k, String(v));
  return q;
}

const splitTs = (ts: string | null | undefined, date: string, time: string | null | undefined): [string, string] => {
  if (ts) { const [d, t = ''] = ts.split(' '); return [d, t]; }
  return [date, time || ''];
};

export const UNIVERSE_TRADE_HEADERS = ['Symbol', 'Trade', 'Entry date', 'Entry time', 'Entry price', 'Exit date', 'Exit time', 'Exit price', 'Shares', 'Weight', 'Score', 'Return', 'Exit reason'];
/** Columns holding fractions shown as percentages. */
export const UNIVERSE_PERCENT_COLUMNS = [9, 11];

/**
 * Trade sheet rows in engine order (oldest first). The Trade column is AmiBroker's: Long / Short, "Open Long" / "Open Short" for a
 * position still open, which is marked at the date and close of the last bar of the run (exit date, exit price) with "open" as reason.
 */
export function universeTradeRows(trades: UniverseTrade[]): (string | number | null)[][] {
  return trades.map(t => {
    const [ed, et] = splitTs(t.entry_ts, t.entry, t.entry_time), [xd, xt] = t.open ? [t.exit, ''] : splitTs(t.exit_ts, t.exit, t.exit_time);
    const trade = `${t.open ? 'Open ' : ''}${t.short ? 'Short' : 'Long'}`;
    return [t.symbol, trade, ed, et, t.entry_px, xd, xt, t.exit_px, t.shares, t.weight, t.score, t.ret, t.reason || (t.open ? 'open' : '')];
  });
}

/** Average and largest number of open positions, and the share of days with at least one. */
export function positionStats(positions: number[] | undefined): {avg: number; max: number; invested: number} | null {
  if (!positions?.length) return null;
  let sum = 0, max = 0, on = 0;
  for (const n of positions) { sum += n; if (n > max) max = n; if (n > 0) on += 1; }
  return {avg: sum / positions.length, max, invested: on / positions.length};
}

export interface TradePage<T> {rows: T[]; total: number; page: number; pages: number; first: number; last: number; years: string[]}

/**
 * One page of the universe trade sheet (rows newest first): optional symbol filter (column 0) and year (entry date,
 * column 1). Every trade stays reachable — the sheet pages instead of cutting the list.
 */
export function tradePage<T extends (string | number | null)[]>(rows: T[], opts: {symbol?: string; year?: string; page?: number; size?: number} = {}): TradePage<T> {
  const size = Math.max(1, opts.size ?? 500), q = (opts.symbol || '').trim().toUpperCase();
  const years = [...new Set(rows.map(r => String(r[2]).slice(0, 4)).filter(y => /^\d{4}$/.test(y)))].sort().reverse();
  const hit = rows.filter(r => (!q || String(r[0]).toUpperCase().includes(q)) && (!opts.year || String(r[2]).startsWith(opts.year)));
  const pages = Math.max(1, Math.ceil(hit.length / size)), page = Math.min(Math.max(1, Math.floor(opts.page || 1)), pages);
  const start = (page - 1) * size, out = hit.slice(start, start + size);
  return {rows: out, total: hit.length, page, pages, first: out.length ? start + 1 : 0, last: start + out.length, years};
}
