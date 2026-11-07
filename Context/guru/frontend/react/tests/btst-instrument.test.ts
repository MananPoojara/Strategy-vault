import {describe, expect, it} from 'vitest';
import {btstInstrument} from '../src/services/btst-instrument';

describe('BTST instrument', () => {
  it('names the futures series a run trades', () => {
    expect(btstInstrument('ytbtst_core', 'NIFTY 50')!.symbol).toBe('NIFTY_F1');
    expect(btstInstrument('ytbtst_old_fixed', 'NIFTYBANK', 3)!.symbol).toBe('BANKNIFTY_F1');
    expect(btstInstrument('ytbtst_old_fixed', 'NIFTYBANK', 3)!.source).toMatch(/team's 2021 copy/);
    expect(btstInstrument('ytbtst_core', 'NIFTY 50', 2)!.source).toMatch(/not the IEOD/);
  });
  it('is null outside BTST index runs', () => {
    expect(btstInstrument('ytmr_monday_reversal_t1', 'NIFTY 50')).toBeNull();
    expect(btstInstrument('ytbtst_core', 'NIFTYMIDCAP100')).toBeNull();
    expect(btstInstrument('ytbtst_core', undefined)).toBeNull();
  });
});
