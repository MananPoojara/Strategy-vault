import type {AuthorFile} from '../author-files';

export default {
  order: 1330,
  author: { id: "etzkorn", name: "Mark Etzkorn", tagline: "Momentum oscillators explained and tested · MACD, RSI and rate-of-change equilibrium systems", books: [
    { id: "metwo", sourceIds: ["title-eaea9328b7643525", "title-959fc0f8fc82690b"], title: "Trading with Oscillators", year: 1997, blurb: "Etzkorn, 1997 · MACD signal-line, RSI 50 and ROC equilibrium stop-and-reverse tests (S&P 500 lengths); stochastic grid and divergences held" },
    { id: "mewwot", sourceIds: ["title-3225f61129ea424c"], title: "What Works in Online Trading (editor)", year: 2001, blurb: "Etzkorn (ed.), 2001 · his spike-reversal day rule (3% spike, out after eight days); the other contributors' articles are counted under their own names" }] },
  identities: ["Mark Etzkorn"],
  profile: {
    "kind": "person",
    "mastery": [
      "Momentum oscillators",
      "Trading-system testing"
    ],
    "summary": "Mark Etzkorn was an editor of Futures and Active Trader magazines. Trading with Oscillators explains how momentum oscillators are built and tests their signals as simple stop-and-reverse systems on nine futures markets; the MACD, RSI and rate-of-change equilibrium tests print exact lengths and are run here. This profile rests on the book itself.",
    "known_for": [
      "Trading with Oscillators: Pinpointing Market Extremes",
      "What Works in Online Trading"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "metwo": {"type":"book","people":["Mark Etzkorn"]},
    "mewwot": {"type":"book","people":["Mark Etzkorn"]},
  },
} satisfies AuthorFile;
