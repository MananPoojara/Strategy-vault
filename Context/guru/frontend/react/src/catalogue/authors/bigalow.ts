import type {AuthorFile} from '../author-files';

export default {
  order: 1390,
  author: { id: "bigalow", name: "Stephen W. Bigalow", tagline: "Candlestick Forum founder · the twelve major candlestick signals, gap patterns and candle entry/exit tactics", books: [
    { id: "swbmcs", sourceIds: ["title-adfe8998c2293aa1"], title: "The Major Candlestick Signals", year: null, blurb: "Bigalow · definitions of the twelve major signals; held: trend and candle size are judged by eye" },
    { id: "swbbpp", sourceIds: ["title-a438888742004342", "title-0b14fff166cc9cb7", "title-360bb3f9856c985d"], title: "Big Profit Patterns Using Candlestick Signals and Gaps", year: null, blurb: "Bigalow · gap set-ups (kicker, fry-pan bottom, J-hook); held: chart reading" },
    { id: "swbtc", sourceIds: ["title-49ae7cab7060c027"], title: "How to Use TC2000 to Find Candlestick Trades", year: null, blurb: "Bigalow · universe scans for oversold stocks; held: manual selection, no exit" },
    { id: "swbcee", sourceIds: ["title-8252eed218e358f6"], title: "Profitable Candlestick Entry and Exit Strategies", year: null, blurb: "Bigalow · entry and exit tactics; held: the author says numeric formulas do not work for candles" }] },
  identities: ["Stephen W. Bigalow"],
  profile: {
    "kind": "person",
    "mastery": [
      "Candlestick charting"
    ],
    "summary": "Stephen W. Bigalow runs the Candlestick Forum and wrote Profitable Candlestick Trading. His e-books in the library define the major Japanese candlestick signals and teach reading them with stochastics; he states that numeric formulas do not work for candle formations, so no complete mechanical rule is given. This profile rests on the books themselves.",
    "known_for": [
      "Profitable Candlestick Trading",
      "The Major Candlestick Signals",
      "Big Profit Patterns Using Candlestick Signals and Gaps"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "swbmcs": {"type":"book","people":["Stephen W. Bigalow"]},
    "swbbpp": {"type":"book","people":["Stephen W. Bigalow"]},
    "swbtc": {"type":"book","people":["Stephen W. Bigalow"]},
    "swbcee": {"type":"book","people":["Stephen W. Bigalow"]},
  },
} satisfies AuthorFile;
