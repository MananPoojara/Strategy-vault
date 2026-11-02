import type {AuthorFile} from '../author-files';

export default {
  order: 1516,
  author: { id: 'tsvend', name: "Vendor and list system code (batch J3)", tagline: "TradeStation and MetaStock systems read from vendor archives and list posts", books: [{ id: 'tsconf', title: "Confluence system (Legan / Fritz, Omega list 1998)", year: 1998, blurb: "Legan's Confluence bull/bear mode stops · 5-point trigger held" }, { id: 'tszlb', title: "Smoothed %b system (ZeroLag.txt code collection)", year: null, blurb: "Two-pole smoothed %b(5) fade above / below a 200 smoother" }, { id: 'tswtc', title: "William Tell Coffee Strategy (Market Analytics)", year: null, blurb: "Fourteen day-of-week patterns for coffee, exit at the first profitable open · no stop" }, { id: 'tsbull', title: "Bullseye Futures DMI signal", year: null, blurb: "+DI / -DI with a 2-bar XAverage, buy stop 35 ticks over the close, sell at market (stop and reverse)" }, { id: 'tsltc', title: "LTC Swing (accuratetradingsystems.com)", year: 2006, blurb: "ECO stays across zero for 5 bars -> stop and reverse at the close" }, { id: 'tsprosc', title: "ProScan S&P Scalping System (Jan Arps)", year: null, blurb: "SlowD(14) swing divergence-style reversals at the close" }, { id: 'tstango', title: "Tango System (dancewiththemarkets.com)", year: null, blurb: "ADX-armed pullbacks to the 20 XMA with stop entries, 3-bar exit stops and re-entries" }, { id: 'tsgear', title: "SqueezePlay MACD system (impressivetrading.com)", year: null, blurb: "MACD(8, 66) histogram cross, limit entries, stop and reverse · point and dollar exits held" }, { id: 'tsbond', title: "Bond System for TS (ADX/DMI + RSI(4) pullback)", year: null, blurb: "Trend pullback buy stop with 30-bar / 2-bar low exits · money stop held" }] },
  identities: [],
  profile: {
    "kind": "collection",
    "mastery": ["System design & testing", "TradeStation code"],
    "summary": "TradeStation EasyLanguage and MetaStock system code found in vendor archives and mailing-list posts in the research share. The printed code is the rule; fixed point and dollar amounts are held, and the systems run on daily index, ETF and stock bars as declared applications.",
    "known_for": ["Confluence system"],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "tsconf": {"type":"code_package","people":["Dale Legan", "Gary Fritz"]},
    "tszlb": {"type":"code_package","people":[]},
    "tswtc": {"type":"code_package","people":[]},
    "tsbull": {"type":"code_package","people":[]},
    "tsltc": {"type":"code_package","people":[]},
    "tsprosc": {"type":"code_package","people":["Jan Arps"]},
    "tstango": {"type":"code_package","people":[]},
    "tsgear": {"type":"code_package","people":[]},
    "tsbond": {"type":"code_package","people":[]},
  },
} satisfies AuthorFile;
