import {describe, expect, it} from 'vitest';
import {buildReport, yoyMonthly} from '../src/compat/report-stats.js';

// 8 daily bars, three long trades with returns +10 %, -5 %, +20 %
const dates = ['2020-01-01', '2020-01-02', '2020-01-03', '2020-01-06', '2020-01-07', '2020-01-08', '2020-01-09', '2020-01-10'];
const close = [100, 110, 108, 100, 95, 120, 114, 120];
const trade = (i: number, x: number, entryPx: number, exitPx: number, ret: number) => ({dir: 1, legs: [[i, entryPx, 1]], exit_i: x, avg_px: entryPx, exit_px: exitPx, ret, entry: dates[i], exit: dates[x]});
const trades = [trade(0, 1, 100, 110, 0.1), trade(2, 4, 108, 102.6, -0.05), trade(5, 7, 100, 120, 0.2)];
const equity = [1, 1.1, 1.1, 1.1, 1.045, 1.045, 1.045, 1.254];
const bh = [1, 1.1, 1.08, 1, 0.95, 1.2, 1.14, 1.2];
const bars = {d: dates, h: close.map(c => c + 2), l: close.map(c => c - 3)};

describe('report statistics (unit mode)', () => {
  const r = buildReport({dates, close, equity, bh, trades}, bars);
  it('counts winners and losers with money profit = equity before the trade x return', () => {
    expect(r.all.trades).toBe(3);
    expect(r.all.winners.count).toBe(2); expect(r.all.losers.count).toBe(1);
    expect(r.all.winners.profit).toBeCloseTo(0.1 + 1.045 * 0.2, 9);
    expect(r.all.losers.profit).toBeCloseTo(1.1 * -0.05, 9);
    expect(r.all.profit_factor).toBeCloseTo((0.1 + 0.209) / 0.055, 9);
    expect(r.all.payoff).toBeCloseTo(((0.1 + 0.209) / 2) / 0.055, 9);
    expect(r.all.winners.largest).toBe(0.2); expect(r.all.losers.largest).toBe(-0.05);
    expect(r.all.winners.max_consecutive).toBe(1);
    expect(r.long.trades).toBe(3); expect(r.short.trades).toBe(0); expect(r.short.avg).toBeNull();
  });
  it('bars held count the entry and the exit bar; exposure stops before the exit bar', () => {
    expect(r.trades.map((t: any) => t.held)).toEqual([2, 3, 3]);
    expect(r.trades.map((t: any) => t.bars)).toEqual([1, 2, 2]);
    expect(r.system.exposure).toBeCloseTo((1 + 2 + 2) / 8, 12);
  });
  it('drawdown starts at the first bar, and the net profit is the last equity minus one', () => {
    expect(r.system.net_profit).toBeCloseTo(0.254, 12);
    expect(r.system.max_sys_dd).toBeCloseTo(1.045 / 1.1 - 1, 12);
    expect(r.system.max_sys_dd_abs).toBeCloseTo(-(1.1 - 1.045), 12);
    expect(r.system.car_mdd).toBeCloseTo(r.system.cagr! / Math.abs(r.system.max_sys_dd), 12);
    expect(r.buy_hold!.net_profit).toBeCloseTo(0.2, 12); expect(r.buy_hold!.exposure).toBe(1);
  });
  it('per-trade running drawdown and worst peak-to-valley', () => {
    expect(r.trades[0].accountDd).toBe(0);
    expect(r.trades[1].accountDd).toBeCloseTo(1.045 / 1.1 - 1, 12);
    // trade 2: entry 108, closes 108 and 100 (bars 2 and 3), exit 102.6 -> worst mark 100/108
    expect(r.trades[1].maxTradeDd).toBeCloseTo(100 / 108 - 1, 9);
  });
  it('MAE and MFE come from the daily lows and highs between entry and exit', () => {
    expect(r.trades[0].mae).toBeCloseTo(Math.min(97, 107) / 100 - 1, 12);   // lows 97 and 107
    expect(r.trades[0].mfe).toBeCloseTo(Math.max(102, 112) / 100 - 1, 12);  // highs 102 and 112
  });
});

describe('YOY-Monthly', () => {
  it('compounds from the last bar of each month, sums the year, and averages the months', () => {
    const d = ['2020-01-15', '2020-01-31', '2020-02-28', '2021-01-29'];
    const e = [1, 1.1, 1.21, 1.331];
    const y = yoyMonthly(d, e);
    expect(y.rows.map(r => r.year)).toEqual([2020, 2021]);
    expect(y.rows[0].months[0]).toBeCloseTo(0.1, 12);          // Jan 2020: 1.1 / 1
    expect(y.rows[0].months[1]).toBeCloseTo(0.1, 12);          // Feb 2020: 1.21 / 1.1
    expect(y.rows[0].total).toBeCloseTo(0.2, 12);               // arithmetic sum, not compounded
    expect(y.rows[1].months[0]).toBeCloseTo(0.1, 12);          // Jan 2021: 1.331 / 1.21
    expect(y.totals[0]).toBeCloseTo(0.2, 12);
    expect(y.avgs[0]).toBeCloseTo(0.1, 12);
    expect(y.avgOfTotals).toBeCloseTo((0.2 + 0.1) / 2, 12);
  });
});

