// Saved custom indicators (spec 2026-10-19 §3): owner-only formulas; the engine is the authority on formula syntax.
import { roleAllows, LabRequestError, validateLabIndicator, validateLabVersion } from './contracts.mjs';

const BASE = '/api/app/lab/indicators';
const ID = ':id{[1-9][0-9]{0,8}}';
const COLUMNS = 'id, owner, name, formula, blocks, version, created_at AS "createdAt", updated_at AS "updatedAt"';
const view = ({ owner, ...row }) => row;
const json = blocks => (blocks === null ? null : JSON.stringify(blocks));

async function body(c) {
  const data = await c.req.json().catch(() => null);
  if (!data || typeof data !== 'object' || Array.isArray(data)) throw new LabRequestError('Provide a JSON object.');
  return data;
}

export function registerLabIndicators(app, pool) {
  const route = handler => async c => {
    try { return await handler(c, c.get('user')); }
    catch (error) {
      if (error instanceof LabRequestError) return c.json({ error: error.message }, error.status);
      if (error.code === '23505' && error.constraint === 'lab_indicator_owner_name_idx') return c.json({ error: 'You already have an indicator with this name; choose another.' }, 409);
      throw error;
    }
  };
  const writer = user => { if (!roleAllows(user.role, 'write')) throw new LabRequestError('Researcher access required', 403); };
  const audit = (client, user, action, id) => client.query('INSERT INTO vault_audit (actor_id,action,entity) VALUES ($1,$2,$3)', [user.id, `lab.indicator.${action}`, `lab_indicator:${id}`]);
  const transaction = async work => {
    const client = await pool.connect();
    try {
      await client.query('BEGIN');
      const result = await work(client);
      await client.query('COMMIT');
      return result;
    } catch (error) { await client.query('ROLLBACK'); throw error; } finally { client.release(); }
  };
  // Locks the owner's row so concurrent saves and deletes apply one at a time against the version the user opened.
  const locked = (id, user, version, work) => transaction(async client => {
    const row = (await client.query(`SELECT ${COLUMNS} FROM lab_indicator WHERE id=$1 FOR UPDATE`, [id])).rows[0];
    if (!row || row.owner !== user.id) throw new LabRequestError('Indicator not found.', 404);
    writer(user);
    if (version !== row.version) throw new LabRequestError(`This indicator changed since you opened it (now version ${row.version}). Reload before saving.`, 409);
    return work(client, row);
  });

  app.get(BASE, route(async (c, user) => {
    const result = await pool.query(`SELECT ${COLUMNS} FROM lab_indicator WHERE owner=$1 ORDER BY lower(name), id`, [user.id]);
    return c.json(result.rows.map(view));
  }));
  app.post(BASE, route(async (c, user) => {
    writer(user);
    const data = validateLabIndicator(await body(c));
    return c.json(await transaction(async client => {
      const row = (await client.query(`INSERT INTO lab_indicator (owner,name,formula,blocks) VALUES ($1,$2,$3,$4::jsonb) RETURNING ${COLUMNS}`, [user.id, data.name, data.formula, json(data.blocks)])).rows[0];
      await audit(client, user, 'create', row.id);
      return view(row);
    }), 201);
  }));
  // PUT replaces the row: omitting blocks clears them (a text edit makes the old block layout stale).
  app.put(`${BASE}/${ID}`, route(async (c, user) => {
    const input = await body(c);
    const version = validateLabVersion(input.version), data = validateLabIndicator(input);
    return c.json(await locked(Number(c.req.param('id')), user, version, async (client, row) => {
      const saved = (await client.query(`UPDATE lab_indicator SET name=$2, formula=$3, blocks=$4::jsonb, version=version+1, updated_at=now() WHERE id=$1 RETURNING ${COLUMNS}`, [row.id, data.name, data.formula, json(data.blocks)])).rows[0];
      await audit(client, user, 'update', row.id);
      return view(saved);
    }));
  }));
  // DELETE carries the version in the query string (?version=N): proxies may drop DELETE bodies.
  app.delete(`${BASE}/${ID}`, route(async (c, user) => {
    const version = validateLabVersion(Number(c.req.query('version') ?? NaN));
    await locked(Number(c.req.param('id')), user, version, async (client, row) => {
      await client.query('DELETE FROM lab_indicator WHERE id=$1', [row.id]);
      await audit(client, user, 'delete', row.id);
    });
    return c.body(null, 204);
  }));
}
