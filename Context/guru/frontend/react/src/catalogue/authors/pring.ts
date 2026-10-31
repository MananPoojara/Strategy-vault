import type {AuthorFile} from '../author-files';

export default {
  order: 560,
  author: { id: 'pring', name: 'Martin J. Pring', tagline: 'Technical Analysis Explained author and founder of Pring Research · KST momentum, moving-average and oscillator systems, three-line break', books: [
    { id: 'mpts', sourceIds: ['source-316768836b88af49'], title: 'Trading Systems Explained', year: 2003, blurb: 'Pring, 2003 · MA crossover, MA + CMO, MA + price-oscillator and Triple Indicator systems; RS and intermarket systems need data' },
    { id: 'mpmm', sourceIds: ['source-d8c03d48f39635a9'], title: 'Martin Pring on Market Momentum', year: 1993, blurb: 'Pring, 1993 · Colby & Meyers tests, KST, trend deviation and McClellan Summation crossovers; interpretation chapters not run' },
    { id: 'mptae', sourceIds: ['source-b3c351c69df0ddf0', 'source-32566f3901efaefd'], title: 'Technical Analysis Explained', year: 1985, blurb: 'Pring, 2nd ed. 1985 · 40-week and 10/30-week averages, Russell 13/50, 9-month MA with ROC confirmation, Coppock buy with standard exits' },
    { id: 'mpasi', sourceIds: ['source-3537c04baaf6fe4f'], title: 'The All-Season Investor', year: 1992, blurb: 'Pring, 1992 · S&P 12-month moving-average rule; the stage allocation model needs bond and commodity data' },
    { id: 'mpcse', sourceIds: ['source-b54f79dae71b9d41'], title: 'Candlesticks Explained', year: 2002, blurb: 'Pring, 2002 · three-line break reversal system; renko, kagi and candle patterns need a size or judgement' },
    { id: 'mpsel', sourceIds: ['source-d94dffd14b5493c9'], title: 'How to Select Stocks Using Technical Analysis', year: null, blurb: 'pring.com course CD · long-term KST momentum system and three MetaStock MA systems' }] },
  identities: ["Martin J. Pring"],
  profile: {
    "kind": "person",
    "mastery": [
      "Momentum",
      "Trend following",
      "Market timing"
    ],
    "summary": "Martin J. Pring is a technical analyst, founder of Pring Research and co-founder of Pring Turner Capital. He wrote Technical Analysis Explained and devised the KST (Know Sure Thing) momentum indicator; his books cover momentum, trading systems and business-cycle asset allocation.",
    "known_for": [
      "Technical Analysis Explained",
      "The KST indicator",
      "Martin Pring on Market Momentum"
    ],
    "sources": [
      "https://www.pringturner.com/investment-advisor-team/martin-pring/"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "mpts": {"type":"book","people":["Martin J. Pring"]},
    "mpmm": {"type":"book","people":["Martin J. Pring"]},
    "mptae": {"type":"book","people":["Martin J. Pring"]},
    "mpasi": {"type":"book","people":["Martin J. Pring"]},
    "mpcse": {"type":"book","people":["Martin J. Pring"]},
    "mpsel": {"type":"code_package","people":["Martin J. Pring"]},
  },
} satisfies AuthorFile;
