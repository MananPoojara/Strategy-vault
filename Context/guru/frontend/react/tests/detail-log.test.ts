import { expect, it } from 'vitest';
import { detailLogRows } from '../src/services/detail-log';
import { tsDetailLog } from '../src/features/yt-results/YtResults';

const run = {
  dates: ['2019-01-01', '2019-01-02', '2019-01-03'], equity_money: [1000000, 999950, 1001900], cash: [1000000, 950000, 1001900], positions: [0, 1, 1],
  portfolio: { brokerage_pct: 0.1 },
  trades: [
    { symbol: 'TECHM', dir: 1, shares: 61, entry: '2019-01-02', entry_px: 811.349976, exit: '2019-01-03', exit_px: 850, open: false, reason: 'Sell signal', bars: 1, entry_value: 49492.35, fees: 101.34, pnl: 2256.31, ret: 0.0456 },
    { symbol: 'ACC', dir: 1, shares: 10, entry: '2019-01-03', entry_px: 1500, exit: '2019-01-03', exit_px: 1500, open: true, reason: 'Open', bars: 0, entry_value: 15000, fees: 15, pnl: -15, ret: -0.001, shrunk: true },
  ],
};

it('writes one block per active day: date, signals, trade lines, the day\'s account and a blank row', () => {
  const rows = detailLogRows(run);
  expect(rows[0]).toEqual(['Date', 'Information']);
  expect(rows.slice(1, 7)).toEqual([
    ['2019-01-02', ''],
    ['', 'Entry signals(score):TECHM=Buy(1), '],
    ['', 'Exit signals:'],
    ['', 'Enter Long, TECHM, Price: 811.35, Shares: 61, Position value: 49492.35, Commission: 49.49'],
    ['', '1 Open Positions: TECHM (+61), Market Value: 49950, Equity: 999950, Cash: 950000'],
    ['', ''],
  ]);
  expect(rows[7]).toEqual(['2019-01-03', '']);
  expect(rows[8][1]).toBe('Entry signals(score):ACC=Buy(1), ');
  expect(rows[9][1]).toBe('Exit signals:TECHM=Sell, ');
  // exits come before entries; the exit commission is the trade's fees less the entry commission
  expect(rows[10][1]).toBe('Exit Long, TECHM, Price: 850, Shares: 61, Commission: 51.85, (Total comm.: 101.34), Profit: 2256.31 (4.56 %), Bars: 1, Reason: Sell signal');
  expect(rows[11][1]).toBe('Enter Long, ACC, Price: 1500, Shares: 10, Position value: 15000, Commission: 15, Shrunk to available cash');
  expect(rows[12][1]).toBe('1 Open Positions: ACC (+10), Market Value: 0, Equity: 1001900, Cash: 1001900');
});

it('lists every signal the engine sends, also those the portfolio did not take, and days with signals only', () => {
  // ZEE: a buy on 2019-01-01 the portfolio rejected, sold on 2019-01-02; BHEL: a short signal still open
  const rows = detailLogRows({ ...run, signals: [['TECHM', 1, 1, 2, 1], ['ACC', 1, 2, -1, 1], ['ZEE', 1, 0, 1, 0], ['BHEL', -1, 1, -1, 0]] });
  expect(rows.slice(1, 6)).toEqual([
    ['2019-01-01', ''],
    ['', 'Entry signals(score):ZEE=Buy(1), '],
    ['', 'Exit signals:'],
    ['', '0 Open Positions: Market Value: 0, Equity: 1000000, Cash: 1000000'],
    ['', ''],
  ]);
  expect(rows[7][1]).toBe('Entry signals(score):BHEL=Short(1), TECHM=Buy(1), ');
  expect(rows[8][1]).toBe('Exit signals:ZEE=Sell, ');
  expect(rows.map(r => r[1]).some(t => t.startsWith('Enter Long, ZEE'))).toBe(false);
});

it('lists positions still open at the end instead of writing an exit for them', () => {
  const rows = detailLogRows(run), text = rows.map(r => r[1]);
  expect(text.some(t => t.startsWith('Exit Long, ACC'))).toBe(false);
  const at = text.findIndex(t => t.startsWith('Open at the end of the backtest'));
  expect(rows[at][0]).toBe('2019-01-03');
  expect(text[at + 1]).toBe('Open Long, ACC, Entered: 2019-01-03, Entry price: 1500, Marked at: 1500, Shares: 10, Unrealised profit: -15 (-0.1 %)');
});

it('gives only the header for a run without trades', () => {
  expect(detailLogRows({ dates: ['2019-01-01'], trades: [] })).toEqual([['Date', 'Information']]);
});

it('fills a single-instrument log from the run\'s trades, its whole-unit account and the closing equity', () => {
  const rows = tsDetailLog({
    from: '2019-01-14', to: '2019-01-16', dates: ['2019-01-14', '2019-01-15', '2019-01-16'], equity: [1, 1.00549, 1.00549], bh: [1, 1, 1],
    instrument: { ticker: 'NIFTYBANK', name: 'Bank Nifty' }, money: { capital: 1000000, final_equity: 1005490, costs_total: 0 },
    trades: [{ entry: '2019-01-14', exit: '2019-01-15', entry_px: 27248.25, exit_px: 27400.75, ret: 0.005597, bars: 1, open: false, dir: 1, reason: 'Sell signal' },
      { entry: '2019-01-16', exit: '2019-01-16', entry_px: 27500, exit_px: 27500, ret: 0, bars: 0, open: true, dir: -1, reason: 'Open' }],
    ledger: [{ entry: '2019-01-14', qty: 36, notional: 980937, costs: 0, pnl: 5490 }, { entry: '2019-01-16', qty: 36, notional: 990000, costs: 0, pnl: 0 }],
  });
  expect(rows.slice(0, 5)).toEqual([
    ['Date', 'Information'],
    ['2019-01-14', ''],
    ['', 'Entry signals(score):Bank Nifty=Buy(1), '],
    ['', 'Exit signals:'],
    ['', 'Enter Long, Bank Nifty, Price: 27248.25, Shares: 36, Position value: 980937'],
  ]);
  expect(rows[5]).toEqual(['', '1 Open Positions: Bank Nifty (+36), Equity: 1000000']);
  expect(rows[9][1]).toBe('Exit signals:Bank Nifty=Sell, ');
  expect(rows[10][1]).toBe('Exit Long, Bank Nifty, Price: 27400.75, Shares: 36, Costs: 0, Profit: 5490 (0.56 %), Bars: 1, Reason: Sell signal');
  expect(rows.map(r => r[1])).toContain('Open Short, Bank Nifty, Entered: 2019-01-16, Entry price: 27500, Marked at: 27500, Shares: 36, Unrealised profit: 0 (0 %)');
});

it('shows returns only when a run has no money account, and the candle time of intraday trades', () => {
  const rows = detailLogRows({ dates: ['2020-01-01'], trades: [{ symbol: 'X', dir: 1, entry: '2020-01-01', entry_px: 100, exit: '2020-01-01', exit_px: 101, ret: 0.01, bars: 0, reason: 'Time exit', entry_time: '2020-01-01 09:20', exit_time: '2020-01-01 15:15' }] });
  expect(rows[4][1]).toBe('Exit Long, X, Price: 101, Time: 15:15, Profit: 1 %, Bars: 0, Reason: Time exit');
  expect(rows[5][1]).toBe('Enter Long, X, Price: 100, Time: 09:20');
});
