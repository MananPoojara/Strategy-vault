import type {AuthorFile} from '../author-files';

export default {
  order: 2430,
  author: { id: "wgallacher", name: "William Gallacher", tagline: "Winner Take All · PLODDER, the ten-day high/low stop-and-reverse system, on daily bars with two labelled readings (R4 #297)", books: [
    { id: "gwta", sourceIds: [], readerPaths: ["Trading Books/William Gallacher - Winner Take All (Revised Ed.)/William Gallacher - Winner Take All (Revised Ed.).pdf"], title: "Winner Take All (revised edition)", year: null, blurb: "Gallacher's commodity-trading book · ch.4 builds PLODDER (stop and reverse one tick beyond the previous ten days' high or low) and tests it on ten commodity futures over two and a half years in % of margin; the rest is industry commentary, fundamentals, money management and option writing" }] },
  identities: ["William Gallacher", "W. R. Gallacher"],
  sourceReview: {
    "gwta": {"type":"book","people":["William Gallacher"]},
  },
} satisfies AuthorFile;
