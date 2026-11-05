import type {AuthorFile} from '../author-files';

export default {
  order: 2050,
  author: { id: "faber", name: "Mebane T. Faber", tagline: "Quantitative asset manager · monthly relative-strength sector rotation", books: [
    { id: "fbrs", sourceIds: [], title: "Relative Strength Strategies for Investing", year: 2010, blurb: "Faber, 2010 (CMT Level II reading) · hold the top 3 sectors by the average 1/3/6/9/12-month return, rebalanced at each month end" }] },
  identities: ["Mebane T. Faber"],
  sourceReview: {
    "fbrs": {"type":"book","people":["Mebane T. Faber"]},
  },
} satisfies AuthorFile;
