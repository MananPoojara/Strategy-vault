import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {test} from 'node:test';
import vm from 'node:vm';
import {instrumentTickers, instrumentControls} from '../../frontend/react/src/services/instrument-controls.ts';
import {stockChoices,positionLimit} from '../../frontend/react/src/services/stock-selection.ts';

// Run the production route parser and click transition, rather than a duplicate
// implementation. These checks cover navigation; backend readiness remains the
// authority for whether the selected real instrument may publish a backtest.
const runtime = readFileSync(new URL('../../frontend/react/src/compat/runtime.js', import.meta.url), 'utf8');
function sourceSection(start, end) {
  const first = runtime.indexOf(start), last = runtime.indexOf(end, first);
  assert.ok(first >= 0 && last > first, `Production route section must exist: ${start}`);
  return runtime.slice(first, last);
}
const defaults = runtime.match(/^const DEF = .*;$/m)?.[0];
const extendedDefaults = runtime.match(/^DEF\.view = .*$/m)?.[0];
assert.ok(defaults && extendedDefaults, 'Production route defaults must exist');
const parser = sourceSection('function readRoute()', '// Market-specific variants');
const transitions = sourceSection('const href = (path, st = {})', '// ---------------------------------------------------------------- shared bits');
const etfSymbols = ['SPY', 'QQQ', 'MDY', 'IWM', 'SMH'];
const indices = [{ticker: 'SPX'}, {ticker: 'NDX'}, {ticker: 'DJI'}];
const indianIndices = [{ticker: 'NIFTY 50'}, {ticker: 'NIFTYBANK'}, {ticker: 'NIFTYMIDCAP100'}];
const usStock = {ticker: 'basket:us_sp500:AAPL'}, indiaStock = {ticker: 'basket:nifty50:RELIANCE'};

function routeHarness(types, hash = '#/s/reviewed?region=us&tk=SPX') {
  let selectedHash = hash;
  const location = {};
  Object.defineProperty(location, 'hash', {
    get: () => selectedHash,
    set: value => { selectedHash = value.startsWith('#') ? value : '#' + value; },
  });
  const definition = {id: 'reviewed', book: 'stc9', instrument_policy: {allowed_types: types, preferred_basket: 'nifty50'}};
  const context = vm.createContext({
    location, URLSearchParams, instrumentTickers, instrumentControls, stockChoices, positionLimit,
    GURU: {connors: {}}, BOOK: {}, S: {}, R: {},
    API_IN: '/api', API_US: '/api/us', API: '',
    TICK_IN: indianIndices, TICK_US: indices,
    TICK_ALL_IN: [...indianIndices, indiaStock],
    TICK_ALL_US: [...indices, ...etfSymbols.map(ticker => ({ticker})), usStock],
    TICKERS: [], BASKETS: [],
    BASKETS_IN: [{id: 'nifty50', symbols: [indiaStock.ticker]}],
    BASKETS_US: [{id: 'us_sp500', symbols: [usStock.ticker]}],
    ALLCAT: [definition], CAT: [],
    marketCat: () => [definition],
  });
  vm.runInContext(`${defaults}\n${extendedDefaults}\n${parser}\n${transitions}\nglobalThis.router = {readRoute, setState};`, context);
  context.router.readRoute();
  return context;
}

test('every visible ETF survives the actual mixed-method click transition', () => {
  for (const types of [['index', 'etf'], ['index', 'etf', 'individual_stock']]) {
    for (const ticker of etfSymbols) {
      const app = routeHarness(types);
      app.router.setState({region: 'us', basket: '', tk: ticker, from: '', to: ''});
      app.router.readRoute();
      assert.equal(app.S.tk, ticker, `${types.join('/')} must retain clicked ${ticker}`);
      assert.equal(new URLSearchParams(app.location.hash.split('?')[1]).get('tk'), ticker);
      assert.ok(app.TICKERS.some(item => item.ticker === ticker));
      assert.ok(app.TICKERS.some(item => item.ticker === 'SPX'));
      assert.equal(app.API, '/api/us');
    }
  }
});

test('ETF-only links retain actual ETFs and do not offer cash indices', () => {
  for (const ticker of etfSymbols) {
    const app = routeHarness(['etf'], `#/s/reviewed?region=us&tk=${ticker}`);
    assert.equal(app.S.tk, ticker);
    assert.ok(app.TICKERS.every(item => etfSymbols.includes(item.ticker)));
  }
});

test('library ETF selectors and Bandy links retain the actual ETF outside a strategy route', () => {
  for (const path of ['/', '/guru/connors', '/strategies']) {
    for (const ticker of etfSymbols) {
      const app = routeHarness(['index'], `#${path}?region=us&tk=${ticker}`);
      assert.equal(app.S.tk, ticker);
      assert.ok(app.TICKERS.some(item => item.ticker === 'SPX'));
      assert.ok(app.TICKERS.some(item => item.ticker === ticker));
    }
  }
});

test('index-only and unresolved policies retain index navigation without claiming ETF compatibility', () => {
  for (const types of [['index'], []]) {
    const app = routeHarness(types, '#/s/reviewed?region=us&tk=QQQ');
    assert.equal(app.S.tk, 'SPX');
    assert.deepEqual(Array.from(app.TICKERS, item => item.ticker), ['SPX', 'NDX', 'DJI']);
  }
});

test('mixed-method Indian links retain admitted local index navigation', () => {
  const app = routeHarness(['index', 'etf'], '#/s/reviewed?region=in&tk=NIFTYBANK');
  assert.equal(app.S.tk, 'NIFTYBANK');
  assert.equal(app.API, '/api');
  assert.ok(app.TICKERS.every(item => !etfSymbols.includes(item.ticker)));
});

test('stock-only navigation selects a native stock without claiming a combined basket result', () => {
  for (const [region, basket, ticker] of [['in', 'nifty50', indiaStock.ticker], ['us', 'us_sp500', usStock.ticker]]) {
    const app = routeHarness(['individual_stock'], `#/s/reviewed?region=${region}`);
    assert.equal(app.S.basket, '');
    assert.equal(app.S.run_scope, 'single');
    assert.equal(app.S.tk, ticker);
    assert.deepEqual(Array.from(app.TICKERS, item => item.ticker), [ticker]);
  }
});

test('explicit native basket URLs stay intact even when the readiness gate must reject the method', () => {
  for (const types of [['index'], ['etf']]) {
    const app = routeHarness(types, `#/s/reviewed?region=us&basket=us_sp500&tk=${encodeURIComponent(usStock.ticker)}`);
    assert.equal(app.S.basket, 'us_sp500');
    assert.equal(app.S.tk, usStock.ticker);
    assert.ok(!app.ALLCAT[0].instrument_policy.allowed_types.includes('individual_stock'));
  }
});
