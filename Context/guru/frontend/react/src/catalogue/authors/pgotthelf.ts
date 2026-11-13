import type {AuthorFile} from '../author-files';

export default {
  order: 2420,
  author: { id: "pgotthelf", name: "Philip Gotthelf", tagline: "Precious Metals Trading · the 30-day moving average trend rule of ch.5, stop and reverse at the close, with two labelled readings (R4 #297)", books: [
    { id: "pgpmt", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Philip Gotthelf - Precious Metals Trading/0471711519.pdf"], title: "Precious Metals Trading", year: 2005, blurb: "Gotthelf (Wiley 2005) · metals fundamentals, seasonals and options; ch.5 follows silver and gold futures with a simple 30-day moving average and a penetration factor it never sizes; chart examples only" }] },
  identities: ["Philip Gotthelf"],
  sourceReview: {
    "pgpmt": {"type":"book","people":["Philip Gotthelf"]},
  },
} satisfies AuthorFile;
