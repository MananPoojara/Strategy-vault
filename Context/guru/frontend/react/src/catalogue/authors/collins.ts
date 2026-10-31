import type {AuthorFile} from '../author-files';

export default {
  order: 440,
  author: { id: 'collins', name: "Art Collins", tagline: "Index-futures day and swing systems with printed TradeStation code", books: [{ id: 'acbff', title: "Beating the Financial Futures Market", year: 2006, blurb: "Collins, 2006 · 75 daily S&P systems run code-exact on the index; intraday, bond and soybean systems need data" }] },
  identities: ["Art Collins"],
  profile: {
    "kind": "person",
    "mastery": [
      "Futures",
      "System design & testing"
    ],
    "summary": "Art Collins has developed and traded mechanical commodity systems since 1986 and was a Chicago Board of Trade member. His Beating the Financial Futures Market combines small statistical biases into rule-based futures strategies; he also wrote Market Beaters, a book of interviews with mechanical traders.",
    "known_for": [
      "Beating the Financial Futures Market (2006)",
      "Market Beaters"
    ],
    "sources": [
      "https://doi.org/10.1002/9781119196594.about",
      "https://nlb.overdrive.com/media/112180"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "acbff": {"type":"book","people":["Art Collins"]},
  },
} satisfies AuthorFile;
