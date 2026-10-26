// Research portal API against a throwaway PostgreSQL (never the live database).
// Run: RESEARCH_TEST_DATABASE_URL=postgresql://vault:...@127.0.0.1:19950/vault node --test tests/research-portal.test.mjs
import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, writeFile, utimes } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createRequire } from 'node:module';
const require = createRequire(new URL('../auth/package.json', import.meta.url));
const { Hono } = await import(require.resolve('hono'));
const pg = (await import(require.resolve('pg'))).default;
const { createResearchStore, registerResearch, pageParams, cleanText } = await import('../auth/src/research.mjs');

const url = process.env.RESEARCH_TEST_DATABASE_URL;
const users = {
  asha: { id: 'u-asha', name: 'Asha', role: 'researcher' }, ravi: { id: 'u-ravi', name: 'Ravi', role: 'researcher' },
  boss: { id: 'u-boss', name: 'Boss', role: 'admin' }, vera: { id: 'u-vera', name: 'Vera', role: 'viewer' },
};
const strategy = (i, site = 'held') => ({ key: `bk${i % 2}::bk${i % 2}:s${i}`, author: i % 3 ? 'Larry Connors' : 'Joe Ross', author_id: i % 3 ? 'connors' : 'ross',
  book: `Book ${i % 2}`, book_id: `bk${i % 2}`, name: `Strategy ${i}`, site, reason: 'Needs intraday data', rules: [{ kind: 'Entry', text: `Buy rule ${i} (p.${i})` }],
  variant: `v${i}`, variants: 1, url: `http://192.168.4.30:8180/#/s/v${i}` });
const fixture = {
  as_of: '2026-10-25T03:20:36+00:00', totals: { strategies: 60, verified: 20 }, reasons: [], authors: [], books: [], todo_authors: [],
  strategies: Array.from({ length: 60 }, (_, i) => strategy(i, i < 20 ? 'verified' : 'held')),
  events: [{ key: 'ev1', at: '2026-10-21T19:07:00+05:30', label: '21 Oct 19:07', who: 'Platform', work: 'Deploy: count goes up', count: 147, proof: 'snap', source: 'work_timeline' }],
  history: [], excel_reviews: [{ key: 'bk1::bk1:s1', verdict: 'Mismatch', reviewer: 'Meera', date: '2026-10-24T00:00:00', notes: 'Exit uses close, book says open (p.12)' }],
};

