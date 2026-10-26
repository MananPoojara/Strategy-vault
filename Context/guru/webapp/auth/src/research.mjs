// Research portal API: the research team's verdicts, notes and one shared timeline.
// Identity and time come from the server (session user, database now()); request bodies never set them.
// Platform progress is read from the generated research-progress.json (make_research_progress.py); it is never written here.
import { createHash, timingSafeEqual } from 'node:crypto';
import { getSignedCookie, setSignedCookie, deleteCookie } from 'hono/cookie';
import { readFile, stat } from 'node:fs/promises';
import { roleAllows } from './contracts.mjs';

export const VERDICTS = { not_checked: 'Not checked', matches: 'Matches the book', mismatch: 'Mismatch', unclear: 'Unclear' };
export const NOTE_TYPES = { rule_error: 'Rule error', data_issue: 'Data issue', question: 'Question', verified_ok: 'Verified OK' };
const TARGETS = new Set(['strategy', 'book', 'author', 'general']);
const SITE = new Set(['verified', 'held', 'unreviewed']);
const KEY = /^[A-Za-z0-9][A-Za-z0-9_.:+-]{0,159}$/;
const EMPTY = { totals: {}, reasons: [], authors: [], books: [], todo_authors: [], strategies: [], events: [], history: [], excel_reviews: [] };

class HttpError extends Error { constructor(status, message) { super(message); this.status = status; } }
const bad = message => new HttpError(400, message);
const route = handler => async c => {
  try { return await handler(c); }
  catch (error) { if (error instanceof HttpError) return c.json({ error: error.message }, error.status); throw error; }
};

export function pageParams(query) {
  const page = Number.parseInt(query.page ?? '1', 10), size = Number.parseInt(query.size ?? '25', 10);
  if (!Number.isSafeInteger(page) || page < 1 || page > 100000) throw bad('Page must be a positive number.');
  if (!Number.isSafeInteger(size) || size < 5 || size > 100) throw bad('Page size must be between 5 and 100.');
  return { page, size, offset: (page - 1) * size };
}
const paged = (items, total, { page, size }) => ({ items, total, page, size, pages: Math.max(1, Math.ceil(total / size)) });

