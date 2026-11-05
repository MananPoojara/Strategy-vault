import {describe, expect, it} from 'vitest';
import {defaultTicker, instrumentTickers} from '../src/services/instrument-controls';
import {reasonLabel} from '../src/services/library-progress';

// lane F3: multi-ETF rotation rules (gate allowed ["SPY"]) open on SPY and name their refusal plainly.
const etfs = ['AGG','EEM','SPY','XLK'].map(ticker => ({ticker}));
describe('multi-ETF rotation presentation', () => {
  it('opens a rotation rule on SPY, not the first ETF', () => {
    const offered = instrumentTickers({allowed_types:['etf']}, [{ticker:'SPX'}], etfs);
    expect(defaultTicker(offered, ['SPY'])).toBe('SPY');
    expect(defaultTicker(offered, null)).toBe('AGG');
    expect(defaultTicker(offered, [])).toBe('AGG');
    expect(defaultTicker([], ['SPY'])).toBeUndefined();
  });
  it('labels the rotation refusal ahead of the fixed-amount label', () => {
    const note = 'Not published on AGG. Multi-ETF rotation: runs only in the SPY context (SPY is the calendar and benchmark; the rule holds the ETFs named in its trades). Open it on SPY for the results.';
    expect(reasonLabel(note)).toBe('Runs on SPY only (multi-ETF rotation)');
    expect(reasonLabel('this rule uses fixed point or dollar amounts set for SPX')).toBe('Fixed amounts set for another market');
  });
});
