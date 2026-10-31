import type {AuthorFile} from '../author-files';

export default {
  order: 960,
  author: { id: "barrie", name: "Scott W. Barrie", tagline: "Market-timing writer · three Dow calendar rules run here", books: [
    { id: "sbcig", sourceIds: ["title-9adc5f14e8e6359a"], title: "The Complete Idiot's Guide to Market Timing", year: 2003, blurb: "Barrie · hold the Dow from an up January to the next down January; hold only trading days 8 to 11 of each month; hold from the end of October two years before the election to the end of April of the election year. The 2.5% band on the 200-day average and the last-day-plus-four rule are the same calculations already in the vault; the S&P/NASDAQ switch is held (needs NASDAQ Composite prices)" }] },
  identities: ["Scott W. Barrie"],
  profile: {
    "kind": "person",
    "mastery": [
      "Market timing",
      "Seasonal and calendar patterns"
    ],
    "summary": "Scott W. Barrie writes on market timing and futures spreads (soybean and grain spread articles are in the research share). The Complete Idiot's Guide to Market Timing (Alpha Books, 2003) surveys timing tools for stock investors: moving averages with filters, percentage swing rules, calendar and presidential-cycle windows and the January Barometer, each measured on the Dow or the S&P 500. This profile rests on the book itself.",
    "known_for": [
      "The Complete Idiot's Guide to Market Timing"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "sbcig": {"type":"book","people":["Scott W. Barrie"]},
  },
} satisfies AuthorFile;
