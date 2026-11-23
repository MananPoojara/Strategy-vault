import type {AuthorFile} from '../author-files';

export default {
  order: 2703,
  author: { id: "forexvirtuoso", name: "Paul Evans (Forex Virtuoso)", tagline: "Forex Soloist EMA/RSI method · runs here on index daily bars as a declared adaptation", books: [
    { id: "fxsolo", sourceIds: [], readerPaths: ["USB HDD 2/Trading Knowledge 3/Forex Virtuoso Trading Method (forexvirtuoso.com)/soloist3.pdf"], title: "Forex Soloist Method (Forex Virtuoso)", year: 2008, blurb: "Paul Evans · 3/5 EMA cross with the 13/21 EMA channel, RSI(21) above/below 50 and the 80 EMA trend filter; exit on the cross back or RSI through 50; runs on NIFTY 50 / NIFTY BANK daily bars (the guide's sixth example is a daily chart) as a declared adaptation, no results printed, unverified" }] },
  identities: ["Paul Evans"],
  profile: {
    "kind": "collection",
    "mastery": ["Forex EMA and RSI systems"],
    "summary": "Forex Virtuoso published short forex system guides. This profile rests on the Forex Soloist guide (read in full); the Forex Virtuoso product it advertises was not read.",
    "known_for": ["Forex Soloist Method"],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "fxsolo": {"type":"course_manual","people":["Paul Evans"]},
  },
} satisfies AuthorFile;
