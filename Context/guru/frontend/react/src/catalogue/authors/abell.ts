import type {AuthorFile} from '../author-files';

export default {
  order: 1320,
  author: { id: "abell", name: "Howard Abell", tagline: "Floor trader and author · day-trading patterns and futures spreads (held)", books: [
    { id: "haddt", sourceIds: ["title-c34b63ac910c13fd"], title: "Digital Day Trading", year: 1999, blurb: "Abell, 1999 · Taylor swing days, inside/constricted-range breakouts, interviews; held: no exits, judgement" },
    { id: "haspr", sourceIds: ["title-327b1e8156c61ae5"], title: "Spread Trading", year: null, blurb: "Abell · futures spreads; held: needs futures prices (djvu copy not readable here)" },
    { id: "ha6mk", sourceIds: [], readerPaths: ["Trading Books/Howard Abell - The Sixth Market. The Electronic Investor Revolution/files/page_163.html"], title: "The Sixth Market: The Electronic Investor Revolution", year: null, blurb: "Abell · ch.13 TradeCast setups (revival, wedge and smart buy/sell): 1/8-point stop entries; held: fixed 1- and 3-point stops/targets on Nasdaq stocks (#166)" }] },
  identities: ["Howard Abell"],
  sourceReview: {
    "haddt": {"type":"book","people":["Howard Abell"]},
    "haspr": {"type":"book","people":["Howard Abell"]},
    "ha6mk": {"type":"book","people":["Howard Abell"]},
  },
} satisfies AuthorFile;
