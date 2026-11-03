import type {AuthorFile} from '../author-files';

export default {
  order: 1811,
  author: { id: "wbressert", name: "Walter Bressert", tagline: "Cycle trader, ProfitTrader software · the RSI3M3 mechanical buy signal with his two-bar reversal stop runs here", books: [
    { id: "wbctp", sourceIds: [], readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/_Misc/Walter Bressert Indicator for OR/WalterBressetStuff/Manual.PDF", "USB HDD 1/TradeStation Systems & Indicators/_Misc/Walter Bressert's ProfitTrader 7.0 + Manual for TS (walterbressert.com)/PT7_Manual_100102.pdf"], title: "The Cycle Trading Pattern Manual, with the ProfitTrader 7.0 manuals", year: null, blurb: "Bressert · RSI3M3 turns up from below 30, buy stop above the setup bar, stop under the cycle bottom, two-bar reversal trailing stop" }] },
  identities: ["Walter Bressert"],
  sourceReview: {
    "wbctp": {"type":"other","people":["Walter Bressert"]},
  },
} satisfies AuthorFile;
