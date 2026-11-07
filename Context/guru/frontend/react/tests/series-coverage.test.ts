import {describe, expect, it} from 'vitest';
import {strategyDataWindow} from '../src/services/series-coverage';

const span = {first: '2000-01-03', last: '2026-10-21'};

describe('strategyDataWindow', () => {
  it('is the intersection of the required series when it is narrower than the instrument history', () => {
    const coverage = {'BTST_5M.C1525': ['2019-02-11', '2026-09-29'], 'BTST_5M.C0915': ['2019-02-12', '2026-09-30']} as const;
    expect(strategyDataWindow(['BTST_5M.C1525', 'BTST_5M.C0915'], {...coverage} as never, span)).toEqual({from: '2019-02-12', to: '2026-09-29'});
  });
  it('needs no preset when the strategy requires nothing, coverage is unknown or one series is missing', () => {
    expect(strategyDataWindow([], {}, span)).toBeNull();
    expect(strategyDataWindow(undefined, {}, span)).toBeNull();
    expect(strategyDataWindow(['A'], undefined, span)).toBeNull();
    expect(strategyDataWindow(['A', 'B'], {A: ['2019-01-01', '2020-01-01']}, span)).toBeNull();
  });
  it('needs no preset when the series cover the whole instrument history or do not overlap', () => {
    expect(strategyDataWindow(['A'], {A: ['2000-01-03', '2026-10-21']}, span)).toBeNull();
    expect(strategyDataWindow(['A', 'B'], {A: ['2019-01-01', '2019-06-01'], B: ['2020-01-01', '2020-06-01']}, span)).toBeNull();
  });
});
