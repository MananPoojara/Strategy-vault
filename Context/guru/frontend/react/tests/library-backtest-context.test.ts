import {expect, it} from 'vitest';
import {chooseBacktestContext, referenceBacktestContexts} from '../src/services/library-backtest-context';

it('only measured positive family counts qualify a compatible context', () => {
  const selected = {availability: {availableFamilies: 0}, label:'Nifty 50'};
  const stock = {availability: {availableFamilies: 11}, label:'RELIANCE'};
  expect(chooseBacktestContext(selected, [stock])).toBe(stock);
  const measured = {...selected, availability: {availableFamilies: 1}};
  expect(chooseBacktestContext(measured, [stock])).toBe(measured);
  for (const count of [undefined, 0, -.5, NaN, Infinity, 1.5]) {
    expect(chooseBacktestContext(selected, [{label:'Registered only', availability:{availableFamilies:count}}])).toBe(selected);
  }
});
it('uses only existing native prices and bounds stock/ETF reference discovery', () => {
  const ticker = 'basket:nifty50:RELIANCE';
  const baskets = [{id:'nifty50',symbols:[ticker]}];
  expect(referenceBacktestContexts('in', baskets, [{ticker:'NIFTY 50'}], [])).toEqual([]);
  expect(referenceBacktestContexts('in', baskets, [{ticker}], [ {ticker:'SPY'} ])).toEqual([
    {region:'in', tk:ticker, basket:'nifty50', label:'RELIANCE', kind:'stock', from:'2010-01-01', to:'2020-09-11'},
  ]);
  const aapl = 'basket:us_sp500:AAPL';
  const us = referenceBacktestContexts('us', [{id:'us_sp500', symbols:[aapl]}], [{ticker:aapl},{ticker:'SPY'}], [{ticker:'SPY'}]);
  expect(us.map(c=>[c.tk,c.kind])).toEqual([[aapl,'stock'],['SPY','ETF']]);
  const other = 'basket:us_sp500:MSFT';
  expect(referenceBacktestContexts('us',[{id:'us_sp500',symbols:[other]}],[{ticker:other}],[])[0].label).toBe('MSFT');
  expect(referenceBacktestContexts('us',[],[],[{ticker:'SPY'}])).toEqual([]);
});
