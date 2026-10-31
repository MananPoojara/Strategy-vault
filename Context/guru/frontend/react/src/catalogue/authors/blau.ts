import type {AuthorFile} from '../author-files';

export default {
  order: 520,
  author: { id: 'blau', name: "William Blau & Tom Bierovic", tagline: "Ergodic momentum systems and pattern setups", books: [{ id: 'wbmdd', title: "Momentum, Direction and Divergence (Blau)", year: 1995, blurb: "Blau, 1995 · Ergodic systems 1–3 with EMA or slow-TSI trend; the other indicators come without complete rules" }, { id: 'tbpk', title: "Playing for Keeps in Stocks & Futures (Bierovic)", year: null, blurb: "Bierovic · every setup ends in an undefined trailing stop, nothing testable" }, { id: 'tbabc', title: "The ABCs of Trading and Technical Analysis (Bierovic)", year: 2000, blurb: "Bierovic, 2000 · Micro-M/W, Six-Shooter, STAMPEDE long and POWERTOOL from the handout’s code" }, { id: 'tbsyn', title: "Synergetic Trading (Bierovic)", year: null, blurb: "Bierovic · setups without complete exits" }] },
  identities: ["Tom Bierovic","William Blau"],
  profile: {
    "kind": "person",
    "mastery": [
      "Indicators",
      "Momentum"
    ],
    "summary": "William Blau developed double-smoothed momentum indicators and first published the True Strength Index in 1991. Tom Bierovic, president of Synergy Futures and later a TradeStation/Omega Research educator, combines trend, directional-movement and momentum indicators with Fibonacci retracements and chart patterns.",
    "known_for": [
      "True Strength Index (Blau)",
      "Momentum, Direction and Divergence (Blau)",
      "Synergetic Trading (Bierovic)"
    ],
    "sources": [
      "https://en.wikipedia.org/wiki/True_strength_index",
      "https://www.wiley.com/en-us/Momentum%2C+Direction%2C+and+Divergence-p-9780471027294",
      "https://store.traders.com/-v16-c01-007int-pdf.html"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "wbmdd": {"type":"book","people":["William Blau"]},
    "tbpk": {"type":"book","people":["Tom Bierovic"]},
    "tbabc": {"type":"course_manual","people":["Tom Bierovic"]},
    "tbsyn": {"type":"book","people":["Tom Bierovic"]},
  },
} satisfies AuthorFile;
