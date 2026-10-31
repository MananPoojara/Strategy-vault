import type {AuthorFile} from '../author-files';

export default {
  order: 1700,
  author: { id: "kraphael", name: "Keith Raphael", tagline: "Telerate TAG workshop speaker \u00b7 the TRADEPLAN 'trade band' runs here", books: [
    { id: "krovl", sourceIds: [], readerPaths: ["Trading Knowledge II/@INO (ino.com)/Keith Raphael - How to Overlay Technical Indicators/workbook.pdf"], title: "How to Overlay Technical Indicators (Telerate TAG XVII workbook)", year: 1995, blurb: "Raphael \u00b7 fade a 1.95-sigma band around the 5-day average, out at the average or a 0.5% stop; Swiss franc futures example, no printed results" }] },
  identities: ["Keith Raphael"],
  sourceReview: {
    "krovl": {"type":"course_manual","people":["Keith Raphael"]},
  },
} satisfies AuthorFile;
