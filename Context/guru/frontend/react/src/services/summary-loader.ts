export interface DateCoverageContext {
  ticker: string;
  label?: string;
  from?: string;
  to?: string;
  first?: string;
  last?: string;
}
export interface OwnerIssue {
  owner: string;
  status: 'needs_data' | 'unavailable';
  kind: 'empty_range' | 'invalid_range' | 'request_failed';
  note: string;
}
export type CoverageIssue = Omit<OwnerIssue, 'owner'>;
export type OwnerRow = Record<string, unknown> & {id: string};
export interface OwnerRowsResult {
  rows: OwnerRow[];
  issue?: OwnerIssue;
  retryableFailure: boolean;
  /** The page left before an answer came. Not a result: never cache or show it. */
  aborted?: boolean;
  /** Do not ask the engine for this result again before this much time has passed. */
  retryAfterMs?: number;
}
interface ResponseLike {
  ok: boolean;
  status: number;
  headers?: {get(name: string): string | null};
  text(): Promise<string>;
  json(): Promise<unknown>;
}
export interface OwnerRowsOptions {
  owner: string;
  definitions: readonly {id: string}[];
  context: DateCoverageContext;
  url: string;
  kind: 'summary' | 'readiness';
  fetcher: (url: string, init?: RequestInit) => Promise<ResponseLike>;
  wait?: (milliseconds: number) => Promise<unknown>;
  retries?: number;
  /** Aborted when the user leaves the page. */
  signal?: AbortSignal;
}
/** The gateway gives up after 120 s and answers 504. A shorter browser limit abandons work the engine keeps doing. */
const ANSWER_BUDGET_MS = 125_000;
/** After a timeout the engine may still be calculating: leave it alone this long before asking again. */
const COOL_DOWN_MS = 30_000;
const DEFAULT_BUSY_WAIT_MS = 5_000, LONGEST_BUSY_WAIT_MS = 60_000;

/** Retry-After as seconds or an HTTP date; undefined when absent or unreadable. */
export function retryAfterMs(header: string | null | undefined, now = Date.now()): number | undefined {
  const value = header?.trim();
  if (!value) return undefined;
  if (/^\d+$/.test(value)) return Number(value) * 1000;
  const date = Date.parse(value);
  return Number.isFinite(date) ? Math.max(0, date - now) : undefined;
}
export interface FailureRecord {generation: number; notBefore: number}
/** Failures are shared within a render; a later navigation may ask again once any cool-down has passed. */
export function failureRecord(outcomes: readonly OwnerRowsResult[], generation: number, now = Date.now()): FailureRecord | undefined {
  const failed = outcomes.filter(outcome => outcome.issue);
  return failed.length ? {generation, notBefore: now + Math.max(0, ...failed.map(outcome => outcome.retryAfterMs || 0))} : undefined;
}
export function retryDue(record: FailureRecord | undefined, generation: number, now = Date.now()): boolean {
  return !!record && record.generation !== generation && now >= record.notBefore;
}

