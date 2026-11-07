import {describe, expect, it} from 'vitest';

import {fixedQuantityDisplay} from '../src/services/return-display';
describe('fixed quantity net profit', () => {
  it('shows the rupee result on the initial capital for sizing 2 only', () => {
    const d = {sizing: {mode: 2, equity: 1000000, alloc: 1000000, final_equity_rs: 1430825.5}, trades: [{open: false}, {open: true}]};
    const x = fixedQuantityDisplay(d)!;
    expect(x.value).toBeCloseTo(0.4308255, 6); expect(x.closedTrades).toBe(1); expect(x.reportMode).toBe('Fixed quantity · ₹4,30,826 on ₹10,00,000');
    expect(fixedQuantityDisplay({sizing: {mode: 1}})).toBeNull(); expect(fixedQuantityDisplay({})).toBeNull();
  });
});
