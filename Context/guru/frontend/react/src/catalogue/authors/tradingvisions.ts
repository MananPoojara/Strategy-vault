import type {AuthorFile} from '../author-files';

export default {
  order: 2800,
  author: { id: "tradingvisions", name: "TradingVisions", tagline: "Impetus DayTrading · TradeStation retrace and impetus stops, run here on SPX 5-minute bars", books: [
    { id: "impet", sourceIds: [], readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/Impetus DayTrading System (tradingvisions.com)/Impetus_Demo.6.txt"], title: "Impetus DayTrading System (TradingVisions)", blurb: "TradeStation EasyLanguage demo on a 5-minute chart with a daily Data2 · between 13:30 and 15:00 a retrace short/long or an impetus break of the day's high/low (stop 0.05 beyond), one trade a day, stops at the prior day's extreme or 0.9%/0.1% of the prior close, a 1-point lock after a 0.8% move, close at the end of the day; runs on SPX 5-minute bars built from 1-minute data (S&P points literal) as a declared adaptation, no trade list printed, unverified" }] },
  identities: ["TradingVisions"],
  profile: {
    "kind": "organization",
    "mastery": ["TradeStation day-trading systems"],
    "summary": "TradingVisions (tradingvisions.com) sold TradeStation system packages for index futures, among them the Impetus DayTrading System tested on the S&P 500 index.",
    "known_for": ["Impetus DayTrading System"],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "impet": {"type":"course_manual","people":["TradingVisions"]},
  },
} satisfies AuthorFile;
