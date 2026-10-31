import type {AuthorFile} from '../author-files';

export default {
  order: 1080,
  author: { id: "jennings", name: "David Jennings", tagline: "Compiler of trading-software code (The Performance Factory) · six systems from his collection run here", books: [
    { id: "djcs", sourceIds: [], readerPaths: ["USB HDD 1/MetaStock/@Knowledge/David Jennings - Code Snippets & How to for Metastock, Tradestation & AIQ/Code Snippets2.doc"], title: "Code Snippets & How To (MetaStock, TradeStation, AIQ)", year: null, blurb: "Jennings collection · Bob Fulks' stop and reverse 1% under the highest high / 2% over the lowest low since the last change; James Taylor's version of Bryan Strain's TFS system: close beyond the 50-day mid-channel with the 7-day volume oscillator agreeing, stop and reverse (Strain's original cross logic as an option); the MarketSpace Stock System (MACD over its 45-day average with the close over the 25-day, out on a 10% drop from 7 days earlier); Glenn Wallace's Buy trigger (10/30-day EMA spread over 5%); the WealthLab Dead Cat Catcher and CMO(20)/CumDown accumulation. Party Girl and CandleCode are held (WealthLab auto stops not defined). The code is by other people, credited in the file; it prints no results" }] },
  identities: ["David Jennings"],
  profile: {
    "kind": "person",
    "mastery": [
      "Trading-software code (MetaStock, TradeStation, AIQ, WealthLab)"
    ],
    "summary": "David Jennings (The Performance Factory) compiled Code Snippets & How To, a large Word file of indicator and system code for MetaStock, TradeStation, AIQ and WealthLab gathered from trader mailing lists around 2000-2002, each piece credited to its writer. The systems in it are other people's work; this profile rests on the file itself.",
    "known_for": [
      "Code Snippets & How To (MetaStock, TradeStation, AIQ)"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "djcs": {"type":"book","people":["David Jennings"]},
  },
} satisfies AuthorFile;
