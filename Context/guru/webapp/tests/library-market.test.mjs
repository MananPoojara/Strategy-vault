import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {test} from 'node:test';
import vm from 'node:vm';
import {libraryMarket, libraryMarketVariants} from '../../frontend/react/src/services/library-market.ts';

const variants = [
  {id: 'shared', family: 'shared', status: 'live'},
  {id: 'india_breadth', family: 'breadth', needs_series: ['NSE_ADVANCES']},
  {id: 'us_breadth', family: 'breadth', needs_series: ['NYSE_ADVANCES']},
  {id: 'held', family: 'held', status: 'not_testable'},
];
test('All browses both catalogues without duplicating shared rules or changing admission', () => {
  const before = JSON.stringify(variants);
  assert.deepEqual(libraryMarketVariants(variants, 'all').map(v => v.id), variants.map(v => v.id));
  assert.equal(libraryMarketVariants(variants, 'all').filter(v => v.id === 'shared').length, 1);
  assert.equal(libraryMarketVariants(variants, 'all').find(v => v.id === 'held').status, 'not_testable');
  assert.equal(JSON.stringify(variants), before);
  assert.deepEqual(libraryMarketVariants(variants, 'in').map(v => v.id), ['shared', 'india_breadth', 'held']);
  assert.deepEqual(libraryMarketVariants(variants, 'us').map(v => v.id), ['shared', 'us_breadth', 'held']);
});
test('All remains a catalogue filter; individual backtests and optimization use their actual market', () => {
  for (const region of ['in', 'us']) {
    const state = {region, library_market: 'all', tk: region === 'us' ? 'QQQ' : 'NIFTY 50'};
    for (const route of ['home', 'guru', 'all']) assert.equal(libraryMarket(state, route), 'all');
    for (const route of ['strategy', 'optimization', 'saved', 'source']) assert.equal(libraryMarket(state, route), region);
    assert.equal(state.region, region);
  }
  assert.equal(libraryMarket({region: 'us'}), 'us');
  assert.equal(libraryMarket({region: 'in', library_market: 'invalid'}), 'in');
  assert.equal(libraryMarket({region: 'all'}), 'in');
});

const runtime = readFileSync(new URL('../../frontend/react/src/compat/runtime.js', import.meta.url), 'utf8');
const start = runtime.indexOf('const foreignTo ='), end = runtime.indexOf('// ---------------------------------------------------------------- shared bits', start);
assert.ok(start >= 0 && end > start);
function routing(state, name = 'home') {
  const context = vm.createContext({
    URLSearchParams, libraryMarket, libraryMarketVariants, ALLCAT: variants, S: state,
    R: {name, path: '/'}, DEF: {region: 'in', library_market: ''}, BOOK: {}, location: {},
  });
  vm.runInContext(runtime.slice(start, end) + '\nglobalThis.routes = {marketCat, href, setState};', context);
  return context.routes;
}
test('the production catalogue includes both markets only on catalogue routes', () => {
  for (const route of ['home', 'guru', 'all']) {
    assert.deepEqual(Array.from(routing({region: 'in', library_market: 'all'}, route).marketCat(), v => v.id), variants.map(v => v.id));
  }
  assert.deepEqual(Array.from(routing({region: 'us', library_market: 'all'}, 'strategy').marketCat(), v => v.id), ['shared', 'us_breadth', 'held']);
});
test('actual share links retain All while browsing and remove it from backtest links', () => {
  const state = {region: 'us', library_market: 'all', tk: 'QQQ'}, routes = routing(state);
  for (const path of ['/', '/guru/connors', '/strategies', '/source/reading-entry']) {
    const query = new URLSearchParams(routes.href(path).split('?')[1]);
    assert.equal(query.get('library_market'), 'all');
    assert.equal(query.get('region'), 'us');
  }
  for (const path of ['/s/kftsm_ma', '/o/kftsm_ma', '/saved']) {
    const query = new URLSearchParams(routes.href(path, state).split('?')[1]);
    assert.equal(query.has('library_market'), false);
    assert.equal(query.get('region'), 'us');
    assert.equal(query.get('tk'), 'QQQ');
  }
});
