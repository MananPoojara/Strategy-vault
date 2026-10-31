import type {AuthorFile} from '../author-files';

export default {
  order: 540,
  author: { id: 'berg', name: "Jim Berg & Richard Anderson", tagline: "Volatility stock trading and macro timing models", books: [{ id: 'bergsth', title: "The Stock Trading Handbook (Berg)", year: 2005, blurb: "Berg, 2005 · 7/14 moving-average cross (his example lengths); volatility system needs chart reading" }, { id: 'ramtm', title: "Market Timing Models (Anderson)", year: 1997, blurb: "Anderson · regressions on yields, CPI and credit spreads need data" }] },
  identities: ["Jim Berg","Richard Anderson"],
  profile: {
    "kind": "person",
    "mastery": [
      "Trend following",
      "Market timing"
    ],
    "summary": "Jim Berg wrote The Stock Trading Handbook, a guide to rule-based stock trading. Richard Anderson's Market Timing Models covers timing frameworks for allocating between stocks and cash. Little independent biographical information was found, so this profile rests on the book titles.",
    "known_for": [
      "The Stock Trading Handbook (Berg)",
      "Market Timing Models (Anderson)"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "bergsth": {"type":"book","people":["Jim Berg"]},
    "ramtm": {"type":"book","people":["Richard Anderson"]},
  },
} satisfies AuthorFile;
