import type {AuthorFile} from '../author-files';

export default {
  order: 1500,
  author: { id: 'v6misc', name: "Small vendor and research systems", tagline: "24-7, RSI 2.3, envelope study and others", books: [{ id: 'ts247', title: "24-7 System", year: null, blurb: "Long from the 25th to the 6th with a 0.5% stop" }, { id: 'fsi23', title: "RSI 2.3 Strategy", year: null, blurb: "RSI(4) limit-entry system, code default" }, { id: 'mjlbb', title: "Envelopes vs Bollinger Bands (Leung & Chong)", year: null, blurb: "DJIA 1985–2000 · N=10 and 20 reproduce the paper’s transactions" }, { id: 'sixpk', title: "The Six-Pack Trading System", year: null, blurb: "Confirmation rule never defined, not testable" }, { id: 'bwtm', title: "Black or White Trading Model", year: null, blurb: "Exit-or-reverse never stated" }, { id: 'fbs50', title: "50 Bar Trading Strategy", year: null, blurb: "Only part of a strategy, no fill" }, { id: 'mfcat', title: "Catscan (Mindfire)", year: null, blurb: "Dollar stop with no point value, needs data" }, { id: 'ltwr', title: "WaveRider (longtermtrading.com)", year: null, blurb: "Futures portfolio with dollar stops" }, { id: 'tvax', title: "AXIOM Index (TradingVisions)", year: null, blurb: "Encrypted code, contradictory text" }, { id: 'pdmb', title: "Identifying Market Bottoms (Desmond)", year: 2002, blurb: "No entry order and no exit" }, { id: 'msftp', title: "Surefire Trading Plans (McRae)", year: null, blurb: "Discretionary by the author’s own words" }, { id: 'sl2040', title: "Safelock 20-40", year: null, blurb: "Intraday ES grid, needs tick bars" }, { id: 'mbbb2', title: "Big Blue-2 (Barna)", year: null, blurb: "Undisclosed offsets, encrypted code" }, { id: 'msfft', title: "Sure-Fire Forex Trading (McRae)", year: null, blurb: "Intraday forex: eyeballed Fibonacci points and oscillator extremes, not testable" }, { id: 'dnrsi', title: "Dennis RSI Trading System (stock index futures)", year: null, blurb: "Dennis \u00b7 5-day RSI of the simulated intraday path, long at 40 or less, short at 60 or more; the 600-point stop and target are held" }, { id: 'ms7ru', title: "MetaStock 7.0 course (Russian, anonymous)", year: null, blurb: "Six System Tester exercises (21-day average, 3 EMAs, 39-day average, momentum and %R waves, RSI 30/70) as teaching examples" }, { id: 'cvan2', title: "TradeSim AN-2 Volatility Trailing Stops (Compuvision)", year: 2004, blurb: "Vendor sample \u00b7 MACD crossing its signal line, out on a 3 x ATR(10) trailing band" }, { id: 'msitg', title: "Guida all'uso di MetaStock (Italian user guide)", year: null, blurb: "TrendRSI expert example \u00b7 buy at RSI(14) below 30, sell above 70" }] },
  identities: [],
  profile: {
    "kind": "collection",
    "mastery": [
      "System design & testing",
      "Mean reversion",
      "Market timing"
    ],
    "summary": "Small vendor and research systems. They range from an RSI(2)-style strategy and an envelope versus Bollinger Band comparison to market-bottom identification and named timing models such as Catscan, WaveRider and the AXIOM Index.",
    "known_for": [
      "RSI 2.3 Strategy",
      "Envelopes vs Bollinger Bands (Leung & Chong)",
      "Identifying Market Bottoms (Desmond)"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "ts247": {"type":"other","people":[]},
    "fsi23": {"type":"code_package","people":["L. K. Daniels"]},
    "mjlbb": {"type":"article","people":["Joseph Man-Joe Leung","Terence Tai-Leung Chong"]},
    "sixpk": {"type":"other","people":[]},
    "bwtm": {"type":"other","people":[]},
    "fbs50": {"type":"other","people":[]},
    "mfcat": {"type":"code_package","people":[]},
    "ltwr": {"type":"code_package","people":[]},
    "tvax": {"type":"code_package","people":[]},
    "pdmb": {"type":"article","people":["Paul F. Desmond"]},
    "msftp": {"type":"book","people":["Mark McRae"]},
    "sl2040": {"type":"code_package","people":[]},
    "mbbb2": {"type":"code_package","people":["Mike Barna"]},
    "msfft": {"type":"book","people":["Mark McRae"]},
    "dnrsi": {"type":"course_manual","people":[]},
    "ms7ru": {"type":"course_manual","people":[]},
    "cvan2": {"type":"other","people":[]},
    "msitg": {"type":"course_manual","people":[]},
  },
} satisfies AuthorFile;
