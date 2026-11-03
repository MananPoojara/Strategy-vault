import type {AuthorFile} from '../author-files';

export default {
  order: 300,
  author: { id: 'boucher', name: 'Mark Boucher', tagline: 'Breadth thrusts and timing models · macro filters need data', books: [{ id: 'mbhfe', title: 'The Hedge Fund Edge', year: 1999, blurb: 'Boucher, 1999 · breadth and up-volume thrusts; most macro models need data' }, { id: 'mbsot', title: 'The Science of Trading (workbook)', year: 1995, blurb: 'Boucher · breadth, Appel and McClellan models and short-term setups' }] },
  identities: ["Mark Boucher"],
  profile: {
    "kind": "person",
    "mastery": [
      "Trend following",
      "Market timing",
      "Position sizing & risk"
    ],
    "summary": "Mark Boucher was president of Investment Research Associates, publisher of the Portfolio Strategy Letter and an active futures trader. His book The Hedge Fund Edge sets out global trend-trading strategies that aim for high profit with minimum risk.",
    "known_for": [
      "The Hedge Fund Edge (1998)",
      "Global trend trading strategies"
    ],
    "sources": [
      "https://www.wiley.com/en-us/The+Hedge+Fund+Edge%3A+Maximum+Profit%2FMinimum+Risk+Global+Trend+Trading+Strategies-p-9780471185383",
      "https://books.google.com/books/about/The_Hedge_Fund_Edge.html?id=Ibl7aP1CZ9IC"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "mbhfe": {"type":"book","people":["Mark Boucher"]},
    "mbsot": {"type":"course_manual","people":["Mark Boucher"]},
  },
} satisfies AuthorFile;
