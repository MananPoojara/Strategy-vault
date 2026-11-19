import type {AuthorFile} from '../author-files';

export default {
  order: 2200,
  author: { id: "joshaughnessy", name: "James P. O'Shaughnessy", tagline: "What Works on Wall Street · the 50 best one-year price performers, bought each year end, run on the vault's S&P 500 basket (labelled universe change); the fundamental factor portfolios need data we do not hold", books: [
    { id: "jowws", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/James P.O'Shaughnessy - What Works on Wall Street/What Works on Wall Street (James P. O'Shaughnessy, 1996).pdf"], title: "What Works on Wall Street", year: 1997, blurb: "O'Shaughnessy, 1997 · Compustat factor tests 1951-1994: relative strength, P/E, price-to-book, price-to-sales, yield and composite strategies, rebalanced annually" }] },
  identities: ["James P. O'Shaughnessy"],
  sourceReview: {
    "jowws": {"type":"book","people":["James P. O'Shaughnessy"]},
  },
} satisfies AuthorFile;
