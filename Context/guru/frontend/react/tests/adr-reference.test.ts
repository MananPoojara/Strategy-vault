import {existsSync} from 'node:fs';
import {describe, expect, it} from 'vitest';
import {ADR_REFERENCES, adrReference} from '../src/services/adr-reference';
import {ADR_BROKERAGE_BPS, ytFamilyFolder, ytFamilyReference, ytReferenceDefaults} from '../src/services/yt-reference';
import {MAIN_VARIANT_NOTES, YT_PRESETS} from '../src/services/yt-family';

/** The read-only local copy of the team's ADR folder; the path check runs only where it exists. */
const ADR_FOLDER = process.env.ADR_FOLDER || `${process.env.HOME}/strategy-vault-data/inbox/adr`;
const ADR_MAINS = ['Basic Long/Short', 'Versions (ST and RSI filters)', 'Without RSI', '2-11 / 5DP patch', 'Five Day Phenomena'];

describe('ADR reference files', () => {
  it('covers the 9 ADR sub-variants and nothing else', () => {
    const ids = Object.keys(ADR_REFERENCES);
    expect(ids).toHaveLength(9);
    expect(ids.every(id => /^ytadr_[a-z0-9_]+$/.test(id))).toBe(true);
    expect(adrReference('ytath_x')).toBeNull();
    expect(adrReference('constructor')).toBeNull();
    expect(ytFamilyReference('ytadr_versions')).toBe(ADR_REFERENCES.ytadr_versions);
    expect(ytFamilyFolder('ytadr_5dp')).toBe('Advance Decline Ratio');
  });
  it('every reference names its AFL; trade sheets carry the settings found for them', () => {
    for (const [id, r] of Object.entries(ADR_REFERENCES)) {
      expect(r.afl.length, id).toBeGreaterThan(0);
      for (const t of r.tradesheets) { expect(t.file).toMatch(/\.xlsx$/i); expect(Object.keys(t.params || {}).length, `${id} ${t.sheet}`).toBeGreaterThan(0); }
      if (!r.tradesheets.length && id !== 'ytadr_versions_5dp_exit' && id !== 'ytadr_long_2_11_5dp') expect(r.summary?.file, id).toMatch(/\.xlsx$/i);
    }
  });
  it('a page opens on the first trade sheet settings with the 0.04% brokerage', () => {
    const d = ytReferenceDefaults('ytadr_versions', 'NIFTY 50')!;
    expect(d.params).toMatchObject({st_on: 1, st_tf: 1, st_fact: 2.5, thr: 0.2, exit_n: 16});
    expect(d.cost).toBe(ADR_BROKERAGE_BPS);
    expect(ytReferenceDefaults('ytadr_longshort', 'NIFTY 50')!.params).toEqual({});
  });
  it('every main variant has a note and presets name real sheets\' settings', () => {
    for (const m of ADR_MAINS) expect(MAIN_VARIANT_NOTES[m], m).toBeTruthy();
    for (const id of Object.keys(YT_PRESETS).filter(i => i.startsWith('ytadr_'))) {
      const known = ADR_REFERENCES[id].tradesheets.map(t => JSON.stringify(t.params));
      for (const p of YT_PRESETS[id]) expect(known, `${id} ${p.label}`).toContain(JSON.stringify(p.params));
    }
  });
  it.runIf(existsSync(ADR_FOLDER))('every referenced file exists in the ADR folder', () => {
    for (const r of Object.values(ADR_REFERENCES)) {
      for (const f of [...r.tradesheets, ...(r.summary ? [r.summary] : [])]) expect(existsSync(`${ADR_FOLDER}/${f.file}`), f.file).toBe(true);
    }
  });
});
