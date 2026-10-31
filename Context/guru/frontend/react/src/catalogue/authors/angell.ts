import type {AuthorFile} from '../author-files';

export default {
  order: 1290,
  author: { id: "angell", name: "George Angell", tagline: "Futures day trader and author · Taylor-book levels, LSS 3-day cycle, small caps, options (held)", books: [
    { id: "gasnw", sourceIds: ["title-9057d8a9e126225f", "title-03762f250c91daef"], title: "Sniper Trading Workbook", year: 2002, blurb: "Angell, 2002 · buy/sell envelopes and range projections for S&P and bond futures; held: levels need judgement, intraday" },
    { id: "gassp", sourceIds: ["title-ffe1b82681a825b6"], title: "Small Stocks for Big Profits", year: 2008, blurb: "Angell, 2008 · small-cap selection and chart reading; held" },
    { id: "gasto", sourceIds: ["title-756076f4305a0b14", "title-f3b8782982320c88"], title: "Sure-Thing Options Trading", year: 1983, blurb: "Angell · option strategies; held" },
    { id: "gacdt", sourceIds: ["title-4cb0eba9141bebe1"], title: "George Angell's Complete Day-Trading Course", year: null, blurb: "Angell · course manual: one- and four-day percent method, envelopes, gap rules; held: intraday futures levels and judgement" }] },
  identities: ["George Angell"],
  sourceReview: {
    "gasnw": {"type":"book","people":["George Angell"]},
    "gassp": {"type":"book","people":["George Angell"]},
    "gasto": {"type":"book","people":["George Angell"]},
    "gacdt": {"type":"course_manual","people":["George Angell"]},
  },
} satisfies AuthorFile;
