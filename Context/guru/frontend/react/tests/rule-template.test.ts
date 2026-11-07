import {describe, expect, it} from 'vitest';
import {paramValues, renderRule} from '../src/services/rule-template';

describe('rule templates', () => {
  const v = {entry_time: 1450, exit_time: 925, pb_pct: 0, hardstop: 2, flt_type: 1, flt_tf: 2, st_period: 20, st_factor: 3, days: 2};
  it('fills numbers, times and options', () => {
    expect(renderRule('Buy on the {entry_time:t} bar close', v)).toBe('Buy on the 14:50 bar close');
    expect(renderRule('Sell on the first bar from {exit_time:t} next day', v)).toBe('Sell on the first bar from 09:25 next day');
    expect(renderRule('Supertrend({st_period}, {st_factor}) on {flt_tf:opt:Daily/Weekly/Monthly} bars is up', v)).toBe('Supertrend(20, 3) on Weekly bars is up');
    expect(renderRule('{days} closes', {days: 2.5})).toBe('2.5 closes');
  });
  it('shows a guarded line only when the guard holds', () => {
    expect(renderRule('?pb_pct>0|Profit booking {pb_pct}%', v)).toBeNull();
    expect(renderRule('?hardstop>0|Hard stop {hardstop}%', v)).toBe('Hard stop 2%');
    expect(renderRule('?flt_type=3|No trend filter', v)).toBeNull();
    expect(renderRule('?flt_type!=3|Trend filter on', v)).toBe('Trend filter on');
  });
  it('leaves unknown keys and plain lines untouched', () => {
    expect(renderRule('Buy on the close', v)).toBe('Buy on the close');
    expect(renderRule('At {nope:t}', v)).toBe('At {nope:t}');
    expect(renderRule('?nope>0|Shown when the value is unknown', v)).toBe('Shown when the value is unknown');
    expect(renderRule('{flt_tf:opt:Daily}', {flt_tf: 3})).toBe('3');
  });
  it('merges schema defaults with page overrides', () => {
    expect(paramValues([{key: 'a', def: 1}, {key: 'b', def: 2}], {b: 5})).toEqual({a: 1, b: 5});
    expect(paramValues(undefined, {b: 5})).toEqual({b: 5});
  });
});
