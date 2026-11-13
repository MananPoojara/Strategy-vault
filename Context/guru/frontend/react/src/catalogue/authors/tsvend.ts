import type {AuthorFile} from '../author-files';

export default {
  order: 1516,
  author: { id: 'tsvend', name: "Vendor and list system code (batch J3)", tagline: "TradeStation and MetaStock systems read from vendor archives and list posts", books: [{ id: 'tsconf', readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/_Misc/_196/Dale Legan's Confluence function for TS/conflu.txt"], title: "Confluence system (Legan / Fritz, Omega list 1998)", year: 1998, blurb: "Legan's Confluence bull/bear mode stops · 5-point trigger held" }, { id: 'tszlb', readerPaths: ["USB HDD 2/tRADING KNO-2/George Pruitt, John R.Hill - Building Winning Trading Systems with TradeStation & Code/Code/ZeroLag.txt"], title: "Smoothed %b system (ZeroLag.txt code collection)", year: null, blurb: "Two-pole smoothed %b(5) fade above / below a 200 smoother" }, { id: "tsfour", sourceIds: [], readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/Fred G.Schutzman - JJMBook  Four % Model System for TS/Code.rar", "USB HDD 1/MetaStock/@Systems & Indicators/@Misc Indicators & Systems/Zweig system for ms.rar"], title: "Four % Model code (Schutzman JJMBook TradeStation code; Zweig system for MetaStock)", year: 1998, blurb: "Duplicate of mbsot_zweig4 · Zweig / Ned Davis 4% model code (written for weekly Value Line closes); cited as a source there, not counted twice" }, { id: 'tswtc', readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/Market Analytics - William Tell Coffee Trading System (marketanalytics.net)/WT Coffe Strategy.txt", "USB HDD 1/TradeStation Systems & Indicators/Market Analytics - William Tell Coffee Trading System (marketanalytics.net)/Manual.pdf"], title: "William Tell Coffee Strategy (Market Analytics)", year: null, blurb: "Thirteen price and calendar patterns for coffee futures, $2,750 stop, exit at the first profitable open (manual rules); manual tables reproduced on Norgate coffee" }, { id: 'tsbull', title: "Bullseye Futures DMI signal", year: null, blurb: "+DI / -DI with a 2-bar XAverage, buy stop 35 ticks over the close, sell at market (stop and reverse)" }, { id: 'tsltc', readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/AATS Swing System for S&P500 Index (accuratetradingsystems.com)/LTC SWING.txt"], title: "LTC Swing (accuratetradingsystems.com)", year: 2006, blurb: "ECO stays across zero for 5 bars -> stop and reverse at the close" }, { id: 'tsprosc', title: "ProScan S&P Scalping System (Jan Arps)", year: null, blurb: "SlowD(14) swing divergence-style reversals at the close" }, { id: 'tstango', readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/Tango System for TS (dancewiththemarkets.com)/Open_Code.rar"], title: "Tango System (dancewiththemarkets.com)", year: null, blurb: "ADX-armed pullbacks to the 20 XMA with stop entries, 3-bar exit stops and re-entries" }, { id: 'tsgear', readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/SqueezePlay for TS2000i, TS 7 (impressivetrading.com)/Open Code.rar"], title: "SqueezePlay MACD system (impressivetrading.com)", year: null, blurb: "MACD(8, 66) histogram cross, limit entries, stop and reverse · point and dollar exits held" }, { id: 'tsbond', readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/Bond System for TS/Open Code.rar"], title: "Bond System for TS (ADX/DMI + RSI(4) pullback)", year: null, blurb: "Trend pullback buy stop with 30-bar / 2-bar low exits · money stop held" }, { id: 'tsklvo', readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/Klinger Volume Oscilator for Ts/Readme.rar"], title: "Klinger Volume Oscillator system (TradeStation list post; KVO from Klinger, S&C 1997)", year: null, blurb: "KVO above / below its 13-day trigger, stop entries 0.26 ATR beyond the bar · stop and reverse" }, { id: 'tssierra', readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/Mesa Sierra Hotel System for TS (mesa-systems.com)/Open Code.rar"], title: "Sierra Hotel (Mesa Systems)", year: null, blurb: "Polynomial-smoothed close with % reversal swings, stop and reverse" }, { id: 'tssigma', readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/Sigma Trend System for TS (sigmatrend.com)/Open Code.rar"], title: "Sigma Trend System (sigmatrend.com)", year: null, blurb: "Kaufman adaptive average (70) turns, stop and reverse · 0.2 price filter held" }, { id: 'tsvega', readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/Vega System for TS/Open Code.rar"], title: "Vega System (stochastic set-ups)", year: null, blurb: "Slow stochastic (12) turn set-ups with 6-bar stop entries and set-up-bar stops" }, { id: 'alwdt', readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/Weaver-DT System for TS (alfaranda.com)/Code.rar"], title: "Weaver-DT (alfaranda.com)", year: null, blurb: "Two-swing trendline breakout stops with a channel-width target · no stop loss" }] },
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
    "tsfour": {"type":"code_package","people":["Fred G. Schutzman"]},
    "tswtc": {"type":"code_package","people":[]},
    "tsbull": {"type":"code_package","people":[]},
    "tsltc": {"type":"code_package","people":[]},
    "tsprosc": {"type":"code_package","people":["Jan Arps"]},
    "tstango": {"type":"code_package","people":[]},
    "tsgear": {"type":"code_package","people":[]},
    "tsbond": {"type":"code_package","people":[]},
    "tsklvo": {"type":"code_package","people":["Stephen J. Klinger"]},
    "tssierra": {"type":"code_package","people":[]},
    "tssigma": {"type":"code_package","people":[]},
    "tsvega": {"type":"code_package","people":[]},
    "alwdt": {"type":"code_package","people":[]},
  },
} satisfies AuthorFile;
