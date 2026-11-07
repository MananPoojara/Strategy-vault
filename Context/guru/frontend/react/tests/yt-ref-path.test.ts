import {describe, expect, it} from 'vitest';
import {ytRefPath} from '../src/services/yt-reference';

describe('ytRefPath', () => {
  it('starts every BTST reference path with BTST/, then the sub-folders and the file', () => {
    expect(ytRefPath('ytbtst_ibs_roc', 'Optim_BTST trad in intraday without flt_Index_NiftyFut-BNF.xlsx')).toBe('BTST/Optim_BTST trad in intraday without flt_Index_NiftyFut-BNF.xlsx');
    expect(ytRefPath('ytbtst_core', 'BTST_Intra day/BTST after 3 consecutive upclose_with profit booking/NIFTY & BANKNIFTY.xlsx'))
      .toBe('BTST/BTST_Intra day/BTST after 3 consecutive upclose_with profit booking/NIFTY & BANKNIFTY.xlsx');
  });
  it('uses the family folder and never doubles it', () => {
    expect(ytRefPath('ytath_x', 'a/b.afl')).toBe('ATH/a/b.afl');
    expect(ytRefPath('ytbtst_core', 'BTST/x.xlsx')).toBe('BTST/x.xlsx');
  });
});
