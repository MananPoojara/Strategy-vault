import assert from 'node:assert/strict';
import {test} from 'node:test';
import {chooseBacktestContext, referenceBacktestContexts} from '../../frontend/react/src/services/library-backtest-context.ts';

const indices = {region:'in', tk:'NIFTY 50', basket:'', label:'Nifty 50', kind:'index', from:'', to:''};
const stocks = {region:'in', tk:'basket:nifty50:RELIANCE', basket:'nifty50', label:'RELIANCE', kind:'stock', from:'2010-01-01', to:'2020-09-11'};
const current = {context:indices, availability:{availableFamilies:0}};
test('a measured stock context makes Bandy discoverable without claiming an index result', () => {
  const reference = {context:stocks, availability:{availableFamilies:11}};
  assert.equal(chooseBacktestContext(current, [reference]), reference);
  assert.equal(current.availability.availableFamilies, 0);
  assert.equal(current.context.tk, 'NIFTY 50');
});
test('preserves results for the selected instrument even when another has more families', () => {
  const selected = {...current, availability:{availableFamilies:1}};
  assert.equal(chooseBacktestContext(selected, [{context:stocks, availability:{availableFamilies:11}}]), selected);
});
test('live registration, missing results, failed results and invalid counts cannot create availability', () => {
  for(const count of [undefined, 0, -1, NaN, Infinity, .5]) {
    assert.equal(chooseBacktestContext(current, [{context:stocks, availability:{availableFamilies:count, status:'live'}}]), current);
  }
});
test('reference selection is bounded to one real stock and one actual US ETF', () => {
  const inPool = [{ticker:stocks.tk}, {ticker:'basket:nifty50:TCS'}];
  const inBaskets = [{id:'nifty50', symbols:inPool.map(t=>t.ticker)}];
  assert.deepEqual(referenceBacktestContexts('in', inBaskets, inPool, []), [stocks]);
  const usStock = 'basket:us_sp500:AAPL';
  const usPool = [{ticker:usStock}, {ticker:'SPY'}, {ticker:'QQQ'}];
  const actual = referenceBacktestContexts('us', [{id:'us_sp500',symbols:[usStock]}], usPool, [{ticker:'SPY'},{ticker:'QQQ'}]);
  assert.equal(actual.length, 2);
  assert.equal(actual[0].tk, usStock);
  assert.equal(actual[0].kind, 'stock');
  assert.equal(actual[1].tk, 'SPY');
  assert.equal(actual[1].basket, '');
  assert.equal(actual[1].kind, 'ETF');
  assert.ok(actual.every(c=>c.region==='us'));
});
test('missing native prices stay absent; no index is substituted for a stock or ETF', () => {
  assert.deepEqual(referenceBacktestContexts('in', [{id:'nifty50',symbols:[stocks.tk]}], [{ticker:'NIFTY 50'}], []), []);
  assert.deepEqual(referenceBacktestContexts('us', [], [{ticker:'SPX'}], []), []);
});
