import type {AuthorFile} from '../author-files';

export default {
  order: 2000,
  author: { id: "ppontikis", name: "Peter Pontikis", tagline: "CMC Markets education course author · percentage bands and RSI with an ADX filter, as teaching examples", books: [
    { id: "pptca", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Peter Pontikis - Technical Analysis (cmcmarkets.com.au)/pdfs/strategy_course_notes.pdf", "USB HDD 2/tRADING KNO-2/Peter Pontikis - Technical Analysis (cmcmarkets.com.au)/pdfs/mechanics_course_notes.pdf"], title: "Technical Analysis course notes (CMC Markets)", year: null, blurb: "Pontikis · weekly 40-week average with 10% bands faded, and RSI(14) 30/70 signals ignored while the ADX is rising" }] },
  identities: ["Peter Pontikis"],
  sourceReview: {
    "pptca": {"type":"course_manual","people":["Peter Pontikis"]},
  },
} satisfies AuthorFile;
