import type {AuthorFile} from '../author-files';

export default {
  order: 510,
  author: { id: 'evidence', name: "Aronson, Acar & other researchers", tagline: "Academic rule tests and short-term stock systems", books: [{ id: 'arebta', title: "Evidence-Based Technical Analysis (Aronson)", year: 2007, blurb: "Aronson, 2007 · trend and extreme-value rules on the index’s own close; the rest need breadth or yield series" }, { id: 'acatr', title: "Advanced Trading Rules (Acar & Satchell, eds.)", year: 2002, blurb: "Acar & Satchell, 2002 · Kuo MA rule, Gasser breakout and the Chang–Osler filter rule" }, { id: 'dgstt', title: "Short-Term Stock Trading (Daragan)", year: null, blurb: "Daragan · ranks about 1,000 US stocks daily, needs that universe" }, { id: 'csadx', title: "ADXcellence (Schaap)", year: 2006, blurb: "Schaap, 2006 · setups need trendlines and have no exits" }, { id: 'bllstr', title: "Simple Technical Trading Rules (Brock, Lakonishok & LeBaron)", year: 1992, blurb: "BLL, 1992 · all 26 moving-average and range-breakout rules" }, { id: 'stwds', title: "Data-Snooping and Technical Trading Rules (Sullivan, Timmermann & White)", year: null, blurb: "STW · the paper’s best rules; its 12%/10% filter trades match on the Dow" }, { id: 'cjlmom', title: "Momentum Strategies (Chan, Jegadeesh & Lakonishok)", year: 1996, blurb: "CJL, 1996 · ranks the whole US stock market, needs that universe" }, { id: 'kdta', title: "Technical Analysis (Kirkpatrick & Dahlquist)", year: 2006, blurb: "K&D, 2006 · 25 breadth, Arms, average and seasonal systems" }, { id: 'ckbtm', title: "Beat the Market (Kirkpatrick)", year: 2008, blurb: "Kirkpatrick, 2008 · stock relative strength needs the US universe" }, { id: 'vsbrw', title: "Beyond the Random Walk (Singal)", year: null, blurb: "Singal · December effect matches 12 of 12 NDX years" }, { id: 'jtwl', title: "Returns to Buying Winners and Selling Losers (Jegadeesh & Titman)", year: 1993, blurb: "J&T, 1993 · needs the US stock universe" }, { id: 'sbcft', title: "Commodity Futures: Trends or Random Walks (Stevenson & Bear)", year: 1970, blurb: "Stevenson & Bear, 1970 · futures contracts only" }, { id: 'rbhetf', title: "ETF Money Flow System (Hanna)", year: 2005, blurb: "Hanna, 2005 · needs fund-flow data" }, { id: 'gh52w', title: "52-Week High and Momentum (George & Hwang)", year: 2004, blurb: "George & Hwang, 2004 · ranks the US stock universe" }, { id: 'cghms', title: "Market States and Momentum (Cooper, Gutierrez & Hameed)", year: 2004, blurb: "CGH, 2004 · decile stock portfolios" }, { id: 'lswtv', title: "Price Momentum and Trading Volume (Lee & Swaminathan)", year: 2000, blurb: "L&S, 2000 · needs stock turnover" }, { id: 'cbsiws', title: "Invest with Success (Schaap)", year: 2008, blurb: "Schaap, 2008 · the 50-50 Strategy (weekly 50 EMA and RSI 20, High Riser entry, stop $0.25 under the average) on US instruments" }] },
  identities: [],
  profile: {
    "kind": "collection",
    "mastery": [
      "Quantitative research",
      "System design & testing",
      "Momentum"
    ],
    "summary": "Academic and statistical testing of technical rules. David Aronson, a former prop trader and Baruch College adjunct, argues for applying the scientific method and statistical inference to trading signals. The set also includes classic papers on moving-average rules, data-snooping bias and momentum returns, and Kirkpatrick & Dahlquist's technical analysis textbook.",
    "known_for": [
      "Evidence-Based Technical Analysis (Aronson)",
      "Simple Technical Trading Rules (Brock, Lakonishok & LeBaron)",
      "Data-Snooping and Technical Trading Rules (Sullivan, Timmermann & White)",
      "Returns to Buying Winners and Selling Losers (Jegadeesh & Titman)"
    ],
    "sources": [
      "https://www.oreilly.com/library/view/evidence-based-technical-analysis/9780470008744/xhtml/AbouttheAuthor.html"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "arebta": {"type":"book","people":["David Aronson"]},
    "acatr": {"type":"book","people":["Emmanuel Acar","Stephen Satchell"]},
    "dgstt": {"type":"book","people":["Vladimir Daragan"]},
    "csadx": {"type":"book","people":["Charles B. Schaap"]},
    "bllstr": {"type":"article","people":["Blake LeBaron","Josef Lakonishok","William Brock"]},
    "stwds": {"type":"article","people":["Allan Timmermann","Halbert White","Ryan Sullivan"]},
    "cjlmom": {"type":"article","people":["Josef Lakonishok","Louis K. C. Chan","Narasimhan Jegadeesh"]},
    "kdta": {"type":"book","people":["Charles D. Kirkpatrick","Julie R. Dahlquist"]},
    "ckbtm": {"type":"book","people":["Charles D. Kirkpatrick"]},
    "vsbrw": {"type":"book","people":["Vijay Singal"]},
    "jtwl": {"type":"article","people":["Narasimhan Jegadeesh","Sheridan Titman"]},
    "sbcft": {"type":"article","people":["Richard A. Stevenson","Robert M. Bear"]},
    "rbhetf": {"type":"book","people":["Robert B. Hanna"]},
    "gh52w": {"type":"article","people":["Chuan-Yang Hwang","Thomas J. George"]},
    "cghms": {"type":"article","people":["Allaudeen Hameed","Michael J. Cooper","Roberto C. Gutierrez Jr."]},
    "lswtv": {"type":"article","people":["Bhaskaran Swaminathan","Charles M. C. Lee"]},
    "cbsiws": {"type":"book","people":["Charles B. Schaap"]},
  },
} satisfies AuthorFile;
