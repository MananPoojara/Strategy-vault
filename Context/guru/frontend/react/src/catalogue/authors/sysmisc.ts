import type {AuthorFile} from '../author-files';

export default {
  order: 1420,
  author: { id: 'sysmisc', name: "Sweeney, Warren, Goslin, Heitkoetter, Ford & Duffy", tagline: "Trend campaigns, alpha-beta channels and band fades", books: [{ id: 'fmess', title: "The Essentials of Trading (Forman)", year: 2006, blurb: "Forman, 2006 · Parabolic SAR flip enters, 60-day average cross exits; book tests forex, runs here as a declared adaptation, unverified" }, { id: 'dpvdd', title: "Price & Volume, Digging Deeper (Dormeier)", year: null, blurb: "Dormeier white paper · VPCI cross-over with ADX and MACD on SPY; readings declared, unverified" }, { id: 'mrcsea', title: "Researching Your Trade (Moore Research Center / Raschke)", year: 1993, blurb: "Moore Research Center September S&P 500 seasonal short (TAG 1993 handout); declared adaptation on the cash index / ETF, unverified" }, { id: 'volexp', title: "Volume Expansion (Anand Rathi)", year: null, blurb: "Anand Rathi deck · close beyond the high-volume candle, stop at its other end, target its range; readings declared, unverified" }, { id: 'jswct', title: "Campaign Trading (Sweeney)", year: 1996, blurb: "Sweeney, 1996 · two-average trend rule; add-ons rest on crude-oil stop levels" }, { id: 'awab', title: "Alpha-beta trend following (Warren, S&C)", year: null, blurb: "Warren, 1985–89 · his dated signals match on SPX" }, { id: 'cgtdd', title: "Trading Day by Day (Goslin)", year: 2002, blurb: "Goslin, 2002 · three-point Status from his shipped code; overrides are judgement" }, { id: 'hkmmts', title: "How to Make Money with Trading Systems (Heitkoetter)", year: 2005, blurb: "Heitkoetter, 2005 · Bollinger fade; does not reproduce the e-mini results on SPX" }, { id: 'gfstsb', title: "Systems Trading for Spread Betting (Ford)", year: 2008, blurb: "Ford, 2008 · three 60-minute systems on real minute bars; the 6-hour ones need forex bars" }, { id: 'jtdpls', title: "Pulsar 2.0 (Duffy)", year: null, blurb: "Duffy · stop and target units never defined, not testable" }, { id: 'cgift', title: "Intelligent Futures Trading (Goslin)", year: null, blurb: "Goslin · guidelines; the mechanical form is cgtdd Status" }, { id: 'hkcgdt', title: "The Complete Guide to Day Trading (Heitkoetter)", year: null, blurb: "Heitkoetter · entry signals without exits" }] },
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
    "fmess": {"type":"book","people":["John Forman"]},
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
