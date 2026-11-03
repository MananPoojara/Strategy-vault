import {expect, it} from 'vitest';
import {dateCoverageIssue, failureRecord, loadOwnerRows, retryAfterMs, retryDue, type OwnerRowsOptions} from '../src/services/summary-loader';

const base = {owner: 'connors', definitions: [{id: 'one'}], context: {ticker: 'AAPL', from: '2024-01-01', to: '2025-12-31', first: '1980-12-12', last: '2020-09-11'}, url: '/api/us/summary', kind: 'summary' as const};
it('blocks an uncovered period without a network request or zero-valued statistics', async () => {
  let calls = 0;
  const result = await loadOwnerRows({...base, fetcher: async () => {calls++; throw new Error('should not fetch');}});
  expect(calls).toBe(0);
  expect(result.issue?.kind).toBe('empty_range');
  expect(result.rows[0]).not.toHaveProperty('stats');
  expect(result.issue?.note).toContain('1980-12-12 to 2020-09-11');
});
it('rejects malformed dates and preserves unknown coverage without guessing a limit', () => {
  expect(dateCoverageIssue({...base.context, from: '2023-02-29'})?.kind).toBe('invalid_range');
  expect(dateCoverageIssue({...base.context, from: '2020-01-02', to: '2020-01-01'})?.kind).toBe('invalid_range');
  expect(dateCoverageIssue({ticker: 'NEW', from: '2030-01-01'})).toBeUndefined();
});
it('does not retry a deterministic date rejection returned in JSON', async () => {
  let calls = 0;
  const result = await loadOwnerRows({...base, context: {ticker: 'AAPL'}, fetcher: async () => {
    calls++;
    return {ok: false, status: 400, text: async () => JSON.stringify({error: {message: 'empty date range'}}), json: async () => null};
  }});
  expect(calls).toBe(1);
  expect(result.retryableFailure).toBe(false);
  expect(result.rows[0]).not.toHaveProperty('stats');
});
it('recovers from transient failure with actual rows and a bounded signal', async () => {
  let calls = 0;
  const actual = {id: 'one', stats: {trades: 2, net_profit: -0.01}};
  const result = await loadOwnerRows({...base, context: {ticker: 'AAPL'}, wait: async () => {}, fetcher: async (_url, init) => {
    expect(init?.signal).toBeInstanceOf(AbortSignal);
    calls++;
    return calls === 1
      ? {ok: false, status: 503, text: async () => 'busy', json: async () => null}
      : {ok: true, status: 200, text: async () => '', json: async () => [actual]};
  }});
  expect(result.rows).toEqual([actual]);
  expect(result.issue).toBeUndefined();
  expect(calls).toBe(2);
});

const reply = (status: number, body: unknown) => ({ok: status >= 200 && status < 300, status,
  text: async () => typeof body === 'string' ? body : JSON.stringify(body), json: async () => body});
const request = (fetcher: OwnerRowsOptions['fetcher'], patch: Partial<OwnerRowsOptions> = {}) =>
  loadOwnerRows({...base, context: {ticker: 'AAPL'}, fetcher, wait: async () => {}, ...patch});

it('checks true calendar dates, partial and unknown coverage without guessing or clipping', () => {
  const c = {ticker: 'AAPL', first: '1980-12-12', last: '2020-09-11'};
  for (const from of ['2023-02-29', '2020-13-01', '01-01-2020'])
    expect(dateCoverageIssue({...c, from})?.kind).toBe('invalid_range');
  expect(dateCoverageIssue({...c, to: '2020-02-30'})?.kind).toBe('invalid_range');
  expect(dateCoverageIssue({...c, from: '2020-02-29'})).toBeUndefined();
  expect(dateCoverageIssue({...c, from: '2020-01-01', to: '2020-01-01'})?.kind).toBe('invalid_range');
  expect(dateCoverageIssue({...c, to: '1970-01-01'})?.kind).toBe('empty_range');
  expect(dateCoverageIssue({...c, from: '1970-01-01', to: '1990-01-01'})).toBeUndefined();
  expect(dateCoverageIssue({...c, from: '2010-01-01', to: '2030-01-01'})).toBeUndefined();
  expect(dateCoverageIssue({...c, first: '2026-01-01', last: '2000-01-01'})).toBeUndefined();
  expect(dateCoverageIssue({ticker: 'NEW', first: 'unknown', last: 'unknown', from: '2030-01-01'})).toBeUndefined();
  expect(dateCoverageIssue({ticker: 'NEW', first: '2020-01-01', to: '2010-01-01'})?.note).toContain('begins 2020-01-01');
  expect(dateCoverageIssue({ticker: 'NEW', last: '2020-01-01', from: '2030-01-01'})?.note).toContain('ends 2020-01-01');
});

