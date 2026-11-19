import type {AuthorFile} from '../author-files';

export default {
  order: 2200,
  author: { id: "wslatyer", name: "Will Slatyer", tagline: "The Speculative Strategist · serial trading through the 13-day EMA of highs and lows runs here; the Henry Morgan tactics and the Woodes Rogers strategy are held (judged exits and chart readings)", books: [
    { id: "wsspec", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Will Slatyer - The Speculative Strategist/The Speculative Strategist.pdf"], title: "The Speculative Strategist", year: 1996, blurb: "Slatyer, 1996 · FIRMROC rate of change, 3/13 moving averages and MACD over daily, weekly and monthly charts; Woodes Rogers strategy, Henry Morgan tactics, serial trading on the DJIA" }] },
  identities: ["Will Slatyer"],
  sourceReview: {
    "wsspec": {"type":"book","people":["Will Slatyer"]},
  },
} satisfies AuthorFile;
