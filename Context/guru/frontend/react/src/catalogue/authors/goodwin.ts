import type {AuthorFile} from '../author-files';

export default {
  order: 820,
  author: { id: "goodwin", name: "Andrew Goodwin", tagline: "Hedge-fund S&P trader · eight of his printed TradeStation rules run here; breadth, Treasury, VIX and fixed-point patterns held", books: [
    { id: "agtsic", sourceIds: ["title-8c40d4a149da20e6"], title: "Trading Secrets of the Inner Circle", year: 1997, blurb: "Goodwin, 1997 · basic 8-day RSI, options-expiration range rule, daily patterns #5, #6, #8, #10, #12 and the 6-day reversal, all from the printed S&P futures code" }] },
  identities: ["Andrew Goodwin"],
  profile: {
    "kind": "person",
    "mastery": [
      "S&P 500 futures trading",
      "Short-term pattern systems"
    ],
    "summary": "Andrew Goodwin wrote Trading Secrets of the Inner Circle (1997), a collection of S&P 500 futures rules with their TradeStation code: oscillator systems filtered by breadth and interest rates, an options-expiration rule, daily day-of-week patterns and a short-term reversal system. This profile rests on the book itself.",
    "known_for": [
      "Trading Secrets of the Inner Circle"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "agtsic": {"type":"book","people":["Andrew Goodwin"]},
  },
} satisfies AuthorFile;
