import type {AuthorFile} from '../author-files';

export default {
  order: 2701,
  author: { id: "eminiwatch", name: "Emini-Watch.com", tagline: "Emini-Watch indicator package · the Better TRIN system runs here on NSE and NYSE TRIN as a declared adaptation", books: [
    { id: "ewtrin", sourceIds: [], readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/Emini Watch Indicators Package (emini-watch.com)/Better TRIN 2007/Better TRIN Indicators.pdf"], title: "Better TRIN Indicators (Emini-Watch, 2007)", year: 2007, blurb: "Emini-Watch · a smoothed, normalised oscillator of the log TRIN with five turn entries A-E at the close and a turn exit; runs on NSE TRIN (NIFTY 50, NIFTY BANK) and NYSE TRIN (SPX, NDX, DJI) as a declared adaptation (the NASDAQ TRIN feed is not held), no results printed, unverified" }] },
  identities: ["Emini-Watch.com"],
  profile: {
    "kind": "organisation",
    "mastery": ["E-mini S&P futures indicators", "TRIN breadth oscillators"],
    "summary": "Emini-Watch.com sold TradeStation indicator packages for the S&P e-mini. This profile rests on the Better TRIN document (installation notes and the full EasyLanguage listing); the Better TRIN system is coded, the rest of the package was not read.",
    "known_for": ["Better TRIN indicators"],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "ewtrin": {"type":"code_package","people":["Emini-Watch.com"]},
  },
} satisfies AuthorFile;
