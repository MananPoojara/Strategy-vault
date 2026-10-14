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
}
interface ResponseLike {
  ok: boolean;
  status: number;
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
    try {return errorMessage(JSON.parse(body)) || clean(body);}
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
  const blocked = (issue: OwnerIssue, retryableFailure = false): OwnerRowsResult => ({
    rows: ids.map(id => kind === 'summary'
      ? {id, blocked: issue.note, readiness: issue.status}
      : {id, status: issue.status, note: issue.note}),
    issue, retryableFailure,
  });
  if (!ids.length) return {rows: [], retryableFailure: false};
  const preflight = dateCoverageIssue(context);
  if (preflight) return blocked({owner, ...preflight});
  const attempts = Number.isFinite(options.retries)
    ? Math.min(3, Math.max(1, Math.trunc(options.retries!))) : 3;
  const wait = options.wait || (milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds)));
  for (let attempt = 1; attempt <= attempts; attempt++) {
    let response: ResponseLike;
    try {response = await fetcher(url, {signal: AbortSignal.timeout(20_000)});}
    catch (error) {
      if (attempt < attempts) {await wait(300); continue;}
      return blocked(requestIssue(owner, context, error instanceof Error ? clean(error.message) : ''), true);
    }
    if (!response.ok) {
      const detail = await responseMessage(response);
      if (response.status === 400 && /empty\s+(?:date\s+)?range/i.test(detail)) return blocked({owner, ...emptyRange(context)});
      if (response.status === 400 && /invalid[^.]*date|date[^.]*invalid|from[^.]*before[^.]*to|reversed|valid YYYY-MM-DD/i.test(detail)) {
        return blocked({owner, status: 'needs_data', kind: 'invalid_range', note:
          `${detail || 'The selected date range is invalid.'} No date is repaired.${coverageText(context)}`});
      }
      const transient = response.status === 429 || response.status >= 500 && response.status <= 599;
      if (transient && attempt < attempts) {await wait(300); continue;}
      return blocked(requestIssue(owner, context, detail || `HTTP ${response.status}.`), transient);
    }
    try {
      const rows = rowsFrom(await response.json(), kind, owned);
      if (!rows) return blocked(requestIssue(owner, context, 'The response did not contain valid owned strategy records.'));
      return {rows, retryableFailure: false};
    } catch (error) {
      const transient = error instanceof TypeError || error instanceof Error && /AbortError|TimeoutError/.test(error.name);
      if (transient && attempt < attempts) {await wait(300); continue;}
      return blocked(requestIssue(owner, context, 'The response could not be read as valid strategy JSON.'), transient);
    }
  }
  return blocked(requestIssue(owner, context, 'The request could not be completed.'), true);
}
