import type {AuthorFile} from '../author-files';

export default {
  order: 320,
  author: { id: 'weissman', name: 'Richard Weissman', tagline: 'Mechanical trend, mean-reversion and swing systems', books: [{ id: 'rwmts', title: 'Mechanical Trading Systems', year: 2004, blurb: 'Weissman, 2004 · trade counts within 0–3 of the book on the S&P' }, { id: 'rwtlc', title: 'Trade Like a Casino', year: 2011, blurb: 'Weissman, 2011 · 11 mechanical systems' }] },
  identities: ["Richard Weissman"],
  profile: {
    "kind": "person",
    "mastery": [
      "System design & testing",
      "Trading psychology"
    ],
    "summary": "Richard L. Weissman is a trader and trading-system developer, formerly president of R. Lloyd Associates, where he built mechanical systems for many markets. His books cover how to develop and choose mechanical trading systems and how to match them to the trader's psychology.",
    "known_for": [
      "Mechanical Trading Systems (2004)",
      "Trade Like a Casino"
    ],
    "sources": [
      "https://www.wiley.com/en-us/Mechanical+Trading+Systems%3A+Pairing+Trader+Psychology+with+Technical+Analysis-p-9780471730972"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "rwmts": {"type":"book","people":["Richard L. Weissman"]},
    "rwtlc": {"type":"book","people":["Richard L. Weissman"]},
  },
} satisfies AuthorFile;
