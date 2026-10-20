import {
  roundingCell, unitReturnConsistent, unitPriceReturnConsistent,
  unitFactorConsistent, unitProductConsistent,
} from './unit-exposure-rounding';

/** Separate Pristine wire types; these do not admit a response by themselves.
 * Validation lives below; presentation and runtime integration remain separate.
 * Existing DEMA/index and funded-account report contracts remain unchanged. */
export const PRISTINE_BOOK = 'p_tpm1';
export const PRISTINE_MODEL = 'pristine_daily_supplied_series_unit_exposure_v1';
export const PRISTINE_CONVENTION = 'tpm1_daily_source_adaptation_v1';
export const PRISTINE_RESEARCH_MODE = 'supplied_usd_series_units';
export const PRISTINE_METHODS = ['ptpm1_daily_cbs_cog', 'ptpm1_bullish_gap_surprise'] as const;
export type PristineMethod = typeof PRISTINE_METHODS[number];
export const pristineMethod = (id?: string, book?: string): id is PristineMethod =>
  book === PRISTINE_BOOK && PRISTINE_METHODS.some(method => method === id);

export const PRISTINE_COMMON_STATS = [
  'total_return','cagr','vol','sharpe','sortino','max_dd','max_dd_days','mar','exposure',
  'bh_total','bh_cagr','bh_max_dd','years','trades','win_rate','avg_trade','median_trade',
  'avg_win','avg_loss','payoff','profit_factor','best','worst','t_stat','max_consec_losses',
  'avg_bars','avg_deployed','open_trade','trades_per_year','net_profit','closed_return_sum',
  'compounded_return','car_mdd','expectancy_ratio','avg_hold_days','monthly_thickness',
] as const;
export const PRISTINE_FINANCIAL_STATS = [...PRISTINE_COMMON_STATS,
  'gross_total_return','gross_avg_trade','gross_win_rate','gross_sharpe','gross_max_dd',
  'avg_net_pnl','ledger_gross_return','fee_return','slippage_return',
] as const;
export const PRISTINE_BENCHMARK_STATS = ['total_return','cagr','sharpe','max_dd','vol','sortino','mar'] as const;
export type PristineStatKey = typeof PRISTINE_FINANCIAL_STATS[number];
export interface PristineStatMap<K extends string = PristineStatKey> {
  readonly values: Readonly<Record<K, number | null>>;
  readonly display_values: Readonly<Record<K, number | null>>;
  readonly null_reasons: Readonly<Partial<Record<K, string>>>;
}
export interface PristineExactAudit {
  readonly negative: boolean;
  readonly numerator_hex: string;
  readonly denominator_hex: string;
}
export type PristineVolume =
  | {readonly state: 'value'; readonly value: number; readonly token: null}
  | {readonly state: 'missing'; readonly value: null; readonly token: null}
  | {readonly state: 'malformed' | 'nonfinite'; readonly value: null; readonly token: string}
  | {readonly state: 'negative'; readonly value: number; readonly token: null};
export interface PristineRow {
  readonly source_index: number;
  readonly date: string;
  readonly open: number;
  readonly high: number;
  readonly low: number;
  readonly close: number;
  readonly volume: PristineVolume;
}
export interface PristineSetup {
  readonly signal_i: number;
  readonly trigger: PristineExactAudit | null;
  readonly stop: PristineExactAudit;
  readonly target: PristineExactAudit | null;
  readonly signal_close: PristineExactAudit;
}
export interface PristinePosition {
  readonly signal_i: number;
  readonly entry_i: number;
  readonly entry_price: number;
  readonly stop: PristineExactAudit;
  readonly target: PristineExactAudit;
}
export interface PristineEvent {
  readonly i: number;
  readonly kind: string;
  readonly reason: string;
  readonly phase: string;
  readonly signal_i: number;
  readonly entry_i: number | null;
  readonly price: number | null;
  readonly exact_price: PristineExactAudit | null;
  readonly stop: PristineExactAudit | null;
  readonly target: PristineExactAudit | null;
  readonly previous_stop: PristineExactAudit | null;
  readonly setup: PristineSetup | null;
  readonly ambiguity: readonly string[];
}
export interface PristineSpan {
  readonly signal_i: number;
  readonly entry_i: number;
  readonly exit_i: number;
  readonly entry_price: number;
  readonly exit_or_mark_price: number;
  readonly open: boolean;
  readonly entry_phase: 'open' | 'stop';
  readonly exit_or_mark_phase: 'open_stop' | 'open_target' | 'open_full_gap' | 'stop' | 'target' | 'close' | 'mark';
}
export interface PristineDay {
  readonly source_index: number;
  readonly date: string;
  readonly equity: number;
  readonly gross_equity: number;
  readonly benchmark_equity: number;
  readonly net_daily_return: number;
  readonly gross_daily_return: number;
  readonly benchmark_daily_return: number;
}
export interface PristineTradeReturn {
  readonly trade_index: number;
  readonly entry_source_index: number;
  readonly exit_source_index: number;
  readonly open: boolean;
  readonly entry_allocation: number;
  readonly gross_return: number;
  readonly net_return: number;
  readonly fee_return: number;
  readonly slippage_return: number;
  readonly gross_contribution: number;
  readonly net_contribution: number;
  readonly fee_contribution: number;
  readonly slippage_contribution: number;
}
export interface PristineDaySupplement {
  readonly observed_exposure_session: boolean;
  readonly paid_fee_contribution: number;
  readonly paid_slippage_contribution: number;
  readonly estimated_exit_fee_contribution: number;
  readonly estimated_exit_slippage_contribution: number;
}
export interface PristineTradeSupplement {
  readonly gross_factor: number;
  readonly net_factor: number;
  readonly gross_curve_entry_allocation: number;
  readonly paid_sides: number;
  readonly estimated_exit_sides: number;
  readonly calendar_days_held: number;
  readonly observed_exposure_sessions: number;
  readonly paid_fee_return: number;
  readonly paid_fee_contribution: number;
  readonly paid_slippage_return: number;
  readonly paid_slippage_contribution: number;
  readonly estimated_exit_fee_return: number;
  readonly estimated_exit_fee_contribution: number;
  readonly estimated_exit_slippage_return: number;
  readonly estimated_exit_slippage_contribution: number;
}
export interface PristineClosedView {
  readonly closed_count: number;
  readonly closed_wins: number;
  readonly closed_nonwins: number;
  readonly closed_return_sum: number;
  readonly closed_average_return: number | null;
  readonly closed_win_rate: number | null;
  readonly closed_compound_equity: number;
  readonly closed_compound_return: number;
}
export interface PristineReport {
  readonly model: typeof PRISTINE_MODEL;
  readonly convention: typeof PRISTINE_CONVENTION;
  readonly kind: 'single_symbol_supplied_series_research';
  readonly owner: 'pristine';
  readonly book: typeof PRISTINE_BOOK;
  readonly id: PristineMethod;
  readonly research_mode: typeof PRISTINE_RESEARCH_MODE;
  readonly context: 'broad_us_supplied_series';
  readonly market: 'us';
  readonly currency: 'USD';
  readonly asset_type: 'individual_stock';
  readonly timeframe: 'D';
  readonly qualified_ticker: string;
  readonly symbol: string;
  readonly from_source_index: number;
  readonly to_source_index: number;
  readonly source_row_count: number;
  readonly source_prefix_row_count: number;
  readonly volume_column_present: boolean;
  readonly basis: string;
  readonly provenance_json: string;
  readonly claim: null;
  readonly book_performance_reproduced: false;
  readonly native_platform_parity: false;
  readonly native_historical_dollars_verified: false;
  readonly historical_membership_verified: false;
  readonly benchmark: {
    readonly kind: 'selected_symbol_buy_hold';
    readonly qualified_ticker: string;
    readonly symbol: string;
  };
  readonly costs: {
    readonly fees_bps: number;
    readonly slippage_bps: number;
    readonly rollover_bps: 0;
    readonly total_side_bps: number;
    readonly basis: string;
  };
  readonly rows: readonly PristineRow[];
  readonly source: {
    readonly signals: readonly {readonly setup: PristineSetup; readonly flat_permission: boolean}[];
    readonly events: readonly PristineEvent[];
    readonly open_position: PristinePosition | null;
    readonly pending_order: PristineSetup | null;
  };
  readonly spans: readonly PristineSpan[];
  readonly days: readonly PristineDay[];
  readonly trade_returns: readonly PristineTradeReturn[];
  readonly day_supplements: readonly PristineDaySupplement[];
  readonly trade_supplements: readonly PristineTradeSupplement[];
  readonly closed_stats: {
    readonly net: PristineClosedView;
    readonly gross: PristineClosedView;
    readonly net_max_drawdown: number;
    readonly gross_max_drawdown: number;
    readonly observed_exposure_sessions: number;
    readonly selected_sessions: number;
    readonly exposure: number;
  };
  readonly statistics: {
    readonly net: PristineStatMap<(typeof PRISTINE_COMMON_STATS)[number]>;
    readonly gross: PristineStatMap<(typeof PRISTINE_COMMON_STATS)[number]>;
    readonly financial: PristineStatMap;
    readonly benchmark: PristineStatMap<(typeof PRISTINE_BENCHMARK_STATS)[number]>;
  };
}
/** Caller context must come from current controls and actual runtime metadata.
 * It is not a way to turn an arbitrary response into an admitted instrument. */
