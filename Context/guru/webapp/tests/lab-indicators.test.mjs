// In-process route tests for saved custom indicators with a recording pool: no database, Docker or private .env needed.
// SQL behaviour itself is checked on an isolated dev stack by lab-indicators.integration.test.mjs.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { registerLabIndicators } from '../auth/src/lab-indicators.mjs';

const { Hono } = createRequire(new URL('../auth/package.json', import.meta.url))('hono');
const USERS = { owner: { id: 'u1', role: 'researcher' }, other: { id: 'u2', role: 'researcher' }, viewer: { id: 'u3', role: 'viewer' }, admin: { id: 'u4', role: 'admin' } };
const FORMULA = 'Fast = Param("Fast", 12, 2, 50, 1);\nPlot(EMA(C, Fast) - EMA(C, 26), "Osc");';
const row = fields => ({ id: 7, owner: 'u1', name: 'My Momentum', formula: FORMULA, blocks: null, version: 3, createdAt: '2026-10-19T00:00:00Z', updatedAt: '2026-10-19T00:00:00Z', ...fields });

// taken: lower-case names the owner already has, so an INSERT/UPDATE to one raises the unique-index violation PostgreSQL would.
function harness(rows = {}, taken = []) {
  const log = [], sqls = [], args = [];
  const query = async (sql, params = []) => {
    log.push(sql.trim().split(/\s+/)[0]);
    sqls.push(sql);
    args.push(params);
    if (/^(INSERT INTO lab_indicator|UPDATE lab_indicator)/.test(sql.trim())) {
      const name = params[1];
      if (taken.includes(name.toLowerCase())) throw Object.assign(new Error('duplicate key'), { code: '23505', constraint: 'lab_indicator_owner_name_idx' });
      const base = /^INSERT/.test(sql.trim()) ? row({ id: 9, owner: params[0], version: 1 }) : { ...rows[params[0]], version: rows[params[0]].version + 1 };
      return { rows: [{ ...base, name, formula: params[2], blocks: params[3] === null ? null : JSON.parse(params[3]) }], rowCount: 1 };
    }
    if (/FROM lab_indicator WHERE id=\$1/.test(sql)) return { rows: rows[params[0]] ? [rows[params[0]]] : [], rowCount: rows[params[0]] ? 1 : 0 };
    if (/FROM lab_indicator WHERE owner=\$1/.test(sql)) { const mine = Object.values(rows).filter(r => r.owner === params[0]); return { rows: mine, rowCount: mine.length }; }
    return { rows: [], rowCount: 0 };
  };
  const pool = { query, connect: async () => ({ query, release() {} }) };
  const app = new Hono();
  app.use('*', async (c, next) => { c.set('user', USERS[c.req.header('x-test-user')]); await next(); });
  registerLabIndicators(app, pool);
  const call = (user, method, path, body) => app.request(`/api/app/lab/indicators${path}`, { method, headers: { 'x-test-user': user, 'Content-Type': 'application/json' }, body: body === undefined ? undefined : JSON.stringify(body) });
  return { log, sqls, args, call };
}
const balanced = log => assert.equal(log.filter(s => s === 'ROLLBACK').length + log.filter(s => s === 'COMMIT').length, log.filter(s => s === 'BEGIN').length);

test('the list holds only the caller\'s indicators, without the owner id', async () => {
  const { args, call } = harness({ 7: row(), 8: row({ id: 8, owner: 'u2', name: 'Theirs' }) });
  const response = await call('owner', 'GET', '');
  assert.equal(response.status, 200);
  const list = await response.json();
  assert.deepEqual(list, [{ id: 7, name: 'My Momentum', formula: FORMULA, blocks: null, version: 3, createdAt: '2026-10-19T00:00:00Z', updatedAt: '2026-10-19T00:00:00Z' }]);
  assert.deepEqual(args[0], ['u1']);
  assert.deepEqual(await (await call('viewer', 'GET', '')).json(), []);
});

test('create trims the name, keeps the formula text exactly, returns 201 and writes one audit row', async () => {
  const { sqls, args, log, call } = harness();
  const response = await call('owner', 'POST', '', { name: '  My Momentum ', formula: FORMULA });
  assert.equal(response.status, 201);
  const json = await response.json();
  assert.deepEqual(Object.keys(json).sort(), ['blocks', 'createdAt', 'formula', 'id', 'name', 'updatedAt', 'version']);
  assert.deepEqual([json.id, json.name, json.formula, json.blocks, json.version], [9, 'My Momentum', FORMULA, null, 1]);
  const insert = sqls.findIndex(sql => /INSERT INTO lab_indicator/.test(sql));
  assert.deepEqual(args[insert], ['u1', 'My Momentum', FORMULA, null]);
  const audit = sqls.findIndex(sql => /INSERT INTO vault_audit/.test(sql));
  assert.deepEqual(args[audit], ['u1', 'lab.indicator.create', 'lab_indicator:9']);
  assert.ok(log.includes('COMMIT'));
});

