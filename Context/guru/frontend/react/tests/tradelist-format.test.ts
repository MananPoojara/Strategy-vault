import {readFileSync} from 'node:fs';
import {expect, it} from 'vitest';
import {TRADE_COLS, tradelistCell, tsMoney} from '../src/services/tradelist-format';

it('has the 17 Tradelist columns in the user order', () => {
  expect(TRADE_COLS.join('|')).toBe('Symbol|Signal|Entry Date|Entry Price|Exit Date|Exit Price|Pts|Gross P&L|Gross P&L %|Days Held|Taxes|Slippage|Net P&L|Net P&L %|MAE|MFE|Exit Reason');
});
it('formats the user example row and shows an em dash for anything not supplied', () => {
  const raw = ['NIFTY31JAN2008FUT', 'LONG', '22-01-2008', 4894.6, '2008-01-31', 5137.45, 242.85, 1650000, 0.05, 9, 14250, 99850, 1540000, 0.046, 0.007, 0.106, 'Base2 Bull Exit'];
  expect(raw.map((v, i) => tradelistCell(i, v))).toEqual(['NIFTY31JAN2008FUT', 'Long', '22/01/2008', '4894.60', '31/01/2008', '5137.45', '242.85', '₹16.5 L', '5.0%', '9', '₹14,250', '₹99,850', '₹15.4 L', '4.6%', '0.7%', '10.6%', 'Base2 Bull Exit']);
  expect(Array.from({length: 17}, (_, i) => tradelistCell(i, null)).every(t => t === '—')).toBe(true);
  expect(tsMoney(25000000)).toBe('₹2.5 Cr');
  expect(tsMoney(-1234567, true)).toBe('−$1,234,567');
});
it('the pristine report table and the runtime table both build from the shared formatter', () => {
  const pristine = readFileSync('src/services/pristine-presenter.ts', 'utf8'), runtime = readFileSync('src/compat/runtime.js', 'utf8');
  for (const src of [pristine, runtime]) expect(src).toMatch(/TRADE_COLS[\s\S]*tradelist-format|tradelist-format[\s\S]*TRADE_COLS/);
  expect(pristine).toContain('tradelistCell(j,v)');
  expect(runtime).toContain('tradelistCell(i,');
});
