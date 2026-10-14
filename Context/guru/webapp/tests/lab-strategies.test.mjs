// In-process route tests with a recording pool: no database, Docker or private .env needed.
// SQL behaviour itself is checked on an isolated dev stack by lab-strategies.integration.test.mjs.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFile } from 'node:fs/promises';
import { registerLabStrategies } from '../auth/src/lab-strategies.mjs';

const { Hono } = createRequire(new URL('../auth/package.json', import.meta.url))('hono');
const SPEC = { v: 1, entry: { all: [] }, exit: { any: [] } };
const USERS = { owner: { id: 'u1', role: 'researcher' }, other: { id: 'u2', role: 'researcher' }, viewer: { id: 'u3', role: 'viewer' }, admin: { id: 'u4', role: 'admin' } };

function harness(rows = {}) {
  const log = [], sqls = [], args = [];
  const query = async (sql, params = []) => {
    log.push(sql.trim().split(/\s+/)[0]);
    sqls.push(sql);
    args.push(params);
    if (/RETURNING id/.test(sql)) return { rows: [{ id: 9 }], rowCount: 1 };
    if (/^SELECT/.test(sql.trim()) && /WHERE s\.id=\$1/.test(sql)) return { rows: rows[params[0]] ? [rows[params[0]]] : [], rowCount: rows[params[0]] ? 1 : 0 };
    return { rows: [], rowCount: 0 };
  };
  const pool = { query, connect: async () => ({ query, release() {} }) };
  const app = new Hono();
  app.use('*', async (c, next) => { c.set('user', USERS[c.req.header('x-test-user')]); await next(); });
  registerLabStrategies(app, pool);
  const call = (user, method, path, body) => app.request(`/api/app/lab/strategies${path}`, { method, headers: { 'x-test-user': user, 'Content-Type': 'application/json' }, body: body === undefined ? undefined : JSON.stringify(body) });
  return { log, sqls, args, call };
}
const row = fields => ({ id: 7, owner: 'u1', ownerName: 'Owner', name: 'RSI dip', spec: SPEC, market: 'in', instrument: 'NIFTY 50', status: 'draft', version: 3, ...fields });

test('viewers cannot create and oversized specs are refused before any query', async () => {
  const { log, call } = harness();
  let response = await call('viewer', 'POST', '', { name: 'A', spec: SPEC, market: 'in', instrument: 'X' });
  assert.equal(response.status, 403);
  response = await call('owner', 'POST', '', { name: 'A', spec: { ...SPEC, pad: 'x'.repeat(8200) }, market: 'in', instrument: 'X' });
  assert.equal(response.status, 400);
  assert.match((await response.json()).error, /8 KB/);
  assert.deepEqual(log, []);
});

test('another user reading or editing a draft gets 404 and nothing is written', async () => {
  const { log, call } = harness({ 7: row() });
  assert.equal((await call('other', 'GET', '/7')).status, 404);
  assert.equal((await call('viewer', 'GET', '/7')).status, 404);
  assert.equal((await call('other', 'PUT', '/7', { version: 3, name: 'B', spec: SPEC, market: 'in', instrument: 'X' })).status, 404);
  assert.equal((await call('other', 'DELETE', '/7')).status, 404);
  assert.equal((await call('owner', 'GET', '/7')).status, 200);
  assert.equal((await call('admin', 'GET', '/7')).status, 200);
  assert.equal((await call('owner', 'GET', '/8')).status, 404);
  assert.ok(!log.includes('UPDATE') && !log.includes('DELETE') && !log.includes('INSERT'));
});

test('editing a published strategy or saving a stale version is a 409 and rolls back', async () => {
  const published = harness({ 7: row({ status: 'published' }) });
  let response = await published.call('owner', 'PUT', '/7', { version: 3, name: 'B', spec: SPEC, market: 'in', instrument: 'X' });
  assert.equal(response.status, 409);
  assert.match((await response.json()).error, /published strategy cannot be edited/);
  assert.equal((await published.call('owner', 'DELETE', '/7')).status, 409);
  const stale = harness({ 7: row() });
  response = await stale.call('owner', 'PUT', '/7', { version: 2, name: 'B', spec: SPEC, market: 'in', instrument: 'X' });
  assert.equal(response.status, 409);
  assert.match((await response.json()).error, /version 3/);
  assert.equal((await stale.call('owner', 'PUT', '/7', { name: 'B', spec: SPEC, market: 'in', instrument: 'X' })).status, 400);
  for (const { log } of [published, stale]) {
    assert.ok(!log.includes('UPDATE') && !log.includes('DELETE'));
    assert.equal(log.filter(s => s === 'ROLLBACK').length, log.filter(s => s === 'BEGIN').length);
  }
});

test('a crypto strategy cannot be submitted to YT Strategies', async () => {
  const { log, call } = harness({ 7: row({ market: 'crypto', instrument: 'BTCUSD' }) });
  const response = await call('owner', 'POST', '/7/submit', { version: 3 });
  assert.equal(response.status, 400);
  assert.match((await response.json()).error, /provisional/);
  assert.ok(!log.includes('UPDATE'));
});

test('review routes are administrator-only and rejection needs a note', async () => {
  const { log, call } = harness({ 7: row({ status: 'submitted' }) });
  assert.equal((await call('owner', 'GET', '/review')).status, 403);
  assert.equal((await call('owner', 'POST', '/7/publish', { version: 3 })).status, 403);
  assert.equal((await call('admin', 'POST', '/7/reject', { version: 3 })).status, 400);
  assert.equal((await call('admin', 'POST', '/7/reject', { version: 3, note: 'x'.repeat(1001) })).status, 400);
  assert.equal((await call('owner', 'GET', '?status=archived')).status, 400);
  assert.ok(!log.includes('UPDATE'));
});

