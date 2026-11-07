import {describe, expect, it} from 'vitest';
import {YT_PRESETS, presetActive, ytMainVariant, ytMainVariants} from '../src/services/yt-family';

const defs = [
  {id: 'ytmr_monday_reversal_t1', family: 'ytmr', group: 'YT Strategies · Monday Reversal', status: 'live'},
  {id: 'ytbtst_core', family: 'ytbtst', group: 'YT Strategies · BTST · Up-close', status: 'live'},
  {id: 'ytbtst_rank_price', family: 'ytbtst', group: 'YT Strategies · BTST · Up-close', status: 'not_testable'},
  {id: 'ytbtst_slc', family: 'ytbtst', group: 'YT Strategies · BTST · Second-last candle', status: 'live'},
  {id: 'ytbtst_ath', family: 'ytbtst', group: 'YT Strategies · BTST · Up-close', status: 'live'},
];

describe('YT family hierarchy', () => {
  it('reads the main variant from the third part of the group', () => {
    expect(ytMainVariant('YT Strategies · BTST · Up-close')).toBe('Up-close');
    expect(ytMainVariant('YT Strategies · Monday Reversal')).toBe('');
    expect(ytMainVariant(undefined)).toBe('');
  });
  it('groups a family into main variants in catalogue order and counts the runnable ones', () => {
    expect(ytMainVariants(defs, 'ytbtst')).toEqual([
      {name: 'Up-close', ids: ['ytbtst_core', 'ytbtst_rank_price', 'ytbtst_ath'], testable: 2},
      {name: 'Second-last candle', ids: ['ytbtst_slc'], testable: 1},
    ]);
    expect(ytMainVariants(defs, 'ytmr')).toEqual([]);
  });
  it('matches a preset only when the overrides are exactly its parameters', () => {
    const [team, bank] = YT_PRESETS.ytbtst_core;
    expect(presetActive({}, team)).toBe(true);
    expect(presetActive({entry_time: 1510}, bank)).toBe(true);
    expect(presetActive({entry_time: 1510, days: 3}, bank)).toBe(false);
    expect(presetActive({entry_time: 1510}, team)).toBe(false);
  });
});

describe('ATH family hierarchy', () => {
  const ath = [
    {id: 'ytath_a', family: 'ytath', group: 'YT Strategies · ATH · ATHC: %Swing exit', status: 'not_testable'},
    {id: 'ytath_b', family: 'ytath', group: 'YT Strategies · ATH · ATHC: Supertrend exit', status: 'not_testable'},
    {id: 'ytath_c', family: 'ytath', group: 'YT Strategies · ATH · ATHC: %Swing exit', status: 'not_testable'},
    {id: 'ytath_d', family: 'ytath', group: 'YT Strategies · ATH · Reshuffling', status: 'not_testable'},
  ];
  it('reads the main variant of an ATH group (names carry no " · " of their own)', () => {
    expect(ytMainVariant('YT Strategies · ATH · ATHC: %Swing exit')).toBe('ATHC: %Swing exit');
    expect(ytMainVariant('YT Strategies · ATH · Reshuffling')).toBe('Reshuffling');
  });
  it('groups ATH into main variants, none runnable yet', () => {
    expect(ytMainVariants([...defs, ...ath], 'ytath')).toEqual([
      {name: 'ATHC: %Swing exit', ids: ['ytath_a', 'ytath_c'], testable: 0},
      {name: 'ATHC: Supertrend exit', ids: ['ytath_b'], testable: 0},
      {name: 'Reshuffling', ids: ['ytath_d'], testable: 0},
    ]);
    expect(ytMainVariants([...defs, ...ath], 'ytbtst').map(m => m.name)).toEqual(['Up-close', 'Second-last candle']);
  });
});
