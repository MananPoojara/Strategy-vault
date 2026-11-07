import {describe, expect, it} from 'vitest';
import {BTST_VERIFY, btstVerify} from '../src/compat/btst-verification.js';
import {BTST_VARIANT_IDS, btstReference} from '../src/services/btst-reference';

describe('BTST match badges', () => {
  it('green for every sub-variant with its own trade sheet, yellow for those checked through a sibling, none for the rest', () => {
    for (const id of BTST_VARIANT_IDS) {
      const r = btstReference(id)!, b = BTST_VERIFY[id as keyof typeof BTST_VERIFY];
      if (r.tradesheets.length) expect(b?.[0], id).toBe('own');
      else if (b) expect(b[0], id).toBe('near');
    }
    for (const id of Object.keys(BTST_VERIFY)) expect(BTST_VARIANT_IDS).toContain(id);
    for (const id of ['ytbtst_slc', 'ytbtst_slc_bull', 'ytbtst_slc_bear', 'ytbtst_index_filter']) expect(btstVerify(id, 'NIFTY 50')).toBeNull();
    for (const a of Object.values(BTST_VERIFY)) { expect(a[1]).toBeGreaterThan(0); expect(a[1]).toBeLessThanOrEqual(100); expect(a[2]).toBeTruthy(); }
  });
  it('shows the sheet of the selected index', () => {
    expect(btstVerify('ytbtst_core', 'NIFTY 50')!.slice(0, 2)).toEqual(['own', 100]);
    expect(btstVerify('ytbtst_core', 'NIFTYBANK')!.slice(0, 2)).toEqual(['own', 98.7]);
    expect(btstVerify('ytbtst_candles', undefined)![1]).toBe(70);
    expect(btstVerify('ytbtst_green', 'NIFTYBANK')!.slice(0, 2)).toEqual(['near', 100]);
  });
});
