// Custom date filter sets (Base 1 / Base 2 style windows): personal sets are owner-only; shared sets are uploaded by an
// admin and visible to everyone. A strategy carries the windows in its date_filter; these rows are the library they come from.
import { roleAllows, LabRequestError, validateLabDateSet, validateLabVersion } from './contracts.mjs';

const BASE = '/api/app/lab/date-sets';
const ID = ':id{[1-9][0-9]{0,8}}';
const COLUMNS = 'id, owner, shared, name, windows, version, created_at AS "createdAt", updated_at AS "updatedAt"';
const DUPLICATE = {
  lab_date_set_owner_name_idx: 'You already have a date filter with this name; choose another.',
  lab_date_set_shared_name_idx: 'A shared date filter with this name already exists; choose another.',
};

async function body(c) {
  const data = await c.req.json().catch(() => null);
  if (!data || typeof data !== 'object' || Array.isArray(data)) throw new LabRequestError('Provide a JSON object.');
  return data;
}
// A shared set is changed by any admin; a personal set only by its owner (with write access).
const canEdit = (user, row) => (row.shared ? roleAllows(user.role, 'admin') : row.owner === user.id && roleAllows(user.role, 'write'));
const visible = (user, row) => row.shared || row.owner === user.id;

export function registerLabDateSets(app, pool) {
  const view = user => ({ owner, ...row }) => ({ ...row, mine: owner === user.id, editable: canEdit(user, { ...row, owner }) });
  const route = handler => async c => {
    try { return await handler(c, c.get('user')); }
    catch (error) {
      if (error instanceof LabRequestError) return c.json({ error: error.message }, error.status);
      if (error.code === '23505' && DUPLICATE[error.constraint]) return c.json({ error: DUPLICATE[error.constraint] }, 409);
      throw error;
    }
  };
  const audit = (client, user, action, id) => client.query('INSERT INTO vault_audit (actor_id,action,entity) VALUES ($1,$2,$3)', [user.id, `lab.date_set.${action}`, `lab_date_set:${id}`]);
  const transaction = async work => {
    const client = await pool.connect();
    try {
      await client.query('BEGIN');
      const result = await work(client);
      await client.query('COMMIT');
      return result;
    } catch (error) { await client.query('ROLLBACK'); throw error; } finally { client.release(); }
  };
  // Locks the row so concurrent saves and deletes apply one at a time against the version the user opened.
  const locked = (id, user, version, work) => transaction(async client => {
    const row = (await client.query(`SELECT ${COLUMNS} FROM lab_date_set WHERE id=$1 FOR UPDATE`, [id])).rows[0];
    if (!row || !visible(user, row)) throw new LabRequestError('Date filter not found.', 404);
    if (!canEdit(user, row)) throw new LabRequestError(row.shared ? 'Only an admin can change a shared date filter.' : 'Researcher access required', 403);
    if (version !== row.version) throw new LabRequestError(`This date filter changed since you opened it (now version ${row.version}). Reload before saving.`, 409);
    return work(client, row);
  });

  app.get(BASE, route(async (c, user) => {
    const result = await pool.query(`SELECT ${COLUMNS} FROM lab_date_set WHERE owner=$1 OR shared ORDER BY shared DESC, lower(name), id`, [user.id]);
    return c.json(result.rows.map(view(user)));
  }));
  app.post(BASE, route(async (c, user) => {
    if (!roleAllows(user.role, 'write')) throw new LabRequestError('Researcher access required', 403);
    const data = validateLabDateSet(await body(c));
    if (data.shared && !roleAllows(user.role, 'admin')) throw new LabRequestError('Only an admin can share a date filter with everyone.', 403);
    return c.json(await transaction(async client => {
      const row = (await client.query(`INSERT INTO lab_date_set (owner,shared,name,windows) VALUES ($1,$2,$3,$4::jsonb) RETURNING ${COLUMNS}`, [user.id, data.shared, data.name, JSON.stringify(data.windows)])).rows[0];
      await audit(client, user, 'create', row.id);
      return view(user)(row);
    }), 201);
  }));
  // PUT replaces the name and the windows (a re-upload); whether a set is shared does not change.
  app.put(`${BASE}/${ID}`, route(async (c, user) => {
    const input = await body(c);
    const version = validateLabVersion(input.version), data = validateLabDateSet(input);
    return c.json(await locked(Number(c.req.param('id')), user, version, async (client, row) => {
      if (input.shared !== undefined && input.shared !== row.shared) throw new LabRequestError('Sharing cannot be changed; create a new date filter instead.');
      const saved = (await client.query(`UPDATE lab_date_set SET name=$2, windows=$3::jsonb, version=version+1, updated_at=now() WHERE id=$1 RETURNING ${COLUMNS}`, [row.id, data.name, JSON.stringify(data.windows)])).rows[0];
      await audit(client, user, 'update', row.id);
      return view(user)(saved);
    }));
  }));
  // DELETE carries the version in the query string (?version=N): proxies may drop DELETE bodies.
  app.delete(`${BASE}/${ID}`, route(async (c, user) => {
    const version = validateLabVersion(Number(c.req.query('version') ?? NaN));
    await locked(Number(c.req.param('id')), user, version, async (client, row) => {
      await client.query('DELETE FROM lab_date_set WHERE id=$1', [row.id]);
      await audit(client, user, 'delete', row.id);
    });
    return c.body(null, 204);
  }));
}
