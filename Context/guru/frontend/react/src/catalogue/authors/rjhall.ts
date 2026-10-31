import type {AuthorFile} from '../author-files';

export default {
  order: 1593,
  author: { id: "rjhall", name: "Robert J. Hall Jr.", tagline: "S&P band trader (bandtraders.com) · his RSI(21) Paint Bars stop-and-reverse rule runs here", books: [
    { id: "rjhtb", sourceIds: [], title: "Trading The Bands (System + Manual for TradeStation)", year: null, blurb: "Hall · Paint Bars: long above RSI(21) 51, short below 49; the band trades need the compiled G Channel" }] },
  identities: ["Robert J. Hall Jr."],
  profile: {
    "kind": "person",
    "mastery": [
      "S&P futures day trading",
      "TradeStation indicators"
    ],
    "summary": "Robert J. Hall Jr. sold Trading The Bands, a TradeStation band and paint-bar method for short-term S&P trading (bandtraders.com). This profile rests on the manual alone.",
    "known_for": [
      "Trading The Bands"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "rjhtb": {"type":"code_package","people":["Robert J. Hall Jr."]},
  },
} satisfies AuthorFile;
