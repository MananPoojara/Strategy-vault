import test from 'node:test';
import assert from 'node:assert/strict';
import {stockChoices, basketSpan, positionLimit, priceRecoveryPeriod} from '../../frontend/react/src/services/stock-selection.ts';

test('stock selection deduplicates universes, keeps explicit source keys and sorts A–Z', () => {
  const prices = [{ticker:'NIFTY 50'}, {ticker:'SPY'},
    {ticker:'basket:nifty50:ZZZ',symbol:'ZZZ'},
    {ticker:'basket:nse500:AAA',symbol:'AAA'},
    {ticker:'basket:nifty50:AAA',symbol:'AAA'},
    {ticker:'basket:nse500:BAD',price_status:'needs_data'}];
  assert.deepEqual(stockChoices(prices).map(p=>p.ticker), ['basket:nse500:AAA','basket:nifty50:ZZZ']);
  assert.equal(stockChoices(prices,'basket:nifty50:AAA')[0].ticker,'basket:nifty50:AAA');
  assert.equal(prices.length,6);
});

test('qualified stock spelling never changes its admitted asset type to an ETF or index', () => {
  assert.deepEqual(stockChoices([{ticker:'SPY'}, {ticker:'basket:us_sp500:SPY',symbol:'SPY'}])
    .map(p=>p.ticker), ['basket:us_sp500:SPY']);
});

test('basket selector span is source coverage union, including different first/last dates', () => {
  assert.deepEqual(basketSpan([{ticker:'A',first:'2000-01-03',last:'2010-01-01'},
    {ticker:'B',first:'2008-01-01',last:'2020-01-01'}, {ticker:'C',first:'bad',last:'bad'}]),
    {first:'2000-01-03',last:'2020-01-01'});
  assert.equal(basketSpan([{ticker:'A'}]),null);
  assert.equal(basketSpan([{ticker:'A',first:'2020-01-01',last:'2000-01-01'}]),null);
});

test('simultaneous position limit accepts only bounded whole counts, never NaN/fractions/grids', () => {
  for (const value of ['1','10','20','100']) assert.equal(positionLimit(value),Number(value));
  for (const value of ['', '0','-1','2.5','101','001','NaN','Infinity','10:20:1', 10, null])
    assert.equal(positionLimit(value),null);
});

const completePrices={scope:'supplied_daily_observations',from:'2009-05-19',to:'2026-10-01',source_index_sha256:'a'.repeat(64),benchmark_sha256:'b'.repeat(64)};
test('a complete price window offers explicit recovery without silently changing the requested dates',()=>{
 const requested={from:'2000-01-03',to:'2026-10-01'};
 assert.deepEqual(priceRecoveryPeriod(completePrices,requested.from,requested.to),{from:'2009-05-19',to:'2026-10-01'});
 assert.deepEqual(requested,{from:'2000-01-03',to:'2026-10-01'});
 assert.deepEqual(priceRecoveryPeriod(completePrices,'2000-01-03','2020-12-31'),{from:'2009-05-19',to:'2020-12-31'});
 assert.equal(priceRecoveryPeriod(completePrices,'2010-01-01','2020-12-31'),null);
 assert.equal(priceRecoveryPeriod(completePrices,'2000-01-03','2008-12-31'),null);
});
test('price recovery rejects absent, malformed or unqualified coverage rather than promising a backtest',()=>{
 for(const coverage of [undefined,null,{}, {...completePrices,scope:'historical_membership'}, {...completePrices,from:'2009-02-31'}, {...completePrices,to:'2008-01-01'}, {...completePrices,source_index_sha256:''}, {...completePrices,benchmark_sha256:'bad'}])
  assert.equal(priceRecoveryPeriod(coverage,'2000-01-03','2026-10-01'),null);
 assert.equal(priceRecoveryPeriod(completePrices,'bad','2026-10-01'),null);
});