export function cleanText(value, max = 10000) {
  if (typeof value !== 'string') throw bad('Write the note text.');
  const text = value.replace(/\r\n?/g, '\n').trim();
  if (!text) throw bad('Write the note text.');
  if (text.length > max) throw bad(`Keep the note under ${max.toLocaleString('en-US')} characters.`);
  if (/[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/.test(text)) throw bad('The note contains control characters.');
  return text;
}
function optionalDate(value, end) {
  if (value === undefined || value === '') return null;
  if (typeof value !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(value) || Number.isNaN(Date.parse(value))) throw bad('Use dates like 2026-10-26.');
  return `${value}T${end ? '23:59:59.999' : '00:00:00'}+05:30`;
}
// Who is working: employees enter with their name and the team PIN; the admin password unlocks removal.
// Both live in .env (RESEARCH_PIN, RESEARCH_ADMIN_PASSWORD); the identity is a signed cookie, never a request body.
const EMP = 'rp_emp', ADM = 'rp_admin';
const COOKIE = { path: '/api/app/research', httpOnly: true, sameSite: 'Strict', secure: (process.env.VAULT_PUBLIC_URL || '').startsWith('https:'), maxAge: 60 * 60 * 12 };
const secret = () => process.env.BETTER_AUTH_SECRET;
export const nameKey = n => String(n || '').replace(/\s+/g, ' ').trim().toLowerCase();
export function cleanName(value) {
  if (typeof value !== 'string') throw bad('Type your name.');
  const name = value.replace(/\s+/g, ' ').trim();
  if (name.length < 2 || name.length > 60 || /[\x00-\x1f\x7f<>]/.test(name)) throw bad('Type your name (2 to 60 letters).');
  return name;
}
export const sameSecret = (supplied, expected) => {
  const a = Buffer.from(String(supplied ?? '')), b = Buffer.from(String(expected ?? ''));
  return b.length > 0 && a.length === b.length && timingSafeEqual(a, b);
};
const writer = user => { if (!roleAllows(user.role, 'write')) throw new HttpError(403, 'Researcher access is needed to write. Ask an administrator.'); };

const EXCEL_VERDICT = { 'Matches the book': 'matches', Mismatch: 'mismatch', Unclear: 'unclear' };
const EXCEL_NOTE_TYPE = { matches: 'verified_ok', mismatch: 'rule_error' };

export function createResearchStore({ pool, progressPath }) {
  let cache = { mtime: -1, data: EMPTY, index: null }, loading = null;

  function buildIndex(data) {
    const strategies = new Map(data.strategies.map(s => [s.key, s]));
    const books = new Map(), authors = new Map();
    for (const s of data.strategies) {
      if (!books.has(s.book_id)) books.set(s.book_id, { id: s.book_id, title: s.book, author: s.author });
      if (!authors.has(s.author_id)) authors.set(s.author_id, { id: s.author_id, name: s.author });
    }
    return { strategies, books, authors };
  }

  async function syncPlatform(data) {
    const client = await pool.connect();
    try {
      await client.query('BEGIN');
      for (const e of data.events) {
        await client.query(`INSERT INTO research_platform_events(event_key,happened_at,label,who,work,backtested,proof,source)
          VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(event_key) DO UPDATE SET happened_at=EXCLUDED.happened_at,label=EXCLUDED.label,
          who=EXCLUDED.who,work=EXCLUDED.work,backtested=EXCLUDED.backtested,proof=EXCLUDED.proof,source=EXCLUDED.source`,
        [e.key, e.at, e.label, e.who, e.work, e.count ?? null, e.proof || '', e.source]);
      }
      // Entries typed in the Excel "Research review" sheet. Idempotent; never overrides a verdict given in the portal.
      for (const r of data.excel_reviews) {
        const verdict = EXCEL_VERDICT[r.verdict], who = r.reviewer || 'Excel sheet';
        const at = r.date && !Number.isNaN(Date.parse(r.date)) ? new Date(r.date).toISOString() : null;
        const ref = createHash('sha1').update(JSON.stringify([r.key, r.verdict, r.reviewer, r.date, r.notes])).digest('hex');
        if (verdict) {
          const added = await client.query(`INSERT INTO research_verdict_history(strategy_key,verdict,previous,actor_name,import_ref,created_at)
            VALUES($1,$2,NULL,$3,$4,COALESCE($5::timestamptz,now())) ON CONFLICT(import_ref) DO NOTHING`, [r.key, verdict, who, `excel:${ref}`, at]);
          if (added.rowCount) await client.query(`INSERT INTO research_verdicts(strategy_key,verdict,author_name,updated_at)
            VALUES($1,$2,$3,COALESCE($4::timestamptz,now())) ON CONFLICT(strategy_key) DO NOTHING`, [r.key, verdict, who, at]);
        }
        if (r.notes) {
          const type = EXCEL_NOTE_TYPE[verdict] || 'question', body = r.notes.slice(0, 10000);
          const note = await client.query(`INSERT INTO research_notes(target_type,target_key,note_type,body,author_name,import_ref,created_at,updated_at)
            VALUES('strategy',$1,$2,$3,$4,$5,COALESCE($6::timestamptz,now()),COALESCE($6::timestamptz,now())) ON CONFLICT(import_ref) DO NOTHING RETURNING id,created_at`,
          [r.key, type, body, who, `excel-note:${ref}`, at]);
          if (note.rowCount) await client.query(`INSERT INTO research_note_history(note_id,action,note_type,body,actor_name,created_at)
            VALUES($1,'created',$2,$3,$4,$5)`, [note.rows[0].id, type, body, `${who} (imported from Excel)`, note.rows[0].created_at]);
        }
      }
      await client.query('COMMIT');
    } catch (error) { await client.query('ROLLBACK'); throw error; } finally { client.release(); }
  }

  async function load() {
    if (!progressPath) return cache;
    let info;
    try { info = await stat(progressPath); } catch { return cache; }
    if (info.mtimeMs === cache.mtime) return cache;
    loading ??= (async () => {
      try {
        const data = { ...EMPTY, ...JSON.parse(await readFile(progressPath, 'utf8')) };
        await syncPlatform(data);
        cache = { mtime: info.mtimeMs, data, index: buildIndex(data) };
      } finally { loading = null; }
    })();
    await loading;
    return cache;
  }
  return { load, get index() { return cache.index || buildIndex(EMPTY); } };
}

function targetLabel(index, type, key) {
  if (type === 'strategy') { const s = index.strategies.get(key); return s ? `${s.name} (${s.book})` : `${key} (no longer listed on the site)`; }
  if (type === 'book') { const b = index.books.get(key); return b ? `${b.title} by ${b.author}` : key; }
  if (type === 'author') return index.authors.get(key)?.name || key;
  return 'General';
}
function noteRow(row, index, who) {
  return {
    id: Number(row.id), target_type: row.target_type, target_key: row.target_key, target_label: targetLabel(index, row.target_type, row.target_key),
    note_type: row.note_type, body: row.deleted_at ? '' : row.body, author_id: row.author_id, author_name: row.author_name,
    imported: Boolean(row.import_ref), created_at: row.created_at, updated_at: row.updated_at, edits: Number(row.edits || 0),
    deleted_at: row.deleted_at, deleted_reason: row.deleted_reason, deleted_by_name: row.deleted_by_name || null,
    can_edit: !row.deleted_at && Boolean(who.name) && nameKey(row.author_name) === nameKey(who.name),
    can_delete: !row.deleted_at && ((Boolean(who.name) && nameKey(row.author_name) === nameKey(who.name)) || who.admin),
  };
}
const NOTE_SELECT = `SELECT n.*, (SELECT count(*) FROM research_note_history h WHERE h.note_id=n.id AND h.action='edited') edits,
  d.name deleted_by_name FROM research_notes n LEFT JOIN "user" d ON d.id=n.deleted_by`;

export function registerResearch(app, { pool, store }) {
  const base = '/api/app/research';
  async function employee(c) {
    const id = await getSignedCookie(c, secret(), EMP);
    if (!id || !/^\d+$/.test(id)) return null;
    const row = (await pool.query('SELECT id, name FROM research_employees WHERE id=$1 AND removed_at IS NULL', [id])).rows[0];
    return row ? { id: Number(row.id), name: row.name } : null;
  }
  const isAdmin = async c => (await getSignedCookie(c, secret(), ADM)) === '1';
  const viewer = async c => { const [emp, admin] = await Promise.all([employee(c), isAdmin(c)]); return { emp, admin, name: emp?.name || null }; };
  async function actor(c) {
    const user = c.get('user'); writer(user);
    const emp = await employee(c);
    if (!emp) throw new HttpError(401, 'Enter your name and the PIN first.');
    return { user, emp };
  }
  const needAdmin = async c => { if (!(await isAdmin(c))) throw new HttpError(403, 'Admin password needed.'); };

  app.get(`${base}/me`, route(async c => {
    const v = await viewer(c);
    return c.json({ employee: v.emp, admin: v.admin, ready: Boolean(process.env.RESEARCH_PIN) });
  }));

  app.post(`${base}/enter`, route(async c => {
    const body = await c.req.json().catch(() => ({}));
    if (!process.env.RESEARCH_PIN) throw new HttpError(503, 'The research PIN is not set up yet. Ask the admin.');
    const name = cleanName(body?.name);
    if (!sameSecret(body?.pin, process.env.RESEARCH_PIN)) throw new HttpError(403, 'Wrong PIN.');
    const row = (await pool.query(`INSERT INTO research_employees(name,name_key) VALUES($1,$2)
      ON CONFLICT(name_key) DO UPDATE SET last_seen_at=now() RETURNING id, name, removed_at`, [name, nameKey(name)])).rows[0];
    if (row.removed_at) throw new HttpError(403, 'This name was removed by the admin.');
    await setSignedCookie(c, EMP, String(row.id), secret(), COOKIE);
    return c.json({ employee: { id: Number(row.id), name: row.name } });
  }));

  app.post(`${base}/admin`, route(async c => {
    const body = await c.req.json().catch(() => ({}));
    if (!sameSecret(body?.password, process.env.RESEARCH_ADMIN_PASSWORD)) throw new HttpError(403, 'Wrong admin password.');
    await setSignedCookie(c, ADM, '1', secret(), { ...COOKIE, maxAge: 60 * 60 * 2 });
    return c.json({ admin: true });
  }));

  app.post(`${base}/leave`, route(async c => {
    deleteCookie(c, EMP, COOKIE); deleteCookie(c, ADM, COOKIE);
    return c.json({ ok: true });
  }));

  app.get(`${base}/employees`, route(async c => {
    await needAdmin(c);
    const rows = await pool.query(`SELECT e.id, e.name, e.created_at, e.last_seen_at, e.removed_at,
        (SELECT count(*) FROM research_verdict_history v WHERE lower(v.actor_name)=e.name_key)::int checks,
        (SELECT count(*) FROM research_verdict_history v WHERE lower(v.actor_name)=e.name_key AND v.verdict='mismatch')::int mismatches,
        (SELECT count(*) FROM research_notes n WHERE lower(n.author_name)=e.name_key AND n.deleted_at IS NULL)::int notes,
        (SELECT count(*) FROM research_notes n WHERE lower(n.author_name)=e.name_key AND n.deleted_at IS NULL AND n.note_type='rule_error')::int rule_errors
      FROM research_employees e ORDER BY e.removed_at NULLS FIRST, e.last_seen_at DESC LIMIT 500`);
    return c.json(rows.rows.map(r => ({ ...r, id: Number(r.id) })));
  }));

  app.delete(`${base}/employees/:id`, route(async c => {
    await needAdmin(c);
    const id = Number(c.req.param('id'));
    if (!Number.isSafeInteger(id) || id < 1) throw bad('Unknown employee.');
    const row = await pool.query('UPDATE research_employees SET removed_at=now() WHERE id=$1 AND removed_at IS NULL RETURNING id', [id]);
    if (!row.rowCount) throw new HttpError(404, 'Unknown employee.');
    await pool.query('INSERT INTO vault_audit(actor_id,action,entity) VALUES($1,$2,$3)', [c.get('user').id, 'research.employee.removed', String(id)]);
    return c.json({ removed: id });
  }));

  // Card browsing: authors, then one author's books, with counts per card.
  app.get(`${base}/browse`, route(async c => {
    const { data } = await store.load();
    const author = c.req.query('author');
    const checked = new Set((await pool.query("SELECT strategy_key FROM research_verdicts WHERE verdict<>'not_checked'")).rows.map(r => r.strategy_key));
    const groups = new Map();
    for (const s of data.strategies) {
      if (author && s.author_id !== author) continue;
      const id = author ? s.book_id : s.author_id;
      const g = groups.get(id) || { id, name: author ? s.book : s.author, author: s.author, books: new Set(), strategies: 0, verified: 0, held: 0, unreviewed: 0, checked: 0 };
      g.books.add(s.book_id); g.strategies++; g[s.site]++; if (checked.has(s.key)) g.checked++;
      groups.set(id, g);
    }
    const items = [...groups.values()].map(g => ({ ...g, books: g.books.size })).sort((a, b) => b.strategies - a.strategies || a.name.localeCompare(b.name));
    return c.json({ items });
  }));

  app.get(`${base}/summary`, route(async c => {
    const { data } = await store.load();
    const [verdicts, notes, activity, people] = await Promise.all([
      pool.query('SELECT verdict, count(*)::int n FROM research_verdicts GROUP BY verdict'),
      pool.query('SELECT note_type, count(*)::int n FROM research_notes WHERE deleted_at IS NULL GROUP BY note_type'),
      pool.query(`SELECT to_char(d,'YYYY-MM-DD') AS "day", sum(v)::int verdicts, sum(n)::int notes FROM (
          SELECT date_trunc('day', created_at AT TIME ZONE 'Asia/Kolkata') d, 1 v, 0 n FROM research_verdict_history
          UNION ALL SELECT date_trunc('day', created_at AT TIME ZONE 'Asia/Kolkata'), 0, 1 FROM research_note_history WHERE action='created'
        ) t WHERE d > (now() AT TIME ZONE 'Asia/Kolkata') - interval '60 days' GROUP BY d ORDER BY d`),
      pool.query(`SELECT count(DISTINCT actor_name)::int n FROM (SELECT actor_name FROM research_verdict_history UNION ALL SELECT actor_name FROM research_note_history) t`),
    ]);
    const team = { verdicts: Object.fromEntries(Object.keys(VERDICTS).map(k => [k, 0])), notes: Object.fromEntries(Object.keys(NOTE_TYPES).map(k => [k, 0])) };
    for (const r of verdicts.rows) team.verdicts[r.verdict] = r.n;
    for (const r of notes.rows) team.notes[r.note_type] = r.n;
    team.verdicts.not_checked += Math.max(0, data.strategies.length - Object.values(team.verdicts).reduce((a, b) => a + b, 0));
    team.people = people.rows[0].n;
    // Per-strategy site status in catalogue order, for the corpus strip.
    const verdictMap = new Map((await pool.query('SELECT strategy_key, verdict FROM research_verdicts')).rows.map(r => [r.strategy_key, r.verdict]));
    const strip = data.strategies.map(s => [s.author_id, s.site[0], (verdictMap.get(s.key) || 'n')[0]]);
    return c.json({ as_of: data.as_of, generated_at: data.generated_at, run: data.run, totals: data.totals, reasons: data.reasons,
      history: data.history, authors: data.authors, books: data.books, todo_authors: data.todo_authors,
      deploys: data.events.filter(e => e.count != null && (e.who === 'Platform')).map(e => ({ at: e.at, count: e.count, work: e.work, label: e.label })),
      team, activity: activity.rows, strip,
      author_index: [...store.index.authors.values()].sort((a, b) => a.name.localeCompare(b.name)),
      book_index: [...store.index.books.values()].sort((a, b) => a.author.localeCompare(b.author) || a.title.localeCompare(b.title)) });
  }));

  app.get(`${base}/strategies`, route(async c => {
    const q = c.req.query(), paging = pageParams(q);
    const { data } = await store.load();
    if (q.site && !SITE.has(q.site)) throw bad('Unknown site status.');
    if (q.verdict && !VERDICTS[q.verdict]) throw bad('Unknown verdict.');
    const verdicts = new Map((await pool.query('SELECT strategy_key, verdict, author_id, author_name, updated_at FROM research_verdicts')).rows.map(r => [r.strategy_key, r]));
    const counts = new Map((await pool.query(`SELECT target_key, count(*)::int n, count(*) FILTER (WHERE note_type='rule_error')::int errors
      FROM research_notes WHERE target_type='strategy' AND deleted_at IS NULL GROUP BY target_key`)).rows.map(r => [r.target_key, r]));
    const words = (q.q || '').toLowerCase().split(/\s+/).filter(Boolean).slice(0, 8);
    const matches = data.strategies.filter(s => {
      const v = verdicts.get(s.key);
      if (q.author && s.author_id !== q.author) return false;
      if (q.book && s.book_id !== q.book) return false;
      if (q.site && s.site !== q.site) return false;
      if (q.verdict && (v?.verdict || 'not_checked') !== q.verdict) return false;
      if (q.reviewer && !(v && (v.author_id === q.reviewer || v.author_name === q.reviewer))) return false;
      if (words.length) { const hay = `${s.name} ${s.book} ${s.author} ${s.variant} ${s.reason}`.toLowerCase(); if (!words.every(w => hay.includes(w))) return false; }
      return true;
    });
    const items = matches.slice(paging.offset, paging.offset + paging.size).map(s => {
      const v = verdicts.get(s.key), n = counts.get(s.key);
      return { ...s, verdict: v?.verdict || 'not_checked', verdict_by: v?.author_name || null, verdict_at: v?.updated_at || null,
        notes: n?.n || 0, rule_errors: n?.errors || 0 };
    });
    const authors = [...new Map(data.strategies.map(s => [s.author_id, s.author])).entries()].map(([id, name]) => ({ id, name })).sort((a, b) => a.name.localeCompare(b.name));
    const books = q.author ? [...new Map(data.strategies.filter(s => s.author_id === q.author).map(s => [s.book_id, s.book])).entries()].map(([id, title]) => ({ id, title })) : [];
    const reviewers = [...new Set([...verdicts.values()].map(v => v.author_name))].sort();
    return c.json({ ...paged(items, matches.length, paging), facets: { authors, books, reviewers } });
  }));

  app.get(`${base}/strategy`, route(async c => {
    const key = c.req.query('key');
    if (typeof key !== 'string' || !KEY.test(key)) throw bad('Unknown strategy.');
    await store.load(); const { index } = store;
    const strategy = index.strategies.get(key), who = await viewer(c);
    const [verdict, history, notes] = await Promise.all([
      pool.query('SELECT verdict, author_name, updated_at FROM research_verdicts WHERE strategy_key=$1', [key]),
      pool.query('SELECT verdict, previous, actor_name, created_at FROM research_verdict_history WHERE strategy_key=$1 ORDER BY created_at DESC, id DESC LIMIT 50', [key]),
      pool.query(`${NOTE_SELECT} WHERE n.target_type='strategy' AND n.target_key=$1 ORDER BY n.created_at DESC, n.id DESC LIMIT 200`, [key]),
    ]);
    if (!strategy && !history.rowCount && !notes.rowCount) throw new HttpError(404, 'This strategy is not on the site.');
    return c.json({ strategy: strategy || null, verdict: verdict.rows[0] || { verdict: 'not_checked' }, history: history.rows,
      notes: notes.rows.map(r => noteRow(r, index, who)) });
  }));

  app.put(`${base}/verdict`, route(async c => {
    const { user, emp } = await actor(c);
    const body = await c.req.json();
    await store.load();
    if (!body || typeof body.key !== 'string' || !store.index.strategies.has(body.key)) throw bad('Unknown strategy.');
    if (!VERDICTS[body.verdict]) throw bad('Choose Not checked, Matches the book, Mismatch or Unclear.');
    const client = await pool.connect();
    try {
      await client.query('BEGIN');
      const current = await client.query('SELECT verdict FROM research_verdicts WHERE strategy_key=$1 FOR UPDATE', [body.key]);
      const previous = current.rows[0]?.verdict || 'not_checked';
      if (previous !== body.verdict) {
        await client.query(`INSERT INTO research_verdicts(strategy_key,verdict,author_id,author_name) VALUES($1,$2,$3,$4)
          ON CONFLICT(strategy_key) DO UPDATE SET verdict=EXCLUDED.verdict, author_id=EXCLUDED.author_id, author_name=EXCLUDED.author_name, updated_at=now()`,
        [body.key, body.verdict, user.id, emp.name]);
        await client.query('INSERT INTO research_verdict_history(strategy_key,verdict,previous,actor_id,actor_name) VALUES($1,$2,$3,$4,$5)',
          [body.key, body.verdict, previous, user.id, emp.name]);
        await client.query('INSERT INTO vault_audit(actor_id,action,entity) VALUES($1,$2,$3)', [user.id, 'research.verdict', body.key.slice(0, 80)]);
      }
      await client.query('COMMIT');
      return c.json({ key: body.key, verdict: body.verdict, previous, changed: previous !== body.verdict });
    } catch (error) { await client.query('ROLLBACK'); throw error; } finally { client.release(); }
  }));

  app.get(`${base}/notes`, route(async c => {
    const q = c.req.query(), paging = pageParams(q), where = [], values = [];
    const add = (sql, value) => { values.push(value); where.push(sql.replaceAll('?', `$${values.length}`)); };
    if (q.target_type) { if (!TARGETS.has(q.target_type)) throw bad('Unknown note target.'); add('n.target_type=?', q.target_type); }
    if (q.target_key) add('n.target_key=?', q.target_key);
    if (q.type) { if (!NOTE_TYPES[q.type]) throw bad('Unknown note type.'); add('n.note_type=?', q.type); }
    if (q.author) add('(n.author_id=? OR n.author_name=?)', q.author);
    if (q.q) add("n.body ILIKE '%' || ? || '%'", q.q.slice(0, 100).replace(/[\\%_]/g, m => '\\' + m));
    if (q.deleted !== 'include') where.push('n.deleted_at IS NULL');
    const sqlWhere = where.length ? `WHERE ${where.join(' AND ')}` : '';
    const [rows, total] = await Promise.all([
      pool.query(`${NOTE_SELECT} ${sqlWhere} ORDER BY n.created_at DESC, n.id DESC LIMIT ${paging.size} OFFSET ${paging.offset}`, values),
      pool.query(`SELECT count(*)::int n FROM research_notes n ${sqlWhere}`, values),
    ]);
    await store.load(); const who = await viewer(c);
    return c.json(paged(rows.rows.map(r => noteRow(r, store.index, who)), total.rows[0].n, paging));
  }));

  app.post(`${base}/notes`, route(async c => {
    const { user, emp } = await actor(c);
    const body = await c.req.json(); await store.load();
    const { index } = store;
    if (!body || !TARGETS.has(body.target_type)) throw bad('Attach the note to a strategy, book, author or the general log.');
    if (!NOTE_TYPES[body.note_type]) throw bad('Choose a note type.');
    const key = body.target_type === 'general' ? '' : body.target_key;
    const known = { strategy: index.strategies, book: index.books, author: index.authors }[body.target_type];
    if (known && !known.has(key)) throw bad(`Unknown ${body.target_type}.`);
    const text = cleanText(body.body);
    const client = await pool.connect();
    try {
      await client.query('BEGIN');
      const note = await client.query(`INSERT INTO research_notes(target_type,target_key,note_type,body,author_id,author_name)
        VALUES($1,$2,$3,$4,$5,$6) RETURNING *`, [body.target_type, key, body.note_type, text, user.id, emp.name]);
      await client.query(`INSERT INTO research_note_history(note_id,action,note_type,body,actor_id,actor_name,created_at)
        VALUES($1,'created',$2,$3,$4,$5,$6)`, [note.rows[0].id, body.note_type, text, user.id, emp.name, note.rows[0].created_at]);
      await client.query('INSERT INTO vault_audit(actor_id,action,entity) VALUES($1,$2,$3)', [user.id, 'research.note.created', String(note.rows[0].id)]);
      await client.query('COMMIT');
      return c.json(noteRow(note.rows[0], index, { name: emp.name, admin: false }), 201);
    } catch (error) { await client.query('ROLLBACK'); throw error; } finally { client.release(); }
  }));

  const noteId = c => { const id = Number(c.req.param('id')); if (!Number.isSafeInteger(id) || id < 1) throw bad('Unknown note.'); return id; };

  app.patch(`${base}/notes/:id`, route(async c => {
    const { user, emp } = await actor(c), who = { name: emp.name, admin: false };
    const id = noteId(c), body = await c.req.json();
    if (body.note_type !== undefined && !NOTE_TYPES[body.note_type]) throw bad('Choose a note type.');
    const text = body.body === undefined ? undefined : cleanText(body.body);
    const client = await pool.connect();
    try {
      await client.query('BEGIN');
      const found = await client.query('SELECT * FROM research_notes WHERE id=$1 FOR UPDATE', [id]);
      const note = found.rows[0];
      if (!note) throw new HttpError(404, 'Unknown note.');
      if (note.deleted_at) throw new HttpError(409, 'This note was removed and cannot be edited.');
      if (nameKey(note.author_name) !== nameKey(emp.name)) throw new HttpError(403, 'Only the person who wrote a note can edit it.');
      const next = { body: text ?? note.body, note_type: body.note_type ?? note.note_type };
      if (next.body === note.body && next.note_type === note.note_type) { await client.query('ROLLBACK'); return c.json(noteRow({ ...note, edits: 0 }, store.index, who)); }
      const saved = await client.query('UPDATE research_notes SET body=$2, note_type=$3, updated_at=now() WHERE id=$1 RETURNING *', [id, next.body, next.note_type]);
      await client.query(`INSERT INTO research_note_history(note_id,action,note_type,body,actor_id,actor_name,created_at) VALUES($1,'edited',$2,$3,$4,$5,$6)`,
        [id, next.note_type, next.body, user.id, emp.name, saved.rows[0].updated_at]);
      await client.query('INSERT INTO vault_audit(actor_id,action,entity) VALUES($1,$2,$3)', [user.id, 'research.note.edited', String(id)]);
      await client.query('COMMIT');
      const edits = (await pool.query("SELECT count(*)::int n FROM research_note_history WHERE note_id=$1 AND action='edited'", [id])).rows[0].n;
      return c.json(noteRow({ ...saved.rows[0], edits }, store.index, who));
    } catch (error) { await client.query('ROLLBACK').catch(() => {}); throw error; } finally { client.release(); }
  }));

  app.delete(`${base}/notes/:id`, route(async c => {
    const user = c.get('user'), who = await viewer(c);
    if (!who.emp && !who.admin) throw new HttpError(401, 'Enter your name and the PIN first.');
    const id = noteId(c), body = await c.req.json().catch(() => ({}));
    const reason = body?.reason ? cleanText(body.reason, 500) : (who.admin ? 'Removed by the admin' : 'Removed by the writer');
    const actorName = who.admin ? 'Admin' : who.name;
    const client = await pool.connect();
    try {
      await client.query('BEGIN');
      const note = (await client.query('SELECT * FROM research_notes WHERE id=$1 FOR UPDATE', [id])).rows[0];
      if (!note) throw new HttpError(404, 'Unknown note.');
      if (note.deleted_at) throw new HttpError(409, 'This note was already removed.');
      if (!who.admin && nameKey(note.author_name) !== nameKey(who.name)) throw new HttpError(403, 'Only the writer or the admin can remove a note.');
      const saved = await client.query('UPDATE research_notes SET deleted_at=now(), deleted_by=$2, deleted_reason=$3 WHERE id=$1 RETURNING *', [id, user.id, reason]);
      await client.query(`INSERT INTO research_note_history(note_id,action,note_type,body,actor_id,actor_name,created_at) VALUES($1,'deleted',$2,$3,$4,$5,$6)`,
        [id, note.note_type, `Removed: ${reason}`, user.id, actorName, saved.rows[0].deleted_at]);
      await client.query('INSERT INTO vault_audit(actor_id,action,entity) VALUES($1,$2,$3)', [user.id, 'research.note.deleted', String(id)]);
      await client.query('COMMIT');
      return c.json(noteRow({ ...saved.rows[0], deleted_by_name: actorName }, store.index, who));
    } catch (error) { await client.query('ROLLBACK').catch(() => {}); throw error; } finally { client.release(); }
  }));

  app.get(`${base}/notes/:id/history`, route(async c => {
    const id = noteId(c);
    const rows = await pool.query('SELECT action, note_type, body, actor_name, created_at FROM research_note_history WHERE note_id=$1 ORDER BY created_at, id', [id]);
    if (!rows.rowCount) throw new HttpError(404, 'Unknown note.');
    return c.json(rows.rows);
  }));

  app.get(`${base}/timeline`, route(async c => {
    const q = c.req.query(), paging = pageParams(q), where = [], values = [];
    const add = (sql, value) => { values.push(value); where.push(sql.replaceAll('?', `$${values.length}`)); };
    if (q.kind) { if (!['note', 'verdict', 'platform'].includes(q.kind)) throw bad('Unknown timeline type.'); add('kind=?', q.kind); }
    if (q.person) add('(actor_id=? OR who=?)', q.person);
    const from = optionalDate(q.from), to = optionalDate(q.to, true);
    if (from) add('at>=?::timestamptz', from);
    if (to) add('at<=?::timestamptz', to);
    const sqlWhere = where.length ? `WHERE ${where.join(' AND ')}` : '';
    const union = `SELECT 'note' kind, h.created_at at, h.id seq, h.actor_name who, h.actor_id, h.action, n.note_type detail, h.body text,
        n.target_type, n.target_key, n.id ref, NULL::int backtested, '' proof, n.deleted_at IS NOT NULL removed FROM research_note_history h JOIN research_notes n ON n.id=h.note_id
      UNION ALL SELECT 'verdict', created_at, id, actor_name, actor_id, 'set', verdict, previous, 'strategy', strategy_key, id, NULL, '', false FROM research_verdict_history
      UNION ALL SELECT 'platform', happened_at, 0, who, NULL, source, label, work, '', event_key, 0, backtested, proof, false FROM research_platform_events`;
    await store.load();
    const [rows, total] = await Promise.all([
      pool.query(`SELECT * FROM (${union}) t ${sqlWhere} ORDER BY at DESC, seq DESC LIMIT ${paging.size} OFFSET ${paging.offset}`, values),
      pool.query(`SELECT count(*)::int n FROM (${union}) t ${sqlWhere}`, values),
    ]);
    const items = rows.rows.map(r => ({ kind: r.kind, at: r.at, who: r.who, actor_id: r.actor_id, action: r.action, detail: r.detail,
      text: r.removed && r.action !== 'deleted' ? '' : r.text, removed: r.removed, ref: Number(r.ref) || null, target_type: r.target_type || null,
      target_key: r.target_key, target_label: r.target_type ? targetLabel(store.index, r.target_type, r.target_key) : null,
      backtested: r.backtested, proof: r.proof }));
    return c.json(paged(items, total.rows[0].n, paging));
  }));

  app.get(`${base}/people`, route(async c => {
    const rows = await pool.query(`WITH acts AS (
        SELECT actor_id, actor_name, 'verdict' kind, verdict detail, created_at FROM research_verdict_history
        UNION ALL SELECT h.actor_id, regexp_replace(h.actor_name, ' \\(imported from Excel\\)$', ''), 'note_' || h.action, n.note_type, h.created_at
          FROM research_note_history h JOIN research_notes n ON n.id=h.note_id)
      SELECT COALESCE(u.id, '') id, COALESCE(u.name, a.actor_name) name, u.role,
        count(*) FILTER (WHERE kind='verdict')::int verdicts,
        count(*) FILTER (WHERE kind='verdict' AND detail='matches')::int matches,
        count(*) FILTER (WHERE kind='verdict' AND detail='mismatch')::int mismatches,
        count(*) FILTER (WHERE kind='verdict' AND detail='unclear')::int unclear,
        count(*) FILTER (WHERE kind='note_created')::int notes,
        count(*) FILTER (WHERE kind='note_created' AND detail='rule_error')::int rule_errors,
        count(*) FILTER (WHERE kind='note_edited')::int edits,
        max(created_at) last_at, (u.id IS NULL) imported
      FROM acts a LEFT JOIN "user" u ON u.id=a.actor_id
      GROUP BY COALESCE(u.id, ''), COALESCE(u.name, a.actor_name), u.role, u.id ORDER BY max(created_at) DESC`);
    const idle = await pool.query(`SELECT id, name, role FROM "user" u WHERE role IN ('admin','researcher') AND NOT EXISTS
      (SELECT 1 FROM research_verdict_history v WHERE v.actor_id=u.id) AND NOT EXISTS (SELECT 1 FROM research_note_history h WHERE h.actor_id=u.id) ORDER BY name LIMIT 200`);
    return c.json([...rows.rows, ...idle.rows.map(u => ({ ...u, verdicts: 0, matches: 0, mismatches: 0, unclear: 0, notes: 0, rule_errors: 0, edits: 0, last_at: null, imported: false }))]);
  }));
}
