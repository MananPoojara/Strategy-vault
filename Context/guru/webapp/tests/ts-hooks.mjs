// Module hooks for ts-loader.mjs: resolve extensionless relative imports from .ts files and strip types from .ts sources.
import { readFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';

const frontend = new URL('../../frontend/react/package.json', import.meta.url);
const { transformSync } = await import(pathToFileURL(createRequire(frontend).resolve('rolldown/experimental')).href);

export async function resolve(specifier, context, next) {
  if (context.parentURL?.endsWith('.ts') && /^\.{1,2}\//.test(specifier) && !/\.[cm]?[jt]sx?$/.test(specifier)) {
    for (const ext of ['.ts', '.tsx', '.js']) {
      const url = new URL(specifier + ext, context.parentURL);
      if (existsSync(fileURLToPath(url))) return { url: url.href, shortCircuit: true };
    }
  }
  return next(specifier, context);
}

export async function load(url, context, next) {
  if (!url.endsWith('.ts')) return next(url, context);
  const path = fileURLToPath(url);
  const { code, errors } = transformSync(path, await readFile(path, 'utf8'), { lang: 'ts' });
  if (errors?.length) throw new Error(`${path}: ${errors.map((e) => e.message).join('; ')}`);
  return { format: 'module', source: code, shortCircuit: true };
}
