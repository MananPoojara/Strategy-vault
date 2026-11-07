import { afterEach, expect, it, vi } from 'vitest';
import { readFileSync } from 'node:fs';

const code = readFileSync('src/services/update-notice.js', 'utf8');
const current = 'a'.repeat(64), next = 'b'.repeat(64);
const settle = async () => { await Promise.resolve(); await Promise.resolve(); await Promise.resolve(); };
afterEach(() => { document.head.innerHTML = ''; document.body.innerHTML = ''; vi.restoreAllMocks(); });
function boot({ build = current, version = current, ok = true, preview = false } = {}) {
  if (build) document.head.innerHTML = `<meta name="vault-build" content="${build}">`;
  const state = { version, ok, error: false };
  const fetch = vi.fn(async () => { if (state.error) throw Error('offline'); return { ok: state.ok, json: async () => ({ build: state.version }) }; });
  let poll = async () => {};
  const reload = vi.fn(), show = vi.fn(function (this: HTMLDialogElement) { this.open = true; });
  Object.defineProperty(HTMLDialogElement.prototype, 'showModal', { configurable: true, value: show });
  new Function('document', 'window', 'location', 'fetch', 'setInterval', code)(document, { addEventListener: vi.fn() }, { reload, search: preview ? '?update-preview=1' : '' }, fetch, (fn: () => Promise<void>, ms: number) => { expect(ms).toBe(30000); poll = fn; });
  return { state, fetch, show, reload, poll: async () => { await poll(); await settle(); } };
}
it('waits for a genuine version change and never reloads a draft automatically', async () => {
  document.body.innerHTML = '<input value="unfinished strategy">';
  const b = boot(); await settle(); expect(b.show).not.toHaveBeenCalled();
  b.state.version = next; await b.poll();
  expect(b.show).toHaveBeenCalledTimes(1); expect(b.reload).not.toHaveBeenCalled();
  expect(document.querySelector('input')?.value).toBe('unfinished strategy');
  const dialog = document.querySelector('dialog')!;
  expect(dialog.getAttribute('aria-labelledby')).toBe('vault-update-heading');
  expect(dialog.textContent).toContain('An update is ready');
  expect(b.fetch).toHaveBeenCalledWith('/vendor/app-version.json', { cache: 'no-store', credentials: 'same-origin' });
  dialog.querySelector('button')!.click(); expect(b.reload).toHaveBeenCalledTimes(1);
});
it('does not keep reopening a dismissed notice for the same release', async () => {
  const b = boot({ version: next }); await settle();
  document.querySelector('dialog')!.open = false; await b.poll(); expect(b.show).toHaveBeenCalledTimes(1);
  b.state.version = 'c'.repeat(64); await b.poll(); expect(b.show).toHaveBeenCalledTimes(2);
  expect(document.querySelectorAll('dialog')).toHaveLength(1);
});
it('ignores failed, malformed and offline checks, then recovers', async () => {
  const b = boot({ ok: false, version: next }); await settle(); expect(b.show).not.toHaveBeenCalled();
  b.state.ok = true; b.state.version = '<bad>'; await b.poll(); expect(b.show).not.toHaveBeenCalled();
  b.state.error = true; b.state.version = next; await b.poll(); expect(b.show).not.toHaveBeenCalled();
  b.state.error = false; await b.poll(); expect(b.show).toHaveBeenCalledTimes(1);
});
it('does not poll development documents without a built version', async () => {
  const b = boot({ build: '' }); await settle(); expect(b.fetch).not.toHaveBeenCalled();
});
it('leaves maintenance documents to their existing checker', async () => {
  document.documentElement.setAttribute('data-vault-maintenance', '');
  try { const b = boot({ version: next }); await settle(); expect(b.fetch).not.toHaveBeenCalled(); }
  finally { document.documentElement.removeAttribute('data-vault-maintenance'); }
});

it('provides an explicit preview without falsely announcing a deployment', async () => {
  const b = boot({ preview: true }); await settle(); expect(b.show).toHaveBeenCalledTimes(1);
  expect(b.fetch).not.toHaveBeenCalled();
});
