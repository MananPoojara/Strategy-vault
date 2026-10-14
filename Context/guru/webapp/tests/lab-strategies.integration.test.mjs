// Manual gate: saved lab strategies against an ISOLATED dev stack (docs/DEVELOPMENT.md "Two developers on one machine").
// VAULT_ENV_FILE=<dev runtime>/.env VAULT_COMPOSE_PROJECT=strategy-vault-dev-<name> node --test tests/lab-strategies.integration.test.mjs
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { randomBytes } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { basename, dirname, resolve } from 'node:path';

const { VAULT_ENV_FILE: envFile, VAULT_COMPOSE_PROJECT: project } = process.env;
// Skips (never fails) unless both variables are set; the live project name is always refused.
const enabled = Boolean(envFile && project);
if (project !== undefined && !/^strategy-vault-dev-[a-z0-9-]+$/.test(project)) throw new Error(`VAULT_COMPOSE_PROJECT must name an isolated dev stack (strategy-vault-dev-<name>); "${project}" is refused.`);
const appDirectory = fileURLToPath(new URL('..', import.meta.url));
const env = enabled ? Object.fromEntries((await readFile(envFile, 'utf8')).split('\n').filter(x => x && !x.startsWith('#')).map(x => { const i = x.indexOf('='); return [x.slice(0, i), x.slice(i + 1)]; })) : {};
const base = process.env.VAULT_TEST_URL || env.VAULT_PUBLIC_URL;
// Live and dev stacks share hosts and differ by port and runtime dir: the env file must sit in ~/.cache/<project>/ and the live port 8180 is refused.
if (enabled && basename(dirname(resolve(envFile))) !== project) throw new Error(`VAULT_ENV_FILE must be the ${project} runtime's own .env (~/.cache/${project}/.env); cleanup runs in that project.`);
if (enabled && new URL(base).port === '8180') throw new Error(`${base} uses the live port 8180; point VAULT_ENV_FILE at your dev runtime.`);
async function call(path, { cookie = '', method = 'GET', body, origin = base } = {}) {
  const response = await fetch(base + path, { method, headers: { ...(cookie ? { Cookie: cookie } : {}), ...(method !== 'GET' ? { Origin: origin, 'Content-Type': 'application/json' } : {}) }, body: body === undefined ? undefined : JSON.stringify(body), redirect: 'manual' });
  const text = await response.text();
  return { status: response.status, text, json: () => JSON.parse(text) };
}
async function signIn(email, password) {
  const response = await fetch(base + '/api/auth/sign-in/email', { method: 'POST', headers: { Origin: base, 'Content-Type': 'application/json' }, body: JSON.stringify({ email, password }) });
  assert.equal(response.status, 200, await response.clone().text());
  return response.headers.getSetCookie().map(x => x.split(';')[0]).join('; ');
}
const fixture = randomBytes(5).toString('hex'), password = randomBytes(20).toString('hex');
const roles = ['researcher', 'researcher', 'viewer'], emails = roles.map((role, i) => `lab${i}.${fixture}@smoke.vault.local`);
const SPEC = { v: 1, direction: 'long', entry: { all: [{ lhs: { s: 'rsi', p: { length: 2 } }, op: 'crosses_below', rhs: { value: 10 } }] }, exit: { any: [{ lhs: { s: 'bars_held' }, op: '>=', rhs: { value: 10 } }] } };
const S = '/api/app/lab/strategies';

