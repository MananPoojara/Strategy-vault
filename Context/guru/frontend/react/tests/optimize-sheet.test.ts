import {describe, expect, it} from 'vitest';
import {AM_COLUMNS, buildOptimizationXlsx, derivedFormulas, derivedValues, optimsFileName, rangeLine, zipStored} from '../src/compat/optimize-sheet.js';
import {labRows, listHTML, surfaceHTML, topTableHTML} from '../src/compat/optimize-view.js';

const am = (over: Record<string, number | null>) => AM_COLUMNS.map(c => (c in over ? over[c] : null));
const A = (over: Record<string, number | null>) => am({'CAR/MDD': 0.1, '# Trades': 12, 'Avg % Profit/Loss': 1.5, 'Avg Bars Held': 6, '% of Winners': 50, '% of Losers': 50, 'W. Avg % Profit': 4, 'W. Avg. Bars Held': 7, 'L. Avg % Loss': -1, 'L. Avg. Bars Held': 5, CAR: 2, ...over});
const text = (b: Uint8Array) => new TextDecoder().decode(b);

describe('optimization sheet', () => {
  it('derived numbers equal the formulas the sheet writes (Long and Short)', () => {
    for (const position of ['Long', 'Short'] as const) {
      const v = derivedValues(position, A({}));
      const at = (n: string) => `<${n}>`;
      const f = derivedFormulas(position, at, 2);
      // evaluate the formula text with the numbers of the row
      const val = (n: string) => (n in v ? v[n] : A({})[AM_COLUMNS.indexOf(n)]);
      const ev = (s: string) => Function(`"use strict";const ABS=Math.abs;return (${s.replace(/<([^>]+)>2/g, (_m, n) => `(${val(n)})`)})`)();
      expect(ev(f[0])).toBeCloseTo(v['Monthly Thickness'], 9); expect(ev(f[1])).toBeCloseTo(v['Expectancy Ratio'], 9);
      expect(ev(f[2])).toBeCloseTo(v['Final Avg % Profit/Loss'], 9); expect(ev(f[6])).toBeCloseTo(v['Final Expectancy Ratio'], 9); expect(ev(f[8])).toBeCloseTo(v['Total Net Profit'], 9);
    }
    // handoff live check: 2.30 - (1.4 * 27.42 * 0.033 + 0.3) = 0.73
    expect(derivedValues('Long', A({'Avg % Profit/Loss': 2.3, 'Avg Bars Held': 28.42, '# Trades': 128}))['Final Avg % Profit/Loss']).toBeCloseTo(0.733, 2);
  });
  it('writes a valid package: two sheets, frozen header, filter, hidden columns, sorted on CAR/MDD', () => {
    const rows = [1, 2, 3].map((n, i) => ({no: n, am: A({'CAR/MDD': [0.1, 0.5, 0.3][i]}), settings: [n]}));
    const bytes = buildOptimizationXlsx({labels: ['Entry ADR Threshold'], rows, rules: {timeframe: 'Daily', index: 'NIFTY 50', period: '01-01-2000 to 06-08-2024', position: 'Long', entry: ['Entry ADR Threshold - 0.1,0.3,0.1'], exit: []}});
    const s = text(bytes);
    expect(s.startsWith('PK')).toBe(true); expect(s).toContain('xl/worksheets/sheet2.xml'); expect(s).toContain('state="frozen"'); expect(s).toContain('<autoFilter ref="A1:AV4"/>'); expect(s).toContain('showGridLines="0"');
    expect((s.match(/hidden="1"/g) || []).length).toBe(28);
    const order = [...s.matchAll(/<c r="A(\d+)" s="\d+"><v>(\d+)<\/v>/g)].map(m => +m[2]);
    expect(order).toEqual([2, 3, 1]);                        // CAR/MDD 0.5, 0.3, 0.1
    expect(s).toContain('<f>X2/(Y2-1)*21</f>');
  });
  it('zip + names', () => {
    expect(text(zipStored([['a.txt', 'hello']])).includes('a.txt')).toBe(true);
    expect(rangeLine('Entry 1: RSI Period', 14, 18, 1)).toBe('Entry 1: RSI Period - 14,18,1');
    expect(optimsFileName('NIFTY 50', 'Long', '')).toBe('NIFTY 50 Long Daily Optims.xlsx');
    expect(optimsFileName('Nifty 50', 'Short', 'Base 2 Bear')).toBe('Nifty 50 Base 2 Bear Short Daily Optims.xlsx');
  });
});

describe('optimizer view', () => {
  const res = {dims: [{key: 'a', values: [1, 2]}, {key: 'b', values: [5, 6]}], fixed: {a: 1, b: 5}, rows: [[1, 5], [1, 6], [2, 5], [2, 6]], sheet: [0.1, 0.4, 0.2, 0.3].map(c => A({CAR: c * 10, 'CAR/MDD': c})), now_sheet: A({CAR: 1})};
  const lab = labRows(res, 'Long');
  it('builds Now and one entry per combination, with the screen columns', () => {
    expect(lab.rows.length).toBe(4); expect(lab.now!.screen.length).toBe(12); expect(lab.rows[1].settings).toEqual([1, 6]);
    const html = topTableHTML(['A', 'B'], lab, [1, 3, 2]);
    expect(html).toContain('>Now<'); expect(html).toContain('>#1<'); expect((html.match(/data-stats=/g) || []).length).toBe(3); expect((html.match(/data-tsrow=/g) || []).length).toBe(3);
  });
  it('list sorts on a clicked column and the surface draws one polygon per cell', () => {
    const asc = listHTML(['A', 'B'], lab, {col: 2, dir: 1}), first = asc.match(/data-tsrow="(\d+)"/)![1];
    expect(first).toBe('0');                                  // CAR 1 is the smallest
    expect(listHTML(['A', 'B'], lab, {col: 2, dir: -1}).match(/data-tsrow="(\d+)"/)![1]).toBe('1');
    expect((surfaceHTML(res as any, ['A', 'B'], lab, 'CAR').match(/<polygon points="[^"]*" style/g) || []).length).toBe(1);
  });
});
