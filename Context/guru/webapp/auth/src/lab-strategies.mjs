// Saved indicators-lab strategies (spec §3): per-user drafts, admin review, frozen YT Strategies snapshots.
import { roleAllows, LabRequestError, labPublishable, LAB_STATUSES, labAction, labCanRead, validateLabStrategy, sameLabOrigin, validateLabVersion, validateLabNote } from './contracts.mjs';

const BASE = '/api/app/lab/strategies';
const ID = ':id{[1-9][0-9]{0,8}}';
const COLUMNS = `s.id, s.owner, u.name AS "ownerName", s.name, s.spec, s.origin, s.market, s.instrument, s.status, s.version,
  s.submitted_at AS "submittedAt", s.reviewed_at AS "reviewedAt", s.review_note AS "reviewNote", s.created_at AS "createdAt", s.updated_at AS "updatedAt"`;
const FROM = 'FROM lab_strategy s JOIN "user" u ON u.id = s.owner';
const TRANSITIONS = {
  submit: "status='submitted', submitted_at=now(), reviewed_by=NULL, reviewed_at=NULL, review_note=NULL", // a resubmission must not show the old rejection note
  withdraw: "status='draft', submitted_at=NULL",
  publish: "status='published', reviewed_by=$2, reviewed_at=now(), review_note=$3",
  reject: "status='rejected', reviewed_by=$2, reviewed_at=now(), review_note=$3",
};
const view = row => row && { ...row, publishable: labPublishable(row) };

async function body(c) {
  const data = await c.req.json().catch(() => null);
  if (!data || typeof data !== 'object' || Array.isArray(data)) throw new LabRequestError('Provide a JSON object.');
  return data;
}

