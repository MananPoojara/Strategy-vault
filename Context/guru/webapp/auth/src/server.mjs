import { Hono } from 'hono';
import { serve } from '@hono/node-server';
import { bodyLimit } from 'hono/body-limit';
import { timingSafeEqual } from 'node:crypto';
import { auth, pool, allowedOrigins } from './config.mjs';
import { bootstrap } from './bootstrap.mjs';
import { validateNoteKey, validateTheme, roleAllows, ROLES, validateSavedView, validateAnnotation } from './contracts.mjs';
import { createBookLibrary, registerBookLibrary } from './book-library.mjs';
import { registerLabStrategies } from './lab-strategies.mjs';

await bootstrap();
export const app = new Hono();
app.use('*', bodyLimit({ maxSize: 1024 * 1024, onError: c => c.json({ error: 'Request too large' }, 413) }));
app.use('*', async (c, next) => {
  c.header('X-Content-Type-Options', 'nosniff');
  c.header('Cache-Control', 'no-store');
  await next();
});
app.get('/healthz', async c => { await pool.query('SELECT 1'); return c.json({ status: 'ok' }); });
// Account creation goes through administrator tools; never expose public signup.
app.all('/api/auth/sign-up/*', c => c.json({ error: 'Ask your administrator for an account.' }, 403));
app.all('/api/auth/*', c => auth.handler(c.req.raw));
app.get('/internal/session', async c => {
  const supplied = Buffer.from(c.req.header('x-vault-internal-token') || '');
  const expected = Buffer.from(process.env.VAULT_INTERNAL_TOKEN);
  if (supplied.length !== expected.length || !timingSafeEqual(supplied, expected)) return c.json({ error: 'Unauthorized' }, 401);
  const session = await auth.api.getSession({ headers: c.req.raw.headers });
  if (!session) return c.json({ error: 'Unauthorized' }, 401);
  return c.json({ user: { id: session.user.id, name: session.user.name, email: session.user.email, role: session.user.role } });
});
app.use('/api/app/*', async (c, next) => {
  const session = await auth.api.getSession({ headers: c.req.raw.headers });
  if (!session || !roleAllows(session.user.role, 'read')) return c.json({ error: 'Please sign in.' }, 401);
  c.set('user', session.user);
  if (!['GET', 'HEAD'].includes(c.req.method)) {
    if (!allowedOrigins.has(c.req.header('origin'))) return c.json({ error: 'Invalid request origin' }, 403);
  }
  await next();
});
registerBookLibrary(app, await createBookLibrary());
registerLabStrategies(app, pool);
app.get('/api/app/me', async c => {
  const user = c.get('user');
  const settings = await pool.query('SELECT theme,last_view FROM vault_settings WHERE user_id = $1', [user.id]);
  return c.json({ user: { id: user.id, name: user.name, email: user.email, role: user.role }, settings: { theme: settings.rows[0]?.theme || 'aqua', lastView: settings.rows[0]?.last_view || '#/' } });
});
app.put('/api/app/settings', async c => {
  const { theme, lastView } = await c.req.json();
  if (theme === undefined && lastView === undefined) return c.json({ error: 'Provide a theme or saved view.' },400);
  if (theme !== undefined) validateTheme(theme);
  if (lastView !== undefined) validateSavedView(lastView);
  await pool.query("INSERT INTO vault_settings (user_id,theme,last_view) VALUES ($1,COALESCE($2,'aqua'),COALESCE($3,'#/')) ON CONFLICT (user_id) DO UPDATE SET theme=COALESCE($2,vault_settings.theme), last_view=COALESCE($3,vault_settings.last_view), updated_at=now()", [c.get('user').id, theme ?? null, lastView ?? null]);
  return c.json({ ...(theme !== undefined ? {theme} : {}), ...(lastView !== undefined ? {lastView} : {}) });
});
app.get('/api/app/notes', async c => {
  const key = validateNoteKey(c.req.query('key'));
  const result = await pool.query('SELECT content FROM vault_notes WHERE note_key=$1', [key]);
  return c.text(result.rows[0]?.content || '');
});
app.post('/api/app/notes', async c => {
  const user = c.get('user');
  if (!roleAllows(user.role,'write')) return c.json({ error: 'Researcher access required' },403);
  const key = validateNoteKey(c.req.query('key'));
  const content = await c.req.text();
  const client = await pool.connect();
  try {
    await client.query('BEGIN');
    await client.query('INSERT INTO vault_notes (note_key,content,author_id) VALUES ($1,$2,$3) ON CONFLICT (note_key) DO UPDATE SET content=EXCLUDED.content, author_id=EXCLUDED.author_id, updated_at=now()', [key,content,user.id]);
    await client.query('INSERT INTO vault_note_history (note_key,content,author_id) VALUES ($1,$2,$3)', [key,content,user.id]);
    await client.query('INSERT INTO vault_audit (actor_id,action,entity) VALUES ($1,$2,$3)', [user.id,'note.saved',key]);
    await client.query('COMMIT');
  } catch (error) { await client.query('ROLLBACK'); throw error; } finally { client.release(); }
  return c.text('saved');
});
app.get('/api/app/annotations', async c => {
  const key = validateNoteKey(c.req.query('key'));
  const result = await pool.query('SELECT tags,ceo,updated_at FROM vault_annotations WHERE entity_key=$1', [key]);
  return c.json(result.rows[0] || {tags:[],ceo:false});
});
app.put('/api/app/annotations', async c => {
  const user=c.get('user');
  if(!roleAllows(user.role,'write'))return c.json({error:'Researcher access required'},403);
  const key=validateNoteKey(c.req.query('key')),body=await c.req.json();
  if(body.ceo!==undefined&&!roleAllows(user.role,'admin'))return c.json({error:'Administrator access required for CEO recommendations'},403);
  const data=validateAnnotation(body,user.role),client=await pool.connect();
  try {
    await client.query('BEGIN');
    const result=await client.query(`INSERT INTO vault_annotations(entity_key,tags,ceo,ceo_author_id,author_id) VALUES($1,$2::jsonb,COALESCE($3,false),CASE WHEN $3::boolean IS NOT NULL THEN $4 ELSE NULL END,$4) ON CONFLICT(entity_key) DO UPDATE SET tags=EXCLUDED.tags, ceo=COALESCE($3,vault_annotations.ceo), ceo_author_id=CASE WHEN $3::boolean IS NOT NULL THEN $4 ELSE vault_annotations.ceo_author_id END, author_id=$4, updated_at=now() RETURNING tags,ceo,updated_at`,[key,JSON.stringify(data.tags),data.ceo??null,user.id]);
    await client.query('INSERT INTO vault_audit(actor_id,action,entity) VALUES($1,$2,$3)',[user.id,'annotation.saved',key]);
    await client.query('COMMIT');return c.json(result.rows[0]);
  } catch(error){await client.query('ROLLBACK');throw error;}finally{client.release();}
});
app.get('/api/app/admin/users', async c => {
  if (!roleAllows(c.get('user').role,'admin')) return c.json({ error: 'Administrator access required' },403);
  const result = await pool.query('SELECT id,name,email,role FROM "user" ORDER BY name,id LIMIT 500');
  return c.json(result.rows);
});
app.post('/api/app/admin/users', async c => {
  const user = c.get('user');
  if (!roleAllows(user.role,'admin')) return c.json({ error: 'Administrator access required' },403);
  const body = await c.req.json();
  if (typeof body.name !== 'string' || body.name.trim().length < 1 || body.name.length > 100 || typeof body.email !== 'string' || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(body.email) || typeof body.password !== 'string' || body.password.length < 12 || body.password.length > 128 || !ROLES.has(body.role)) return c.json({ error: 'Provide a name, valid email, role and password of 12–128 characters.' },400);
  const email = body.email.trim().toLowerCase();
  if ((await pool.query('SELECT id FROM "user" WHERE email=$1',[email])).rowCount) return c.json({ error: 'An account with this email already exists.' },409);
  const created = await auth.api.signUpEmail({ body: { name: body.name.trim(), email, password: body.password } });
  await pool.query('UPDATE "user" SET role=$1, "emailVerified"=true WHERE id=$2',[body.role,created.user.id]);
  await pool.query('INSERT INTO vault_audit (actor_id,action,entity) VALUES ($1,$2,$3)',[user.id,'user.created',created.user.id]);
  return c.json({ id: created.user.id, name: created.user.name, email, role: body.role },201);
});
app.onError((error,c) => {
  if (error instanceof SyntaxError || /Invalid (note identifier|theme|saved view|annotation|recommendation permission)/.test(error.message)) return c.json({ error: 'Invalid request' },400);
  console.error('Application request failed:', error.code || error.name);
  return c.json({ error: 'Request failed. Please try again.' },500);
});
const server = serve({ fetch: app.fetch, hostname: process.env.AUTH_BIND || '127.0.0.1', port: Number(process.env.AUTH_PORT || 8102) });
console.info('Strategy Vault authentication and application data service ready.');
for (const signal of ['SIGTERM','SIGINT']) process.on(signal, () => server.close(async () => { await pool.end(); process.exit(0); }));
