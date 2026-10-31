import type {AuthorFile} from '../author-files';

export default {
  order: 890,
  author: { id: "sheimo", name: "Michael D. Sheimo", tagline: "Stockbroker and author (Stock Market Rules) · the day-after-Thanksgiving to year-end Santa Claus window runs here (US only)", books: [
    { id: "mdssc", sourceIds: ["live-file-e0ed60451244fcb0b32f24cf25f265c3", "live-file-add5a1522d31ae044e4f01c4cc40f9c1", "live-file-7d66ee9631ad9fab22c4e34119a7580b", "live-file-d85f9b0830c24046a1c3e0d32bdcc129"], title: "Trading The Santa Claus Rally (Stocks & Commodities, Dec 1992)", year: 1992, blurb: "Sheimo · buy the close of the day after Thanksgiving, sell the close of the year's last trading day" }] },
  identities: ["Michael D. Sheimo"],
  profile: {
    "kind": "person",
    "mastery": [
      "Seasonal stock patterns",
      "Stock selection"
    ],
    "summary": "Michael D. Sheimo, a Minneapolis stockbroker and registered options principal, wrote Stock Market Rules and The Stock Selector System; his 1992 Stocks & Commodities article tested the Thanksgiving-to-year-end rally on five Dow stocks. This profile rests on the article and the book.",
    "known_for": [
      "Stock Market Rules",
      "Trading The Santa Claus Rally"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "mdssc": {"type":"article","people":["Michael D. Sheimo"]},
  },
} satisfies AuthorFile;
