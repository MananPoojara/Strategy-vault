import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { ResearchWork } from '../src/pages/ResearchWork';
import { readView, viewHref } from '../src/services/research-work';
import { checkTradesheets, filterStrategies, headline, loadTradesheets, statusClass, TS_HINT, TS_URL, type TsStrategy } from '../src/services/research-tradesheets';

const row = (id: string, status: string, extra: Partial<TsStrategy> = {}): TsStrategy => ({
  category: 'Inside Bar', id, name: `Name ${id}`, status, detail: '', sheets: 0, matched: 0, verified_by_jay: false, verified_as: '', ...extra,
});
const strategies = [
  row('ytib_a', 'Matching', { verified_by_jay: true, verified_as: 'Inside Bar', sheets: 2, matched: 2 }),
  row('ytib_b', 'Partly matching', { sheets: 2, matched: 1 }),
  row('ytsn_x', 'Not matching', { category: 'Sniper' }),
  row('ytsn_y', 'Not compared yet', { category: 'Sniper', detail: 'checked against AmiBroker' }),
  row('ytsn_z', 'No trade sheets', { category: 'Sniper' }),
];
const counts = { 'Matching': 1, 'Partly matching': 1, 'Not matching': 1, 'Not compared yet': 1, 'No trade sheets': 1 };
const data = {
  version: 1, generated_at: '2026-11-20T10:00:00+00:00',
  source: { file: 'Tradesheet Match Status - 19-11-2026.xlsx', made: '20-11-2026', date: '2026-11-20', date_from: 'Notes: Made', strategies_note: '', caveat: 'Old runs.', status_file: 'Strategy Tradesheet Status.xlsx', status_made: '11-11-2026' },
  statuses: ['Matching', 'Partly matching', 'Not matching', 'Not compared yet', 'No trade sheets'],
  totals: { ...counts, total: 5, verified_by_jay: 1 },
  verified_rule: 'Complete sheet.',
  categories: [{ category: 'Inside Bar', counts: { ...counts, 'Not matching': 0, 'Not compared yet': 0, 'No trade sheets': 0 }, total: 2, verified_by_jay: 1 }],
  strategies,
  history: [
    { date: '2026-11-19', date_from: 'Notes: Made', made: '19-11-2026', kind: 'seed', label: 'Backup workbook', source: 'old.xlsx', total: 4, counts: { ...counts, 'No trade sheets': 0 }, changes: [] },
    { date: '2026-11-20', date_from: 'Notes: Made', made: '20-11-2026', kind: 'run', label: 'Current match workbook', source: 'new.xlsx', total: 5, counts, changes: [{ id: 'ytsn_z', name: 'Name ytsn_z', from: null, to: 'No trade sheets' }] },
  ],
};
const json = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

it('reads the view from the hash', () => {
  expect(readView('#/research?v=tradesheets')).toBe('tradesheets');
  expect(readView('#/research?v=other')).toBe('claude');
  expect(viewHref('tradesheets')).toBe('#/research?v=tradesheets');
});

it('computes the headline and filters strategies', () => {
  expect(headline(data.totals)).toMatchObject({ done: 1, mismatch: 2, notMatching: 1, partly: 1, pending: 1, none: 1, total: 5, jay: 1 });
  expect(filterStrategies(strategies, 'jay', '').map(s => s.id)).toEqual(['ytib_a']);
  expect(filterStrategies(strategies, 'Not compared yet', '').map(s => s.id)).toEqual(['ytsn_y']);
  expect(filterStrategies(strategies, 'all', 'amibroker').map(s => s.id)).toEqual(['ytsn_y']);
  expect(filterStrategies(strategies, 'all', 'sniper')).toHaveLength(3);
  expect(statusClass('Partly matching')).toBe('partly');
});

it('turns a missing or malformed file into a build notice', async () => {
  const missing = await loadTradesheets(vi.fn(async () => new Response('', { status: 404 })) as unknown as typeof fetch);
  expect(missing).toEqual({ ok: false, problem: expect.stringContaining(TS_HINT) });
  expect(checkTradesheets({ strategies: [{}], totals: { total: 1 } }).ok).toBe(false);
  expect(checkTradesheets(data).ok).toBe(true);
});

let root: Root, host: HTMLDivElement, served: unknown = data;
beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  served = data;
  location.hash = '#/research?v=tradesheets';
  vi.stubGlobal('fetch', vi.fn(async (url: string) => {
    if (url === TS_URL) return served === undefined ? new Response('', { status: 404 }) : json(served);
    if (url === 'data/research-work.json') return new Response('', { status: 404 });
    return json([]);
  }));
  host = document.createElement('div'); document.body.append(host); root = createRoot(host);
});
afterEach(() => { act(() => root.unmount()); host.remove(); vi.unstubAllGlobals(); });
const settle = async () => { for (let i = 0; i < 5; i++) await act(async () => { await Promise.resolve(); }); };

it('renders the headline, history, categories and a filterable list', async () => {
  act(() => root.render(<ResearchWork />));
  await settle();
  expect(host.querySelector('h1')?.textContent).toBe('Tradesheet verified');
  expect(host.querySelector('[aria-current="page"]')?.textContent).toBe('Tradesheet verified');
  const cards = [...host.querySelectorAll('.rw-ts-card')].map(c => c.textContent);
  expect(cards).toEqual(["1Done: every trade sheet matches (incl. 1 on the Status workbook's Complete sheet)", '2Mismatch: 1 not matching, 1 partly', '1Pending verification (not compared yet)', '1No trade sheets']);
  const history = host.querySelector('section[aria-label="History"]')!;
  expect([...history.querySelectorAll('tbody tr td:first-child > b')].map(b => b.textContent)).toEqual([expect.stringContaining('20 Nov'), expect.stringContaining('19 Nov')]);
  expect(history.textContent).toContain('new → No trade sheets');
  expect(host.querySelector('section[aria-label="By category"]')?.textContent).toContain('Inside Bar');
  expect(host.querySelectorAll('.rw-ts-rows li')).toHaveLength(5);
  act(() => ([...host.querySelectorAll('button')].find(b => b.textContent?.startsWith('Done: on the Complete sheet')) as HTMLButtonElement).click());
  expect([...host.querySelectorAll('.rw-ts-rows li b')].map(b => b.textContent)).toEqual(['Name ytib_a']);
  expect(host.textContent).toContain('2 of 2 trade sheets match');
  expect(host.textContent).toContain('Tradesheet Match Status - 19-11-2026.xlsx');
});

it('shows a build notice when the file is missing', async () => {
  served = undefined;
  act(() => root.render(<ResearchWork />));
  await settle();
  expect(host.querySelector('[role="alert"]')?.textContent).toContain('build_research_tradesheets.py');
  expect(host.querySelector('.rw-ts-rows')).toBeNull();
});
