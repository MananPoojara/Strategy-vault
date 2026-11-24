import type {AuthorFile} from '../author-files';

export default {
  order: 2752,
  author: { id: "krivelyova", name: "Anna Krivelyova & Cesare Robotti", tagline: "Federal Reserve Bank of Atlanta working paper · a geomagnetic-storm seasonal rule as a month calendar", books: [
    { id: "krgms", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Anna Krivelyova, C.Robotti - Playing the Field Geomagnetic Storms and International Stock Markets (Article)/Playing the Field Geomagnetic Storms and International Stock Markets (Article).pdf"], title: "Playing the Field: Geomagnetic Storms and International Stock Markets (Atlanta Fed WP 2003-5)", year: 2003, blurb: "Krivelyova and Robotti, 2003 · short in March-April and September-October, long the other eight months (their Strategy II, tested against buy-and-hold); run on the NSE indices, T-bill leg and costs not modelled, readings declared, unverified" }] },
  identities: ["Anna Krivelyova", "Cesare Robotti"],
  profile: {
    "kind": "person",
    "mastery": [
      "Financial economics"
    ],
    "summary": "Anna Krivelyova and Cesare Robotti wrote a 2003 Federal Reserve Bank of Atlanta working paper on geomagnetic storms and stock returns; its Strategy II is a calendar hedge. This profile rests on that paper alone.",
    "known_for": [
      "Geomagnetic storms and international stock markets"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "krgms": {"type":"article","people":["Anna Krivelyova","Cesare Robotti"]},
  },
} satisfies AuthorFile;
