import { expect, it } from 'vitest';
import { keptPeriod } from '../src/services/stock-selection';

const span = { first: '2001-11-09', last: '2024-10-07' };

it('keeps a chosen To date when another stock is picked', () => {
  expect(keptPeriod(span, '2010-01-01', '2020-12-31')).toEqual({ from: '2010-01-01', to: '2020-12-31' });
  expect(keptPeriod(span, '', '2020-12-31')).toEqual({ from: '', to: '2020-12-31' });
});

it('falls back to the new stock\'s own first or last bar for a date outside its data', () => {
  expect(keptPeriod(span, '2000-01-03', '2020-12-31')).toEqual({ from: '', to: '2020-12-31' });
  expect(keptPeriod(span, '2010-01-01', '2026-10-16')).toEqual({ from: '2010-01-01', to: '' });
});

it('resets a period that leaves no bars in the new stock, and keeps both dates for an unknown span', () => {
  expect(keptPeriod(span, '', '2001-01-01')).toEqual({ from: '', to: '' });
  expect(keptPeriod(span, '2025-01-01', '')).toEqual({ from: '', to: '' });
  expect(keptPeriod(null, '2010-01-01', '2020-12-31')).toEqual({ from: '2010-01-01', to: '2020-12-31' });
});
