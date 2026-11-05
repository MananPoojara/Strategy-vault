import type {AuthorFile} from '../author-files';

export default {
  order: 2070,
  author: { id: "wilcox", name: "Cole Wilcox & Eric Crittenden", tagline: "Blackstar Funds · the all-time-high trend-following test on stocks runs here", books: [
    { id: "wcdtf", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Cole Wilcox - Does Trend Following Work on Stocks (Article)/Does trendfollowing work on stocks.pdf"], title: "Does Trend Following Work on Stocks?", year: 2005, blurb: "Wilcox & Crittenden · buy the open after a close at the all-time high, exit the open after a close below a 10-ATR trailing stop (ATR length not printed: 14, labelled)" }] },
  identities: ["Cole Wilcox", "Eric Crittenden"],
  sourceReview: {
    "wcdtf": {"type":"article","people":["Cole Wilcox","Eric Crittenden"]},
  },
} satisfies AuthorFile;
