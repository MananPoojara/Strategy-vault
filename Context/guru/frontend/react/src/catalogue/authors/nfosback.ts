import type {AuthorFile} from '../author-files';

export default {
  order: 1596,
  author: { id: "nfosback", name: "Norman G. Fosback", tagline: "Market researcher · the complete Seasonality Indicator from Stock Market Logic runs here (US)", books: [
    { id: "nfsml", sourceIds: [], title: "Stock Market Logic", year: 1976, blurb: "Fosback, 1976 · month-end and pre-holiday days combined; US instruments only (US exchange holidays)" }] },
  identities: ["Norman G. Fosback"],
  profile: {
    "kind": "person",
    "mastery": [
      "Market indicators",
      "Stock market seasonality"
    ],
    "summary": "Norman G. Fosback wrote Stock Market Logic (1976), a study of stock market indicators that includes his Seasonality Indicator. This profile rests on the book itself.",
    "known_for": [
      "Stock Market Logic"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "nfsml": {"type":"book","people":["Norman G. Fosback"]},
  },
} satisfies AuthorFile;
