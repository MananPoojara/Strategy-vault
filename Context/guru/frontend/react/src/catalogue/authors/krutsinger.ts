import type {AuthorFile} from '../author-files';

export default {
  order: 240,
  author: { id: 'krutsinger', name: 'Joe Krutsinger', tagline: 'Packaged mechanical systems · Buy Monday, RainMaker and the Secrets of the Masters', books: [{ id: 'jkbwts', title: 'Building Winning Trading Systems (seminar manual)', year: 1997, blurb: 'Krutsinger, 1997 · Dinapoli5, Gettess30, Buy Monday-1400, Saidenberg30, RainMaker' }, { id: 'jk36k', title: '$36,000 Worth of Trading Systems?', year: 1997, blurb: 'Krutsinger, 1997 · JK1 Buy Monday; JK20–31 code is not readable' }, { id: 'jkbk', title: 'Product pack (Boredom Killer, Dealer’s Choice)', year: 2008, blurb: 'Buy Monday 2008 code, SAFE worksheet, JK-Call Buyer' }, { id: 'jkzap', title: 'ZAP seminar slides', year: 2002, blurb: 'Ideas only, no complete rule' }] },
  identities: ["Joe Krutsinger"],
  profile: {
    "kind": "person",
    "mastery": [
      "System design & testing",
      "Futures",
      "Day trading"
    ],
    "summary": "Joe Krutsinger is a CTA and trading-system designer who wrote his first system in 1976. He consulted for traders and firms, turning their ideas into computer-tested mechanical systems, and wrote books on mechanical strategies for index futures, forex, commodities and equities.",
    "known_for": [
      "The Trading Systems Toolkit",
      "Mechanical system packages and seminars"
    ],
    "sources": [
      "https://www.trade2win.com/members/joekrut.16757/about",
      "https://www.amazon.com/Trading-Systems-Toolkit-Money-Making-Futures/dp/1557385343",
      "https://www.vantagepointsoftware.com/news/new-twist-on-an-old-indicator/"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "jkbwts": {"type":"course_manual","people":["Joe Krutsinger"]},
    "jk36k": {"type":"code_package","sameWorkAs":"jkbwts","people":["Joe Krutsinger"]},
    "jkbk": {"type":"code_package","people":["Joe Krutsinger"]},
    "jkzap": {"type":"course_manual","people":["Joe Krutsinger"]},
  },
} satisfies AuthorFile;