await test('saved lab strategies: ownership, versions, review and frozen YT snapshots', { skip: enabled ? false : 'set VAULT_ENV_FILE and VAULT_COMPOSE_PROJECT (isolated dev stack) to run' }, async t => {
  let admin, owner, other, viewer, id;
  try {
    admin = await signIn(env.VAULT_ADMIN_EMAIL, env.VAULT_ADMIN_PASSWORD);
    for (const [i, role] of roles.entries()) assert.equal((await call('/api/app/admin/users', { cookie: admin, method: 'POST', body: { name: `Lab ${i}`, email: emails[i], password, role } })).status, 201);
    [owner, other, viewer] = await Promise.all(emails.map(email => signIn(email, password)));
    await t.test('create needs sign-in, write role and a trusted origin', async () => {
      const draft = { name: ' RSI dip ', spec: SPEC, market: 'in', instrument: 'NIFTY 50' };
      assert.equal((await call(S, { method: 'POST', body: draft })).status, 401);
      assert.equal((await call(S, { cookie: viewer, method: 'POST', body: draft })).status, 403);
      assert.equal((await call(S, { cookie: owner, method: 'POST', body: draft, origin: 'https://untrusted.example' })).status, 403);
      assert.equal((await call(S, { cookie: owner, method: 'POST', body: { ...draft, spec: { ...SPEC, pad: 'x'.repeat(8200) } } })).status, 400);
      const created = await call(S, { cookie: owner, method: 'POST', body: draft });
      assert.equal(created.status, 201, created.text);
      const json = created.json(); id = json.id;
      assert.deepEqual({ name: json.name, status: json.status, version: json.version, publishable: json.publishable, spec: json.spec }, { name: 'RSI dip', status: 'draft', version: 1, publishable: true, spec: SPEC });
    });
    await t.test('another user and a viewer cannot see the draft', async () => {
      assert.equal((await call(`${S}/${id}`, { cookie: other })).status, 404);
      assert.equal((await call(`${S}/${id}`, { cookie: viewer })).status, 404);
      assert.equal((await call(`${S}/${id}`, { cookie: other, method: 'PUT', body: { version: 1, name: 'Hijack', spec: SPEC, market: 'in', instrument: 'NIFTY 50' } })).status, 404);
      assert.deepEqual((await call(S, { cookie: other })).json(), []);
      assert.equal((await call(S + '?status=draft', { cookie: owner })).json()[0].id, id);
    });
    await t.test('a stale version is refused instead of overwriting', async () => {
      const first = await call(`${S}/${id}`, { cookie: owner, method: 'PUT', body: { version: 1, name: 'RSI dip v2', spec: SPEC, market: 'in', instrument: 'NIFTY 50' } });
      assert.equal(first.status, 200, first.text); assert.equal(first.json().version, 2);
      const stale = await call(`${S}/${id}`, { cookie: owner, method: 'PUT', body: { version: 1, name: 'Lost update', spec: SPEC, market: 'in', instrument: 'NIFTY 50' } });
      assert.equal(stale.status, 409);
      assert.equal((await call(`${S}/${id}`, { cookie: owner })).json().name, 'RSI dip v2');
    });
    await t.test('submit, withdraw, reject with a note, edit, resubmit', async () => {
      assert.equal((await call(`${S}/${id}/submit`, { cookie: owner, method: 'POST', body: { version: 2 } })).json().status, 'submitted');
      assert.equal((await call(`${S}/${id}`, { cookie: owner, method: 'PUT', body: { version: 2, name: 'x', spec: SPEC, market: 'in', instrument: 'NIFTY 50' } })).status, 409);
      assert.equal((await call(`${S}/${id}/withdraw`, { cookie: owner, method: 'POST', body: { version: 2 } })).json().status, 'draft');
      assert.equal((await call(`${S}/${id}/submit`, { cookie: owner, method: 'POST', body: { version: 2 } })).status, 200);
      assert.equal((await call(`${S}/review`, { cookie: owner })).status, 403);
      assert.ok((await call(`${S}/review`, { cookie: admin })).json().some(s => s.id === id));
      assert.equal((await call(`${S}/${id}/reject`, { cookie: admin, method: 'POST', body: { version: 2 } })).status, 400);
      const rejected = await call(`${S}/${id}/reject`, { cookie: admin, method: 'POST', body: { version: 2, note: 'Add a time exit.' } });
      assert.deepEqual([rejected.json().status, rejected.json().reviewNote], ['rejected', 'Add a time exit.']);
      const edited = await call(`${S}/${id}`, { cookie: owner, method: 'PUT', body: { version: 2, name: 'RSI dip v3', spec: SPEC, market: 'in', instrument: 'NIFTY 50' } });
      assert.deepEqual([edited.json().status, edited.json().version], ['draft', 3]);
      assert.equal((await call(`${S}/${id}/submit`, { cookie: owner, method: 'POST', body: { version: 3 } })).status, 200);
    });
    await t.test('publish freezes the reviewed version for every signed-in role', async () => {
      assert.equal((await call(`${S}/${id}/publish`, { cookie: owner, method: 'POST', body: { version: 3 } })).status, 403);
      assert.equal((await call(`${S}/${id}/publish`, { cookie: admin, method: 'POST', body: { version: 2 } })).status, 409);
      const published = await call(`${S}/${id}/publish`, { cookie: admin, method: 'POST', body: { version: 3 } });
      assert.equal(published.json().status, 'published', published.text);
      for (const [method, path, body] of [['PUT', `${S}/${id}`, { version: 3, name: 'x', spec: SPEC, market: 'in', instrument: 'NIFTY 50' }], ['DELETE', `${S}/${id}?version=3`], ['POST', `${S}/${id}/withdraw`, { version: 3 }]])
        assert.equal((await call(path, { cookie: owner, method, body })).status, 409);
      const snapshot = (await call(`${S}/published`, { cookie: viewer })).json().find(s => s.id === id);
      assert.deepEqual({ version: snapshot.version, name: snapshot.name, spec: snapshot.spec, market: snapshot.market }, { version: 3, name: 'RSI dip v3', spec: SPEC, market: 'in' });
      assert.equal((await call(`${S}/${id}`, { cookie: viewer })).status, 200);
      const copy = await call(`${S}/${id}/duplicate`, { cookie: other, method: 'POST' });
      assert.equal(copy.status, 201);
      assert.deepEqual({ name: copy.json().name, status: copy.json().status, version: copy.json().version }, { name: 'Copy of RSI dip v3', status: 'draft', version: 1 });
      const copyPath = `${S}/${copy.json().id}`;
      assert.equal((await call(copyPath, { cookie: other, method: 'DELETE' })).status, 400);
      assert.equal((await call(copyPath + '?version=9', { cookie: other, method: 'DELETE' })).status, 409);
      assert.equal((await call(copyPath + '?version=1', { cookie: other, method: 'DELETE' })).status, 204);
    });
    await t.test('crypto strategies save but cannot be submitted', async () => {
      const crypto = (await call(S, { cookie: owner, method: 'POST', body: { name: 'BTC dip', spec: SPEC, market: 'crypto', instrument: 'BTCUSD' } })).json();
      assert.equal(crypto.publishable, false);
      const refused = await call(`${S}/${crypto.id}/submit`, { cookie: owner, method: 'POST', body: { version: 1 } });
      assert.equal(refused.status, 400); assert.match(refused.json().error, /provisional/);
      // A draft cannot be published directly (status guard, 409); submitting one is where the provisional guard answers 400.
      const gl = (await call(S, { cookie: owner, method: 'POST', body: { name: 'FTSE dip', spec: SPEC, market: 'gl', instrument: 'UKX' } })).json();
      for (const strategy of [crypto, gl]) assert.equal((await call(`${S}/${strategy.id}/publish`, { cookie: admin, method: 'POST', body: { version: 1 } })).status, 409);
      assert.equal((await call(`${S}/${gl.id}/submit`, { cookie: owner, method: 'POST', body: { version: 1 } })).status, 400);
    });
    await t.test('two simultaneous saves of one version: one wins, one conflicts', async () => {
      const draft = (await call(S, { cookie: owner, method: 'POST', body: { name: 'Race', spec: SPEC, market: 'us', instrument: 'SPY' } })).json();
      const save = name => call(`${S}/${draft.id}`, { cookie: owner, method: 'PUT', body: { version: 1, name, spec: SPEC, market: 'us', instrument: 'SPY' } });
      const results = await Promise.all([save('A'), save('B')]);
      assert.deepEqual(results.map(r => r.status).sort(), [200, 409]);
      const saved = (await call(`${S}/${draft.id}`, { cookie: owner })).json();
      assert.deepEqual([saved.version, saved.name], [2, results.find(r => r.status === 200).json().name]);
    });
  } finally {
    // Only this run's fixture users and their strategies are removed.
    const users = `SELECT id FROM "user" WHERE email IN ('${emails.join("','")}')`;
    const sql = `DELETE FROM vault_audit WHERE actor_id IN (${users}) OR entity IN (${users}) OR entity IN (SELECT 'lab_strategy:'||id FROM lab_strategy WHERE owner IN (${users})); DELETE FROM lab_strategy_version WHERE strategy_id IN (SELECT id FROM lab_strategy WHERE owner IN (${users})); DELETE FROM "user" WHERE email IN ('${emails.join("','")}');`;
    try { execFileSync('docker', ['compose', '-p', project, '--project-directory', appDirectory, '--env-file', envFile, 'exec', '-T', 'postgres', 'psql', '-U', 'vault', '-d', 'vault', '-v', 'ON_ERROR_STOP=1', '-c', sql], { stdio: ['ignore', 'pipe', 'pipe'] }); }
    catch (error) { console.error(`cleanup failed; remove ${emails.join(', ')} by hand:`, error.message); } // keep the original assertion failure visible
  }
});
