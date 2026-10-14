import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {orderAuthors} from '../../frontend/react/src/services/author-order.ts';
import {chooseBacktestContext, referenceBacktestContexts} from '../../frontend/react/src/services/library-backtest-context.ts';
import {libraryScope, inLibraryScope} from '../../frontend/react/src/services/library-scope.ts';
import {libraryMarket} from '../../frontend/react/src/services/library-market.ts';

const require = createRequire(import.meta.url);
const discovery = require('../../frontend/library-discovery.js');
const browser = require('../../frontend/library-browser.js');
const truth = require('../../frontend/backtest-trust.js');
const runtime = readFileSync(new URL('../../frontend/react/src/compat/runtime.js', import.meta.url), 'utf8');

function productionBlock(start, end) {
  const first = runtime.indexOf(start), last = runtime.indexOf(end, first);
  assert.ok(first >= 0 && last > first, `Production block remains discoverable: ${start}`);
  return runtime.slice(first, last);
}

const source = [
  productionBlock('const href = ', 'const stratLink = '),
  productionBlock('async function heroStats(', '\nfunction libraryInstrumentHTML'),
  productionBlock('async function rows(S = getS())', '\n// ---------------------------------------------------------------- small charts'),
  productionBlock('async function homeModel(){', '\nlet startup;'),
].join('\n');

const NIFTY = 'NIFTY 50', STOCK = 'basket:nifty50:RELIANCE';
const FROM = '2024-01-01', TO = '2025-12-31';
const SAMPLE_FROM = '2010-01-01', SAMPLE_TO = '2020-09-11';
const measured = (sharpe = 0.5) => ({stats: {trades: 30, net_profit: 0.1, sharpe, t_stat: 2}, spark: [1, 1.1]});

function fixture({selected = NIFTY, allUnmeasured = false, selectedStockFamilies = 0} = {}) {
  // These are test-only records for route/count behavior, not strategy or price results.
  const authors = [
    {id: 'connors', name: 'Larry Connors', books: [{id: 'stc9', title: 'Swing Trading College', live: true}]},
    {id: 'bandy', name: 'Howard Bandy', books: [{id: 'hbqts', title: 'Quantitative Trading Systems', live: true}]},
    {id: 'reading', name: 'Reading Author', books: [{id: 'reading_book', title: 'Trading Psychology', live: false}]},
    {id: 'untested', name: 'Untested Author', books: [{id: 'untested_book', title: 'Stock Rules', live: true}]},
  ];
  const rule = (id, guru, book, family, types) => ({
    id, guru, book, family, family_name: family, name: id, dir: 'Long', group: 'Daily', rules: [],
    status: 'live', instrument_policy: {requirement: types.length === 1 && types[0] === 'individual_stock' ? 'individual_stock' : 'index_or_etf', allowed_types: types},
  });
  const variants = [rule('index_method', 'connors', 'stc9', 'index_family', ['index', 'etf'])];
  for (let i = 0; i < 11; i++) variants.push(rule(`stock_method_${i}`, 'bandy', 'hbqts', `stock_family_${i}`, ['individual_stock']));
  variants.push(rule('stock_sibling', 'bandy', 'hbqts', 'stock_family_0', ['individual_stock']));
  variants.push(rule('registered_only', 'untested', 'untested_book', 'unmeasured_family', ['individual_stock']));
  const calls = [], stockSummary = Object.fromEntries(variants.filter(v => v.guru === 'bandy').map(v => [v.id, measured()]));
  const currentStockSummary = Object.fromEntries(variants.filter(v => v.guru === 'bandy' && v.id.startsWith('stock_method_') && Number(v.id.split('_').at(-1)) < selectedStockFamilies).map(v => [v.id, measured()]));
  const state = {
    region: 'in', market: 'india', tk: selected, basket: selected === STOCK ? 'nifty50' : '',
    from: FROM, to: TO, library_scope: 'trading', library_view: 'authors', library_sort: 'default',
    library_q: '', book_category: '', library_page: '1', tab: 'sharpe', q: '', sort: 'sharpe',
    guru: '', book: '', style: '', dir: '', family: '', hold: '', verdict: '', min: '0',
  };
  const context = {
    URLSearchParams, S: state, R: {name: 'home', path: '/'}, DEF: {tk: NIFTY, region: 'in', basket: '', from: '', to: '', tab: 'sig'},
    LIB: authors, GURU: Object.fromEntries(authors.map(a => [a.id, a])),
    BOOK: Object.fromEntries(authors.flatMap(a => a.books.map(b => [b.id, {...b, guru: a}]))),
    CAT: variants, ALLCAT: variants, CURATED_AUTHOR_IDS: new Set(authors.map(a => a.id)),
    TICK_ALL_IN: [{ticker: NIFTY}, {ticker: STOCK}], TICK_ALL_US: [],
    TICK_IN: [{ticker: NIFTY}], TICK_US: null, TICKERS: [{ticker: selected}],
    BASKETS: [{id: 'nifty50', name: 'Nifty 50', symbols: [STOCK]}], LABEL: {[NIFTY]: 'Nifty 50', [STOCK]: 'RELIANCE'},
    HOME_STATS: {}, GUILL: '', VaultLibraryDiscovery: discovery, VaultLibraryBrowser: browser, TRUST: truth,
    orderAuthors, chooseBacktestContext, referenceBacktestContexts, libraryScope, inLibraryScope, libraryMarket,
    libraryRecords: () => browser.records(authors, {}, {}),
    groupBy: (records, key) => {
      const grouped = new Map();
      for (const record of records) {const id = key(record); if (!grouped.has(id)) grouped.set(id, []); grouped.get(id).push(record);}
      return grouped;
    },
    getS: () => state,
    getSum: async (tk, from = '', to = '') => {
      calls.push({kind: 'summary', tk, from, to});
      if (allUnmeasured) return {};
      if (tk === NIFTY) return {index_method: measured()};
      if (tk === STOCK && from === SAMPLE_FROM && to === SAMPLE_TO) return stockSummary;
      if (tk === STOCK && from === FROM && to === TO) return currentStockSummary;
      return {};
    },
    getReadiness: async (tk, from = '', to = '') => {
      calls.push({kind: 'readiness', tk, from, to});
      return Object.fromEntries(variants.map(v => [v.id,
        v.instrument_policy.allowed_types.includes('individual_stock') && tk === NIFTY
          ? {status: 'needs_data', note: 'Requires a validated individual stock; selected index is not a constituent stock.'}
          : {status: 'ready', note: ''},
      ]));
    },
    photoOf: () => null, initials: name => name.slice(0, 1), gTag: () => '', fx: value => String(value),
    sortRows: records => records, TABS: {sharpe: {win: () => ({from: '', to: ''})}, awaiting: {}},
  };
  vm.createContext(context);
  vm.runInContext(source + '\nglobalThis.production = {homeModel, heroStats, rows};', context);
  return {context, state, calls, variants, production: context.production};
}

