import test from 'node:test';
import assert from 'node:assert/strict';
import {dateCoverageIssue, loadOwnerRows} from '../../frontend/react/src/services/summary-loader.ts';

const context = {ticker: 'basket:us_sp500:AAPL', label: 'AAPL', from: '2010-01-01', to: '2020-09-11', first: '1980-12-12', last: '2020-09-11'};
const definitions = [{id: 'owned_one'}, {id: 'owned_two'}];
const url = '/api/us/guru/connors/summary?ticker=basket%3Aus_sp500%3AAAPL&from=2010-01-01&to=2020-09-11';
const response = (status, body) => ({ok: status >= 200 && status < 300, status, text: async () => typeof body === 'string' ? body : JSON.stringify(body), json: async () => body});
function options(fetcher, overrides = {}) {
  return {owner: 'connors', definitions, context, url, kind: 'summary', fetcher, wait: async () => {}, ...overrides};
}
function assertBlocked(result, kind = 'summary') {
  assert.deepEqual(result.rows.map(row => row.id), definitions.map(row => row.id));
  assert.ok(result.rows.every(row => !Object.hasOwn(row, 'stats') && !Object.hasOwn(row, 'spark')));
  assert.ok(result.rows.every(row => kind === 'readiness' ? row.status === result.issue.status && row.note === result.issue.note : row.readiness === result.issue.status && row.blocked === result.issue.note));
}

test('date preflight rejects actual out-of-coverage periods without requesting or fabricating results', async () => {
  let calls = 0;
  const result = await loadOwnerRows(options(async () => { calls++; }, {context: {...context, from: '2024-01-01', to: '2025-12-31'}}));
  assert.equal(calls, 0);
  assert.equal(result.issue.kind, 'empty_range');
  assert.equal(result.issue.owner, 'connors');
  assert.equal(result.issue.status, 'needs_data');
  assert.match(result.issue.note, /AAPL/);
  assert.match(result.issue.note, /1980-12-12 to 2020-09-11/);
  assert.equal(result.retryableFailure, false);
  assertBlocked(result);
});

test('coverage before its first bar is also unavailable, while partially overlapping ranges are not silently clipped', () => {
  assert.equal(dateCoverageIssue({...context, from: '', to: '1970-01-01'}).kind, 'empty_range');
  assert.equal(dateCoverageIssue({...context, from: '1970-01-01', to: '1990-01-01'}), undefined);
  assert.equal(dateCoverageIssue({...context, from: '2010-01-01', to: '2030-01-01'}), undefined);
});

test('reversed, equal and invalid calendar dates are rejected without repairing dates', () => {
  for (const overrides of [
    {from: '2020-09-12', to: '2020-09-11'},
    {from: '2020-09-11', to: '2020-09-11'},
    {from: '2023-02-29'}, {to: '2020-02-30'}, {from: '01-01-2020'},
  ]) assert.equal(dateCoverageIssue({...context, ...overrides}).kind, 'invalid_range');
  assert.equal(dateCoverageIssue({...context, from: '2020-02-29'}), undefined);
});

test('unknown or inconsistent coverage is never replaced by guessed history', () => {
  assert.equal(dateCoverageIssue({ticker: 'NEW', from: '2030-01-01', to: '2031-01-01'}), undefined);
  assert.equal(dateCoverageIssue({ticker: 'NEW', from: '2030-01-01', first: 'not a date', last: 'not a date'}), undefined);
  assert.equal(dateCoverageIssue({...context, first: '2026-01-01', last: '2000-01-01'}), undefined);
});

test('successful summary and readiness payloads preserve actual owned rows and attach a bounded abort signal', async () => {
  const row = {id: 'owned_one', stats: {trades: 2, net_profit: -0.01}}, signals = [];
  const result = await loadOwnerRows(options(async (requested, init) => { assert.equal(requested, url); signals.push(init.signal); return response(200, [row]); }));
  assert.equal(result.rows[0], row);
  assert.equal(result.issue, undefined);
  assert.equal(result.retryableFailure, false);
  assert.ok(signals[0] instanceof AbortSignal);
  assert.equal(signals[0].aborted, false);
  const ready = {id: 'owned_two', status: 'ready', note: 'Observed inputs passed'};
  const readiness = await loadOwnerRows(options(async () => response(200, {strategies: [ready]}), {kind: 'readiness'}));
  assert.equal(readiness.rows[0], ready);
});

