import type {AuthorFile} from '../author-files';

export default {
  order: 110,
  author: { id: 'stridsman', name: 'Thomas Stridsman', tagline: 'System evaluation · the Active Trader systems re-tested with full code', books: [{ id: 'strmm', title: 'Trading Systems and Money Management', year: 2003, blurb: 'Stridsman, 2003 · eight Active Trader systems and their variants' }, { id: 'strtw', title: 'Trading Systems That Work', year: 2000, blurb: 'Stridsman, 2000 · systems from the book’s TradeStation code' }] },
  identities: ["Thomas Stridsman"],
  profile: {
    "kind": "person",
    "mastery": [
      "System design & testing",
      "Position sizing & risk"
    ],
    "summary": "Thomas Stridsman is a trading-system designer and financial writer who served as senior staff editor and contributing editor at Active Trader. He wrote two McGraw-Hill books on building and evaluating systems with TradeStation and Excel and on money management, and later traded his approach through a CTA.",
    "known_for": [
      "Trading Systems That Work",
      "Trading Systems and Money Management"
    ],
    "sources": [
      "https://www.trendfollowing.com/pdfs/stridsman.pdf",
      "https://books.google.com/books/about/Tradings_Systems_That_Work_Building_and.html?id=dTmr94EhEoUC"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "strmm": {"type":"book","people":["Thomas Stridsman"]},
    "strtw": {"type":"book","people":["Thomas Stridsman"]},
  },
} satisfies AuthorFile;
