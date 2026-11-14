// Research YTS API: QA issues found on the YT Strategies pages, filed per YT collection and strategy.
// Research YTS has its own sign-in, separate from the app login: each QA has a username and password, and
// Research YTS admins manage those accounts. The QA identity is a signed, httpOnly cookie scoped to this API;
// identity and time always come from the server, never from request bodies.
// A QA sees and works on the issues they reported; an admin sees every QA's issues and can act on all of them.
// Deleting an issue is soft and needs a reason; deactivating a QA keeps their history.
import { randomBytes, scrypt, timingSafeEqual } from 'node:crypto';
import { promisify } from 'node:util';
import { deleteCookie, getSignedCookie, setSignedCookie } from 'hono/cookie';

export const CATEGORIES = {
  wrong_trades: 'Wrong trades (entry / exit)',
  wrong_stats: 'Wrong statistics or report numbers',
  data: 'Data problem',
  page: 'Page or display problem',
  rules: 'Rule or settings question',
  other: 'Other',
};
export const SEVERITIES = { low: 'Low', normal: 'Normal', high: 'High' };
export const STATUSES = { open: 'Open', in_progress: 'In progress', fixed: 'Fixed', not_issue: 'Not an issue' };
const COLLECTION = /^yt_[a-z0-9_]{1,36}$/;
export const GENERAL = 'general'; // collection of a general issue: one issue about several strategies
const MAX_STRATEGIES = 300;
export const MAX_FILE_BYTES = 10 * 1024 * 1024, MAX_FILES = 20;
export const YTS_UPLOAD = /^\/api\/app\/research-yts\/issues\/\d+\/files$/; // the only route that takes a large body
// Only these image types are shown in the page; every other file is downloaded, never opened in the vault.
const INLINE_IMAGES = new Set(['image/png', 'image/jpeg', 'image/gif', 'image/webp']);
const STRATEGY = /^[A-Za-z0-9][A-Za-z0-9_.:+-]{0,119}$/;
const USERNAME = /^[a-z0-9][a-z0-9._-]{1,39}$/;
const CONTROL = /[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/;

export class HttpError extends Error { constructor(status, message) { super(message); this.status = status; } }
const bad = message => new HttpError(400, message);
const route = handler => async c => {
  try { return await handler(c); }
  catch (error) { if (error instanceof HttpError) return c.json({ error: error.message }, error.status); throw error; }
};

/* ---------------------------------------------------------------- input checks */

export function cleanText(value, { field, min = 1, max = 10000, multiline = true }) {
  if (typeof value !== 'string') throw bad(`Write the ${field}.`);
  const text = (multiline ? value.replace(/\r\n?/g, '\n') : value.replace(/\s+/g, ' ')).trim();
  if (text.length < min) throw bad(min > 1 ? `Write the ${field} (at least ${min} characters).` : `Write the ${field}.`);
  if (text.length > max) throw bad(`Keep the ${field} under ${max.toLocaleString('en-US')} characters.`);
  if (CONTROL.test(text)) throw bad(`The ${field} contains control characters.`);
  return text;
}
const optionalText = (value, field, max) => value === undefined || value === null || value === '' ? '' : cleanText(value, { field, max, multiline: false });
export function cleanDate(value, field) {
  if (value === undefined || value === null || value === '') return null;
  if (typeof value !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(value) || Number.isNaN(Date.parse(value))) throw bad(`Use a date like 2026-11-13 for ${field}.`);
  return value;
}
export function cleanLink(value) {
  if (value === undefined || value === null || value === '') return '';
  if (typeof value !== 'string' || value.length > 2000 || !value.startsWith('#/') || CONTROL.test(value) || /[<>"\s]/.test(value)) throw bad('The page link must be a link inside the vault (starting with #/).');
  return value;
}
export function cleanUsername(value) {
  const name = typeof value === 'string' ? value.trim().toLowerCase() : '';
  if (!USERNAME.test(name)) throw bad('Usernames are 2 to 40 letters, numbers, dots, dashes or underscores.');
  return name;
}
export function cleanDisplayName(value) {
  const name = cleanText(value, { field: 'name', min: 2, max: 60, multiline: false });
  if (/[<>]/.test(name)) throw bad('Type the person\'s name (2 to 60 letters).');
  return name;
}
export function cleanPassword(value, field = 'password') {
  if (typeof value !== 'string' || value.length < 1 || value.length > 128 || CONTROL.test(value)) throw bad(`The ${field} must be 1 to 128 characters.`);
  return value;
}
const pick = (map, value, field) => { if (!Object.hasOwn(map, value)) throw bad(`Choose a ${field}.`); return value; };

/** Checks a new issue; returns the columns to store. */
export function validateIssue(body) {
  if (!body || typeof body !== 'object') throw bad('Send the issue details.');
  const general = body.collection === GENERAL;
  if (typeof body.collection !== 'string' || (!general && !COLLECTION.test(body.collection))) throw bad('Choose the YT collection.');
  const strategy = general ? '' : body.strategy_id ?? '';
  if (strategy !== '' && (typeof strategy !== 'string' || !STRATEGY.test(strategy))) throw bad('Choose a strategy from the list, or "Whole collection".');
  const picked = general ? cleanStrategies(body.strategies) : { strategies: [], collections: [] };
  const from = cleanDate(body.period_from, 'From'), to = cleanDate(body.period_to, 'To');
  if (from && to && from > to) throw bad('The period starts after it ends.');
  return {
    collection: body.collection, strategy_id: strategy, ...picked,
    category: pick(CATEGORIES, body.category, 'category'),
    severity: body.severity === undefined ? 'normal' : pick(SEVERITIES, body.severity, 'severity'),
    title: cleanText(body.title, { field: 'title', min: 3, max: 160, multiline: false }),
    body: cleanText(body.body, { field: 'description' }),
    instrument: optionalText(body.instrument, 'instrument', 80),
    period_from: from, period_to: to,
    page_link: cleanLink(body.page_link),
  };
}
/** The strategies a general issue is about, as [{ collection, id }]; returns the ids and their collections. */
export function cleanStrategies(value) {
  if (value === undefined || value === null) return { strategies: [], collections: [] };
  if (!Array.isArray(value) || value.length > MAX_STRATEGIES) throw bad(`Choose up to ${MAX_STRATEGIES} strategies.`);
  const ids = new Set(), books = new Set();
  for (const x of value) {
    if (!x || typeof x !== 'object' || typeof x.id !== 'string' || !STRATEGY.test(x.id) || typeof x.collection !== 'string' || !COLLECTION.test(x.collection)) throw bad('Choose the strategies from the list.');
    ids.add(x.id); books.add(x.collection);
  }
  return { strategies: [...ids], collections: [...books] };
}
/** A file name safe to store and to send back in a download header. */
export function cleanFileName(value) {
  const name = typeof value === 'string' ? value.normalize('NFC').replace(/[\\/]/g, '_').replace(/\s+/g, ' ').trim() : '';
  if (!name || name.length > 200 || CONTROL.test(name) || /[<>"]/.test(name)) throw bad('Give the file a name of up to 200 characters (no < > or ").');
  return name;
}
export function cleanContentType(value) {
  const type = typeof value === 'string' ? value.split(';')[0].trim().toLowerCase() : '';
  return /^[a-z0-9][a-z0-9.+-]*\/[a-z0-9][a-z0-9.+-]*$/.test(type) && type.length <= 100 ? type : 'application/octet-stream';
}
/** Checks an edit; only the fields sent are changed. */
export function validateEdit(body) {
  if (!body || typeof body !== 'object') throw bad('Send the changes.');
  const out = {};
  if (body.title !== undefined) out.title = cleanText(body.title, { field: 'title', min: 3, max: 160, multiline: false });
  if (body.body !== undefined) out.body = cleanText(body.body, { field: 'description' });
  if (body.category !== undefined) out.category = pick(CATEGORIES, body.category, 'category');
  if (body.severity !== undefined) out.severity = pick(SEVERITIES, body.severity, 'severity');
  if (body.instrument !== undefined) out.instrument = optionalText(body.instrument, 'instrument', 80);
  if (body.strategies !== undefined) Object.assign(out, cleanStrategies(body.strategies)); // general issues only (checked by the route)
  if (!Object.keys(out).length) throw bad('Nothing to change.');
  return out;
}
export function pageParams(query) {
  const page = Number.parseInt(query.page ?? '1', 10), size = Number.parseInt(query.size ?? '25', 10);
  if (!Number.isSafeInteger(page) || page < 1 || page > 100000) throw bad('Page must be a positive number.');
  if (!Number.isSafeInteger(size) || size < 5 || size > 100) throw bad('Page size must be between 5 and 100.');
  return { page, size, offset: (page - 1) * size };
}
const idParam = (c, what) => {
  const raw = c.req.param('id'), id = Number.parseInt(raw, 10);
  if (!Number.isSafeInteger(id) || id < 1 || String(id) !== raw) throw new HttpError(404, `No such ${what}.`);
  return id;
};

/* ---------------------------------------------------------------- passwords */

const scryptAsync = promisify(scrypt);
const SCRYPT = { N: 16384, r: 8, p: 1, maxmem: 64 * 1024 * 1024 };
export async function hashPassword(password) {
  const salt = randomBytes(16), key = await scryptAsync(password, salt, 64, SCRYPT);
  return `scrypt$${SCRYPT.N}$${SCRYPT.r}$${SCRYPT.p}$${salt.toString('base64')}$${key.toString('base64')}`;
}
export async function checkPassword(password, stored) {
  const [kind, N, r, p, salt, key] = String(stored || '').split('$');
  if (kind !== 'scrypt' || !salt || !key || typeof password !== 'string') return false;
  const expected = Buffer.from(key, 'base64');
  const actual = await scryptAsync(password, Buffer.from(salt, 'base64'), expected.length, { N: Number(N), r: Number(r), p: Number(p), maxmem: SCRYPT.maxmem });
  return actual.length === expected.length && timingSafeEqual(actual, expected);
}

/** Creates the first Research YTS admin from the local .env (RESEARCH_YTS_ADMIN_USERNAME / _PASSWORD) when it does not exist.
 *  An existing account is never changed here: its password is managed on the page. */
export async function seedResearchYtsAdmin(pool, env = process.env) {
  if (!env.RESEARCH_YTS_ADMIN_PASSWORD) return false;
  const username = cleanUsername(env.RESEARCH_YTS_ADMIN_USERNAME || 'admin');
  const added = await pool.query(`INSERT INTO yts_qa_users(username,display_name,password_hash,is_admin) VALUES($1,$2,$3,true)
    ON CONFLICT (username) DO NOTHING`, [username, env.RESEARCH_YTS_ADMIN_NAME || 'Admin', await hashPassword(cleanPassword(env.RESEARCH_YTS_ADMIN_PASSWORD))]);
  return added.rowCount > 0;
}

/* ---------------------------------------------------------------- API */

const COOKIE_NAME = 'yts_qa';
const SESSION_SECONDS = 60 * 60 * 12;
const COLUMNS = `i.id,i.collection,i.strategy_id,i.category,i.severity,i.status,i.title,i.body,i.instrument,
  to_char(i.period_from,'YYYY-MM-DD') AS period_from,to_char(i.period_to,'YYYY-MM-DD') AS period_to,i.page_link,
  i.strategies,i.collections,i.reporter_name,i.qa_user_id,i.created_at,i.updated_at,i.deleted_at,i.deleted_reason,
  (SELECT count(*)::int FROM yts_issue_events e WHERE e.issue_id=i.id AND e.action='comment') AS comments,
  (SELECT count(*)::int FROM yts_attachments a WHERE a.issue_id=i.id AND a.deleted_at IS NULL) AS files`;
const shape = (row, qa) => {
  const own = row.qa_user_id !== null && Number(row.qa_user_id) === qa.id;
  const { qa_user_id, ...rest } = row;
  return { ...rest, reporter_id: qa_user_id === null ? null : Number(qa_user_id), can_edit: !row.deleted_at && (own || qa.admin), can_delete: !row.deleted_at && (own || qa.admin) };
};
const publicQa = u => ({ id: Number(u.id), username: u.username, name: u.display_name, admin: u.is_admin });

export function registerResearchYts(app, { pool, secret = process.env.BETTER_AUTH_SECRET, secure = (process.env.VAULT_PUBLIC_URL || '').startsWith('https:'), now = () => Date.now() }) {
  const base = '/api/app/research-yts';
  const cookieOptions = { path: base, httpOnly: true, sameSite: 'Strict', secure, maxAge: SESSION_SECONDS };
  const failures = new Map(); // `${ip}|${username}` -> { count, until }: 5 wrong passwords lock that pair for a minute

  async function qaOf(c) {
    const value = await getSignedCookie(c, secret, COOKIE_NAME);
    const match = typeof value === 'string' && /^(\d+)\.(\d+)$/.exec(value);
    if (!match) return null;
    const row = (await pool.query('SELECT id,username,display_name,is_admin,session_stamp,deactivated_at FROM yts_qa_users WHERE id=$1', [match[1]])).rows[0];
    if (!row || row.deactivated_at || String(row.session_stamp) !== match[2]) return null;
    return publicQa(row);
  }
  const signIn = (c, row) => setSignedCookie(c, COOKIE_NAME, `${row.id}.${row.session_stamp}`, secret, cookieOptions);
  async function need(c) {
    const qa = await qaOf(c);
    if (!qa) throw new HttpError(401, 'Sign in to Research YTS first.');
    return qa;
  }
  async function needAdmin(c) {
    const qa = await need(c);
    if (!qa.admin) throw new HttpError(403, 'Only a Research YTS admin can manage QA accounts.');
    return qa;
  }
  const tx = async work => {
    const client = await pool.connect();
    try { await client.query('BEGIN'); const out = await work(client); await client.query('COMMIT'); return out; }
    catch (error) { await client.query('ROLLBACK'); throw error; } finally { client.release(); }
  };
  const one = async (id, qa, client = pool) => {
    const row = (await client.query(`SELECT ${COLUMNS} FROM yts_issues i WHERE i.id=$1`, [id])).rows[0];
    if (!row || (!qa.admin && Number(row.qa_user_id) !== qa.id)) throw new HttpError(404, 'No such issue.');
    return shape(row, qa);
  };
  // Locks an issue the QA may work on (their own, or any for an admin) that has not been deleted.
  const live = async (client, id, qa) => {
    const row = (await client.query('SELECT qa_user_id,deleted_at,status,collection FROM yts_issues WHERE id=$1 FOR UPDATE', [id])).rows[0];
    if (!row || (!qa.admin && Number(row.qa_user_id) !== qa.id)) throw new HttpError(404, 'No such issue.');
    if (row.deleted_at) throw new HttpError(409, 'This issue was deleted.');
    return row;
  };
  const event = (client, id, action, c, qa, { status = null, body = '' } = {}) =>
    client.query('INSERT INTO yts_issue_events(issue_id,action,status,body,actor_id,actor_name,qa_user_id) VALUES($1,$2,$3,$4,$5,$6,$7)',
      [id, action, status, body, c.get('user')?.id ?? null, qa.name, qa.id]);
  const audit = (client, c, action, entity) =>
    client.query('INSERT INTO vault_audit(actor_id,action,entity) VALUES($1,$2,$3)', [c.get('user')?.id ?? null, action, entity]);

  /* ---- sign-in */
  app.get(`${base}/me`, route(async c => c.json({ qa: await qaOf(c) })));

  app.post(`${base}/login`, route(async c => {
    const body = await c.req.json().catch(() => ({}));
    const username = typeof body?.username === 'string' ? body.username.trim().toLowerCase() : '';
    const key = `${c.req.header('x-vault-client-ip') || ''}|${username}`, held = failures.get(key);
    if (held && held.until > now()) throw new HttpError(429, 'Too many wrong passwords. Wait a minute and try again.');
    const row = USERNAME.test(username)
      ? (await pool.query('SELECT id,username,display_name,is_admin,session_stamp,password_hash,deactivated_at FROM yts_qa_users WHERE username=$1', [username])).rows[0]
      : undefined;
    const ok = await checkPassword(typeof body?.password === 'string' ? body.password : '', row?.password_hash || 'scrypt$16384$8$1$AAAAAAAAAAAAAAAAAAAAAA==$AAAA');
    if (!row || !ok || row.deactivated_at) {
      const count = (held && held.until > now() ? 0 : held?.count || 0) + 1;
      failures.set(key, { count: count >= 5 ? 0 : count, until: count >= 5 ? now() + 60_000 : 0 });
      throw new HttpError(401, row?.deactivated_at && ok ? 'This Research YTS account is deactivated. Ask the admin.' : 'Wrong username or password.');
    }
    failures.delete(key);
    await pool.query('UPDATE yts_qa_users SET last_login_at=now() WHERE id=$1', [row.id]);
    await signIn(c, row);
    return c.json({ qa: publicQa(row) });
  }));

  app.post(`${base}/logout`, route(async c => { deleteCookie(c, COOKIE_NAME, { path: base, secure }); return c.json({ ok: true }); }));

  app.post(`${base}/password`, route(async c => {
    const qa = await need(c), body = await c.req.json();
    const next = cleanPassword(body?.next, 'new password');
    const row = (await pool.query('SELECT password_hash FROM yts_qa_users WHERE id=$1', [qa.id])).rows[0];
    if (!(await checkPassword(typeof body?.current === 'string' ? body.current : '', row.password_hash))) throw bad('Your current password is wrong.');
    const updated = (await pool.query('UPDATE yts_qa_users SET password_hash=$2,session_stamp=session_stamp+1,updated_at=now() WHERE id=$1 RETURNING id,session_stamp', [qa.id, await hashPassword(next)])).rows[0];
    await signIn(c, updated); // other browsers signed in as this QA are signed out
    return c.json({ ok: true });
  }));

  /* ---- QA accounts (Research YTS admins only) */
  const accounts = async () => (await pool.query(`SELECT u.id,u.username,u.display_name,u.is_admin,u.created_at,u.last_login_at,u.deactivated_at,
      (SELECT count(*)::int FROM yts_issues i WHERE i.qa_user_id=u.id AND i.deleted_at IS NULL) AS issues,
      (SELECT count(*)::int FROM yts_issues i WHERE i.qa_user_id=u.id AND i.deleted_at IS NULL AND i.status IN ('open','in_progress')) AS open
    FROM yts_qa_users u ORDER BY u.deactivated_at IS NOT NULL, u.is_admin DESC, lower(u.display_name)`)).rows
    .map(r => ({ id: Number(r.id), username: r.username, name: r.display_name, admin: r.is_admin, created_at: r.created_at, last_login_at: r.last_login_at, deactivated_at: r.deactivated_at, issues: r.issues, open: r.open }));

  app.get(`${base}/qa-users`, route(async c => { await needAdmin(c); return c.json(await accounts()); }));

  app.post(`${base}/qa-users`, route(async c => {
    await needAdmin(c);
    const body = await c.req.json();
    const username = cleanUsername(body?.username), name = cleanDisplayName(body?.name), password = cleanPassword(body?.password);
    const admin = body?.admin === true;
    const added = await tx(async client => {
      const row = (await client.query(`INSERT INTO yts_qa_users(username,display_name,password_hash,is_admin) VALUES($1,$2,$3,$4)
        ON CONFLICT (username) DO NOTHING RETURNING id`, [username, name, await hashPassword(password), admin])).rows[0];
      if (!row) throw new HttpError(409, `The username "${username}" is already taken.`);
      await audit(client, c, 'yts_qa.created', username);
      return row;
    });
    return c.json((await accounts()).find(a => a.id === Number(added.id)), 201);
  }));

  app.post(`${base}/qa-users/:id/password`, route(async c => {
    const admin = await needAdmin(c), id = idParam(c, 'account'), body = await c.req.json();
    const password = cleanPassword(body?.password);
    const row = (await pool.query('UPDATE yts_qa_users SET password_hash=$2,session_stamp=session_stamp+1,updated_at=now() WHERE id=$1 RETURNING id,username,session_stamp', [id, await hashPassword(password)])).rows[0];
    if (!row) throw new HttpError(404, 'No such account.');
    await audit(pool, c, 'yts_qa.password_reset', row.username);
    if (Number(row.id) === admin.id) await signIn(c, row); // resetting your own password keeps you signed in here
    return c.json({ ok: true });
  }));

  const setActive = active => route(async c => {
    const admin = await needAdmin(c), id = idParam(c, 'account');
    if (!active && id === admin.id) throw bad('You cannot deactivate your own account.');
    await tx(async client => {
      const row = (await client.query('SELECT username,is_admin,deactivated_at FROM yts_qa_users WHERE id=$1 FOR UPDATE', [id])).rows[0];
      if (!row) throw new HttpError(404, 'No such account.');
      if (!active && row.is_admin) {
        const others = (await client.query('SELECT count(*)::int AS n FROM yts_qa_users WHERE is_admin AND deactivated_at IS NULL AND id<>$1', [id])).rows[0].n;
        if (!others) throw bad('Keep at least one active Research YTS admin.');
      }
      await client.query(active
        ? 'UPDATE yts_qa_users SET deactivated_at=NULL,updated_at=now() WHERE id=$1'
        : 'UPDATE yts_qa_users SET deactivated_at=COALESCE(deactivated_at,now()),session_stamp=session_stamp+1,updated_at=now() WHERE id=$1', [id]);
      await audit(client, c, active ? 'yts_qa.reactivated' : 'yts_qa.deactivated', row.username);
    });
    return c.json((await accounts()).find(a => a.id === id));
  });
  app.post(`${base}/qa-users/:id/deactivate`, setActive(false));
  app.post(`${base}/qa-users/:id/reactivate`, setActive(true));

  /* ---- issues */
  app.get(`${base}/options`, c => c.json({ categories: CATEGORIES, severities: SEVERITIES, statuses: STATUSES }));

  // Counts per collection and per strategy, for the side list and the strategy picker (a QA's own issues; all for an admin).
  app.get(`${base}/summary`, route(async c => {
    const qa = await need(c);
    const rows = (await pool.query(`SELECT collection,strategy_id,strategies,collections,status FROM yts_issues
      WHERE deleted_at IS NULL AND ($1::bigint IS NULL OR qa_user_id=$1)`, [qa.admin ? null : qa.id])).rows;
    const blank = () => ({ open: 0, in_progress: 0, fixed: 0, not_issue: 0 });
    const totals = blank(), collections = {}, strategies = {};
    // A general issue counts once in the totals and under "general", and also under each collection and strategy it names.
    for (const r of rows) {
      totals[r.status] += 1;
      for (const book of new Set([r.collection, ...r.collections])) (collections[book] ??= blank())[r.status] += 1;
      for (const id of new Set([r.strategy_id, ...r.strategies].filter(Boolean))) (strategies[id] ??= blank())[r.status] += 1;
    }
    return c.json({ totals, collections, strategies });
  }));

  app.get(`${base}/issues`, route(async c => {
    const qa = await need(c), q = c.req.query(), paging = pageParams(q), where = [], args = [];
    const add = (sql, value) => { args.push(value); where.push(sql.replaceAll('?', `$${args.length}`)); };
    if (q.deleted !== '1') where.push('i.deleted_at IS NULL');
    if (!qa.admin) add('i.qa_user_id=?', qa.id);
    else if (q.reporter) { const r = Number.parseInt(q.reporter, 10); if (!Number.isSafeInteger(r) || r < 1) throw bad('Unknown reporter.'); add('i.qa_user_id=?', r); }
    if (q.collection === GENERAL) add('i.collection=?', GENERAL);
    else if (q.collection) { if (!COLLECTION.test(q.collection)) throw bad('Unknown collection.'); add('(i.collection=? OR ?=ANY(i.collections))', q.collection); }
    if (q.strategy) { if (!STRATEGY.test(q.strategy)) throw bad('Unknown strategy.'); add('(i.strategy_id=? OR ?=ANY(i.strategies))', q.strategy); }
    if (q.status) {
      const list = q.status.split(',');
      if (!list.every(s => Object.hasOwn(STATUSES, s))) throw bad('Unknown status.');
      add('i.status = ANY(?)', list);
    }
    if (q.category) add('i.category=?', pick(CATEGORIES, q.category, 'category'));
    if (q.q) add(`(i.title ILIKE ? ESCAPE '\\' OR i.body ILIKE ? ESCAPE '\\' OR i.reporter_name ILIKE ? ESCAPE '\\' OR i.instrument ILIKE ? ESCAPE '\\')`,
      `%${cleanText(q.q, { field: 'search', max: 100, multiline: false }).replace(/[\\%_]/g, m => '\\' + m)}%`);
    const filter = where.length ? `WHERE ${where.join(' AND ')}` : '';
    const total = (await pool.query(`SELECT count(*)::int AS n FROM yts_issues i ${filter}`, args)).rows[0].n;
    const rows = (await pool.query(`SELECT ${COLUMNS} FROM yts_issues i ${filter}
      ORDER BY CASE i.status WHEN 'open' THEN 0 WHEN 'in_progress' THEN 1 ELSE 2 END, i.updated_at DESC, i.id DESC
      LIMIT $${args.length + 1} OFFSET $${args.length + 2}`, [...args, paging.size, paging.offset])).rows;
    return c.json({ items: rows.map(r => shape(r, qa)), total, page: paging.page, size: paging.size, pages: Math.max(1, Math.ceil(total / paging.size)) });
  }));

  app.get(`${base}/issues/:id`, route(async c => {
    const qa = await need(c), id = idParam(c, 'issue'), issue = await one(id, qa);
    const events = (await pool.query(`SELECT id,action,status,body,actor_name,created_at FROM yts_issue_events
      WHERE issue_id=$1 ORDER BY created_at, id`, [id])).rows;
    return c.json({ issue, events, files: await files(id, qa, issue) });
  }));

  app.post(`${base}/issues`, route(async c => {
    const qa = await need(c), data = validateIssue(await c.req.json());
    const issue = await tx(async client => {
      const id = (await client.query(`INSERT INTO yts_issues(collection,strategy_id,category,severity,title,body,instrument,period_from,period_to,page_link,reporter_name,author_id,qa_user_id,strategies,collections)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15) RETURNING id`,
      [data.collection, data.strategy_id, data.category, data.severity, data.title, data.body, data.instrument, data.period_from, data.period_to, data.page_link, qa.name, c.get('user')?.id ?? null, qa.id, data.strategies, data.collections])).rows[0].id;
      await event(client, id, 'created', c, qa, { status: 'open', body: data.body });
      await audit(client, c, 'yts_issue.created', String(id));
      return one(id, qa, client);
    });
    return c.json(issue, 201);
  }));

  app.patch(`${base}/issues/:id`, route(async c => {
    const qa = await need(c), id = idParam(c, 'issue'), data = validateEdit(await c.req.json());
    const issue = await tx(async client => {
      const row = await live(client, id, qa);
      if (data.strategies && row.collection !== GENERAL) throw bad('Only a general issue lists several strategies.');
      const keys = Object.keys(data);
      await client.query(`UPDATE yts_issues SET ${keys.map((k, i) => `${k}=$${i + 2}`).join(',')},updated_at=now() WHERE id=$1`, [id, ...keys.map(k => data[k])]);
      await event(client, id, 'edited', c, qa, { body: keys.filter(k => k !== 'collections').map(k => `${k}: ${Array.isArray(data[k]) ? data[k].join(', ') || 'all YT strategies' : data[k]}`).join('\n').slice(0, 10000) });
      return one(id, qa, client);
    });
    return c.json(issue);
  }));

  // The reporter (or an admin) moves the issue along: an admin marks it fixed, the QA reopens it if it is not.
  app.put(`${base}/issues/:id/status`, route(async c => {
    const qa = await need(c), id = idParam(c, 'issue'), body = await c.req.json();
    const status = pick(STATUSES, body?.status, 'status');
    const note = body?.note ? cleanText(body.note, { field: 'note', max: 4000 }) : '';
    const issue = await tx(async client => {
      const row = await live(client, id, qa);
      if (row.status === status) throw bad(`The issue is already "${STATUSES[status]}".`);
      await client.query('UPDATE yts_issues SET status=$2,updated_at=now() WHERE id=$1', [id, status]);
      await event(client, id, 'status', c, qa, { status, body: note });
      return one(id, qa, client);
    });
    return c.json(issue);
  }));

  app.post(`${base}/issues/:id/comments`, route(async c => {
    const qa = await need(c), id = idParam(c, 'issue'), body = await c.req.json();
    const text = cleanText(body?.body, { field: 'comment', max: 4000 });
    const issue = await tx(async client => {
      await live(client, id, qa);
      await client.query('UPDATE yts_issues SET updated_at=now() WHERE id=$1', [id]);
      await event(client, id, 'comment', c, qa, { body: text });
      return one(id, qa, client);
    });
    return c.json(issue, 201);
  }));

  /* ---- photos and files */
  // Listed with the issue; the bytes are fetched one file at a time.
  const files = async (id, qa, issue) => (await pool.query(`SELECT id,name,content_type,size,uploader_name,qa_user_id,created_at FROM yts_attachments
      WHERE issue_id=$1 AND deleted_at IS NULL ORDER BY created_at, id`, [id])).rows
    .map(f => ({ id: Number(f.id), name: f.name, content_type: f.content_type, size: f.size, image: INLINE_IMAGES.has(f.content_type), uploader_name: f.uploader_name, created_at: f.created_at,
      can_delete: !issue.deleted_at && (qa.admin || Number(f.qa_user_id) === qa.id || issue.reporter_id === qa.id) }));
  const fileParam = c => {
    const raw = c.req.param('file'), id = Number.parseInt(raw, 10);
    if (!Number.isSafeInteger(id) || id < 1 || String(id) !== raw) throw new HttpError(404, 'No such file.');
    return id;
  };

  // One file per request: the body is the file itself, its name is in ?name=.
  app.post(`${base}/issues/:id/files`, route(async c => {
    const qa = await need(c), id = idParam(c, 'issue'), name = cleanFileName(c.req.query('name'));
    const type = cleanContentType(c.req.header('content-type'));
    const data = Buffer.from(await c.req.arrayBuffer());
    if (!data.length) throw bad('The file is empty.');
    if (data.length > MAX_FILE_BYTES) throw new HttpError(413, `Files can be up to ${MAX_FILE_BYTES / 1024 / 1024} MB.`);
    const issue = await tx(async client => {
      await live(client, id, qa);
      const n = (await client.query('SELECT count(*)::int AS n FROM yts_attachments WHERE issue_id=$1 AND deleted_at IS NULL', [id])).rows[0].n;
      if (n >= MAX_FILES) throw bad(`An issue can have up to ${MAX_FILES} files. Remove one first.`);
      await client.query('INSERT INTO yts_attachments(issue_id,name,content_type,size,data,uploader_name,qa_user_id) VALUES($1,$2,$3,$4,$5,$6,$7)',
        [id, name, type, data.length, data, qa.name, qa.id]);
      await client.query('UPDATE yts_issues SET updated_at=now() WHERE id=$1', [id]);
      await event(client, id, 'file', c, qa, { body: `Added ${name}` });
      return one(id, qa, client);
    });
    return c.json({ issue, files: await files(id, qa, issue) }, 201);
  }));

  app.get(`${base}/issues/:id/files/:file`, route(async c => {
    const qa = await need(c), id = idParam(c, 'issue'), fid = fileParam(c);
    await one(id, qa); // the same people who can see the issue
    const f = (await pool.query('SELECT name,content_type,data FROM yts_attachments WHERE id=$1 AND issue_id=$2 AND deleted_at IS NULL', [fid, id])).rows[0];
    if (!f) throw new HttpError(404, 'No such file.');
    const image = INLINE_IMAGES.has(f.content_type);
    const ascii = f.name.replace(/[^\x20-\x7e]/g, '_').replace(/["\\]/g, '_');
    return c.body(f.data, 200, {
      'Content-Type': image ? f.content_type : 'application/octet-stream',
      'Content-Disposition': `${image && c.req.query('download') !== '1' ? 'inline' : 'attachment'}; filename="${ascii}"; filename*=UTF-8''${encodeURIComponent(f.name)}`,
      'Cache-Control': 'private, no-store',
    });
  }));

  app.delete(`${base}/issues/:id/files/:file`, route(async c => {
    const qa = await need(c), id = idParam(c, 'issue'), fid = fileParam(c);
    const issue = await tx(async client => {
      const row = await live(client, id, qa);
      const f = (await client.query('SELECT name,qa_user_id FROM yts_attachments WHERE id=$1 AND issue_id=$2 AND deleted_at IS NULL FOR UPDATE', [fid, id])).rows[0];
      if (!f) throw new HttpError(404, 'No such file.');
      if (!qa.admin && Number(f.qa_user_id) !== qa.id && Number(row.qa_user_id) !== qa.id) throw new HttpError(403, 'Only the person who added the file, the reporter or an admin can remove it.');
      await client.query('UPDATE yts_attachments SET deleted_at=now(),deleted_by=$2 WHERE id=$1', [fid, qa.id]);
      await client.query('UPDATE yts_issues SET updated_at=now() WHERE id=$1', [id]);
      await event(client, id, 'file', c, qa, { body: `Removed ${f.name}` });
      return one(id, qa, client);
    });
    return c.json({ issue, files: await files(id, qa, issue) });
  }));

  app.delete(`${base}/issues/:id`, route(async c => {
    const qa = await need(c), id = idParam(c, 'issue'), body = await c.req.json().catch(() => ({}));
    const reason = cleanText(body?.reason, { field: 'reason for deleting', min: 3, max: 500, multiline: false });
    const issue = await tx(async client => {
      await live(client, id, qa);
      await client.query('UPDATE yts_issues SET deleted_at=now(),deleted_by=$2,deleted_reason=$3,updated_at=now() WHERE id=$1', [id, c.get('user')?.id ?? null, reason]);
      await event(client, id, 'deleted', c, qa, { body: reason });
      await audit(client, c, 'yts_issue.deleted', String(id));
      return one(id, qa, client);
    });
    return c.json(issue);
  }));
}
