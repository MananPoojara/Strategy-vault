import type {AuthorFile} from '../author-files';

export default {
  order: 1410,
  author: { id: "pesavento", name: "Larry Pesavento", tagline: "Fibonacci pattern trader · Gartley, AB=CD and butterfly ratio patterns, the opening price principle, astro cycles", books: [
    { id: "lppps", sourceIds: ["title-a6a6e5fbd863df33"], title: "Profitable Patterns for Stock Trading", year: 1999, blurb: "Pesavento · Gartley 222, AB=CD, butterfly; held: swing points chosen on the chart" },
    { id: "lpopp", sourceIds: ["title-0414f04cbc84d8db"], title: "The Opening Price Principle (with Peggy MacKay)", year: 2000, blurb: "Pesavento & MacKay, 2000 · the open is near the day's high or low ~70% of the time; held: used with judgement" },
    { id: "lprmc", sourceIds: ["title-bcdb3406527f189c"], title: "The Ruth Miller Method of Trading Corn", year: null, blurb: "Miller & Pesavento · Moon-Uranus aspects and Square of Nine for corn futures; needs data" },
    { id: "lpastro", sourceIds: ["title-a069962e404f8eb3"], title: "Astro Cycles: The Trader's Viewpoint", year: null, blurb: "Pesavento · planetary cycles; no trading rule" }] },
  identities: ["Larry Pesavento"],
  profile: {
    "kind": "person",
    "mastery": [
      "Fibonacci ratios",
      "Pattern recognition"
    ],
    "summary": "Larry Pesavento trades Fibonacci ratio patterns (Gartley 222, AB=CD, butterfly) and wrote on the opening price and on market cycles. The patterns start from swing points chosen on the chart, so no complete mechanical rule is given. This profile rests on the books themselves.",
    "known_for": [
      "Profitable Patterns for Stock Trading",
      "Fibonacci Ratios with Pattern Recognition",
      "The Opening Price Principle"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "lppps": {"type":"book","people":["Larry Pesavento"]},
    "lpopp": {"type":"book","people":["Larry Pesavento"]},
    "lprmc": {"type":"book","people":["Larry Pesavento"]},
    "lpastro": {"type":"book","people":["Larry Pesavento"]},
  },
} satisfies AuthorFile;
