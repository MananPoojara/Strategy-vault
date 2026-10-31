import type {AuthorFile} from '../author-files';

export default {
  order: 1230,
  author: { id: "nasdaqprofits", name: "NasdaqProfits.com", tagline: "Calendar-date trading ebook · the 60% table runs here", books: [
    { id: "nqpcal", sourceIds: [], title: "Nasdaq Profits (ebook)", year: 2008, blurb: "Open-to-close trades on the calendar dates its table marks 60% or better up or down; written for the Nasdaq Composite, run on NDX/QQQ and others as a declared application" }] },
  sourceReview: {
    "nqpcal": {"type":"book","people":[]},
  },
} satisfies AuthorFile;