test('research portal API', { skip: !url && 'set RESEARCH_TEST_DATABASE_URL to a throwaway database' }, async t => {
  const pool = new pg.Pool({ connectionString: url, max: 3 });
  await pool.query('DROP TABLE IF EXISTS research_note_history, research_notes, research_verdict_history, research_verdicts, research_platform_events, vault_audit, vault_annotations, vault_note_history, vault_settings, vault_notes, "user" CASCADE');
  await pool.query('CREATE TABLE "user"(id text PRIMARY KEY, name text NOT NULL, email text, role text NOT NULL)');
  for (const u of Object.values(users)) await pool.query('INSERT INTO "user"(id,name,role) VALUES($1,$2,$3)', [u.id, u.name, u.role]);
  for (const f of ['001-app.sql', '002-research.sql']) await pool.query(await readFile(new URL(`../db/${f}`, import.meta.url), 'utf8'));
  const dir = await mkdtemp(join(tmpdir(), 'research-portal-')), file = join(dir, 'research-progress.json');
  await writeFile(file, JSON.stringify(fixture));
  const app = new Hono();
  app.use('*', async (c, next) => { c.set('user', users[c.req.header('x-user')]); await next(); });
  app.onError((e, c) => { console.error(e); return c.json({ error: 'failed' }, 500); });
  registerResearch(app, { pool, store: createResearchStore({ pool, progressPath: file }) });
  const call = async (who, path, method = 'GET', body) => {
    const r = await app.request(`/api/app/research${path}`, { method, headers: { 'x-user': who, 'content-type': 'application/json' }, body: body && JSON.stringify(body) });
    return { status: r.status, body: await r.json() };
  };
  try {
    await t.test('helpers validate paging and text', () => {
      assert.deepEqual(pageParams({ page: '3', size: '10' }), { page: 3, size: 10, offset: 20 });
      assert.throws(() => pageParams({ page: '0' })); assert.throws(() => pageParams({ size: '500' }));
      assert.equal(cleanText('  a\r\nb '), 'a\nb'); assert.throws(() => cleanText('   ')); assert.throws(() => cleanText('x\x01'));
    });
    await t.test('Excel entries and platform events are imported once', async () => {
      await call('asha', '/summary'); await call('asha', '/summary');
      const s = await call('asha', '/strategy?key=bk1::bk1:s1');
      assert.equal(s.body.verdict.verdict, 'mismatch'); assert.equal(s.body.verdict.author_name, 'Meera');
      assert.equal(s.body.notes.length, 1); assert.equal(s.body.notes[0].note_type, 'rule_error'); assert.equal(s.body.notes[0].imported, true);
      assert.equal(s.body.notes[0].can_edit, false);
      assert.equal((await pool.query('SELECT count(*)::int n FROM research_platform_events')).rows[0].n, 1);
    });
    let noteId;
    await t.test('notes: author and time come from the server', async () => {
      const r = await call('asha', '/notes', 'POST', { target_type: 'strategy', target_key: 'bk0::bk0:s2', note_type: 'rule_error', body: 'Stop is 2% in the book', author_name: 'Fake', created_at: '2000-01-01' });
      assert.equal(r.status, 201); assert.equal(r.body.author_name, 'Asha'); assert.ok(new Date(r.body.created_at).getFullYear() >= 2026);
      assert.equal(r.body.can_edit, true); noteId = r.body.id;
      assert.equal((await call('asha', '/notes', 'POST', { target_type: 'strategy', target_key: 'nope', note_type: 'question', body: 'x' })).status, 400);
      assert.equal((await call('asha', '/notes', 'POST', { target_type: 'general', note_type: 'bogus', body: 'x' })).status, 400);
      assert.equal((await call('asha', '/notes', 'POST', { target_type: 'book', target_key: 'bk1', note_type: 'data_issue', body: 'Pages 40-41 missing' })).status, 201);
      assert.equal((await call('asha', '/notes', 'POST', { target_type: 'author', target_key: 'ross', note_type: 'question', body: 'Which edition?' })).status, 201);
      assert.equal((await call('asha', '/notes', 'POST', { target_type: 'general', note_type: 'question', body: 'Weekly sync at 4' })).status, 201);
    });
    await t.test('permissions: viewers read only, only the writer edits', async () => {
      assert.equal((await call('vera', '/notes', 'POST', { target_type: 'general', note_type: 'question', body: 'x' })).status, 403);
      assert.equal((await call('vera', '/verdict', 'PUT', { key: 'bk0::bk0:s2', verdict: 'matches' })).status, 403);
      assert.equal((await call('ravi', `/notes/${noteId}`, 'PATCH', { body: 'hijack' })).status, 403);
      assert.equal((await call('boss', `/notes/${noteId}`, 'PATCH', { body: 'admin edit' })).status, 403);
      assert.equal((await call('ravi', `/notes/${noteId}`, 'DELETE', { reason: 'no' })).status, 403);
      assert.equal((await call('vera', '/notes')).status, 200);
    });
    await t.test('edits keep history', async () => {
      const r = await call('asha', `/notes/${noteId}`, 'PATCH', { body: 'Stop is 2.5% in the book (p.31)', note_type: 'rule_error' });
      assert.equal(r.status, 200); assert.equal(r.body.edits, 1); assert.equal(r.body.body, 'Stop is 2.5% in the book (p.31)');
      const h = await call('asha', `/notes/${noteId}/history`);
      assert.deepEqual(h.body.map(x => [x.action, x.body]), [['created', 'Stop is 2% in the book'], ['edited', 'Stop is 2.5% in the book (p.31)']]);
    });
    await t.test('removal is soft, needs a reason and is audited', async () => {
      assert.equal((await call('asha', `/notes/${noteId}`, 'DELETE', {})).status, 400);
      const r = await call('boss', `/notes/${noteId}`, 'DELETE', { reason: 'Duplicate of note 5' });
      assert.equal(r.status, 200); assert.ok(r.body.deleted_at); assert.equal(r.body.body, '');
      assert.equal((await call('asha', `/notes/${noteId}`, 'PATCH', { body: 'again' })).status, 409);
      assert.equal((await pool.query('SELECT count(*)::int n FROM research_notes WHERE id=$1', [noteId])).rows[0].n, 1);
      const list = await call('asha', '/notes?target_key=bk0::bk0:s2'); assert.equal(list.body.total, 0);
      const all = await call('asha', '/notes?target_key=bk0::bk0:s2&deleted=include'); assert.equal(all.body.items[0].deleted_reason, 'Duplicate of note 5');
      assert.equal((await call('asha', `/notes/${noteId}/history`)).body.at(-1).action, 'deleted');
      assert.equal((await pool.query("SELECT count(*)::int n FROM vault_audit WHERE action='research.note.deleted'")).rows[0].n, 1);
      const tl = await call('asha', '/timeline?kind=note&size=50');
      assert.ok(tl.body.items.filter(i => i.ref === noteId && i.action !== 'deleted').every(i => i.text === '' && i.removed));
    });
    await t.test('verdicts record every change', async () => {
      assert.equal((await call('asha', '/verdict', 'PUT', { key: 'bk0::bk0:s2', verdict: 'bogus' })).status, 400);
      assert.equal((await call('asha', '/verdict', 'PUT', { key: 'bk0::bk0:s2', verdict: 'unclear' })).body.previous, 'not_checked');
      assert.equal((await call('ravi', '/verdict', 'PUT', { key: 'bk0::bk0:s2', verdict: 'matches' })).body.previous, 'unclear');
      assert.equal((await call('ravi', '/verdict', 'PUT', { key: 'bk0::bk0:s2', verdict: 'matches' })).body.changed, false);
      const s = await call('asha', '/strategy?key=bk0::bk0:s2');
      assert.equal(s.body.verdict.author_name, 'Ravi'); assert.deepEqual(s.body.history.map(h => h.verdict), ['matches', 'unclear']);
    });
    await t.test('strategy list filters and paginates', async () => {
      const p1 = await call('asha', '/strategies?size=25'), p3 = await call('asha', '/strategies?size=25&page=3');
      assert.equal(p1.body.total, 60); assert.equal(p1.body.pages, 3); assert.equal(p1.body.items.length, 25); assert.equal(p3.body.items.length, 10);
      assert.equal((await call('asha', '/strategies?site=verified')).body.total, 20);
      assert.equal((await call('asha', '/strategies?author=ross')).body.total, 20);
      assert.equal((await call('asha', '/strategies?verdict=matches')).body.items[0].key, 'bk0::bk0:s2');
      assert.equal((await call('asha', '/strategies?verdict=mismatch&reviewer=Meera')).body.total, 1);
      assert.equal((await call('asha', '/strategies?q=v59')).body.total, 1); assert.equal((await call('asha', '/strategies?q=strategy ross')).body.total, 20);
      assert.equal((await call('asha', '/strategies?site=bogus')).status, 400);
    });
    await t.test('timeline merges team and platform events newest first, with filters', async () => {
      const tl = await call('asha', '/timeline?size=100');
      const times = tl.body.items.map(i => Date.parse(i.at));
      assert.deepEqual(times, [...times].sort((a, b) => b - a));
      assert.equal(tl.body.items.at(-1).kind, 'platform');
      assert.ok(tl.body.items.every(i => i.kind !== 'note' || i.target_label));
      assert.equal((await call('asha', '/timeline?kind=platform')).body.total, 1);
      assert.ok((await call('asha', '/timeline?person=u-ravi')).body.items.every(i => i.who === 'Ravi'));
      assert.equal((await call('asha', '/timeline?from=2026-10-21&to=2026-10-21')).body.total, 1);
      const p = await call('asha', '/timeline?size=5&page=2'); assert.equal(p.body.page, 2); assert.equal(p.body.items.length, Math.min(5, tl.body.total - 5));
      assert.equal((await call('asha', '/timeline?from=26-10-2026')).status, 400);
    });
    await t.test('people counts and a regenerated progress file appends new events', async () => {
      const people = (await call('asha', '/people')).body, asha = people.find(p => p.name === 'Asha');
      assert.equal(asha.notes, 4); assert.equal(asha.verdicts, 1); assert.equal(asha.edits, 1);
      assert.ok(people.find(p => p.name === 'Meera' && p.imported));
      fixture.events.push({ key: 'ev2', at: '2026-10-26T10:00:00+05:30', label: '26 Oct 10:00', who: 'Platform', work: 'Deploy', count: 600, proof: '', source: 'work_timeline' });
      await writeFile(file, JSON.stringify(fixture)); await utimes(file, new Date(), new Date(Date.now() + 5000));
      assert.equal((await call('asha', '/timeline?kind=platform')).body.total, 2);
      assert.equal((await pool.query('SELECT count(*)::int n FROM research_verdict_history WHERE import_ref IS NOT NULL')).rows[0].n, 1);
    });
  } finally { await pool.end(); }
});