const queryOf = href => new URLSearchParams(href.split('?')[1] || '');

test('production Guru cards find measured Bandy stock results, label them and route to the same native stock and period', async () => {
  const f = fixture(), model = await f.production.homeModel();
  assert.deepEqual(Array.from(model.authorCards, card => card.id), ['bandy', 'connors', 'reading', 'untested']);
  const bandy = model.authorCards.find(card => card.id === 'bandy');
  assert.equal(bandy.available, 11); // sibling variants do not become extra families
  assert.equal(bandy.contextChanged, true);
  assert.equal(bandy.availableContext, 'RELIANCE · stock · India · 2010-01-01 to 2020-09-11');
  assert.equal(bandy.openLabel, 'Open stock backtests');
  assert.ok(bandy.href.startsWith('#/guru/bandy?'));
  const q = queryOf(bandy.href);
  assert.equal(q.get('tk'), STOCK);
  assert.equal(q.get('basket'), 'nifty50');
  assert.equal(q.get('from'), SAMPLE_FROM);
  assert.equal(q.get('to'), SAMPLE_TO);
  assert.equal(q.get('tab'), 'sharpe');
  assert.equal(model.authorStats.bandy.tested, 11);
  assert.equal(model.authorStats.bandy.tk, STOCK);
  assert.equal(model.authorStats.bandy.href, bandy.href);
  assert.equal(f.calls.filter(c => c.kind === 'summary' && c.tk === STOCK && c.from === SAMPLE_FROM && c.to === SAMPLE_TO).length, 1);
});

test('a measured current index card retains its selected context and explicit dates', async () => {
  const f = fixture(), model = await f.production.homeModel();
  const connors = model.authorCards.find(card => card.id === 'connors');
  assert.equal(connors.available, 1);
  assert.equal(connors.contextChanged, false);
  assert.equal(connors.availableContext, 'Nifty 50 · index · India · 2024-01-01 to 2025-12-31');
  const q = queryOf(connors.href);
  assert.equal(q.get('from'), FROM);
  assert.equal(q.get('to'), TO);
  assert.equal(q.get('basket'), null);
  assert.equal(q.get('tk') || f.context.DEF.tk, NIFTY);
  assert.equal(model.authorStats.connors.tk, NIFTY);
});

test('production homeModel stages modal statistics without changing the currently committed statistics', async () => {
  const f = fixture(), committed = Object.freeze({existing: Object.freeze({tk: 'old selection', tested: 7})});
  f.context.HOME_STATS = committed;
  const model = await f.production.homeModel();
  assert.equal(f.context.HOME_STATS, committed);
  assert.notEqual(model.authorStats, committed);
  assert.equal(model.authorStats.bandy.tested, 11);
  assert.equal(model.authorStats.bandy.tk, STOCK);
});

