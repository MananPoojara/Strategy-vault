import type {AuthorFile} from '../author-files';

export default {
  order: 770,
  author: { id: "csmith", name: "Courtney D. Smith", tagline: "FX and futures trader · swing-point trend analysis, enhanced channel breakouts, the Conqueror", books: [
    { id: "csmlt", sourceIds: ["title-bafc2391a8c8f66d"], title: "How to Make a Living Trading Foreign Exchange", year: 2010, blurb: "Smith, 2010 · trend analysis, 55/20 channel breakout with the rejection rule, Conqueror I/II, stochastic 50 line, reversal days and RRR run as declared applications; inside days, slingshot and divergences held" },
    { id: "csops", sourceIds: ["title-9dfc05d60c19e423"], title: "Option Strategies (2nd ed.)", year: 1996, blurb: "Smith, 1996 · option spread structures picked to a market view; nothing testable on the underlying" },
    { id: "csscf", sourceIds: ["title-d9e81a63106a6b24"], title: "Seasonal Charts for Futures Traders", year: 1987, blurb: "Smith, 1987 · futures seasonal sourcebook; needs futures prices, no fixed rule" }] },
  identities: ["Courtney D. Smith","Courtney Smith"],
  profile: {
    "kind": "person",
    "mastery": [
      "Currency and futures trend following",
      "Mechanical trading methods"
    ],
    "summary": "Courtney D. Smith is a money manager and former bank trading-desk head who has written on options, seasonals, spreads and currency trading. How to Make a Living Trading Foreign Exchange sets out complete methods (swing-point trend analysis, an enhanced 55/20 channel breakout, the Conqueror adapted from Babcock and Freeburg, a stochastic trend rule and reversal-day trades); these run here on indices, ETFs and stocks as a declared application. This profile rests on the books themselves.",
    "known_for": [
      "How to Make a Living Trading Foreign Exchange",
      "Option Strategies",
      "Seasonal Charts for Futures Traders"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "csmlt": {"type":"book","people":["Courtney D. Smith","Courtney Smith"]},
    "csops": {"type":"book","people":["Courtney D. Smith","Courtney Smith"]},
    "csscf": {"type":"book","people":["Courtney D. Smith","Courtney Smith"]},
  },
} satisfies AuthorFile;
