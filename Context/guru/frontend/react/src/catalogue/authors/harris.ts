import type {AuthorFile} from '../author-files';

export default {
  order: 90,
  author: { id: 'harris', name: 'Michael Harris', tagline: 'Price-pattern trading · exact OHLC patterns found by pattern search', books: [{ id: 'mhpst', title: 'Profitability and Systematic Trading', year: 2008, blurb: 'Harris, Wiley 2008 · 32 appendix price patterns with % target and stop · 2 FTSE patterns held (same-day trades need intraday data)' }, { id: 'mhstp', title: 'Short-Term Trading with Price Patterns', year: 2000, blurb: 'Harris, Traders Press · most patterns were tested on single stocks the vault does not have' }, { id: 'mhart', title: 'Active Trader articles', year: 2001, blurb: 'Harris, Active Trader 2001 · Keeping It Simple' }] },
  identities: ["Michael Harris"],
  profile: {
    "kind": "person",
    "mastery": [
      "Pattern recognition",
      "Short-term swing",
      "Quantitative research"
    ],
    "summary": "Michael Harris has traded stocks, commodities and currency futures for about three decades and runs Price Action Lab. In the 1990s he developed a theory of identifying and trading price patterns, and he builds software that finds short-term price-action anomalies.",
    "known_for": [
      "Price Action Lab",
      "Short-Term Trading with Price Patterns",
      "Profitability and Systematic Trading"
    ],
    "sources": [
      "https://www.priceactionlab.com/Blog/about/",
      "https://www.priceactionlab.com/Blog/2015/06/trading-systems-price-patterns/"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "mhpst": {"type":"book","people":["Michael Harris"]},
    "mhstp": {"type":"book","people":["Michael Harris"]},
    "mhart": {"type":"article","people":["Michael Harris"]},
  },
} satisfies AuthorFile;
