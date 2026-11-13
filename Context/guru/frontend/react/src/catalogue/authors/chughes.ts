import type {AuthorFile} from '../author-files';

export default {
  order: 2200,
  author: { id: "chughes", name: "Chuck Hughes", tagline: "The Key to Profits in Tough Times · the Major Trend System (month-end close vs 20-month EMA) and the 50/100-day EMA system; 2 run here, 2 held (fundamentals, options)", books: [
    { id: "chktp", sourceIds: [], readerPaths: ["Trading Books/Chuck Hughes - The Key to Profits in Tough Times/Chuck Hughes - The Key to Profits in Tough Times.pdf"], title: "The Key to Profits in Tough Times", year: 2010, blurb: "Hughes, Legacy Publishing 2010 (a set of his reports) · Major Trend System long/short, 50/100-day EMA crossovers with the Major Trend as the short filter, Wealth Building Formula stocks, call and put buying; results as charts and end values" }] },
  identities: ["Chuck Hughes"],
  sourceReview: {
    "chktp": {"type":"book","people":["Chuck Hughes"]},
  },
} satisfies AuthorFile;
