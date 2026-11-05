import type {AuthorFile} from '../author-files';

export default {
  order: 2040,
  author: { id: "stovall", name: "Sam Stovall", tagline: "S&P chief investment strategist · Sell in May sector rotation and monthly sector momentum with the Select Sector SPDRs", books: [
    { id: "stv7", sourceIds: [], title: "The Seven Rules of Wall Street", year: 2009, blurb: "Stovall, 2009 · Rule 3 Sell in May (SPY November-April, XLP May-October) and Rule 5 There's Always a Bull Market Someplace (top 3 sector SPDRs by 12-month change, monthly)" }] },
  identities: ["Sam Stovall"],
  sourceReview: {
    "stv7": {"type":"book","people":["Sam Stovall"]},
  },
} satisfies AuthorFile;
