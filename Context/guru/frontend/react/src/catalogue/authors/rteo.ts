import type {AuthorFile} from '../author-files';

export default {
  order: 1890,
  author: { id: "rteo", name: "Rayner Teo", tagline: "TradingwithRayner \u00b7 his summary of Andreas Clenow's systematic trend-following rule", books: [
    { id: "rtutf", sourceIds: [], readerPaths: ["Prasad/ALL BOOKS/The-ultimate-guide-to-Trend-Following.pdf"], title: "The Ultimate Guide to Trend Following (e-book)", year: null, blurb: "Teo (Clenow's rule) \u00b7 50 EMA over 100 EMA and a close above the 50-day high, 3-ATR trailing stop from the peak close; short mirror" }] },
  identities: ["Rayner Teo"],
  sourceReview: {
    "rtutf": {"type":"book","people":["Rayner Teo"]},
  },
} satisfies AuthorFile;
