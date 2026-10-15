import {expect, it} from 'vitest';
import {dateCoverageIssue, loadOwnerRows, type OwnerRowsOptions} from '../src/services/summary-loader';

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
  expect(delays).toEqual([300, 300]);
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
    return calls === 1 ? reply(429, 'limited') : reply(200, [{id: 'one'}]);
  }});
  expect(calls).toBe(2);
  expect(recovered.issue).toBeUndefined();
});
