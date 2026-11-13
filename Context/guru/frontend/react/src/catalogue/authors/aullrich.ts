import type {AuthorFile} from '../author-files';

export default {
  order: 2110,
  author: { id: "aullrich", name: "Arthur H. Ullrich", tagline: "Day Trading S&P 500 Index Futures · a 1996 e-mail course on 5-minute S&P futures bars; 15 entries and tools, all held (discretionary or no exit)", books: [
    { id: "uldsp", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Arthur H.Ullrich - DayTrading S&P Futures/Ullrich.zip"], title: "Day Trading S&P 500 Index Futures Contract", year: 1996, blurb: "Ullrich's course text (revised 20 Oct 1996, Windows Write file) · Trend Turn, BEK, 123, Second Signal, 20/20, Gap Trade, English, outside and engulfing bars, Holy Grail, 4th Bar; stops, targets and trailing by judgement; no performance tables" }] },
  identities: ["Arthur H. Ullrich"],
  sourceReview: {
    "uldsp": {"type":"course_manual","people":["Arthur H. Ullrich"]},
  },
} satisfies AuthorFile;
