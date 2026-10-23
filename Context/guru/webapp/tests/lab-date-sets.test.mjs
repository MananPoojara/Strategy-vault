// In-process route tests for custom date filter sets with a recording pool: no database, Docker or private .env needed.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { registerLabDateSets } from '../auth/src/lab-date-sets.mjs';
import { validateLabDateSet } from '../auth/src/contracts.mjs';

const { Hono } = createRequire(new URL('../auth/package.json', import.meta.url))('hono');
const USERS = { owner: { id: 'u1', role: 'researcher' }, other: { id: 'u2', role: 'researcher' }, viewer: { id: 'u3', role: 'viewer' }, admin: { id: 'u4', role: 'admin' } };
const W = [{ from: '2008-01-01', to: '2008-01-31' }, { from: '2008-11-21', to: '2010-12-31', label: 'recovery' }];
const row = fields => ({ id: 7, owner: 'u1', shared: false, name: 'Base 2', windows: W, version: 3, createdAt: '2026-10-22T00:00:00Z', updatedAt: '2026-10-22T00:00:00Z', ...fields });

function harness(rows = {}, taken = {}) {
  const log = [], sqls = [], args = [];
  const query = async (sql, params = []) => {
    log.push(sql.trim().split(/\s+/)[0]);
    sqls.push(sql); args.push(params);
    const s = sql.trim();
    if (/^INSERT INTO lab_date_set/.test(s)) {
      const [owner, shared, name, windows] = params;
      if (taken[name.toLowerCase()]) throw Object.assign(new Error('duplicate key'), { code: '23505', constraint: taken[name.toLowerCase()] });
      return { rows: [row({ id: 9, owner, shared, name, windows: JSON.parse(windows), version: 1 })], rowCount: 1 };
    }
    if (/^UPDATE lab_date_set/.test(s)) {
      const [id, name, windows] = params;
      return { rows: [{ ...rows[id], name, windows: JSON.parse(windows), version: rows[id].version + 1 }], rowCount: 1 };
    }
    if (/FROM lab_date_set WHERE id=\$1/.test(sql)) return { rows: rows[params[0]] ? [rows[params[0]]] : [], rowCount: rows[params[0]] ? 1 : 0 };
    if (/FROM lab_date_set WHERE owner=\$1 OR shared/.test(sql)) { const seen = Object.values(rows).filter(r => r.owner === params[0] || r.shared); return { rows: seen, rowCount: seen.length }; }
    return { rows: [], rowCount: 0 };
  };
  const pool = { query, connect: async () => ({ query, release() {} }) };
  const app = new Hono();
  app.use('*', async (c, next) => { c.set('user', USERS[c.req.header('x-test-user')]); await next(); });
  registerLabDateSets(app, pool);
  const call = (user, method, path, body) => app.request(`/api/app/lab/date-sets${path}`, { method, headers: { 'x-test-user': user, 'Content-Type': 'application/json' }, body: body === undefined ? undefined : JSON.stringify(body) });
  return { log, sqls, args, call };
}

test('the list holds the caller\'s own sets and every shared set, with mine / editable flags and no owner id', async () => {
  const rows = { 7: row(), 8: row({ id: 8, owner: 'u2', name: 'Theirs' }), 10: row({ id: 10, owner: 'u4', shared: true, name: 'Official Base 2' }) };
  const { call } = harness(rows);
  const mine = await (await call('owner', 'GET', '')).json();
  assert.deepEqual(mine.map(x => [x.id, x.mine, x.editable, x.shared]), [[7, true, true, false], [10, false, false, true]]);
  assert.ok(mine.every(x => !('owner' in x)));
  const admin = await (await call('admin', 'GET', '')).json();
  assert.deepEqual(admin.map(x => [x.id, x.mine, x.editable]), [[10, true, true]]);
  const viewer = await (await call('viewer', 'GET', '')).json();
  assert.deepEqual(viewer.map(x => [x.id, x.editable]), [[10, false]]);
});

