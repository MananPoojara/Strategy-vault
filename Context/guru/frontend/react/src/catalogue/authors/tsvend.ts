import type {AuthorFile} from '../author-files';

export default {
  order: 1516,
  author: { id: 'tsvend', name: "Vendor and list system code (batch J3)", tagline: "TradeStation and MetaStock systems read from vendor archives and list posts", books: [{ id: 'tsconf', title: "Confluence system (Legan / Fritz, Omega list 1998)", year: 1998, blurb: "Legan's Confluence bull/bear mode stops · 5-point trigger held" }, { id: 'tszlb', title: "Smoothed %b system (ZeroLag.txt code collection)", year: null, blurb: "Two-pole smoothed %b(5) fade above / below a 200 smoother" }, { id: 'tswtc', title: "William Tell Coffee Strategy (Market Analytics)", year: null, blurb: "Fourteen day-of-week patterns for coffee, exit at the first profitable open · no stop" }, { id: 'tsbull', title: "Bullseye Futures DMI signal", year: null, blurb: "+DI / -DI with a 2-bar XAverage, buy stop 35 ticks over the close, sell at market (stop and reverse)" }] },
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
  },
} satisfies AuthorFile;