import {statisticsBlocks, tradeRows, TRADE_COLUMNS, yoyAoa, reportHTML, fPct} from '../src/compat/report-view.js';
import {parseCell, labSheet, buildLabXlsx} from '../src/compat/lab-xlsx.js';

describe('report views', () => {
  const rep = buildReport({dates, close, equity, bh, trades}, bars);
  it('Statistics blocks hold the text shown on the page, a dash where a column has no value', () => {
    const blocks = statisticsBlocks(rep, 'NIFTY 50');
    expect(blocks[0].title).toBe('Returns'); expect(blocks[0].header).toEqual(['', 'All trades', 'Long trades', 'Short trades', 'Buy&Hold (NIFTY 50)']);
    const net = blocks[0].rows.find(r => r[0] === 'Net Profit %')!;
    expect(net[1]).toBe('25.40%'); expect(net[2]).toBe('—'); expect(net[4]).toBe('20.00%');
    expect(blocks[2].rows[0][0]).toBe('Winners');
  });
  it('Trades: one row per trade plus the exit reason, open trade written as Open', () => {
    const rows = tradeRows(rep, 'NIFTY 50');
    expect(TRADE_COLUMNS.at(-1)).toBe('Exit reason'); expect(rows[0][0]).toBe('NIFTY 50'); expect(rows[0][2]).toBe('01-01-2020'); expect(rows[0][6]).toBe('10.00%'); expect(rows[0].length).toBe(TRADE_COLUMNS.length);
    // money mode: 1,000,000 buys 10,000 shares at 100, the 10 % trade makes 100,000
    expect(rows[0][13]).toBe('100.0%'); expect(rows[0][14]).toBe('10000'); expect(rows[0][15]).toBe('1,000,000'); expect(rows[0][9]).toBe('100,000'); expect(rows[0][16]).toBe('0');
  });
  it('the lab writer types cells from their text and writes its look', () => {
    expect(parseCell('12.50%')).toEqual({s: 7, n: 0.125}); expect(parseCell('1,234.50')).toEqual({s: 6, n: 1234.5}); expect(parseCell('62')).toEqual({s: 5, n: 62});
    expect(parseCell('1,000,000')).toEqual({s: 8, n: 1000000}); expect(parseCell('05-04-2000')).toEqual({s: 4, t: '05-04-2000'}); expect(parseCell('—')).toEqual({s: 4, t: '—'}); expect(parseCell('')).toBeNull();
    const t = labSheet({layout: 'table', header: ['A', 'B'], rows: [['x', '1.5'], ['y', '2']]});
    expect(t).toContain('state="frozen"'); expect(t).toContain('<row r="1" ht="34" customHeight="1">'); expect(t).toContain('<c r="B2" s="6"><v>1.5</v></c>'); expect(t).toContain('<c r="B3" s="10"><v>2</v></c>');
    const b = labSheet({layout: 'blocks', blocks: [{title: 'T', header: ['', 'H'], rows: [['a', '1']]}]});
    expect(b).toContain('showGridLines="0"'); expect(b).toContain('<row r="1" ht="22" customHeight="1"><c r="A1" s="2"'); expect(b).toContain('<row r="4"></row>');
    expect(String.fromCharCode(...buildLabXlsx([{name: 'S', layout: 'table', header: ['A'], rows: []}]).slice(0, 2))).toBe('PK');
  });
  it('YOY rows are in percent units with Total and Avg rows', () => {
    const rows = yoyAoa(yoyMonthly(['2020-01-31', '2020-02-28'], [1.1, 1.21]));
    expect(rows[0][0]).toBe('Year'); expect(rows[1][1]).toBe(10); expect(rows[1][13]).toBe(20);
    expect(rows.at(-2)![0]).toBe('Total'); expect(rows.at(-1)![0]).toBe('Avg');
  });
  it('the page block shows the five cards and no headline tiles (the page header already has them)', () => {
    const html = reportHTML(rep, 'NIFTY 50');
    expect((html.match(/class="metric"/g) || []).length).toBe(0);
    expect((html.match(/class="tbl rep-card"/g) || []).length).toBe(5);
    expect((html.match(/<figure class="rep-chart/g) || []).length).toBe(5);
    expect(html).toContain('data-rep-mode="dist"'); expect(html).toContain('stroke-linecap="round"'); expect(html).toContain('preserveAspectRatio="none"');
    expect(html).toContain('Buy&amp;Hold (NIFTY 50)'); expect(html).toContain(fPct(0.254));
  });
});

describe('report without an index for the basket (F&O)', () => {
  it('leaves the Buy&Hold column empty and keeps the system columns', async () => {
    const {reportBlocks, statisticsBlocks} = await import('../src/compat/report-view.js');
    const r = buildReport({dates, close, equity, bh: null, trades}, bars);
    expect(r.buy_hold).toBeNull(); expect(r.buyhold).toBeNull();
    const blocks = reportBlocks(r);
    for (const [, head, rows] of blocks) for (const row of [...(head ? [head] : []), ...rows]) expect(row[5]).toBeNull();
    expect(blocks[0][2][0][2]).toBe(1000000);
    const sheet = statisticsBlocks(r, 'no index');
    expect(sheet[0].header.at(-1)).toBe('Buy&Hold (no index)');
    expect(sheet.flatMap((b: any) => b.rows.map((x: any) => x.at(-1))).every((v: any) => v === '—')).toBe(true);
  });
});
