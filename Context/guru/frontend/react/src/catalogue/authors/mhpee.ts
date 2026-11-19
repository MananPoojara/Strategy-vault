import type {AuthorFile} from '../author-files';

export default {
  order: 2460,
  author: { id: "mhpee", name: "M.H. Pee", tagline: "S&C 2001 · the trend detection index on a futures basket", books: [
    { id: "mhptdi", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V19/C10/112TREN.pdf"], title: "Trend Detection Index (S&C, 2001)", year: 2001, blurb: "Pee · 20-day momentum direction traded while the TDI is positive, always in; Figure 2's futures basket on Norgate continuous contracts (13 of the 15 run: no Deutschemark, no crude)" }] },
  identities: ["M.H. Pee"],
  sourceReview: {
    "mhptdi": {"type":"article","people":["M.H. Pee"]},
  },
} satisfies AuthorFile;
