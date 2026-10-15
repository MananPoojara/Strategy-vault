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

  it('does not offer ETF choices for index-only or unresolved policies', () => {
    expect(instrumentTickers({allowed_types: ['index']}, indices, etfs)).toEqual(indices);
    expect(instrumentTickers(undefined, indices, etfs)).toEqual(indices);
  });
});