export function registerLabStrategies(app, pool) {
  const route = handler => async c => {
    try { return await handler(c, c.get('user')); }
    catch (error) { if (error instanceof LabRequestError) return c.json({ error: error.message }, error.status); throw error; }
  };
  // Locks the row so concurrent edits, transitions and reviews apply one at a time.
  const locked = async (id, work) => {
    const client = await pool.connect();
    try {
      await client.query('BEGIN');
      const row = (await client.query(`SELECT ${COLUMNS} ${FROM} WHERE s.id=$1 FOR UPDATE OF s`, [id])).rows[0];
      const result = await work(client, row);
      await client.query('COMMIT');
      return result;
    } catch (error) { await client.query('ROLLBACK'); throw error; } finally { client.release(); }
  };
  const reread = async (client, id) => view((await client.query(`SELECT ${COLUMNS} ${FROM} WHERE s.id=$1`, [id])).rows[0]);
  const audit = (client, user, action, id) => client.query('INSERT INTO vault_audit (actor_id,action,entity) VALUES ($1,$2,$3)', [user.id, `lab.${action}`, `lab_strategy:${id}`]);
  const insert = async (client, user, data, action) => {
    const id = (await client.query('INSERT INTO lab_strategy (owner,name,spec,market,instrument,origin) VALUES ($1,$2,$3::jsonb,$4,$5,$6::jsonb) RETURNING id', [user.id, data.name, JSON.stringify(data.spec), data.market, data.instrument, data.origin ? JSON.stringify(data.origin) : null])).rows[0].id;
    await audit(client, user, action, id);
    return reread(client, id);
  };

  app.get(BASE, route(async (c, user) => {
    const status = c.req.query('status') ?? null;
    if (status !== null && !LAB_STATUSES.has(status)) throw new LabRequestError('Unknown status filter.');
    const result = await pool.query(`SELECT ${COLUMNS} ${FROM} WHERE s.owner=$1 AND ($2::text IS NULL OR s.status=$2) ORDER BY s.updated_at DESC LIMIT 200`, [user.id, status]);
    return c.json(result.rows.map(view));
  }));
  app.get(`${BASE}/published`, route(async c => {
    const result = await pool.query(`SELECT v.id AS "snapshotId", v.strategy_id AS id, v.version, v.name, v.spec, v.spec->'origin' AS origin, v.market, v.instrument, u.name AS "ownerName", v.published_at AS "publishedAt"
      FROM lab_strategy_version v LEFT JOIN "user" u ON u.id = v.owner ORDER BY v.published_at DESC LIMIT 200`);
    return c.json(result.rows);
  }));
  app.get(`${BASE}/review`, route(async (c, user) => {
    if (!roleAllows(user.role, 'admin')) throw new LabRequestError('Administrator access required', 403);
    const result = await pool.query(`SELECT ${COLUMNS} ${FROM} WHERE s.status='submitted' ORDER BY s.submitted_at LIMIT 200`);
    return c.json(result.rows.map(view));
  }));
  app.get(`${BASE}/${ID}`, route(async (c, user) => {
    const row = (await pool.query(`SELECT ${COLUMNS} ${FROM} WHERE s.id=$1`, [Number(c.req.param('id'))])).rows[0];
    if (!labCanRead(row, user)) throw new LabRequestError('Strategy not found.', 404);
    return c.json(view(row));
  }));
  app.post(BASE, route(async (c, user) => {
    if (!roleAllows(user.role, 'write')) throw new LabRequestError('Researcher access required', 403);
    const data = validateLabStrategy(await body(c));
    const client = await pool.connect();
    try {
      await client.query('BEGIN');
      const created = await insert(client, user, data, 'create');
      await client.query('COMMIT');
      return c.json(created, 201);
    } catch (error) { await client.query('ROLLBACK'); throw error; } finally { client.release(); }
  }));
  app.put(`${BASE}/${ID}`, route(async (c, user) => {
    const input = await body(c);
    const version = validateLabVersion(input.version), data = validateLabStrategy(input);
    return c.json(await locked(Number(c.req.param('id')), async (client, row) => {
      labAction(row, user, 'update', version);
      if (!sameLabOrigin(row.origin, data.origin)) throw new LabRequestError('A strategy keeps the book template it was customized from (spec.origin); start a new customization instead.');
      await client.query("UPDATE lab_strategy SET name=$2, spec=$3::jsonb, market=$4, instrument=$5, status='draft', version=version+1, updated_at=now() WHERE id=$1", [row.id, data.name, JSON.stringify(data.spec), data.market, data.instrument]);
      await audit(client, user, 'update', row.id);
      return reread(client, row.id);
    }));
  }));
  // DELETE carries the version in the query string (?version=N): proxies may drop DELETE bodies.
  app.delete(`${BASE}/${ID}`, route(async (c, user) => {
    const raw = c.req.query('version');
    const version = raw === undefined ? undefined : validateLabVersion(Number(raw));
    await locked(Number(c.req.param('id')), async (client, row) => {
      labAction(row, user, 'delete', version);
      await client.query('DELETE FROM lab_strategy WHERE id=$1', [row.id]);
      await audit(client, user, 'delete', row.id);
    });
    return c.body(null, 204);
  }));
  app.post(`${BASE}/${ID}/duplicate`, route(async (c, user) => {
    const copy = await locked(Number(c.req.param('id')), async (client, row) => {
      labAction(row, user, 'duplicate');
      return insert(client, user, { ...row, name: `Copy of ${row.name}`.slice(0, 120) }, 'duplicate');
    });
    return c.json(copy, 201);
  }));
  app.post(`${BASE}/${ID}/:action{submit|withdraw|publish|reject}`, route(async (c, user) => {
    const action = c.req.param('action'), input = await body(c);
    const version = validateLabVersion(input.version);
    const note = action === 'publish' || action === 'reject' ? validateLabNote(input.note, action === 'reject') : null;
    return c.json(await locked(Number(c.req.param('id')), async (client, row) => {
      labAction(row, user, action, version);
      const review = action === 'publish' || action === 'reject';
      await client.query(`UPDATE lab_strategy SET ${TRANSITIONS[action]}, updated_at=now() WHERE id=$1`, review ? [row.id, user.id, note] : [row.id]);
      // The published snapshot is insert-only: no route updates or deletes lab_strategy_version.
      if (action === 'publish') await client.query('INSERT INTO lab_strategy_version (strategy_id,version,owner,name,spec,market,instrument,published_by) VALUES ($1,$2,$3,$4,$5::jsonb,$6,$7,$8)', [row.id, row.version, row.owner, row.name, JSON.stringify(row.spec), row.market, row.instrument, user.id]);
      await audit(client, user, action, row.id);
      return reread(client, row.id);
    }));
  }));
}
