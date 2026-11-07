import {describe, expect, it} from 'vitest';
import {UNIVERSE_ONLY_IDS, UNIVERSE_TRADE_HEADERS, isUniverseOnly, normaliseRunScope, positionStats, universeAflRows, universeChoiceLabel, universeQuery, universeRunnable, universeSchema, universeTradeRows, universeTradedLine, universeText} from '../src/services/btst-universe';
import {paramOptions} from '../src/services/param-dialog';
import {universeReferenceDefaults, universeTradesheet, btstReference} from '../src/services/btst-reference';
import {AFL_PARAMS} from '../src/services/afl-params';

const info = {universe: 'FO STOCKS C&P', symbols: 494, first: '2007-01-02', last: '2023-02-07', ids: ['ytbtst_core', ...UNIVERSE_ONLY_IDS],
  params: [{key: 'max_positions', label: 'Maximum open positions', def: 10, min: 1, max: 100, step: 1}, {key: 'pct', label: 'Position size', def: 0, min: 0, max: 100, step: 0.5}, {key: 'pad', label: 'Pad', def: 0, min: 0, max: 1, step: 1}]};
const core = [{key: 'src', label: 'Data source', def: 1, min: 1, max: 3, step: 1}, {key: 'days', label: 'Lookback days', def: 2, min: 2, max: 10, step: 1}];

