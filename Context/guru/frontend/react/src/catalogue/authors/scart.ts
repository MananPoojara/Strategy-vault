import type {AuthorFile} from '../author-files';

export default {
  order: 1450,
  author: { id: 'scart', name: "Stocks & Commodities authors", tagline: "Single-article systems", books: [{ id: 'ddrvi', title: "Refining the RVI (Dorsey)", year: null, blurb: "Dorsey · inertia system; S&P win rate 52% vs the article’s 53%" }, { id: 'aatt', title: "Trading the Trend (Abraham)", year: 1998, blurb: "Abraham, S&C 1998 · ATR trailing trend system" }, { id: 'rhlss', title: "LSS 3-day cycle (Angell)", year: null, blurb: "Level formulas missing; day type is judgement" }, { id: 'cmeod', title: "End of Day Trading System (Mathews)", year: null, blurb: "Judgement terms throughout" }, { id: 'mjzz', title: "Zigzag Signpost (M.S. Johnson)", year: null, blurb: "Exits left to the trader" }] },
  identities: [],
  profile: {
    "kind": "collection",
    "mastery": [
      "Indicators",
      "Short-term swing",
      "Cycles"
    ],
    "summary": "Systems and indicators from Technical Analysis of Stocks & Commodities magazine articles. They include Dorsey's refined Relative Volatility Index, Abraham's trend system, Angell's LSS 3-day cycle, a swing indicator, an end-of-day system and a zigzag signpost.",
    "known_for": [
      "Refining the RVI (Dorsey)",
      "LSS 3-day cycle (Angell)",
      "Trading the Trend (Abraham)"
    ],
    "sources": [],
    "confidence": "medium"
  },
  sourceReview: {
    "ddrvi": {"type":"article","people":["Donald Dorsey"]},
    "aatt": {"type":"article","people":["Andrew Abraham"]},
    "rhlss": {"type":"article","people":["George Angell"]},
    "cmeod": {"type":"book","people":["Chris Mathews"]},
    "mjzz": {"type":"book","people":["Michael S Johnson"]},
  },
} satisfies AuthorFile;
