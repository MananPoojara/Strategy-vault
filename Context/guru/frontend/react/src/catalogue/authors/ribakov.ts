import type {AuthorFile} from '../author-files';

export default {
  order: 1140,
  author: { id: "ribakov", name: "Vladimir Ribakov", tagline: "Forex trader and educator · the double doji breakout runs here", books: [
    { id: "vrsdd", sourceIds: ["title-f8e033f2bd32cc88"], title: "The Secret of the Double Doji Forex Trading Strategy", year: 2009, blurb: "Ribakov · two dojis in a row (a body of at most a tenth of the range here; the book says 2-4 pips), stop orders just beyond their high and low, stop at the other side, target equal to the stop distance (the half-and-double and 80/20 methods as options)" }] },
  identities: ["Vladimir Ribakov"],
  profile: {
    "kind": "person",
    "mastery": [
      "Forex trading",
      "Candlestick patterns"
    ],
    "summary": "Vladimir Ribakov, a forex trader and the creator of the Broker Nightmare and sRs Trend Rider products, wrote The Secret of the Double Doji (2009), a short e-book on trading the breakout of two consecutive doji candles on forex charts with three ways of managing the trade. This profile rests on the e-book itself.",
    "known_for": [
      "The Secret of the Double Doji Forex Trading Strategy"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "vrsdd": {"type":"book","people":["Vladimir Ribakov"]},
  },
} satisfies AuthorFile;