it('keeps an independent healthy owner despite another owner failure, and validates readiness responses', async () => {
  const actual = {id: 'one', stats: {trades: 3, net_profit: 0.02}};
  const [good, bad] = await Promise.all([
    request(async () => reply(200, [actual])),
    request(async () => reply(400, 'empty date range'), {owner: 'other', definitions: [{id: 'two'}]}),
  ]);
  expect(good.rows).toEqual([actual]);
  expect(good.issue).toBeUndefined();
  expect(bad.rows).toEqual([{id: 'two', blocked: bad.issue?.note, readiness: 'needs_data'}]);
  const readiness = await request(async () => reply(200, {strategies: [{id: 'one', status: 'ready'}]}), {kind: 'readiness'});
  expect(readiness.rows).toEqual([{id: 'one', status: 'ready'}]);
  const failed = await request(async () => reply(403, {message: 'access denied'}), {kind: 'readiness'});
  expect(failed.rows).toEqual([{id: 'one', status: 'unavailable', note: failed.issue?.note}]);
});

it('handles bounded transient errors, captured URLs, JSON error fields and access failures', async () => {
  const urls: string[] = [], delays: number[] = [];
  const exhausted = await request(async url => {urls.push(url); return reply(503, {error: {field: 'server', message: 'busy'}});},
    {retries: 999, wait: async ms => {delays.push(ms);}});
  expect(urls).toEqual([base.url, base.url, base.url]);
  expect(delays).toEqual([5000, 5000]);
  expect(exhausted.retryableFailure).toBe(true);
  expect(exhausted.rows[0]).not.toHaveProperty('stats');
  expect((await request(async () => {throw new TypeError('network');}, {retries: 1})).retryableFailure).toBe(true);
  const invalid = await request(async () => reply(400, {detail: 'Native requested from must be a valid YYYY-MM-DD date'}));
  expect(invalid.issue?.kind).toBe('invalid_range');
  expect(invalid.retryableFailure).toBe(false);
  const generic = await request(async () => reply(404, {}));
  expect(generic.issue?.note).toContain('HTTP 404');
  expect(generic.retryableFailure).toBe(false);
});

it('rejects malformed/foreign/duplicate response records without certifying them or retrying', async () => {
  for (const payload of [{strategies: []}, [null], [['one']], [{id: 'foreign'}], [{id: 'one'}, {id: 'one'}], [{}], [0]]) {
    let calls = 0;
    const result = await request(async () => {calls++; return reply(200, payload);});
    expect(calls).toBe(1);
    expect(result.issue?.kind).toBe('request_failed');
    expect(result.retryableFailure).toBe(false);
    expect(result.rows[0]).not.toHaveProperty('stats');
  }
  const invalid = await request(async () => ({...reply(200, null), json: async () => {throw new SyntaxError('bad JSON');}}));
  expect(invalid.issue?.status).toBe('unavailable');
  expect(invalid.retryableFailure).toBe(false);
  const brokenBody = await request(async () => ({...reply(200, null), json: async () => {throw new TypeError('connection terminated');}}), {retries: 1});
  expect(brokenBody.retryableFailure).toBe(true);
});

