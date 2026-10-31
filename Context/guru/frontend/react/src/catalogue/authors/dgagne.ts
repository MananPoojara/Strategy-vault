import type {AuthorFile} from '../author-files';

export default {
  order: 1120,
  author: { id: "dgagne", name: "Dave Gagné", tagline: "Swing trader (Trading Master Plan) · the spring buy and sell setup runs here on US markets", books: [
    { id: "dgtmp", sourceIds: [], readerPaths: ["Trading Knowledge I/Dave Gagné - Trading Master Plan Deluxe Ed. (tradingmasterplan.com)/Deluxe Ed/Trading Master Plan Deluxe Online Edition/Trading Master Plan.pdf"], title: "Trading Master Plan", year: 2005, blurb: "Gagné · after three lower highs, lower lows and red bars in an uptrend, buy a cent over the prior day's high; stop 5 cents under the low, trail 20-day average minus 20-day ATR, sell half at the 20-day average plus 5 ATR" }] },
  identities: ["Dave Gagné"],
  profile: {
    "kind": "person",
    "mastery": [
      "Swing trading",
      "Money management"
    ],
    "summary": "Dave Gagné wrote Trading Master Plan (2005), a course for stock traders on market cycles, candlesticks, moving averages, buy and sell setups and money management, centred on his spring buy and sell setup with a moving-average and ATR trailing stop. This profile rests on the book itself.",
    "known_for": [
      "Trading Master Plan"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "dgtmp": {"type":"book","people":["Dave Gagné"]},
  },
} satisfies AuthorFile;
