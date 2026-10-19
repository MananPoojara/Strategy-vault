// Lets node:test import the frontend's .ts modules on a Node build without TypeScript support:
// node --import ./tests/ts-loader.mjs --test tests/library-context-integration.test.mjs
// Types are stripped with the Rolldown (oxc) transform Vite already installs; no extra dependency.
import { register } from 'node:module';

register(new URL('./ts-hooks.mjs', import.meta.url));
