import type {AuthorFile} from '../author-files';

export default {
  order: 1490,
  author: { id: 't2wgrp', name: "Trade-to-Win & vendor code", tagline: "RSI(2) swing systems and systems rebuilt from vendor code", books: [{ id: 't2w', title: "Trade-to-Win systems (MRX, R90, Crouching Tiger, Oops, STE, RVT)", year: null, blurb: "MRX: 386 trades, 80.05% vs the manual’s 391, 80.05%; Crouching Tiger 235 vs 232" }, { id: 'rarsys', title: "Systems from vendor code archives", year: null, blurb: "Trend Weaver, Crouching Tiger and STE read from the vendors’ own TradeStation code" }, { id: 'sctw', title: "Trend Weaver (Shay Campbell)", year: null, blurb: "Campbell · 150-day trend with 20-day range stops" }, { id: 'msws', title: "Wallstreet Trade Systems & Indicators (Greg Morris, MetaStock)", year: null, blurb: "96 complete MetaStock formula systems, run exactly as printed" }, { id: 'mstt', title: "Trading Tech. Systems & Indicators (MetaStock)", year: 2001, blurb: "94 systems from Traders’ Tips reprints and Peter’s web pages" }, { id: 'tilma', title: "Better Moving Averages and Oscillators (Tillson)", year: null, blurb: "T3, DEMA, EMA, SMA, ILRS: same ranking as the book on NDX 1993–97" }, { id: 'bcsts', title: "Aulas Swing Trading System (Castelo)", year: null, blurb: "Three MetaStock systems, no published results" }, { id: 'mslib', title: "Libanes MetaStock strategies", year: null, blurb: "21 Spanish MetaStock systems run as printed; results are on Spanish stocks" }, { id: 'mscvl', title: "Civila formulas and systems", year: null, blurb: "Repeats the Guppy formula index; two need breadth or bond data" }, { id: 'mssta', title: "Santabolsa collection", year: null, blurb: "Repeats Libanes and earlier collections" }, { id: 'msadd', title: "MetaStock add-on pack (Equis)", year: null, blurb: "38 Equis system tests run as stored; optimisation-variable systems are not testable" }] },
  identities: [],
  profile: {
    "kind": "collection",
    "mastery": [
      "System design & testing",
      "Indicators",
      "Short-term swing"
    ],
    "summary": "Vendor and community code archives. These include Trade-to-Win systems (MRX, R90, Crouching Tiger, Oops), MetaStock system and indicator collections such as Greg Morris's, Tillson's improved moving averages and oscillators, the Connors Group Chairman's Club, and Brazilian and Spanish MetaStock formula sets.",
    "known_for": [
      "Trade-to-Win systems",
      "Better Moving Averages and Oscillators (Tillson)",
      "MetaStock system collections"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "t2w": {"type":"code_package","people":[]},
    "rarsys": {"type":"code_package","people":["Mark Johnson","Shay Campbell"]},
    "sctw": {"type":"code_package","people":["Shay Campbell"]},
    "msws": {"type":"code_package","people":["Gregory L. Morris"]},
    "mstt": {"type":"code_package","people":[]},
    "tilma": {"type":"article","people":["Tim Tillson"]},
    "bcsts": {"type":"course_manual","people":["Bulli Castelo"]},
    "mslib": {"type":"code_package","people":[]},
    "mscvl": {"type":"code_package","people":[]},
    "mssta": {"type":"code_package","people":[]},
    "msadd": {"type":"code_package","people":[]},
  },
} satisfies AuthorFile;
