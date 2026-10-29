import { createServer } from 'vite';
import react from '@vitejs/plugin-react';
import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const out = resolve(process.env.VAULT_MAINTENANCE_OUT || resolve(root, 'dist'));
const server = await createServer({ configFile: false, root, plugins: [react()], server: { middlewareMode: true }, appType: 'custom' });
try {
  const { default: MaintenancePage, maintenanceStyles } = await server.ssrLoadModule('/src/pages/MaintenancePage.tsx');
  let fonts = '';
  for (const [name, weight, file] of [['IBM Plex Sans', '400', 'IBMPlexSans-400.woff2'], ['IBM Plex Sans', '600', 'IBMPlexSans-600.woff2'], ['Archivo', '400 800', 'Archivo-400-800.woff2']]) {
    const bytes = await readFile(resolve(root, 'public/vendor/fonts', file));
    fonts += `@font-face{font-family:"${name}";font-style:normal;font-weight:${weight};font-display:swap;src:url(data:font/woff2;base64,${bytes.toString('base64')}) format('woff2');}`;
  }
  const markup = renderToStaticMarkup(createElement(MaintenancePage));
  const html = `<!doctype html><html lang="en" data-vault-maintenance><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="robots" content="noindex,nofollow"><meta name="theme-color" content="#f3f5f7"><title>Updating Strategy Vault</title><link rel="icon" href="data:,"><style>${fonts}${maintenanceStyles}</style><script src="/maintenance-watch.js" defer></script></head><body>${markup}</body></html>`;
  await mkdir(out, { recursive: true });
  await writeFile(resolve(out, 'maintenance.html'), html);
  await writeFile(resolve(out, 'maintenance-watch.js'), await readFile(resolve(root, 'src/services/maintenance-watch.js')));
  console.log('Built standalone maintenance page and reconnect checker.');
} finally { await server.close(); }
