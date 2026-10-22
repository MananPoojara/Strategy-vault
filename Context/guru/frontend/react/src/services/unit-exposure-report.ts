import {unitReturnConsistent, unitPriceReturnConsistent, unitFactorConsistent, unitFeeConsistent,
  unitProductConsistent, unitCurveConsistent, unitEntryCurveConsistent} from './unit-exposure-rounding';

export const UNIT_EXPOSURE_METHOD = 'hbqts_dema16_8';
export const UNIT_EXPOSURE_BOOK = 'hbqts';
export const UNIT_EXPOSURE_MODEL = 'canonical_decimal_unit_exposure_v2';
export const UNIT_EXPOSURE_CONVENTION = 'qts_dema16_8_first_close_index_unit_exposure_v2';
export const UNIT_EXPOSURE_ARITHMETIC = 'pinned Rust Display canonical-decimal rational; binary64 nearest-even including subnormals; nonzero-to-zero underflow and overflow rejected';
export const unitExposureMethod = (id?: string, book?: string) => id === UNIT_EXPOSURE_METHOD && book === UNIT_EXPOSURE_BOOK;
export interface UnitExposureDay {
  source_index: number; date: string; equity: number; gross_equity: number; benchmark_equity: number;
  net_daily_return: number; gross_daily_return: number; benchmark_daily_return: number;
}
export interface UnitExposureTrade {
  trade_index: number; entry_source_index: number; exit_source_index: number; open: boolean;
  entry_allocation: number; gross_return: number; net_return: number; fee_return: number; slippage_return: number;
  gross_contribution: number; net_contribution: number; fee_contribution: number; slippage_contribution: number;
}
export interface UnitExposureReport {
  model: typeof UNIT_EXPOSURE_MODEL; book: typeof UNIT_EXPOSURE_BOOK; id: typeof UNIT_EXPOSURE_METHOD;
  source_convention: typeof UNIT_EXPOSURE_CONVENTION; source_performance_reproduced: false;
  native_amibroker_parity: false; basis: string; arithmetic: typeof UNIT_EXPOSURE_ARITHMETIC;
  initial_equity: 1; open_exit_cost: 'estimated_liquidation'; days: UnitExposureDay[]; trade_returns: UnitExposureTrade[];
}
export interface UnitExposureAdmission {
  id: string; book: string; ticker: string; region: string;
  definition: unknown; parameters: unknown; instrument: unknown;
  costs: {fees_bps: number; slippage_bps: number; rollover_bps: number};
  window?: {from?: string; to?: string};
}
export interface UnitExposureView {
  model: typeof UNIT_EXPOSURE_MODEL; report: UnitExposureReport; basis: string;
  ticker: string; region: string; closedReturnSum: number; closedTrades: number;
}
type Fields = Record<string, unknown>;
const object = (v: unknown): v is Fields => Boolean(v) && typeof v === 'object' && !Array.isArray(v);
const finite = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
const positive = (v: unknown): v is number => finite(v) && v > 0;
const integer = (v: unknown): v is number => finite(v) && Number.isSafeInteger(v) && v >= 0;
const scalar = (v: unknown): v is number => finite(v) && !Object.is(v, -0);
const text = (v: unknown): v is string => typeof v === 'string' && Boolean(v.trim());
const date = (v: unknown): v is string => typeof v === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(v)
  && Number.isFinite(Date.parse(v)) && new Date(v).toISOString().slice(0, 10) === v;
