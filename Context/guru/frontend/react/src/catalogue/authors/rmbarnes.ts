import type {AuthorFile} from '../author-files';

export default {
  order: 1730,
  author: { id: "rmbarnes", name: "Robert M. Barnes", tagline: "INO workshop author \u00b7 DRV and Volume-Price methods run here", books: [
    { id: "rbtie", sourceIds: [], readerPaths: ["Trading Knowledge II/@INO (ino.com)/Robert M.Barnes - Timing is Everything/workbook.pdf"], title: "Timing is Everything (INO workshop workbook)", blurb: "Barnes \u00b7 Directional Relative Volatility stop-and-reverse and the volume-price trend method, S&P futures optima as defaults" }] },
  identities: ["Robert M. Barnes"],
  sourceReview: {
    "rbtie": {"type":"course_manual","people":["Robert M. Barnes"]},
  },
} satisfies AuthorFile;
