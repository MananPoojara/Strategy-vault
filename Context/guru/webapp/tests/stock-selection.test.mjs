import test from 'node:test';
import assert from 'node:assert/strict';
import {stockChoices, basketSpan, positionLimit} from '../../frontend/react/src/services/stock-selection.ts';

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
