import type {AuthorFile} from '../author-files';

export default {
  order: 690,
  author: { id: "nison", name: "Steve Nison", tagline: "Brought Japanese candlestick charting to the West · pattern definitions and candle-plus-Western confluence; no mechanical rule (held)", books: [
    { id: "snjcct", sourceIds: ["title-81397d6d53d0f930", "title-1364d0fbc6ba6db0", "title-51b23babb9e56ea1"], title: "Japanese Candlestick Charting Techniques", year: 1991, blurb: "Nison, 1991 · reversal and continuation patterns, candles with Western tools; held: trend and candle size judged by eye, no exits" },
    { id: "sncc", sourceIds: ["title-44f8e5d8ecbf5555"], title: "The Candlestick Course", year: 2003, blurb: "Nison, 2003 · workbook; held: 'Candle charts are best used as a tool, not a system'" },
    { id: "snccb", sourceIds: ["title-3d6f58adfe56a413"], title: "Candlestick Charting Basics (slides)", year: null, blurb: "Nison seminar slides · definitions only" }] },
  identities: ["Steve Nison"],
  profile: {
    "kind": "person",
    "mastery": [
      "Candlestick charting"
    ],
    "summary": "Steve Nison introduced Japanese candlestick charting to Western traders with Japanese Candlestick Charting Techniques (1991). His books in the library define the candle patterns and combine them with Western technical tools; he presents candles as a tool, not a mechanical system, so no complete rule is given. This profile rests on the books themselves.",
    "known_for": [
      "Japanese Candlestick Charting Techniques",
      "Beyond Candlesticks",
      "The Candlestick Course"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "snjcct": {"type":"book","people":["Steve Nison"]},
    "sncc": {"type":"book","people":["Steve Nison"]},
    "snccb": {"type":"book","people":["Steve Nison"]},
  },
} satisfies AuthorFile;
