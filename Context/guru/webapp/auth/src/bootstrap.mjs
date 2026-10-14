import { getMigrations } from 'better-auth/db/migration';
import { readFile, readdir } from 'node:fs/promises';
import { auth, pool } from './config.mjs';
import { validateNoteKey } from './contracts.mjs';

export async function bootstrap() {
  const migrations = await getMigrations(auth.options);
  await migrations.runMigrations();
  await pool.query(await readFile(new URL('../../db/001-app.sql', import.meta.url), 'utf8').catch(() => readFile(new URL('../../../db/001-app.sql', import.meta.url), 'utf8')));
  await pool.query(await readFile(new URL('../../db/002-lab-strategies.sql', import.meta.url), 'utf8').catch(() => readFile(new URL('../../../db/002-lab-strategies.sql', import.meta.url), 'utf8')));
  await pool.query(await readFile(new URL('../../db/003-lab-origin.sql', import.meta.url), 'utf8').catch(() => readFile(new URL('../../../db/003-lab-origin.sql', import.meta.url), 'utf8')));
  const email = process.env.VAULT_ADMIN_EMAIL?.trim().toLowerCase();
  const password = process.env.VAULT_ADMIN_PASSWORD;
  if (email && password) {
    const exists = await pool.query('SELECT id FROM "user" WHERE email = $1', [email]);
    if (!exists.rowCount) {
      const result = await auth.api.signUpEmail({ body: { name: process.env.VAULT_ADMIN_NAME || 'Manan', email, password } });
      await pool.query('UPDATE "user" SET role = $1, "emailVerified" = true WHERE id = $2', ['admin', result.user.id]);
      console.info('Initial administrator created. Credentials remain in the local environment file.');
    }
  }
  const dir = process.env.VAULT_NOTES_DIR;
  let imported = 0;
  if (dir) {
    for (const file of await readdir(dir).catch(() => [])) {
      if (!file.endsWith('.txt')) continue;
      let key; try { key = validateNoteKey(file.slice(0, -4)); } catch { continue; }
      const content = await readFile(`${dir}/${file}`, 'utf8');
      const result = await pool.query('INSERT INTO vault_notes (note_key, content) VALUES ($1, $2) ON CONFLICT DO NOTHING', [key, content]);
      imported += result.rowCount;
    }
  }
  console.info(`Application schema ready; imported ${imported} existing research notes.`);
}
if (process.argv[1]?.endsWith('/bootstrap.mjs')) {
  try { await bootstrap(); } finally { await pool.end(); }
}
