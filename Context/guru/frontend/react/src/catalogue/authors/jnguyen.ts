import type {AuthorFile} from '../author-files';

export default {
  order: 2110,
  author: { id: "jnguyen", name: "Jason Nguyen", tagline: "Impact Trading System · SMA 8/20/40 trend with a CCI(8) pullback, stop entry and 1R/2R targets; runs here on daily bars over the 1-minute windows", books: [
    { id: "jnits", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Jason Nguyen - Impact Trading System (tradingimpact.com)/impact forex trading system part 1.pdf"], title: "Impact Trading System", year: null, blurb: "Nguyen e-book (tradingimpact.com, part 1, no date) · hourly forex charts; two lots with 1R/2R targets or an SMA5-slope exit for lot 2; chart examples only" }] },
  identities: ["Jason Nguyen"],
  sourceReview: {
    "jnits": {"type":"book","people":["Jason Nguyen"]},
  },
} satisfies AuthorFile;
