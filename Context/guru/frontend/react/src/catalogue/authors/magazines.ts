import type {AuthorFile} from '../author-files';

export default {
  order: 1480,
  author: { id: 'magazines', name: "Active Trader, Futures & newsletters", tagline: "Trading System Lab columns and magazine systems", books: [{ id: 'atm00', title: "Active Trader 2000 — Trading System Lab", year: 2000, blurb: "Five systems; Gold Digger does not reproduce its printed trade count" }, { id: 'atm01', title: "Active Trader 2001 — Trading System Lab", year: 2001, blurb: "Six systems" }, { id: 'atm02', title: "Active Trader 2002 — Trading System Lab", year: 2002, blurb: "Six systems; Counterpunch futures version reproduces (38% vs 36% winners)" }, { id: 'atm03', title: "Active Trader 2003", year: 2003, blurb: "Nineteen systems including seasonal holiday trades" }, { id: 'atm04', title: "Active Trader 2004", year: 2004, blurb: "QQQ crash system and others" }, { id: 'atm10', title: "Active Trader 2010", year: 2010, blurb: "1-percent QQQ system: trade count reproduces, win rate does not" }, { id: 'atmmisc', title: "Active Trader — Trading System Lab binders", year: null, blurb: "Countertrend HLR and others; Anti-Trend EMA’s printed exit is self-defeating, so not testable" }, { id: 'ftm', title: "Futures Magazine 2001–2005", year: null, blurb: "Fourteen systems; Matoba MACD reproduces its 30.90% max drawdown" }, { id: 'ctcn', title: "Commodity Traders Club News", year: null, blurb: "Four systems" }, { id: 'mrina', title: "Inside Advantage (Ruggiero)", year: null, blurb: "Dip-buyer; OOPS needs futures gaps" }, { id: 'tsx', title: "TS Express", year: null, blurb: "One system" }, { id: 'atmcsa', title: "Active Trader — Currency System Analysis binder", year: null, blurb: "Ten currency-column systems run on the indices" }, { id: 'atmrc', title: "Active Trader — Risk Control binder", year: null, blurb: "Stridsman’s 2002 swing pattern, fixed and trailing stop" }, { id: 'ftm2', title: "Futures Magazine 2001–2005 (further columns)", year: null, blurb: "Collins vote matches trade counts; Santa Claus rally, MACD-R, skewness pulse" }, { id: 'wmny', title: "Working Money 2000–2004", year: null, blurb: "Sixteen systems; Valcu z-score and Brown SMH results reproduce" }, { id: 'sfo', title: "SFO Magazine 2001–2004", year: null, blurb: "Toepke S&P windows (76 W / 14 L vs 78 / 12) and sector seasonals" }, { id: 'stkch', title: "StockCharts articles", year: null, blurb: "Arthur Hill EMA crossovers, 39-period stochastic, gap-down trail" }, { id: 'ataa', title: "ATAA Journal 1990–1998", year: null, blurb: "Coppock buy signals match the article’s five dates exactly" }] },
  identities: [],
  profile: {
    "kind": "collection",
    "mastery": [
      "System design & testing",
      "Futures"
    ],
    "summary": "Systems published in trading magazines and newsletters. It covers Active Trader's Trading System Lab (2000-2010), Futures Magazine (2001-05), Commodity Traders Club News, Murray Ruggiero's Inside Advantage and TS Express. Each item is typically a short, fully specified system tested on stocks, futures or currencies.",
    "known_for": [
      "Active Trader - Trading System Lab",
      "Futures Magazine",
      "Inside Advantage (Ruggiero)"
    ],
    "sources": [],
    "confidence": "medium"
  },
  sourceReview: {
    "atm00": {"type":"magazine_volume","people":[]},
    "atm01": {"type":"magazine_volume","people":[]},
    "atm02": {"type":"magazine_volume","people":[]},
    "atm03": {"type":"magazine_volume","people":[]},
    "atm04": {"type":"magazine_volume","people":[]},
    "atm10": {"type":"magazine_volume","people":[]},
    "atmmisc": {"type":"magazine_volume","people":[]},
    "ftm": {"type":"magazine_volume","people":[]},
    "ctcn": {"type":"magazine_volume","people":[]},
    "mrina": {"type":"magazine_volume","people":["Murray Ruggiero"]},
    "tsx": {"type":"magazine_volume","people":[]},
    "atmcsa": {"type":"magazine_volume","people":[]},
    "atmrc": {"type":"magazine_volume","people":[]},
    "ftm2": {"type":"magazine_volume","sameWorkAs":"ftm","people":[]},
    "wmny": {"type":"magazine_volume","people":[]},
    "sfo": {"type":"magazine_volume","people":[]},
    "stkch": {"type":"other","people":[]},
    "ataa": {"type":"magazine_volume","people":[]},
  },
} satisfies AuthorFile;