const exactKeys = (v: Fields, keys: readonly string[]) => Object.keys(v).length === keys.length && keys.every(k => Object.hasOwn(v, k));
const invalid = (): never => {throw new Error('The reviewed unit-exposure report is missing or inconsistent. Results remain unavailable; reload after the report contract is restored.');};
const sum = (values: number[]) => values.reduce((a, b) => a + b, 0);
/** Scoped display convention; it never changes an audited model fraction/sign. */
export function unitStatisticDisplay(value: number): number | null {
  if (!Number.isFinite(value)) return null;
  const scaled = value * 1e6;
  if (!Number.isFinite(scaled)) return value;
  const magnitude = Math.abs(scaled);
  const integral = magnitude >= 2 ** 52 ? magnitude : Math.floor(magnitude) + Number(magnitude - Math.floor(magnitude) >= .5);
  return (scaled < 0 || Object.is(scaled, -0) ? -integral : integral) / 1e6;
}
/** Retains actual metadata unchanged; no fabricated provider/type information. */
export function unitExposureInstrumentContext(ticker: string, region: string, rows: readonly unknown[], indices: readonly unknown[]) {
  if (!((region === 'us' && ticker === 'SPX') || (region === 'in' && ticker === 'NIFTY 50'))) return null;
  const found = rows.filter(row => object(row) && row.ticker === ticker);
  if (found.length !== 1 || !indices.includes(found[0]) || !object(found[0])) return null;
  const metadata = found[0];
  if (metadata.asset_type !== undefined && metadata.asset_type !== 'index') return null;
  if (!date(metadata.first) || !date(metadata.last) || metadata.first > metadata.last) return null;
  return {classification_basis: 'existing_runtime_index_selector', region, metadata};
}
const COMMON_STATS = ['total_return','cagr','vol','sharpe','sortino','max_dd','max_dd_days','mar','exposure','bh_total','bh_cagr','bh_max_dd','years','trades','win_rate','avg_trade','median_trade','avg_win','avg_loss','payoff','profit_factor','best','worst','t_stat','max_consec_losses','avg_bars','avg_deployed','open_trade','trades_per_year','net_profit','closed_return_sum','compounded_return','car_mdd','expectancy_ratio','avg_hold_days','monthly_thickness'] as const;
const FINANCIAL_STATS = [...COMMON_STATS,'gross_total_return','gross_avg_trade','gross_win_rate','gross_sharpe','gross_max_dd','avg_net_pnl','ledger_gross_return','fee_return','slippage_return'];
const BENCHMARK_STATS = ['total_return','cagr','sharpe','max_dd','vol','sortino','mar'];
const DAY_KEYS = ['source_index','date','equity','gross_equity','benchmark_equity','net_daily_return','gross_daily_return','benchmark_daily_return'];
const TRADE_KEYS = ['trade_index','entry_source_index','exit_source_index','open','entry_allocation','gross_return','net_return','fee_return','slippage_return','gross_contribution','net_contribution','fee_contribution','slippage_contribution'];
const REPORT_KEYS = ['model','book','id','source_convention','source_performance_reproduced','native_amibroker_parity','basis','arithmetic','initial_equity','open_exit_cost','days','trade_returns'];
function context(data: Fields, admission: UnitExposureAdmission, adapter: boolean) {
  if (!unitExposureMethod(admission.id, admission.book) || data.id !== admission.id || data.book !== admission.book
    || data.ticker !== admission.ticker || !object(admission.definition) || admission.definition.id !== admission.id
    || admission.definition.book !== admission.book || !object(admission.definition.instrument_policy)
    || !object(admission.parameters) || !exactKeys(admission.parameters, ['fast','slow','hold'])
    || admission.parameters.fast !== 16 || admission.parameters.slow !== 8 || admission.parameters.hold !== 1
    || !object(admission.instrument) || !object(admission.instrument.metadata)) return invalid();
  const policy = admission.definition.instrument_policy, instrument = admission.instrument, metadata = instrument.metadata;
  if (!object(metadata)) return invalid();
  if (policy.requirement !== 'index' || !Array.isArray(policy.allowed_types) || policy.allowed_types.length !== 1
    || policy.allowed_types[0] !== 'index' || policy.execution_scope !== 'single_symbol_index_timing_research'
    || policy.source_convention !== UNIT_EXPOSURE_CONVENTION || !text(policy.source_basis)
    || policy.source_rule_reviewed !== true || policy.independent_selected_ledger_checked !== true
    || policy.source_performance_reproduced !== false || policy.market_adaptation !== true
    || Object.hasOwn(policy, 'account_model') || Object.hasOwn(policy, 'allowed_symbols')) return invalid();
  if (metadata.ticker !== admission.ticker || !date(metadata.first) || !date(metadata.last) || metadata.first > metadata.last) return invalid();
  if (adapter) {
    if (admission.region !== 'test_only' || admission.ticker !== 'TEST_ONLY_SYNTHETIC'
      || instrument.classification_basis !== 'independent_synthetic_reference') return invalid();
  } else if (!((admission.region === 'us' && admission.ticker === 'SPX') || (admission.region === 'in' && admission.ticker === 'NIFTY 50'))
    || instrument.classification_basis !== 'existing_runtime_index_selector' || instrument.region !== admission.region
    || (metadata.asset_type !== undefined && metadata.asset_type !== 'index')) return invalid();
  const provenance = data.provenance;
  if (provenance !== null && !object(provenance)) return invalid();
  if (object(provenance)) {
    if (provenance.qualified_ticker !== undefined && provenance.qualified_ticker !== admission.ticker
      || provenance.symbol !== undefined && provenance.symbol !== admission.ticker
      || provenance.asset_type !== undefined && provenance.asset_type !== 'index') return invalid();
    for (const source of [metadata, object(metadata.provenance) ? metadata.provenance : {}]) {
      for (const key of ['symbol','qualified_ticker','asset_type','trusted_market','currency','source_sha256','csv_sha256']) {
        if (source[key] !== undefined && source[key] !== provenance[key]) return invalid();
      }
    }
  }
  return {policy, metadata: metadata as Fields & {first: string; last: string}};
}
function checkStats(value: unknown, keys: readonly string[]): Fields {
  if (!object(value) || !exactKeys(value, keys) || keys.some(k => value[k] !== null && !finite(value[k]))) return invalid();
  return value;
}
function core(data: unknown, admission: UnitExposureAdmission, adapter: boolean): UnitExposureView {
  if (!object(data) || Object.hasOwn(data, 'source_account') || Object.hasOwn(data,'kind') || !object(data.unit_exposure_report)) return invalid();
  const {policy, metadata} = context(data, admission, adapter), raw = data.unit_exposure_report;
  if (!exactKeys(raw, REPORT_KEYS) || raw.model !== UNIT_EXPOSURE_MODEL || raw.book !== UNIT_EXPOSURE_BOOK || raw.id !== UNIT_EXPOSURE_METHOD
    || raw.source_convention !== UNIT_EXPOSURE_CONVENTION || raw.source_performance_reproduced !== false
    || raw.native_amibroker_parity !== false || raw.arithmetic !== UNIT_EXPOSURE_ARITHMETIC || raw.initial_equity !== 1
    || raw.open_exit_cost !== 'estimated_liquidation' || !text(raw.basis) || raw.basis !== data.accounting_basis
    || raw.basis !== policy.source_basis || data.model_initial_equity !== 1 || data.benchmark_symbol !== admission.ticker
    || !Array.isArray(raw.days) || !Array.isArray(raw.trade_returns) || !Array.isArray(data.dates) || data.dates.length < 1
    || !Array.isArray(data.trades) || raw.trade_returns.length !== data.trades.length || !object(data.ohlc)) return invalid();
  const N = data.dates.length, arrays = ['close','equity','gross_equity','bh'];
  if (raw.days.length !== N || arrays.some(k => !Array.isArray(data[k]) || data[k].length !== N || data[k].some((v: unknown) => !positive(v)))
    || !exactKeys(data.ohlc, ['d','o','h','l','c','v']) || ['d','o','h','l','c','v'].some(k => !Array.isArray((data.ohlc as Fields)[k]) || ((data.ohlc as Fields)[k] as unknown[]).length !== N)) return invalid();
  const costs = data.costs;
  if (!object(costs) || !finite(costs.fees_bps) || !finite(costs.slippage_bps) || costs.fees_bps < 0 || costs.slippage_bps < 0
    || costs.fees_bps > 5000 || costs.slippage_bps > 5000 || costs.fees_bps + costs.slippage_bps > 5000
    || costs.total_side_bps !== costs.fees_bps + costs.slippage_bps || costs.rollover_bps !== 0 || costs.taxes !== null || costs.rolls !== null
    || ['basis','rollover_status','slippage_status','money_status','open_trade_cost_basis'].some(k => !text(costs[k]))
    || costs.fees_bps !== admission.costs.fees_bps || costs.slippage_bps !== admission.costs.slippage_bps || costs.rollover_bps !== admission.costs.rollover_bps) return invalid();
  const stats = checkStats(data.stats, FINANCIAL_STATS), grossStats = checkStats(data.gross_stats, COMMON_STATS);
  checkStats(data.benchmark_stats, BENCHMARK_STATS);
  const days: UnitExposureDay[] = [], first = raw.days[0], ohlc = data.ohlc;
  if (!object(first) || !integer(first.source_index)) return invalid();
  for (let i = 0; i < N; i++) {
    const d: unknown = raw.days[i], day: unknown = data.dates[i];
    if (!object(d) || !exactKeys(d, DAY_KEYS) || !date(day) || d.date !== day || !integer(d.source_index)
      || d.source_index !== first.source_index + i || (i > 0 && day <= (data.dates[i-1] as string))
      || (ohlc.d as unknown[])[i] !== day || ['o','h','l','c'].some(k => !positive((ohlc[k] as unknown[])[i]))
      || ((ohlc.v as unknown[])[i] !== null && !finite((ohlc.v as unknown[])[i]))) return invalid();
    const [o,h,l,c] = ['o','h','l','c'].map(k => (ohlc[k] as number[])[i]);
    if (l > Math.min(o,c) || Math.max(o,c) > h || c !== (data.close as number[])[i]
      || !positive(d.equity) || !positive(d.gross_equity) || !positive(d.benchmark_equity)
      || d.equity !== (data.equity as number[])[i] || d.gross_equity !== (data.gross_equity as number[])[i]
      || d.benchmark_equity !== (data.bh as number[])[i]
      || !scalar(d.net_daily_return) || !scalar(d.gross_daily_return) || !scalar(d.benchmark_daily_return)
      || !unitPriceReturnConsistent(i ? days[i-1].equity : 1, d.equity, d.net_daily_return)
      || !unitPriceReturnConsistent(i ? days[i-1].gross_equity : 1, d.gross_equity, d.gross_daily_return)
      || !unitPriceReturnConsistent(i ? days[i-1].benchmark_equity : 1, d.benchmark_equity, d.benchmark_daily_return)
      || !unitFactorConsistent((data.close as number[])[0], c, d.benchmark_equity)) return invalid();
    days.push(d as unknown as UnitExposureDay);
  }
  if ((data.dates[0] as string) < metadata.first || (data.dates[N-1] as string) > metadata.last
    || days[0].benchmark_equity !== 1) return invalid();
  const audits: UnitExposureTrade[] = [], covered = new Set<number>(); let lastExit = -1;
  for (let i = 0; i < raw.trade_returns.length; i++) {
    const a: unknown = raw.trade_returns[i], t: unknown = data.trades[i];
    if (!object(a) || !exactKeys(a, TRADE_KEYS) || !object(t) || !object(t.accounting)
      || a.trade_index !== i || !integer(a.entry_source_index) || !integer(a.exit_source_index)
      || typeof a.open !== 'boolean' || t.open !== a.open || !positive(a.entry_allocation)
      || TRADE_KEYS.filter(k => k.endsWith('_return') || k.endsWith('_contribution')).some(k => !scalar(a[k]))
      || !Array.isArray(t.legs) || t.legs.length !== 1 || !Array.isArray(t.legs[0]) || t.legs[0].length !== 3
      || !integer(t.legs[0][0]) || !positive(t.legs[0][1]) || t.legs[0][2] !== 1 || !integer(t.exit_i)
      || t.dir !== 1 || t.entry_kind !== 'close' || !Array.isArray(t.exits) || t.exits.length !== 0 || !text(t.reason)
      || !object(t.at_entry) || !object(t.at_exit)) return invalid();
    const entry = a.entry_source_index - first.source_index, exit = a.exit_source_index - first.source_index;
    if (entry < 0 || exit < entry || exit >= N || entry <= lastExit || t.legs[0][0] !== entry || t.exit_i !== exit
      || t.entry !== data.dates[entry] || t.exit !== data.dates[exit] || t.signal !== data.dates[exit]
      || t.avg_px !== t.legs[0][1] || t.avg_px !== (data.close as number[])[entry] || t.exit_px !== (data.close as number[])[exit]
      || t.bars !== exit-entry || (a.open ? i !== raw.trade_returns.length-1 || exit !== N-1 || exit !== entry : exit !== entry+1)
      || a.entry_allocation !== (entry ? days[entry-1].equity : 1)
      || t.ret !== a.net_return || t.ret_full_precision !== a.net_return) return invalid();
    const audit = a as unknown as UnitExposureTrade, account = t.accounting;
    if (!unitPriceReturnConsistent(t.avg_px as number, t.exit_px as number, audit.gross_return)
      || !unitFeeConsistent(costs.fees_bps, audit.fee_return) || !unitFeeConsistent(costs.slippage_bps, audit.slippage_return)
      || !unitReturnConsistent(audit.gross_return, audit.net_return, audit.fee_return, audit.slippage_return)
      || !unitProductConsistent(audit.entry_allocation, audit.gross_return, audit.gross_contribution)
      || !unitProductConsistent(audit.entry_allocation, audit.net_return, audit.net_contribution)
      || !unitProductConsistent(audit.entry_allocation, audit.fee_return, audit.fee_contribution)
      || !unitProductConsistent(audit.entry_allocation, audit.slippage_return, audit.slippage_contribution)
      || !unitCurveConsistent(audit.entry_allocation, days[exit].equity, audit.net_return)
      || !unitCurveConsistent(entry ? days[entry-1].gross_equity : 1, days[exit].gross_equity, audit.gross_return)) return invalid();
    if (!a.open && (!unitEntryCurveConsistent(audit.entry_allocation, days[entry].equity, audit.fee_return, audit.slippage_return)
      || days[entry].gross_equity !== (entry ? days[entry-1].gross_equity : 1))) return invalid();
    if (account.symbol !== admission.ticker || account.signal_side !== 'Long' || account.pts !== (t.exit_px as number)-(t.avg_px as number)
      || account.days_held !== (Date.parse(t.exit as string)-Date.parse(t.entry as string))/86400000
      || account.gross_ret !== audit.gross_return || account.net_ret !== audit.net_return
      || account.fee_ret !== audit.fee_return || account.slippage_ret !== audit.slippage_return
      || account.cost_status !== (a.open ? 'open_liquidation_estimate' : 'modeled_round_trip_cost')
      || ['gross_pnl','fees','slippage','taxes','net_pnl','mae','mfe','mae_ret_observed','mfe_ret_observed'].some(k => account[k] !== null)
      || ['money_status','return_basis','pts_basis','excursion_basis'].some(k => !text(account[k]))
      || account.excursion_bars !== 0 || account.excursion_status !== 'unavailable') return invalid();
    for (let j = entry; j <= exit; j++) covered.add(j);
    lastExit = exit; audits.push(audit);
  }
  for (let i = 0; i < N; i++) if (!covered.has(i) && (days[i].equity !== (i ? days[i-1].equity : 1)
    || days[i].gross_equity !== (i ? days[i-1].gross_equity : 1))) return invalid();
  const completed = audits.filter(t => !t.open);
  for (const [s, key, eq] of [[stats,'net_return','equity'],[grossStats,'gross_return','gross_equity']] as const) {
    const returns = completed.map(t => t[key]), n = returns.length, closedSum = sum(returns), wins = returns.filter(v => v > 0).length;
    let streak = 0, longest = 0; for (const r of returns) {streak = r <= 0 ? streak+1 : 0; longest = Math.max(longest,streak);}
    const open = audits.find(t => t.open), positiveReturns = returns.filter(v => v > 0), nonwins = returns.filter(v => v <= 0);
    const averageWin = sum(positiveReturns)/positiveReturns.length, averageLoss = sum(nonwins)/nonwins.length;
    const ordered = [...returns].sort((a,b) => a-b), median = n ? n%2 ? ordered[Math.floor(n/2)] : (ordered[n/2-1]+ordered[n/2])/2 : NaN;
    if (s.avg_win !== unitStatisticDisplay(averageWin) || s.avg_loss !== unitStatisticDisplay(averageLoss)
      || s.payoff !== unitStatisticDisplay(averageWin/-averageLoss)
      || s.profit_factor !== unitStatisticDisplay(sum(positiveReturns)/-sum(nonwins))
      || s.best !== unitStatisticDisplay(n ? Math.max(...returns) : NaN)
      || s.worst !== unitStatisticDisplay(n ? Math.min(...returns) : NaN) || s.median_trade !== unitStatisticDisplay(median)
      || ((n < 2 || sum(returns.map(v => (v-closedSum/n)*(v-closedSum/n))) === 0) && s.t_stat !== null)
      || ((!nonwins.length || averageLoss === 0) && s.expectancy_ratio !== null)) return invalid();
    const daily = days.map(d => d[eq === 'equity' ? 'net_daily_return' : 'gross_daily_return']);
    const dailyMean = sum(daily)/N, dailyVarianceNumerator = sum(daily.map(v => (v-dailyMean)*(v-dailyMean)));
    if ((sum(daily.map(v => Math.min(v,0)*Math.min(v,0))) === 0 && s.sortino !== null)
      || (dailyVarianceNumerator === 0 && (s.sharpe !== null || s.vol !== 0))) return invalid();
    if (s.trades !== n || s.win_rate !== unitStatisticDisplay(n ? wins/n : NaN)
      || s.avg_trade !== unitStatisticDisplay(n ? closedSum/n : NaN) || s.net_profit !== unitStatisticDisplay(closedSum)
      || s.closed_return_sum !== s.net_profit || s.max_consec_losses !== longest
      || s.open_trade !== unitStatisticDisplay(open ? open[key] : NaN)
      || s.total_return !== unitStatisticDisplay(days[N-1][eq]-1) || s.compounded_return !== s.total_return
      || s.exposure !== unitStatisticDisplay(covered.size/N) || s.avg_bars !== (n ? 1 : null)
      || s.avg_deployed !== (n ? 1 : null) || !finite(s.max_dd) || s.max_dd > 0
      || !integer(s.max_dd_days) || s.max_dd_days > N) return invalid();
  }
  if (stats.avg_net_pnl !== stats.avg_trade || stats.ledger_gross_return !== unitStatisticDisplay(sum(audits.map(t => t.gross_contribution)))
    || stats.fee_return !== unitStatisticDisplay(sum(audits.map(t => t.fee_contribution)))
    || stats.slippage_return !== unitStatisticDisplay(sum(audits.map(t => t.slippage_contribution)))) return invalid();
  return {model: UNIT_EXPOSURE_MODEL, report: raw as unknown as UnitExposureReport, basis: raw.basis,
    ticker: admission.ticker, region: admission.region, closedReturnSum: sum(completed.map(t => t.net_return)), closedTrades: completed.length};
}
/** Uncached strict entry point; cache/freezing is integrated only after runtime callsite review. */
export function parseUnitExposureReport(data: unknown, admission: UnitExposureAdmission): UnitExposureView {
  try {return core(data, admission, false);} catch {return invalid();}
}
const validated = new WeakMap<object, UnitExposureView>();
const protectedViews = new WeakSet<object>();
const FINANCIAL_ROOTS = ['id','book','ticker','dates','close','equity','gross_equity','bh','trades','ohlc',
  'unit_exposure_report','model_initial_equity','stats','gross_stats','benchmark_symbol','benchmark_stats',
  'costs','accounting_basis','provenance'];