test('create: researchers save personal sets; only an admin shares; one audit row', async () => {
  const { sqls, args, call } = harness();
  const response = await call('owner', 'POST', '', { name: ' Base 2 ', windows: W });
  assert.equal(response.status, 201);
  const json = await response.json();
  assert.deepEqual([json.name, json.shared, json.mine, json.editable, json.windows], ['Base 2', false, true, true, W]);
  const audit = sqls.findIndex(sql => /INSERT INTO vault_audit/.test(sql));
  assert.deepEqual(args[audit], ['u1', 'lab.date_set.create', 'lab_date_set:9']);
  assert.equal((await call('owner', 'POST', '', { name: 'Shared', windows: W, shared: true })).status, 403);
  assert.equal((await call('viewer', 'POST', '', { name: 'Mine', windows: W })).status, 403);
  const shared = await call('admin', 'POST', '', { name: 'Official', windows: W, shared: true });
  assert.equal(shared.status, 201);
  assert.equal((await shared.json()).shared, true);
});

test('duplicate names give a clear 409', async () => {
  const { call } = harness({}, { 'base 2': 'lab_date_set_owner_name_idx', official: 'lab_date_set_shared_name_idx' });
  const a = await call('owner', 'POST', '', { name: 'BASE 2', windows: W });
  assert.equal(a.status, 409);
  assert.match((await a.json()).error, /already have a date filter/);
  const b = await call('admin', 'POST', '', { name: 'Official', windows: W, shared: true });
  assert.match((await b.json()).error, /shared date filter with this name/);
});

test('update (re-upload) and delete: owner for personal, any admin for shared, version checked, sharing fixed', async () => {
  const rows = { 7: row(), 10: row({ id: 10, owner: 'u4', shared: true, name: 'Official', version: 1 }) };
  const { call } = harness(rows);
  const next = [{ from: '2020-03-24', to: '2021-11-30' }];
  const ok = await call('owner', 'PUT', '/7', { name: 'Base 2', windows: next, version: 3 });
  assert.equal(ok.status, 200);
  assert.deepEqual((await ok.json()).windows, next);
  assert.equal((await call('owner', 'PUT', '/7', { name: 'Base 2', windows: next, version: 2 })).status, 409);
  assert.equal((await call('other', 'PUT', '/7', { name: 'Base 2', windows: next, version: 3 })).status, 404);
  assert.equal((await call('owner', 'PUT', '/10', { name: 'Official', windows: next, version: 1 })).status, 403);
  assert.equal((await call('admin', 'PUT', '/7', { name: 'Base 2', windows: next, version: 3 })).status, 404);
  assert.equal((await call('admin', 'PUT', '/10', { name: 'Official', windows: next, version: 1 })).status, 200);
  assert.equal((await call('owner', 'PUT', '/7', { name: 'Base 2', windows: next, version: 3, shared: true })).status, 400);
  assert.equal((await call('owner', 'DELETE', '/7?version=3')).status, 204);
  assert.equal((await call('owner', 'DELETE', '/10?version=1')).status, 403);
  assert.equal((await call('admin', 'DELETE', '/10?version=1')).status, 204);
});

test('validation: names, 1-200 windows, real ISO dates, from before to, labels, no extra fields', () => {
  assert.deepEqual(validateLabDateSet({ name: 'x', windows: [{ from: '2024-02-29', to: '2024-03-01', label: ' Q1 ' }] }).windows, [{ from: '2024-02-29', to: '2024-03-01', label: 'Q1' }]);
  const bad = (body, msg) => assert.throws(() => validateLabDateSet(body), e => msg.test(e.message), JSON.stringify(body));
  bad({ name: '', windows: W }, /Name must be/);
  bad({ name: 'x'.repeat(61), windows: W }, /Name must be/);
  bad({ name: 'x', windows: [] }, /1 to 200/);
  bad({ name: 'x', windows: Array(201).fill(W[0]) }, /1 to 200/);
  bad({ name: 'x', windows: [{ from: '2023-02-29', to: '2023-03-01' }] }, /real dates/);
  bad({ name: 'x', windows: [{ from: '01-01-2008', to: '2008-01-31' }] }, /real dates/);
  bad({ name: 'x', windows: [{ from: '2008-01-31', to: '2008-01-31' }] }, /must be before/);
  bad({ name: 'x', windows: [{ from: '2008-01-01', to: '2008-01-31', note: 1 }] }, /unknown field/);
  bad({ name: 'x', windows: [{ from: '2008-01-01', to: '2008-01-31', label: 'x'.repeat(61) }] }, /label/);
  bad({ name: 'x', windows: W, shared: 'yes' }, /shared must be/);
});
