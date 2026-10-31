import type {AuthorFile} from '../author-files';

export default {
  order: 790,
  author: { id: "schlossberg", name: "Boris Schlossberg", tagline: "Currency strategist (BK Asset Management) · Bollinger band \"bands\" trend trade and the Tao of RSI run here; swing-picked, scale-in and pip setups held", books: [
    { id: "bstacm", sourceIds: ["title-ebd4d46abe3e06cd", "title-ab7f977352944a34"], title: "Technical Analysis of the Currency Market", year: 2006, blurb: "Schlossberg, 2006 · Ch.10 setups: Bollinger band trend and the Tao of RSI (three exits) run on daily bars; range, exhaustion, MACD turn and 3-SMA setups held" }] },
  identities: ["Boris Schlossberg"],
  profile: {
    "kind": "person",
    "mastery": [
      "Currency trading",
      "Technical analysis"
    ],
    "summary": "Boris Schlossberg is a currency strategist and co-founder of BK Asset Management with Kathy Lien. Technical Analysis of the Currency Market applies classic indicators to FX; its Ch.10 prints rule-based setups with Bollinger band bands, RSI and moving averages. This profile rests on the book itself.",
    "known_for": [
      "Technical Analysis of the Currency Market",
      "High Probability Trading Setups for the Currency Market (with Kathy Lien)"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "bstacm": {"type":"book","people":["Boris Schlossberg"]},
  },
} satisfies AuthorFile;
