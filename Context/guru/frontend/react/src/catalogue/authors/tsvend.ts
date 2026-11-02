import type {AuthorFile} from '../author-files';

export default {
  order: 1516,
  author: { id: 'tsvend', name: "Vendor and list system code (batch J3)", tagline: "TradeStation and MetaStock systems read from vendor archives and list posts", books: [{ id: 'tsconf', title: "Confluence system (Legan / Fritz, Omega list 1998)", year: 1998, blurb: "Legan's Confluence bull/bear mode stops · 5-point trigger held" }] },
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
  },
} satisfies AuthorFile;
