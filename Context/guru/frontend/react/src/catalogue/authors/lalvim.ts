import type {AuthorFile} from '../author-files';

export default {
  order: 1805,
  author: { id: "lalvim", name: "Luiz Alvim", tagline: "alphafin.com · the Global Trading System rules (3-bar reversal + Slow Alpha turn) run here", books: [
    { id: "lagts", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Luiz Alvin - The Global System (alphafin.com)/Global 2.doc"], title: "The Global Trading System (alphafin.com manual, ch.3 'Making the Trade Decision')", blurb: "Alvim · a 3-bar reversal close inside the 5/5 channel after the Slow Alpha (MACD 19/39) turns; stop beyond the 5-bar extreme, out when the Slow Alpha turns back" }] },
  identities: ["Luiz Alvim"],
  sourceReview: {
    "lagts": {"type":"book","people":["Luiz Alvim"]},
  },
} satisfies AuthorFile;
