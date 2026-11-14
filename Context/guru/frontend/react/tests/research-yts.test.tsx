import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { ResearchYts } from '../src/pages/ResearchYts';
import { customPages } from '../src/app/custom-pages';

let reported = '';
let root: Root, host: HTMLDivElement, posted: unknown[] = [], uploads: { url: string; type: string; size: number }[] = [], signedIn: null | { id: number; username: string; name: string; admin: boolean } = null;
const PRIYA = { id: 2, username: 'priya', name: 'Priya Shah', admin: false };
const catalogue = [
  { id: 'ytbb_plain', name: 'Bollinger Band Breakout (Long only)', book: 'yt_bb' },
  { id: 'ytbb_athc', name: 'BB Breakout above All-Time-High Close (Long only)', book: 'yt_bb' },
  { id: 'ytd7_long', name: 'Double 7 Long', book: 'yt_d7' },
];
const issue = { id: 7, collection: 'yt_bb', strategy_id: 'ytbb_plain', category: 'wrong_trades', severity: 'high', status: 'open', title: 'Exit at the wrong price',
  body: 'Trade 3 exits at the open.', instrument: 'RELIANCE', period_from: null, period_to: null, page_link: '', reporter_name: 'Priya',
  created_at: '2026-11-13T05:00:00Z', updated_at: '2026-11-13T05:00:00Z', deleted_at: null, deleted_reason: null, comments: 1, files: 2, can_edit: true, can_delete: true, strategies: [], collections: [] };
const general = { ...issue, id: 9, collection: 'general', strategy_id: '', title: 'Equity curve labels overlap', strategies: ['ytbb_plain', 'ytd7_long'], collections: ['yt_bb', 'yt_d7'], files: 0 };
const files = [
  { id: 3, name: 'exit-trade.png', content_type: 'image/png', size: 52000, image: true, uploader_name: 'Priya', created_at: issue.created_at, can_delete: true },
  { id: 4, name: 'team sheet.xlsx', content_type: 'application/octet-stream', size: 2400000, image: false, uploader_name: 'Priya', created_at: issue.created_at, can_delete: false },
];
const json = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

beforeEach(() => {
  posted = []; uploads = []; signedIn = PRIYA; reported = '';
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  window.VaultRuntime = { ...(window.VaultRuntime || {}), ytCollections: () => [{ key: 'bbands', name: 'Bollinger Band', book: 'yt_bb' }, { key: 'double7', name: 'Double 7', book: 'yt_d7' }, { key: 'unused', name: 'No strategies', book: 'yt_none' }] } as typeof window.VaultRuntime;
  vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
    if (url === '/api/in/guru/yt/strategies') return json(catalogue);
    if (url === '/api/app/research-yts/me') return json({ qa: signedIn });
    if (url === '/api/app/research-yts/login') { const b = JSON.parse(String(init?.body)); if (b.password !== 'priya-pass') return json({ error: 'Wrong username or password.' }, 401); signedIn = PRIYA; return json({ qa: PRIYA }); }
    if (url === '/api/app/research-yts/qa-users' && !init?.method) return json([{ ...PRIYA, admin: true, id: 1, username: 'admin', name: 'Admin', created_at: issue.created_at, last_login_at: issue.created_at, deactivated_at: null, issues: 0, open: 0 }, { ...PRIYA, created_at: issue.created_at, last_login_at: null, deactivated_at: null, issues: 1, open: 1 }]);
    if (url === '/api/app/research-yts/summary') return json({ totals: { open: 1, in_progress: 0, fixed: 2, not_issue: 0 }, collections: { yt_bb: { open: 1, in_progress: 0, fixed: 2, not_issue: 0 } }, strategies: { ytbb_plain: { open: 1, in_progress: 0, fixed: 0, not_issue: 0 } } });
    if (url.startsWith('/api/app/research-yts/issues?')) return json({ items: [issue], total: 1, page: 1, size: 20, pages: 1 });
    if (url === '/api/app/research-yts/issues' && init?.method === 'POST') { posted.push(JSON.parse(String(init.body))); return json({ ...issue, id: 8 }, 201); }
    if (url.startsWith('/api/app/research-yts/issues/8/files?') && init?.method === 'POST') { const b = init.body as File; uploads.push({ url, type: String((init.headers as Record<string, string>)['Content-Type']), size: b.size }); return json({ issue, files }, 201); }
    if (url === '/api/app/research-yts/issues/9') return json({ issue: general, events: [], files: [] });
    if (url === '/api/app/research-yts/issues/7') return json({ issue: { ...issue, page_link: reported }, files, events: [{ id: 1, action: 'created', status: 'open', body: issue.body, actor_name: 'Priya', created_at: issue.created_at }, { id: 2, action: 'comment', status: null, body: 'Same on TCS.', actor_name: 'Ravi', created_at: issue.created_at }] });
    return json({ error: 'unexpected ' + url }, 404);
  }));
  try { localStorage.clear(); } catch { /* ignore */ }
});
afterEach(async () => { await act(async () => root?.unmount()); host?.remove(); vi.unstubAllGlobals(); location.hash = ''; });
async function mount(hash: string) {
  location.hash = hash;
  host = document.createElement('div'); document.body.append(host); root = createRoot(host);
  await act(async () => root.render(<ResearchYts />));
  await act(async () => { await new Promise(r => setTimeout(r, 0)); });
}
const setValue = async (el: HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement, value: string) => act(async () => {
  const proto = el instanceof HTMLSelectElement ? HTMLSelectElement.prototype : el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
  Object.getOwnPropertyDescriptor(proto, 'value')!.set!.call(el, value);
  el.dispatchEvent(new Event(el instanceof HTMLSelectElement ? 'change' : 'input', { bubbles: true }));
});

