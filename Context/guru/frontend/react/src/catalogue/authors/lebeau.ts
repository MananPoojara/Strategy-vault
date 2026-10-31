import type {AuthorFile} from '../author-files';

export default {
  order: 150,
  author: { id: 'lebeau', name: 'Charles LeBeau & David Lucas', tagline: 'ADX, volatility breakouts and exits · Computer Analysis of the Futures Market', books: [{ id: 'lebcafm', title: 'Computer Analysis of the Futures Market', year: 1992, blurb: 'LeBeau & Lucas, 1992 · ADX + Parabolic, volatility breakout, Babcock DMI, Lambert CCI and the Ch.3 entry/exit tests (16 rules) · day-trading methods held (S&P points, judgement)' }, { id: 'lebtc', title: 'System Traders Club manuals', year: 1998, blurb: 'traderclub.com · High Beta Stock system runs; the bond and Swiss franc systems need futures data' }, { id: 'lebsp', title: "Traderclub S&P / NYFE systems", year: null, blurb: "LeBeau · Prudent, Remarkable, Mack and Modified Piranha; on the manuals’ own data 458 of 462 trades match · held: stops are fixed S&P/NYFE futures points" }, { id: 'lebfut', title: "Traderclub futures systems", year: null, blurb: "LeBeau · ten currency, crude, bond and yen systems, all need futures data; five reproduce their manuals exactly" }, { id: 'lebws', title: "Workshop handouts (Futures Strategies, Precise Exits, On Average)", year: null, blurb: "LeBeau · one ATR breakout system; the rest are examples without complete rules" }, { id: 'lebtcb', title: "Traders Club bulletins and forum", year: null, blurb: "LeBeau’s club · Ronco adaptive channel and volatility breakouts, TBond Attack and Johnson’s Pretty Good Oscillator" }] },
  identities: ["Charles LeBeau","David Lucas"],
  profile: {
    "kind": "person",
    "mastery": [
      "Indicators",
      "Futures",
      "Position sizing & risk"
    ],
    "summary": "Charles (Chuck) LeBeau spent twenty years at E.F. Hutton before becoming a CTA and hedge-fund manager. He is known for research on exits, including the Chandelier Exit, and ran the System Traders Club. With David Lucas he wrote a classic guide to computer analysis of futures indicators.",
    "known_for": [
      "Chandelier Exit",
      "Technical Traders Guide to Computer Analysis of the Futures Markets",
      "System Traders Club"
    ],
    "sources": [
      "https://cmtassociation.org/presenter/chuck-lebeau/",
      "https://traders.com/documentation/feedbk_docs/2003/11/Abstracts_new/Interview/interview.html"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "lebcafm": {"type":"book","people":["Charles LeBeau","David Lucas"]},
    "lebtc": {"type":"code_package","people":["Charles LeBeau","David Elden","Terence Tan"]},
    "lebsp": {"type":"code_package","people":["Bruce Arnheim","Charles LeBeau","David Elden","Jack Bicer"]},
    "lebfut": {"type":"code_package","people":["Charles LeBeau","David Elden","David Lucas","Terence Tan"]},
    "lebws": {"type":"course_manual","people":["Charles LeBeau"]},
    "lebtcb": {"type":"magazine_volume","people":["Charles LeBeau"]},
  },
} satisfies AuthorFile;