function freezeFinancial(value: unknown, seen = new WeakSet<object>()) {
  if (!value || typeof value !== 'object' || seen.has(value)) return;
  seen.add(value);
  for (const child of Object.values(value)) freezeFinancial(child, seen);
  Object.freeze(value);
}
function cheapContext(data: unknown, admission: UnitExposureAdmission) {
  if (!object(data) || Object.hasOwn(data,'source_account') || Object.hasOwn(data,'kind') || !object(data.unit_exposure_report)) return invalid();
  const {policy,metadata}=context(data,admission,false), r=data.unit_exposure_report, costs=data.costs;
  if (r.model !== UNIT_EXPOSURE_MODEL || r.id !== admission.id || r.book !== admission.book
    || r.source_convention !== UNIT_EXPOSURE_CONVENTION || r.basis !== policy.source_basis
    || r.basis !== data.accounting_basis || !object(costs)
    || costs.fees_bps !== admission.costs.fees_bps || costs.slippage_bps !== admission.costs.slippage_bps
    || costs.rollover_bps !== admission.costs.rollover_bps || !Array.isArray(data.dates) || !data.dates.length) return invalid();
  const first=data.dates[0], last=data.dates.at(-1), from=admission.window?.from, to=admission.window?.to;
  if (!date(first) || !date(last) || first < metadata.first || last > metadata.last
    || (from && (!date(from) || first < from)) || (to && (!date(to) || last > to))
    || (from && to && from > to)) return invalid();
  return data;
}
/** One full audit per real response; all financial references are protected before reuse.
 * Every use still binds the current selector, policy, parameters, costs and dates.
 * The root stays extensible for nonfinancial diagnostics such as _ms. */
