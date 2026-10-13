import { describe, expect, it } from 'vitest';
import { instrumentControls } from '../src/services/instrument-controls';

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