it('is registered as the "Research YTS" navigation page', () => {
  expect(customPages.find(p => p.path === '/research-yts')).toMatchObject({ label: 'Research YTS', showInNavigation: true });
});

it('lists collections that have strategies, with open counts, and the issues for a strategy', async () => {
  await mount('#/research-yts?c=yt_bb');
  const side = [...host.querySelectorAll('.yts-side a')].map(a => a.textContent);
  expect(side).toEqual(['All collections1', 'General issues (several strategies)', 'Bollinger Band1', 'Double 7']);
  const strategy = host.querySelector<HTMLSelectElement>('.yts-tools select')!;
  expect([...strategy.options].map(o => o.textContent)).toEqual(['All strategies in this collection', 'Bollinger Band Breakout (Long only) (1 open)', 'BB Breakout above All-Time-High Close (Long only)']);
  expect(host.querySelector('.yts-row')?.textContent).toContain('Exit at the wrong price');
  expect(host.querySelector('.yts-row')?.textContent).toContain('Bollinger Band · Bollinger Band Breakout (Long only) · RELIANCE');
  expect(host.querySelector('.yts-row')?.textContent).toContain('1 comment · 2 files');
});

it('opens a new issue pre-filled from the strategy page and sends it with the typed name', async () => {
  await mount('#/research-yts?new=1&c=yt_bb&s=ytbb_plain&tk=RELIANCE&from=2020-01-01&to=2026-10-19&link=%23%2Fs%2Fytbb_plain');
  const form = host.querySelector('form.yts-form')!;
  const selects = form.querySelectorAll('select');
  expect(selects[0].value).toBe('yt_bb');
  expect(selects[1].value).toBe('ytbb_plain');
  expect(form.querySelector<HTMLInputElement>('input[placeholder^="e.g. NIFTY"]')!.value).toBe('RELIANCE');
  expect([...form.querySelectorAll<HTMLInputElement>('input[type="date"]')].map(i => i.value)).toEqual(['2020-01-01', '2026-10-19']);
  expect(form.textContent).toContain('The page you came from is attached');
  expect(form.textContent).toContain('Reported by Priya Shah');
  await setValue(selects[2], 'wrong_trades');
  await setValue(form.querySelector<HTMLInputElement>('input[maxlength="160"]')!, 'Exit at the wrong price');
  await setValue(form.querySelector('textarea')!, 'Trade 3 exits at the open.');
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); await new Promise(r => setTimeout(r, 0)); });
  expect(posted).toEqual([{ collection: 'yt_bb', strategy_id: 'ytbb_plain', category: 'wrong_trades', severity: 'normal', title: 'Exit at the wrong price', body: 'Trade 3 exits at the open.',
    instrument: 'RELIANCE', period_from: '2020-01-01', period_to: '2026-10-19', page_link: '#/s/ytbb_plain' }]);
  expect(location.hash).toBe('#/research-yts?c=yt_bb&s=ytbb_plain&issue=8');
});

