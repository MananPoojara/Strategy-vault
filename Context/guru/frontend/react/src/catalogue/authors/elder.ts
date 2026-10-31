import type {AuthorFile} from '../author-files';

export default {
  order: 380,
  author: { id: 'elder', name: 'Alexander Elder', tagline: 'Triple Screen, Impulse System, Force Index', books: [{ id: 'elcim', title: 'Come Into My Trading Room', year: 2002, blurb: 'Elder, 2002 · Impulse System, Triple Screen with Force Index' }, { id: 'eltfl', title: 'Trading for a Living', year: 1993, blurb: 'Elder, 1993 · original Triple Screen with Elder-ray, Force Index, smoothed ROC' }, { id: 'elee', title: "Entries & Exits", year: 2006, blurb: "Elder, 2006 · interviews; one system needs a sentiment index, the rest have judged exits" }, { id: 'elsss', title: "Sell and Sell Short", year: 2008, blurb: "Elder, 2008 · selling methods without complete rules" }, { id: 'elsgee', title: "Study Guide for Entries & Exits", year: 2006, blurb: "Elder, 2006 · questions and answers, no complete rule" }, { id: 'elsgss', title: "Study Guide for Sell & Sell Short", year: 2008, blurb: "Elder, 2008 · questions and answers, no complete rule" }] },
  identities: ["Alexander Elder"],
  profile: {
    "kind": "person",
    "mastery": [
      "Trading psychology",
      "Indicators",
      "Position sizing & risk"
    ],
    "summary": "Dr. Alexander Elder is a psychiatrist turned trader, born in Leningrad, who received asylum in the US. His books organise trading around the 'three Ms' of Mind, Method and Money: psychology, multi-timeframe technical tactics and money management.",
    "known_for": [
      "Trading for a Living (1993)",
      "Triple Screen trading system",
      "Elder-ray and Force Index",
      "Come Into My Trading Room"
    ],
    "sources": [
      "https://www.wiley.com/en-us/Trading+for+a+Living%3A+Psychology%2C+Trading+Tactics%2C+Money+Management-p-9780471592242",
      "https://www.investopedia.com/articles/trading/03/040903.asp"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "elcim": {"type":"book","people":["Alexander Elder"]},
    "eltfl": {"type":"book","people":["Alexander Elder"]},
    "elee": {"type":"book","people":["Alexander Elder"]},
    "elsss": {"type":"book","people":["Alexander Elder"]},
    "elsgee": {"type":"book","people":["Alexander Elder"]},
    "elsgss": {"type":"book","people":["Alexander Elder"]},
  },
} satisfies AuthorFile;
