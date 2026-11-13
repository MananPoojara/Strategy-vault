import type {AuthorFile} from '../author-files';

export default {
  order: 310,
  author: { id: 'katsanos', name: 'Markos Katsanos', tagline: 'Intermarket systems · most need gold, currency or foreign-index data', books: [{ id: 'mkits', title: 'Intermarket Trading Strategies', year: 2008, blurb: 'Katsanos, 2008 · DAX and euro MA systems, relative strength vs the S&P 500' }, { id: 'mkfve', readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V21/C04/076KATS.pdf"], title: 'Detecting Breakouts (S&C article)', year: 2003, blurb: 'Finite Volume Elements divergence, long only' }, { id: "sckvfi", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V22/C07/146KATS.pdf"], title: "Volume Flow Indicator Performance (S&C V.22:7)", year: 2004, blurb: "Katsanos · VFI Test 1 weekly > 0 and Test 2 daily divergence (needs volume)" }, { id: "scmkpsb", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V23/C01/004KATS.pdf", "Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V23/C01/004SB.pdf"], title: "Penny Stock Breakout System (S&C V.23:1)", year: 2005, blurb: "MetaStock buy filters with FVE; 18% one-day drop or 70-bar exit" }] },
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
    "scmkpsb": {"type":"article","people":["Markos Katsanos"]},
    "mkits": {"type":"book","people":["Markos Katsanos"]},
    "mkfve": {"type":"article","people":["Markos Katsanos"]},
    "sckvfi": {"type":"article","people":["Markos Katsanos"]},
  },
} satisfies AuthorFile;