describe('BTST universe scope', () => {
  it('keeps universe in the URL only for BTST strategy pages; universe-only ids always', () => {
    expect(normaliseRunScope('universe', true, 'ytbtst_core')).toBe('universe');
    expect(normaliseRunScope(null, true, 'ytbtst_rank_price')).toBe('universe');
    expect(normaliseRunScope('basket', true, 'ytbtst_nifty_incremental')).toBe('universe');
    expect(normaliseRunScope('universe', true, 'connors_rsi2')).toBe('single');
    expect(normaliseRunScope('universe', false, 'ytbtst_core')).toBe('single');
    expect(normaliseRunScope('basket', true, 'connors_rsi2')).toBe('basket');
    expect(normaliseRunScope('', true, 'ytbtst_core')).toBe('single');
  });
  it('knows the runnable and universe-only ids', () => {
    expect(UNIVERSE_ONLY_IDS).toHaveLength(7);
    expect(isUniverseOnly('ytath_pctswing_exit_turnover_running_buy_2')).toBe(true); // every ATH sub-variant is stock-universe only
    expect(isUniverseOnly('ytbtst_core')).toBe(false);
    expect(isUniverseOnly('ytbtst_rank_roc')).toBe(true);
    expect(isUniverseOnly('ytbtst_core')).toBe(false);
    expect(universeRunnable(info, 'ytbtst_core')).toBe(true);
    expect(universeRunnable(info, 'ytbtst_slc')).toBe(false);
    expect(universeRunnable(null, 'ytbtst_rank_ibs')).toBe(true);
    expect(universeRunnable(null, 'ytath_pctswing_exit_turnover_running_buy_2')).toBe(false); // ATH needs the engine's info
    expect(universeRunnable(null, 'ytbtst_core')).toBe(false);
  });
  it('an ATH universe uses its own wording and scope', () => {
    const ath = {...info, universe: 'NSE 500 C&P', symbols: 1301, ids: ['ytath_pctswing_exit_turnover_running_buy_2'], choice_label: 'NSE 500 stocks · NSE 500 C&P (1301)', traded: 'Traded: NSE 500 C&P stocks on cash prices', badge: 'NSE 500 stocks'};
    expect(universeChoiceLabel(ath)).toBe('NSE 500 stocks · NSE 500 C&P (1301)');
    expect(universeTradedLine(1, ath)).toBe('Traded: NSE 500 C&P stocks on cash prices');
    expect(universeText(ath, 'badge', 'F&O stocks')).toBe('NSE 500 stocks');
    expect(universeText(info, 'badge', 'F&O stocks')).toBe('F&O stocks');
    expect(universeRunnable(ath, 'ytath_pctswing_exit_turnover_running_buy_2')).toBe(true);
    expect(normaliseRunScope(null, true, 'ytath_pctswing_exit_turnover_running_buy_2')).toBe('universe');
    expect(normaliseRunScope('universe', true, 'ytath_other')).toBe('universe');
    expect(normaliseRunScope(null, true, 'ytath_other')).toBe('universe'); // every ATH sub-variant is stock-universe only
    expect(normaliseRunScope('basket', true, 'ytbtst_core')).toBe('basket');
  });
  it('labels the choice and the traded line', () => {
    expect(universeChoiceLabel(info)).toBe('F&O stocks · FO STOCKS C&P (494)');
    expect(universeTradedLine(1, info)).toBe('Traded: F&O stock futures, near-month, FO STOCKS C&P watchlist (494 symbols) from the IEOD_FO database (DATABASE/Intraday_FO/IEOD_FO STOCKS-EODIEOD_UPDATED TILL 07-02-2023)');
    expect(universeTradedLine(3, info)).toMatch(/, as the team's 2021 copy$/);
  });
});

describe('universe parameter schema', () => {
  it('falls back to the core rules plus own keys and portfolio keys', () => {
    const s = universeSchema(info, 'ytbtst_rank_lookback', core);
    expect(s.map(p => p.key)).toEqual(['src', 'days', 'lb_version', 'max_positions', 'pct', 'pad']);
    expect(s[0]).toMatchObject({min: 1, max: 3, step: 2});
    expect(paramOptions(s[0])!.map(o => o.value)).toEqual([1, 3]);
    expect(paramOptions(s.find(p => p.key === 'lb_version')!)!.map(o => o.value)).toEqual([2, 11]);
    expect(paramOptions(s.find(p => p.key === 'pad')!)!.map(o => o.label)).toEqual(['off', 'on']);
    expect(s.find(p => p.key === 'max_positions')!.label).toBe('MAXIMUM POSITION');
  });
  it('prefers the engine schema and relabels choices', () => {
    const s = universeSchema({...info, schemas: {ytbtst_rank_price: [{key: 'src', label: 'x', def: 1, min: 1, max: 3, step: 2}, {key: 'rank_dir', label: 'Ranking (1 high to low, 2 low to high)', def: 1, min: 1, max: 2, step: 1}]}}, 'ytbtst_rank_price', core);
    expect(s.map(p => p.key)).toEqual(['src', 'rank_dir']);
    expect(paramOptions(s[1])!.map(o => o.label)).toEqual(['High to low', 'Low to high']);
  });
  it('maps the core AFL portfolio rows to max_positions / pct on the universe', () => {
    const rows = universeAflRows(AFL_PARAMS.ytbtst_core)!;
    expect(rows.find(r => r.label === 'MAXIMUM POSITION')!.key).toBe('max_positions');
    expect(rows.find(r => r.label === 'POS. SIZE NO OF SHARE / %OF EQUITY')!.key).toBe('pct');
    expect(rows.filter(r => r.why).every(r => !/index run/.test(r.why!))).toBe(true);
    // a stock AFL that already maps them is unchanged
    expect(universeAflRows(AFL_PARAMS.ytbtst_rank_price)!.filter(r => r.key === 'max_positions')).toHaveLength(1);
  });
  it('builds the run query', () => {
    const q = universeQuery('ytbtst_rank_price', {from: '', to: '2020-01-01', slippage: '3', params: {src: 3, rank_dir: 2}});
    expect(q.toString()).toBe('id=ytbtst_rank_price&cost=0&slippage=3&to=2020-01-01&p.src=3&p.rank_dir=2');
  });
});

describe('universe reference and trades', () => {
  it('opens on the stock sheet settings and never on an index sheet', () => {
    expect(universeReferenceDefaults('ytbtst_rank_price')!.params).toEqual({days: 2, entry_time: 1450, exit_time: 915, max_positions: 10, pct: 10, rank_dir: 1});
    expect(universeReferenceDefaults('ytbtst_core')!.sheet).toBeNull();
    expect(universeReferenceDefaults('ytbtst_core')!.cost).toBe(3);
    expect(universeTradesheet(btstReference('ytbtst_rank_price')!, {rank_dir: 2})!.file).toMatch(/LOW to HIGH ranking\.xlsx$/);
    expect(universeTradesheet(btstReference('ytbtst_core')!, {})).toBeNull();
  });
  it('splits entry/exit timestamps and marks open trades at the last bar', () => {
    const rows = universeTradeRows([
      {symbol: 'GAIL', entry: '2007-01-05', entry_ts: '2007-01-05 14:50', exit: '2007-01-08', exit_ts: '2007-01-08 09:15', entry_px: 34.05, exit_px: 34.06, shares: 2936, weight: 0.1, score: 34.05, ret: -0.0002, reason: 'Time exit 0915'},
      {symbol: 'ABB 2007-13', entry: '2023-02-07', entry_time: '14:50', exit: '2023-02-07', exit_time: null, entry_px: 10, exit_px: 10, shares: 1, weight: 0.1, score: 1, ret: 0, open: true},
      {symbol: 'SBIN', entry: '2023-02-07', exit: '2023-02-08', entry_px: 10, exit_px: 9, shares: 1, weight: 0.1, score: 1, ret: 0.1, short: true},
    ]);
    expect(rows[0]).toEqual(['GAIL', 'Long', '2007-01-05', '14:50', 34.05, '2007-01-08', '09:15', 34.06, 2936, 0.1, 34.05, -0.0002, 'Time exit 0915']);
    expect(rows[1].slice(0, 8)).toEqual(['ABB 2007-13', 'Open Long', '2023-02-07', '14:50', 10, '2023-02-07', '', 10]); // marked at the last bar, like AmiBroker's Open Long
    expect(rows[1][12]).toBe('open');
    expect(rows[2][1]).toBe('Short');
    expect(rows.every(r => r.length === UNIVERSE_TRADE_HEADERS.length)).toBe(true);
  });
  it('summarises open positions', () => {
    expect(positionStats([0, 2, 4, 2])).toEqual({avg: 2, max: 4, invested: 0.75});
    expect(positionStats([])).toBeNull();
  });
});

import {PLAIN_LABELS, plainOptions, strategyKeys} from '../src/services/param-dialog';
import {btstFuturesSymbol} from '../src/services/btst-instrument';
describe('Backtest settings window helpers', () => {
  it('gives plain choices on the parameter grid', () => {
    expect(plainOptions({key: 'src', label: '', def: 1, min: 1, max: 3, step: 2})!.map(o => o.label)).toEqual(['IEOD (as stored)', 'IEOD – team\'s 2021 copy']);
    expect(plainOptions({key: 'src', label: '', def: 1, min: 1, max: 3, step: 1})!.map(o => o.value)).toEqual([1, 2, 3]);
    expect(plainOptions({key: 'flt_type', label: '', def: 3, min: 1, max: 3, step: 1})!.map(o => o.label)).toEqual(['Supertrend', 'SMA', 'Off']);
    expect(plainOptions({key: 'lb_version', label: '', def: 2, min: 2, max: 11, step: 9})!.map(o => o.value)).toEqual([2, 11]);
    expect(plainOptions({key: 'days', label: '', def: 2, min: 2, max: 10, step: 1})).toBeNull();
    expect(PLAIN_LABELS.exit_time).toBe('Exit time (next day)');
  });
  it('orders strategy rows by the AFL, then the rest, never src', () => {
    const sch = ['src', 'days', 'entry_time', 'rank_dir', 'max_positions'].map(key => ({key, label: key, def: 0, min: 0, max: 1, step: 1}));
    expect(strategyKeys(sch, ['max_positions', '@cost', undefined, 'entry_time', 'days', 'src'])).toEqual(['max_positions', 'entry_time', 'days', 'rank_dir']);
    expect(strategyKeys(sch)).toEqual(['days', 'entry_time', 'rank_dir', 'max_positions']);
  });
  it('names the BTST futures series of an index button', () => {
    expect(btstFuturesSymbol('NIFTY 50')).toBe('NIFTY_F1');
    expect(btstFuturesSymbol('NIFTYBANK')).toBe('BANKNIFTY_F1');
    expect(btstFuturesSymbol('NIFTYMIDCAP100')).toBeNull();
  });
});

import {tradePage} from '../src/services/btst-universe';
describe('universe trade sheet pages', () => {
  const rows = Array.from({length: 1203}, (_, i) => [i % 3 ? 'ACC' : 'BEL', 'Long', `${2020 + (i % 4)}-01-0${1 + (i % 9)}`, i] as (string | number)[]);
  it('pages through every trade instead of cutting the list', () => {
    const p1 = tradePage(rows), p3 = tradePage(rows, {page: 3});
    expect([p1.total, p1.pages, p1.first, p1.last]).toEqual([1203, 3, 1, 500]);
    expect([p3.first, p3.last, p3.rows.length]).toEqual([1001, 1203, 203]);
    expect(tradePage(rows, {page: 99}).page).toBe(3);
  });
  it('filters by year and symbol and lists the years newest first', () => {
    expect(tradePage(rows).years).toEqual(['2023', '2022', '2021', '2020']);
    const y = tradePage(rows, {year: '2021'});
    expect(y.total).toBe(301); expect(y.rows.every(r => String(r[2]).startsWith('2021'))).toBe(true);
    expect(tradePage(rows, {symbol: 'bel', year: '2021'}).rows.every(r => r[0] === 'BEL')).toBe(true);
    expect(tradePage([], {}).first).toBe(0);
  });
});