it('shows one issue with its strategy as plain text, its files, history and comments', async () => {
  await mount('#/research-yts?issue=7');
  expect(host.querySelector('.yts-one-title')?.textContent).toBe('Exit at the wrong price');
  expect(host.querySelector('.yts-facts')?.textContent).toContain('Bollinger Band Breakout (Long only)');
  expect(host.querySelector('.yts-facts')?.textContent).toContain('Bollinger Band');
  expect(host.querySelectorAll('.yts-facts a')).toHaveLength(0);
  expect([...host.querySelectorAll('.yts-links a')].map(a => [a.textContent, a.getAttribute('href')])).toEqual([['Open the strategy →', '#/s/ytbb_plain']]);
  expect([...host.querySelectorAll('.yts-history li')].map(li => li.textContent)).toEqual([expect.stringContaining('Priya reported the issue'), expect.stringContaining('Ravi commented')]);
  const img = host.querySelector<HTMLImageElement>('.yts-thumbs img')!;
  expect(img.getAttribute('src')).toBe('/api/app/research-yts/issues/7/files/3');
  expect(host.querySelector('.yts-thumbs')?.textContent).toContain('Remove');
  const other = host.querySelector('.yts-attach .yts-file-list li')!;
  expect(other.querySelector('a')?.getAttribute('href')).toBe('/api/app/research-yts/issues/7/files/4?download=1');
  expect(other.textContent).toContain('2.3 MB');
  expect(other.textContent).not.toContain('Remove');
});

it('files a general issue about strategies from several collections, with a photo', async () => {
  await mount('#/research-yts?new=1');
  const form = host.querySelector('form.yts-form')!;
  const general = [...form.querySelectorAll<HTMLButtonElement>('[role="radio"]')].find(b => b.textContent?.startsWith('General'))!;
  await act(async () => general.click());
  expect(form.querySelector('select[required]')?.closest('label')?.textContent).toContain('Type of problem'); // no collection to choose
  const boxes = () => [...form.querySelectorAll<HTMLInputElement>('.yts-picker-list input[type="checkbox"]')];
  expect(boxes().map(b => b.closest('label')?.textContent?.trim())).toEqual([
    'Bollinger Band Breakout (Long only) · Bollinger Band', 'BB Breakout above All-Time-High Close (Long only) · Bollinger Band', 'Double 7 Long · Double 7']);
  await act(async () => { boxes()[0].click(); }); await act(async () => { boxes()[2].click(); });
  expect(form.querySelector('.yts-picker')?.textContent).toContain('2 strategies chosen');
  const input = form.querySelector<HTMLInputElement>('.yts-files input[type="file"]')!;
  const photo = new File([new Uint8Array(1200)], 'chart.png', { type: 'image/png' });
  Object.defineProperty(input, 'files', { value: [photo], configurable: true });
  await act(async () => { input.dispatchEvent(new Event('change', { bubbles: true })); });
  expect(form.querySelector('.yts-file-list')?.textContent).toContain('chart.png');
  await setValue(form.querySelector('select[required]')!, 'page');
  await setValue(form.querySelector<HTMLInputElement>('input[maxlength="160"]')!, 'Equity curve labels overlap');
  await setValue(form.querySelector('textarea')!, 'Labels overlap on narrow screens.');
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); await new Promise(r => setTimeout(r, 0)); });
  await act(async () => { await new Promise(r => setTimeout(r, 0)); });
  expect(posted).toEqual([expect.objectContaining({ collection: 'general', strategy_id: '', category: 'page', strategies: [{ id: 'ytbb_plain', collection: 'yt_bb' }, { id: 'ytd7_long', collection: 'yt_d7' }] })]);
  expect(uploads).toEqual([{ url: '/api/app/research-yts/issues/8/files?name=chart.png', type: 'image/png', size: 1200 }]);
  expect(location.hash).toContain('issue=8');
});

