import type {AuthorFile} from '../author-files';

export default {
  order: 830,
  author: { id: "cfwright", name: "Charles F. Wright", tagline: "Trader and trading-system designer · six of his printed TradeStation strategies run here; RangeLeader held (futures-dollar stop and target)", books: [
    { id: "cfwtab", sourceIds: ["title-63bdacda30f9b1cd", "source-384f6c256b2a2aa5", "title-dfebd207e13bbd34"], title: "Trading as a Business", year: 1998, blurb: "Wright · 9/18 MA cross, stochastic cross, gap-open breakout, 50-day channel, MA direction + %R (weekly) and the up-key-reversal breakout, all from the printed code" }] },
  identities: ["Charles F. Wright"],
  profile: {
    "kind": "person",
    "mastery": [
      "Trading-system design",
      "TradeStation strategies"
    ],
    "summary": "Charles F. Wright wrote Trading as a Business (1998), a step-by-step account of designing, testing and trading mechanical strategies with printed TradeStation code for stocks and S&P 500 futures. This profile rests on the book itself.",
    "known_for": [
      "Trading as a Business"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "cfwtab": {"type":"book","people":["Charles F. Wright"]},
  },
} satisfies AuthorFile;
