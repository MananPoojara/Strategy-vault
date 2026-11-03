import type {AuthorFile} from '../author-files';

export default {
  order: 1870,
  author: { id: "wthorp", name: "Wayne A. Thorp", tagline: "AAII financial analyst \u00b7 the nine-day RSI and slow stochastic rules from his 2000 AAII Journal articles", books: [
    { id: "wtaaii", sourceIds: [], readerPaths: ["Trading Knowledge III/!PDFs/Wayne Thorp - Measuring Internal Strength - Wilders RSI Indicator.pdf"], title: "AAII Journal indicator articles: Wilder's RSI (May 2000) and the Stochastic Oscillator (Oct 2000)", year: 2000, blurb: "Thorp \u00b7 RSI(9) crosses back above 30 / below 70 (MSFT: 5 round trips, +106.5%); slow %D (5,3) back above 20 / below 80" }] },
  identities: ["Wayne A. Thorp"],
  sourceReview: {
    "wtaaii": {"type":"article","people":["Wayne A. Thorp"]},
  },
} satisfies AuthorFile;
