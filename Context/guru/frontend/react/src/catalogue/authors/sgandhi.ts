import type {AuthorFile} from '../author-files';

export default {
  order: 2090,
  author: { id: "sgandhi", name: "Sourabh Gandhi", tagline: "Breakout Intraday Calculator · the 0.45 / 0.75 range breakout runs here on index minute bars", books: [
    { id: "sgbic", sourceIds: [], readerPaths: ["Prasad/ALL BOOKS/Intraday-Using-Breakout.pdf"], title: "Breakout Intraday Calculator (Intraday trading using Breakout Method)", year: null, blurb: "Gandhi · yesterday's range x 0.45 / 0.75 against today's range sets the breakout levels; 0.7% target, 0.8% stop, square off after 10 minutes on the wrong side; written for single NSE stocks" }] },
  identities: ["Sourabh Gandhi"],
  sourceReview: {
    "sgbic": {"type":"other","people":["Sourabh Gandhi"]},
  },
} satisfies AuthorFile;
