import type {AuthorFile} from '../author-files';

export default {
  order: 2440,
  author: { id: "jdicks", name: "James Dicks", tagline: "Forex Trading Secrets · the EMA 5/13 cross from ch.15, on daily bars with two labelled readings (R4 #297)", books: [
    { id: "jdfts", sourceIds: [], readerPaths: ["Trading Knowledge I/James Dicks - Forex Trading Secrets. Trading Strategies for the Forex Market/James Dicks - Forex Trading Secrets. Trading Strategies for the Forex Market.pdf"], title: "Forex Trading Secrets: Trading Strategies for the Forex Market", year: 2009, blurb: "Dicks (McGraw-Hill 2009) · forex primer; ch.15 'My favorite ways to trade': 50 EMA + RSI/MACD, EMA 5/13 cross, gap fade, daily high-low; only the 5/13 cross is mechanical" }] },
  identities: ["James Dicks"],
  sourceReview: {
    "jdfts": {"type":"book","people":["James Dicks"]},
  },
} satisfies AuthorFile;