it('skips unused owners and recovers through the default bounded wait', async () => {
  let calls = 0;
  expect(await request(async () => {calls++; return reply(200, []);}, {definitions: []})).toEqual({rows: [], retryableFailure: false});
  expect(calls).toBe(0);
  const recovered = await loadOwnerRows({...base, context: {ticker: 'AAPL'}, fetcher: async () => {
    calls++;
    return calls === 1 ? {...reply(429, 'limited'), headers: new Headers({'Retry-After': '0'})} : reply(200, [{id: 'one'}]);
  }});
  expect(calls).toBe(2);
  expect(recovered.issue).toBeUndefined();
});

const timedOut = () => Object.assign(new Error('signal timed out'), {name: 'TimeoutError'});
it('never repeats a timed-out, dropped or 5xx call, and asks the engine to be left alone afterwards', async () => {
  for (const fail of [async () => {throw timedOut();}, async () => {throw new TypeError('network');}, async () => reply(504, 'gateway timeout'), async () => reply(500, 'boom')]) {
    let calls = 0;
    const result = await request(async () => {calls++; return fail();});
    expect(calls).toBe(1);
    expect(result.retryableFailure).toBe(true);
    expect(result.retryAfterMs).toBe(30_000);
    expect(result.rows[0]).not.toHaveProperty('stats');
  }
});
it('waits exactly what Retry-After asks, then gives up on a wait too long to hold the page', async () => {
  const delays: number[] = [];
  const busy = (after: string) => ({...reply(429, 'busy'), headers: new Headers({'Retry-After': after})});
  let calls = 0;
  const recovered = await request(async () => ++calls === 1 ? busy('7') : reply(200, [{id: 'one'}]), {wait: async ms => {delays.push(ms);}});
  expect(delays).toEqual([7000]);
  expect(recovered.rows).toEqual([{id: 'one'}]);
  calls = 0;
  const tooLong = await request(async () => {calls++; return busy('600');});
  expect(calls).toBe(1);
  expect(tooLong.retryAfterMs).toBe(600_000);
  expect(retryAfterMs('3')).toBe(3000);
  expect(retryAfterMs('Thu, 01 Jan 2026 00:00:10 GMT', Date.parse('2026-01-01T00:00:00Z'))).toBe(10_000);
  expect(retryAfterMs('soon')).toBeUndefined();
  expect(retryAfterMs(null)).toBeUndefined();
});
it('stops at once when the user leaves: no retry, no blocked rows, nothing to cache', async () => {
  const leaving = new AbortController();
  let calls = 0;
  const inFlight = request((_url, init) => new Promise((_resolve, reject) => {
    calls++;
    init!.signal!.addEventListener('abort', () => reject(new DOMException('left', 'AbortError')));
  }), {signal: leaving.signal});
  leaving.abort();
  expect(await inFlight).toEqual({rows: [], retryableFailure: true, aborted: true});
  expect(calls).toBe(1);
  const waiting = new AbortController();
  calls = 0;
  const duringWait = loadOwnerRows({...base, context: {ticker: 'AAPL'}, signal: waiting.signal, fetcher: async () => {
    calls++;
    setTimeout(() => waiting.abort(), 0);
    return reply(503, 'busy');
  }});
  expect((await duringWait).aborted).toBe(true);
  expect(calls).toBe(1);
});
it('holds a failed result through the cool-down, shares it within a render, and then allows a new ask', () => {
  const issue = {owner: 'x', status: 'unavailable' as const, kind: 'request_failed' as const, note: 'n'};
  const record = failureRecord([{rows: [], retryableFailure: false}, {rows: [], issue, retryableFailure: true, retryAfterMs: 30_000}], 5, 1000)!;
  expect(record).toEqual({generation: 5, notBefore: 31_000});
  expect(failureRecord([{rows: [], retryableFailure: false}], 5)).toBeUndefined();
  expect(retryDue(record, 5, 99_000)).toBe(false);
  expect(retryDue(record, 6, 30_999)).toBe(false);
  expect(retryDue(record, 6, 31_000)).toBe(true);
  expect(retryDue(undefined, 6, 31_000)).toBe(false);
});
