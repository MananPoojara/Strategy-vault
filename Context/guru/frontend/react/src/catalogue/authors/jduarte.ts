import type {AuthorFile} from '../author-files';

export default {
  order: 1780,
  author: { id: "jduarte", name: "Joe Duarte", tagline: "Market timing author \u00b7 two calendar rules from chapter 6 run here", books: [
    { id: "jdmtd", sourceIds: [], readerPaths: ["Trading Knowledge I/Joe Duarte - Marking Timing for Dummies/Market_Timing_For_Dummies.pdf"], title: "Market Timing for Dummies", blurb: "Duarte \u00b7 in November-January, March-April and July-August; the small-cap January window from 31 December to 28 February" }] },
  identities: ["Joe Duarte"],
  sourceReview: {
    "jdmtd": {"type":"book","people":["Joe Duarte"]},
  },
} satisfies AuthorFile;
