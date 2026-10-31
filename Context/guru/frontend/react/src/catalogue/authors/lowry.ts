import type {AuthorFile} from '../author-files';

export default {
  order: 1060,
  author: { id: "lowry", name: "Scot Lowry", tagline: "Futures trader · the Delphic Phenomenon moving-average system runs here", books: [
    { id: "slmma", sourceIds: [], readerPaths: ["Trading Knowledge I/Scot Lowry - The Magic Of Moving Averages/The Magic Of Moving Averages (Scot Lowry).pdf"], title: "The Magic of Moving Averages", year: null, blurb: "Lowry · after the 18-day average crosses the 40-day, buy on a stop just above the 18-day after the first dip below it (sell short the mirror); stop just beyond the 40-day, after about a week halfway between the two averages. The 4-day average convergence rules and the dip-timing judgement are not coded" }] },
  identities: ["Scot Lowry"],
  profile: {
    "kind": "person",
    "mastery": [
      "Commodity futures",
      "Moving-average trading"
    ],
    "summary": "Scot Lowry is a futures trader who wrote The Magic of Moving Averages, a short self-published manual (late 1990s, charts from 1997) on what he calls the Delphic Phenomenon: trading the first pullback through the 18-day average after it crosses the 40-day, with later chapters on the convergence of the 4-, 18- and 40-day averages. This profile rests on the book itself.",
    "known_for": [
      "The Magic of Moving Averages"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "slmma": {"type":"book","people":["Scot Lowry"]},
  },
} satisfies AuthorFile;
