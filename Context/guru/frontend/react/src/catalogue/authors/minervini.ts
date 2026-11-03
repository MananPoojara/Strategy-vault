import type {AuthorFile} from '../author-files';

export default {
  order: 1540,
  author: { id: 'minervini', name: 'Mark Minervini', tagline: 'US Investing Champion · SEPA growth-stock method: Trend Template, volatility-contraction pivots, tight stops', books: [{ id: 'wizard', title: 'Trade Like a Stock Market Wizard', year: 2013, blurb: 'Minervini, 2013 · Trend Template and VCP pivots; held: the template is a qualifier, the pivot is read by eye, fundamentals and RS rank need data' }, { id: 'champion', title: 'Think & Trade Like a Champion', year: 2017, blurb: 'Minervini, 2017 · cheat entries, post-breakout violations, stops and selling; held: pattern entries read by eye' }] },
  identities: ["Mark Minervini"],
  profile: {
    "kind": "person",
    "mastery": [
      "Momentum",
      "Breakout",
      "Position sizing & risk"
    ],
    "summary": "Mark Minervini is a US stock trader featured in Schwager's Stock Market Wizards. He won the 1997 U.S. Investing Championship with a long-only stock portfolio. His SEPA methodology looks for leading growth stocks in Stage 2 uptrends and buys low-risk entry points, such as volatility-contraction breakouts, with tight stops.",
    "known_for": [
      "SEPA methodology",
      "Trend Template",
      "Trade Like a Stock Market Wizard"
    ],
    "sources": [
      "https://www.minerviniselect.com/about.php"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "wizard": {"type":"book","people":["Mark Minervini"]},
    "champion": {"type":"book","people":["Mark Minervini"]},
  },
} satisfies AuthorFile;
