import type {AuthorFile} from '../author-files';

export default {
  order: 1420,
  author: { id: 'sysmisc', name: "Sweeney, Warren, Goslin, Heitkoetter, Ford & Duffy", tagline: "Trend campaigns, alpha-beta channels and band fades", books: [{ id: 'jswct', title: "Campaign Trading (Sweeney)", year: 1996, blurb: "Sweeney, 1996 · two-average trend rule; add-ons rest on crude-oil stop levels" }, { id: 'awab', title: "Alpha-beta trend following (Warren, S&C)", year: null, blurb: "Warren, 1985–89 · his dated signals match on SPX" }, { id: 'cgtdd', title: "Trading Day by Day (Goslin)", year: 2002, blurb: "Goslin, 2002 · three-point Status from his shipped code; overrides are judgement" }, { id: 'hkmmts', title: "How to Make Money with Trading Systems (Heitkoetter)", year: 2005, blurb: "Heitkoetter, 2005 · Bollinger fade; does not reproduce the e-mini results on SPX" }, { id: 'gfstsb', title: "Systems Trading for Spread Betting (Ford)", year: 2008, blurb: "Ford, 2008 · every system needs hourly forex bars" }, { id: 'jtdpls', title: "Pulsar 2.0 (Duffy)", year: null, blurb: "Duffy · stop and target units never defined, not testable" }, { id: 'cgift', title: "Intelligent Futures Trading (Goslin)", year: null, blurb: "Goslin · guidelines; the mechanical form is cgtdd Status" }, { id: 'hkcgdt', title: "The Complete Guide to Day Trading (Heitkoetter)", year: null, blurb: "Heitkoetter · entry signals without exits" }] },
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
