import type {AuthorFile} from '../author-files';

export default {
  order: 740,
  author: { id: "jcarter", name: "John F. Carter", tagline: "Futures and options trader (Simpler Trading) · the Squeeze, HOLP/LOHP and Propulsion plays (backtested); intraday YM/ES plays held", books: [
    { id: "jcmtt", sourceIds: ["title-68858e52c624c55c"], title: "Mastering the Trade (3rd edition)", year: 2018, blurb: "Carter, 2018 · the Squeeze, HOLP/LOHP and Propulsion pull-back plays on daily bars; gap, pivot, tick and 3:52 day trades held" },
    { id: "jcteu", sourceIds: ["title-000348d7a368b51a"], title: "Trading the Euro: Box It, Squeeze It, Love It, Leave It", year: null, blurb: "Carter · euro box play (hand-drawn box) and squeeze; held: forex, chart reading" },
    { id: "jcym5", sourceIds: ["title-13d8a00ffeb58a5c"], title: "My 5-Minute Method for Trading the Mini-Dow for a Living", year: null, blurb: "Carter · 5-minute YM pivot entries with 20-point stops; held: intraday futures" },
    { id: "jcags", sourceIds: ["title-526d7a8dfa1e2250"], title: "Online Day Trading Seminar on Grains", year: 2007, blurb: "Carter & Hubert Senters · grain futures setups; held: futures not in the vault" },
    { id: "jccbot", sourceIds: ["title-260b2a8f34b4cf47"], title: "CBOT Trading Lessons WorkBook", year: null, blurb: "Carter · mini-sized Dow lessons and pivot method; held: intraday futures" }] },
  identities: ["John F. Carter"],
  profile: {
    "kind": "person",
    "mastery": [
      "Stock index futures",
      "Options",
      "Swing trading"
    ],
    "summary": "John F. Carter trades stock-index futures, currencies and stock options and founded Simpler Trading. Mastering the Trade sets out his day-trading and swing setups: the opening gap, pivots, the Squeeze, HOLP/LOHP and Propulsion plays. This profile rests on the books themselves.",
    "known_for": [
      "Mastering the Trade",
      "The Squeeze"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "jcmtt": {"type":"book","people":["John F. Carter"]},
    "jcteu": {"type":"article","people":["John F. Carter"]},
    "jcym5": {"type":"course_manual","people":["John F. Carter"]},
    "jcags": {"type":"course_manual","people":["John F. Carter"]},
    "jccbot": {"type":"course_manual","people":["John F. Carter"]},
  },
} satisfies AuthorFile;
