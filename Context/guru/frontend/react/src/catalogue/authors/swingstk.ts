import type {AuthorFile} from '../author-files';

export default {
  order: 1430,
  author: { id: 'swingstk', name: "Palmquist, Burns, Carr, Swing & Smith", tagline: "Stock swing and moving-average signals", books: [{ id: 'palets', title: "Effective Trading Strategies (Palmquist)", year: null, blurb: "Palmquist · seven volume-based stock setups, run on NSE stocks and SPY" }, { id: 'sbma', title: "Moving Averages 101 (Burns)", year: 2015, blurb: "Burns, 2015 · ten moving-average signals" }, { id: 'tkttl', title: "Trend Trading for a Living (Carr)", year: 2008, blurb: "Carr, 2008 · setups need chart judgement and no exit is assigned" }, { id: 'lsgst', title: "A Practical Guide to Swing Trading (Larry Swing)", year: null, blurb: "Swing · gap entries need intraday bars" }, { id: 'zsms', title: "Zach’s Market Score (Smith)", year: null, blurb: "Smith · the buy/cash rule is never stated" }] },
  identities: [],
  profile: {
    "kind": "collection",
    "mastery": [
      "Short-term swing",
      "Trend following",
      "Indicators"
    ],
    "summary": "Practical swing and trend-trading guides for stocks. They cover moving-average methods, trend trading for a living, swing-trading setups and a market-scoring model. All are practitioner-level books focused on entry and exit rules.",
    "known_for": [
      "Trend Trading for a Living (Carr)",
      "Moving Averages 101 (Burns)",
      "A Practical Guide to Swing Trading (Swing)"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "palets": {"type":"book","people":["Steve Palmquist"]},
    "sbma": {"type":"book","people":["Holly Burns","Steve Burns"]},
    "tkttl": {"type":"book","people":["Thomas K. Carr"]},
    "lsgst": {"type":"book","people":["Larry Swing"]},
    "zsms": {"type":"course_manual","people":[]},
  },
} satisfies AuthorFile;
