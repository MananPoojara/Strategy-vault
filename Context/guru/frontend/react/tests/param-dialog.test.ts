import {describe, expect, it} from 'vitest';
import {changedParams, clampParam, inactiveParams, paramOptions, paramSections, paramTitle, timeOptions, type ParamDef} from '../src/services/param-dialog';

const P = (key: string, label: string, def: number, min: number, max: number, step: number): ParamDef => ({key, label, def, min, max, step});
const schema = [
  P('src', 'Data source (1 IEOD team database 2007-2023, 2 futures feed 2019-2026)', 1, 1, 2, 1),
  P('days', 'Lookback days', 2, 2, 10, 1),
  P('entry_time', 'Entry time (HHMM, 5-minute grid)', 1450, 1400, 1520, 5),
  P('exit_time', 'Exit time next day (HHMM, 5-minute grid)', 925, 915, 955, 5),
  P('pb_pct', 'Profit booking % (0 = off)', 0, 0, 3, 0.5),
  P('flt_type', 'Trend filter (1 Supertrend, 2 SMA, 3 off)', 3, 1, 3, 1),
  P('st_period', 'Supertrend period', 20, 1, 20, 1),
  P('hdays', 'Highest close of N days', 3, 3, 15, 2),
];

describe('backtest parameter window', () => {
  it('groups parameters into AmiBroker-like sections', () => {
    expect(paramSections(schema).map(s => [s.title, s.params.map(p => p.key)])).toEqual([
      ['Data', ['src']], ['Entry', ['days', 'entry_time']], ['Rule settings', ['hdays']], ['Exit', ['exit_time', 'pb_pct']], ['Trend filter', ['flt_type', 'st_period']],
    ]);
  });
  it('reads coded choices from the label, only when they cover the whole range', () => {
    expect(paramOptions(schema[5])).toEqual([{value: 1, label: 'Supertrend'}, {value: 2, label: 'SMA'}, {value: 3, label: 'off'}]);
    expect(paramOptions(schema[0])!.map(o => o.label)).toEqual(['IEOD team database 2007-2023', 'futures feed 2019-2026']);
    expect(paramOptions(schema[4])).toBeNull();
    expect(paramOptions(schema[2])).toBeNull();
    expect(paramTitle(schema[5])).toBe('Trend filter');
    expect(paramOptions(P('flt_type', 'Trend filter (1 Supertrend, 3 off)', 1, 1, 3, 2))).toEqual([{value: 1, label: 'Supertrend'}, {value: 3, label: 'off'}]);
    expect(paramOptions(P('ibs_dir', 'IBS test (1 IBS <= level, final AFL; 2 IBS >= level, earlier AFL)', 1, 1, 2, 1))!.map(o => o.label)).toEqual(['IBS <= level, final AFL', 'IBS >= level, earlier AFL']);
    // ATH ranking label shared by variants offering modes 1-11 or 1-12: a dropdown with the choices in range
    const rank = 'RANKING MODE (1 NONE; 2 POSITIVE ROC; 3 NEGATIVE ROC; 4 RSI; 5 STD; 6 POSITIVE ROC/STD; 7 NEGATIVE ROC/STD; 8 POSITIVE CLOSE; 9 SWING; 10 POSITIVE ROC + POSITIVE CLOSE; 11 POSITIVE CLOSE + SWING; 12 LAST ATHC where offered)';
    expect(paramOptions(P('rankmode', rank, 1, 1, 11, 1))!.map(o => o.label)).toEqual(['NONE', 'POSITIVE ROC', 'NEGATIVE ROC', 'RSI', 'STD', 'POSITIVE ROC/STD', 'NEGATIVE ROC/STD', 'POSITIVE CLOSE', 'SWING', 'POSITIVE ROC + POSITIVE CLOSE', 'POSITIVE CLOSE + SWING']);
    expect(paramOptions(P('rankmode', rank, 1, 1, 12, 1))![11]).toEqual({value: 12, label: 'LAST ATHC'});
    expect(paramTitle(P('rankmode', rank, 1, 1, 11, 1))).toBe('RANKING MODE');
    expect(paramTitle(schema[4])).toBe('Profit booking % (0 = off)');
    expect(paramTitle(schema[2])).toBe('Entry time');
  });
  it('offers clock times on the real minute grid', () => {
    const t = timeOptions(schema[2])!;
    expect(t[0]).toEqual({value: 1400, label: '14:00'});
    expect(t.map(o => o.value)).toContain(1500);
    expect(t.map(o => o.value)).not.toContain(1460);
    expect(t.at(-1)).toEqual({value: 1520, label: '15:20'});
    expect(timeOptions(schema[1])).toBeNull();
  });
  it('clamps to range and grid and returns only changed values', () => {
    expect(clampParam(schema[4], 1.3)).toBe(1.3);
    expect(clampParam(schema[4], 9)).toBe(9);
    expect(clampParam(schema[4], -2)).toBe(0);
    expect(clampParam(schema[6], 25)).toBe(25);
    expect(clampParam(schema[6], 0)).toBe(1);
    expect(clampParam(P('hardstop', 'Hard Stoploss %', 0, 0, 50, 1), 0)).toBe(0);
    expect(clampParam(P('pennyflt', 'Penny Stock Price', 0, 0, 100, 1), 150)).toBe(150);
    expect(clampParam(schema[5], 7)).toBe(3);
    expect(clampParam(schema[2], 1503)).toBe(1505);
    expect(clampParam(schema[2], 1600)).toBe(1520);
    expect(clampParam(schema[7], 6)).toBe(6);
    expect(clampParam(schema[1], NaN)).toBe(2);
    expect(changedParams(schema, {days: 2, entry_time: 1510, pb_pct: 0.5, flt_type: 3})).toEqual({entry_time: 1510, pb_pct: 0.5});
  });
  it('greys out the settings of trend filters that are not selected', () => {
    expect([...inactiveParams({flt_type: 3})].sort()).toEqual(['flt_tf', 'sma_period', 'st_factor', 'st_period', 'st_point']);
    expect([...inactiveParams({flt_type: 1})]).toEqual(['sma_period']);
    expect([...inactiveParams({flt_type: 2})].sort()).toEqual(['st_factor', 'st_period', 'st_point']);
    expect(inactiveParams({}).size).toBe(0);
  });
  it('greys out the sizing field that the chosen sizing does not use', () => {
    expect([...inactiveParams({sizing: 1})]).toEqual(['alloc']);
    expect([...inactiveParams({sizing: 2})]).toEqual(['pct']);
  });
});