export function validatedUnitExposureReport(data: unknown, admission: UnitExposureAdmission): UnitExposureView {
  try {
    const root=cheapContext(data,admission), cached=validated.get(root);
    if (cached) return cached;
    const view=core(root,admission,false);
    for (const key of FINANCIAL_ROOTS) {
      if (!Object.hasOwn(root,key)) return invalid();
      freezeFinancial(root[key]);
      Object.defineProperty(root,key,{value:root[key],writable:false,configurable:false,enumerable:true});
    }
    Object.freeze(view); protectedViews.add(view); validated.set(root,view);
    return view;
  } catch {return invalid();}
}
export function isValidatedUnitExposureView(value: unknown): value is UnitExposureView {
  return object(value) && protectedViews.has(value);
}
export function isValidatedUnitExposurePair(data:unknown,view:unknown):view is UnitExposureView {
  return object(data) && isValidatedUnitExposureView(view) && validated.get(data)===view;
}
/** Explicit offline reference projections only; public admission never reads this flag. */
export function parseUnitExposureAdapter(data: unknown, admission: UnitExposureAdmission, binding: unknown): UnitExposureView {
  if (!object(binding) || binding.scope !== 'independent_reference_projection_test_only' || !text(binding.case_id)
    || typeof binding.reference_sha256 !== 'string' || !/^[a-f0-9]{64}$/.test(binding.reference_sha256)) return invalid();
  try {return core(data, admission, true);} catch {return invalid();}
}
