import type {AuthorFile} from '../author-files';

export default {
  order: 1240,
  author: { id: "forman", name: "John Forman", tagline: "Currency trader and trading educator · the four Ch.7 test systems and the stochastic 40/60 system run here", books: [
    { id: "jfeot", sourceIds: ["title-25c31f3ca7f1f280"], title: "The Essentials of Trading: From the Basics to Building a Winning Strategy", year: 2006, blurb: "Forman · close crosses the 92-day average to enter and the 50-day to exit; Parabolic SAR 0.02/0.20 stop and reverse; 20-day average entry with SAR 0.09/0.50 exit; SAR entry with 60-day average exit; 5-period stochastic through 40/60" }] },
  identities: ["John Forman"],
  profile: {
    "kind": "person",
    "mastery": [
      "Currency trading",
      "Trading system design"
    ],
    "summary": "John Forman is a currency trader and trading educator. The Essentials of Trading (Wiley, 2006) walks through building a mechanical trend system on daily USD/JPY: moving-average and Parabolic SAR entries and exits tested in four combinations, then a GBP/USD fast stochastic system and position sizing. This profile rests on the book itself.",
    "known_for": [
      "The Essentials of Trading"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "jfeot": {"type":"book","people":["John Forman"]},
  },
} satisfies AuthorFile;
