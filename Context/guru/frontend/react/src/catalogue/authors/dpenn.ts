import type {AuthorFile} from '../author-files';

export default {
  order: 1130,
  author: { id: "dpenn", name: "David Penn", tagline: "Technical analyst and trading writer · the BOSO stochastic strategy runs here", books: [
    { id: "dpstmi", sourceIds: [], readerPaths: ["Trading Knowledge I/David Penn - The Three Secrets to Trading Momentum Indicators/STMI.pdf"], title: "The Three Secrets to Trading Momentum Indicators", year: 2009, blurb: "Penn · BOSO: when the 20-day stochastic breaks above 80 (below 20), buy (sell) on a follow-through close beyond that day's high (low); get out after it slips back out of the zone and a close follows through" }] },
  identities: ["David Penn"],
  profile: {
    "kind": "person",
    "mastery": [
      "Momentum indicators",
      "Technical analysis writing"
    ],
    "summary": "David Penn wrote The Three Secrets to Trading Momentum Indicators (Marketplace Books, 2009), a guide to candlesticks, stochastics, RSI, MACD histogram, moving-average trios and TRIX for momentum traders, including the BOSO way of buying overbought and selling oversold markets. This profile rests on the book itself.",
    "known_for": [
      "The Three Secrets to Trading Momentum Indicators"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "dpstmi": {"type":"book","people":["David Penn"]},
  },
} satisfies AuthorFile;