it('opens the reported page with its parameters instead of the plain strategy link', async () => {
  reported = '#/s/ytbb_plain?tk=RELIANCE&from=2020-01-01';
  await mount('#/research-yts?issue=7');
  expect([...host.querySelectorAll('.yts-links a')].map(a => [a.textContent, a.getAttribute('href')])).toEqual([['Open the page with issue parameters →', '#/s/ytbb_plain?tk=RELIANCE&from=2020-01-01']]);
});

it('shows a general issue with the strategies it is about, by collection', async () => {
  await mount('#/research-yts?issue=9');
  expect(host.querySelector('.yts-facts')?.textContent).toContain('General issue: 2 strategies');
  expect([...host.querySelectorAll('.yts-general-list > div')].map(d => d.textContent)).toEqual(['Bollinger BandBollinger Band Breakout (Long only)', 'Double 7Double 7 Long']);
  expect([...host.querySelectorAll('.yts-general-list a')].map(a => a.getAttribute('href'))).toEqual(['#/s/ytbb_plain', '#/s/ytd7_long']);
  expect(host.textContent).not.toContain('Open the strategy');
});

it('keeps everything behind the Research YTS sign-in until a QA signs in', async () => {
  signedIn = null;
  await mount('#/research-yts?c=yt_bb');
  expect(host.querySelector('.yts-gate h2')?.textContent).toBe('Sign in to Research YTS');
  expect(host.querySelector('.yts-row')).toBeNull();
  await setValue(host.querySelector<HTMLInputElement>('#yts-username')!, 'priya');
  await setValue(host.querySelector<HTMLInputElement>('#yts-password')!, 'wrong');
  await act(async () => { host.querySelector('form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); await new Promise(r => setTimeout(r, 0)); });
  expect(host.querySelector('[role="alert"]')?.textContent).toContain('Wrong username or password.');
  expect(host.querySelector<HTMLInputElement>('#yts-password')!.value).toBe('');
  await setValue(host.querySelector<HTMLInputElement>('#yts-password')!, 'priya-pass');
  await act(async () => { host.querySelector('form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); await new Promise(r => setTimeout(r, 0)); });
  await act(async () => { await new Promise(r => setTimeout(r, 0)); });
  expect(host.querySelector('.yts-who')?.textContent).toContain('Signed in as Priya Shah (priya)');
  expect(host.querySelector('.yts-who')?.textContent).not.toContain('QA accounts');
  expect(host.querySelector('.yts-row')?.textContent).toContain('Exit at the wrong price');
});

it('gives a Research YTS admin the QA accounts panel', async () => {
  signedIn = { id: 1, username: 'admin', name: 'Admin', admin: true };
  await mount('#/research-yts?view=accounts');
  expect(host.querySelector('.yts-who')?.textContent).toContain('Signed in as Admin (admin)Admin');
  expect(host.querySelector('.yts-accounts h2')?.textContent).toBe('QA accounts');
  const rows = [...host.querySelectorAll('.yts-acct')];
  expect(rows.map(r => r.querySelector('b')?.textContent)).toEqual(['Admin', 'Priya Shah']);
  expect(rows[0].textContent).not.toContain('Deactivate');
  expect(rows[1].textContent).toContain('Deactivate');
  expect(rows[1].querySelector('a')?.getAttribute('href')).toContain('reporter=2');
});
