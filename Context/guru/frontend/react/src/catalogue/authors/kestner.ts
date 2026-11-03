import type {AuthorFile} from '../author-files';

export default {
  order: 450,
  author: { id: 'kestner', name: "Lars Kestner", tagline: "Side-by-side tests of classic trend and oscillator systems", books: [{ id: 'lkqts', title: "Quantitative Trading Strategies", year: 2003, blurb: "Kestner, 2003 · Classic trend and oscillator methods. Selected-stock research uses reviewed rules and declared conventions; the book’s original performance remains unverified." }, { id: 'lkcpt', title: "A Comparison of Popular Trading Systems (2nd ed.)", year: 1999, blurb: "Kestner, 1999 · 35 systems; the cash index rarely gaps, so the Gap system trades far less than on futures" }] },
  identities: ["Lars Kestner"],
  profile: {
    "kind": "person",
    "mastery": [
      "System design & testing",
      "Quantitative research",
      "Trend following"
    ],
    "summary": "Lars Kestner was a vice president of equity derivatives trading at Salomon Smith Barney and a founding partner of the proprietary trading firm Sabre II. His work quantifies popular technical trading strategies and compares them across many markets using consistent performance measures.",
    "known_for": [
      "Quantitative Trading Strategies (2003)",
      "A Comparison of Popular Trading Systems",
      "K-ratio"
    ],
    "sources": [
      "https://books.google.com/books?hl=en&id=zZDr-3DRMqcC"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "lkqts": {"type":"book","people":["Lars N. Kestner"]},
    "lkcpt": {"type":"book","people":["Lars N. Kestner"]},
  },
} satisfies AuthorFile;
