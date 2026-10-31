import {readFileSync} from 'node:fs';
import {expect, it} from 'vitest';

// runtime.js is one module with page-wide state, so its list presenter is evaluated from source, as basket-timing.test.ts does.
const runtime = readFileSync('src/compat/runtime.js', 'utf8');
const source = runtime.slice(runtime.indexOf('// One page of a ranked list.'), runtime.indexOf('function awaiting(rs)'));
const paginator = runtime.slice(runtime.indexOf('function libraryPagination('), runtime.indexOf('function authorResultHTML('));

function viewWith(state: Record<string, string>, unavailable: string[] = []) {
  const deps = {
    S: {tab: 'sharpe', view: 'grid', tk: 'NIFTY 50', list_page: '1', ...state}, TABS: {sharpe: {metric: {label: 'Sharpe', get: (r: any) => r.s.sharpe, fmt: String}}},
    esc: String, familyDisplayName: (v: any) => v.name, gateBadge: () => '', byline: () => '', verdictOf: () => ({c: '', t: 'SIG'}), spark: () => '', kpiTriplet: () => '',
    HOLD: {swing: 's'}, holdOf: () => 'swing', LABEL: {}, cls: () => '', pct: String, fx: String, stratLink: () => '#', appLink: () => '#', rulesList: () => '', unavailableBookCard: (m: string) => `<i data-missing>${m}</i>`, awaiting: () => '',
  };
  const make = new Function('deps', `const {${Object.keys(deps).join(',')}}=deps;${paginator}${source};return viewStrategies;`);
  return (rows: unknown[], perVariant = false) => make(deps)(rows, perVariant, unavailable) as string;
}
// Sorted best first by the caller, as sortRows does over every row before the presenter runs.
const ranked = (n: number) => Array.from({length: n}, (_, i) => ({v: {id: `id${i}`, family: `f${i}`, name: `Strategy ${i}`, style: 's', dir: 'Long', book: 'b', guru: 'g'}, tk: 'NIFTY 50', s: {sharpe: n - i}}));
const titles = (html: string) => [...html.matchAll(/<h3>(Strategy \d+)<\/h3>/g)].map(match => match[1]);

it('draws 24 cards from the whole ranking, with the true total and true rank on any page', () => {
  const first = viewWith({})(ranked(906));
  expect(titles(first)).toHaveLength(24);
  expect(titles(first)[0]).toBe('Strategy 0');
  expect(first).toContain('<b>906 strategies</b> · showing 1–24');
  expect(first).toContain('Page 1 of 38 · 906 entries');
  const third = viewWith({list_page: '3'})(ranked(906));
  expect(titles(third)[0]).toBe('Strategy 48');
  expect(third).toContain('#49');
  expect(third).toContain('showing 49–72');
  const last = viewWith({list_page: '38'})(ranked(906));
  expect(titles(last)).toHaveLength(18);
  expect(last).toContain('showing 889–906');
});

it('clamps a page past the end, for example after a filter shrank the list', () => {
  const html = viewWith({list_page: '38'})(ranked(30));
  expect(titles(html)).toHaveLength(6);
  expect(html).toContain('Page 2 of 2 · 30 entries');
  expect(viewWith({list_page: 'x'})(ranked(30))).toContain('showing 1–24');
});

it('counts one entry per strategy family across pages, and keeps table and list views paged and ranked', () => {
  const variants = [...ranked(30), ...ranked(30).map(r => ({...r, v: {...r.v, id: r.v.id + 'b'}}))];
  expect(viewWith({})(variants)).toContain('<b>30 strategies</b>');
  expect(viewWith({})(variants, true)).toContain('<b>60 rule sets</b>');
  const table = viewWith({view: 'table', list_page: '2'})(ranked(30));
  expect(table).toContain('<td class="rk">25</td>');
  expect(table).not.toContain('<td class="rk">24</td>');
  const list = viewWith({view: 'list', list_page: '2'})(ranked(30));
  expect(list).toContain('<span class="ln">25</span>');
  expect(list).toContain('Page 2 of 2');
});

it('lists unavailable methods once, on the last page, and says so on the others', () => {
  const unavailable = ['held one'];
  const first = viewWith({}, unavailable)(ranked(30));
  expect(first).not.toContain('data-missing');
  expect(first).toContain('1 unavailable listed on the last page');
  const last = viewWith({list_page: '2'}, unavailable)(ranked(30));
  expect(last.match(/data-missing/g)).toHaveLength(1);
});

it('a new sort or filter returns to page 1; paging keeps the rest of the state', () => {
  const setState = runtime.slice(runtime.indexOf('const setState = patch =>'), runtime.indexOf('\n', runtime.indexOf('go(R.path, st); };')));
  let went: any;
  const make = new Function('S', 'R', 'BOOK', 'go', `${setState};return setState;`);
  const set = make({sort: 'cagr', list_page: '5'}, {path: '/strategies'}, {}, (_path: string, st: unknown) => {went = st;});
  set({sort: 'sharpe'});
  expect(went).toMatchObject({sort: 'sharpe', list_page: '1'});
  set({list_page: '6'});
  expect(went).toMatchObject({sort: 'cagr', list_page: '6'});
});
