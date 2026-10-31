import type {AuthorFile} from '../author-files';

export default {
  order: 810,
  author: { id: "downs", name: "Walter T. Downs", tagline: "Pattern researcher and forex system seller · the Forex Gambit pullback and 37 Trading for Tigers bar patterns run here; open-interest patterns held", books: [
    { id: "wtdfg", sourceIds: ["title-32a9f443563ba84c"], title: "The Forex Gambit", year: null, blurb: "Downs, e-book · inside-down bar above the 30-day Bollinger middle line, buy at the close, stop at the bar low, out after a 2 SD close and a lower close, breakeven after 4 days, 16-day time exit" },
    { id: "wtdtt", sourceIds: ["title-40f478385458a965"], title: "Trading for Tigers", year: null, blurb: "Downs · 14 monthly and 8 weekly breakout patterns (out at the end of the month/week), 9 market-on-close weekly/monthly patterns (out at the next open) and 6 MIST daily patterns (US only); open-interest versions and code-only patterns held" }] },
  identities: ["Walter T. Downs"],
  profile: {
    "kind": "person",
    "mastery": [
      "Currency trading",
      "Bar pattern research"
    ],
    "summary": "Walter T. Downs sold The Forex Gambit, a daily currency system: buy an inside-down bar that closes in its upper half above a rising 30-day Bollinger middle line, stop at the bar's low, exit after a close above the 2 SD band followed by a lower close. In Trading for Tigers he catalogued short-term monthly, weekly and market-on-close bar patterns tested across futures and stocks, and the MIST short-term trend definition. This profile rests on the two books themselves.",
    "known_for": [
      "The Forex Gambit",
      "Trading for Tigers"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "wtdfg": {"type":"book","people":["Walter T. Downs"]},
    "wtdtt": {"type":"book","people":["Walter T. Downs"]},
  },
} satisfies AuthorFile;