test('viewers cannot create, and bad names or formulas are refused before any query', async () => {
  const { log, call } = harness();
  assert.equal((await call('viewer', 'POST', '', { name: 'A', formula: FORMULA })).status, 403);
  for (const [input, message] of [
    [{ name: '', formula: FORMULA }, /1–60/], [{ name: '   ', formula: FORMULA }, /1–60/], [{ name: 'x'.repeat(61), formula: FORMULA }, /1–60/],
    [{ name: 'a\nb', formula: FORMULA }, /1–60/], [{ formula: FORMULA }, /1–60/], [{ name: 7, formula: FORMULA }, /1–60/],
    [{ name: 'A' }, /1–4000/], [{ name: 'A', formula: '  \n ' }, /1–4000/], [{ name: 'A', formula: 'x'.repeat(4001) }, /1–4000/], [{ name: 'A', formula: ['C'] }, /1–4000/],
    [{ name: 'A', formula: 'C\u0000' }, /control characters/],
  ]) {
    const response = await call('owner', 'POST', '', input);
    assert.equal(response.status, 400, JSON.stringify(input));
    assert.match((await response.json()).error, message);
  }
  assert.equal((await call('owner', 'POST', '', { name: 'x'.repeat(60), formula: 'x'.repeat(4000) })).status, 201, 'the limits themselves are allowed');
  assert.equal((await call('owner', 'POST', '', { name: 'Tabs', formula: 'a = C;\r\n\tPlot(a, "A");' })).status, 201, 'tabs and line breaks are allowed');
  assert.equal(log.filter(s => s === 'INSERT').length, 4, 'only the two valid creates (each with its audit row) reached the database');
});

test('a name the owner already uses, in any case, is a 409 and rolls back', async () => {
  const created = harness({}, ['my momentum']);
  let response = await created.call('owner', 'POST', '', { name: 'MY MOMENTUM', formula: FORMULA });
  assert.equal(response.status, 409);
  assert.match((await response.json()).error, /already have an indicator with this name/);
  assert.ok(!created.sqls.some(sql => /vault_audit/.test(sql)));
  const renamed = harness({ 7: row({ name: 'Other' }) }, ['my momentum']);
  response = await renamed.call('owner', 'PUT', '/7', { version: 3, name: 'my momentum', formula: FORMULA });
  assert.equal(response.status, 409);
  for (const { log } of [created, renamed]) { assert.ok(log.includes('ROLLBACK') && !log.includes('COMMIT')); balanced(log); }
});

test('an edit locks the row, bumps the version and returns the saved row', async () => {
  const { sqls, args, call } = harness({ 7: row() });
  const response = await call('owner', 'PUT', '/7', { version: 3, name: ' Renamed ', formula: 'Plot(C, "Close");' });
  assert.equal(response.status, 200);
  const json = await response.json();
  assert.deepEqual([json.id, json.name, json.formula, json.version, json.owner], [7, 'Renamed', 'Plot(C, "Close");', 4, undefined]);
  assert.ok(sqls.some(sql => /FROM lab_indicator WHERE id=\$1 FOR UPDATE/.test(sql)));
  assert.match(sqls.find(sql => /^UPDATE lab_indicator/.test(sql.trim())), /version=version\+1/);
  assert.deepEqual(args[sqls.findIndex(sql => /vault_audit/.test(sql))], ['u1', 'lab.indicator.update', 'lab_indicator:7']);
});

test('another user, a missing row or a stale version never writes', async () => {
  const { log, call } = harness({ 7: row() });
  const edit = { version: 3, name: 'B', formula: FORMULA };
  assert.equal((await call('other', 'PUT', '/7', edit)).status, 404);
  assert.equal((await call('admin', 'PUT', '/7', edit)).status, 404, 'indicators are private, even from administrators');
  assert.equal((await call('other', 'DELETE', '/7?version=3')).status, 404);
  assert.equal((await call('owner', 'PUT', '/8', edit)).status, 404);
  let response = await call('owner', 'PUT', '/7', { ...edit, version: 2 });
  assert.equal(response.status, 409);
  assert.match((await response.json()).error, /now version 3/);
  assert.equal((await call('owner', 'PUT', '/7', { name: 'B', formula: FORMULA })).status, 400);
  assert.equal((await call('owner', 'PUT', '/7', { ...edit, version: '3' })).status, 400);
  assert.ok(!log.includes('UPDATE') && !log.includes('DELETE') && !log.includes('INSERT'));
  balanced(log);
});

