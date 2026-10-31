import type {AuthorFile} from '../author-files';

export default {
  order: 1470,
  author: { id: 'swing12', name: "Chong, Pierce, Link & others", tagline: "Turtle variant, NR4 swing and breakout systems", books: [{ id: 'rcmm', title: "Market Millions (Ray Chong)", year: null, blurb: "Chong · 20- and 55-day Turtle variant; pyramid table reproduces exactly" }, { id: 'sprfs', title: "Rapid Fire Swing Trading (Pierce)", year: null, blurb: "Pierce · NR4/inside-bar setups; 8 of his 12 printed setups also occur, all with the same result" }, { id: 'mlhpt', title: "High Probability Trading (Marcel Link)", year: 2003, blurb: "Link, 2003 · 10-bar breakout and his sample system" }, { id: 'mapsl', title: "The Strategy Lab (Perry)", year: null, blurb: "Exits only at an external link" }, { id: 'kwcci', title: "Woodies CCI 1.3", year: null, blurb: "Proprietary indicators and a menu of exits" }, { id: 'stgdg', title: "Definitive Guide to Swing Trading Stocks", year: null, blurb: "Hand-drawn trend lines and no stop size" }, { id: 'jrfsp', title: "FirstStrike Plus (Risnick)", year: null, blurb: "Open-range entry, same-day order decided by the bar path" }, { id: 'bpqst', title: "Quantum Swing Trader (Poulos)", year: null, blurb: "Up- and downtrend setups from the book’s own search criteria" }, { id: 'wmmts', title: "Market Turn Strategy (McWilliams)", year: 2005, blurb: "Formula recovered from his spreadsheet; reproduces his DIA example exactly" }, { id: 'phbttk', title: "Big Trends Toolkit 2.0 (Headley)", year: null, blurb: "Percent R long and short" }, { id: 'vdso', title: "Supreme Order (Dasilva)", year: null, blurb: "Its own rules need intraday bars" }, { id: 'mjmcr', title: "Monte Carlo Resampling of Equity Curves (Mark Johnson)", year: 2005, blurb: "50/300 crossover and 300-day breakout" }, { id: 'hwdti', title: "Dynamic Trading Indicators (Helweg & Stendahl)", year: 2002, blurb: "20 WMA / 56 SMA reversal" }, { id: 'mhnats', title: "The Alchemy of Trading Systems (Hanson)", year: null, blurb: "Needs S&P futures open interest" }, { id: 'mltwg', title: "Trading Without Gambling (Marcel Link)", year: 2003, blurb: "Link, 2003 · a trading-plan book; its moving-average crossover example has no lengths or exit" }] },
  identities: [],
  profile: {
    "kind": "collection",
    "mastery": [
      "Short-term swing",
      "Pattern recognition",
      "Indicators"
    ],
    "summary": "Swing-trading books and course methods. Marcel Link's High Probability Trading sits alongside vendor swing courses and toolkits such as Woodies CCI, FirstStrike Plus, Quantum Swing Trader and Price Headley's Big Trends Toolkit.",
    "known_for": [
      "High Probability Trading (Link)",
      "Market Millions (Chong)",
      "Woodies CCI"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "rcmm": {"type":"book","people":["Raymond Chong"]},
    "sprfs": {"type":"book","people":["Stephen A. Pierce"]},
    "mlhpt": {"type":"book","people":["Marcel Link"]},
    "mapsl": {"type":"other","people":["M. A. Perry"]},
    "kwcci": {"type":"other","people":[]},
    "stgdg": {"type":"book","people":["Kevin Brown"]},
    "jrfsp": {"type":"other","people":["Joel Rensink"]},
    "bpqst": {"type":"course_manual","people":["Bill Poulos","Greg Poulos"]},
    "wmmts": {"type":"book","people":["William McWilliams"]},
    "phbttk": {"type":"code_package","people":["Price Headley"]},
    "vdso": {"type":"book","people":["Virgil Dasilva"]},
    "mjmcr": {"type":"course_manual","people":["Mark Johnson"]},
    "hwdti": {"type":"book","people":["David Stendahl","Mark W. Helweg"]},
    "mhnats": {"type":"book","people":["Matthew Hanson"]},
    "mltwg": {"type":"book","people":["Marcel Link"]},
  },
} satisfies AuthorFile;
