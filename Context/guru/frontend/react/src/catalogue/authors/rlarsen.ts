import type {AuthorFile} from '../author-files';

export default {
  order: 1200,
  author: { id: "rlarsen", name: "Roy Larsen", tagline: "MetaStock programmer (Trade Equity) · the PS Fractal sample entry and exit run here", books: [
    { id: "rlpsf", sourceIds: [], title: "Trade Equity for MetaStock: PS Fractal sample code", year: 2002, blurb: "Larsen · long-only fractal breakout: buy stop at the latest up-fractal high, exit stop at the latest down-fractal low" }] },
  identities: ["Roy Larsen"],
  profile: {
    "kind": "person",
    "mastery": [
      "MetaStock programming"
    ],
    "summary": "Roy Larsen wrote Trade Equity, a MetaStock add-on for portfolio equity testing; its user guide prints the PS Fractal sample entry and exit code. This profile rests on that guide alone.",
    "known_for": [
      "Trade Equity for MetaStock"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "rlpsf": {"type":"code_package","people":["Roy Larsen"]},
  },
} satisfies AuthorFile;
