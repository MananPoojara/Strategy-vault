// Research YTS API against a real, disposable Postgres (RESEARCH_YTS_TEST_DATABASE_URL). Never point it at the app database:
// it works inside a schema it creates (with a minimal "user" table) and drops that schema afterwards.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { Hono } from 'hono';
import pg from 'pg';
import { registerResearchYts, seedResearchYtsAdmin, validateIssue, validateEdit, cleanLink, cleanUsername, cleanFileName, cleanContentType, hashPassword, checkPassword, YTS_UPLOAD } from '../auth/src/research-yts.mjs';

test('issue validation keeps the collection, strategy and fields the QA typed', () => {
  const ok = validateIssue({ collection: 'yt_bb', strategy_id: 'ytbb_plain', category: 'wrong_trades', title: '  Exit price wrong ', body: 'Line 1\r\nLine 2', period_from: '2020-01-01', period_to: '2020-12-31', page_link: '#/s/ytbb_plain?tk=basket%3Anse500%3ARELIANCE' });
  assert.equal(ok.title, 'Exit price wrong');
  assert.equal(ok.body, 'Line 1\nLine 2');
  assert.equal(ok.severity, 'normal');
  assert.throws(() => validateIssue({ collection: 'kora', category: 'data', title: 'abc', body: 'x' }), /collection/);
  assert.throws(() => validateIssue({ collection: 'yt_bb', category: 'nope', title: 'abc', body: 'x' }), /category/);
  assert.throws(() => validateIssue({ collection: 'yt_bb', category: 'data', title: 'abc', body: 'x', period_from: '2021-01-01', period_to: '2020-01-01' }), /starts after/);
  assert.throws(() => cleanLink('https://evil.example/'), /inside the vault/);
  assert.equal(cleanUsername('  Priya.S '), 'priya.s');
  assert.throws(() => cleanUsername('a b'), /Usernames/);
});

test('a general issue lists its strategies and their collections; a normal issue has none', () => {
  const g = validateIssue({ collection: 'general', strategy_id: 'ignored', category: 'page', title: 'Labels overlap', body: 'x',
    strategies: [{ collection: 'yt_bb', id: 'ytbb_plain' }, { collection: 'yt_d7', id: 'ytd7_long' }, { collection: 'yt_bb', id: 'ytbb_plain' }] });
  assert.deepEqual([g.collection, g.strategy_id, g.strategies, g.collections], ['general', '', ['ytbb_plain', 'ytd7_long'], ['yt_bb', 'yt_d7']]);
  assert.deepEqual(validateIssue({ collection: 'general', category: 'page', title: 'All pages', body: 'x' }).strategies, [], 'no strategies: all YT strategies');
  assert.deepEqual(validateIssue({ collection: 'yt_bb', category: 'data', title: 'abc', body: 'x', strategies: [{ collection: 'yt_d7', id: 'ytd7_long' }] }).strategies, [], 'ignored on a normal issue');
  assert.throws(() => validateIssue({ collection: 'general', category: 'page', title: 'abc', body: 'x', strategies: [{ collection: 'kora', id: 'x' }] }), /strategies/);
  assert.throws(() => validateIssue({ collection: 'general', category: 'page', title: 'abc', body: 'x', strategies: 'ytbb_plain' }), /strategies/);
  assert.deepEqual(validateEdit({ strategies: [] }), { strategies: [], collections: [] });
});

test('file names and types are cleaned; only the upload route takes a large body', () => {
  assert.equal(cleanFileName('  ../trade sheet\\v2.xlsx '), '.._trade sheet_v2.xlsx');
  assert.throws(() => cleanFileName(''), /name/);
  assert.throws(() => cleanFileName('a<script>.png'), /name/);
  assert.equal(cleanContentType('image/PNG; charset=x'), 'image/png');
  assert.equal(cleanContentType('not a type'), 'application/octet-stream');
  assert.ok(YTS_UPLOAD.test('/api/app/research-yts/issues/12/files'));
  assert.ok(!YTS_UPLOAD.test('/api/app/research-yts/issues/12/files/3'));
  assert.ok(!YTS_UPLOAD.test('/api/app/notes'));
});