test('a viewer cannot change even an indicator they own', async () => {
  const { log, call } = harness({ 7: row({ owner: 'u3' }) });
  assert.equal((await call('viewer', 'PUT', '/7', { version: 3, name: 'B', formula: FORMULA })).status, 403);
  assert.equal((await call('viewer', 'DELETE', '/7?version=3')).status, 403);
  assert.ok(!log.includes('UPDATE') && !log.includes('DELETE'));
});

test('deleting needs the current version in the query string', async () => {
  const { log, sqls, args, call } = harness({ 7: row() });
  for (const path of ['/7', '/7?version=', '/7?version=abc', '/7?version=0'])
    assert.equal((await call('owner', 'DELETE', path)).status, 400, path);
  assert.equal((await call('owner', 'DELETE', '/7?version=2')).status, 409);
  assert.ok(!log.includes('DELETE'));
  const response = await call('owner', 'DELETE', '/7?version=3');
  assert.equal(response.status, 204);
  assert.equal(await response.text(), '');
  assert.ok(log.includes('DELETE') && log.includes('COMMIT'));
  assert.deepEqual(args[sqls.findIndex(sql => /vault_audit/.test(sql))], ['u1', 'lab.indicator.delete', 'lab_indicator:7']);
});

test('malformed bodies and ids are client errors, never a server error or a query', async () => {
  const { log, call } = harness({ 7: row() });
  for (const [method, path] of [['POST', ''], ['PUT', '/7']]) {
    assert.equal((await call('owner', method, path)).status, 400, `${method} ${path} without a JSON body`);
    assert.equal((await call('owner', method, path, [FORMULA])).status, 400, `${method} ${path} with an array body`);
  }
  for (const path of ['/abc', '/0', '/07', '/1234567890', '/-1', '/1.5'])
    assert.equal((await call('owner', 'PUT', path, { version: 1, name: 'A', formula: FORMULA })).status, 404, path);
  assert.deepEqual(log, []);
});

const BLOCKS = { v: 1, nodes: [{ id: 'n1', kind: 'ema', x: 40, y: 80 }], links: [] };
test('a block layout is stored with the formula, returned as-is, and cleared by an edit that omits it', async () => {
  const created = harness();
  let response = await created.call('owner', 'POST', '', { name: 'Blocks', formula: FORMULA, blocks: BLOCKS });
  assert.equal(response.status, 201);
  assert.deepEqual((await response.json()).blocks, BLOCKS);
  assert.equal(created.args[created.sqls.findIndex(sql => /INSERT INTO lab_indicator/.test(sql))][3], JSON.stringify(BLOCKS));
  assert.match(created.sqls.find(sql => /INSERT INTO lab_indicator/.test(sql)), /\$4::jsonb/);
  assert.equal((await created.call('owner', 'POST', '', { name: 'Null blocks', formula: FORMULA, blocks: null })).status, 201);
  const edited = harness({ 7: row({ blocks: BLOCKS }) });
  response = await edited.call('owner', 'PUT', '/7', { version: 3, name: 'Typed', formula: FORMULA });
  assert.equal(response.status, 200);
  assert.equal((await response.json()).blocks, null);
  response = await edited.call('owner', 'PUT', '/7', { version: 3, name: 'Kept', formula: FORMULA, blocks: BLOCKS });
  assert.deepEqual((await response.json()).blocks, BLOCKS);
});

test('blocks must be a JSON object within 64 KB', async () => {
  const { log, call } = harness();
  for (const [blocks, message] of [[[BLOCKS], /JSON object or null/], ['{}', /JSON object or null/], [3, /JSON object or null/], [false, /JSON object or null/],
    [{ pad: 'x'.repeat(65536) }, /64 KB/], [{ label: 'a\u0000b' }, /NUL/]]) {
    const response = await call('owner', 'POST', '', { name: 'A', formula: FORMULA, blocks });
    assert.equal(response.status, 400, JSON.stringify(blocks).slice(0, 40));
    assert.match((await response.json()).error, message);
  }
  assert.deepEqual(log, []);
  assert.equal((await call('owner', 'POST', '', { name: 'A', formula: FORMULA, blocks: { pad: 'x'.repeat(65000) } })).status, 201);
});
