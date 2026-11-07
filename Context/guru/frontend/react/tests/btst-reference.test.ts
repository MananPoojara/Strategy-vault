import {describe, expect, it} from 'vitest';
import {BEST_SETTING, BTST_BROKERAGE_BPS, BTST_VARIANT_IDS, BestSetting, NO_TRADESHEET, bestActive, btstReference, mainTradesheet, referenceDefaults, referencedPaths, universeReferenceDefaults} from '../src/services/btst-reference';
import {AFL_PARAMS} from '../src/services/afl-params';

describe('BTST reference files', () => {
  it('covers the BTST sub-variants and nothing else', () => {
    expect(BTST_VARIANT_IDS).toContain('ytbtst_core');
    expect(BTST_VARIANT_IDS).toContain('ytbtst_old_fixed');
    expect(BTST_VARIANT_IDS.every(id => /^ytbtst_[a-z_]+$/.test(id))).toBe(true);
    expect(btstReference('ytmr_monday_reversal_t1')).toBeNull();
    expect(btstReference(undefined)).toBeNull();
    expect(btstReference('constructor')).toBeNull();
  });
  it('shows exactly one trade sheet: the one of the selected index, else the main one', () => {
    const core = btstReference('ytbtst_core')!, old = btstReference('ytbtst_old_fixed')!, candles = btstReference('ytbtst_candles')!;
    expect(mainTradesheet(core, 'NIFTY 50')!.sheet).toBe('Bktst_NIFTY_EN 1450_Ex 925_2day');
    expect(mainTradesheet(core, 'NIFTYBANK')!.sheet).toBe('Bktst_BN_EN 1510_Ex 925_2day');
    expect(new Set(core.tradesheets.map(t => t.file)).size).toBe(1);
    expect(mainTradesheet(old, 'NIFTY 50')!.sheet).toBe('NIFTY-TL');
    expect(mainTradesheet(old, 'NIFTYBANK')!.sheet).toBe('BN-TL');
    expect(mainTradesheet(old, 'NIFTYMIDCAP100')!.sheet).toBe('NIFTY-TL');
    expect(mainTradesheet(candles, 'NIFTYBANK')!.file).toMatch(/15 min/);
    expect(mainTradesheet(candles, 'NIFTY 50')!.file).toMatch(/5 minute/);
    expect(mainTradesheet(btstReference('ytbtst_ibs_roc')!, 'NIFTY 50')).toBeNull();
    expect(mainTradesheet(btstReference('ytbtst_rank_price')!, 'NIFTY 50')!.file).toMatch(/HIGH to LOW/);
    expect(NO_TRADESHEET).toBe('No Reference Tradesheet for this Variant');
  });
  it('every reference names an AFL, and every one without a trade list points to a results workbook', () => {
    for (const id of BTST_VARIANT_IDS) {
      const r = btstReference(id)!;
      expect(r.afl).toMatch(/\.afl$/);
      if (!r.tradesheets.length) expect(r.summary?.file).toMatch(/\.xlsx?$/);
    }
    expect(referencedPaths().length).toBeGreaterThan(20);
  });
  it('opens each page with its reference sheet settings and the team brokerage', () => {
    expect(referenceDefaults('ytbtst_core', 'NIFTY 50')).toMatchObject({params: {}, cost: 3});
    expect(referenceDefaults('ytbtst_core', 'NIFTYBANK')!.params).toEqual({entry_time: 1510});
    expect(referenceDefaults('ytbtst_candles', 'NIFTY 50')!.params).toEqual({entry_candle: 3, exit_candle: 3, pt_pct: 0, flt_type: 3});
    expect(referenceDefaults('ytbtst_candles', 'NIFTYBANK')!.params).toMatchObject({tf: 2, pt_pct: 7, flt_tf: 2, st_period: 20, st_factor: 3});
    expect(referenceDefaults('ytbtst_ibs_roc', 'NIFTY 50')).toMatchObject({params: {roc_days: 3, roc_thr: 1.5, entry_time: 1440, exit_time: 915}, cost: 2, sheet: null});
    expect(referenceDefaults('ytbtst_ath', 'NIFTY 50')).toMatchObject({params: {}, cost: BTST_BROKERAGE_BPS, sheet: null}); // its best setting ran on stocks
    expect(referenceDefaults('ytbtst_index_filter', 'NIFTY 50')).toMatchObject({params: {}, cost: BTST_BROKERAGE_BPS, sheet: null});
    expect(referenceDefaults('ytmr_monday_reversal_t1', 'NIFTY 50')).toBeNull();
  });
  it('compares every sub-variant without a trade sheet with ONE highest-return row, except the INDEX filter (not built)', () => {
    const noSheet = BTST_VARIANT_IDS.filter(id => !btstReference(id)!.tradesheets.length);
    const withBest = BTST_VARIANT_IDS.filter(id => btstReference(id)!.best);
    expect(withBest.sort()).toEqual(noSheet.filter(id => id !== 'ytbtst_index_filter').sort());
    expect(withBest).toHaveLength(11);
    expect(btstReference('ytbtst_index_filter')!.best).toBeUndefined();
    expect(BEST_SETTING).toBe("Comparing with the team's highest-return setting");
  });
  it("keeps each best row's numbers consistent with its period: ln(1 + Net) / ln(1 + CAR) = the years run", () => {
    const start = (b: BestSetting) => b.from || (b.ticker === 'NIFTYBANK' ? '2007-05-29' : '2007-01-02'); // first data day
    for (const id of BTST_VARIANT_IDS) {
      const b = btstReference(id)!.best; if (!b) continue;
      const years = (Date.parse(b.to) - Date.parse(start(b))) / (365.25 * 864e5);
      const implied = Math.log(1 + b.team.net / 100) / Math.log(1 + b.team.car / 100);
      expect(Math.abs(implied - years), id).toBeLessThan(0.35);
      expect(b.team.trades, id).toBeGreaterThan(0);
      expect(b.row, id).toMatch(/row/);
      expect(b.file, id).toMatch(/\.xlsx$/);
      if (b.scope === 'index') expect(['NIFTY 50', 'NIFTYBANK'], id).toContain(b.ticker);
      else expect(b.params, id).toMatchObject({max_positions: 10, pct: 10, pb_pct: 0, penny: 10}); // the team's stock runs used Penny Stock Price 10
    }
  });
  it('opens a page without a trade sheet on its best setting, on the instrument it ran on', () => {
    expect(referenceDefaults('ytbtst_slc_bear', 'NIFTYBANK')).toMatchObject({params: {st_period: 20, st_factor: 3, flt_tf: 3}, cost: 4});
    expect(referenceDefaults('ytbtst_slc', 'NIFTYBANK')).toMatchObject({params: {tf: 1}, cost: 4});
    expect(universeReferenceDefaults('ytbtst_athc')).toMatchObject({params: {athc_pct: 20, days: 2, entry_time: 1450, exit_time: 915, max_positions: 10, pct: 10}, cost: 3, sheet: null});
    expect(universeReferenceDefaults('ytbtst_volume')!.params).toMatchObject({vol_days: 3, vol_mult: 1});
    expect(universeReferenceDefaults('ytbtst_ibs_roc')).toMatchObject({params: {}, cost: BTST_BROKERAGE_BPS}); // its best ran on BANKNIFTY_F1
    expect(universeReferenceDefaults('ytbtst_rank_price')!.sheet!.file).toMatch(/HIGH to LOW/); // trade-sheet sub-variants unchanged
  });
  it('knows when a run is exactly the best setting', () => {
    const b = btstReference('ytbtst_ibs_roc')!.best!, run = {universe: false, ticker: 'NIFTYBANK', params: {...b.params, sizing: 1}, cost: 2, from: '', to: '2021-10-28'};
    expect(bestActive(b, run)).toBe(true);
    expect(bestActive(b, {...run, ticker: 'NIFTY 50'})).toBe(false);
    expect(bestActive(b, {...run, cost: 3})).toBe(false);
    expect(bestActive(b, {...run, to: ''})).toBe(false);
    expect(bestActive(b, {...run, params: {...run.params, roc_thr: 2}})).toBe(false);
    expect(bestActive(b, {...run, universe: true})).toBe(false);
    const s = btstReference('ytbtst_green')!.best!;
    expect(bestActive(s, {universe: true, params: {...s.params}, cost: 3, from: '', to: '2021-10-28'})).toBe(true);
    const slc = btstReference('ytbtst_slc')!.best!;
    expect(bestActive(slc, {universe: false, ticker: 'NIFTYBANK', params: {tf: 1}, cost: 4, from: '', to: '2021-09-02'})).toBe(false); // needs the team's start date
  });
  it('lets a setting choose between the workbooks of one sub-variant', () => {
    const r = btstReference('ytbtst_rank_price')!;
    expect(mainTradesheet(r, undefined, {rank_dir: 1})!.file).toMatch(/HIGH to LOW/);
    expect(mainTradesheet(r, undefined, {rank_dir: 2})!.file).toMatch(/LOW to HIGH/);
    expect(mainTradesheet(btstReference('ytbtst_rank_lookback')!, undefined, {})!.file).toMatch(/VER 2/);
    expect(mainTradesheet(btstReference('ytbtst_rank_lookback')!, undefined, {lb_version: 11})!.file).toMatch(/VER 11/);
  });
  it('maps every AFL Param() of a sub-variant to an engine key or says why not', () => {
    for (const [id, rows] of Object.entries(AFL_PARAMS)) {
      expect(BTST_VARIANT_IDS).toContain(id);
      for (const r of rows) expect(Boolean(r.key) !== Boolean(r.why)).toBe(true);
    }
    const core = AFL_PARAMS.ytbtst_core.filter(r => r.key).map(r => r.label);
    expect(core).toEqual(expect.arrayContaining(['ENTRY TIME', 'EXIT TIME', 'Lookback Days', 'Profit Booking pct', 'Hard Stoploss %', 'FILTER TYPE', 'BROKERAGE']));
  });
});
