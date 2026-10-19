// Manual gate: saved custom indicators against an ISOLATED dev stack (docs/DEVELOPMENT.md "Two developers on one machine").
// VAULT_ENV_FILE=<dev runtime>/.env VAULT_COMPOSE_PROJECT=strategy-vault-dev-<name> node --test tests/lab-indicators.integration.test.mjs
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
const roles = ['researcher', 'researcher', 'viewer'], emails = roles.map((role, i) => `ind${i}.${fixture}@smoke.vault.local`);
const FORMULA = 'Fast = Param("Fast", 12, 2, 50, 1);\nPlot(EMA(C, Fast) - EMA(C, 26), "Osc");';
const BLOCKS = { v: 1, nodes: [{ id: 'n1', kind: 'ema', x: 40, y: 80 }], links: [] };
const I = '/api/app/lab/indicators';

await test('saved custom indicators: ownership, unique names, versions, blocks', { skip: enabled ? false : 'set VAULT_ENV_FILE and VAULT_COMPOSE_PROJECT (isolated dev stack) to run' }, async t => {
  let admin, owner, other, viewer, id;
  try {
    admin = await signIn(env.VAULT_ADMIN_EMAIL, env.VAULT_ADMIN_PASSWORD);
    for (const [i, role] of roles.entries()) assert.equal((await call('/api/app/admin/users', { cookie: admin, method: 'POST', body: { name: `Ind ${i}`, email: emails[i], password, role } })).status, 201);
    [owner, other, viewer] = await Promise.all(emails.map(email => signIn(email, password)));
    await t.test('create needs sign-in, write role and a trusted origin', async () => {
      const draft = { name: ' My Momentum ', formula: FORMULA, blocks: BLOCKS };
      assert.equal((await call(I, { method: 'POST', body: draft })).status, 401);
      assert.equal((await call(I, { cookie: viewer, method: 'POST', body: draft })).status, 403);
      assert.equal((await call(I, { cookie: owner, method: 'POST', body: draft, origin: 'https://untrusted.example' })).status, 403);
      assert.equal((await call(I, { cookie: owner, method: 'POST', body: { ...draft, formula: 'x'.repeat(4001) } })).status, 400);
      const created = await call(I, { cookie: owner, method: 'POST', body: draft });
      assert.equal(created.status, 201, created.text);
      const json = created.json(); id = json.id;
      assert.deepEqual({ name: json.name, formula: json.formula, blocks: json.blocks, version: json.version, owner: json.owner }, { name: 'My Momentum', formula: FORMULA, blocks: BLOCKS, version: 1, owner: undefined });
      assert.ok(json.createdAt && json.updatedAt);
    });
    await t.test('names are unique per owner regardless of case, but not across owners', async () => {
      const duplicate = await call(I, { cookie: owner, method: 'POST', body: { name: 'MY MOMENTUM', formula: FORMULA } });
      assert.equal(duplicate.status, 409, duplicate.text); assert.match(duplicate.json().error, /already have an indicator/);
      const theirs = await call(I, { cookie: other, method: 'POST', body: { name: 'My Momentum', formula: FORMULA } });
      assert.equal(theirs.status, 201, theirs.text);
      assert.equal(theirs.json().blocks, null);
      const second = (await call(I, { cookie: owner, method: 'POST', body: { name: 'Second', formula: 'Plot(C, "C");' } })).json();
      assert.equal((await call(`${I}/${second.id}`, { cookie: owner, method: 'PUT', body: { version: 1, name: 'my momentum', formula: FORMULA } })).status, 409);
      assert.equal((await call(`${I}/${second.id}?version=1`, { cookie: owner, method: 'DELETE' })).status, 204);
    });
    await t.test('another user and a viewer cannot see or change it', async () => {
      assert.deepEqual((await call(I, { cookie: owner })).json().map(x => x.id), [id]);
      assert.ok(!(await call(I, { cookie: other })).json().some(x => x.id === id));
      assert.deepEqual((await call(I, { cookie: viewer })).json(), []);
      assert.equal((await call(`${I}/${id}`, { cookie: other, method: 'PUT', body: { version: 1, name: 'Hijack', formula: FORMULA } })).status, 404);
      assert.equal((await call(`${I}/${id}?version=1`, { cookie: admin, method: 'DELETE' })).status, 404);
    });
    await t.test('a stale version is refused; a text edit without blocks clears them', async () => {
      const first = await call(`${I}/${id}`, { cookie: owner, method: 'PUT', body: { version: 1, name: 'my momentum', formula: 'Plot(C, "Close");' } });
      assert.equal(first.status, 200, first.text);
      assert.deepEqual([first.json().version, first.json().name, first.json().blocks], [2, 'my momentum', null]);
      assert.equal((await call(`${I}/${id}`, { cookie: owner, method: 'PUT', body: { version: 1, name: 'Lost', formula: FORMULA } })).status, 409);
      assert.equal((await call(I, { cookie: owner })).json()[0].name, 'my momentum');
    });
    await t.test('two simultaneous saves of one version: one wins, one conflicts', async () => {
      const save = name => call(`${I}/${id}`, { cookie: owner, method: 'PUT', body: { version: 2, name, formula: FORMULA } });
      const results = await Promise.all([save('A'), save('B')]);
      assert.deepEqual(results.map(r => r.status).sort(), [200, 409]);
    });
    await t.test('delete needs the current version', async () => {
      assert.equal((await call(`${I}/${id}`, { cookie: owner, method: 'DELETE' })).status, 400);
      assert.equal((await call(`${I}/${id}?version=2`, { cookie: owner, method: 'DELETE' })).status, 409);
      assert.equal((await call(`${I}/${id}?version=3`, { cookie: owner, method: 'DELETE' })).status, 204);
      assert.deepEqual((await call(I, { cookie: owner })).json(), []);
    });
  } finally {
    // Only this run's fixture users and their indicators are removed (indicators cascade with the user).
    const users = `SELECT id FROM "user" WHERE email IN ('${emails.join("','")}')`;
    const sql = `DELETE FROM vault_audit WHERE actor_id IN (${users}) OR entity IN (${users}) OR entity IN (SELECT 'lab_indicator:'||id FROM lab_indicator WHERE owner IN (${users})); DELETE FROM "user" WHERE email IN ('${emails.join("','")}');`;
    try { execFileSync('docker', ['compose', '-p', project, '--project-directory', appDirectory, '--env-file', envFile, 'exec', '-T', 'postgres', 'psql', '-U', 'vault', '-d', 'vault', '-v', 'ON_ERROR_STOP=1', '-c', sql], { stdio: ['ignore', 'pipe', 'pipe'] }); }
    catch (error) { console.error(`cleanup failed; remove ${emails.join(', ')} by hand:`, error.message); } // keep the original assertion failure visible
  }
});
