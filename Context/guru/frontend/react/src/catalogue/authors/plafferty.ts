import type {AuthorFile} from '../author-files';

export default {
  order: 2200,
  author: { id: "plafferty", name: "Patrick Lafferty", tagline: "Single Stock Futures · the daily reversal pattern (6-day extreme with a reversed close, out beyond the previous day's low or high) runs here as a declared application; the outside-day limit entry is held", books: [
    { id: "plssf", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Patrick Lafferty - Single Stock Futures/Lafferty, Patrick - Single Stock Futures.pdf"], title: "Single Stock Futures", year: 2002, blurb: "Lafferty, McGraw-Hill 2002 · single stock futures primer; ch.9 daily reversal patterns traded on CSCO, Intel and the Dow" }] },
  identities: ["Patrick Lafferty"],
  sourceReview: {
    "plssf": {"type":"book","people":["Patrick Lafferty"]},
  },
} satisfies AuthorFile;
