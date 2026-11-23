import type {AuthorFile} from '../author-files';

export default {
  order: 1420,
  author: { id: 'sysmisc', name: "Sweeney, Warren, Goslin, Heitkoetter, Ford & Duffy", tagline: "Trend campaigns, alpha-beta channels and band fades", books: [ { id: 'tmem', title: "The 3-Minute E-Mini Trading System (12th ed.)", year: 2007, blurb: "3-minute-e-mini.com, 2007 · five daily patterns (Bounce, Escalator, Opening, Wider Range, Modified Escalator); run on NIFTY / BANKNIFTY futures daily bars, readings declared, unverified" }, { id: 'jani', title: "Simulating Trading Portfolios (Jani, TASC 1994)", year: 1994, blurb: "Jani, 1994 \u00b7 5-day MA crossing the 25-day MA on the open, 50-day MA as the stop; the article tests six futures, runs here on the NSE indices as a declared adaptation, unverified" }, { id: 'fxcm', title: "FXCM GBP/USD Strategy Workshop (SF 1)", year: null, blurb: "FXCM \u00b7 15,5,5 slow stochastic cross with a 200-day SMA filter, exit on the cross back; the 2-hour GBP/USD original runs here on daily NSE index bars, readings declared, unverified" }, { id: 'kmifx', title: "Instant Forex Profits (Kishore M.)", year: null, blurb: "Kishore, Powerup Capital \u00b7 Bollinger-band touch, confirming candle, entry at the open of the third candle, exit at the opposite band; the forex original runs here on daily NSE index bars with a declared pip-to-percent rule, unverified" }, { id: 'kzsea', title: "Seasonality and Trading (Katz, TASC 1997)", year: 1997, blurb: "Katz, 1997 \u00b7 date-specific open-to-close model: past same-date up-percentage and mean change decide a long or short from the open to the close; the S&P original runs here on the NSE indices with a declared threshold scaling, unverified" }, { id: 'rwsl', title: "The Slingshot Trading Method (Watts)", year: null, blurb: "Watts · 5- and 7-day momentum against low-momentum cross, daily bars, always in the market; run on the NSE indices, readings declared, unverified" }, { id: 'pvbs', title: "Secrets of a Pivot Boss (Ochoa)", year: 2012, blurb: "Ochoa, 2012 · Camarilla H4 / L4 breakout on 5-minute bars; run on NIFTY / BANKNIFTY futures, readings declared, unverified; the H3 / L3 reversals are not coded" },{ id: 'lbspx', title: "System for SPX 60 mins (Labunsky)", year: null, blurb: "Labunsky \u00b7 fitted C-O average comparisons on 60-minute bars, stop and reverse; run on SPX/SPY minute bars, readings declared, unverified" }, { id: 'jwtri', title: "The Range Indicator (Weinberg, TASC 1995)", year: 1995, blurb: "Weinberg, 1995 \u00b7 EMA 3/10/30 trend system with the Range Indicator as entry and exit filter; article tests currency futures, runs here as a declared adaptation, unverified" }, { id: 'thvl', title: "Value Line 30-week momentum (Hayes)", year: null, blurb: "Hayes \u00b7 30-week momentum of the Value Line index as a market-timing slope rule; index-only, declared readings, unverified" }, { id: 'dwshark', title: "Shark-32 (Downs, TASC 1998)", year: 1998, blurb: "Downs, 1998 · three-bar shark-fin pattern, buy the close above its high, printed Sell as the long exit; four declared readings, unverified" }, { id: 'fmess', title: "The Essentials of Trading (Forman)", year: 2006, blurb: "Forman, 2006 · Parabolic SAR flip enters, 60-day average cross exits; book tests forex, runs here as a declared adaptation, unverified" }, { id: 'dpvdd', title: "Price & Volume, Digging Deeper (Dormeier)", year: null, blurb: "Dormeier white paper · VPCI cross-over with ADX and MACD on SPY; readings declared, unverified" }, { id: 'mrcsea', title: "Researching Your Trade (Moore Research Center / Raschke)", year: 1993, blurb: "Moore Research Center September S&P 500 seasonal short (TAG 1993 handout); declared adaptation on the cash index / ETF, unverified" }, { id: 'volexp', title: "Volume Expansion (Anand Rathi)", year: null, blurb: "Anand Rathi deck · close beyond the high-volume candle, stop at its other end, target its range; readings declared, unverified" }, { id: 'jswct', title: "Campaign Trading (Sweeney)", year: 1996, blurb: "Sweeney, 1996 · two-average trend rule; add-ons rest on crude-oil stop levels" }, { id: 'awab', title: "Alpha-beta trend following (Warren, S&C)", year: null, blurb: "Warren, 1985–89 · his dated signals match on SPX" }, { id: 'cgtdd', title: "Trading Day by Day (Goslin)", year: 2002, blurb: "Goslin, 2002 · three-point Status from his shipped code; overrides are judgement" }, { id: 'hkmmts', title: "How to Make Money with Trading Systems (Heitkoetter)", year: 2005, blurb: "Heitkoetter, 2005 · Bollinger fade; does not reproduce the e-mini results on SPX" }, { id: 'gfstsb', title: "Systems Trading for Spread Betting (Ford)", year: 2008, blurb: "Ford, 2008 · three 60-minute systems on real minute bars; the 6-hour ones need forex bars" }, { id: 'jtdpls', title: "Pulsar 2.0 (Duffy)", year: null, blurb: "Duffy · stop and target units never defined, not testable" }, { id: 'cgift', title: "Intelligent Futures Trading (Goslin)", year: null, blurb: "Goslin · guidelines; the mechanical form is cgtdd Status" }, { id: 'hkcgdt', title: "The Complete Guide to Day Trading (Heitkoetter)", year: null, blurb: "Heitkoetter · entry signals without exits" }] },
  identities: [],
  profile: {
    "kind": "collection",
    "mastery": [
      "System design & testing",
      "Trend following",
      "Day trading"
    ],
    "summary": "Miscellaneous system-trading guides. The titles cover campaign trading, alpha-beta trend following, day-by-day and futures system trading, building and buying trading systems, spread-betting systems and day trading. These authors are mostly practitioners and educators, not academics.",
    "known_for": [
      "Campaign Trading (Sweeney)",
      "How to Make Money with Trading Systems (Heitkoetter)",
      "Systems Trading for Spread Betting (Ford)"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "tmem": {"type":"course_manual","people":[]},
    "rwsl": {"type":"course_manual","people":["Ryan Watts"]},
    "kzsea": {"type":"article","people":["Thomas Katz"]},
    "kmifx": {"type":"course_manual","people":["Kishore M."]},
    "jani": {"type":"article","people":["Jani"]},
    "fxcm": {"type":"course_manual","people":[]},
    "pvbs": {"type":"book","people":["Franklin Ochoa"]},
    "jwtri": {"type":"article","people":["Jack L. Weinberg"]},
    "thvl": {"type":"article","people":["Tim Hayes"]},
    "dwshark": {"type":"article","people":["Walter T. Downs"]},
    "fmess": {"type":"book","people":["John Forman"]},
    "lbspx": {"type":"code_package","people":["Labunsky"]},
    "volexp": {"type":"course_manual","people":["Anand Rathi"]},
    "dpvdd": {"type":"article","people":["Buff Pelz Dormeier"]},
    "mrcsea": {"type":"article","people":["Moore Research Center","Linda Bradford Raschke"]},
    "jswct": {"type":"book","people":["John Sweeney"]},
    "awab": {"type":"article","people":["Anthony W. Warren"]},
    "cgtdd": {"type":"book","people":["Chick Goslin"]},
    "hkmmts": {"type":"article","people":["Markus Heitkoetter"]},
    "gfstsb": {"type":"book","people":["Gary Ford"]},
    "jtdpls": {"type":"code_package","people":["Joseph T. Duffy"]},
    "cgift": {"type":"book","people":["Chick Goslin"]},
    "hkcgdt": {"type":"book","people":["Markus Heitkoetter"]},
  },
} satisfies AuthorFile;
