import type {AuthorFile} from '../author-files';

export default {
  order: 2705,
  author: { id: "agdolan", name: "Kenneth Agostino & Brian Dolan", tagline: "GAIN Capital forex research · the DMI trend system with the extreme-point rule runs here on index daily bars", books: [
    { id: "agdmi", sourceIds: [], readerPaths: ["Trading Knowledge III/!PDFs/Make the Trend Your Friend in Forex.pdf"], title: "Make the Trend Your Friend in Forex (Agostino & Dolan, S&C 2004)", year: 2004, blurb: "Agostino and Dolan, 2004 · a DI+/DI- crossover confirmed by Wilder's extreme-point rule, exit at the first lower ADX close once the ADX has been above 25; runs on NIFTY 50 / NIFTY BANK daily bars as a declared adaptation (trendline and parabolic confirmations not coded), no results printed, unverified" }] },
  identities: ["Kenneth Agostino", "Brian Dolan"],
  profile: {
    "kind": "collection",
    "mastery": ["Forex trend following", "Directional movement index (DMI) systems"],
    "summary": "Kenneth Agostino and Brian Dolan of GAIN Capital wrote this Stocks & Commodities forex article on trend identification with trendlines, the DMI and the parabolic indicator. The article was read in full; the DMI entry and ADX exit are coded.",
    "known_for": ["Make the Trend Your Friend in Forex"],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "agdmi": {"type":"article","people":["Kenneth Agostino","Brian Dolan"]},
  },
} satisfies AuthorFile;
