import type {AuthorFile} from '../author-files';

export default {
  order: 760,
  author: { id: "merriman", name: "Raymond A. Merriman", tagline: "Market-cycles and financial-astrology analyst · cycle time bands and geocosmic correlations (held: no entry/exit rules)", books: [
    { id: "rmubsmt", sourceIds: ["title-4c11231f4f172433", "title-556136c7d0e2b2d4", "title-87deb7d4dd9b7825", "title-bb075e23f46880ff"], title: "The Ultimate Book on Stock Market Timing (Vol.I-IV)", year: 1997, blurb: "Merriman, 1997- · stock-index cycles and geocosmic correlations; held: cycle bands by the analyst, astrology" },
    { id: "rmsmt", sourceIds: ["title-9ab831dbe8fdd3ee"], title: "Stock Market Timing (Geocosmic Correlations to Trading Cycles)", year: null, blurb: "Merriman · held: astrology" },
    { id: "rmsms", sourceIds: ["title-b1c48e72fc0f976f"], title: "The Sun, the Moon and Silver", year: null, blurb: "Merriman · silver cycles; held: futures and astrology" }] },
  identities: ["Raymond A. Merriman"],
  profile: {
    "kind": "person",
    "mastery": [
      "Market cycles",
      "Financial astrology"
    ],
    "summary": "Raymond A. Merriman writes on stock-market cycles and their correlation with planetary and solar-lunar phenomena. His books in the library describe cycle lengths and time bands for turning points and confirm them with technical studies; they give no complete entry and exit rules. This profile rests on the books themselves.",
    "known_for": [
      "The Ultimate Book on Stock Market Timing",
      "Merriman on Market Cycles"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "rmubsmt": {"type":"book","people":["Raymond A. Merriman"]},
    "rmsmt": {"type":"book","people":["Raymond A. Merriman"]},
    "rmsms": {"type":"book","people":["Raymond A. Merriman"]},
  },
} satisfies AuthorFile;
