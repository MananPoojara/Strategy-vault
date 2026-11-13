import type {AuthorFile} from '../author-files';

export default {
  order: 2120,
  author: { id: "osaari", name: "Orville Saari", tagline: "Momentum Trend Trader · a momentum line (MACD 39/78/1) and midpoint averages; the book says it is not a trading system, so all 5 set-ups are held", books: [
    { id: "osmtt", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Orville Saari - Momentum Trend Trader (85 MB) (momentumtrend.com)/Momentum Trend Trader.pdf"], title: "Momentum Trend Trader", year: 2003, blurb: "Saari e-book (momentumtrend.com, 2003-2004) · momentum line, stochastics with longer-term momentum, the 'simple but explosive' new-high pullback, 27/54 midpoint averages; entries and exits left to the trader" }] },
  identities: ["Orville Saari"],
  sourceReview: {
    "osmtt": {"type":"book","people":["Orville Saari"]},
  },
} satisfies AuthorFile;
