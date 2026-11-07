import {describe, expect, it} from 'vitest';
import {athOptimizeQuery, athOptParams, comboCount, defaultRange, gridProblem, optGridHeight, rangeValues, sortRows, validAthResult} from './ath-optimize';

const d = {key: 'exitpct', label: 'Exit', def: 10, min: 1, max: 50, step: 1};
describe('ath optimization helpers', () => {
  it('builds ranges inside the allowed values', () => {
    expect(rangeValues(8, 12, 2, d)).toEqual([8, 10, 12]);
    expect(rangeValues(-5, 3, 1, d)).toEqual([1, 2, 3]);
    expect(rangeValues(60, 62, 1, d)).toEqual([60, 61, 62]);
    expect(rangeValues(0, 5, 1, {min: 0, max: 1, step: 1})).toEqual([0, 1]);
    expect(rangeValues(0.1, 0.5, 0.1, {min: 0, max: 1})).toEqual([0.1, 0.2, 0.3, 0.4, 0.5]);
    expect(rangeValues(5, 1, 1, d)).toEqual([]);
    expect(rangeValues(1, 2, 0, d)).toEqual([]);
    expect(defaultRange(d)).toEqual({from: 10, to: 10, step: 1});
  });
  it('shows the first 10 settings of the grid, then scrolls', () => {
    const sec = (n: number) => ({params: Array(n).fill(0)});
    expect(optGridHeight([sec(4), sec(6)])).toBe(0);
    expect(optGridHeight([sec(2), sec(12)])).toBe(40 + 36 + 2 * 46 + 36 + 8 * 46 + 2);
    expect(optGridHeight([sec(11)])).toBe(40 + 36 + 10 * 46 + 2);
  });
  it('limits the grid like the engine', () => {
    expect(gridProblem({})).toMatch(/at least one/);
    expect(gridProblem({a: []})).toMatch(/empty/);
    expect(gridProblem({a: Array(21).fill(1)})).toMatch(/more than 20/);
    expect(gridProblem({a: Array(8).fill(1), b: Array(8).fill(1)})).toMatch(/64 combinations/);
    expect(gridProblem({a: Array(6).fill(1), b: Array(10).fill(1)})).toBe('');
    expect(comboCount({a: [1, 2], b: [1, 2, 3]})).toBe(6);
  });
  it('writes the query with fixed settings and grids', () => {
    const q = athOptimizeQuery('ytath_x', {from: '2010-01-01', slippage: 5, params: {pct: 2, exitpct: 9}, grids: {exitpct: [8, 10]}});
    expect(q.get('grid.exitpct')).toBe('8,10');
    expect(q.has('p.exitpct')).toBe(false);
    expect(q.get('p.pct')).toBe('2');
    expect(q.get('from')).toBe('2010-01-01');
  });
  it('ranks rows, missing values last, and filters non-parameters', () => {
    const r = (c: number | null, dd: number | null, s: number | null = 1) => ({p: {}, stats: {cagr: c, max_dd: dd, sharpe: s}});
    const rows = [r(0.1, -0.5), r(null, -0.1), r(0.2, -0.4), r(0.1, -0.1)];
    expect(sortRows(rows, 'cagr').map(x => x.stats.cagr)).toEqual([0.2, 0.1, 0.1, null]);
    expect(sortRows(rows, 'car_mdd')[0]).toBe(rows[3]);
    expect(sortRows(rows, 'car_mdd').at(-1)).toBe(rows[1]);
    expect(athOptParams([d, {...d, key: 'equity'}, {...d, key: 'flat', max: 1, min: 1}]).map(x => x.key)).toEqual(['exitpct']);
    const money = ['sizing', 'alloc', 'pct', 'deadstock'].map(key => ({...d, key}));
    const unused = {...d, key: 'tstart', label: "TRADING START BAR (not used by this AFL's signals)"};
    expect(athOptParams([d, ...money, {...d, key: 'max_positions'}, unused]).map(x => x.key)).toEqual(['exitpct', 'max_positions']);
  });
  it('recognises a usable answer', () => {
    expect(validAthResult({status: 'ready', rows: [{p: {}, stats: {}}]})).toBe(true);
    expect(validAthResult({status: 'ready', rows: []})).toBe(false);
    expect(validAthResult(null)).toBe(false);
  });
});
