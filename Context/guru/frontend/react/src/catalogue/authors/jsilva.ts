import type {AuthorFile} from '../author-files';

export default {
  order: 1599,
  author: { id: "jsilva", name: "Jose Silva", tagline: "MetaStock programmer (metastocktools.com) · three of his teaching examples run here, labelled as such", books: [
    { id: "jsursc", sourceIds: [], title: "URSC3 toolkit for MetaStock: system development articles", year: 2006, blurb: "Silva, 2006 · price breakout and 'do not trade' contrarian breakout examples, run as the author's demonstrations" },
    { id: "jsmdk", sourceIds: [], readerPaths: ["USB HDD 1/MetaStock/@Systems & Indicators/MACDH Divergence Kit (metastocktools.com)/Documentation/Indicators.pdf"], title: "MACDH Divergence Kit for MetaStock: minor divergence profit formulas", year: 2006, blurb: "Silva, 2004-2006 · long on a bullish MACD-histogram trough divergence, short on a bearish peak divergence, close fills ('do not trade' example)" },
    { id: "jsrev2", sourceIds: [], title: "Reversals pattern II (MetaStock Rev2 kit)", year: 2006, blurb: "Silva \u00b7 two consecutive closes down (up) by 0.5% then long (short) at the open of a chosen weekday, exit at that day's close; one card per weekday, run on the NSE indices from the real-open start, readings declared, unverified" }] },
  identities: ["Jose Silva"],
  profile: {
    "kind": "person",
    "mastery": [
      "MetaStock programming",
      "System testing"
    ],
    "summary": "Jose Silva runs metastocktools.com and wrote the URSC3 MetaStock toolkit; its 2006 system development articles use simple breakout rules as teaching examples. This profile rests on those articles alone.",
    "known_for": [
      "URSC3 toolkit for MetaStock"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "jsursc": {"type":"article","people":["Jose Silva"]},
    "jsmdk": {"type":"code_package","people":["Jose Silva"]},
    "jsrev2": {"type":"code_package","people":["Jose Silva"]},
  },
} satisfies AuthorFile;
