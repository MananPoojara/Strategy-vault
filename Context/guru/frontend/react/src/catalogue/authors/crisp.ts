import type {AuthorFile} from '../author-files';

export default {
  order: 580,
  author: { id: 'crisp', name: 'Mark Crisp', tagline: 'Stress Free Trading author · weekly two-week breakout for commodities, momentum share systems', books: [
    { id: 'mccic', sourceIds: ['source-cdf91ebc4b08fed7'], title: 'Cashing in with Commodities', year: null, blurb: 'Crisp · weekly two-week breakout with 2.5 x ATR stop, weekly-low trail and 50%/75% profit protection; 1-2-3 entries are judgement' },
    { id: 'mcmts', sourceIds: ['source-6561a237c31c3cbf', 'source-ff3f4d22bab11310'], title: 'Momentum Trading Systems (Daily Momentum and MTS manual)', year: 2006, blurb: 'Crisp · both systems keep one undefined piece (6-week gain base, sector judgement), not run' },
    { id: 'mcstis', sourceIds: ['source-de5b3bf4a9841280'], title: 'Swing Trading Indicator (Crisp)', year: null, blurb: 'Uses repainting zigzag, not testable' }] },
  identities: ["Mark Crisp"],
  sourceReview: {
    "mccic": {"type":"book","people":["Mark Crisp"]},
    "mcmts": {"type":"book","people":["Mark Crisp"]},
    "mcstis": {"type":"code_package","people":["Mark Crisp"]},
  },
} satisfies AuthorFile;
