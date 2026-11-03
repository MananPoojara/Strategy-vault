import type {AuthorFile} from '../author-files';

export default {
  order: 1090,
  author: { id: "skyba", name: "Erik Skyba", tagline: "TradeStation market technician (CMT) · the Swing 5 RSI strategy runs here", books: [
    { id: "essw5", sourceIds: [], readerPaths: ["USB HDD 1/2011/_TradeStation/Erik Skyba - Swing 5 Relative Strength Index/TradeStation Labs.mht"], title: "Swing 5 - Relative Strength Index Strategy (TradeStation Labs)", year: 2010, blurb: "Skyba · buy when the 5-day RSI is under 30 with the close above the 200-day exponential average, or under 25 below it on a long black candle; sell when the RSI crosses above 40. Tested by the author on SPY 2000-2010" }] },
  identities: ["Erik Skyba"],
  profile: {
    "kind": "person",
    "mastery": [
      "Short-term mean-reversion strategies",
      "TradeStation strategy research"
    ],
    "summary": "Erik D. Skyba, CMT, was a senior market technician at TradeStation, writing the TradeStation Labs 'Technical Focus' strategy reports. His Swing 5 - Relative Strength Index Strategy (June 2010) buys 5-day RSI pullbacks with a 200-day exponential average filter and was tested on SPY. This profile rests on that report.",
    "known_for": [
      "Swing 5 - Relative Strength Index Strategy (TradeStation Labs, 2010)"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "essw5": {"type":"article","people":["Erik Skyba"]},
  },
} satisfies AuthorFile;
