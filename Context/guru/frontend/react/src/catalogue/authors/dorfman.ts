import type {AuthorFile} from '../author-files';

export default {
  order: 1150,
  author: { id: "dorfman", name: "Robert Dorfman", tagline: "Hedge fund trader (SilverHawk Funds) · the 3-period EMA turn runs here", books: [
    { id: "rdhft", sourceIds: ["title-3ef05b0007493018"], title: "Hedge Fund Trading Secrets Revealed", year: 2008, blurb: "Dorfman · after three lower bars a close back above the 3-period EMA; buy over that bar's high with a stop under its low; half out at the nearer of the 10-day and 20-day averages, stop to break-even, the rest at the next average (short: the mirror)" }] },
  identities: ["Robert Dorfman"],
  profile: {
    "kind": "person",
    "mastery": [
      "Stock and forex trading",
      "Moving averages"
    ],
    "summary": "L. Robert 'The Hawk' Dorfman, founder of SilverHawk Funds, wrote Hedge Fund Trading Secrets Revealed (2008), part memoir of his hedge fund career and part trading manual built around the 3-period EMA, his chart moving averages and candlestick patterns for stocks and currencies. This profile rests on the book itself.",
    "known_for": [
      "Hedge Fund Trading Secrets Revealed"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "rdhft": {"type":"book","people":["Robert Dorfman"]},
  },
} satisfies AuthorFile;
