import type {AuthorFile} from '../author-files';

export default {
  order: 1380,
  author: { id: "person", name: "John L. Person", tagline: "Chicago floor trader and educator · pivot points, the three-day pivot average and COMAS crossovers; co-author of the Commodity Trader's Almanac", books: [
    { id: "jpcppt", sourceIds: ["title-9e42d5961530dd6a", "title-a05a280f277befc3", "title-6ad63e85d6185b37"], title: "Candlestick and Pivot Point Trading Triggers", year: 2007, blurb: "Person, 2007 · close or pivot crossing the three-day pivot average (stop and reverse); doji triggers need trend judgement" },
    { id: "jpfxc", sourceIds: ["title-9582d45f1a636fb7", "title-b07e9d78b08ec5c7"], title: "Forex Conquered (book and course)", year: 2007, blurb: "Person, 2007 · same pivot-average rules; 15-minute forex pivot systems with pip stops need forex data" },
    { id: "jpcgt", sourceIds: ["title-73a228b2abb1908b", "title-ab4a421675b5e6db"], title: "A Complete Guide to Technical Trading Tactics", year: 2004, blurb: "Person, 2004 · pivot targets, candles and tactics; no complete rule" }] },
  identities: ["John L. Person"],
  profile: {
    "kind": "person",
    "mastery": [
      "Pivot points",
      "Candlesticks",
      "Futures and forex"
    ],
    "summary": "John L. Person started as a runner on the Chicago Mercantile Exchange floor in 1979, worked with George Lane and teaches pivot-point and candlestick trading. His books combine floor-trader pivot levels with candle patterns; the three-day pivot average and its crossover (COMAS) are stated as mechanical rules. With Jeffrey Hirsch he wrote the Commodity Trader's Almanac. This profile rests on the books themselves.",
    "known_for": [
      "Candlestick and Pivot Point Trading Triggers",
      "Forex Conquered",
      "A Complete Guide to Technical Trading Tactics",
      "Commodity Trader's Almanac"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "jpcppt": {"type":"book","people":["John L. Person"]},
    "jpfxc": {"type":"book","people":["John L. Person"]},
    "jpcgt": {"type":"book","people":["John L. Person"]},
  },
} satisfies AuthorFile;
