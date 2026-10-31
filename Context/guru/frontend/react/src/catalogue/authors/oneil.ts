import type {AuthorFile} from '../author-files';

export default {
  order: 1550,
  author: { id: 'oneil', name: "William O'Neil", tagline: "Founder of Investor's Business Daily · CANSLIM growth-stock method, follow-through days and the 7-8% loss rule", books: [{ id: 'canslim', title: 'How to Make Money in Stocks', year: 1988, blurb: "O'Neil (2nd ed. 1995 read) · base breakouts, CANSLIM screens and market direction; held: chart reading, fundamentals and NYSE volume" }] },
  identities: ["William O'Neil"],
  profile: {
    "kind": "person",
    "mastery": [
      "Momentum",
      "Breakout"
    ],
    "summary": "William O'Neil (1933-2023) was a US stockbroker who founded William O'Neil & Co. and Investor's Business Daily. His CAN SLIM method combines strong earnings growth, leadership and relative strength with buying breakouts from chart bases, along with a rule for gauging the market's direction.",
    "known_for": [
      "CAN SLIM",
      "How to Make Money in Stocks",
      "Investor's Business Daily"
    ],
    "sources": [
      "https://en.wikipedia.org/wiki/William_O%27Neil",
      "https://en.wikipedia.org/wiki/CAN_SLIM"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "canslim": {"type":"book","people":["William J. O'Neil"]},
  },
} satisfies AuthorFile;
