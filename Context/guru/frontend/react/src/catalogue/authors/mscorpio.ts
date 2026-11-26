import type {AuthorFile} from '../author-files';

export default {
  order: 2755,
  author: { id: "mscorpio", name: "Massimiliano Scorpio", tagline: "Technical Analysis of Stocks & Commodities · Developing Hot Zones", books: [
    { id: "sczon", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V20/C12/157zone.pdf"], title: "Developing Hot Zones (S&C V.20:12)", year: 2002, blurb: "Scorpio, 2002 · after a key reversal up day, buy and sell stops around an opening zone of 30% of ATR(10), exit at the close; run on the NSE indices, DAX result not compared, readings declared, unverified" }] },
  identities: ["Massimiliano Scorpio"],
  profile: {
    "kind": "person",
    "mastery": ["Trading systems"],
    "summary": "Massimiliano Scorpio wrote Developing Hot Zones for Technical Analysis of Stocks & Commodities in December 2002; this profile rests on that article alone.",
    "known_for": ["Developing Hot Zones"],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "sczon": {"type":"article","people":["Massimiliano Scorpio"]},
  },
} satisfies AuthorFile;
