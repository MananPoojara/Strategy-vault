import type {AuthorFile} from '../author-files';

export default {
  order: 1310,
  author: { id: "wyckoff", name: "Richard D. Wyckoff", tagline: "Tape reading and the Wyckoff method of accumulation and distribution (judgement, held)", books: [
    { id: "rwdtb", sourceIds: ["title-b9d705e1fc938a87", "title-0d03b88d788c0a44", "title-24b512b9bc7d792e", "title-18946614c3ba3f12"], title: "The Day Trader's Bible", year: 1919, blurb: "Wyckoff · tape reading; held" },
    { id: "rwhit", sourceIds: ["title-7d1fbe74c56129b2", "title-d8b6e05f7d9b1d93"], title: "How I Trade and Invest in Stocks and Bonds", year: 1922, blurb: "Wyckoff · trading principles; held" },
    { id: "rwsmt", sourceIds: ["title-0cfa0308d47e2924"], title: "Stock Market Technique No. 1", year: 1933, blurb: "Wyckoff · the Wyckoff method (also the course units); held" }] },
  identities: ["Richard D. Wyckoff"],
  sourceReview: {
    "rwdtb": {"type":"book","people":["Richard D. Wyckoff"]},
    "rwhit": {"type":"book","people":["Richard D. Wyckoff"]},
    "rwsmt": {"type":"book","people":["Richard D. Wyckoff"]},
  },
} satisfies AuthorFile;
