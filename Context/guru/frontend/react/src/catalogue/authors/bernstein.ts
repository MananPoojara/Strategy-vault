import type {AuthorFile} from '../author-files';

export default {
  order: 180,
  author: { id: 'bernstein', name: 'Jake Bernstein', tagline: 'Seasonals, moving-average channels and short-term momentum', books: [{ id: 'jb624', title: '6-2-4 Winning Strategies & Systems', year: 2006, blurb: 'Bernstein · MAC and momentum systems' }, { id: 'jbudt', title: 'The Ultimate Day Trader', year: 2009, blurb: 'Bernstein, 2009 · daily-bar day-trade rules; intraday ones need data' }, { id: 'jb30d', title: '30 Days to Market Mastery', year: 2007, blurb: 'Bernstein, 2007' }, { id: 'jbsmsw', title: 'Stock Market Strategies That Work', year: 2002, blurb: 'Bernstein · S&P key-date seasonals' }, { id: 'jbssf', title: 'How to Trade the New Single Stock Futures', year: 2002, blurb: 'Bernstein, 2002' }, { id: 'jbcdt', title: 'The Compleat Day Trader', year: 1995, blurb: 'Bernstein · most rules need intraday data' }, { id: 'jbcdt2', title: "The Compleat Day Trader II", year: null, blurb: "Bernstein · day-of-week breakouts and S&P date trades; intraday rules need minute bars" }, { id: 'jbeft', title: "Strategies for the Electronic Futures Trader", year: null, blurb: "Bernstein · Friday–Monday and Wednesday–Thursday trades" }, { id: 'jbhsms', title: "Hot Stock Market Strategies", year: null, blurb: "Bernstein · S&P, Dow and US-stock key-date seasonals plus the IBM/EBAY gap trade" }, { id: 'jbcaf', title: "Cyclic Analysis in Futures Trading", year: null, blurb: "Bernstein · 25/75 slow stochastic and a two-close 10/8 channel" }, { id: 'jbtywr', title: "Trade Your Way to Riches Now! (seminar)", year: 1996, blurb: "Bernstein seminar binder · Dow key-date seasonals and a crude-oil outside-day pattern" }, { id: 'jbtifm', title: "Trading the International Futures Markets", year: 2000, blurb: "Bernstein · 2000ST range breakout and a 9/18 moving-average crossover from the printed TradeStation code" }, { id: 'jbhpp', title: "High Probability Patterns and Rule Based Trading (seminar)", year: 2010, blurb: "Bernstein seminar slides · a Dow key-date seasonal; its timing triggers are coded from his other books" }] },
  identities: ["Jake Bernstein"],
  profile: {
    "kind": "person",
    "mastery": [
      "Seasonality",
      "Futures",
      "Cycles"
    ],
    "summary": "Jake Bernstein is a futures market analyst and newsletter publisher, trained in psychology, who has written dozens of books on futures and stock trading. He is known mainly for seasonal and cyclic futures analysis and short-term day-trading methods.",
    "known_for": [
      "Seasonal futures trading",
      "The Compleat Day Trader",
      "Cyclic Analysis in Futures Trading"
    ],
    "sources": [
      "https://en.wikipedia.org/wiki/Jake_Bernstein_(market_analyst)"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "jb624": {"type":"book","people":["Jake Bernstein"]},
    "jbudt": {"type":"book","people":["Jake Bernstein"]},
    "jb30d": {"type":"book","people":["Jake Bernstein"]},
    "jbsmsw": {"type":"book","people":["Elliott Bernstein","Jake Bernstein"]},
    "jbssf": {"type":"book","people":["Jake Bernstein"]},
    "jbcdt": {"type":"book","people":["Jake Bernstein"]},
    "jbcdt2": {"type":"book","people":["Jake Bernstein"]},
    "jbeft": {"type":"book","people":["Jake Bernstein"]},
    "jbhsms": {"type":"book","people":["Jake Bernstein"]},
    "jbcaf": {"type":"book","people":["Jake Bernstein"]},
    "jbtywr": {"type":"course_manual","people":["Jake Bernstein"]},
    "jbtifm": {"type":"book","people":["Jake Bernstein"]},
    "jbhpp": {"type":"course_manual","people":["Jake Bernstein"]},
  },
} satisfies AuthorFile;
