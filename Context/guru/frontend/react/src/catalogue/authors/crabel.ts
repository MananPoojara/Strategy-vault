import type {AuthorFile} from '../author-files';

export default {
  order: 70,
  author: { id: 'crabel', name: 'Toby Crabel', tagline: 'Opening range breakouts after narrow-range and inside-day patterns', books: [{ id: 'tcdt', title: 'Day Trading with Short Term Price Patterns and Opening Range Breakout', year: 1990, blurb: 'Crabel, 1990 · ID, NR4, NR7, ID/NR4, 2/3-bar NR, Doji and two-day patterns with ORB stretches' }, { id: 'tcsp', title: 'System & Portfolio Development (TAG seminar)', year: 1998, blurb: 'Crabel, 1998 seminar handout · results only, no rules' }] },
  identities: ["Toby Crabel"],
  profile: {
    "kind": "person",
    "mastery": [
      "Breakout",
      "Day trading",
      "Pattern recognition"
    ],
    "summary": "Toby Crabel is an American futures trader, author and former professional tennis player. His 1990 book on short-term price patterns and the opening range breakout became a classic. He later worked with Victor Niederhoffer, then founded a systematic managed-futures firm that grew past $5 billion.",
    "known_for": [
      "Opening range breakout",
      "Day Trading with Short Term Price Patterns and Opening Range Breakout",
      "Range contraction patterns (NR4/NR7)"
    ],
    "sources": [
      "https://en.wikipedia.org/wiki/Toby_Crabel",
      "https://tobycrabel.substack.com/p/the-evolution-of-the-opening-range",
      "https://store.traders.com/tocroprabrpa.html"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "tcdt": {"type":"book","people":["Toby Crabel"]},
    "tcsp": {"type":"course_manual","people":["Toby Crabel"]},
  },
} satisfies AuthorFile;
