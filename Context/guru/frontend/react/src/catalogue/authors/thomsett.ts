import type {AuthorFile} from '../author-files';

export default {
  order: 610,
  author: { id: "thomsett", name: "Michael C. Thomsett", tagline: "Options and candlestick author · the three-day swing set-up; option strategies need option prices", books: [
    { id: "mtpos", sourceIds: ["title-668b0250fa7e8253"], title: "Put Option Strategies for Smarter Trading", year: 2009, blurb: "Thomsett, 2009 · three-day swing set-up on the stock (reversal day after a 3-day trend); put strategies need option prices" }] },
  identities: ["Michael C. Thomsett"],
  profile: {
    "kind": "person",
    "mastery": [
      "Options",
      "Candlesticks",
      "Swing trading"
    ],
    "summary": "Michael C. Thomsett writes on options, stock investing and candlestick charting. His books in the library are mainly option strategies (which need option prices) and candlestick patterns without exits; the three-day swing set-up from Put Option Strategies is the one complete stock rule. This profile rests on the books themselves.",
    "known_for": [
      "Options Trading for the Conservative Investor",
      "Trading with Candlesticks",
      "The Options Trading Body of Knowledge"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "mtpos": {"type":"book","people":["Michael C. Thomsett"]},
  },
} satisfies AuthorFile;