test('deleting needs the current version in the query string', async () => {
  const { log, call } = harness({ 7: row() });
  assert.equal((await call('owner', 'DELETE', '/7')).status, 400);
  assert.equal((await call('owner', 'DELETE', '/7?version=abc')).status, 400);
  assert.equal((await call('owner', 'DELETE', '/7?version=2')).status, 409);
  assert.ok(!log.includes('DELETE'));
  assert.equal((await call('owner', 'DELETE', '/7?version=3')).status, 204);
  assert.ok(log.includes('DELETE') && log.includes('COMMIT'));
});

test('malformed bodies and ids are client errors, never a server error or a query', async () => {
  const { log, call } = harness({ 7: row() });
  for (const [method, path] of [['POST', ''], ['PUT', '/7'], ['POST', '/7/submit']]) {
    assert.equal((await call('owner', method, path)).status, 400, `${method} ${path} without a JSON body`);
    assert.equal((await call('owner', method, path, [SPEC])).status, 400, `${method} ${path} with an array body`);
  }
  for (const path of ['/abc', '/0', '/07', '/1234567890', '/-1', '/7/archive', '/1.5'])
    assert.equal((await call('owner', 'GET', path)).status, 404, path);
  assert.deepEqual(log, []);
});

test('publishing records one frozen snapshot of the reviewed version and does not bump it', async () => {
  const { sqls, call } = harness({ 7: row({ status: 'submitted' }) });
  const response = await call('admin', 'POST', '/7/publish', { version: 3, note: 'Looks right' });
  assert.equal(response.status, 200);
  const snapshot = sqls.filter(sql => /INSERT INTO lab_strategy_version/.test(sql));
  assert.equal(snapshot.length, 1);
  const update = sqls.find(sql => /^\s*UPDATE lab_strategy SET/.test(sql));
  assert.match(update, /status='published'/);
  assert.doesNotMatch(update, /version/);
  assert.ok(sqls.some(sql => /INSERT INTO vault_audit/.test(sql)));
  assert.ok(sqls.some(sql => /FOR UPDATE OF s/.test(sql)));
});

test('route SQL never updates or deletes a published snapshot', async () => {
  const source = await readFile(new URL('../auth/src/lab-strategies.mjs', import.meta.url), 'utf8');
  assert.doesNotMatch(source, /UPDATE\s+lab_strategy_version/i);
  assert.doesNotMatch(source, /DELETE\s+FROM\s+lab_strategy_version/i);
  assert.doesNotMatch(source, /TRUNCATE/i);
});

const ORIGIN = { book: 'stc9', method: 'rsi2_4', template_version: 1 };
test('a customized strategy stores its origin, and an edit cannot change it', async () => {
  const created = harness();
  assert.equal((await created.call('owner', 'POST', '', { name: 'My RSI(2)', spec: { ...SPEC, origin: ORIGIN }, market: 'us', instrument: 'SPY' })).status, 201);
  const insert = created.sqls.findIndex(sql => /INSERT INTO lab_strategy \(owner,name,spec,market,instrument,origin\)/.test(sql));
  assert.equal(created.args[insert][5], JSON.stringify(ORIGIN));
  const plain = harness();
  await plain.call('owner', 'POST', '', { name: 'Mine', spec: SPEC, market: 'us', instrument: 'SPY' });
  assert.equal(plain.args[plain.sqls.findIndex(sql => /INSERT INTO lab_strategy \(/.test(sql))][5], null);
  const variant = harness({ 7: row({ spec: { ...SPEC, origin: ORIGIN }, origin: ORIGIN }) });
  for (const spec of [SPEC, { ...SPEC, origin: { ...ORIGIN, method: 'rsi4_25' } }]) {
    const response = await variant.call('owner', 'PUT', '/7', { version: 3, name: 'B', spec, market: 'us', instrument: 'SPY' });
    assert.equal(response.status, 400);
    assert.match((await response.json()).error, /keeps the book template/);
  }
  assert.ok(!variant.log.includes('UPDATE'));
  assert.equal((await variant.call('owner', 'PUT', '/7', { version: 3, name: 'B', spec: { ...SPEC, origin: ORIGIN, exit: { any: [], stop_loss: { pct: 5 } } }, market: 'us', instrument: 'SPY' })).status, 200);
  assert.ok(variant.log.includes('UPDATE'));
  const fresh = harness({ 7: row() });
  assert.equal((await fresh.call('owner', 'PUT', '/7', { version: 3, name: 'B', spec: { ...SPEC, origin: ORIGIN }, market: 'us', instrument: 'SPY' })).status, 400, 'origin is set at creation only');
  assert.ok(!fresh.log.includes('UPDATE'));
});

test('duplicating a variant keeps its origin and YT Strategies lists the origin of each snapshot', async () => {
  const { sqls, args, call } = harness({ 7: row({ status: 'published', spec: { ...SPEC, origin: ORIGIN }, origin: ORIGIN }) });
  assert.equal((await call('other', 'POST', '/7/duplicate')).status, 201);
  assert.equal(args[sqls.findIndex(sql => /INSERT INTO lab_strategy \(/.test(sql))][5], JSON.stringify(ORIGIN));
  await call('viewer', 'GET', '/published');
  assert.ok(sqls.some(sql => /v\.spec->'origin' AS origin/.test(sql)));
});
