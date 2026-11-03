import type {AuthorFile} from '../author-files';

export default {
  order: 1900,
  author: { id: "sduddella", name: "Suri Duddella", tagline: "surinotes.com · three chart-pattern setups with their own stops and targets: n-Bar Rally/Decline, NR7 inside day, Donchian 4-week", books: [
    { id: "sdtcp", sourceIds: [], readerPaths: ["Trading Knowledge I/Suri Duddella - Trade Chart Patterns Like The Pros (Included TS Code) (surinotes.com)/Trade Chart Patterns _Suri_Duddella.pdf"], title: "Trade Chart Patterns Like The Pros", year: 2007, blurb: "Duddella · subset: n-Bar Decline/Rally (62% target), NR7ID 10-cent breakout (3-day exit), Donchian 20-day with 2-ATR target; the hand-drawn and intraday patterns are held" }] },
  identities: ["Suri Duddella"],
  sourceReview: {
    "sdtcp": {"type":"book","people":["Suri Duddella"]},
  },
} satisfies AuthorFile;
