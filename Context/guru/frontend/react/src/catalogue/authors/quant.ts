import type {AuthorFile} from '../author-files';

export default {
  order: 430,
  author: { id: 'quant', name: 'Bysshe & Chan', tagline: 'Opening-range and statistical-arbitrage books · need intraday or pair data', books: [{ id: 'gb10', title: 'Trading the 10 O’clock Bulls (Bysshe)', year: 2004, blurb: 'Opening-range rules need intraday bars' }, { id: 'ecqt', title: 'Quantitative Trading (Chan)', year: 2008, blurb: 'Pairs and stat-arb need stock pairs and data the vault does not have' }] },
  identities: [],
  profile: {
    "kind": "person",
    "mastery": [
      "Quantitative research",
      "Day trading",
      "Mean reversion"
    ],
    "summary": "Geoff Bysshe, a former floor trader and hedge-fund manager, co-founded MarketGauge in 1995; his Trading the 10 O'clock Bulls covers opening-range intraday strategies. Dr. Ernest Chan, a Cornell theoretical-physics PhD, founded QTS Capital Management and writes on algorithmic trading, especially statistically tested mean-reversion and momentum strategies.",
    "known_for": [
      "Trading the 10 O'clock Bulls (Bysshe)",
      "Quantitative Trading (Chan)",
      "Algorithmic Trading (Chan)"
    ],
    "sources": [
      "https://www.marketgauge.com/TradingThe10OclockBulls.pdf",
      "https://qtscm.com/"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "gb10": {"type":"book","people":["Geoff Bysshe"]},
    "ecqt": {"type":"book","people":["Ernest P. Chan"]},
  },
} satisfies AuthorFile;
