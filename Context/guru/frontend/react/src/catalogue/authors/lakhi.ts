import type {AuthorFile} from '../author-files';

export default {
  order: 990,
  author: { id: "lakhi", name: "Shiraz A. Lakhi", tagline: "FTSE 100 index trader · the Champion Trader CT-formula runs here", books: [
    { id: "slct", sourceIds: [], title: "Champion Trader", year: 2001, blurb: "Lakhi · with the 14-day average of the daily midpoint rising, a fresh low pivot sets an abort line at its low and a signal line at the next bar's high; buy when the signal line is hit first, stop at the abort line, sell when the average turns down. Falling average: the mirror" }] },
  identities: ["Shiraz A. Lakhi"],
  profile: {
    "kind": "person",
    "mastery": [
      "Index trading",
      "Pivot patterns"
    ],
    "summary": "Shiraz A. Lakhi wrote Champion Trader (2001), a home-study course for trading the FTSE 100 index with index options or spread bets using his CT-formula: a 14-day average of the daily midpoint for the trend, and three-bar pivots that set a signal line and an abort line. This profile rests on the book itself.",
    "known_for": [
      "Champion Trader"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "slct": {"type":"book","people":["Shiraz A. Lakhi"]},
  },
} satisfies AuthorFile;