test('production render publishes only the newest completed home modal statistics', async () => {
  const queued = [], committed = [], failures = [], initial = {existing: {tk: 'initial'}};
  let cleanupCalls = 0, tickets = 0;
  const context = {
    busy: 0, HOME_STATS: initial, STRAT: null, LIST_HASH: '', pendingFocus: null,
    R: {name: 'home', path: '/'}, S: {tk: NIFTY, tab: 'sharpe'}, location: {hash: '#/'},
    customPages: [], resolveRoute: () => ({path: '/'}), bookCtx: () => {},
    homeModel: () => new Promise(resolve => queued.push(resolve)),
    cleanupCharts: () => { cleanupCalls++; }, document: {activeElement: null},
    isUS: () => false, LABEL: {[NIFTY]: 'Nifty 50'}, TABS: {sharpe: {name: 'Top Sharpe'}},
    vaultStore: {
      begin: () => ++tickets,
      commit: (ticket, result) => committed.push({ticket, result}),
      fail: (ticket, message) => failures.push({ticket, message}),
    },
  };
  vm.createContext(context);
  vm.runInContext(productionBlock('async function render() {', '\nlet pendingFocus=') + '\nglobalThis.runRender = render;', context);
  const older = context.runRender(), newer = context.runRender();
  assert.equal(queued.length, 2);
  const newestStats = {bandy: {tk: STOCK, tested: 11}}, staleStats = {bandy: {tk: NIFTY, tested: 0}};
  queued[1]({authorStats: newestStats});
  await newer;
  assert.equal(context.HOME_STATS, newestStats);
  queued[0]({authorStats: staleStats});
  await older;
  assert.equal(context.HOME_STATS, newestStats);
  assert.equal(committed.length, 1);
  assert.equal(committed[0].ticket, 2);
  assert.equal(cleanupCalls, 1);
  assert.deepEqual(failures, []);
});

test('measured current stock results stay selected even when the reference window has more families', async () => {
  const f = fixture({selected: STOCK, selectedStockFamilies: 3}), model = await f.production.homeModel();
  const bandy = model.authorCards.find(card => card.id === 'bandy');
  assert.equal(bandy.available, 3);
  assert.equal(bandy.contextChanged, false);
  assert.equal(queryOf(bandy.href).get('from'), FROM);
  assert.equal(queryOf(bandy.href).get('to'), TO);
  assert.ok(bandy.availableContext.endsWith(`${FROM} to ${TO}`));
});

test('live registrations and admitted prices without measured summaries invent neither counts nor alternate routes', async () => {
  const f = fixture({allUnmeasured: true}), model = await f.production.homeModel();
  assert.ok(model.authorCards.every(card => card.available === 0));
  assert.ok(model.authorCards.every(card => card.contextChanged === false));
  assert.ok(model.authorCards.every(card => card.availableContext === undefined));
  assert.deepEqual(Array.from(model.authorCards, card => card.id), ['connors', 'bandy', 'reading', 'untested']);
  for (const card of model.authorCards) {
    const q = queryOf(card.href);
    assert.equal(q.get('basket'), null);
    assert.equal(q.get('from'), FROM);
    assert.equal(q.get('to'), TO);
  }
  assert.equal(model.authorCards.find(card => card.id === 'reading').status, 'reading_only');
});

test('production heroStats requests the exact selected period for both overall and author summaries', async () => {
  const f = fixture();
  const total = await f.production.heroStats();
  const author = await f.production.heroStats('connors');
  assert.equal(total.tested, 1);
  assert.equal(author.tested, 1);
  assert.ok(f.calls.length === 2 && f.calls.every(c => c.tk === NIFTY && c.from === FROM && c.to === TO));
});

test('production Sharpe rows use explicit route dates rather than silently widening the selected window', async () => {
  const f = fixture({selected: STOCK}), q = queryOf((await fixture().production.homeModel()).authorCards.find(card => card.id === 'bandy').href);
  const route = {...f.state, guru: 'bandy', tk: q.get('tk'), basket: q.get('basket'), from: q.get('from'), to: q.get('to'), tab: q.get('tab')};
  f.calls.length = 0;
  const rows = await f.production.rows(route);
  assert.equal(rows.length, 12); // 11 measured families plus one sibling variant
  assert.ok(rows.every(row => row.tk === STOCK && row.w.from === SAMPLE_FROM && row.w.to === SAMPLE_TO));
  assert.deepEqual(f.calls, [{kind: 'summary', tk: STOCK, from: SAMPLE_FROM, to: SAMPLE_TO}]);
});

test('production Sharpe rows preserve the full-history default when no explicit period is selected', async () => {
  const f = fixture();
  await f.production.rows({...f.state, from: '', to: ''});
  assert.deepEqual(f.calls, [{kind: 'summary', tk: NIFTY, from: '', to: ''}]);
});
