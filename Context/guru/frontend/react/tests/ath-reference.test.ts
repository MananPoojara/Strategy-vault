import {existsSync} from 'node:fs';
import {describe, expect, it} from 'vitest';
import {ATH_VARIANT_IDS, athReference, athReferencedPaths} from '../src/services/ath-reference';
import {AFL_PARAMS_ATH} from '../src/services/afl-params-ath';
import {AFL_PARAMS_ALL} from '../src/services/afl-params-all';
import {ATH_BROKERAGE_BPS, ytFamilyFolder, ytFamilyReference, ytReferenceDefaults} from '../src/services/yt-reference';
import {NO_TRADESHEET, mainTradesheet} from '../src/services/btst-reference';
import {MAIN_VARIANT_NOTES} from '../src/services/yt-family';

/** The team's ATH folder on the share (read-only); the path check runs only where it is mounted. */
const ATH_FOLDER = process.env.ATH_FOLDER || '/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=share/Vrajesh/AFL CODE/ATH';
const ATH_MAINS = ['ATHC: %Swing exit', 'ATHC+% buy: %Swing exit', 'ATHC: Supertrend exit', 'ATHC: Other exits', 'After consecutive losses', 'Bull / Bear market', 'Delay entry',
  'No-January trade', 'Ranking & stock selection', 'Reshuffling', 'Intraday & next-day (BTST)', 'Long-term highs', 'Short & combos', 'Scanners & tools'];

describe('ATH reference files', () => {
  it('covers the ATH sub-variants and nothing else', () => {
    expect(ATH_VARIANT_IDS.length).toBeGreaterThan(200);
    expect(ATH_VARIANT_IDS.every(id => /^ytath_[a-z0-9_]+$/.test(id))).toBe(true);
    expect(new Set(ATH_VARIANT_IDS).size).toBe(ATH_VARIANT_IDS.length);
    expect(athReference('ytbtst_core')).toBeNull();
    expect(athReference(undefined)).toBeNull();
    expect(athReference('constructor')).toBeNull();
  });
  it('every reference names its AFL; the dot is green exactly when a trade sheet exists', () => {
    let green = 0, red = 0;
    for (const id of ATH_VARIANT_IDS) {
      const r = athReference(id)!;
      expect(r.afl).toMatch(/\.afl$/i);
      for (const t of r.tradesheets) { expect(t.file).toMatch(/\.(xlsx|xlsb)$/i); expect(t.sheet.length).toBeGreaterThan(0); }
      if (r.summary) expect(r.summary.file).toMatch(/\.xlsx$/i);
      const sheet = mainTradesheet(r, 'NIFTY 50');
      expect(Boolean(sheet)).toBe(r.tradesheets.length > 0);
      sheet ? green++ : red++;
    }
    expect(green).toBeGreaterThan(0);
    expect(red).toBeGreaterThan(0);
    expect(NO_TRADESHEET).toBe('No Reference Tradesheet for this Variant');
  });
  it.skipIf(!existsSync(ATH_FOLDER))('every referenced AFL and Excel exists in the ATH folder', () => {
    const missing = athReferencedPaths().filter(p => !existsSync(`${ATH_FOLDER}/${p}`));
    expect(missing).toEqual([]);
  }, 120000);
  it('maps every AFL Param() of a sub-variant to an engine key or says why not', () => {
    expect(Object.keys(AFL_PARAMS_ATH).sort()).toEqual([...ATH_VARIANT_IDS].sort());
    for (const rows of Object.values(AFL_PARAMS_ATH)) for (const r of rows) expect(Boolean(r.key) !== Boolean(r.why)).toBe(true);
    const sizing = Object.values(AFL_PARAMS_ATH).flat().filter(r => r.var.toLowerCase() === 'maxpo');
    expect(sizing.length).toBeGreaterThan(100);
    expect(sizing.every(r => r.key === 'max_positions')).toBe(true);
    expect(Object.keys(AFL_PARAMS_ALL)).toContain('ytbtst_core');
    expect(Object.keys(AFL_PARAMS_ALL)).toContain(ATH_VARIANT_IDS[0]);
  });
  it('looks a sub-variant up in its own family', () => {
    const id = ATH_VARIANT_IDS[0];
    expect(ytFamilyReference(id)).toBe(athReference(id));
    expect(ytFamilyReference('ytbtst_core')?.afl).toMatch(/btst/i);
    expect(ytFamilyReference('ytmr_monday_reversal_t1')).toBeNull();
    expect(ytFamilyFolder(id)).toBe('ATH');
    expect(ytFamilyFolder('ytbtst_core')).toBe('BTST');
    expect(ytReferenceDefaults(id, 'NIFTY 50')).toMatchObject({params: {}, cost: ATH_BROKERAGE_BPS});
    expect(ytReferenceDefaults('ytmr_monday_reversal_t1', 'NIFTY 50')).toBeNull();
  });
  it('describes every ATH main variant', () => {
    for (const m of ATH_MAINS) expect(MAIN_VARIANT_NOTES[m]?.length).toBeGreaterThan(30);
  });
});
