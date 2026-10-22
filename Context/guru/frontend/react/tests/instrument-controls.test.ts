import { describe, expect, it } from 'vitest';
import { instrumentControls, instrumentTickers } from '../src/services/instrument-controls';

describe('compatible instrument controls', () => {
  it('explains and disables stock baskets for an index method', () => {
    const controls = instrumentControls({ allowed_types: ['index'] });
    expect(controls.basket).toBe(false);
    expect(controls.index).toBe(true);
    expect(controls.note).toContain('index');
  });
  it('enables actual stocks and disables index substitutes for stock methods', () => {
    const controls = instrumentControls({ allowed_types: ['individual_stock'] });
    expect(controls.basket).toBe(true);
    expect(controls.index).toBe(false);
    expect(controls.note).toContain('stock');
  });
  it('keeps source-account stock selection without implying equal-slot basket execution', () => {
    const controls = instrumentControls({allowed_types: ['individual_stock'], execution_scope: 'single_symbol_source_account'});
    expect([controls.stock,controls.index,controls.basket,controls.etf]).toEqual([true,false,false,false]);
    expect(controls.note).toContain('individual stock risk account');
    expect(controls.note).toContain('position limits are unavailable');
    expect(instrumentControls({allowed_types: ['individual_stock'],execution_scope: 'single_symbol'}).basket).toBe(true);
  });
  it('keeps single-stock percentage research on actual stocks without offering a basket account', () => {
    const controls = instrumentControls({allowed_types:['individual_stock'],execution_scope:'single_symbol_stock_price_research'});
    expect([controls.stock,controls.index,controls.basket,controls.etf]).toEqual([true,false,false,false]);
    expect(controls.note).toContain('percentage research on actual individual stock prices');
    expect(controls.note).toContain('Choose an admitted Stock above');
    expect(controls.note).toContain('basket execution and position limits are unavailable');
    expect(controls.note).toContain('no basket execution has been reviewed');
    expect(controls.note).not.toMatch(/risk account|cash account/);
  });
  it('does not substitute an index or ETF when a single-stock research policy admits no stock', () => {
    for (const allowed_types of [[],['index'],['etf'],['index','etf']]) {
      const controls = instrumentControls({allowed_types,execution_scope:'single_symbol_stock_price_research'});
      expect([controls.stock,controls.index,controls.basket,controls.etf]).toEqual([false,false,false,false]);
      expect(controls.note).toContain('does not admit a Stock');
    }
    const mixed = instrumentControls({allowed_types:['individual_stock','index','etf'],execution_scope:'single_symbol_stock_price_research'});
    expect([mixed.stock,mixed.index,mixed.basket,mixed.etf]).toEqual([true,false,false,false]);
  });
  it('preserves type-driven behavior for unknown execution scopes', () => {
    for (const execution_scope of ['single_symbol','unknown_future_scope','single_symbol_stock_price_research_v2']) {
      for (const allowed_types of [[],['individual_stock'],['index','etf','individual_stock']]) {
        expect(instrumentControls({allowed_types,execution_scope})).toEqual(instrumentControls({allowed_types}));
      }
    }
  });
  it('exposes ETFs for ETF methods without offering stock or index substitutes', () => {
    const controls = instrumentControls({ allowed_types: ['etf'] });
    expect([controls.index, controls.basket, controls.etf]).toEqual([false, false, true]);
    expect(controls.note).toContain('ETF');
  });
  it('allows every source-supported type for a mixed method', () => {
    const controls = instrumentControls({ allowed_types: ['index', 'etf', 'individual_stock'] });
    expect([controls.index, controls.basket, controls.etf]).toEqual([true, true, true]);
    expect(controls.note).toBe('');
  });
  it('keeps data navigation available during source review without asserting compatibility', () => {
    for (const policy of [undefined, {}, {allowed_types: []}]) {
      const controls = instrumentControls(policy);
      expect(controls.index).toBe(true);
      expect(controls.basket).toBe(true);
      expect(controls.etf).toBe(false);
      expect(controls.note).toBe('');
    }
  });
});

describe('route ticker choices match visible instrument controls', () => {
  const indices = [{ticker: 'SPX'}, {ticker: 'NDX'}, {ticker: 'DJI'}];
  const etfs = ['SPY', 'QQQ', 'MDY', 'IWM', 'SMH'].map(ticker => ({ticker}));

  it('retains every visible ETF choice for an index/ETF method', () => {
    const choices = instrumentTickers({allowed_types: ['index', 'etf']}, indices, etfs);
    for (const ticker of etfs) expect(choices).toContain(ticker);
    expect(choices).toContain(indices[0]);
  });

  it('retains ETF choices when individual stocks are also supported', () => {
    const choices = instrumentTickers({allowed_types: ['index', 'etf', 'individual_stock']}, indices, etfs);
    expect(choices).toContain(etfs[1]);
    expect(choices).toContain(indices[0]);
  });

  it('keeps an ETF-only method on actual ETF choices', () => {
    expect(instrumentTickers({allowed_types: ['etf']}, indices, etfs)).toEqual(etfs);
  });

  it('offers no index or ETF route in the single-stock percentage research scope', () => {
    for (const allowed_types of [[],['individual_stock'],['index'],['etf'],['individual_stock','index','etf']]) {
      expect(instrumentTickers({allowed_types,execution_scope:'single_symbol_stock_price_research'},indices,etfs)).toEqual([]);
    }
    expect(instrumentTickers({allowed_types:['index','etf'],execution_scope:'unknown_future_scope'},indices,etfs)).toEqual([...indices,...etfs]);
  });

  it('does not offer ETF choices for index-only or unresolved policies', () => {
    expect(instrumentTickers({allowed_types: ['index']}, indices, etfs)).toEqual(indices);
    expect(instrumentTickers(undefined, indices, etfs)).toEqual(indices);
  });
});

it('describes the reviewed Bandy single-instrument cash account without imposing a false source ETF ban',()=>{
 const controls=instrumentControls({allowed_types:['individual_stock'],execution_scope:'single_symbol_source_account'},'hbiab_ma5_20','hbiab');
 expect([controls.stock,controls.basket,controls.index,controls.etf]).toEqual([true,false,false,false]);
 expect(controls.note).toContain('cash account');expect(controls.note).toContain('admitted Stock above');
 const mixed=instrumentControls({allowed_types:['individual_stock','etf'],execution_scope:'single_symbol_source_account'},'hbiab_macd_example','hbiab');
 expect(mixed.note).toContain('Stock or ETF');expect(mixed.basket).toBe(false);
 const etf=instrumentControls({allowed_types:['etf'],execution_scope:'single_symbol_source_account'},'hbiab_macd_example','hbiab');
 expect(etf.note).toContain('admitted ETF above');expect(etf.basket).toBe(false);
 expect(controls.note).not.toContain('risk account');
});

describe('declared open-instrument application', () => {
  it('routes a stock-research rule to index/ETF only when the API declares it', () => {
    const policy = {allowed_types:['index','etf','individual_stock'],execution_scope:'single_symbol_stock_price_research',declared_application:'Book tests stocks; index and ETF runs are a declared application.'};
    for (const id of [undefined,'lkqts_sob']) {
      const c = instrumentControls(policy,id,'lkqts');
      expect([c.stock,c.index,c.basket,c.etf]).toEqual([true,true,false,true]);
    }
  });
});
