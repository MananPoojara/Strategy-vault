import type {AuthorFile} from '../author-files';

export default {
  order: 1440,
  author: { id: 'vendors', name: "Arnold & Elm Trader", tagline: "Timing newsletters and calendar index systems", books: [{ id: 'capttm', title: "Timing the Market (Curtis Arnold)", year: null, blurb: "Arnold · 14/50 crossover and price-vs-both-averages; sentiment series need data" }, { id: 'elmts4', title: "Dow TS4 (Elm Trader)", year: 2005, blurb: "Elm Trader · month-turn Dow trade; 49 of 49 trades in the same months as the book" }, { id: 'mjthir', title: "Thirteen system (Mark Johnson)", year: null, blurb: "Johnson · MACD(13,130) system from thirteen.go" }, { id: 'pwabal', title: "Abalone (Peter Waite)", year: null, blurb: "Waite · from the manual’s TradeStation code" }, { id: 'pwpeg', title: "Pegasus (Peter Waite)", year: null, blurb: "Waite · open TradeStation code: Abalone's rising averages plus a 100-day 1-SD band, three exits; the $3,000 risk cap is held" }, { id: 'jwbent', title: "Bent Nail (James Wolf)", year: null, blurb: "Wolf · stop-and-reverse at the close" }, { id: 'fcfail', title: "TL-Failure Swing (Corrias)", year: null, blurb: "Corrias · long and short, short exit as printed" }, { id: 'kfaber', title: "Aberration (third-party code)", year: null, blurb: "Third-party TradeStation copy, labelled; Fitschen’s original is not on the drive" }, { id: 'ubvma', title: "Uncle Bill timing system", year: null, blurb: "Part of the rule is visual and lengths are printed as ??" }, { id: 'gcplad', title: "PLAD (Greg Cook)", year: null, blurb: "Rules only in a compressed binary file" }, { id: 'ltan1', title: "Anomaly I (longtermtrading.com)", year: null, blurb: "Futures portfolio with dollar stops, needs futures data" }, { id: 'ltrsg', title: "Ready-Set-Go (longtermtrading.com)", year: null, blurb: "Futures portfolio with dollar stops, needs futures data" }, { id: 'cpgap', title: "Gap Fill and Gap Continue (Capstone)", year: null, blurb: "Needs 1-minute E-mini bars" }, { id: 'ltan2', title: "Anomaly 2 (longtermtrading.com)", year: null, blurb: "Futures portfolio with dollar stops, needs futures data" }] },
  identities: [],
  profile: {
    "kind": "collection",
    "mastery": [
      "System design & testing",
      "Market timing",
      "Pattern recognition"
    ],
    "summary": "Commercial vendor trading systems and Curtis Arnold's Timing the Market. It includes named black-box or semi-published systems such as Dow TS4, Thirteen, Abalone, Bent Nail, Aberration, PLAD and gap-fill/continue rules. Rules come from vendor code or manuals and vary in completeness.",
    "known_for": [
      "Timing the Market (Curtis Arnold)",
      "Aberration",
      "Gap Fill and Gap Continue (Capstone)"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "capttm": {"type":"book","people":["Curtis M. Arnold"]},
    "elmts4": {"type":"code_package","people":[]},
    "mjthir": {"type":"code_package","people":["Mark Johnson"]},
    "pwabal": {"type":"code_package","people":["Peter B. Waite"]},
    "pwpeg": {"type":"code_package","people":["Peter B. Waite"]},
    "jwbent": {"type":"code_package","people":["James Wolf"]},
    "fcfail": {"type":"code_package","people":[]},
    "kfaber": {"type":"code_package","people":[]},
    "ubvma": {"type":"code_package","people":[]},
    "gcplad": {"type":"code_package","people":["Greg Cook"]},
    "ltan1": {"type":"code_package","people":["Alan Pryor"]},
    "ltrsg": {"type":"code_package","people":["Alan Pryor"]},
    "cpgap": {"type":"book","people":[]},
    "ltan2": {"type":"code_package","people":["Alan Pryor"]},
  },
} satisfies AuthorFile;
