import type {AuthorFile} from '../author-files';

export default {
  order: 330,
  author: { id: 'bandy', name: 'Howard Bandy', tagline: 'Quantitative system design in AmiBroker', books: [{ id: 'hbqts', title: 'Quantitative Trading Systems', year: 2007, blurb: 'Bandy, 2007 · AFL systems from the book’s code' }, { id: 'hbiab', title: 'Introduction to AmiBroker', year: 2008, blurb: 'Bandy, 2008' }] },
  identities: ["Howard Bandy"],
  profile: {
    "kind": "person",
    "mastery": [
      "System design & testing",
      "Quantitative research"
    ],
    "summary": "Dr. Howard B. Bandy writes on quantitative trading-system design through his imprint Blue Owl Press, with examples in AmiBroker. He works only with rules stated in unambiguous mathematical terms and stresses testing and validation methods that show why systems that look good in backtests often lose money in live trading.",
    "known_for": [
      "Quantitative Trading Systems",
      "Introduction to AmiBroker"
    ],
    "sources": [
      "https://books.google.com/books/about/Quantitative_Trading_Systems.html?id=znhBnQEACAAJ",
      "https://www.amazon.com/Quantitative-Trading-Systems-Howard-Bandy/dp/0979183839"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "hbqts": {"type":"book","people":["Howard B. Bandy"]},
    "hbiab": {"type":"book","people":["Howard B. Bandy"]},
  },
} satisfies AuthorFile;
