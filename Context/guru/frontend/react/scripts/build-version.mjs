import { createHash } from 'node:crypto';
import { readdir, readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve, relative } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

export async function stampVersion(app, login) {
  const marker = /<meta name="vault-build" content="[a-f0-9]{64}">/g;
  const hash = createHash('sha256');
  for (const [label, root] of [['app', app], ['login', login]]) {
    for (const file of (await readdir(root, { recursive: true, withFileTypes: true }))
      .filter((entry) => entry.isFile()).map((entry) => resolve(entry.parentPath, entry.name)).sort()) {
      const name = relative(root, file);
      if (name === 'vendor/app-version.json' || name.startsWith('data/')) continue;
      const bytes = await readFile(file);
      hash.update(label + '/' + name + '\0');
      hash.update(name.endsWith('.html') ? bytes.toString().replace(marker, '') : bytes);
    }
  }
  const build = hash.digest('hex');
  for (const file of [resolve(app, 'index.html'), resolve(login, 'login.html')]) {
    const html = (await readFile(file, 'utf8')).replace(marker, '');
    if (!html.includes('</head>')) throw new Error(`Missing head in ${file}`);
    await writeFile(file, html.replace('</head>', `<meta name="vault-build" content="${build}"></head>`));
  }
  await mkdir(resolve(app, 'vendor'), { recursive: true });
  await writeFile(resolve(app, 'vendor/app-version.json'), JSON.stringify({ build }) + '\n');
  return build;
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const root = fileURLToPath(new URL('..', import.meta.url));
  console.log('Stamped frontend build:', await stampVersion(resolve(root, 'dist'), resolve(root, 'dist-login')));
}
