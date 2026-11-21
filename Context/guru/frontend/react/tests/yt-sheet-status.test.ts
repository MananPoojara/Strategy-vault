import { expect, it } from 'vitest';
import { statusTime, ytStatusBadge, ytStatusBySet, ytStatusPanel, ytStatusToggle, YT_STATUS_LINK } from '../src/services/yt-sheet-status';

const row = (id: string, category: string, status: string, sheets = 0, matched = 0, detail = '') =>
  ({ id, category, name: `Name ${id}`, status, detail, sheets, matched, verified_by_jay: false, verified_as: '' });
const body = {
  version: 1, generated_at: '2026-11-20T13:01:30+00:00',
  source: { file: 'Tradesheet Match Status.xlsx', made: '20-11-2026' },
  totals: { total: 8 },
  strategies: [
    row('ytkr_a', 'Kora Reddy', 'Matching', 3, 3),
    row('ytkr_b', 'Kora Reddy', 'Partly matching', 4, 2, 'two sheets differ'),
    row('ytkr_c', 'Kora Reddy', 'Not matching'),
    row('ytkr_d', 'Kora Reddy', 'Not compared yet'),
    row('ytkr_e', 'Kora Reddy', 'No trade sheets'),
    // In the catalogue under the stocks book, whatever the workbook category says.
    row('ytsws_x', '% Swing', 'Matching', 2, 2),
    // Not in the catalogue: falls back to the category name.
    row('ytsw_new', '% Swing', 'Not compared yet'),
    row('ytzz_lost', 'Unknown group', 'Matching'),
  ],
};
const catalogue = [
  { id: 'ytkr_a', book: 'yt_kr' }, { id: 'ytkr_b', book: 'yt_kr' }, { id: 'ytkr_c', book: 'yt_kr' }, { id: 'ytkr_d', book: 'yt_kr' }, { id: 'ytkr_e', book: 'yt_kr' },
  { id: 'ytsws_x', book: 'yt_sws' },
];
const sets = [
  { key: 'kora', book: 'yt_kr', name: 'Kora Reddy' }, { key: 'swing', book: 'yt_sw', name: '% Swing' },
  { key: 'swingstocks', book: 'yt_sws', name: '% Swing (Stocks)' }, { key: 'sniper', book: 'yt_sn', name: 'Sniper System' },
];

it('groups strategies by collection through the catalogue book, then the category', () => {
  const idx = ytStatusBySet(body, catalogue, sets)!;
  expect(idx.sets.kora).toMatchObject({ matching: 1, partly: 1, notMatching: 1, pending: 1, none: 1, compared: 3, total: 5, sheets: 7, sheetsMatched: 5 });
  expect(idx.sets.kora.problems.map(s => s.id)).toEqual(['ytkr_b', 'ytkr_c']);
  expect(idx.sets.swingstocks).toMatchObject({ matching: 1, compared: 1, total: 1 });
  expect(idx.sets.swing).toMatchObject({ pending: 1, compared: 0, total: 1 });
  expect(idx.sets.sniper).toBeUndefined();
  expect(idx.unmapped.map(s => s.id)).toEqual(['ytzz_lost']);
  expect(idx.made).toBe('20-11-2026');
});

it('returns null for a missing or malformed file', () => {
  expect(ytStatusBySet(null, catalogue, sets)).toBeNull();
  expect(ytStatusBySet({ strategies: 'x' }, catalogue, sets)).toBeNull();
});

it('formats the badge and toggle', () => {
  const idx = ytStatusBySet(body, catalogue, sets)!;
  expect(ytStatusBadge(idx.sets.kora)).toContain('1 of 3 match');
  expect(ytStatusBadge(idx.sets.swing)).toContain('1 pending');
  expect(ytStatusBadge(undefined)).toContain('yt-match none');
  expect(ytStatusToggle(idx.sets.kora)).toBe('1/3');
  expect(ytStatusToggle(idx.sets.swing)).toBe('');
});

it('renders the panel with current figures, the as-of note, the link and the earlier tally', () => {
  const idx = ytStatusBySet(body, catalogue, sets)!;
  const el = document.createElement('div');
  el.innerHTML = ytStatusPanel(idx.sets.kora, idx, '<p id="old">old tally</p>', '12-11-2026');
  const counts = [...el.querySelectorAll('[data-yt-status-counts] > div')].map(d => d.textContent);
  expect(counts).toEqual(['1matching', '1partly matching', '1not matching', '1pending', '1no trade sheets']);
  expect(el.querySelector('[data-yt-status-sheets]')?.textContent).toBe('Trade sheets: 5 of 7 compared sheets match');
  expect(el.querySelectorAll('.yt-sheets-list li')).toHaveLength(2);
  expect(el.textContent).toContain('two sheets differ');
  expect(el.querySelector('[data-yt-status-asof]')?.textContent).toContain(`Status as of ${statusTime(body.generated_at)}`);
  expect(el.querySelector(`a[href="${YT_STATUS_LINK}"]`)).not.toBeNull();
  const old = el.querySelector('details[data-yt-sheets-earlier]')!;
  expect(old.querySelector('summary')?.textContent).toBe('Earlier sheet comparison (12-11-2026)');
  expect(old.querySelector('#old')).not.toBeNull();
});

it('renders an empty collection and escapes text', () => {
  const idx = ytStatusBySet({ ...body, strategies: [row('ytkr_a', 'Kora Reddy', 'Not matching', 0, 0, '<b>x</b>')] }, catalogue, sets)!;
  const el = document.createElement('div');
  el.innerHTML = ytStatusPanel(idx.sets.kora, idx);
  expect(el.querySelector('.ref-sub')?.textContent).toBe('<b>x</b>');
  expect(el.querySelector('details')).toBeNull();
  el.innerHTML = ytStatusPanel(undefined, idx);
  expect(el.textContent).toContain('No strategy of this collection');
  expect(statusTime('not a date')).toBe('not a date');
});
