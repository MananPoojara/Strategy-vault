import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { ResearchWork } from '../src/pages/ResearchWork';
import { customPages } from '../src/app/custom-pages';
import { checkWork, dayKey, groupByDay, helperKind, loadResearchWork, strategiesAdded, BUILD_HINT, DATA_URL, type WorkEvent } from '../src/services/research-work';

const ev = (key: string, at: string, extra: Partial<WorkEvent> = {}): WorkEvent => ({
  key: key.padEnd(40, '0'), short: key, at, label: `Commit ${key}`, who: 'Jay Patel', work: '', with: [], files: 1, insertions: 1, deletions: 0,
  merge: false, added: [], removed: [], count: 0, ...extra,
});
const events = [
  ev('a1', '2026-11-19T10:00:00+05:30', { count: 335, added: ['ytkr_long'] }),
  ev('a3', '2026-11-20T12:00:00+05:30', { count: 343, added: ['ytkr_short', 'ytnr_6h'], with: ['Claude Opus 5.5'], work: '- Kora Reddy\n- Nick Radge' }),
  ev('a2', '2026-11-19T23:50:00+05:30', { count: 335 }),
  // 20:00 UTC on 19 Nov is 01:30 on 20 Nov in India.
  ev('a4', '2026-11-19T20:00:00Z', { count: 343 }),
];
const work = {
  version: 1, generated_at: '2026-11-20T08:00:00+00:00', branch: 'jay', head: 'a3'.padEnd(40, '0'),
  uncommitted: { files: 3, note: 'x' }, method: 'count = ids at the commit.',
  totals: { commits: 4, days: 2, strategies: 343, added: 3, authors: ['Jay Patel'] },
  days: [{ date: '2026-11-20', commits: 2, added: 2, removed: 0, count: 343, authors: ['Jay Patel'] }, { date: '2026-11-19', commits: 2, added: 1, removed: 0, count: 335, authors: ['Jay Patel'] }],
  events,
};
const json = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

it('groups events by India day, newest day and newest event first', () => {
  const g = groupByDay(events);
  expect(g.map(d => d.day)).toEqual(['2026-11-20', '2026-11-19']);
  expect(g[0].items.map(e => e.short)).toEqual(['a3', 'a4']);
  expect(g[1].items.map(e => e.short)).toEqual(['a2', 'a1']);
  expect(dayKey('2026-11-19T20:00:00Z')).toBe('2026-11-20');
});

it('lists added strategies newest first and colours Claude-assisted commits', () => {
  expect(strategiesAdded(events).map(s => s.id)).toEqual(['ytkr_short', 'ytnr_6h', 'ytkr_long']);
  expect(helperKind(events[1])).toBe('claude');
  expect(helperKind(events[0])).toBe('');
});

it('turns a missing or malformed file into a build notice', async () => {
  const missing = await loadResearchWork(vi.fn(async () => new Response('nope', { status: 404 })) as unknown as typeof fetch);
  expect(missing).toEqual({ ok: false, problem: expect.stringContaining(BUILD_HINT) });
  const broken = await loadResearchWork(vi.fn(async () => new Response('{not json', { status: 200 })) as unknown as typeof fetch);
  expect(broken.ok).toBe(false);
  expect(checkWork({ events: [{ key: 'x' }], days: [], totals: {} }).ok).toBe(false);
  expect(checkWork(null).ok).toBe(false);
  expect(checkWork(work).ok).toBe(true);
});

let root: Root, host: HTMLDivElement, served: unknown = work;
beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  served = work;
  location.hash = '#/research';
  vi.stubGlobal('fetch', vi.fn(async (url: string) => {
    if (url === DATA_URL) return served === undefined ? new Response('', { status: 404 }) : json(served);
    if (url === '/api/in/guru/yt/strategies') return json([{ id: 'ytkr_short', name: 'Kora Reddy Weekday Short' }]);
    return json({ error: 'unexpected ' + url }, 404);
  }));
  host = document.createElement('div'); document.body.append(host); root = createRoot(host);
});
afterEach(() => { act(() => root.unmount()); host.remove(); vi.unstubAllGlobals(); });
const settle = async () => { for (let i = 0; i < 5; i++) await act(async () => { await Promise.resolve(); }); };

it('is registered as the Research page', () => {
  expect(customPages.find(p => p.path === '/research')).toMatchObject({ label: 'Research', showInNavigation: true });
  expect(customPages.filter(p => p.path === '/research')).toHaveLength(1);
});

it('renders the daily timeline newest first and opens a commit', async () => {
  act(() => root.render(<ResearchWork />));
  await settle();
  expect(host.querySelector('h1')?.textContent).toBe("Claude's work");
  expect(host.textContent).toContain('YT strategies coded');
  expect(host.textContent).toContain('3 files had uncommitted changes');
  const labels = [...host.querySelectorAll('.rw-label')].map(n => n.textContent);
  expect(labels).toEqual(['Commit a3', 'Commit a4', 'Commit a2', 'Commit a1']);
  expect(host.querySelector('.rw-feed li')?.className).toBe('w-claude');
  expect(host.textContent).toContain('Kora Reddy Weekday Short · ytnr_6h');
  act(() => (host.querySelector('.rw-event') as HTMLButtonElement).click());
  const dialog = host.querySelector('[role="dialog"]');
  expect(dialog?.textContent).toContain('- Kora Reddy');
  expect(dialog?.textContent).toContain('with Claude Opus 5.5');
  act(() => { window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' })); });
  expect(host.querySelector('[role="dialog"]')).toBeNull();
});

it('shows a build notice, not a crash, when the file is missing', async () => {
  served = undefined;
  act(() => root.render(<ResearchWork />));
  await settle();
  const alert = host.querySelector('[role="alert"]');
  expect(alert?.textContent).toContain('build_research_work.py');
  expect(host.querySelector('.rw-feed')).toBeNull();
});
