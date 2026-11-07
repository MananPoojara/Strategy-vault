import { expect, it } from 'vitest';
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { stampVersion } from '../scripts/build-version.mjs';
it('stamps app and login with one stable content identity that changes with deployed files', async () => {
  const root = await mkdtemp(join(tmpdir(), 'vault-version-'));
  const app = join(root, 'app'), login = join(root, 'login');
  try {
    await mkdir(app); await mkdir(login);
    await writeFile(join(app, 'index.html'), '<html><head></head></html>');
    await writeFile(join(login, 'login.html'), '<html><head></head></html>');
    const id = await stampVersion(app, login);
    expect(id).toMatch(/^[a-f0-9]{64}$/); expect(await stampVersion(app, login)).toBe(id);
    for (const file of [join(app, 'index.html'), join(login, 'login.html')]) expect(await readFile(file, 'utf8')).toContain(`content="${id}"`);
    expect(JSON.parse(await readFile(join(app, 'vendor/app-version.json'), 'utf8'))).toEqual({ build: id });
    await writeFile(join(app, 'new.js'), 'changed'); expect(await stampVersion(app, login)).not.toBe(id);
  } finally { await rm(root, { recursive: true, force: true }); }
});
