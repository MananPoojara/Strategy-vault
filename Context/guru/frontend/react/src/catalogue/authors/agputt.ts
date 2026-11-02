import type {AuthorFile} from '../author-files';

export default {
  order: 2020,
  author: { id: "agputt", name: "Arthur G. Putt & William Brower", tagline: "Using EasyLanguage 2000 · the book's daily tutorial systems (2% Reversal, Trend Daily, RSI-%R) as printed", books: [
    { id: "pbuel", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Arthur G.Putt, William Brower - Using EasyLanguage 2000/Using EL 2k by Arthur G. Putt & William Brower.pdf"], title: "Using EasyLanguage 2000 (Putt & Brower)", year: null, blurb: "Putt & Brower · 2% Reversal stops, Trend Daily 32-day average setups with a dollar stop and target (US only), RSI-%R zone crosses" }] },
  identities: ["Arthur G. Putt", "William Brower"],
  sourceReview: {
    "pbuel": {"type":"book","people":["Arthur G. Putt", "William Brower"]},
  },
} satisfies AuthorFile;
