import type {AuthorFile} from '../author-files';

export default {
  order: 40,
  author: { id: 'covel', name: 'Michael Covel & the Turtles', tagline: 'Trend following · the original Turtle rules and the systems Covel and Curtis Faith document', books: [{ id: 'covel', title: 'Turtle Rules, Way of the Turtle, Trend Following', year: 2007, blurb: 'Faith / Covel · Turtle System 1 and 2, Way of the Turtle ch.10 systems, Mechanica 89/13, Trend Commandments reversals' }] },
  identities: ["Michael Covel"],
  profile: {
    "kind": "person",
    "mastery": [
      "Trend following",
      "Breakout",
      "Position sizing & risk"
    ],
    "summary": "Michael Covel is an author who co-founded TurtleTrader.com (later TrendFollowing.com) in 1996 to document trend following. The Turtles were traders taught a complete mechanical futures system by Richard Dennis and William Eckhardt; former Turtle Curtis Faith published the original rules and Way of the Turtle.",
    "known_for": [
      "Turtle Trading System",
      "Trend Following",
      "The Complete TurtleTrader",
      "Way of the Turtle (Curtis Faith)"
    ],
    "sources": [
      "https://en.wikipedia.org/wiki/Michael_Covel",
      "https://c.mql5.com/3/131/Curtis_Faith_-_Original_Turtle_Rules.pdf",
      "https://en.wikipedia.org/wiki/Richard_Dennis"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "covel": {"type":"other","people":["Curtis Faith","Michael Covel"]},
  },
} satisfies AuthorFile;
