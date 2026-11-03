import type {AuthorFile} from '../author-files';

export default {
  order: 2010,
  author: { id: "akrzhechevsky", name: "Alex Krzhechevsky", tagline: "WWW Trading System (webtradingsystem.com) · EMA 5/13 forex cross, run on daily bars as a declared application", books: [
    { id: "wwwts", sourceIds: [], readerPaths: ["USB HDD 2/Trading Knowledge 3/FOREX WWW Trading System $99 (WebTradingSystem.com)/system.pdf"], title: "WWW Trading System (webtradingsystem.com)", year: null, blurb: "Krzhechevsky · EMA 5 crossing EMA 13, long or short at the next open, out on the cross back; the pip stop and filters are held" }] },
  identities: ["Alex Krzhechevsky"],
  sourceReview: {
    "wwwts": {"type":"other","people":["Alex Krzhechevsky"]},
  },
} satisfies AuthorFile;
