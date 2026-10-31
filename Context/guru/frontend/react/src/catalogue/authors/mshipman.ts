import type {AuthorFile} from '../author-files';

export default {
  order: 1806,
  author: { id: "mshipman", name: "Mark Shipman", tagline: "Fund manager, Kogan Page author · his weekly moving-average and breakout investment rules run here", books: [
    { id: "msbml", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Mark Shipman - Big Money Little Efford/Big Money Little Effort.PDF"], title: "Big Money, Little Effort", year: 2008, blurb: "Shipman · invested while the 30-week average is above the 50-week, in cash when below; S&P 500 ledger 1951-2007" },
    { id: "msnib", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Mark Shipman - The Next Big Investment Boom/Mark Shipman The Next Big Investment Boom.pdf"], title: "The Next Big Investment Boom", blurb: "Shipman · invested above a rising 40-week average, or only after a 12-week upside breakout; out when the close or the average turns" }] },
  identities: ["Mark Shipman"],
  sourceReview: {
    "msbml": {"type":"book","people":["Mark Shipman"]},
    "msnib": {"type":"book","people":["Mark Shipman"]},
  },
} satisfies AuthorFile;