export interface PristineAdmission {
  readonly id: string;
  readonly book: string;
  readonly ticker: string;
  readonly region: string;
  readonly researchMode: string;
  readonly definition: unknown;
  readonly parameters: unknown;
  readonly instrument: unknown;
  readonly costs: {readonly fees_bps: number; readonly slippage_bps: number; readonly rollover_bps: number};
  readonly window?: {readonly from?: string; readonly to?: string};
}


export interface PristineInstrumentContext {
  readonly kind: 'individual_stock';
  readonly classification_basis: 'native_stock_metadata';
  readonly metadata: unknown;
}
export interface PristineView {
  readonly report: PristineReport;
  readonly context: PristineAdmission;
}

// These are response-consistency checks, not a browser strategy calculator or
// a replacement for the server's source/data qualification. Earlier prefix
// rows are deliberately not reconstructed from the selected-window payload.
type JsonObject = Record<string, unknown>;
const DATA_BASIS = 'Explicit supplied-USD-series-unit research on unchanged vendor prices. Constants are stored series units; historical-dollar quote/adjustment equivalence and book-native performance are not verified. Current/Past watchlists remain snapshots, not historical membership.';
const COST_BASIS = 'Separate return deductions on deployed normalized entry allocation; slippage does not rewrite fills. Closed trades pay two sides. Terminal open marks retain one paid entry side and one estimated exit side; no exit transaction.';
const SCALE = 1n << 1075n;
const MAX_SELECTED_ROWS = 100_000;
const fail = (where: string): never => { throw new Error(`Invalid Pristine report: ${where}`); };
function requireThat(condition: unknown, where: string): asserts condition {
  if (!condition) fail(where);
}
function object(value: unknown, where: string): JsonObject {
  requireThat(value !== null && typeof value === 'object' && !Array.isArray(value), where);
  const prototype = Object.getPrototypeOf(value);
  requireThat(prototype === Object.prototype || prototype === null, `${where} prototype`);
  for (const descriptor of Object.values(Object.getOwnPropertyDescriptors(value))) {
    requireThat('value' in descriptor && descriptor.enumerable, `${where} data fields`);
  }
  requireThat(Object.getOwnPropertySymbols(value).length === 0, `${where} symbols`);
  return value as JsonObject;
}
function keys(value: unknown, expected: readonly string[], where: string): JsonObject {
  const result = object(value, where);
  const actual = Object.keys(result);
  requireThat(actual.length === expected.length && expected.every(key => Object.hasOwn(result, key)), `${where} fields`);
  return result;
}
function list(value: unknown, where: string, limit = MAX_SELECTED_ROWS): unknown[] {
  requireThat(Array.isArray(value) && value.length <= limit, where);
  requireThat(Object.keys(value).length === value.length, `${where} dense array`);
  return value;
}
const finite = (value: unknown): value is number => typeof value === 'number' && Number.isFinite(value);
const scalar = (value: unknown): value is number => finite(value) && !Object.is(value, -0);
const positive = (value: unknown): value is number => scalar(value) && value > 0;
const integer = (value: unknown): value is number => Number.isSafeInteger(value) && (value as number) >= 0;
const text = (value: unknown): value is string => typeof value === 'string' && value.length > 0 && value.length <= 1_048_576;
const equalNumber = (a: unknown, b: unknown) => typeof a === 'number' && typeof b === 'number' && Object.is(a, b);
function date(value: unknown): value is string {
  if (typeof value !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
  const time = Date.parse(`${value}T00:00:00Z`);
  return Number.isFinite(time) && new Date(time).toISOString().slice(0, 10) === value;
}
const calendarDays = (first: string, last: string) => (Date.parse(`${last}T00:00:00Z`) - Date.parse(`${first}T00:00:00Z`)) / 86_400_000;

// JSON-only snapshots also bind contextual flags/unknown provenance fields.
// Signed zero is distinguished in cache signatures; no number formatter parity
// with pinned Rust Display is asserted here.
function signature(value: unknown): string {
  const active = new Set<object>();
  let nodes = 0;
  function visit(current: unknown): string {
    requireThat(++nodes <= 2_000_000, 'context size');
    if (current === null) return 'null';
    if (typeof current === 'string') { requireThat(current.length <= 4_194_304, 'string size'); return JSON.stringify(current); }
    if (typeof current === 'boolean') return current ? 'true' : 'false';
    if (finite(current)) return Object.is(current, -0) ? '-0' : String(current);
    requireThat(typeof current === 'object' && current !== null && !active.has(current), 'JSON value');
    active.add(current);
    let result: string;
    if (Array.isArray(current)) {
      list(current, 'JSON array', 1_000_000);
      result = `[${current.map(visit).join(',')}]`;
    } else {
      const obj = object(current, 'JSON object');
      result = `{${Object.keys(obj).sort().map(key => `${JSON.stringify(key)}:${visit(obj[key])}`).join(',')}}`;
    }
    active.delete(current);
    return result;
  }
  return visit(value);
}
const same = (a: unknown, b: unknown) => signature(a) === signature(b);
function protectedClone<T>(value: T): T {
  if (Array.isArray(value)) return Object.freeze(value.map(item => protectedClone(item))) as T;
  if (value !== null && typeof value === 'object') {
    const result: JsonObject = Object.create(Object.getPrototypeOf(value)) as JsonObject;
    for (const [key, item] of Object.entries(value)) Object.defineProperty(result, key, {value: protectedClone(item), enumerable: true});
    return Object.freeze(result) as T;
  }
  return value;
}
function protect(value: unknown): void {
  if (value === null || typeof value !== 'object') return;
  for (const child of Object.values(value)) protect(child);
  Object.freeze(value);
}
const hash = (value: unknown) => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
const FALSE_EVIDENCE = ['source_price_units_verified', 'historical_membership_verified', 'corporate_actions_independently_verified', 'quote_accuracy_independently_verified'] as const;
function nativeMetadata(value: unknown, ticker: string): JsonObject {
  const metadata = object(value, 'native metadata');
  requireThat(metadata.ticker === ticker && metadata.native === true && text(metadata.symbol) && metadata.symbol !== ticker, 'native ticker identity');
  requireThat(metadata.trusted_market === 'us' && metadata.currency === 'USD' && metadata.asset_type === 'individual_stock' && metadata.timeframe === 'D', 'native stock scope');
  requireThat(date(metadata.first) && date(metadata.last) && metadata.first <= metadata.last, 'native coverage');
  const provenance = object(metadata.provenance, 'native provenance');
  requireThat(provenance.qualified_ticker === ticker && provenance.symbol === metadata.symbol, 'provenance identity');
  requireThat(provenance.trusted_market === 'us' && provenance.currency === 'USD' && provenance.asset_type === 'individual_stock' && provenance.timeframe === 'D', 'provenance scope');
  requireThat(provenance.first === metadata.first && provenance.last === metadata.last && integer(provenance.bars) && provenance.bars > 0, 'provenance coverage');
  if (Object.hasOwn(metadata, 'bars')) requireThat(metadata.bars === provenance.bars, 'native row count alias');
  requireThat(hash(provenance.source_sha256) && hash(provenance.csv_sha256) && text(provenance.source), 'source hashes');
  requireThat(provenance.adjustment_status === 'not_independently_verified', 'unverified adjustment');
  for (const key of FALSE_EVIDENCE) {
    requireThat(provenance[key] === false, `provenance ${key}`);
    if (Object.hasOwn(metadata, key)) requireThat(metadata[key] === false, `metadata ${key}`);
  }
  for (const key of ['source_sha256', 'csv_sha256', 'adjustment_status'] as const) {
    if (Object.hasOwn(metadata, key)) requireThat(metadata[key] === provenance[key], `metadata ${key}`);
  }
  return metadata;
}
/** Classification comes only from one explicitly native runtime metadata row. */
export function pristineInstrumentContext(ticker: string, region: string, runtimeRows: readonly unknown[]): PristineInstrumentContext | null {
  try {
    if (!text(ticker) || region !== 'us' || !Array.isArray(runtimeRows)) return null;
    const matches = runtimeRows.filter(row => object(row, 'runtime ticker').ticker === ticker);
    if (matches.length !== 1) return null;
    nativeMetadata(matches[0], ticker);
    return {kind: 'individual_stock', classification_basis: 'native_stock_metadata', metadata: matches[0]};
  } catch { return null; }
}
function admissionContext(value: PristineAdmission): {metadata: JsonObject; signature: string} {
  const context = object(value, 'admission');
  requireThat(pristineMethod(value.id, value.book) && value.region === 'us' && value.researchMode === PRISTINE_RESEARCH_MODE && text(value.ticker), 'admission method/mode');
  keys(value.parameters, [], 'parameters');
  const definition = object(value.definition, 'definition');
  requireThat(definition.id === value.id && definition.book === PRISTINE_BOOK, 'definition identity');
  if (Object.hasOwn(definition, 'owner')) requireThat(definition.owner === 'pristine', 'definition owner');
  const policy = object(definition.instrument_policy, 'instrument policy');
  const required: JsonObject = {
    requirement: 'individual_stock', execution_scope: 'single_symbol_supplied_series_research',
    source_convention: PRISTINE_CONVENTION, research_mode: PRISTINE_RESEARCH_MODE,
    source_rule_reviewed: true, independent_selected_ledger_checked: true,
    source_performance_reproduced: false, market_adaptation: true,
  };
  for (const [key, expected] of Object.entries(required)) requireThat(policy[key] === expected, `policy ${key}`);
  requireThat(same(policy.allowed_types, ['individual_stock']), 'policy allowed types');
  const instrument = keys(value.instrument, ['kind', 'classification_basis', 'metadata'], 'instrument');
  requireThat(instrument.kind === 'individual_stock' && instrument.classification_basis === 'native_stock_metadata', 'instrument classification');
  const metadata = nativeMetadata(instrument.metadata, value.ticker);
  const costs = keys(value.costs, ['fees_bps', 'slippage_bps', 'rollover_bps'], 'selected costs');
  requireThat(finite(costs.fees_bps) && costs.fees_bps >= 0 && costs.fees_bps <= 5000 && finite(costs.slippage_bps) && costs.slippage_bps >= 0 && costs.slippage_bps <= 5000 && finite(costs.rollover_bps) && costs.rollover_bps === 0, 'selected cost domain');
  // Exact canonical cost admission remains the server's responsibility. The
  // client checks the public bound without creating a different decimal model.
  requireThat(costs.fees_bps + costs.slippage_bps <= 5000, 'selected combined costs');
  if (value.window !== undefined) {
    const window = object(value.window, 'selected window');
    requireThat(Object.keys(window).every(key => key === 'from' || key === 'to'), 'window fields');
    requireThat((window.from === undefined || date(window.from)) && (window.to === undefined || date(window.to)), 'window dates');
    requireThat(!window.from || !window.to || (window.from as string) <= (window.to as string), 'window order');
  }
  requireThat(Object.keys(context).every(key => ['id','book','ticker','region','researchMode','definition','parameters','instrument','costs','window'].includes(key)), 'admission fields');
  // Omitted optional window stays omitted in its protected snapshot.
  const stable = {...context};
  if (stable.window === undefined) delete stable.window;
  return {metadata, signature: signature(stable)};
}

interface Fraction {readonly numerator: bigint; readonly denominator: bigint}
const auditCache = new WeakMap<object, Fraction>();
function audit(value: unknown, where: string): Fraction {
  const obj = keys(value, ['negative','numerator_hex','denominator_hex'], where);
  const cached = auditCache.get(obj);
  if (cached && Object.isFrozen(obj)) return cached;
  requireThat(typeof obj.negative === 'boolean' && typeof obj.numerator_hex === 'string' && typeof obj.denominator_hex === 'string', `${where} audit types`);
  requireThat(/^(0|[1-9a-f][0-9a-f]*)$/.test(obj.numerator_hex) && /^[1-9a-f][0-9a-f]*$/.test(obj.denominator_hex) && obj.numerator_hex.length <= 8192 && obj.denominator_hex.length <= 8192, `${where} audit digits`);
  const magnitude = BigInt(`0x${obj.numerator_hex}`), denominator = BigInt(`0x${obj.denominator_hex}`);
  requireThat(magnitude !== 0n || obj.negative === false, `${where} signed zero`);
  const result = {numerator: obj.negative ? -magnitude : magnitude, denominator};
  auditCache.set(obj, result);
  return result;
}
const compare = (a: Fraction, b: Fraction) => {
  const difference = a.numerator * b.denominator - b.numerator * a.denominator;
  return difference < 0n ? -1 : difference > 0n ? 1 : 0;
};
const equalAudit = (a: unknown, b: unknown) => compare(audit(a, 'audit alias'), audit(b, 'audit alias')) === 0;
function auditNumber(value: unknown, number: number, where: string): void {
  const a = audit(value, where), cell = roundingCell(number);
  requireThat(scalar(number) && a.numerator * SCALE >= cell[0] * a.denominator && a.numerator * SCALE <= cell[1] * a.denominator, `${where} rounded scalar`);
}
const positiveAudit = (value: unknown, where: string) => {
  const result = audit(value, where); requireThat(result.numerator > 0n, `${where} positive`); return result;
};
function sideCost(bps: number, sides: number, value: number): boolean {
  if (!scalar(value) || value < 0 || !integer(sides)) return false;
  if (bps === 0 || sides === 0) return value === 0;
  if (value === 0) return false;
  const c = roundingCell(bps), v = roundingCell(value), n = BigInt(sides);
  return c[0] * n <= v[1] * 10000n && c[1] * n >= v[0] * 10000n;
}
function factorReturn(factor: number, value: number): boolean {
  if (!positive(factor) || !scalar(value)) return false;
  const f = roundingCell(factor), v = roundingCell(value);
  return f[0] <= SCALE + v[1] && f[1] >= SCALE + v[0];
}
// Exact preimages of the ordinary rounded allocation multiplication. A quotient
// of two reported doubles is not necessarily the original rounded factor.
const factorBuffer = new ArrayBuffer(8), factorBits = new DataView(factorBuffer);
function factorNumber(bits: bigint): number { factorBits.setBigUint64(0,bits,false); return factorBits.getFloat64(0,false); }
function productFactors(allocation: number, equity: number): readonly [bigint,bigint] | null {
  if (!positive(allocation) || !positive(equity)) return null;
  const maximum = 0x7fefffffffffffffn;
  let low = 1n, high = maximum + 1n;
  while (low < high) { const middle = (low + high) >> 1n; if (allocation * factorNumber(middle) < equity) low = middle + 1n; else high = middle; }
  const first = low;
  if (first > maximum || allocation * factorNumber(first) !== equity) return null;
  high = maximum + 1n;
  while (low < high) { const middle = (low + high) >> 1n; if (allocation * factorNumber(middle) <= equity) low = middle + 1n; else high = middle; }
  return [roundingCell(factorNumber(first))[0],roundingCell(factorNumber(low-1n))[1]];
}
function factorCellsWithCosts(entry: number, mark: number, factors: readonly [bigint,bigint], fees: number, slip: number, sides: number): boolean {
  const e = roundingCell(entry), x = roundingCell(mark);
  const c = roundingCell(fees), s = roundingCell(slip), n = BigInt(sides);
  const low = factors[0] * 10000n + n * (c[0] + s[0]);
  const high = factors[1] * 10000n + n * (c[1] + s[1]);
  return high * e[1] >= x[0] * SCALE * 10000n && low * e[0] <= x[1] * SCALE * 10000n;
}
function priceFactorWithCosts(entry: number, mark: number, factor: number, fees: number, slip: number, sides: number): boolean {
  return positive(factor) && factorCellsWithCosts(entry,mark,roundingCell(factor),fees,slip,sides);
}
function markedCurve(entry: number, mark: number, allocation: number, equity: number, fees: number, slip: number, sides: number): boolean {
  const factors = productFactors(allocation,equity);
  return factors !== null && factorCellsWithCosts(entry,mark,factors,fees,slip,sides);
}

function volume(value: unknown): void {
  const obj = keys(value, ['state','value','token'], 'volume');
  if (obj.state === 'value') requireThat(finite(obj.value) && obj.value >= 0 && obj.token === null, 'known volume');
  else if (obj.state === 'negative') requireThat(finite(obj.value) && obj.value < 0 && obj.token === null, 'negative volume evidence');
  else if (obj.state === 'missing') requireThat(obj.value === null && obj.token === null, 'missing volume evidence');
  else requireThat((obj.state === 'malformed' || obj.state === 'nonfinite') && obj.value === null && text(obj.token), 'typed volume evidence');
}
function setup(value: unknown, report: PristineReport): PristineSetup {
  const obj = keys(value, ['signal_i','trigger','stop','target','signal_close'], 'setup');
  requireThat(integer(obj.signal_i) && obj.signal_i >= report.from_source_index && obj.signal_i <= report.to_source_index, 'setup coordinate');
  positiveAudit(obj.stop, 'setup stop');
  positiveAudit(obj.signal_close, 'signal close');
  auditNumber(obj.signal_close, report.rows[obj.signal_i - report.from_source_index].close, 'signal close');
  if (report.id === 'ptpm1_daily_cbs_cog') {
    positiveAudit(obj.trigger, 'CBS trigger'); positiveAudit(obj.target, 'CBS target');
  } else requireThat(obj.trigger === null && obj.target === null, 'BGS setup levels');
  return obj as unknown as PristineSetup;
}
const EVENT_FIELDS = ['i','kind','reason','phase','signal_i','entry_i','price','exact_price','stop','target','previous_stop','setup','ambiguity'];
const SPAN_FIELDS = ['signal_i','entry_i','exit_i','entry_price','exit_or_mark_price','open','entry_phase','exit_or_mark_phase'];
const FLAGS = ['entry_day_low_may_precede_breakout_conservative_stop_assumed','daily_stop_target_order_unseen_stop_first'];
function sourceAndSpans(report: PristineReport): void {
  keys(report.source, ['signals','events','open_position','pending_order'], 'source');
  list(report.source.signals, 'signals'); list(report.source.events, 'events', 8 * report.rows.length + 2);
  const signals = new Map<number, {setup: PristineSetup; flat_permission: boolean}>();
  let lastSignal = report.from_source_index - 1;
  for (const signal of report.source.signals) {
    keys(signal, ['setup','flat_permission'], 'source signal');
    setup(signal.setup, report);
    requireThat(typeof signal.flat_permission === 'boolean' && !signals.has(signal.setup.signal_i), 'unique source signal');
    requireThat(signal.setup.signal_i > lastSignal, 'signal order');
    lastSignal = signal.setup.signal_i;
    signals.set(signal.setup.signal_i, signal);
  }
  let pending: PristineSetup | null = null;
  let held: PristinePosition | null = null;
  let entry: PristineEvent | null = null;
  let lastI = report.from_source_index;
  let terminal = false, pendingAtEnd = false;
  const derivedSpans: PristineSpan[] = [];
  const handledSignals = new Set<number>();
  const flatAtClose = new Map<number, boolean>();
  const exitPhases: Record<string, PristineSpan['exit_or_mark_phase']> = {
    protective_at_open:'open_stop', target_at_open:'open_target', cbs_full_gap_at_open:'open_full_gap',
    protective_intraday:'stop', target_intraday:'target', second_held_day_close:'close',
  };
  for (const event of report.source.events) {
    keys(event, EVENT_FIELDS, 'source event');
    requireThat(integer(event.i) && event.i >= lastI && event.i >= report.from_source_index && event.i <= report.to_source_index && integer(event.signal_i) && event.signal_i >= report.from_source_index && event.signal_i <= event.i, 'event coordinate/order');
    if (event.i > lastI) {
      flatAtClose.set(lastI, held === null);
      requireThat(!pending || pending.signal_i + 1 === event.i, 'pending resolution on next selected session');
    }
    lastI = event.i;
    requireThat(!terminal && !pendingAtEnd, 'events after terminal evidence');
    list(event.ambiguity, 'ambiguity', 2);
    requireThat(new Set(event.ambiguity).size === event.ambiguity.length && event.ambiguity.every(flag => FLAGS.includes(flag)), 'ambiguity flags');
    const row = report.rows[event.i - report.from_source_index];
    if (event.price === null) requireThat(event.exact_price === null, 'null event price audit');
    else { requireThat(positive(event.price), 'event price'); auditNumber(event.exact_price, event.price, 'event price audit'); }
    const orderKind = ['order_queued','order_expired','order_skipped','pending_at_end'].includes(event.kind);
    if (orderKind) {
      requireThat(event.entry_i === null && event.stop === null && event.target === null && event.previous_stop === null && event.ambiguity.length === 0, 'order fields');
      const candidate = setup(event.setup, report);
      requireThat(candidate.signal_i === event.signal_i, 'order signal alias');
      const known = signals.get(event.signal_i);
      requireThat(known && same(known.setup, candidate) && known.flat_permission && held === null, 'order source/flat binding');
      if (event.kind === 'order_queued' || (event.kind === 'order_skipped' && event.phase === 'signal_close')) {
        requireThat(pending === null && event.i === event.signal_i && event.price === null && !handledSignals.has(event.signal_i), 'new close order');
        handledSignals.add(event.signal_i);
        const invalidTarget = candidate.trigger !== null && candidate.target !== null && compare(audit(candidate.trigger, 'trigger'), audit(candidate.target, 'target')) >= 0;
        if (event.kind === 'order_queued') {
          requireThat(event.reason === 'completed_source_setup' && event.phase === 'close' && !invalidTarget, 'queue reason/target'); pending = candidate;
        } else requireThat(event.reason === 'no_positive_target_interval' && invalidTarget, 'skipped signal target');
      } else {
        requireThat(pending !== null && same(pending, candidate), 'pending order binding');
        if (event.kind === 'pending_at_end') {
          requireThat(event.i === report.to_source_index && event.signal_i === event.i && event.reason === 'next_session_not_in_selected_window' && event.phase === 'close' && event.price === null, 'terminal pending');
          pendingAtEnd = true;
        } else {
          requireThat(event.i === event.signal_i + 1, 'next-session order');
          if (event.kind === 'order_skipped') {
            requireThat(report.id === 'ptpm1_daily_cbs_cog' && event.reason === 'no_positive_target_interval' && event.phase === 'open' && event.price === row.open, 'skipped opening target');
            requireThat(compare(audit(event.exact_price, 'open'), audit(candidate.target, 'target')) >= 0, 'opening target interval');
          } else {
            requireThat(event.price === null && ((report.id === 'ptpm1_daily_cbs_cog' && event.reason === 'next_session_trigger_unreached' && event.phase === 'close') || (report.id === 'ptpm1_bullish_gap_surprise' && event.reason === 'next_open_gap_below_half' && event.phase === 'open')), 'expired order reason');
          }
          pending = null;
        }
      }
      continue;
    }
    requireThat(event.setup === null && integer(event.entry_i) && event.entry_i <= event.i && event.price !== null && event.exact_price !== null && event.stop !== null && event.target !== null, 'position event fields');
    positiveAudit(event.stop, 'position stop'); positiveAudit(event.target, 'position target');
    if (event.kind === 'entry') {
      requireThat(!held && !entry && pending && event.i === event.entry_i && event.signal_i + 1 === event.i && event.signal_i === pending.signal_i && event.previous_stop === null && event.ambiguity.length === 0, 'entry state');
      requireThat(equalAudit(event.stop, pending.stop), 'entry stop alias');
      if (report.id === 'ptpm1_daily_cbs_cog') {
        requireThat(event.reason === 'queued_buy_stop' && (event.phase === 'open' || event.phase === 'intraday_stop') && equalAudit(event.target, pending.target), 'CBS entry');
        if (event.phase === 'open') requireThat(event.price === row.open, 'CBS actual opening fill');
        else requireThat(equalAudit(event.exact_price, pending.trigger) && event.price >= row.low && event.price <= row.high, 'CBS buy-stop fill');
      } else requireThat(event.reason === 'half_gap_immediate_open' && event.phase === 'open' && event.price === row.open, 'BGS actual opening fill');
      const fillCell = roundingCell(event.price);
      const stop = audit(event.stop, 'stop'), target = audit(event.target, 'target');
      requireThat(stop.numerator * SCALE < fillCell[1] * stop.denominator && target.numerator * SCALE > fillCell[0] * target.denominator && compare(stop, target) < 0, 'initial positive bracket');
      held = {signal_i:event.signal_i,entry_i:event.i,entry_price:event.price,stop:event.stop,target:event.target};
      entry = event; pending = null;
    } else {
      requireThat(held && entry && event.signal_i === held.signal_i && event.entry_i === held.entry_i && equalAudit(event.target, held.target), 'held event binding');
      if (event.kind === 'stop_update') {
        requireThat(report.id === 'ptpm1_daily_cbs_cog' && event.reason === 'prior_completed_low_after_two_bars' && event.phase === 'before_open' && event.i - held.entry_i >= 2 && event.previous_stop !== null && equalAudit(event.previous_stop, held.stop) && compare(audit(event.stop, 'new stop'), audit(held.stop, 'old stop')) > 0 && equalAudit(event.exact_price, event.stop) && event.ambiguity.length === 0, 'causal trailing update');
        held = {signal_i:event.signal_i,entry_i:event.entry_i,entry_price:held.entry_price,stop:event.stop,target:event.target};
      } else {
        requireThat(event.previous_stop === null && equalAudit(event.stop, held.stop), 'held stop alias');
        if (event.kind === 'terminal_mark') {
          requireThat(event.i === report.to_source_index && event.reason === 'selected_last_close_not_filled_exit' && event.phase === 'close' && event.price === row.close && event.ambiguity.length === 0, 'terminal open mark'); terminal = true;
        } else {
          requireThat(event.kind === 'exit' && Object.hasOwn(exitPhases, event.reason), 'exit reason');
          if (event.reason.endsWith('_at_open')) requireThat(event.phase === 'open' && event.price === row.open && !(entry.i === event.i && entry.phase === 'intraday_stop') && event.ambiguity.length === 0, 'causal opening exit');
          else if (event.reason.endsWith('_intraday')) {
            requireThat(event.phase === 'intraday_assumed' && event.price >= row.low && event.price <= row.high && equalAudit(event.exact_price, event.reason === 'protective_intraday' ? event.stop : event.target), 'intraday level fill');
            requireThat(event.reason === 'protective_intraday' || event.ambiguity.length === 0, 'target ambiguity');
          } else requireThat(report.id === 'ptpm1_bullish_gap_surprise' && event.phase === 'close' && event.i - held.entry_i >= 1 && event.price === row.close && event.ambiguity.length === 0, 'BGS time-close');
          if (event.reason === 'cbs_full_gap_at_open') requireThat(report.id === 'ptpm1_daily_cbs_cog' && event.i - held.entry_i >= 2 && row.open > report.rows[event.i - report.from_source_index - 1].high, 'mature full gap');
        }
        derivedSpans.push({signal_i:held.signal_i,entry_i:held.entry_i,exit_i:event.i,entry_price:held.entry_price,exit_or_mark_price:event.price,open:event.kind === 'terminal_mark',entry_phase:entry.phase === 'open' ? 'open' : 'stop',exit_or_mark_phase:event.kind === 'terminal_mark' ? 'mark' : exitPhases[event.reason]});
        if (!terminal) { held = null; entry = null; }
      }
    }
  }
  flatAtClose.set(lastI, held === null);
  requireThat(!held || terminal, 'held position requires terminal mark');
  requireThat(!pending || pendingAtEnd, 'pending requires terminal evidence');
  requireThat(same(report.source.open_position, held) && same(report.source.pending_order, pending), 'terminal source state');
  const closeStates = [...flatAtClose];
  let closeIndex = 0, flat = true;
  for (const [i, signal] of signals) {
    // Merge selected close states once; no missing indicator prefix is invented.
    while (closeIndex < closeStates.length && closeStates[closeIndex][0] <= i) flat = closeStates[closeIndex++][1];
    requireThat(signal.flat_permission === flat && (!flat || handledSignals.has(i)), 'signal close permission/queue');
  }
  list(report.spans, 'spans');
  for (const span of report.spans) keys(span, SPAN_FIELDS, 'span');
  requireThat(same(report.spans, derivedSpans), 'source/span one-to-one binding');
}

const TRADE_FIELDS = ['trade_index','entry_source_index','exit_source_index','open','entry_allocation','gross_return','net_return','fee_return','slippage_return','gross_contribution','net_contribution','fee_contribution','slippage_contribution'];
const TRADE_EXTRA_FIELDS = ['gross_factor','net_factor','gross_curve_entry_allocation','paid_sides','estimated_exit_sides','calendar_days_held','observed_exposure_sessions','paid_fee_return','paid_fee_contribution','paid_slippage_return','paid_slippage_contribution','estimated_exit_fee_return','estimated_exit_fee_contribution','estimated_exit_slippage_return','estimated_exit_slippage_contribution'];
const DAY_FIELDS = ['source_index','date','equity','gross_equity','benchmark_equity','net_daily_return','gross_daily_return','benchmark_daily_return'];
const DAY_EXTRA_FIELDS = ['observed_exposure_session','paid_fee_contribution','paid_slippage_contribution','estimated_exit_fee_contribution','estimated_exit_slippage_contribution'];
const CLOSED_FIELDS = ['closed_count','closed_wins','closed_nonwins','closed_return_sum','closed_average_return','closed_win_rate','closed_compound_equity','closed_compound_return'];
function closedView(view: PristineClosedView, report: PristineReport, gross: boolean): void {
  keys(view, CLOSED_FIELDS, 'closed view');
  let count = 0, wins = 0, sum = 0, compound = 1;
  for (let i = 0; i < report.trade_returns.length; i++) {
    const trade = report.trade_returns[i], extra = report.trade_supplements[i];
    if (trade.open) continue;
    const value = gross ? trade.gross_return : trade.net_return;
    count++; wins += Number(value > 0); sum += value;
    compound *= gross ? extra.gross_factor : extra.net_factor;
  }
  requireThat(view.closed_count === count && view.closed_wins === wins && view.closed_nonwins === count - wins, 'closed classifications (zero is nonwin)');
  requireThat(equalNumber(view.closed_return_sum, sum) && equalNumber(view.closed_compound_equity, compound), 'closed aggregates');
  requireThat(equalNumber(view.closed_average_return, count ? sum / count : null) || (count === 0 && view.closed_average_return === null), 'closed average');
  requireThat(equalNumber(view.closed_win_rate, count ? wins / count : null) || (count === 0 && view.closed_win_rate === null), 'closed win rate');
  requireThat(unitPriceReturnConsistent(1, compound, view.closed_compound_return), 'closed compound return');
}
function account(report: PristineReport): void {
  const n = report.rows.length, from = report.from_source_index;
  list(report.days, 'days'); list(report.day_supplements, 'day supplements');
  list(report.trade_returns, 'trade returns'); list(report.trade_supplements, 'trade supplements');
  requireThat(report.days.length === n && report.day_supplements.length === n && report.trade_returns.length === report.spans.length && report.trade_supplements.length === report.spans.length, 'aligned account arrays');
  const exposure = new Array<boolean>(n).fill(false);
  const paidFee = new Array<number>(n).fill(0), paidSlip = new Array<number>(n).fill(0);
  const estimateFee = new Array<number>(n).fill(0), estimateSlip = new Array<number>(n).fill(0);
  let allocation = 1, grossAllocation = 1, next = 0;
  for (let i = 0; i < report.spans.length; i++) {
    const span = report.spans[i], trade = report.trade_returns[i], extra = report.trade_supplements[i];
    keys(trade, TRADE_FIELDS, 'trade return'); keys(extra, TRADE_EXTRA_FIELDS, 'trade supplement');
    requireThat(Object.values(trade).every(value => typeof value === 'boolean' || scalar(value)) && Object.values(extra).every(scalar), 'trade finite canonical scalars');
    requireThat(trade.trade_index === i && trade.entry_source_index === span.entry_i && trade.exit_source_index === span.exit_i && trade.open === span.open, 'trade/span identity');
    requireThat(span.entry_i >= from + next && span.exit_i >= span.entry_i && span.exit_i <= report.to_source_index && (!span.open || i === report.spans.length - 1), 'sequential spans');
    requireThat(equalNumber(trade.entry_allocation, allocation) && positive(allocation) && equalNumber(extra.gross_curve_entry_allocation, grossAllocation), 'net/gross entry allocations');
    requireThat(unitPriceReturnConsistent(span.entry_price, span.exit_or_mark_price, trade.gross_return) && unitReturnConsistent(trade.gross_return, trade.net_return, trade.fee_return, trade.slippage_return), 'trade return equations');
    requireThat(sideCost(report.costs.fees_bps, 2, trade.fee_return) && sideCost(report.costs.slippage_bps, 2, trade.slippage_return), 'two-side final return costs');
    requireThat(unitFactorConsistent(span.entry_price, span.exit_or_mark_price, extra.gross_factor) && factorReturn(extra.gross_factor, trade.gross_return) && factorReturn(extra.net_factor, trade.net_return) && priceFactorWithCosts(span.entry_price, span.exit_or_mark_price, extra.net_factor, report.costs.fees_bps, report.costs.slippage_bps, 2), 'independently rounded factors');
    for (const kind of ['gross','net','fee','slippage'] as const) requireThat(unitProductConsistent(allocation, trade[`${kind}_return`], trade[`${kind}_contribution`]), `${kind} contribution`);
    requireThat(extra.paid_sides === (span.open ? 1 : 2) && extra.estimated_exit_sides === Number(span.open), 'paid/estimated sides');
    requireThat(extra.calendar_days_held === calendarDays(report.rows[span.entry_i - from].date, report.rows[span.exit_i - from].date) && extra.observed_exposure_sessions === span.exit_i - span.entry_i + 1, 'calendar/observed holding duration');
    for (const stage of ['paid','estimated_exit'] as const) {
      const sides = stage === 'paid' ? extra.paid_sides : extra.estimated_exit_sides;
      for (const [kind, bps] of [['fee',report.costs.fees_bps],['slippage',report.costs.slippage_bps]] as const) {
        const fraction = extra[`${stage}_${kind}_return`], contribution = extra[`${stage}_${kind}_contribution`];
        requireThat(sideCost(bps, sides, fraction) && unitProductConsistent(allocation, fraction, contribution), `${stage} ${kind} audit`);
      }
    }
    // A single side is independently rounded; never halve a rounded two-side
    // charge (which can underflow at the smallest supported magnitudes).
    const oneFee = span.open ? extra.paid_fee_contribution : trade.fee_contribution / 2;
    const oneSlip = span.open ? extra.paid_slippage_contribution : trade.slippage_contribution / 2;
    // For closed trades, use the recorded per-day charge as the authoritative
    // one-side rounded product, checking its return cell rather than division.
    const entryDay = span.entry_i - from, exitDay = span.exit_i - from;
    let feeCharge = oneFee, slipCharge = oneSlip;
    if (!span.open) {
      const divisor = span.entry_i === span.exit_i ? 2 : 1;
      const day = report.day_supplements[entryDay];
      keys(day, DAY_EXTRA_FIELDS, 'entry day supplement');
      feeCharge = day.paid_fee_contribution / divisor;
      slipCharge = day.paid_slippage_contribution / divisor;
    }
    const chargeConsistent = (bps: number, charge: number) => {
      if (!scalar(charge) || charge < 0) return false;
      if (bps === 0) return charge === 0;
      if (charge === 0) return false;
      const c = roundingCell(bps), factors = productFactors(allocation,charge);
      return factors !== null && c[0] <= factors[1] * 10000n && c[1] >= factors[0] * 10000n;
    };
    requireThat(chargeConsistent(report.costs.fees_bps, feeCharge) && chargeConsistent(report.costs.slippage_bps, slipCharge), 'one-side paid contribution');
    paidFee[entryDay] += feeCharge; paidSlip[entryDay] += slipCharge;
    if (span.open) { estimateFee[exitDay] += feeCharge; estimateSlip[exitDay] += slipCharge; }
    else { paidFee[exitDay] += feeCharge; paidSlip[exitDay] += slipCharge; }
    for (let k = next; k < entryDay; k++) requireThat(equalNumber(report.days[k].equity, allocation) && equalNumber(report.days[k].gross_equity, grossAllocation), 'idle account curve');
    for (let k = entryDay; k <= exitDay; k++) {
      const day = report.days[k], last = k === exitDay;
      const mark = last ? span.exit_or_mark_price : report.rows[k].close;
      exposure[k] = true;
      if (last) {
        requireThat(equalNumber(day.equity, allocation * extra.net_factor) && equalNumber(day.gross_equity, grossAllocation * extra.gross_factor), 'final factor curve');
      } else {
        requireThat(markedCurve(span.entry_price, mark, grossAllocation, day.gross_equity, 0, 0, 0), 'held gross factor');
        // Inverse rounded-product cells are needed for net marks; independent
        // costs and factors must not be reconstructed as 1+rounded_return.
        requireThat(markedCurve(span.entry_price, mark, allocation, day.equity, report.costs.fees_bps, report.costs.slippage_bps, 1), 'held one-side net mark');
      }
    }
    allocation = report.days[exitDay].equity; grossAllocation = report.days[exitDay].gross_equity;
    next = exitDay + 1;
  }
  for (let k = next; k < n; k++) requireThat(equalNumber(report.days[k].equity, allocation) && equalNumber(report.days[k].gross_equity, grossAllocation), 'trailing idle account curve');
  let previous = 1, previousGross = 1, previousBenchmark = 1;
  for (let k = 0; k < n; k++) {
    const day = report.days[k], extra = report.day_supplements[k], row = report.rows[k];
    keys(day, DAY_FIELDS, 'day'); keys(extra, DAY_EXTRA_FIELDS, 'day supplement');
    requireThat(day.source_index === row.source_index && day.date === row.date && [day.equity,day.gross_equity,day.benchmark_equity].every(positive), 'daily identity/equity');
    requireThat(unitPriceReturnConsistent(previous, day.equity, day.net_daily_return) && unitPriceReturnConsistent(previousGross, day.gross_equity, day.gross_daily_return) && unitPriceReturnConsistent(previousBenchmark, day.benchmark_equity, day.benchmark_daily_return), 'daily returns from initial1');
    requireThat(unitFactorConsistent(report.rows[0].close, row.close, day.benchmark_equity), 'selected stock buy/hold');
    requireThat(extra.observed_exposure_session === exposure[k] && equalNumber(extra.paid_fee_contribution, paidFee[k]) && equalNumber(extra.paid_slippage_contribution, paidSlip[k]) && equalNumber(extra.estimated_exit_fee_contribution, estimateFee[k]) && equalNumber(extra.estimated_exit_slippage_contribution, estimateSlip[k]), 'daily paid/estimated/exposure audits');
    previous = day.equity; previousGross = day.gross_equity; previousBenchmark = day.benchmark_equity;
  }
  const closed = keys(report.closed_stats, ['net','gross','net_max_drawdown','gross_max_drawdown','observed_exposure_sessions','selected_sessions','exposure'], 'closed stats');
  const count = exposure.filter(Boolean).length;
  requireThat(closed.selected_sessions === n && closed.observed_exposure_sessions === count && equalNumber(closed.exposure, count / n), 'observed session exposure');
  closedView(report.closed_stats.net, report, false); closedView(report.closed_stats.gross, report, true);
  for (const [key, curve] of [['net_max_drawdown','equity'],['gross_max_drawdown','gross_equity']] as const) {
    validatePristineDrawdown(report.closed_stats[key], report.days.map(day => day[curve]));
  }
}
/** Private production validator; a copied-module test alias may exercise it. */
function validatePristineDrawdown(worst: number, curve: readonly number[]): void {
  requireThat(scalar(worst) && worst <= 0 && worst >= -1, 'drawdown domain');
  let peak = 1, seenWorst = worst === 0;
  for (const equity of curve) {
    peak = Math.max(peak, equity);
    if (unitPriceReturnConsistent(peak, equity, worst)) seenWorst = true;
    // Occurrence alone does not establish the maximum: a later observation
    // must not be worse. Compare exact rounding-cell bounds, with no epsilon
    // and no JavaScript reconstruction of the pinned decimal formatter.
    const p = roundingCell(peak), value = roundingCell(equity), w = roundingCell(worst);
    requireThat((value[1] - p[0]) * SCALE >= w[0] * p[0], 'observed drawdown exceeds reported worst');
    if (equity < peak && worst === 0) fail('nonflat drawdown cannot be zero');
  }
  requireThat(seenWorst, 'drawdown belongs to observed curve');
}

function statisticDisplay(value: number): number {
  const scaled = value * 1e6;
  if (!Number.isFinite(scaled)) return value;
  const magnitude = Math.abs(scaled);
  const integral = magnitude >= 2 ** 52 ? magnitude : Math.floor(magnitude) + Number(magnitude - Math.floor(magnitude) >= .5);
  return (scaled < 0 || Object.is(scaled, -0) ? -integral : integral) / 1e6;
}
const NULL_REASONS = new Set(['arithmetic_nonfinite','arithmetic_underflow','zero_elapsed_years','fewer_two_daily_observations','zero_drawdown','zero_daily_standard_deviation','zero_downside_deviation','no_winning_closed_trades','no_nonwinning_closed_trades','no_closed_trades','no_open_trade','no_winners_or_negative_average_nonwin','zero_closed_loss_magnitude','fewer_two_closed_trades','zero_closed_standard_deviation','no_positive_average_calendar_hold','no_negative_average_nonwin']);
function statMap(map: PristineStatMap<string>, expected: readonly string[], where: string): void {
  keys(map, ['values','display_values','null_reasons'], where);
  keys(map.values, expected, `${where} raw`); keys(map.display_values, expected, `${where} display`);
  const reasons = object(map.null_reasons, `${where} null reasons`);
  requireThat(Object.keys(reasons).every(key => expected.includes(key)), `${where} reason keys`);
  for (const key of expected) {
    const raw = map.values[key], display = map.display_values[key];
    if (raw === null) requireThat(display === null && text(reasons[key]) && NULL_REASONS.has(reasons[key] as string), `${where}.${key} explicit null`);
    else requireThat(finite(raw) && equalNumber(display, statisticDisplay(raw)) && !Object.hasOwn(reasons, key), `${where}.${key} display alias`);
  }
}
function statistics(report: PristineReport): void {
  keys(report.statistics, ['net','gross','financial','benchmark'], 'statistics');
  const {net, gross, financial, benchmark} = report.statistics;
  statMap(net, PRISTINE_COMMON_STATS, 'net statistics'); statMap(gross, PRISTINE_COMMON_STATS, 'gross statistics');
  statMap(financial, PRISTINE_FINANCIAL_STATS, 'financial statistics'); statMap(benchmark, PRISTINE_BENCHMARK_STATS, 'benchmark statistics');
  const alias = (a: PristineStatMap<string>, ak: string, b: PristineStatMap<string>, bk: string) => {
    requireThat((a.values[ak] === null && b.values[bk] === null) || equalNumber(a.values[ak], b.values[bk]), `statistic alias ${ak}`);
    requireThat(a.null_reasons[ak] === b.null_reasons[bk], `statistic null alias ${ak}`);
  };
  for (const key of PRISTINE_COMMON_STATS) alias(financial, key, net, key);
  for (const [target, source] of [['gross_total_return','total_return'],['gross_avg_trade','avg_trade'],['gross_win_rate','win_rate'],['gross_sharpe','sharpe'],['gross_max_dd','max_dd']] as const) alias(financial, target, gross, source);
  alias(financial, 'avg_net_pnl', net, 'avg_trade');
  for (const map of [net, gross]) for (const [target, source] of [['bh_total','total_return'],['bh_cagr','cagr'],['bh_max_dd','max_dd']] as const) alias(map, target, benchmark, source);
  const terminal = report.days.at(-1)!;
  for (const [map, view, final, dd, isGross] of [[net,report.closed_stats.net,terminal.equity,report.closed_stats.net_max_drawdown,false],[gross,report.closed_stats.gross,terminal.gross_equity,report.closed_stats.gross_max_drawdown,true]] as const) {
    const values = map.values;
    requireThat(values.trades === view.closed_count && (view.closed_count === 0 ? values.win_rate === null && values.avg_trade === null : equalNumber(values.win_rate, view.closed_win_rate) && equalNumber(values.avg_trade, view.closed_average_return)), 'closed stats aliases');
    requireThat(equalNumber(values.closed_return_sum, view.closed_return_sum) && equalNumber(values.net_profit, view.closed_return_sum) && equalNumber(values.max_dd, dd) && equalNumber(values.exposure, report.closed_stats.exposure), 'source report statistics aliases');
    requireThat(typeof values.total_return === 'number' && unitPriceReturnConsistent(1, final, values.total_return), 'total return/curve');
    requireThat(typeof values.compounded_return === 'number' && unitPriceReturnConsistent(1, final, values.compounded_return), 'curve compounded statistics');
    requireThat(equalNumber(values.years, calendarDays(report.rows[0].date, report.rows.at(-1)!.date) / 365.25), 'elapsed years');
    const open = report.trade_returns.find(trade => trade.open);
    requireThat(open ? equalNumber(values.open_trade, isGross ? open.gross_return : open.net_return) : values.open_trade === null, 'open return alias');
  }
  requireThat(typeof benchmark.values.total_return === 'number' && unitPriceReturnConsistent(1, terminal.benchmark_equity, benchmark.values.total_return), 'benchmark total');
  for (const [key, source] of [['ledger_gross_return','gross_contribution'],['fee_return','fee_contribution'],['slippage_return','slippage_contribution']] as const) {
    let total = 0;
    for (const trade of report.trade_returns) total += trade[source];
    requireThat(equalNumber(financial.values[key], total), `${key} ledger sum`);
  }
}

const REPORT_FIELDS = ['model','convention','kind','owner','book','id','research_mode','context','market','currency','asset_type','timeframe','qualified_ticker','symbol','from_source_index','to_source_index','source_row_count','source_prefix_row_count','volume_column_present','basis','provenance_json','claim','book_performance_reproduced','native_platform_parity','native_historical_dollars_verified','historical_membership_verified','benchmark','costs','rows','source','spans','days','trade_returns','day_supplements','trade_supplements','closed_stats','statistics'];
/** Fully audits the response against current controls; it grants no cache brand. */
export function parsePristineReport(data: unknown, admission: PristineAdmission): PristineView {
  const context = admissionContext(admission);
  const envelope = keys(data, ['pristine_report'], 'envelope');
  const obj = keys(envelope.pristine_report, REPORT_FIELDS, 'report');
  const report = obj as unknown as PristineReport;
  const literals: JsonObject = {
    model:PRISTINE_MODEL, convention:PRISTINE_CONVENTION, kind:'single_symbol_supplied_series_research',
    owner:'pristine', book:PRISTINE_BOOK, id:admission.id, research_mode:PRISTINE_RESEARCH_MODE,
    context:'broad_us_supplied_series',market:'us',currency:'USD',asset_type:'individual_stock',timeframe:'D',
    qualified_ticker:admission.ticker,symbol:context.metadata.symbol,basis:DATA_BASIS,claim:null,
    book_performance_reproduced:false,native_platform_parity:false,native_historical_dollars_verified:false,historical_membership_verified:false,
  };
  for (const [key, value] of Object.entries(literals)) requireThat(obj[key] === value, `report ${key}`);
  requireThat(typeof report.volume_column_present === 'boolean' && integer(report.from_source_index) && integer(report.to_source_index) && report.from_source_index <= report.to_source_index && integer(report.source_row_count) && report.source_row_count === object(context.metadata.provenance, 'provenance').bars && report.to_source_index < report.source_row_count && report.source_prefix_row_count === report.to_source_index + 1, 'source coverage coordinates');
  requireThat(text(report.provenance_json), 'original provenance string');
  let provenance: unknown;
  try { provenance = JSON.parse(report.provenance_json); } catch { fail('original provenance JSON'); }
  requireThat(same(provenance, context.metadata.provenance), 'unchanged original provenance');
  keys(report.benchmark, ['kind','qualified_ticker','symbol'], 'benchmark identity');
  requireThat(report.benchmark.kind === 'selected_symbol_buy_hold' && report.benchmark.qualified_ticker === admission.ticker && report.benchmark.symbol === report.symbol, 'actual selected stock benchmark');
  keys(report.costs, ['fees_bps','slippage_bps','rollover_bps','total_side_bps','basis'], 'report costs');
  for (const key of ['fees_bps','slippage_bps','rollover_bps'] as const) requireThat(equalNumber(report.costs[key], admission.costs[key]), `current ${key}`);
  requireThat(equalNumber(report.costs.total_side_bps, report.costs.fees_bps + report.costs.slippage_bps) && report.costs.basis === COST_BASIS, 'cost basis/combined bps');
  list(report.rows, 'selected rows');
  requireThat(report.rows.length > 0 && report.rows.length === report.to_source_index - report.from_source_index + 1, 'selected row count');
  for (let i = 0; i < report.rows.length; i++) {
    const row = report.rows[i]; keys(row, ['source_index','date','open','high','low','close','volume'], 'source row');
    requireThat(row.source_index === report.from_source_index + i && date(row.date) && (i === 0 || report.rows[i - 1].date < row.date) && row.date >= (context.metadata.first as string) && row.date <= (context.metadata.last as string), 'source row index/date');
    requireThat([row.open,row.high,row.low,row.close].every(positive) && row.low <= Math.min(row.open,row.close) && row.high >= Math.max(row.open,row.close), 'source OHLC');
    volume(row.volume);
    requireThat(!admission.window?.from || row.date >= admission.window.from, 'selected from');
    requireThat(!admission.window?.to || row.date <= admission.window.to, 'selected to');
  }
  if (report.from_source_index === 0) requireThat(report.rows[0].date === context.metadata.first, 'first source date');
  if (report.to_source_index === report.source_row_count - 1) requireThat(report.rows.at(-1)!.date === context.metadata.last, 'last source date');
  // Validate the entire JSON tree before traversing/protecting it. This excludes
  // accessors, cycles, sparse arrays, nonfinite hidden extras and prototype tricks.
  signature(data);
  sourceAndSpans(report);
  account(report);
  statistics(report);
  const stableContext = {...admission};
  if (stableContext.window === undefined) delete stableContext.window;
  return {report, context:protectedClone(stableContext)};
}

const validViews = new WeakSet<object>();
const reportCache = new WeakMap<object, {readonly view: PristineView; readonly context: string}>();
/** A cached response is usable only with the same complete, current context. */
export function validatedPristineReport(data: unknown, admission: PristineAdmission): PristineView {
  const binding = admissionContext(admission);
  const envelope = keys(data, ['pristine_report'], 'cached response');
  const cached = reportCache.get(envelope);
  if (cached) {
    requireThat(binding.signature === cached.context && envelope.pristine_report === cached.view.report && isValidatedPristinePair(data, cached.view), 'cached response/context binding');
    return cached.view;
  }
  const parsed = parsePristineReport(data, admission);
  protect(parsed.report);
  Object.defineProperty(envelope, 'pristine_report', {value:parsed.report,enumerable:true,writable:false,configurable:false});
  const view = Object.freeze(parsed);
  reportCache.set(envelope, {view,context:binding.signature}); validViews.add(view);
  return view;
}
export function isValidatedPristineView(value: unknown): value is PristineView {
  return value !== null && typeof value === 'object' && validViews.has(value);
}
export function isValidatedPristinePair(data: unknown, value: unknown): value is PristineView {
  return data !== null && typeof data === 'object' && isValidatedPristineView(value) && reportCache.get(data)?.view === value && (data as JsonObject).pristine_report === value.report;
}