function clean(value: unknown): string {
  return typeof value === 'string'
    ? value.replace(/[\u0000-\u001f\u007f]/g, ' ').replace(/\s+/g, ' ').trim().slice(0, 600)
    : '';
}
function validDate(value?: string): boolean {
  if (!value || !/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
  const parsed = new Date(value + 'T00:00:00Z');
  return Number.isFinite(parsed.getTime()) && parsed.toISOString().slice(0, 10) === value;
}
function coverageText(context: DateCoverageContext): string {
  const first = validDate(context.first) ? context.first : undefined;
  const last = validDate(context.last) ? context.last : undefined;
  if (first && last && first > last) return '';
  if (first && last) return ` Available history: ${first} to ${last}.`;
  if (first) return ` Available history begins ${first}.`;
  if (last) return ` Available history ends ${last}.`;
  return '';
}
function emptyRange(context: DateCoverageContext, barsAbsent = false): CoverageIssue {
  const period = context.from || context.to
    ? ` (${context.from || 'first available bar'} to ${context.to || 'latest available bar'})`
    : '';
  const label = clean(context.label) || clean(context.ticker);
  const reason = barsAbsent
    ? `No price bars are available for ${label} in the selected period${period}.`
    : `For some backtests, the selected period has fewer than two usable price bars for ${label}${period}.`;
  return {status: 'needs_data', kind: 'empty_range', note:
    `${reason}${coverageText(context)} Choose another period.`};
}

/** Reject provably invalid/disjoint requests, without guessing missing coverage
 * or silently trimming a requested interval. Overlapping intervals still go to
 * the backend, which retains its native-history, warmup and execution guards.
 */
export function dateCoverageIssue(context: DateCoverageContext): CoverageIssue | undefined {
  for (const key of ['from', 'to'] as const) {
    if (context[key] && !validDate(context[key])) return {
      status: 'needs_data', kind: 'invalid_range',
      note: `The selected ${key === 'from' ? 'From' : 'To'} date for ${clean(context.label) || clean(context.ticker)} is invalid. Use a valid YYYY-MM-DD calendar date. No date is repaired.${coverageText(context)}`,
    };
  }
  if (context.from && context.to && context.from >= context.to) return {
    status: 'needs_data', kind: 'invalid_range',
    note: `From must be before To for ${clean(context.label) || clean(context.ticker)}. At least two observed price bars are required for a backtest. Choose another period.${coverageText(context)}`,
  };
  const first = validDate(context.first) ? context.first : undefined;
  const last = validDate(context.last) ? context.last : undefined;
  if (first && last && first > last) return undefined;
  if ((first && context.to && context.to < first) || (last && context.from && context.from > last)) return emptyRange(context, true);
  return undefined;
}

function errorMessage(value: unknown): string {
  if (typeof value === 'string') return clean(value);
  if (!value || typeof value !== 'object') return '';
  const object = value as Record<string, unknown>;
  return errorMessage(object.message) || errorMessage(object.error) || errorMessage(object.detail) || errorMessage(object.field);
}
async function responseMessage(response: ResponseLike): Promise<string> {
  try {
    const body = await response.text();
    try {return errorMessage(JSON.parse(body));}
    catch {return clean(body);}
  } catch {return '';}
}
function requestIssue(owner: string, context: DateCoverageContext, detail: string): OwnerIssue {
  return {owner, status: 'unavailable', kind: 'request_failed', note:
    `Some backtests could not be loaded for ${clean(context.label) || clean(context.ticker)} in this period. ${detail || 'The request could not be completed.'} No performance results are inferred.${coverageText(context)}`};
}
function rowsFrom(body: unknown, kind: OwnerRowsOptions['kind'], owned: ReadonlySet<string>): OwnerRow[] | undefined {
  const rows = kind === 'readiness' && body && typeof body === 'object'
    ? (body as Record<string, unknown>).strategies
    : body;
  if (!Array.isArray(rows)) return undefined;
  const seen = new Set<string>();
  for (const row of rows) {
    if (!row || typeof row !== 'object' || Array.isArray(row) || !Object.hasOwn(row, 'id') ||
        typeof row.id !== 'string' || !owned.has(row.id) || seen.has(row.id)) return undefined;
    seen.add(row.id);
  }
  return rows;
}

/** Load only one owner; failures produce owned blocked records, never zero
 * returns or ready flags. The caller combines independent owner outcomes and
 * decides caching; final transient failures are explicitly marked retryable.
 */
export async function loadOwnerRows(options: OwnerRowsOptions): Promise<OwnerRowsResult> {
  const {owner, kind, fetcher} = options;
  const context = {...options.context}, url = options.url;
  const ids = [...new Set(options.definitions.map(def => def.id))];
  const owned = new Set(ids);
  const blocked = (issue: OwnerIssue, retryableFailure = false, retryAfter?: number): OwnerRowsResult => ({
    rows: ids.map(id => kind === 'summary'
      ? {id, blocked: issue.note, readiness: issue.status}
      : {id, status: issue.status, note: issue.note}),
    issue, retryableFailure, ...(retryAfter === undefined ? {} : {retryAfterMs: retryAfter}),
  });
  const aborted: OwnerRowsResult = {rows: [], retryableFailure: true, aborted: true};
  if (!ids.length) return {rows: [], retryableFailure: false};
  const preflight = dateCoverageIssue(context);
  if (preflight) return blocked({owner, ...preflight});
  const attempts = Number.isFinite(options.retries)
    ? Math.min(3, Math.max(1, Math.trunc(options.retries!))) : 3;
  const {signal} = options;
  const wait = options.wait || (milliseconds => new Promise(resolve => {
    const timer = setTimeout(resolve, milliseconds);
    signal?.addEventListener('abort', () => {clearTimeout(timer); resolve(undefined);}, {once: true});
  }));
  const budget = AbortSignal.timeout(ANSWER_BUDGET_MS);
  for (let attempt = 1; attempt <= attempts; attempt++) {
    let response: ResponseLike;
    try {response = await fetcher(url, {signal: signal ? AbortSignal.any([signal, budget]) : budget});}
    catch (error) {
      if (signal?.aborted) return aborted;
      // No retry: the engine may still be calculating this, and a second call would run beside it.
      const late = error instanceof Error && error.name === 'TimeoutError';
      return blocked(requestIssue(owner, context, late ? 'The calculation engine did not answer in time.' : error instanceof Error ? clean(error.message) : ''), true, COOL_DOWN_MS);
    }
    if (!response.ok) {
      const detail = await responseMessage(response);
      if (signal?.aborted) return aborted;
      if (response.status === 400 && /empty\s+(?:date\s+)?range/i.test(detail)) return blocked({owner, ...emptyRange(context)});
      if (response.status === 400 && /invalid[^.]*date|date[^.]*invalid|from[^.]*before[^.]*to|reversed|valid YYYY-MM-DD/i.test(detail)) {
        return blocked({owner, status: 'needs_data', kind: 'invalid_range', note:
          `${detail || 'The selected date range is invalid.'} No date is repaired.${coverageText(context)}`});
      }
      // Only "busy" is safe to repeat, and only after the wait the gateway asks for. 500/502/504 may have work behind them.
      const busy = response.status === 429 || response.status === 503;
      const asked = retryAfterMs(response.headers?.get?.('Retry-After'));
      const delay = asked ?? DEFAULT_BUSY_WAIT_MS;
      if (busy && attempt < attempts && delay <= LONGEST_BUSY_WAIT_MS) {
        await wait(delay);
        if (signal?.aborted) return aborted;
        continue;
      }
      const transient = busy || response.status >= 500 && response.status <= 599;
      return blocked(requestIssue(owner, context, detail || `HTTP ${response.status}.`), transient, transient ? asked ?? COOL_DOWN_MS : undefined);
    }
    try {
      const rows = rowsFrom(await response.json(), kind, owned);
      if (!rows) return blocked(requestIssue(owner, context, 'The response did not contain valid owned strategy records.'));
      return {rows, retryableFailure: false};
    } catch (error) {
      if (signal?.aborted) return aborted;
      const transient = error instanceof TypeError || error instanceof Error && /AbortError|TimeoutError/.test(error.name);
      return blocked(requestIssue(owner, context, 'The response could not be read as valid strategy JSON.'), transient, transient ? COOL_DOWN_MS : undefined);
    }
  }
  return blocked(requestIssue(owner, context, 'The request could not be completed.'), true);
}
