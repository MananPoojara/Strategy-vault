import type {AuthorFile} from '../author-files';

export default {
  order: 310,
  author: { id: 'katsanos', name: 'Markos Katsanos', tagline: 'Intermarket systems · most need gold, currency or foreign-index data', books: [{ id: 'mkits', title: 'Intermarket Trading Strategies', year: 2008, blurb: 'Katsanos, 2008 · DAX and euro MA systems, relative strength vs the S&P 500' }, { id: 'mkfve', title: 'Detecting Breakouts (S&C article)', year: 2003, blurb: 'Finite Volume Elements divergence, long only' }] },
  identities: ["Markos Katsanos"],
  profile: {
    "kind": "person",
    "mastery": [
      "Indicators",
      "System design & testing",
      "Breakout"
    ],
    "summary": "Markos Katsanos is a structural engineer turned technical analyst and trading-system developer who writes regularly for Technical Analysis of Stocks & Commodities. He created volume-based indicators, including the Volume Flow Indicator (VFI) and Finite Volume Elements (FVE), and wrote Intermarket Trading Strategies.",
    "known_for": [
      "Volume Flow Indicator (VFI)",
      "Finite Volume Elements (FVE)",
      "Intermarket Trading Strategies",
      "High-volume breakout (VPN) indicator"
    ],
    "sources": [
      "https://mkatsanos.com/about/",
      "https://mkatsanos.com/volume-flow-vfi-indicator/",
      "https://mkatsanos.com/articles/"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "mkits": {"type":"book","people":["Markos Katsanos"]},
    "mkfve": {"type":"article","people":["Markos Katsanos"]},
  },
} satisfies AuthorFile;