test('plain and JSON empty-date errors are deterministic and are not retried', async () => {
  for (const body of ['empty date range', {error: 'empty date range'}, {error: {field: 'from', message: 'empty date range'}}]) {
    let calls = 0, waits = 0;
    const result = await loadOwnerRows(options(async () => { calls++; return response(400, body); }, {wait: async () => { waits++; }}));
    assert.equal(calls, 1); assert.equal(waits, 0);
    assert.equal(result.issue.kind, 'empty_range');
    assert.equal(result.issue.status, 'needs_data');
    assert.match(result.issue.note, /fewer than two usable price bars/);
    assert.doesNotMatch(result.issue.note, /No price bars/);
    assert.equal(result.retryableFailure, false);
    assertBlocked(result);
  }
});

test('a valid owner remains available when another owner fails, without foreign blocked IDs', async () => {
  const healthy = {id: 'other_rule', stats: {trades: 4, net_profit: 0.02}};
  const [good, bad] = await Promise.all([
    loadOwnerRows(options(async () => response(200, [healthy]), {owner: 'chande', definitions: [{id: 'other_rule'}]})),
    loadOwnerRows(options(async () => response(400, 'empty date range'))),
  ]);
  assert.deepEqual(good.rows, [healthy]);
  assert.equal(good.issue, undefined);
  assertBlocked(bad);
  assert.ok(bad.rows.every(row => row.id !== 'other_rule'));
});

test('transient failures retry at most three times with the captured URL and injectable delay', async () => {
  const calls = [], waits = [];
  const result = await loadOwnerRows(options(async (requested) => { calls.push(requested); return response(503, {message: 'temporarily busy'}); }, {retries: 999, wait: async ms => { waits.push(ms); }}));
  assert.deepEqual(calls, [url, url, url]);
  assert.deepEqual(waits, [300, 300]);
  assert.equal(result.issue.kind, 'request_failed');
  assert.equal(result.issue.status, 'unavailable');
  assert.equal(result.retryableFailure, true);
  assertBlocked(result);
});

test('network failures and rate limiting can recover without inventing earlier failed results', async () => {
  let calls = 0;
  const actual = {id: 'owned_one', stats: {trades: 3, net_profit: 0.03}};
  const result = await loadOwnerRows(options(async () => {
    calls++;
    if (calls === 1) throw new TypeError('network down');
    if (calls === 2) return response(429, 'too many requests');
    return response(200, [actual]);
  }));
  assert.equal(calls, 3);
  assert.deepEqual(result.rows, [actual]);
  assert.equal(result.issue, undefined);
  assert.equal(result.retryableFailure, false);
});

test('wrong payload shapes, unknown/duplicate IDs and invalid JSON fail only this owner without retry', async () => {
  for (const body of [{strategies: []}, [null], [{id: 'foreign'}], [{id: 'owned_one'}, {id: 'owned_one'}], [{id: ''}]]) {
    let calls = 0;
    const result = await loadOwnerRows(options(async () => { calls++; return response(200, body); }));
    assert.equal(calls, 1);
    assert.equal(result.issue.kind, 'request_failed');
    assert.equal(result.retryableFailure, false);
    assertBlocked(result);
  }
  const malformed = await loadOwnerRows(options(async () => ({ok: true, status: 200, json: async () => {throw new SyntaxError('bad JSON');}, text: async () => ''})));
  assert.equal(malformed.issue.status, 'unavailable');
  assertBlocked(malformed);
});

test('readiness failures preserve their own unavailable status without fabricated ready values', async () => {
  const result = await loadOwnerRows(options(async () => response(403, {message: 'access denied'}), {kind: 'readiness'}));
  assert.equal(result.issue.status, 'unavailable');
  assert.match(result.issue.note, /access denied/);
  assert.equal(result.retryableFailure, false);
  assertBlocked(result, 'readiness');
});

test('an owner with no requested definitions does not perform unnecessary requests', async () => {
  let calls = 0;
  const result = await loadOwnerRows(options(async () => { calls++; }, {definitions: []}));
  assert.equal(calls, 0);
  assert.deepEqual(result, {rows: [], retryableFailure: false});
});