test('passwords are stored as salted scrypt hashes and checked exactly', async () => {
  const a = await hashPassword('admin@123'), b = await hashPassword('admin@123');
  assert.match(a, /^scrypt\$16384\$8\$1\$/);
  assert.notEqual(a, b, 'a new salt each time');
  assert.equal(await checkPassword('admin@123', a), true);
  assert.equal(await checkPassword('admin@124', a), false);
  assert.equal(await checkPassword('admin@123', 'plain'), false);
});

const url = process.env.RESEARCH_YTS_TEST_DATABASE_URL;
test('QAs sign in separately, see their own issues, and the admin manages accounts and sees all', { skip: !url && 'set RESEARCH_YTS_TEST_DATABASE_URL to a disposable database' }, async () => {
  const schema = `yts_test_${process.pid}`;
  const setup = new pg.Pool({ connectionString: url, max: 1 });
  await setup.query(`CREATE SCHEMA ${schema}`);
  await setup.end();
  const pool = new pg.Pool({ connectionString: url, max: 4, options: `-c search_path=${schema}` });
  try {
    await pool.query('CREATE TABLE "user"(id text PRIMARY KEY, name text, role text)');
    await pool.query(`INSERT INTO "user" VALUES ('shared','admin','admin')`);
    for (const f of ['001-app.sql', '003-research-yts.sql']) await pool.query(await readFile(new URL(`../db/${f}`, import.meta.url), 'utf8'));
    await pool.query(await readFile(new URL('../db/003-research-yts.sql', import.meta.url), 'utf8')); // safe to run twice
    assert.equal(await seedResearchYtsAdmin(pool, { RESEARCH_YTS_ADMIN_USERNAME: 'admin', RESEARCH_YTS_ADMIN_PASSWORD: 'admin@123' }), true);
    assert.equal(await seedResearchYtsAdmin(pool, { RESEARCH_YTS_ADMIN_USERNAME: 'admin', RESEARCH_YTS_ADMIN_PASSWORD: 'changed!' }), false, 'an existing admin is never overwritten');

    let clock = Date.now();
    const app = new Hono();
    app.use('*', async (c, next) => { c.set('user', { id: 'shared', name: 'admin', role: 'admin' }); await next(); }); // everyone shares one app login
    registerResearchYts(app, { pool, secret: 'test-secret-test-secret-test-secret', secure: false, now: () => clock });
    const jar = () => ({ cookie: '' });
    const call = async (who, method, path, body) => {
      const r = await app.request(`/api/app/research-yts${path}`, { method, headers: { 'content-type': 'application/json', ...(who.cookie ? { cookie: who.cookie } : {}) }, body: body === undefined ? undefined : JSON.stringify(body) });
      const set = r.headers.get('set-cookie');
      if (set) who.cookie = /yts_qa=([^;]*)/.test(set) && !/Max-Age=0/i.test(set) ? `yts_qa=${/yts_qa=([^;]*)/.exec(set)[1]}` : '';
      return { status: r.status, json: await r.json() };
    };
    const admin = jar(), priya = jar(), ravi = jar(), nobody = jar();
    const upload = async (who, id, name, type, bytes) => {
      const r = await app.request(`/api/app/research-yts/issues/${id}/files?name=${encodeURIComponent(name)}`, { method: 'POST', headers: { 'content-type': type, cookie: who.cookie }, body: bytes });
      return { status: r.status, json: await r.json() };
    };
    const fetchFile = (who, id, file, q = '') => app.request(`/api/app/research-yts/issues/${id}/files/${file}${q}`, { headers: { cookie: who.cookie } });

    assert.equal((await call(nobody, 'GET', '/me')).json.qa, null);
    assert.equal((await call(nobody, 'GET', '/issues')).status, 401, 'issues are locked until a QA signs in');
    assert.equal((await call(nobody, 'POST', '/issues', { collection: 'yt_bb', category: 'data', title: 'abc', body: 'x' })).status, 401);
    assert.equal((await call(admin, 'POST', '/login', { username: 'admin', password: 'wrong' })).status, 401);
    const signedIn = await call(admin, 'POST', '/login', { username: 'Admin', password: 'admin@123' });
    assert.equal(signedIn.status, 200);
    assert.deepEqual(signedIn.json.qa, { id: signedIn.json.qa.id, username: 'admin', name: 'Admin', admin: true });

    // the admin adds two QAs
    const p = await call(admin, 'POST', '/qa-users', { username: 'priya', name: 'Priya Shah', password: 'priya-pass' });
    const r = await call(admin, 'POST', '/qa-users', { username: 'ravi', name: 'Ravi Patel', password: 'ravi-pass' });
    assert.equal(p.status, 201);
    assert.equal((await call(admin, 'POST', '/qa-users', { username: 'priya', name: 'Other', password: 'xxxxxx' })).status, 409);
    assert.equal((await call(admin, 'POST', '/qa-users', { username: 'x', name: 'Short', password: '123' })).status, 400);
    assert.equal((await call(priya, 'POST', '/login', { username: 'priya', password: 'priya-pass' })).status, 200);
    assert.equal((await call(ravi, 'POST', '/login', { username: 'ravi', password: 'ravi-pass' })).status, 200);
    assert.equal((await call(priya, 'GET', '/qa-users')).status, 403, 'a QA cannot manage accounts');

    // each QA files issues under their own name, whatever the shared app account is called
    const mine = await call(priya, 'POST', '/issues', { collection: 'yt_bb', strategy_id: 'ytbb_plain', category: 'wrong_trades', severity: 'high', title: 'Exit at wrong price', body: 'Trade 3 exits at the open.', reporter_name: 'Someone else' });
    assert.equal(mine.status, 201);
    assert.equal(mine.json.reporter_name, 'Priya Shah', 'the name comes from the Research YTS account, never the request');
    const his = await call(ravi, 'POST', '/issues', { collection: 'yt_gann', category: 'page', title: 'Card text overlaps', body: 'On a phone.' });
    assert.equal(his.json.reporter_name, 'Ravi Patel');

    // a QA sees only their own issues; the admin sees both
    assert.deepEqual((await call(priya, 'GET', '/issues')).json.items.map(i => i.title), ['Exit at wrong price']);
    assert.equal((await call(priya, 'GET', `/issues/${his.json.id}`)).status, 404);
    assert.equal((await call(priya, 'POST', `/issues/${his.json.id}/comments`, { body: 'hi' })).status, 404);
    assert.deepEqual((await call(priya, 'GET', '/summary')).json.totals, { open: 1, in_progress: 0, fixed: 0, not_issue: 0 });
    assert.equal((await call(admin, 'GET', '/issues')).json.total, 2);
    assert.equal((await call(admin, 'GET', `/issues?reporter=${his.json.reporter_id}`)).json.items[0].title, 'Card text overlaps');
    assert.equal((await call(priya, 'GET', `/issues?reporter=${his.json.reporter_id}`)).json.total, 1, 'the reporter filter cannot widen a QA\'s view');

    // photos and files: added by the reporter, seen only by those who see the issue, images shown and other files downloaded
    const png = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 1, 2, 3]);
    const shot = await upload(priya, mine.json.id, 'exit trade.png', 'image/png', png);
    assert.equal(shot.status, 201);
    assert.deepEqual(shot.json.files.map(f => [f.name, f.content_type, f.size, f.image, f.uploader_name, f.can_delete]), [['exit trade.png', 'image/png', 11, true, 'Priya Shah', true]]);
    assert.equal(shot.json.issue.files, 1);
    const page = await upload(priya, mine.json.id, 'page.html', 'text/html', Buffer.from('<script>alert(1)</script>'));
    const img = await fetchFile(priya, mine.json.id, shot.json.files[0].id);
    assert.equal(img.headers.get('content-type'), 'image/png');
    assert.match(img.headers.get('content-disposition'), /^inline; filename="exit trade.png"/);
    assert.deepEqual(Buffer.from(await img.arrayBuffer()), png);
    const html = await fetchFile(priya, mine.json.id, page.json.files[1].id);
    assert.equal(html.headers.get('content-type'), 'application/octet-stream', 'a non-image file is never opened in the vault');
    assert.match(html.headers.get('content-disposition'), /^attachment;/);
    assert.equal((await fetchFile(ravi, mine.json.id, shot.json.files[0].id)).status, 404, 'another QA cannot fetch the file');
    assert.equal((await upload(ravi, mine.json.id, 'x.png', 'image/png', png)).status, 404);
    assert.equal((await upload(priya, mine.json.id, 'empty.txt', 'text/plain', Buffer.alloc(0))).status, 400);
    const removed = await call(priya, 'DELETE', `/issues/${mine.json.id}/files/${page.json.files[1].id}`, {});
    assert.deepEqual(removed.json.files.map(f => f.name), ['exit trade.png']);
    assert.equal((await fetchFile(priya, mine.json.id, page.json.files[1].id)).status, 404);

    // a general issue about strategies in two collections shows under both, and under each strategy
    const gen = await call(priya, 'POST', '/issues', { collection: 'general', category: 'page', title: 'Labels overlap', body: 'On every page.',
      strategies: [{ collection: 'yt_bb', id: 'ytbb_athc' }, { collection: 'yt_d7', id: 'ytd7_long' }] });
    assert.equal(gen.status, 201);
    assert.deepEqual([gen.json.collection, gen.json.strategies, gen.json.collections], ['general', ['ytbb_athc', 'ytd7_long'], ['yt_bb', 'yt_d7']]);
    assert.deepEqual((await call(priya, 'GET', '/issues?collection=yt_bb')).json.items.map(i => i.title).sort(), ['Exit at wrong price', 'Labels overlap']);
    assert.deepEqual((await call(priya, 'GET', '/issues?collection=yt_d7')).json.items.map(i => i.title), ['Labels overlap']);
    assert.deepEqual((await call(priya, 'GET', '/issues?collection=general')).json.items.map(i => i.title), ['Labels overlap']);
    assert.deepEqual((await call(priya, 'GET', '/issues?strategy=ytd7_long')).json.items.map(i => i.title), ['Labels overlap']);
    const sum = (await call(priya, 'GET', '/summary')).json;
    assert.equal(sum.totals.open, 2, 'a general issue counts once');
    assert.deepEqual([sum.collections.general.open, sum.collections.yt_bb.open, sum.collections.yt_d7.open, sum.strategies.ytd7_long.open], [1, 2, 1, 1]);
    assert.deepEqual((await call(priya, 'PATCH', `/issues/${gen.json.id}`, { strategies: [{ collection: 'yt_d7', id: 'ytd7_long' }] })).json.collections, ['yt_d7']);
    assert.equal((await call(priya, 'PATCH', `/issues/${mine.json.id}`, { strategies: [] })).status, 400, 'only a general issue lists strategies');
    assert.ok((await call(priya, 'DELETE', `/issues/${gen.json.id}`, { reason: 'Test issue' })).json.deleted_at);

    // the admin fixes it, the QA reopens it; both are in the history under their own names
    assert.equal((await call(admin, 'PUT', `/issues/${mine.json.id}/status`, { status: 'fixed', note: 'Fixed in the 13 Nov build' })).json.status, 'fixed');
    assert.equal((await call(priya, 'PUT', `/issues/${mine.json.id}/status`, { status: 'open' })).json.status, 'open');
    assert.equal((await call(priya, 'PATCH', `/issues/${mine.json.id}`, { title: 'Exit at the wrong price' })).json.title, 'Exit at the wrong price');
    assert.equal((await call(admin, 'POST', `/issues/${mine.json.id}/comments`, { body: 'Looking at it.' })).status, 201);
    const detail = (await call(priya, 'GET', `/issues/${mine.json.id}`)).json;
    assert.deepEqual(detail.files.map(f => f.name), ['exit trade.png']);
    assert.equal(detail.files[0].can_delete, true);
    assert.deepEqual(detail.events.map(e => [e.action, e.actor_name]), [['created', 'Priya Shah'], ['file', 'Priya Shah'], ['file', 'Priya Shah'], ['file', 'Priya Shah'], ['status', 'Admin'], ['status', 'Priya Shah'], ['edited', 'Priya Shah'], ['comment', 'Admin']]);

    // password changes and deactivation sign the person out everywhere; history stays
    assert.equal((await call(admin, 'POST', `/qa-users/${his.json.reporter_id}/password`, { password: 'ravi-new-pass' })).status, 200);
    assert.equal((await call(ravi, 'GET', '/issues')).status, 401, 'old session ended by the reset');
    assert.equal((await call(ravi, 'POST', '/login', { username: 'ravi', password: 'ravi-new-pass' })).status, 200);
    assert.equal((await call(admin, 'POST', `/qa-users/${his.json.reporter_id}/deactivate`)).json.deactivated_at !== null, true);
    assert.equal((await call(ravi, 'GET', '/issues')).status, 401);
    assert.match((await call(ravi, 'POST', '/login', { username: 'ravi', password: 'ravi-new-pass' })).json.error, /deactivated/);
    assert.equal((await call(admin, 'GET', `/issues/${his.json.id}`)).json.issue.reporter_name, 'Ravi Patel', 'a deactivated QA keeps their issues');
    assert.equal((await call(admin, 'POST', `/qa-users/${signedIn.json.qa.id}/deactivate`)).status, 400, 'the admin cannot deactivate themself');
    assert.equal((await call(admin, 'POST', `/qa-users/${his.json.reporter_id}/reactivate`)).json.deactivated_at, null);

    // own password change keeps this browser signed in
    assert.equal((await call(priya, 'POST', '/password', { current: 'wrong', next: 'priya-pass-2' })).status, 400);
    assert.equal((await call(priya, 'POST', '/password', { current: 'priya-pass', next: 'priya-pass-2' })).status, 200);
    assert.equal((await call(priya, 'GET', '/issues')).status, 200);

    // five wrong passwords lock that username for a minute
    for (let i = 0; i < 5; i++) assert.equal((await call(nobody, 'POST', '/login', { username: 'priya', password: 'nope' })).status, 401);
    assert.equal((await call(nobody, 'POST', '/login', { username: 'priya', password: 'priya-pass-2' })).status, 429);
    clock += 61_000;
    assert.equal((await call(nobody, 'POST', '/login', { username: 'priya', password: 'priya-pass-2' })).status, 200);

    // QAs edit/delete only their own; deleting needs a reason and keeps the row
    assert.equal((await call(priya, 'DELETE', `/issues/${mine.json.id}`, {})).status, 400);
    assert.ok((await call(priya, 'DELETE', `/issues/${mine.json.id}`, { reason: 'Reported twice' })).json.deleted_at);
    assert.equal((await call(admin, 'GET', '/issues')).json.total, 1);
    assert.equal((await call(priya, 'POST', '/logout')).status, 200);
    assert.equal((await call(priya, 'GET', '/issues')).status, 401);
    const accounts = (await call(admin, 'GET', '/qa-users')).json;
    assert.deepEqual(accounts.map(a => [a.username, a.admin, a.issues]), [['admin', true, 0], ['priya', false, 0], ['ravi', false, 1]]);
    assert.ok(!JSON.stringify(accounts).includes('scrypt'), 'password hashes never leave the server');
  } finally {
    await pool.query(`DROP SCHEMA ${schema} CASCADE`).catch(() => {});
    await pool.end();
  }
});
